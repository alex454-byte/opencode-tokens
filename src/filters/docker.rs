use std::process::Command;
use std::time::Instant;
use crate::tracker;
use crate::filters::generic;

pub fn run(args: &[String], ultra: bool) -> i32 {
    if args.is_empty() {
        return passthrough("docker", args, ultra);
    }

    let subcmd = args[0].as_str();
    let rest = &args[1..];

    match subcmd {
        "ps" => docker_ps(rest, ultra),
        "images" => docker_images(rest, ultra),
        "logs" => docker_logs(rest, ultra),
        _ => passthrough("docker", args, ultra),
    }
}

pub fn kubectl(args: &[String], ultra: bool) -> i32 {
    if args.is_empty() {
        return passthrough("kubectl", args, ultra);
    }

    let subcmd = args[0].as_str();
    let rest = &args[1..];

    match subcmd {
        "get" => kubectl_get(rest, ultra),
        "logs" => kubectl_logs(rest, ultra),
        _ => passthrough("kubectl", args, ultra),
    }
}

fn docker_ps(args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let mut cmd_args = vec!["ps", "--format", "{{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Image}}"];
    let extra: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    cmd_args.extend(extra);

    let output = Command::new("docker").args(&cmd_args).output();
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);

            let mut compact = String::new();
            compact.push_str(&format!("{:<12} {:<25} {:<15} {}\n", "ID", "NAME", "STATUS", "IMAGE"));
            for line in raw.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 4 {
                    let id = &parts[0][..12.min(parts[0].len())];
                    let name = if ultra && parts[1].len() > 20 {
                        &parts[1][..20]
                    } else {
                        parts[1]
                    };
                    compact.push_str(&format!("{:<12} {:<25} {:<15} {}\n", id, name, parts[2], parts[3]));
                }
            }

            tracker::record("docker ps", &raw, &compact, elapsed);
            print!("{compact}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: docker ps failed: {e}");
            1
        }
    }
}

fn docker_images(args: &[String], _ultra: bool) -> i32 {
    let start = Instant::now();
    let mut cmd_args = vec!["images", "--format", "{{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"];
    let extra: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    cmd_args.extend(extra);

    let output = Command::new("docker").args(&cmd_args).output();
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);

            let mut compact = String::new();
            compact.push_str(&format!("{:<45} {:>10} {}\n", "IMAGE", "SIZE", "CREATED"));
            for line in raw.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    compact.push_str(&format!("{:<45} {:>10} {}\n", parts[0], parts[1], parts[2]));
                }
            }

            tracker::record("docker images", &raw, &compact, elapsed);
            print!("{compact}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: docker images failed: {e}");
            1
        }
    }
}

fn docker_logs(args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let output = Command::new("docker").arg("logs").args(args).output();
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let compressed = crate::filters::log::run_on_string(&raw, ultra);
            tracker::record("docker logs", &raw, &compressed, elapsed);
            print!("{compressed}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: docker logs failed: {e}");
            1
        }
    }
}

fn kubectl_get(args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let output = Command::new("kubectl").arg("get").args(args).output();
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);
            let compressed = generic::compress(&raw, ultra);
            tracker::record(&format!("kubectl get {}", args.join(" ")), &raw, &compressed, elapsed);
            print!("{compressed}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: kubectl get failed: {e}");
            1
        }
    }
}

fn kubectl_logs(args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let output = Command::new("kubectl").arg("logs").args(args).output();
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let compressed = crate::filters::log::run_on_string(&raw, ultra);
            tracker::record("kubectl logs", &raw, &compressed, elapsed);
            print!("{compressed}");
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: kubectl logs failed: {e}");
            1
        }
    }
}

fn passthrough(bin: &str, args: &[String], ultra: bool) -> i32 {
    let start = Instant::now();
    let output = Command::new(bin).args(args).output();
    match output {
        Ok(out) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let raw = String::from_utf8_lossy(&out.stdout);
            let compressed = generic::compress(&raw, ultra);
            tracker::record(&format!("{bin} {}", args.join(" ")), &raw, &compressed, elapsed);
            print!("{compressed}");
            if !out.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&out.stderr));
            }
            out.status.code().unwrap_or(1)
        }
        Err(e) => {
            eprintln!("oct: {bin} failed: {e}");
            127
        }
    }
}
