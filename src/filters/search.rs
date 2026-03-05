use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;
use crate::tracker;

pub fn grep(pattern: &str, path: &str, ultra: bool) -> i32 {
    let start = Instant::now();

    // Use ripgrep if available, fall back to grep
    let (cmd, args) = if which("rg") {
        ("rg", vec!["--no-heading", "--line-number", pattern, path])
    } else {
        ("grep", vec!["-rn", pattern, path])
    };

    let output = Command::new(cmd).args(&args).output();

    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);

            // Group results by file
            let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
            let mut order = Vec::new();

            for line in raw.lines() {
                if let Some((file, rest)) = line.split_once(':') {
                    if !grouped.contains_key(file) {
                        order.push(file.to_string());
                    }
                    grouped.entry(file.to_string()).or_default().push(rest.to_string());
                }
            }

            let mut compact = String::new();
            let max_per_file = if ultra { 3 } else { 10 };

            for file in &order {
                if let Some(matches) = grouped.get(file) {
                    compact.push_str(&format!("{}  ({} matches)\n", file, matches.len()));
                    for (i, m) in matches.iter().enumerate() {
                        if i >= max_per_file {
                            compact.push_str(&format!("  ... +{} more\n", matches.len() - max_per_file));
                            break;
                        }
                        compact.push_str(&format!("  {m}\n"));
                    }
                }
            }

            if compact.is_empty() {
                compact = "No matches found.\n".to_string();
            }

            tracker::record(&format!("grep {pattern} {path}"), &raw, &compact, elapsed);
            print!("{compact}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: grep failed: {e}");
            1
        }
    }
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
