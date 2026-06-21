//! Rough token estimation so users can see "how much am I sending to the AI?".

use std::path::Path;

const CHARS_PER_TOKEN: u64 = 4;

/// Estimate tokens from a raw byte length using the ~4-chars-per-token heuristic.
///
/// Rounds up so any non-empty input is at least one token.
pub fn estimate_tokens_from_bytes(bytes: u64) -> u64 {
    bytes.div_ceil(CHARS_PER_TOKEN)
}

/// Estimate the number of LLM tokens a file represents, based on its size on disk.
///
/// Dependency-free and approximate; meant for "ballpark" reporting, not billing.
/// Returns 0 if the file can't be read.
pub fn estimate_file_tokens(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|m| estimate_tokens_from_bytes(m.len()))
        .unwrap_or(0)
}
