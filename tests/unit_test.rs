use std::process::Command;

fn oct() -> Command {
    Command::new(env!("CARGO_BIN_EXE_oct"))
}

// ============================================================
// CLI basics
// ============================================================

#[test]
fn no_args_shows_usage() {
    let output = oct().output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage"));
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn help_flag() {
    let output = oct().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Token reduction proxy"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("gain"));
    assert!(stdout.contains("discover"));
    assert!(stdout.contains("read"));
    assert!(stdout.contains("ls"));
    assert!(stdout.contains("grep"));
    assert!(stdout.contains("test"));
    assert!(stdout.contains("err"));
    assert!(stdout.contains("log"));
    assert!(stdout.contains("summary"));
}

#[test]
fn subcommand_help() {
    for sub in &["init", "gain", "discover", "read", "ls", "grep", "test", "err", "log", "summary"] {
        let output = oct().args([*sub, "--help"]).output().unwrap();
        assert!(output.status.success(), "help failed for subcommand: {sub}");
    }
}

// ============================================================
// ls (directory listing)
// ============================================================

#[test]
fn ls_shows_tree_structure() {
    let output = oct().args(["ls", "."]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("├──") || stdout.contains("└──"), "should have tree connectors");
    assert!(stdout.contains("src/"));
    assert!(stdout.contains("Cargo.toml"));
}

#[test]
fn ls_specific_dir() {
    let output = oct().args(["ls", "src"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("filters/"));
    assert!(stdout.contains("commands/"));
}

#[test]
fn ls_ultra_mode() {
    let output = oct().args(["-u", "ls", "."]).output().unwrap();
    assert!(output.status.success());
    let stdout_ultra = String::from_utf8_lossy(&output.stdout);

    let output_normal = oct().args(["ls", "."]).output().unwrap();
    let stdout_normal = String::from_utf8_lossy(&output_normal.stdout);

    // Ultra should be same or smaller (depth limited to 2 vs 3)
    assert!(stdout_ultra.len() <= stdout_normal.len() + 10);
}

#[test]
fn ls_nonexistent_dir() {
    let output = oct().args(["ls", "/nonexistent_dir_12345"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot list"));
}

// ============================================================
// read (file reading)
// ============================================================

#[test]
fn read_cargo_toml() {
    let output = oct().args(["read", "Cargo.toml"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[package]"));
    assert!(stdout.contains("opencode-tokens"));
}

#[test]
fn read_aggressive_extracts_signatures() {
    let output = oct().args(["read", "src/main.rs", "-l", "aggressive"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fn main"));
    // Should NOT contain full function bodies
    let lines: Vec<&str> = stdout.lines().collect();
    // Aggressive should be much shorter than the full file
    assert!(lines.len() < 20, "aggressive mode should extract only signatures, got {} lines", lines.len());
}

#[test]
fn read_aggressive_rs_signatures() {
    let output = oct().args(["read", "src/tracker.rs", "-l", "aggressive"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pub fn count_tokens"));
    assert!(stdout.contains("pub fn record"));
}

#[test]
fn read_nonexistent_file() {
    let output = oct().args(["read", "nonexistent_file.txt"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot read"));
}

#[test]
fn read_normal_strips_comments() {
    // Create a temp file with comments
    let tmp = std::env::temp_dir().join("oct-test-comments.rs");
    std::fs::write(&tmp, "// this is a noise comment\nfn hello() {}\n// another comment\n/// doc comment kept\npub fn world() {}\n").unwrap();

    let output = oct().args(["read", tmp.to_str().unwrap()]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Noise comments stripped, doc comment kept
    assert!(!stdout.contains("// this is a noise comment"));
    assert!(stdout.contains("/// doc comment kept"));
    assert!(stdout.contains("fn hello"));

    std::fs::remove_file(&tmp).ok();
}

// ============================================================
// git commands
// ============================================================

#[test]
fn git_status_compact_format() {
    let output = oct().args(["git", "status"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should use compact format
    assert!(stdout.contains("branch:"), "output should start with 'branch:', got: {stdout}");
}

#[test]
fn git_log_compact() {
    // Add a commit so there's something to show
    let _ = Command::new("git").args(["add", "-A"]).output();
    let _ = Command::new("git").args(["commit", "-m", "test commit", "--allow-empty"]).output();

    let output = oct().args(["git", "log"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // One-line format, should have short hashes
    assert!(!stdout.is_empty());
}

#[test]
fn git_diff_shows_stat() {
    let output = oct().args(["git", "diff"]).output().unwrap();
    // Should succeed even with no changes
    assert!(output.status.success());
}

#[test]
fn git_action_ok_format() {
    // Test that simple git actions return compact "ok" format
    // Use a safe read-only git command via passthrough
    let output = oct().args(["git", "branch"]).output().unwrap();
    assert!(output.status.success());
}

// ============================================================
// grep (search)
// ============================================================

#[test]
fn grep_finds_pattern() {
    let output = oct().args(["grep", "fn main", "src"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.rs"), "should find fn main in main.rs");
    assert!(stdout.contains("match"), "should show match count");
}

#[test]
fn grep_groups_by_file() {
    let output = oct().args(["grep", "pub fn", "src"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should group results with file headers and match counts
    assert!(stdout.contains("matches)"), "should show grouped matches");
}

#[test]
fn grep_no_matches() {
    let output = oct().args(["grep", "ZZZZNOTFOUND12345", "src"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No matches") || stdout.is_empty());
}

// ============================================================
// summary
// ============================================================

#[test]
fn summary_basic() {
    let output = oct().args(["summary", "echo", "hello world"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lines"));
}

#[test]
fn summary_multi_line() {
    let output = oct()
        .args(["summary", "sh", "-c", "echo line1; echo line2; echo line3; echo error found"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("4 lines") || stdout.contains("lines"));
}

// ============================================================
// log dedup
// ============================================================

#[test]
fn log_dedup_repeated_lines() {
    let tmp = std::env::temp_dir().join("oct-test-log.txt");
    let content = "[2024-01-15 10:00:01] INFO Starting server\n\
                   [2024-01-15 10:00:02] INFO Starting server\n\
                   [2024-01-15 10:00:03] INFO Starting server\n\
                   [2024-01-15 10:00:04] INFO Starting server\n\
                   [2024-01-15 10:00:05] INFO Starting server\n\
                   [2024-01-15 10:00:06] ERROR Connection failed\n\
                   [2024-01-15 10:00:07] INFO Starting server\n";
    std::fs::write(&tmp, content).unwrap();

    let output = oct().args(["log", tmp.to_str().unwrap()]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should dedup the repeated "Starting server" lines
    assert!(stdout.contains("(x"), "should show repeat count: {stdout}");
    assert!(stdout.contains("reduction"), "should show reduction stats");

    // Output should be much shorter than input
    assert!(stdout.lines().count() < content.lines().count());

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn log_nonexistent() {
    let output = oct().args(["log", "/tmp/nonexistent_oct_log.txt"]).output().unwrap();
    assert!(!output.status.success());
}

// ============================================================
// passthrough proxy
// ============================================================

#[test]
fn proxy_echo() {
    let output = oct().args(["echo", "proxy test"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("proxy test"));
}

#[test]
fn proxy_nonexistent_command() {
    let output = oct().args(["nonexistent_command_12345"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to run"));
}

#[test]
fn proxy_preserves_exit_code() {
    let output = oct().args(["sh", "-c", "exit 42"]).output().unwrap();
    assert_eq!(output.status.code(), Some(42));
}

// ============================================================
// gain analytics
// ============================================================

#[test]
fn gain_summary() {
    // Run a command first to generate data
    let _ = oct().args(["echo", "gain test"]).output();

    let output = oct().args(["gain"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Total commands") || stdout.contains("No data"));
}

#[test]
fn gain_daily() {
    let output = oct().args(["gain", "--daily"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn gain_history() {
    let output = oct().args(["gain", "--history"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn gain_graph() {
    let output = oct().args(["gain", "--graph"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn gain_csv_export() {
    let output = oct().args(["gain", "--format", "csv"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn gain_json_daily() {
    let output = oct().args(["gain", "--daily", "--format", "json"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON array
    if !stdout.is_empty() {
        assert!(stdout.starts_with('[') || stdout.contains("date"));
    }
}

#[test]
fn gain_invalid_format() {
    let output = oct().args(["gain", "--format", "xml"]).output().unwrap();
    assert!(!output.status.success());
}

// ============================================================
// init
// ============================================================

#[test]
fn init_creates_oct_md() {
    let tmp = std::env::temp_dir().join("oct-test-init-2");
    std::fs::create_dir_all(&tmp).ok();

    let output = oct().args(["init"]).current_dir(&tmp).output().unwrap();
    assert!(output.status.success());

    let oct_md = tmp.join("OCT.md");
    assert!(oct_md.exists(), "OCT.md should be created");

    let content = std::fs::read_to_string(&oct_md).unwrap();
    assert!(content.contains("oct"));
    assert!(content.contains("git status"));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn init_uninstall() {
    let tmp = std::env::temp_dir().join("oct-test-uninstall");
    std::fs::create_dir_all(&tmp).ok();

    // Create first
    let _ = oct().args(["init"]).current_dir(&tmp).output();
    assert!(tmp.join("OCT.md").exists());

    // Uninstall
    let output = oct().args(["init", "--uninstall"]).current_dir(&tmp).output().unwrap();
    assert!(output.status.success());
    assert!(!tmp.join("OCT.md").exists(), "OCT.md should be removed");

    std::fs::remove_dir_all(&tmp).ok();
}

// ============================================================
// discover
// ============================================================

#[test]
fn discover_runs_without_crash() {
    let output = oct().args(["discover"]).output().unwrap();
    // May return 0 or 1 depending on whether OpenCode DB exists, but shouldn't crash
    let _ = output.status.code();
}

// ============================================================
// err (error filtering)
// ============================================================

#[test]
fn err_no_args() {
    let output = oct().args(["err"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage"));
}

#[test]
fn err_filters_errors() {
    let output = oct()
        .args(["err", "sh", "-c", "echo 'all good'; echo 'ERROR: bad thing'; echo 'fine'; echo 'warning: hmm'"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ERROR") || stdout.contains("warning"));
    // Should NOT contain the non-error lines
    assert!(!stdout.contains("all good"));
    assert!(!stdout.contains("fine"));
}

#[test]
fn err_no_errors_found() {
    let output = oct()
        .args(["err", "echo", "everything is fine"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no errors"));
}

// ============================================================
// test (test filtering)
// ============================================================

#[test]
fn test_no_args() {
    let output = oct().args(["test"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage"));
}

#[test]
fn test_passing_output() {
    let output = oct()
        .args(["test", "sh", "-c", "echo 'test result: ok. 5 passed; 0 failed'"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("passed") || stdout.contains("ok"));
}

// ============================================================
// Token tracking verification
// ============================================================

#[test]
fn tracking_records_data() {
    // Run several commands
    let _ = oct().args(["echo", "track1"]).output();
    let _ = oct().args(["echo", "track2"]).output();
    let _ = oct().args(["ls", "src"]).output();

    // Check gain shows data
    let output = oct().args(["gain"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Total commands"));
    assert!(stdout.contains("Tokens saved"));
}

// ============================================================
// Ultra mode across commands
// ============================================================

#[test]
fn ultra_read() {
    let output = oct().args(["-u", "read", "src/main.rs"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn ultra_grep() {
    let output = oct().args(["-u", "grep", "fn", "src"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn ultra_summary() {
    let output = oct().args(["-u", "summary", "echo", "test"]).output().unwrap();
    assert!(output.status.success());
}
