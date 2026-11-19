//! Counting 1-, 2- and 3-grams over the 27-symbol alphabet.

/// Number of symbols: `a`-`z` and space.
pub const NCHAR: usize = 27;
/// Index of the space symbol.
pub const SPACE: usize = 26;

/// Symbol index of a character (`a`-`z` = 0..25, space = 26).
pub fn index(c: char) -> Option<usize> {
    match c {
        'a'..='z' => Some(c as usize - 'a' as usize),
        ' ' => Some(SPACE),
        _ => None,
    }
}

/// Character of a symbol index.
pub fn symbol(i: usize) -> char {
    if i == SPACE {
        ' '
    } else {
        (b'a' + i as u8) as char
    }
}

/// Raw n-gram counts of a normalized text.
pub struct Counts {
    pub uni: Vec<u64>,
    pub bi: Vec<u64>,
    pub tri: Vec<u64>,
    /// Number of symbols in the text.
    pub len: u64,
}

/// Probabilities of the n-grams that actually occur.
pub struct Model {
    pub uni: Vec<(usize, f64)>,
    pub bi: Vec<(usize, usize, f64)>,
    pub tri: Vec<(usize, usize, usize, f64)>,
}

/// Counts every 1-, 2- and 3-gram in one pass. The text must already be normalized.
pub fn count(text: &str) -> Counts {
    let idx: Vec<usize> = text
        .chars()
        .map(|c| index(c).expect("text must be normalized to a-z and space"))
        .collect();
    let mut uni = vec![0u64; NCHAR];
    let mut bi = vec![0u64; NCHAR * NCHAR];
    let mut tri = vec![0u64; NCHAR * NCHAR * NCHAR];
    for i in 0..idx.len() {
        uni[idx[i]] += 1;
        if i + 1 < idx.len() {
            bi[idx[i] * NCHAR + idx[i + 1]] += 1;
        }
        if i + 2 < idx.len() {
            tri[(idx[i] * NCHAR + idx[i + 1]) * NCHAR + idx[i + 2]] += 1;
        }
    }
    Counts {
        uni,
        bi,
        tri,
        len: idx.len() as u64,
    }
}

impl Counts {
    /// Counts of order `n` (1..=3).
    fn table(&self, n: usize) -> &[u64] {
        match n {
            1 => &self.uni,
            2 => &self.bi,
            3 => &self.tri,
            _ => panic!("n-gram order must be 1, 2 or 3"),
        }
    }

    /// Total number of n-grams of order `n` in the text.
    pub fn total(&self, n: usize) -> u64 {
        self.len.saturating_sub(n as u64 - 1)
    }

    /// Number of distinct n-grams of order `n` that occur.
    pub fn kinds(&self, n: usize) -> usize {
        self.table(n).iter().filter(|&&k| k > 0).count()
    }

    /// All occurring n-grams of order `n`, most frequent first (ties in alphabetical order).
    pub fn ranked(&self, n: usize) -> Vec<(String, u64)> {
        let mut v = Vec::new();
        for (i, &k) in self.table(n).iter().enumerate() {
            if k == 0 {
                continue;
            }
            let mut gram = String::new();
            let mut rest = i;
            let mut chars = Vec::new();
            for _ in 0..n {
                chars.push(symbol(rest % NCHAR));
                rest /= NCHAR;
            }
            for c in chars.iter().rev() {
                gram.push(*c);
            }
            v.push((gram, k));
        }
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    }

    /// Probabilities of the occurring n-grams.
    pub fn model(&self) -> Model {
        let p = |k: u64, n: usize| k as f64 / self.total(n) as f64;
        let mut uni = Vec::new();
        for a in 0..NCHAR {
            if self.uni[a] > 0 {
                uni.push((a, p(self.uni[a], 1)));
            }
        }
        let mut bi = Vec::new();
        for a in 0..NCHAR {
            for b in 0..NCHAR {
                let k = self.bi[a * NCHAR + b];
                if k > 0 {
                    bi.push((a, b, p(k, 2)));
                }
            }
        }
        let mut tri = Vec::new();
        for a in 0..NCHAR {
            for b in 0..NCHAR {
                for c in 0..NCHAR {
                    let k = self.tri[(a * NCHAR + b) * NCHAR + c];
                    if k > 0 {
                        tri.push((a, b, c, p(k, 3)));
                    }
                }
            }
        }
        Model { uni, bi, tri }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_small_text() {
        let c = count("the cat");
        assert_eq!(c.len, 7);
        assert_eq!(c.total(2), 6);
        assert_eq!(c.total(3), 5);
        assert_eq!(c.uni[index('t').unwrap()], 2);
        let ranked = c.ranked(2);
        assert_eq!(ranked.len(), 6);
        assert!(ranked.iter().all(|(_, k)| *k == 1));
        assert_eq!(c.ranked(3)[0].0, " ca");
    }

    #[test]
    fn probabilities_sum_to_one() {
        let m = count("hello world this is a test").model();
        let s1: f64 = m.uni.iter().map(|x| x.1).sum();
        let s2: f64 = m.bi.iter().map(|x| x.2).sum();
        let s3: f64 = m.tri.iter().map(|x| x.3).sum();
        for s in [s1, s2, s3] {
            assert!((s - 1.0).abs() < 1e-12);
        }
    }
}
