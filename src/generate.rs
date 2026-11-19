//! Random text from n-gram statistics (Shannon's approximations of English).

use rand::RngExt;
use rand::rngs::StdRng;
use std::collections::HashMap;

/// Picks one character with probability proportional to its count.
fn pick(cands: &[(char, u64)], rng: &mut StdRng) -> char {
    let sum: u64 = cands.iter().map(|c| c.1).sum();
    let mut r = rng.random_range(0..sum);
    for &(c, n) in cands {
        if r < n {
            return c;
        }
        r -= n;
    }
    ' '
}

/// Generates `len` characters with the k-th order approximation.
/// k = 0: every symbol equally likely, k = 1: letter frequencies,
/// k >= 2: the next letter depends on the previous k-1 letters.
pub fn generate(text: &str, k: usize, len: usize, rng: &mut StdRng) -> String {
    let chars: Vec<char> = text.chars().collect();
    if k == 0 {
        let alphabet: Vec<(char, u64)> = "abcdefghijklmnopqrstuvwxyz "
            .chars()
            .map(|c| (c, 1))
            .collect();
        return (0..len).map(|_| pick(&alphabet, rng)).collect();
    }
    if chars.len() < k + 1 {
        return String::new();
    }

    // previous k-1 characters -> candidates for the next character
    let mut counts: HashMap<String, HashMap<char, u64>> = HashMap::new();
    for w in chars.windows(k) {
        let prefix: String = w[..k - 1].iter().collect();
        *counts
            .entry(prefix)
            .or_default()
            .entry(w[k - 1])
            .or_insert(0) += 1;
    }
    let mut table: HashMap<String, Vec<(char, u64)>> = HashMap::new();
    for (prefix, next) in counts {
        let mut v: Vec<(char, u64)> = next.into_iter().collect();
        v.sort();
        table.insert(prefix, v);
    }

    // start from k-1 characters at a random position of the text
    let start = rng.random_range(0..chars.len() - k);
    let mut out: Vec<char> = chars[start..start + k - 1].to_vec();
    while out.len() < len {
        let context: String = out[out.len() - (k - 1)..].iter().collect();
        match table.get(&context) {
            Some(cands) => out.push(pick(cands, rng)),
            None => out.push(' '),
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn length_and_alphabet() {
        let text = "the cat sat on the mat and the dog sat on the log";
        for k in 0..4 {
            let s = generate(text, k, 50, &mut StdRng::seed_from_u64(3));
            assert_eq!(s.chars().count(), 50);
            assert!(s.chars().all(|c| c == ' ' || c.is_ascii_lowercase()));
        }
    }

    #[test]
    fn high_order_reproduces_seen_trigrams() {
        let text = "abc abd abc abd";
        let s = generate(text, 3, 40, &mut StdRng::seed_from_u64(5));
        let chars: Vec<char> = s.chars().collect();
        for w in chars.windows(3) {
            let t: String = w.iter().collect();
            assert!(text.contains(&t), "{t:?} never appears in the source");
        }
    }
}
