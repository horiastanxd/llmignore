use llmignore::{scan, ScanOptions};
use std::fs;
use tempfile::tempdir;

fn names(result: &llmignore::ScanResult) -> Vec<String> {
    result
        .included
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect()
}

#[test]
fn respects_llmignore_patterns() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join(".llmignore"), "secret.txt\n*.log\n").unwrap();
    fs::write(root.join("main.rs"), "code").unwrap();
    fs::write(root.join("secret.txt"), "x").unwrap();
    fs::write(root.join("app.log"), "x").unwrap();

    let result = scan(root, &ScanOptions::default()).unwrap();
    let included = names(&result);

    assert!(
        included.contains(&"main.rs".to_string()),
        "main.rs should be included"
    );
    assert!(
        !included.contains(&"secret.txt".to_string()),
        "secret.txt should be ignored"
    );
    assert!(
        !included.contains(&"app.log".to_string()),
        "app.log should be ignored"
    );
}
