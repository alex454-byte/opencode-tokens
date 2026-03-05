use std::process::Command;
use std::time::Instant;
use crate::tracker;

pub fn run(cmd: &[String], ultra: bool) -> i32 {
    if cmd.is_empty() {
        eprintln!("Usage: oct test <command> [args...]");
        return 1;
    }

    let start = Instant::now();
    let output = Command::new(&cmd[0])
        .args(&cmd[1..])
        .output();

    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let raw = format!("{stdout}{stderr}");

            let cmd_name = cmd[0].as_str();
            let compressed = match cmd_name {
                "cargo" => filter_cargo_test(&raw, ultra),
                "pytest" | "python" => filter_pytest(&raw, ultra),
                "go" => filter_go_test(&raw, ultra),
                "vitest" | "jest" | "mocha" => filter_js_test(&raw, ultra),
                _ => filter_generic_test(&raw, ultra),
            };

            tracker::record(&cmd.join(" "), &raw, &compressed, elapsed);
            print!("{compressed}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: failed to run '{}': {}", cmd[0], e);
            127
        }
    }
}

fn filter_cargo_test(raw: &str, _ultra: bool) -> String {
    let mut failures = Vec::new();
    let mut summary_line = String::new();
    let mut in_failure = false;
    let mut current_failure = Vec::new();

    for line in raw.lines() {
        if line.starts_with("---- ") && line.ends_with(" ----") {
            if !current_failure.is_empty() {
                failures.push(current_failure.join("\n"));
            }
            current_failure = vec![line.to_string()];
            in_failure = true;
            continue;
        }
        if line.starts_with("test result:") {
            if !current_failure.is_empty() {
                failures.push(current_failure.join("\n"));
                current_failure = Vec::new();
            }
            in_failure = false;
            summary_line = line.to_string();
            continue;
        }
        if in_failure {
            current_failure.push(line.to_string());
        }
    }

    if !current_failure.is_empty() {
        failures.push(current_failure.join("\n"));
    }

    let mut result = String::new();
    if failures.is_empty() {
        if !summary_line.is_empty() {
            result.push_str(&summary_line);
        } else {
            result.push_str("all tests passed");
        }
    } else {
        result.push_str(&format!("{} failure(s):\n\n", failures.len()));
        for f in &failures {
            result.push_str(f);
            result.push_str("\n\n");
        }
        if !summary_line.is_empty() {
            result.push_str(&summary_line);
        }
    }

    result
}

fn filter_pytest(raw: &str, _ultra: bool) -> String {
    let mut failures = Vec::new();
    let mut summary = String::new();
    let mut in_failure = false;
    let mut current = Vec::new();

    for line in raw.lines() {
        if line.starts_with("FAILED ") || line.starts_with("ERRORS ") {
            failures.push(line.to_string());
            continue;
        }
        if line.starts_with("___ ") || line.starts_with("=== FAILURES") {
            if !current.is_empty() {
                failures.push(current.join("\n"));
                current = Vec::new();
            }
            in_failure = true;
            continue;
        }
        if line.starts_with("=== ") && (line.contains("passed") || line.contains("failed") || line.contains("error")) {
            summary = line.to_string();
            in_failure = false;
            continue;
        }
        if in_failure {
            current.push(line.to_string());
        }
    }

    if !current.is_empty() {
        failures.push(current.join("\n"));
    }

    let mut result = String::new();
    if failures.is_empty() {
        result.push_str(if summary.is_empty() { "all tests passed" } else { &summary });
    } else {
        for f in &failures {
            result.push_str(f);
            result.push('\n');
        }
        if !summary.is_empty() {
            result.push_str(&summary);
        }
    }
    result
}

fn filter_go_test(raw: &str, _ultra: bool) -> String {
    let mut failures = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;

    for line in raw.lines() {
        if line.starts_with("--- FAIL:") {
            failed += 1;
            failures.push(line.to_string());
        } else if line.starts_with("--- PASS:") {
            passed += 1;
        } else if line.contains("FAIL") && !line.starts_with("---") {
            failures.push(line.to_string());
        }
    }

    let mut result = String::new();
    if failures.is_empty() {
        result.push_str(&format!("ok: {passed} passed"));
    } else {
        result.push_str(&format!("{failed} failed, {passed} passed:\n\n"));
        for f in &failures {
            result.push_str(f);
            result.push('\n');
        }
    }
    result
}

fn filter_js_test(raw: &str, _ultra: bool) -> String {
    let mut failures = Vec::new();
    let mut summary = String::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.contains("FAIL") || trimmed.starts_with("✕") || trimmed.starts_with("×") || trimmed.starts_with("✗") {
            failures.push(trimmed.to_string());
        }
        if trimmed.contains("Tests:") || trimmed.contains("Test Suites:") {
            summary.push_str(trimmed);
            summary.push('\n');
        }
    }

    if failures.is_empty() {
        if summary.is_empty() { "all tests passed".to_string() } else { summary }
    } else {
        let mut result = String::new();
        for f in &failures {
            result.push_str(f);
            result.push('\n');
        }
        result.push_str(&summary);
        result
    }
}

fn filter_generic_test(raw: &str, ultra: bool) -> String {
    // Keep lines with: FAIL, ERROR, WARN, panic, assert, error
    let keywords = ["FAIL", "ERROR", "error", "WARN", "panic", "assert", "traceback", "Exception"];

    let mut important: Vec<&str> = raw
        .lines()
        .filter(|l| keywords.iter().any(|k| l.contains(k)))
        .collect();

    if important.is_empty() {
        // All passed — just show last few lines (usually the summary)
        let lines: Vec<&str> = raw.lines().collect();
        let start = lines.len().saturating_sub(5);
        return lines[start..].join("\n");
    }

    let max = if ultra { 20 } else { 50 };
    if important.len() > max {
        important.truncate(max);
        important.push("... (truncated)");
    }

    important.join("\n")
}
