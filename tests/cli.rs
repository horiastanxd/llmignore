use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn cmd(root: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("llmignore").unwrap();
    c.current_dir(root);
    c
}

#[test]
fn init_creates_llmignore() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    cmd(root).arg("init").assert().success();
    let content = fs::read_to_string(root.join(".llmignore")).unwrap();
    assert!(
        content.contains(".env"),
        "default ruleset should ignore .env"
    );
    assert!(content.contains("node_modules/"));
}

#[test]
fn init_refuses_to_overwrite_without_force() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join(".llmignore"), "custom\n").unwrap();
    cmd(root).arg("init").assert().failure();
    // original preserved
    assert_eq!(
        fs::read_to_string(root.join(".llmignore")).unwrap(),
        "custom\n"
    );
}

#[test]
fn init_force_overwrites() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join(".llmignore"), "custom\n").unwrap();
    cmd(root).args(["init", "--force"]).assert().success();
    assert!(fs::read_to_string(root.join(".llmignore"))
        .unwrap()
        .contains("node_modules/"));
}

#[test]
fn list_prints_included_not_secrets() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("main.rs"), "code").unwrap();
    fs::write(root.join(".env"), "SECRET=1").unwrap();

    let out = cmd(root).arg("list").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("main.rs"), "should list main.rs");
    assert!(!stdout.contains(".env"), "must not list ignored .env");
}

#[test]
fn list_ignored_shows_excluded_files() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("main.rs"), "code").unwrap();
    fs::write(root.join("app.log"), "noise").unwrap();

    let out = cmd(root).args(["list", "--ignored"]).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("app.log"),
        "ignored list should show app.log"
    );
    assert!(
        !stdout.contains("main.rs"),
        "included file should not appear in ignored list"
    );
}

#[test]
fn list_json_is_valid() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("main.rs"), "code").unwrap();

    let out = cmd(root).args(["list", "--json"]).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert!(v.get("included").is_some(), "json has included field");
}

#[test]
fn check_included_file_exits_zero() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("main.rs"), "code").unwrap();
    cmd(root).args(["check", "main.rs"]).assert().success();
}

#[test]
fn check_ignored_file_exits_nonzero() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join(".env"), "SECRET=1").unwrap();
    cmd(root).args(["check", ".env"]).assert().failure();
}

#[test]
fn scan_exits_nonzero_when_secret_exposed() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    // user .llmignore that forgot secrets
    fs::write(root.join(".llmignore"), "*.log\n").unwrap();
    fs::write(root.join(".env"), "SECRET=1").unwrap();

    let out = cmd(root).arg("scan").assert().failure();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains(".env"),
        "scan should report the exposed .env"
    );
}

#[test]
fn scan_clean_repo_exits_zero() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("main.rs"), "code").unwrap();
    fs::write(root.join(".env"), "SECRET=1").unwrap(); // covered by defaults
    cmd(root).arg("scan").assert().success();
}

#[test]
fn sync_generates_tool_files() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    cmd(root).arg("init").assert().success();
    cmd(root).arg("sync").assert().success();
    let cursor = fs::read_to_string(root.join(".cursorignore")).unwrap();
    assert!(cursor.contains("node_modules/"));
}
