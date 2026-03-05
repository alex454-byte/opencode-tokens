use std::fs;
use std::path::PathBuf;

pub fn run(global: bool) -> i32 {
    if global {
        println!("Installing oct globally for OpenCode...");
        install_global()
    } else {
        println!("Installing oct for current project...");
        install_local()
    }
}

fn install_global() -> i32 {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine home directory");
            return 1;
        }
    };

    // 1. Create shell wrapper at ~/.oct/oct-shell.sh
    let hook_dir = home.join(".oct");
    fs::create_dir_all(&hook_dir).ok();

    let shell_wrapper = hook_dir.join("oct-shell.sh");
    let wrapper_content = r#"#!/bin/bash
# oct shell wrapper for OpenCode
# Routes commands through oct for token-efficient output
#
# OpenCode config: "shell": { "path": "$HOME/.oct/oct-shell.sh", "args": [] }

if [ "$1" = "-c" ]; then
  shift
  CMD="$*"
else
  CMD="$*"
fi

# Extract the base command
BASE=$(echo "$CMD" | awk '{print $1}')

case "$BASE" in
  git|gh|docker|kubectl|ls|find|grep|rg|cat)
    exec oct $CMD ;;
  cargo)
    SUB=$(echo "$CMD" | awk '{print $2}')
    case "$SUB" in
      test) exec oct test $CMD ;;
      *)    eval "$CMD" ;;
    esac ;;
  pytest|vitest|jest)
    exec oct test $CMD ;;
  ruff|eslint|tsc|golangci-lint)
    exec oct err $CMD ;;
  *)
    eval "$CMD" ;;
esac
"#;
    if let Err(e) = fs::write(&shell_wrapper, wrapper_content) {
        eprintln!("Failed to write shell wrapper: {e}");
        return 1;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&shell_wrapper, fs::Permissions::from_mode(0o755)).ok();
    }

    // 2. Patch .opencode.json to use the wrapper shell
    let config_path = home.join(".opencode.json");
    patch_opencode_config(&config_path, &shell_wrapper);

    // 3. Also create AGENTS.md in home for projects that read it
    let agents_md = home.join(".oct").join("AGENTS.md");
    let agents_content = r#"# oct - Token Reduction Proxy

IMPORTANT: Always prefix shell commands with `oct` for token-efficient output.

Instead of:          Use:
  git status         oct git status
  git log            oct git log
  git diff           oct git diff
  ls                 oct ls .
  grep pattern .     oct grep pattern .
  cat file           oct read file
  cargo test         oct test cargo test
  pytest             oct test pytest

This reduces token usage by 60-90% per command.
Run `oct gain` to see savings.
"#;
    fs::write(&agents_md, agents_content).ok();

    println!("\nInstalled:");
    println!("  Shell wrapper: {}", shell_wrapper.display());
    println!("  Instructions:  {}", agents_md.display());
    println!("  Config patched: {}", config_path.display());
    println!("\nOpenCode will now route commands through oct automatically.");
    println!("Run `oct gain` anytime to see your token savings.");
    0
}

fn install_local() -> i32 {
    // Create AGENTS.md in current project (OpenCode reads this)
    let agents_content = r#"# oct - Token Reduction Proxy

IMPORTANT: Always prefix shell commands with `oct` for token-efficient output.

Instead of:          Use:
  git status         oct git status
  git log            oct git log
  git diff           oct git diff
  ls                 oct ls .
  grep pattern .     oct grep pattern .
  cat file           oct read file
  cargo test         oct test cargo test
  pytest             oct test pytest

This reduces token usage by 60-90% per command.
Run `oct gain` to see savings.
"#;
    if let Err(e) = fs::write("AGENTS.md", agents_content) {
        eprintln!("Failed to write AGENTS.md: {e}");
        return 1;
    }
    println!("Created AGENTS.md in current directory.");
    println!("OpenCode will read this and use oct for commands.");
    println!("\nFor automatic interception (no AI cooperation needed), run:");
    println!("  oct init --global");
    0
}

fn patch_opencode_config(config_path: &PathBuf, shell_wrapper: &PathBuf) {
    let config = if config_path.exists() {
        let backup = config_path.with_extension("json.bak");
        fs::copy(config_path, &backup).ok();
        println!("  Backup: {}", backup.display());
        fs::read_to_string(config_path).unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    };

    if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&config) {
        let obj = json.as_object_mut().unwrap();

        // Set shell to our wrapper
        let shell_obj = serde_json::json!({
            "path": shell_wrapper.to_string_lossy(),
            "args": []
        });
        obj.insert("shell".to_string(), shell_obj);

        if let Ok(pretty) = serde_json::to_string_pretty(&json) {
            fs::write(config_path, pretty).ok();
        }
    }
}

pub fn uninstall(global: bool) -> i32 {
    if global {
        if let Some(home) = dirs::home_dir() {
            // Remove shell wrapper dir
            let hook_dir = home.join(".oct");
            if hook_dir.exists() {
                fs::remove_dir_all(&hook_dir).ok();
                println!("Removed {}", hook_dir.display());
            }

            // Restore opencode config from backup
            let config_path = home.join(".opencode.json");
            let backup = config_path.with_extension("json.bak");
            if backup.exists() {
                fs::copy(&backup, &config_path).ok();
                fs::remove_file(&backup).ok();
                println!("Restored {}", config_path.display());
            }
        }
    }

    // Remove local AGENTS.md
    if std::path::Path::new("AGENTS.md").exists() {
        fs::remove_file("AGENTS.md").ok();
        println!("Removed AGENTS.md");
    }

    println!("oct uninstalled");
    0
}
