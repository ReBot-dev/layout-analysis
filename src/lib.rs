//! Character n-gram analysis of English text and keyboard layout evaluation / optimization.

pub mod generate;
pub mod keyboard;
pub mod ngram;
pub mod optimize;
pub mod text;

use std::fs;
use text::Source;

/// Reads the files, strips the source wrappers and normalizes them into one text.
pub fn load(paths: &[String], source: Source) -> Result<String, String> {
    let mut all = String::new();
    for p in paths {
        let raw = fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        let body = text::extract(&raw, source).map_err(|e| format!("{p}: {e}"))?;
        let norm = text::normalize(&body);
        if norm.is_empty() {
            continue;
        }
        if !all.is_empty() {
            all.push(' ');
        }
        all.push_str(&norm);
    }
    if all.is_empty() {
        return Err("no letters found in the input".to_string());
    }
    Ok(all)
}
