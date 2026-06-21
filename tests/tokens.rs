use llmignore::tokens::{estimate_file_tokens, estimate_tokens_from_bytes};
use std::fs;
use tempfile::tempdir;

#[test]
fn estimates_about_four_chars_per_token() {
    assert_eq!(estimate_tokens_from_bytes(400), 100);
    assert_eq!(estimate_tokens_from_bytes(0), 0);
    assert_eq!(estimate_tokens_from_bytes(3), 1, "rounds up partial tokens");
}

#[test]
fn estimates_file_tokens_from_size() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a.txt");
    fs::write(&path, "x".repeat(40)).unwrap();
    assert_eq!(estimate_file_tokens(&path), 10);
}
