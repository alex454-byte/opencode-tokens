use std::process::Command;
use std::time::Instant;
use crate::tracker;

pub fn run(args: &[String], ultra: bool) -> i32 {
    if args.is_empty() {
        return passthrough(&["git"], ultra);
    }

    let subcmd = args[0].as_str();
    let rest = &args[1..];

    match subcmd {
        "status" => git_status(rest, ultra),
        "log" => git_log(rest, ultra),
        "diff" => git_diff(rest, ultra),
        "add" | "commit" | "push" | "pull" | "fetch" | "checkout" | "switch" | "merge" | "rebase" | "stash" => {
            git_action(subcmd, rest, ultra)
        }
        _ => passthrough(&build_args("git", args), ultra),
    }
}

fn git_status(_args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "--branch"])
        .output();

    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);

            let mut branch = String::new();
            let mut staged = Vec::new();
            let mut modified = Vec::new();
            let mut untracked = Vec::new();

            for line in raw.lines() {
                if line.starts_with("## ") {
                    branch = line[3..].to_string();
                    continue;
                }
                if line.len() < 4 { continue; }

                let xy = &line[..2];
                let file = &line[3..];

                match xy.as_bytes() {
                    [b'?', b'?'] => untracked.push(file),
                    [x, _] if *x != b' ' => staged.push(file),
                    [_, y] if *y != b' ' => modified.push(file),
                    _ => modified.push(file),
                }
            }

            let mut compact = format!("branch: {branch}\n");
            if !staged.is_empty() {
                compact.push_str(&format!("staged ({}): {}\n", staged.len(), abbrev_list(&staged, ultra)));
            }
            if !modified.is_empty() {
                compact.push_str(&format!("modified ({}): {}\n", modified.len(), abbrev_list(&modified, ultra)));
            }
            if !untracked.is_empty() {
                compact.push_str(&format!("untracked ({}): {}\n", untracked.len(), abbrev_list(&untracked, ultra)));
            }
            if staged.is_empty() && modified.is_empty() && untracked.is_empty() {
                compact.push_str("clean\n");
            }

            tracker::record("git status", &raw, &compact, elapsed);
            print!("{compact}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: git status failed: {e}");
            1
        }
    }
}

fn git_log(args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let mut cmd_args = vec!["log", "--oneline", "--no-decorate"];

    // Check if -n is already specified
    let has_limit = args.iter().any(|a| a.starts_with("-n") || a.starts_with("--max-count"));
    if !has_limit {
        cmd_args.push("-n");
        cmd_args.push("10");
    }

    let extra: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    cmd_args.extend(extra);

    let output = Command::new("git").args(&cmd_args).output();
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);
            let compact = if ultra {
                // Even shorter: just hash + first 50 chars
                raw.lines()
                    .map(|l| {
                        if l.len() > 60 { format!("{}...", &l[..57]) } else { l.to_string() }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                raw.to_string()
            };
            tracker::record("git log", &raw, &compact, elapsed);
            println!("{compact}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: git log failed: {e}");
            1
        }
    }
}

fn git_diff(args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let mut cmd_args = vec!["diff".to_string(), "--stat".to_string()];
    cmd_args.extend(args.to_vec());

    let output = Command::new("git")
        .args(&cmd_args)
        .output();

    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw_stat = String::from_utf8_lossy(&out.stdout);

            // Also get the actual diff but truncated
            let diff_output = Command::new("git")
                .args(["diff"])
                .args(args)
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();

            let max_diff_lines = if ultra { 30 } else { 100 };
            let diff_lines: Vec<&str> = diff_output.lines().take(max_diff_lines).collect();
            let truncated = diff_output.lines().count() > max_diff_lines;

            let mut compact = raw_stat.to_string();
            if !diff_lines.is_empty() {
                compact.push('\n');
                compact.push_str(&diff_lines.join("\n"));
                if truncated {
                    compact.push_str("\n... (diff truncated)");
                }
            }

            tracker::record("git diff", &diff_output, &compact, elapsed);
            print!("{compact}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: git diff failed: {e}");
            1
        }
    }
}

fn git_action(subcmd: &str, args: &[String], _ultra: bool) -> i32 {
    let start = Instant::now();
    let output = Command::new("git")
        .arg(subcmd)
        .args(args)
        .output();

    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );

            let code = out.status.code().unwrap_or(1);
            let compact = if code == 0 {
                // Extract useful bits from success output
                let short = extract_action_summary(subcmd, &raw);
                format!("ok {short}")
            } else {
                // On failure, show the error
                raw.clone()
            };

            tracker::record(&format!("git {subcmd}"), &raw, &compact, elapsed);
            println!("{compact}");
            code
        }
        Err(e) => {
            eprintln!("oct: git {subcmd} failed: {e}");
            1
        }
    }
}

fn extract_action_summary(subcmd: &str, raw: &str) -> String {
    match subcmd {
        "commit" => {
            // Extract short hash from commit output
            raw.lines()
                .find(|l| l.contains('['))
                .map(|l| {
                    l.split_whitespace()
                        .find(|w| w.len() >= 7 && w.chars().all(|c| c.is_ascii_hexdigit() || c == ']'))
                        .unwrap_or("")
                        .trim_end_matches(']')
                        .to_string()
                })
                .unwrap_or_default()
        }
        "push" | "pull" => {
            // Extract branch name
            raw.lines()
                .find(|l| l.contains("->") || l.contains("branch"))
                .map(|l| l.trim().to_string())
                .unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn abbrev_list(items: &[&str], ultra: bool) -> String {
    let max = if ultra { 5 } else { 15 };
    if items.len() <= max {
        items.join(", ")
    } else {
        let shown: Vec<&str> = items[..max].to_vec();
        format!("{}, ... +{} more", shown.join(", "), items.len() - max)
    }
}

fn build_args<'a>(cmd: &'a str, args: &'a [String]) -> Vec<&'a str> {
    let mut v = vec![cmd];
    for a in args {
        v.push(a.as_str());
    }
    v
}

fn passthrough(args: &[&str], _ultra: bool) -> i32 {
    if args.is_empty() { return 1; }
    let output = Command::new(args[0]).args(&args[1..]).output();
    match output {
        Ok(out) => {
            let raw = String::from_utf8_lossy(&out.stdout);
            let compressed = crate::filters::generic::compress(&raw, _ultra);
            tracker::record(&args.join(" "), &raw, &compressed, 0);
            print!("{compressed}");
            if !out.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&out.stderr));
            }
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: {} failed: {e}", args[0]);
            127
        }
    }
}
