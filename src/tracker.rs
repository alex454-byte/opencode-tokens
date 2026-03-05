use rusqlite::{Connection, params};
use std::path::PathBuf;
use tiktoken_rs::cl100k_base;

fn db_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("oct");
    std::fs::create_dir_all(&dir).ok();
    dir.join("tracking.db")
}

fn open_db() -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            command TEXT NOT NULL,
            input_tokens INTEGER NOT NULL,
            output_tokens INTEGER NOT NULL,
            saved_tokens INTEGER NOT NULL,
            exec_ms INTEGER NOT NULL DEFAULT 0
        );"
    )?;
    Ok(conn)
}

pub fn count_tokens(text: &str) -> usize {
    let bpe = cl100k_base().unwrap();
    bpe.encode_with_special_tokens(text).len()
}

pub fn record(command: &str, input: &str, output: &str, exec_ms: u64) {
    let input_tokens = count_tokens(input) as i64;
    let output_tokens = count_tokens(output) as i64;
    let saved = input_tokens - output_tokens;

    if let Ok(conn) = open_db() {
        conn.execute(
            "INSERT INTO events (command, input_tokens, output_tokens, saved_tokens, exec_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![command, input_tokens, output_tokens, saved, exec_ms],
        ).ok();
    }
}

pub struct GainSummary {
    pub total_commands: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub saved_tokens: i64,
    pub savings_pct: f64,
}

pub fn get_summary() -> Option<GainSummary> {
    let conn = open_db().ok()?;
    let mut stmt = conn.prepare(
        "SELECT COUNT(*), COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COALESCE(SUM(saved_tokens),0) FROM events"
    ).ok()?;
    stmt.query_row([], |row| {
        let total: i64 = row.get(0)?;
        let input: i64 = row.get(1)?;
        let output: i64 = row.get(2)?;
        let saved: i64 = row.get(3)?;
        let pct = if input > 0 { (saved as f64 / input as f64) * 100.0 } else { 0.0 };
        Ok(GainSummary {
            total_commands: total,
            input_tokens: input,
            output_tokens: output,
            saved_tokens: saved,
            savings_pct: pct,
        })
    }).ok()
}

pub struct DailyGain {
    pub date: String,
    pub commands: i64,
    pub saved: i64,
    pub input: i64,
}

pub fn get_daily(days: i64) -> Vec<DailyGain> {
    let conn = match open_db() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = conn.prepare(
        "SELECT date(timestamp) as d, COUNT(*), SUM(saved_tokens), SUM(input_tokens)
         FROM events
         WHERE timestamp >= datetime('now', ?1)
         GROUP BY d ORDER BY d"
    ).unwrap();
    let days_str = format!("-{days} days");
    stmt.query_map(params![days_str], |row| {
        Ok(DailyGain {
            date: row.get(0)?,
            commands: row.get(1)?,
            saved: row.get(2)?,
            input: row.get(3)?,
        })
    }).unwrap().filter_map(|r| r.ok()).collect()
}

pub fn get_history(limit: i64) -> Vec<(String, String, i64, i64, i64)> {
    let conn = match open_db() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = conn.prepare(
        "SELECT timestamp, command, input_tokens, output_tokens, saved_tokens
         FROM events ORDER BY id DESC LIMIT ?1"
    ).unwrap();
    stmt.query_map(params![limit], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    }).unwrap().filter_map(|r| r.ok()).collect()
}
