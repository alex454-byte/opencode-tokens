use std::fs;
use std::path::Path;
use std::time::Instant;
use crate::tracker;

pub fn read_file(path: &str, level: &str, ultra: bool) -> i32 {
    let start = Instant::now();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("oct: cannot read '{}': {}", path, e);
            return 1;
        }
    };

    let elapsed = start.elapsed().as_millis() as u64;

    let compressed = match level {
        "aggressive" => aggressive_compress(&content, path),
        _ => normal_compress(&content, ultra),
    };

    tracker::record(&format!("read {path}"), &content, &compressed, elapsed);
    print!("{compressed}");
    0
}

fn normal_compress(content: &str, ultra: bool) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut blank_run = 0u32;

    for line in &lines {
        let trimmed = line.trim();

        // Skip pure comment lines (but keep doc comments)
        if is_noise_comment(trimmed) {
            continue;
        }

        // Collapse blank lines
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                result.push(String::new());
            }
            continue;
        }
        blank_run = 0;

        if ultra && line.len() > 120 {
            let mut truncated = line[..117].to_string();
            truncated.push_str("...");
            result.push(truncated);
        } else {
            result.push(line.to_string());
        }
    }

    // Truncate very long files
    let max = if ultra { 80 } else { 300 };
    if result.len() > max {
        let omitted = result.len() - max;
        result.truncate(max);
        result.push(format!("... ({omitted} lines omitted)"));
    }

    result.join("\n")
}

fn aggressive_compress(content: &str, path: &str) -> String {
    // Extract only signatures: function/struct/class/impl declarations
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mut signatures = Vec::new();
    let mut depth = 0i32;

    for line in content.lines() {
        let trimmed = line.trim();

        if is_signature(trimmed, ext) && depth <= 1 {
            signatures.push(trimmed.to_string());
        }

        depth += trimmed.matches('{').count() as i32;
        depth -= trimmed.matches('}').count() as i32;
        depth = depth.max(0);
    }

    if signatures.is_empty() {
        format!("({} lines, no signatures extracted)", content.lines().count())
    } else {
        signatures.join("\n")
    }
}

fn is_signature(line: &str, ext: &str) -> bool {
    match ext {
        "rs" => {
            line.starts_with("pub ") || line.starts_with("fn ")
                || line.starts_with("struct ") || line.starts_with("enum ")
                || line.starts_with("impl ") || line.starts_with("trait ")
                || line.starts_with("mod ") || line.starts_with("type ")
        }
        "go" => {
            line.starts_with("func ") || line.starts_with("type ")
                || line.starts_with("interface ")
        }
        "py" => {
            line.starts_with("def ") || line.starts_with("class ")
                || line.starts_with("async def ")
        }
        "ts" | "tsx" | "js" | "jsx" => {
            line.starts_with("export ") || line.starts_with("function ")
                || line.starts_with("class ") || line.starts_with("interface ")
                || line.starts_with("const ") || line.starts_with("async function ")
        }
        _ => false,
    }
}

fn is_noise_comment(line: &str) -> bool {
    // Keep doc comments, skip noise
    if line.starts_with("///") || line.starts_with("//!") || line.starts_with("/**") {
        return false;
    }
    if line.starts_with("# ") && !line.starts_with("#!") && !line.starts_with("#[") {
        // Python comments - keep docstrings though
        return false;
    }
    // Skip single-line // comments that are just noise
    if line.starts_with("//") {
        let content = line[2..].trim();
        // Keep TODO/FIXME/HACK/SAFETY comments
        if content.starts_with("TODO") || content.starts_with("FIXME")
            || content.starts_with("HACK") || content.starts_with("SAFETY")
        {
            return false;
        }
        return true;
    }
    false
}

pub fn list_dir(path: &str, ultra: bool) -> i32 {
    let start = Instant::now();
    let mut entries = Vec::new();

    if let Err(e) = walk_dir(path, "", &mut entries, 0, if ultra { 2 } else { 3 }) {
        eprintln!("oct: cannot list '{}': {}", path, e);
        return 1;
    }

    let elapsed = start.elapsed().as_millis() as u64;

    // Build tree output
    let raw = entries.join("\n");
    let compact = if entries.len() > 200 {
        let mut truncated = entries[..200].to_vec();
        truncated.push(format!("... +{} more entries", entries.len() - 200));
        truncated.join("\n")
    } else {
        raw.clone()
    };

    tracker::record(&format!("ls {path}"), &raw, &compact, elapsed);
    println!("{compact}");
    0
}

fn walk_dir(
    path: &str,
    prefix: &str,
    entries: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) -> std::io::Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    let mut items: Vec<_> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .collect();
    items.sort_by_key(|e| e.file_name());

    // Filter hidden files at top level
    items.retain(|e| {
        let name = e.file_name();
        let name_str = name.to_string_lossy();
        !name_str.starts_with('.') || depth > 0 && !name_str.starts_with('.')
    });

    for (i, entry) in items.iter().enumerate() {
        let is_last = i == items.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        let display = if is_dir {
            format!("{}{}{}/", prefix, connector, name.to_string_lossy())
        } else {
            format!("{}{}{}", prefix, connector, name.to_string_lossy())
        };
        entries.push(display);

        if is_dir {
            let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            walk_dir(
                &entry.path().to_string_lossy(),
                &new_prefix,
                entries,
                depth + 1,
                max_depth,
            )?;
        }
    }

    Ok(())
}
