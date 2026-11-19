//! Turning raw text into the 27-symbol alphabet (a-z and space).

use std::str::FromStr;

/// Where the input text comes from. Each source has its own wrapper to strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Any plain text file. Used as is.
    Plain,
    /// A Project Gutenberg e-book. The license header and footer are removed.
    Gutenberg,
    /// A Tatoeba sentence export (TSV). Only the sentence column is kept.
    Tatoeba,
}

impl FromStr for Source {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "plain" => Ok(Source::Plain),
            "gutenberg" => Ok(Source::Gutenberg),
            "tatoeba" => Ok(Source::Tatoeba),
            _ => Err(format!(
                "unknown source {s:?} (expected plain, gutenberg or tatoeba)"
            )),
        }
    }
}

/// Removes the wrapper of the given source and returns the body text.
pub fn extract(text: &str, source: Source) -> Result<String, String> {
    match source {
        Source::Plain => Ok(text.to_string()),
        Source::Gutenberg => gutenberg_body(text),
        Source::Tatoeba => Ok(tatoeba_body(text)),
    }
}

/// Body of a Project Gutenberg e-book: from the line after
/// `*** START OF ... ***` to just before `*** END OF`.
/// The START marker may be wrapped over two lines, so the closing `***` is searched for.
fn gutenberg_body(text: &str) -> Result<String, String> {
    let start = text
        .find("*** START OF")
        .ok_or("no `*** START OF` marker (is this a Project Gutenberg file?)")?;
    let close = start
        + 3
        + text[start + 3..]
            .find("***")
            .ok_or("START marker is not closed")?;
    let body_start = match text[close..].find('\n') {
        Some(i) => close + i + 1,
        None => text.len(),
    };
    let end = text[body_start..]
        .find("*** END OF")
        .map(|i| body_start + i)
        .unwrap_or(text.len());
    Ok(text[body_start..end].to_string())
}

/// Sentence column (3rd) of a Tatoeba TSV export, one sentence per line.
/// Lines with fewer columns are used as they are.
fn tatoeba_body(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let sentence = line.split('\t').nth(2).unwrap_or(line);
        out.push_str(sentence);
        out.push(' ');
    }
    out
}

/// Maps text onto the 27-symbol alphabet:
/// letters are lowercased, everything else becomes a space,
/// and runs of spaces are collapsed into one.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercases_and_collapses() {
        assert_eq!(
            normalize("  Price: $120.00 (tax)\r\n\tDone!"),
            "price tax done"
        );
        assert_eq!(normalize("café — Ünïcode"), "caf n code");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn gutenberg_two_line_marker() {
        let text = "header\n*** START OF THE PROJECT GUTENBERG EBOOK A LONG\nTITLE ***\nBody text.\n*** END OF THE PROJECT\nlicense";
        assert_eq!(extract(text, Source::Gutenberg).unwrap(), "Body text.\n");
    }

    #[test]
    fn gutenberg_without_marker_is_an_error() {
        assert!(extract("just text", Source::Gutenberg).is_err());
    }

    #[test]
    fn tatoeba_keeps_sentence_column() {
        let text = "1\teng\tHello there.\tuser\n2\teng\tBye.\tuser\n";
        assert_eq!(
            normalize(&extract(text, Source::Tatoeba).unwrap()),
            "hello there bye"
        );
    }
}
