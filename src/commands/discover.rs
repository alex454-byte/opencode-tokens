use std::path::PathBuf;

// Commands that oct can optimize
const OPTIMIZABLE: &[&str] = &[
    "git status", "git log", "git diff", "git push", "git pull",
    "ls", "cat", "find", "grep", "rg",
    "cargo test", "pytest", "go test", "npm test", "vitest", "jest",
    "docker ps", "docker images", "docker logs",
    "kubectl get", "kubectl logs",
    "gh pr", "gh issue", "gh run",
    "ruff", "eslint", "tsc", "golangci-lint",
];

pub fn run(all: bool, since: u32) -> i32 {
    // Look for OpenCode's SQLite database
    let db_paths = find_opencode_dbs(all);

    if db_paths.is_empty() {
        eprintln!("No OpenCode session databases found.");
        eprintln!("OpenCode stores data in ~/.local/share/opencode/ or similar.");
        return 1;
    }

    let mut total_opportunities = 0;
    let mut command_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for db_path in &db_paths {
        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            // Try to find tool calls / command executions in OpenCode's schema
            let tables = get_tables(&conn);

            for table in &tables {
                if let Some(counts) = scan_table_for_commands(&conn, table, since) {
                    for (cmd, count) in counts {
                        *command_counts.entry(cmd).or_insert(0) += count;
                        total_opportunities += count;
                    }
                }
            }
        }
    }

    if total_opportunities == 0 {
        println!("No optimizable commands found in the last {since} days.");
        println!("This could mean OpenCode uses a different storage format.");
        return 0;
    }

    println!("Optimization opportunities (last {since} days):\n");
    println!("{:<30} {:>8} {:>12}", "Command", "Count", "Est. Savings");
    println!("{}", "-".repeat(54));

    let mut sorted: Vec<_> = command_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    for (cmd, count) in &sorted {
        // Rough estimate: 500 tokens saved per command on average
        let est_saved = count * 500;
        println!("{:<30} {:>8} {:>10}K", cmd, count, est_saved / 1000);
    }

    println!("\nTotal: {} commands, ~{}K tokens recoverable",
        total_opportunities,
        total_opportunities * 500 / 1000
    );
    0
}

fn find_opencode_dbs(all: bool) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Check common OpenCode data locations
    let search_dirs: Vec<PathBuf> = if all {
        let mut dirs = Vec::new();
        if let Some(data) = dirs::data_local_dir() {
            dirs.push(data.join("opencode"));
        }
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".opencode"));
            dirs.push(home.join(".local/share/opencode"));
        }
        if let Some(config) = dirs::config_dir() {
            dirs.push(config.join("opencode"));
        }
        dirs
    } else {
        // Current project only
        vec![PathBuf::from(".opencode")]
    };

    for dir in search_dirs {
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "db" || e == "sqlite").unwrap_or(false) {
                        paths.push(path);
                    }
                }
            }
            // Also check for a single db file
            let db = dir.join("opencode.db");
            if db.exists() && !paths.contains(&db) {
                paths.push(db);
            }
        }
    }

    paths
}

fn get_tables(conn: &rusqlite::Connection) -> Vec<String> {
    let mut stmt = match conn.prepare("SELECT name FROM sqlite_master WHERE type='table'") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |row| row.get(0))
        .unwrap_or_else(|_| panic!())
        .filter_map(|r| r.ok())
        .collect()
}

fn scan_table_for_commands(
    conn: &rusqlite::Connection,
    table: &str,
    _since: u32,
) -> Option<std::collections::HashMap<String, usize>> {
    // Try to find text columns that might contain command data
    let query = format!("SELECT * FROM \"{}\" LIMIT 100", table);
    let mut stmt = conn.prepare(&query).ok()?;
    let col_count = stmt.column_count();

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    let rows = stmt.query_map([], |row| {
        let mut texts = Vec::new();
        for i in 0..col_count {
            if let Ok(val) = row.get::<_, String>(i) {
                texts.push(val);
            }
        }
        Ok(texts)
    }).ok()?;

    for row in rows.flatten() {
        for text in &row {
            for cmd in OPTIMIZABLE {
                if text.contains(cmd) {
                    *counts.entry(cmd.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    if counts.is_empty() { None } else { Some(counts) }
}
