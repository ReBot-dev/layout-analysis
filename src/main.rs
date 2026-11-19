use clap::{Args, Parser, Subcommand};
use layout_analysis::keyboard::{self, BUILTIN, CostTables, Layout, Params};
use layout_analysis::optimize::{self, Settings};
use layout_analysis::text::Source;
use layout_analysis::{generate, load, ngram};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::fs;
use std::process;

#[derive(Parser)]
#[command(
    version,
    about = "Character n-gram analysis of English text and keyboard layout evaluation / optimization"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct Input {
    /// Text files to read (concatenated)
    #[arg(required = true)]
    files: Vec<String>,
    /// Input format: plain, gutenberg (strip the license header/footer) or tatoeba (TSV export)
    #[arg(long, default_value = "plain")]
    source: Source,
}

#[derive(Args)]
struct Model {
    /// TOML file overriding the cost model parameters (see params.example.toml)
    #[arg(long)]
    params: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Frequency tables of letters, bigrams and trigrams
    Analyze {
        #[command(flatten)]
        input: Input,
        /// Number of rows to show per table
        #[arg(long, default_value_t = 30)]
        top: usize,
        /// Print CSV (n,gram,count,probability) instead of tables
        #[arg(long)]
        csv: bool,
    },
    /// Random text from the k-th order approximation
    Generate {
        #[command(flatten)]
        input: Input,
        /// Order: 0 = uniform, 1 = letter frequencies, k = depends on the previous k-1 letters
        #[arg(short, long, default_value_t = 3)]
        k: usize,
        /// Number of characters
        #[arg(short = 'n', long, default_value_t = 100)]
        length: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// Typing cost of layouts for the given text
    Eval {
        #[command(flatten)]
        input: Input,
        #[command(flatten)]
        model: Model,
        /// Layouts to evaluate: built-in names or 30 keys in row order (default: all built-in)
        #[arg(short, long)]
        layout: Vec<String>,
        /// Print CSV
        #[arg(long)]
        csv: bool,
    },
    /// Search for the best (or worst) layout with simulated annealing
    Optimize {
        #[command(flatten)]
        input: Input,
        #[command(flatten)]
        model: Model,
        /// Maximize the cost instead: a "ZETSUBOU" layout
        #[arg(long)]
        worst: bool,
        /// Cost terms to optimize: 1 = single keys, 2 = + bigrams, 3 = + trigrams
        #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=3))]
        order: u8,
        /// Number of swap attempts
        #[arg(long, default_value_t = 300_000)]
        steps: usize,
        /// Initial layout
        #[arg(long, default_value = "qwerty")]
        from: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// List the built-in layouts
    Layouts,
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    process::exit(1);
}

fn read_input(input: &Input) -> String {
    load(&input.files, input.source).unwrap_or_else(|e| fail(e))
}

fn read_params(model: &Model) -> Params {
    match &model.params {
        None => Params::default(),
        Some(path) => {
            let s = fs::read_to_string(path).unwrap_or_else(|e| fail(format!("{path}: {e}")));
            toml::from_str(&s).unwrap_or_else(|e| fail(format!("{path}: {e}")))
        }
    }
}

fn parse_layout(s: &str) -> Layout {
    Layout::parse(s).unwrap_or_else(|e| fail(e))
}

/// Shows a space as a visible character.
fn visible(gram: &str) -> String {
    gram.replace(' ', "_")
}

fn analyze(text: &str, top: usize, csv: bool) {
    let counts = ngram::count(text);
    if csv {
        println!("n,gram,count,probability");
    } else {
        println!("{} characters (a-z and space; `_` = space)\n", counts.len);
    }
    for n in 1..=3 {
        let total = counts.total(n);
        let ranked = counts.ranked(n);
        if csv {
            for (g, k) in &ranked {
                println!("{n},{},{k},{:.6}", visible(g), *k as f64 / total as f64);
            }
            continue;
        }
        let max_kinds = match n {
            1 => 27,
            2 => 27 * 27 - 1,
            _ => 27 * 27 * 27 - 53,
        };
        println!(
            "{n}-grams: {} kinds of {max_kinds} possible ({:.1}%)",
            ranked.len(),
            ranked.len() as f64 / max_kinds as f64 * 100.0
        );
        println!("{:>4}  {:<5} {:>12} {:>8}", "rank", "gram", "count", "%");
        for (i, (g, k)) in ranked.iter().take(top).enumerate() {
            println!(
                "{:>4}  {:<5} {:>12} {:>8.3}",
                i + 1,
                visible(g),
                k,
                *k as f64 / total as f64 * 100.0
            );
        }
        println!();
    }
}

/// Built-in names when none are given; every layout is checked before the text is read.
fn layouts_to_eval(names: &[String]) -> Vec<(String, Layout)> {
    let names: Vec<String> = if names.is_empty() {
        BUILTIN.iter().map(|(n, _)| n.to_string()).collect()
    } else {
        names.to_vec()
    };
    names
        .into_iter()
        .map(|n| (n.clone(), parse_layout(&n)))
        .collect()
}

fn eval(text: &str, params: &Params, layouts: &[(String, Layout)], csv: bool) {
    let model = ngram::count(text).model();
    let tables = CostTables::new(params);
    let qwerty = parse_layout("qwerty");
    let e_q = keyboard::evaluate(&qwerty, &tables, &model, 3);
    if csv {
        println!("layout,cost,relative,sfb_pct,alternation_pct,home_distance,keys");
    } else {
        println!(
            "{:<12} {:>8} {:>9} {:>7} {:>7} {:>9}  keys",
            "layout", "cost E", "QWERTY=100", "SFB%", "alt%", "home dist"
        );
    }
    for (name, l) in layouts {
        let e = keyboard::evaluate(l, &tables, &model, 3);
        let s = keyboard::stats(l, params, &model);
        if csv {
            println!(
                "{name},{e:.6},{:.2},{:.3},{:.3},{:.4},{}",
                e / e_q * 100.0,
                s.sfb,
                s.alternation,
                s.home_distance,
                l.compact()
            );
        } else {
            let label = if name.len() > 12 {
                "custom"
            } else {
                name.as_str()
            };
            println!(
                "{label:<12} {e:>8.4} {:>9.1} {:>7.2} {:>7.1} {:>9.3}  {}",
                e / e_q * 100.0,
                s.sfb,
                s.alternation,
                s.home_distance,
                l.compact()
            );
        }
    }
}

fn print_layout(name: &str, l: &Layout, text: &str, params: &Params) {
    let model = ngram::count(text).model();
    let tables = CostTables::new(params);
    let e = keyboard::evaluate(l, &tables, &model, 3);
    let e_q = keyboard::evaluate(&parse_layout("qwerty"), &tables, &model, 3);
    let s = keyboard::stats(l, params, &model);
    println!("{name}\n");
    print!("{}", l.grid());
    println!();
    println!("keys          {}", l.compact());
    println!(
        "cost E        {e:.4}  (QWERTY = 100: {:.1})",
        e / e_q * 100.0
    );
    println!("SFB           {:.2}% of letter bigrams", s.sfb);
    println!("alternation   {:.1}% of letter bigrams", s.alternation);
    println!("home distance {:.3} keys per keystroke", s.home_distance);
    let f = s.finger_load;
    println!(
        "finger load   L: {:.1} {:.1} {:.1} {:.1} | R: {:.1} {:.1} {:.1} {:.1}  (%, pinky..index | index..pinky)",
        f[0], f[1], f[2], f[3], f[6], f[7], f[8], f[9]
    );
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Analyze { input, top, csv } => analyze(&read_input(&input), top, csv),
        Command::Generate {
            input,
            k,
            length,
            seed,
        } => {
            let text = read_input(&input);
            let mut rng = StdRng::seed_from_u64(seed);
            println!("{}", generate::generate(&text, k, length, &mut rng));
        }
        Command::Eval {
            input,
            model,
            layout,
            csv,
        } => {
            let layouts = layouts_to_eval(&layout);
            let params = read_params(&model);
            eval(&read_input(&input), &params, &layouts, csv)
        }
        Command::Optimize {
            input,
            model,
            worst,
            order,
            steps,
            from,
            seed,
        } => {
            let init = parse_layout(&from);
            let params = read_params(&model);
            let text = read_input(&input);
            let counts = ngram::count(&text);
            let tables = CostTables::new(&params);
            let settings = Settings {
                order: order as usize,
                worst,
                steps,
                ..Settings::default()
            };
            let mut rng = StdRng::seed_from_u64(seed);
            let result = optimize::anneal(&init, &tables, &counts.model(), &settings, &mut rng);
            let title = if worst {
                "worst layout (ZETSUBOU)"
            } else {
                "optimized layout"
            };
            print_layout(title, &result, &text, &params);
        }
        Command::Layouts => {
            for (name, _) in BUILTIN {
                println!("{name}\n{}", parse_layout(name).grid());
            }
        }
    }
}
