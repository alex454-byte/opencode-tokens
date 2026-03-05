use std::process::Command;
use std::time::Instant;
use crate::tracker;

pub fn run(cmd: &[String], ultra: bool) -> i32 {
    if cmd.is_empty() {
        eprintln!("Usage: oct summary <command> [args...]");
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

            let summary = heuristic_summary(&raw, ultra);
            tracker::record(&cmd.join(" "), &raw, &summary, elapsed);
            print!("{summary}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: failed to run '{}': {}", cmd[0], e);
            127
        }
    }
}

fn heuristic_summary(raw: &str, ultra: bool) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();

    if total == 0 {
        return "(no output)\n".to_string();
    }

    let mut result = Vec::new();

    // Count patterns
    let errors = lines.iter().filter(|l| l.contains("error") || l.contains("Error") || l.contains("ERROR")).count();
    let warnings = lines.iter().filter(|l| l.contains("warn") || l.contains("Warn") || l.contains("WARN")).count();
    let successes = lines.iter().filter(|l| l.contains("ok") || l.contains("pass") || l.contains("success") || l.contains("✓")).count();

    // Status line
    if errors > 0 || warnings > 0 {
        result.push(format!("{total} lines: {errors} errors, {warnings} warnings, {successes} ok"));
    } else {
        result.push(format!("{total} lines: all ok ({successes} success indicators)"));
    }

    // First meaningful line (often a header/title)
    if let Some(first) = lines.iter().find(|l| !l.trim().is_empty()) {
        let truncated = if first.len() > 80 { &first[..77] } else { first };
        result.push(format!("first: {truncated}"));
    }

    // Last meaningful line (often a summary)
    if let Some(last) = lines.iter().rev().find(|l| !l.trim().is_empty()) {
        let truncated = if last.len() > 80 { &last[..77] } else { last };
        result.push(format!("last:  {truncated}"));
    }

    // If errors, show first few
    if errors > 0 {
        let max = if ultra { 3 } else { 5 };
        result.push(String::new());
        result.push("errors:".to_string());
        let error_lines: Vec<&&str> = lines.iter()
            .filter(|l| l.contains("error") || l.contains("Error") || l.contains("ERROR"))
            .take(max)
            .collect();
        for el in &error_lines {
            result.push(format!("  {el}"));
        }
        if errors > max {
            result.push(format!("  ... +{} more", errors - max));
        }
    }

    result.join("\n") + "\n"
}
