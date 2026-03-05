use crate::tracker;

pub fn run(graph: bool, history: bool, daily: bool, format: Option<String>) -> i32 {
    if let Some(fmt) = &format {
        return export(fmt, daily);
    }

    if history {
        return show_history();
    }

    if daily {
        return show_daily();
    }

    if graph {
        return show_graph();
    }

    show_summary()
}

fn show_summary() -> i32 {
    match tracker::get_summary() {
        Some(s) if s.total_commands > 0 => {
            println!("Total commands:  {}", s.total_commands);
            println!("Input tokens:    {}", format_tokens(s.input_tokens));
            println!("Output tokens:   {}", format_tokens(s.output_tokens));
            println!("Tokens saved:    {} ({:.1}%)", format_tokens(s.saved_tokens), s.savings_pct);
        }
        _ => {
            println!("No data yet. Run commands through oct to start tracking.");
        }
    }
    0
}

fn show_history() -> i32 {
    let rows = tracker::get_history(10);
    if rows.is_empty() {
        println!("No history yet.");
        return 0;
    }
    println!("{:<20} {:<30} {:>8} {:>8} {:>8}", "Time", "Command", "In", "Out", "Saved");
    println!("{}", "-".repeat(78));
    for (ts, cmd, inp, out, saved) in &rows {
        let cmd_short: String = cmd.chars().take(28).collect();
        println!("{:<20} {:<30} {:>8} {:>8} {:>8}", ts, cmd_short, inp, out, saved);
    }
    0
}

fn show_daily() -> i32 {
    let days = tracker::get_daily(30);
    if days.is_empty() {
        println!("No data yet.");
        return 0;
    }
    println!("{:<12} {:>8} {:>10} {:>10} {:>7}", "Date", "Cmds", "Input", "Saved", "Pct");
    println!("{}", "-".repeat(52));
    for d in &days {
        let pct = if d.input > 0 {
            (d.saved as f64 / d.input as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "{:<12} {:>8} {:>10} {:>10} {:>6.1}%",
            d.date, d.commands, format_tokens(d.input), format_tokens(d.saved), pct
        );
    }
    0
}

fn show_graph() -> i32 {
    let days = tracker::get_daily(30);
    if days.is_empty() {
        println!("No data yet.");
        return 0;
    }

    let max_saved = days.iter().map(|d| d.saved).max().unwrap_or(1).max(1);
    let bar_width = 40;

    println!("Token savings - last 30 days\n");
    for d in &days {
        let bar_len = ((d.saved as f64 / max_saved as f64) * bar_width as f64) as usize;
        let bar: String = "█".repeat(bar_len);
        println!("{} {:>7} │{}", &d.date[5..], format_tokens(d.saved), bar);
    }
    println!();
    0
}

fn export(fmt: &str, daily: bool) -> i32 {
    if daily {
        let days = tracker::get_daily(30);
        match fmt {
            "json" => {
                let entries: Vec<serde_json::Value> = days
                    .iter()
                    .map(|d| {
                        serde_json::json!({
                            "date": d.date,
                            "commands": d.commands,
                            "input_tokens": d.input,
                            "saved_tokens": d.saved
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&entries).unwrap());
            }
            "csv" => {
                println!("date,commands,input_tokens,saved_tokens");
                for d in &days {
                    println!("{},{},{},{}", d.date, d.commands, d.input, d.saved);
                }
            }
            _ => {
                eprintln!("Unknown format: {fmt}. Use json or csv.");
                return 1;
            }
        }
    } else if let Some(s) = tracker::get_summary() {
        match fmt {
            "json" => {
                let j = serde_json::json!({
                    "total_commands": s.total_commands,
                    "input_tokens": s.input_tokens,
                    "output_tokens": s.output_tokens,
                    "saved_tokens": s.saved_tokens,
                    "savings_pct": s.savings_pct
                });
                println!("{}", serde_json::to_string_pretty(&j).unwrap());
            }
            "csv" => {
                println!("total_commands,input_tokens,output_tokens,saved_tokens,savings_pct");
                println!("{},{},{},{},{:.1}", s.total_commands, s.input_tokens, s.output_tokens, s.saved_tokens, s.savings_pct);
            }
            _ => {
                eprintln!("Unknown format: {fmt}. Use json or csv.");
                return 1;
            }
        }
    }
    0
}

fn format_tokens(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
