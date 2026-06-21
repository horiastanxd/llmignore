use llmignore::scan::find_exposed_secrets;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn flags_env_file_not_covered_by_user_llmignore() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    // User wrote a .llmignore but forgot secrets -> defaults do NOT apply.
    fs::write(root.join(".llmignore"), "*.log\n").unwrap();
    fs::write(root.join(".env"), "SECRET=abc").unwrap();
    fs::write(root.join("main.rs"), "code").unwrap();

    let findings = find_exposed_secrets(root).unwrap();

    assert!(
        findings.iter().any(|f| f.path == Path::new(".env")),
        "expected .env to be flagged as exposed, got: {findings:?}"
    );
    assert!(
        !findings.iter().any(|f| f.path == Path::new("main.rs")),
        "main.rs is not a secret and must not be flagged"
    );
}

#[test]
fn does_not_flag_secrets_already_covered_by_defaults() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    // No .llmignore -> embedded defaults apply -> .env is already ignored.
    fs::write(root.join(".env"), "SECRET=abc").unwrap();
    fs::write(root.join("main.rs"), "code").unwrap();

    let findings = find_exposed_secrets(root).unwrap();

    assert!(
        findings.is_empty(),
        "defaults already ignore .env, nothing should be exposed: {findings:?}"
    );
}

#[test]
fn flags_terraform_state_and_git_credentials() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join(".llmignore"), "*.log\n").unwrap();
    fs::write(root.join("terraform.tfstate"), "{}").unwrap();
    fs::write(root.join(".git-credentials"), "https://x:y@h").unwrap();

    let findings = find_exposed_secrets(root).unwrap();
    let paths: Vec<_> = findings.iter().map(|f| f.path.clone()).collect();

    assert!(
        paths.contains(&Path::new("terraform.tfstate").to_path_buf()),
        "tfstate should be flagged: {paths:?}"
    );
    assert!(
        paths.contains(&Path::new(".git-credentials").to_path_buf()),
        ".git-credentials should be flagged: {paths:?}"
    );
}

#[test]
fn finding_includes_a_reason() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join(".llmignore"), "\n").unwrap();
    fs::write(root.join("id_rsa"), "PRIVATE KEY").unwrap();

    let findings = find_exposed_secrets(root).unwrap();
    let f = findings
        .iter()
        .find(|f| f.path == Path::new("id_rsa"))
        .expect("id_rsa should be flagged");
    assert!(
        f.reason.to_lowercase().contains("ssh"),
        "reason should mention SSH key, got: {}",
        f.reason
    );
}
