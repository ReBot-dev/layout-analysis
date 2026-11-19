//! Simulated annealing over key assignments.

use crate::keyboard::{BLANK, CostTables, Layout, NSLOT, evaluate};
use crate::ngram::Model;
use rand::RngExt;
use rand::rngs::StdRng;

pub struct Settings {
    /// Cost terms to use (1 = single keys only, 3 = up to trigrams).
    pub order: usize,
    /// Maximize the cost instead (a "ZETSUBOU" layout).
    pub worst: bool,
    pub steps: usize,
    /// Start and end temperature of the exponential cooling schedule.
    pub t_start: f64,
    pub t_end: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            order: 3,
            worst: false,
            steps: 300_000,
            t_start: 0.05,
            t_end: 1e-5,
        }
    }
}

/// Swaps two keys at a time and accepts worse states with probability exp(-dE / T).
/// Returns the best layout found.
pub fn anneal(
    init: &Layout,
    tables: &CostTables,
    model: &Model,
    settings: &Settings,
    rng: &mut StdRng,
) -> Layout {
    let mut current = init.clone();
    let mut cur = evaluate(&current, tables, model, settings.order);
    let mut best = current.clone();
    let mut best_e = cur;
    let steps = settings.steps.max(1);

    for step in 0..steps {
        let progress = step as f64 / steps as f64;
        let temp = settings.t_start * (settings.t_end / settings.t_start).powf(progress);
        let i = rng.random_range(0..NSLOT);
        let j = rng.random_range(0..NSLOT);
        if i == j || (current.0[i] == BLANK && current.0[j] == BLANK) {
            continue;
        }
        current.0.swap(i, j);
        let next = evaluate(&current, tables, model, settings.order);
        let diff = if settings.worst {
            cur - next
        } else {
            next - cur
        };
        if diff < 0.0 || (-diff / temp).exp() > rng.random_range(0.0..1.0) {
            cur = next;
            let improved = if settings.worst {
                cur > best_e
            } else {
                cur < best_e
            };
            if improved {
                best = current.clone();
                best_e = cur;
            }
        } else {
            current.0.swap(i, j);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::Params;
    use crate::ngram::count;
    use rand::SeedableRng;

    fn model() -> Model {
        count("the quick brown fox jumps over the lazy dog and then the end").model()
    }

    #[test]
    fn best_beats_qwerty_and_worst_is_worse() {
        let t = CostTables::new(&Params::default());
        let m = model();
        let q = Layout::parse("qwerty").unwrap();
        let s = Settings {
            steps: 20_000,
            ..Settings::default()
        };
        let best = anneal(&q, &t, &m, &s, &mut StdRng::seed_from_u64(1));
        let worst = anneal(
            &q,
            &t,
            &m,
            &Settings { worst: true, ..s },
            &mut StdRng::seed_from_u64(1),
        );
        let eq = evaluate(&q, &t, &m, 3);
        assert!(evaluate(&best, &t, &m, 3) < eq);
        assert!(evaluate(&worst, &t, &m, 3) > eq);
        // still a valid layout
        assert!(Layout::parse(&best.compact()).is_ok());
    }

    #[test]
    fn same_seed_same_result() {
        let t = CostTables::new(&Params::default());
        let m = model();
        let q = Layout::parse("qwerty").unwrap();
        let s = Settings {
            steps: 5_000,
            ..Settings::default()
        };
        let a = anneal(&q, &t, &m, &s, &mut StdRng::seed_from_u64(7));
        let b = anneal(&q, &t, &m, &s, &mut StdRng::seed_from_u64(7));
        assert_eq!(a, b);
    }
}
