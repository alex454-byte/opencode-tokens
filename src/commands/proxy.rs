use std::process::Command;
use std::time::Instant;
use crate::filters;
use crate::tracker;

pub fn run(args: &[String], ultra: bool) -> i32 {
    if args.is_empty() {
        return 1;
    }

    let cmd_name = &args[0];
    let cmd_args = &args[1..];

    // Route to specialized filters based on the command
    match cmd_name.as_str() {
        "git" => return filters::git::run(cmd_args, ultra),
        "gh" => return filters::gh::run(cmd_args, ultra),
        "docker" => return filters::docker::run(cmd_args, ultra),
        "kubectl" | "k" => return filters::docker::kubectl(cmd_args, ultra),
        "cargo" if cmd_args.first().map(|s| s.as_str()) == Some("test") => {
            return filters::test::run(args, ultra);
        }
        "pytest" | "go" | "vitest" | "jest" => {
            return filters::test::run(args, ultra);
        }
        "ruff" | "eslint" | "tsc" | "golangci-lint" => {
            return filters::error::run(args, ultra);
        }
        _ => {}
    }

    // Generic passthrough with basic compression
    let start = Instant::now();
    let result = Command::new(cmd_name)
        .args(cmd_args)
        .output();

    match result {
        Ok(output) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let compressed = filters::generic::compress(&raw, ultra);
            tracker::record(
                &args.join(" "),
                &raw,
                &compressed,
                elapsed,
            );

            print!("{compressed}");
            if !stderr.is_empty() {
                eprint!("{stderr}");
            }

            output.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: failed to run '{}': {}", cmd_name, e);
            127
        }
    }
}
