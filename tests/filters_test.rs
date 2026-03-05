use std::process::Command;

fn oct() -> Command {
    Command::new(env!("CARGO_BIN_EXE_oct"))
}

#[test]
fn help_works() {
    let output = oct().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Token reduction proxy"));
}

#[test]
fn ls_current_dir() {
    let output = oct().args(["ls", "."]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("src/"));
    assert!(stdout.contains("Cargo.toml"));
}

#[test]
fn read_file_normal() {
    let output = oct().args(["read", "Cargo.toml"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("opencode-tokens"));
}

#[test]
fn read_file_aggressive() {
    let output = oct()
        .args(["read", "src/main.rs", "-l", "aggressive"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Aggressive mode should extract signatures
    assert!(stdout.contains("fn main"));
}

#[test]
fn gain_works() {
    let output = oct().args(["gain"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn gain_json_export() {
    let output = oct().args(["gain", "--format", "json"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.contains("total_commands") || stdout.contains("{}") || stdout.is_empty());
}

#[test]
fn git_status_compressed() {
    // Only run if we're in a git repo
    let git_check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output();
    if git_check.map(|o| o.status.success()).unwrap_or(false) {
        let output = oct().args(["git", "status"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should have compact format with "branch:" prefix
        assert!(stdout.contains("branch:"));
    }
}

#[test]
fn unknown_command_passthrough() {
    let output = oct().args(["echo", "hello"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello"));
}

#[test]
fn ultra_mode_flag() {
    let output = oct().args(["-u", "ls", "."]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn init_local() {
    // Run in a temp directory
    let tmp = std::env::temp_dir().join("oct-test-init");
    std::fs::create_dir_all(&tmp).ok();

    let output = oct()
        .args(["init"])
        .current_dir(&tmp)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(tmp.join("OCT.md").exists());

    // Cleanup
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn summary_echo() {
    let output = oct()
        .args(["summary", "echo", "hello world"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lines") || stdout.contains("hello"));
}
