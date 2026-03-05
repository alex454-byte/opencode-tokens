use std::process::Command;
use std::time::Instant;
use crate::tracker;

pub fn run(cmd: &[String], ultra: bool) -> i32 {
    if cmd.is_empty() {
        eprintln!("Usage: oct err <command> [args...]");
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
                "ruff" => filter_ruff(&raw, ultra),
                "eslint" => filter_eslint(&raw, ultra),
                "tsc" => filter_tsc(&raw, ultra),
                "golangci-lint" => filter_golangci(&raw, ultra),
                _ => filter_generic_errors(&raw, ultra),
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

fn filter_ruff(raw: &str, _ultra: bool) -> String {
    // Group by rule code
    let mut by_rule: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut order = Vec::new();

    for line in raw.lines() {
        // Format: file.py:10:5: E501 line too long
        if let Some(pos) = line.find(": ") {
            let location = &line[..pos];
            let rest = &line[pos + 2..];
            let rule = rest.split_whitespace().next().unwrap_or("unknown");
            if !by_rule.contains_key(rule) {
                order.push(rule.to_string());
            }
            by_rule.entry(rule.to_string()).or_default().push(location.to_string());
        }
    }

    if by_rule.is_empty() {
        return "no issues".to_string();
    }

    let mut result = String::new();
    for rule in &order {
        if let Some(locations) = by_rule.get(rule) {
            result.push_str(&format!("{rule} ({} occurrences)\n", locations.len()));
            for loc in locations.iter().take(3) {
                result.push_str(&format!("  {loc}\n"));
            }
            if locations.len() > 3 {
                result.push_str(&format!("  ... +{} more\n", locations.len() - 3));
            }
        }
    }
    result
}

fn filter_eslint(raw: &str, _ultra: bool) -> String {
    // Group errors by file
    let mut current_file = String::new();
    let mut file_errors: Vec<String> = Vec::new();
    let mut result = String::new();
    let mut total = 0u32;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        // File header lines don't start with spaces
        if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.starts_with("✖") {
            if !current_file.is_empty() && !file_errors.is_empty() {
                result.push_str(&format!("{} ({} issues)\n", current_file, file_errors.len()));
                for e in file_errors.iter().take(5) {
                    result.push_str(&format!("  {e}\n"));
                }
                if file_errors.len() > 5 {
                    result.push_str(&format!("  ... +{} more\n", file_errors.len() - 5));
                }
            }
            current_file = trimmed.to_string();
            file_errors = Vec::new();
        } else if trimmed.starts_with("✖") || trimmed.contains("problem") {
            result.push_str(&format!("\n{trimmed}\n"));
        } else {
            total += 1;
            file_errors.push(trimmed.to_string());
        }
    }

    // Flush last file
    if !current_file.is_empty() && !file_errors.is_empty() {
        result.push_str(&format!("{} ({} issues)\n", current_file, file_errors.len()));
        for e in file_errors.iter().take(5) {
            result.push_str(&format!("  {e}\n"));
        }
    }

    if total == 0 && result.is_empty() {
        "no issues".to_string()
    } else {
        result
    }
}

fn filter_tsc(raw: &str, _ultra: bool) -> String {
    // Group TypeScript errors by file
    let mut by_file: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut order = Vec::new();

    for line in raw.lines() {
        // Format: src/file.ts(10,5): error TS2345: ...
        if let Some(paren_pos) = line.find('(') {
            let file = &line[..paren_pos];
            if !by_file.contains_key(file) {
                order.push(file.to_string());
            }
            by_file.entry(file.to_string()).or_default().push(line.to_string());
        }
    }

    if by_file.is_empty() {
        return if raw.contains("error") {
            crate::filters::generic::compress(raw, false)
        } else {
            "no errors".to_string()
        };
    }

    let mut result = String::new();
    let total: usize = by_file.values().map(|v| v.len()).sum();
    result.push_str(&format!("{total} error(s) in {} file(s):\n\n", order.len()));

    for file in &order {
        if let Some(errors) = by_file.get(file) {
            result.push_str(&format!("{file} ({} errors)\n", errors.len()));
            for e in errors.iter().take(5) {
                // Just show the error code and message
                if let Some(err_pos) = e.find("error TS") {
                    result.push_str(&format!("  {}\n", &e[err_pos..]));
                } else {
                    result.push_str(&format!("  {e}\n"));
                }
            }
            if errors.len() > 5 {
                result.push_str(&format!("  ... +{} more\n", errors.len() - 5));
            }
        }
    }
    result
}

fn filter_golangci(raw: &str, _ultra: bool) -> String {
    // Group by linter name
    let mut by_linter: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut order = Vec::new();

    for line in raw.lines() {
        // Format: file.go:10:5: message (linter)
        if let Some(paren_start) = line.rfind('(') {
            if let Some(paren_end) = line.rfind(')') {
                let linter = &line[paren_start + 1..paren_end];
                if !by_linter.contains_key(linter) {
                    order.push(linter.to_string());
                }
                by_linter.entry(linter.to_string()).or_default().push(line.to_string());
            }
        }
    }

    if by_linter.is_empty() {
        return "no issues".to_string();
    }

    let mut result = String::new();
    for linter in &order {
        if let Some(issues) = by_linter.get(linter) {
            result.push_str(&format!("{linter} ({} issues)\n", issues.len()));
            for issue in issues.iter().take(3) {
                result.push_str(&format!("  {issue}\n"));
            }
            if issues.len() > 3 {
                result.push_str(&format!("  ... +{} more\n", issues.len() - 3));
            }
        }
    }
    result
}

fn filter_generic_errors(raw: &str, ultra: bool) -> String {
    let error_keywords = ["error", "Error", "ERROR", "warning", "Warning", "WARN", "fatal", "Fatal"];

    let important: Vec<&str> = raw
        .lines()
        .filter(|l| error_keywords.iter().any(|k| l.contains(k)))
        .collect();

    if important.is_empty() {
        return "no errors or warnings".to_string();
    }

    let max = if ultra { 20 } else { 50 };
    let mut result = String::new();
    for (i, line) in important.iter().enumerate() {
        if i >= max {
            result.push_str(&format!("... +{} more\n", important.len() - max));
            break;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}
