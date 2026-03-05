use std::collections::HashMap;
use std::fs;
use std::time::Instant;
use crate::tracker;

pub fn run(path: &str, ultra: bool) -> i32 {
    let start = Instant::now();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("oct: cannot read '{}': {}", path, e);
            return 1;
        }
    };

    let elapsed = start.elapsed().as_millis() as u64;
    let compressed = dedup_log(&content, ultra);

    tracker::record(&format!("log {path}"), &content, &compressed, elapsed);
    print!("{compressed}");
    0
}

pub fn run_on_string(content: &str, ultra: bool) -> String {
    dedup_log(content, ultra)
}

fn dedup_log(content: &str, ultra: bool) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for line in &lines {
        // Normalize: strip timestamps and varying numbers for dedup key
        let key = normalize_log_line(line);

        let count = seen.entry(key.clone()).or_insert(0);
        *count += 1;

        if *count == 1 {
            order.push(key);
            result.push(line.to_string());
        }
    }

    // Append repeat counts
    let mut output = Vec::new();
    for (i, line) in result.iter().enumerate() {
        if i < order.len() {
            let count = seen.get(&order[i]).copied().unwrap_or(1);
            if count > 1 {
                output.push(format!("{line}  (x{count})"));
            } else {
                output.push(line.clone());
            }
        }
    }

    let max = if ultra { 50 } else { 200 };
    if output.len() > max {
        let omitted = output.len() - max;
        output.truncate(max);
        output.push(format!("... ({omitted} unique lines omitted)"));
    }

    let total = lines.len();
    let unique = output.len();
    output.push(format!("\n[{total} lines -> {unique} unique ({:.0}% reduction)]",
        (1.0 - unique as f64 / total.max(1) as f64) * 100.0));

    output.join("\n")
}

fn normalize_log_line(line: &str) -> String {
    // Strip common timestamp patterns for dedup comparison
    let mut normalized = line.to_string();

    // ISO timestamps: 2024-01-15T10:30:45.123Z
    if let Some(pos) = normalized.find('T') {
        if pos >= 8 && pos <= 12 {
            if let Some(end) = normalized[pos..].find(|c: char| c == ' ' || c == ']') {
                normalized = normalized[pos + end..].to_string();
            }
        }
    }

    // Common log prefixes: [2024-01-15 10:30:45]
    if normalized.starts_with('[') {
        if let Some(end) = normalized.find(']') {
            normalized = normalized[end + 1..].to_string();
        }
    }

    // Normalize numbers (IDs, ports, PIDs) to reduce false uniqueness
    let mut chars: Vec<char> = Vec::new();
    let mut in_number = false;
    for c in normalized.trim().chars() {
        if c.is_ascii_digit() {
            if !in_number {
                chars.push('#');
                in_number = true;
            }
        } else {
            in_number = false;
            chars.push(c);
        }
    }

    chars.into_iter().collect()
}
