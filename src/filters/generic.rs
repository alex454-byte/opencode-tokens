/// Generic output compression: dedup, blank line collapse, truncation
pub fn compress(input: &str, ultra: bool) -> String {
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() {
        return String::new();
    }

    let mut result = Vec::new();
    let mut prev_line: Option<&str> = None;
    let mut repeat_count = 0u32;
    let mut blank_run = 0u32;

    for line in &lines {
        let trimmed = line.trim();

        // Collapse blank lines
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                flush_repeats(&mut result, prev_line, repeat_count);
                prev_line = None;
                repeat_count = 0;
                result.push(String::new());
            }
            continue;
        }
        blank_run = 0;

        // Dedup consecutive identical lines
        if Some(*line) == prev_line {
            repeat_count += 1;
            continue;
        }

        flush_repeats(&mut result, prev_line, repeat_count);
        prev_line = Some(line);
        repeat_count = 0;
    }

    flush_repeats(&mut result, prev_line, repeat_count);

    // Ultra mode: truncate long lines
    if ultra {
        for line in &mut result {
            if line.len() > 120 {
                line.truncate(117);
                line.push_str("...");
            }
        }
    }

    // Truncate very long output
    let max_lines = if ultra { 50 } else { 200 };
    if result.len() > max_lines {
        let omitted = result.len() - max_lines;
        result.truncate(max_lines);
        result.push(format!("... ({omitted} lines omitted)"));
    }

    result.join("\n")
}

fn flush_repeats(result: &mut Vec<String>, line: Option<&str>, count: u32) {
    if let Some(l) = line {
        result.push(l.to_string());
        if count > 0 {
            result.push(format!("  ... (repeated {count}x)"));
        }
    }
}
