mod commands;
mod filters;
mod tracker;

use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "oct", about = "Token reduction proxy for OpenCode")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run a command through the proxy (e.g., oct git status)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,

    /// Ultra-compact output mode
    #[arg(short, long, global = true)]
    ultra: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize OpenCode integration
    Init {
        /// Install globally for all projects
        #[arg(long)]
        global: bool,
        /// Remove all oct artifacts
        #[arg(long)]
        uninstall: bool,
    },
    /// Show token savings analytics
    Gain {
        /// Show ASCII graph of last 30 days
        #[arg(long)]
        graph: bool,
        /// Show recent command history
        #[arg(long)]
        history: bool,
        /// Show daily breakdown
        #[arg(long)]
        daily: bool,
        /// Export format: json or csv
        #[arg(long)]
        format: Option<String>,
    },
    /// Scan OpenCode sessions to find optimization opportunities
    Discover {
        /// Scan all projects
        #[arg(long)]
        all: bool,
        /// Days to look back
        #[arg(long, default_value = "30")]
        since: u32,
    },
    /// Smart file read with compression
    Read {
        path: String,
        /// Compression level: normal, aggressive
        #[arg(short, long, default_value = "normal")]
        level: String,
    },
    /// Compact directory listing
    Ls {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Compact grep with grouping
    Grep {
        pattern: String,
        #[arg(default_value = ".")]
        path: String,
    },
    /// Filter test output to failures only
    Test {
        /// Command and args to run
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Filter to errors/warnings only
    Err {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Deduplicate log output
    Log {
        path: String,
    },
    /// Heuristic summary of command output
    Summary {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let ultra = cli.ultra;
    let exit_code = match cli.command {
        Some(Commands::Init { global, uninstall }) => {
            if uninstall {
                commands::init::uninstall(global)
            } else {
                commands::init::run(global)
            }
        }
        Some(Commands::Gain { graph, history, daily, format }) => {
            commands::gain::run(graph, history, daily, format)
        }
        Some(Commands::Discover { all, since }) => {
            commands::discover::run(all, since)
        }
        Some(Commands::Read { path, level }) => {
            filters::file::read_file(&path, &level, ultra)
        }
        Some(Commands::Ls { path }) => {
            filters::file::list_dir(&path, ultra)
        }
        Some(Commands::Grep { pattern, path }) => {
            filters::search::grep(&pattern, &path, ultra)
        }
        Some(Commands::Test { cmd }) => {
            filters::test::run(&cmd, ultra)
        }
        Some(Commands::Err { cmd }) => {
            filters::error::run(&cmd, ultra)
        }
        Some(Commands::Log { path }) => {
            filters::log::run(&path, ultra)
        }
        Some(Commands::Summary { cmd }) => {
            filters::summary::run(&cmd, ultra)
        }
        None => {
            if cli.args.is_empty() {
                eprintln!("Usage: oct <command> [args...] or oct --help");
                1
            } else {
                commands::proxy::run(&cli.args, ultra)
            }
        }
    };

    process::exit(exit_code);
}
