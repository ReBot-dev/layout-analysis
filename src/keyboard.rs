//! Keyboard geometry, layouts and the typing-cost model.

use crate::ngram::{Model, NCHAR, SPACE, index};
use serde::Deserialize;

/// Number of letter keys: 3 rows x 10 columns (the QWERTY letter keys plus `; , . /`).
pub const NSLOT: usize = 30;
/// The thumb's space bar. Space is always typed here.
pub const SPACE_SLOT: usize = 30;
/// Marks a key with no letter on it.
pub const BLANK: char = '_';

/// Fingers: 0-3 = left pinky..index, 4 = thumb, 6-9 = right index..pinky.
const COLUMN_FINGER: [usize; 10] = [0, 1, 2, 3, 3, 6, 6, 7, 8, 9];
const THUMB: usize = 4;

/// Parameters of the cost model. Every field can be overridden from a TOML file.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Params {
    /// Horizontal offset of the top and bottom rows (row-staggered keyboard), in key units.
    pub top_row_offset: f64,
    pub bottom_row_offset: f64,
    /// Load weight of each finger.
    pub weight_index: f64,
    pub weight_middle: f64,
    pub weight_ring: f64,
    pub weight_pinky: f64,
    /// Same-finger bigram: `sfb * weight * distance`.
    pub sfb: f64,
    /// Adjacent fingers of one hand jumping between the top and bottom rows:
    /// `scissor + same_hand * distance`.
    pub scissor: f64,
    /// Other same-hand bigrams: `same_hand * distance`.
    pub same_hand: f64,
    /// Bigrams that alternate hands (negative = bonus).
    pub alternate: f64,
    /// Same-finger skipgram (1st and 3rd key on one finger): `sfs * weight * distance`.
    pub sfs: f64,
    /// One-hand trigram whose finger direction reverses.
    pub redirect: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            top_row_offset: -0.25,
            bottom_row_offset: 0.5,
            weight_index: 1.0,
            weight_middle: 1.0,
            weight_ring: 1.2,
            weight_pinky: 1.5,
            sfb: 5.0,
            scissor: 2.5,
            same_hand: 0.4,
            alternate: -0.2,
            sfs: 2.0,
            redirect: 1.0,
        }
    }
}

impl Params {
    fn weight(&self, finger: usize) -> f64 {
        match finger {
            0 | 9 => self.weight_pinky,
            1 | 8 => self.weight_ring,
            2 | 7 => self.weight_middle,
            THUMB => 1.0,
            _ => self.weight_index,
        }
    }
}

/// A key: position (x, y) in key units and the finger that presses it.
#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub x: f64,
    pub y: f64,
    pub finger: usize,
}

/// 0 = left hand, 1 = right hand, 2 = thumb.
fn hand(finger: usize) -> usize {
    match finger {
        0..=3 => 0,
        THUMB => 2,
        _ => 1,
    }
}

fn dist(a: Key, b: Key) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// The 30 letter keys (row by row) followed by the space bar.
pub fn keys(params: &Params) -> Vec<Key> {
    let mut keys = Vec::new();
    for (y, dx) in [
        (-1.0, params.top_row_offset),
        (0.0, 0.0),
        (1.0, params.bottom_row_offset),
    ] {
        for (col, &finger) in COLUMN_FINGER.iter().enumerate() {
            keys.push(Key {
                x: col as f64 + dx,
                y,
                finger,
            });
        }
    }
    keys.push(Key {
        x: 4.5,
        y: 2.0,
        finger: THUMB,
    });
    keys
}

/// Distance of each key from its finger's home key (`A S D F` / `J K L ;`).
pub fn home_distance(keys: &[Key]) -> Vec<f64> {
    keys.iter()
        .map(|&k| {
            if k.finger == THUMB {
                0.0
            } else {
                let home_col = COLUMN_FINGER.iter().position(|&f| f == k.finger).unwrap();
                let home_col = if k.finger == 6 { 6 } else { home_col };
                dist(k, keys[10 + home_col])
            }
        })
        .collect()
}

/// Costs that depend only on key positions, computed once for every key pair and triple.
pub struct CostTables {
    n: usize,
    c1: Vec<f64>,
    c2: Vec<f64>,
    c3: Vec<f64>,
}

impl CostTables {
    pub fn new(params: &Params) -> Self {
        let keys = keys(params);
        let home = home_distance(&keys);
        let n = keys.len();

        let mut c1 = vec![0.0; n];
        for s in 0..n {
            c1[s] = params.weight(keys[s].finger) * home[s];
        }

        let mut c2 = vec![0.0; n * n];
        for s1 in 0..NSLOT {
            for s2 in 0..NSLOT {
                let (a, b) = (keys[s1], keys[s2]);
                let d = dist(a, b);
                c2[s1 * n + s2] = if a.finger == b.finger && s1 != s2 {
                    params.sfb * params.weight(a.finger) * d
                } else if hand(a.finger) == hand(b.finger) {
                    let adjacent = (a.finger as i32 - b.finger as i32).abs() == 1;
                    if adjacent && (a.y - b.y).abs() >= 2.0 {
                        params.scissor + params.same_hand * d
                    } else {
                        params.same_hand * d
                    }
                } else {
                    params.alternate
                };
            }
        }

        let mut c3 = vec![0.0; n * n * n];
        for s1 in 0..NSLOT {
            for s2 in 0..NSLOT {
                for s3 in 0..NSLOT {
                    let (a, b, c) = (keys[s1], keys[s2], keys[s3]);
                    let mut cost = 0.0;
                    if a.finger == c.finger && s1 != s3 {
                        cost += params.sfs * params.weight(a.finger) * dist(a, c);
                    }
                    if hand(a.finger) == hand(b.finger) && hand(b.finger) == hand(c.finger) {
                        let d1 = b.finger as i32 - a.finger as i32;
                        let d2 = c.finger as i32 - b.finger as i32;
                        if d1 * d2 < 0 {
                            cost += params.redirect;
                        }
                    }
                    c3[(s1 * n + s2) * n + s3] = cost;
                }
            }
        }
        CostTables { n, c1, c2, c3 }
    }
}

/// A layout: the letter on each of the 30 keys (`_` for an empty key).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout(pub Vec<char>);

/// Built-in layouts, written over the QWERTY key positions.
pub const BUILTIN: [(&str, &str); 5] = [
    ("qwerty", "qwertyuiopasdfghjkl_zxcvbnm___"),
    ("dvorak", "___pyfgcrlaoeuidhtns_qjkxbmwvz"),
    ("colemak", "qwfpgjluy_arstdhneiozxcvbkm___"),
    ("colemak-dh", "qwfpbjluy_arstgmneiozxcdvkh___"),
    ("workman", "qdrwbjfup_ashtgyneoizxmcvkl___"),
];

impl Layout {
    /// A built-in name (`qwerty`, `dvorak`, ...) or 30 characters in row order
    /// (`_` for an empty key; spaces and `/` between rows are ignored).
    pub fn parse(s: &str) -> Result<Layout, String> {
        if let Some((_, keys)) = BUILTIN.iter().find(|(name, _)| *name == s) {
            return Layout::parse(keys);
        }
        let chars: Vec<char> = s
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '/')
            .collect();
        if chars.len() != NSLOT {
            return Err(format!(
                "{s:?} is neither a built-in layout nor 30 keys (got {} keys)",
                chars.len()
            ));
        }
        let mut seen = [false; 26];
        for &c in &chars {
            if c == BLANK {
                continue;
            }
            match index(c) {
                Some(i) if i < 26 => {
                    if seen[i] {
                        return Err(format!("letter {c:?} appears twice in {s:?}"));
                    }
                    seen[i] = true;
                }
                _ => return Err(format!("{c:?} is not a lowercase letter or `_`")),
            }
        }
        if let Some(i) = seen.iter().position(|&x| !x) {
            return Err(format!(
                "letter {:?} is missing in {s:?}",
                (b'a' + i as u8) as char
            ));
        }
        Ok(Layout(chars))
    }

    /// Key of each symbol (space goes to the space bar).
    fn key_of(&self) -> [usize; NCHAR] {
        let mut map = [SPACE_SLOT; NCHAR];
        for (s, &c) in self.0.iter().enumerate() {
            if c != BLANK {
                map[index(c).unwrap()] = s;
            }
        }
        map
    }

    /// The layout as one 30-character string, e.g. `qwertyuiop/asdfghjkl_/zxcvbnm___`.
    pub fn compact(&self) -> String {
        let rows: Vec<String> = self.0.chunks(10).map(|r| r.iter().collect()).collect();
        rows.join("/")
    }

    /// Three rows with the hand boundary marked.
    pub fn grid(&self) -> String {
        let mut out = String::new();
        for (r, row) in self.0.chunks(10).enumerate() {
            out.push_str(&" ".repeat(r));
            for (c, ch) in row.iter().enumerate() {
                if c == 5 {
                    out.push_str("| ");
                }
                out.push(*ch);
                out.push(' ');
            }
            out.push('\n');
        }
        out
    }
}

/// Expected cost per keystroke. `order` = 1 uses only single-key costs,
/// 2 adds bigram costs and 3 adds trigram costs.
pub fn evaluate(layout: &Layout, tables: &CostTables, model: &Model, order: usize) -> f64 {
    let n = tables.n;
    let key = layout.key_of();
    let mut e = 0.0;
    for &(a, p) in &model.uni {
        e += p * tables.c1[key[a]];
    }
    if order >= 2 {
        for &(a, b, p) in &model.bi {
            e += p * tables.c2[key[a] * n + key[b]];
        }
    }
    if order >= 3 {
        for &(a, b, c, p) in &model.tri {
            e += p * tables.c3[(key[a] * n + key[b]) * n + key[c]];
        }
    }
    e
}

/// Summary statistics of a layout.
pub struct Stats {
    /// Same-finger bigrams, % of letter bigrams (bigrams without space).
    pub sfb: f64,
    /// Hand alternation, % of letter bigrams.
    pub alternation: f64,
    /// Average distance from the home key per keystroke (unweighted, space = 0).
    pub home_distance: f64,
    /// Share of keystrokes per finger, % (index 0-9, see the finger numbering).
    pub finger_load: [f64; 10],
}

pub fn stats(layout: &Layout, params: &Params, model: &Model) -> Stats {
    let keys = keys(params);
    let home = home_distance(&keys);
    let key = layout.key_of();
    let mut letter_pairs = 0.0;
    let mut sfb = 0.0;
    let mut alt = 0.0;
    for &(a, b, p) in &model.bi {
        if a == SPACE || b == SPACE {
            continue;
        }
        let (k1, k2) = (keys[key[a]], keys[key[b]]);
        letter_pairs += p;
        if k1.finger == k2.finger && key[a] != key[b] {
            sfb += p;
        }
        if hand(k1.finger) != hand(k2.finger) {
            alt += p;
        }
    }
    let mut home_distance = 0.0;
    let mut finger_load = [0.0; 10];
    for &(a, p) in &model.uni {
        home_distance += p * home[key[a]];
        finger_load[keys[key[a]].finger] += p * 100.0;
    }
    let pct = |x: f64| {
        if letter_pairs > 0.0 {
            x / letter_pairs * 100.0
        } else {
            0.0
        }
    };
    Stats {
        sfb: pct(sfb),
        alternation: pct(alt),
        home_distance,
        finger_load,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngram::count;

    #[test]
    fn builtin_layouts_are_valid() {
        for (name, _) in BUILTIN {
            assert!(Layout::parse(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn parse_rejects_bad_layouts() {
        assert!(Layout::parse("abc").is_err());
        assert!(Layout::parse("qqertyuiopasdfghjkl_zxcvbnm___").is_err());
        assert!(Layout::parse("qwertyuiopasdfghjkl_zxcvbnm__;").is_err());
        assert_eq!(
            Layout::parse("qwertyuiop/asdfghjkl_/zxcvbnm___").unwrap(),
            Layout::parse("qwerty").unwrap()
        );
    }

    #[test]
    fn home_keys_have_zero_distance() {
        let k = keys(&Params::default());
        let h = home_distance(&k);
        for col in [0, 1, 2, 3, 6, 7, 8, 9] {
            assert_eq!(h[10 + col], 0.0);
        }
        assert!(h[10 + 4] > 0.0 && h[10 + 5] > 0.0);
    }

    #[test]
    fn same_finger_bigram_is_detected() {
        // "de" is typed with the left middle finger on QWERTY.
        let m = count("de de de").model();
        let s = stats(&Layout::parse("qwerty").unwrap(), &Params::default(), &m);
        assert!((s.sfb - 100.0).abs() < 1e-9);
        assert_eq!(s.alternation, 0.0);
    }

    #[test]
    fn alternating_text_is_cheaper_on_dvorak() {
        let m = count("the theory that these then thus").model();
        let t = CostTables::new(&Params::default());
        let q = evaluate(&Layout::parse("qwerty").unwrap(), &t, &m, 3);
        let d = evaluate(&Layout::parse("dvorak").unwrap(), &t, &m, 3);
        assert!(d < q);
    }
}
