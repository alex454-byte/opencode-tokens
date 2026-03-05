use std::process::Command;
use std::time::Instant;
use crate::tracker;

pub fn run(args: &[String], ultra: bool) -> i32 {
    if args.is_empty() {
        return passthrough(args);
    }

    let subcmd = args[0].as_str();
    let rest = &args[1..];

    match subcmd {
        "pr" => gh_pr(rest, ultra),
        "issue" => gh_issue(rest, ultra),
        "run" => gh_run(rest, ultra),
        _ => passthrough(args),
    }
}

fn gh_pr(args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let sub = args.first().map(|s| s.as_str()).unwrap_or("list");

    match sub {
        "list" => {
            let output = Command::new("gh")
                .args(["pr", "list", "--json", "number,title,state,author,updatedAt", "--limit", "20"])
                .output();
            format_json_table(output, start, "gh pr list", ultra,
                &["number", "title", "state"],
                |obj| {
                    let num = obj.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
                    let title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let state = obj.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    let title_short = if ultra && title.len() > 40 {
                        format!("{}...", &title[..37])
                    } else {
                        title.to_string()
                    };
                    format!("#{:<5} {:<45} {}", num, title_short, state)
                })
        }
        _ => {
            let mut cmd_args = vec!["pr".to_string()];
            cmd_args.extend(args.to_vec());
            passthrough_with_compress(&cmd_args, ultra)
        }
    }
}

fn gh_issue(args: &[String], ultra: bool) -> i32 {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("list");
    let start = Instant::now();

    match sub {
        "list" => {
            let output = Command::new("gh")
                .args(["issue", "list", "--json", "number,title,state,labels", "--limit", "20"])
                .output();
            format_json_table(output, start, "gh issue list", ultra,
                &["number", "title", "state"],
                |obj| {
                    let num = obj.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
                    let title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let state = obj.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    format!("#{:<5} {:<45} {}", num, title, state)
                })
        }
        _ => {
            let mut cmd_args = vec!["issue".to_string()];
            cmd_args.extend(args.to_vec());
            passthrough_with_compress(&cmd_args, ultra)
        }
    }
}

fn gh_run(args: &[String], ultra: bool) -> i32 {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("list");
    let start = Instant::now();

    match sub {
        "list" => {
            let output = Command::new("gh")
                .args(["run", "list", "--json", "databaseId,displayTitle,status,conclusion,headBranch", "--limit", "10"])
                .output();
            format_json_table(output, start, "gh run list", ultra,
                &["databaseId", "displayTitle", "status"],
                |obj| {
                    let id = obj.get("databaseId").and_then(|v| v.as_i64()).unwrap_or(0);
                    let title = obj.get("displayTitle").and_then(|v| v.as_str()).unwrap_or("");
                    let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let conclusion = obj.get("conclusion").and_then(|v| v.as_str()).unwrap_or("");
                    let state = if conclusion.is_empty() { status } else { conclusion };
                    format!("{:<10} {:<40} {}", id, title, state)
                })
        }
        _ => {
            let mut cmd_args = vec!["run".to_string()];
            cmd_args.extend(args.to_vec());
            passthrough_with_compress(&cmd_args, ultra)
        }
    }
}

fn format_json_table(
    output: Result<std::process::Output, std::io::Error>,
    start: Instant,
    cmd_name: &str,
    _ultra: bool,
    _headers: &[&str],
    formatter: impl Fn(&serde_json::Value) -> String,
) -> i32 {
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);

            if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
                let mut compact = String::new();
                for item in &items {
                    compact.push_str(&formatter(item));
                    compact.push('\n');
                }
                tracker::record(cmd_name, &raw, &compact, elapsed);
                print!("{compact}");
            } else {
                let compressed = crate::filters::generic::compress(&raw, _ultra);
                tracker::record(cmd_name, &raw, &compressed, elapsed);
                print!("{compressed}");
            }
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: gh failed: {e}");
            1
        }
    }
}

fn passthrough(args: &[String]) -> i32 {
    let output = Command::new("gh").args(args).output();
    match output {
        Ok(out) => {
            print!("{}", String::from_utf8_lossy(&out.stdout));
            eprint!("{}", String::from_utf8_lossy(&out.stderr));
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: gh failed: {e}");
            127
        }
    }
}

fn passthrough_with_compress(args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let output = Command::new("gh").args(args).output();
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);
            let compressed = crate::filters::generic::compress(&raw, ultra);
            tracker::record(&format!("gh {}", args.join(" ")), &raw, &compressed, elapsed);
            print!("{compressed}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: gh failed: {e}");
            127
        }
    }
}
