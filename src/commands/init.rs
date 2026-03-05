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
    // Create the hook script
    let hook_dir = match dirs::home_dir() {
        Some(h) => h.join(".oct"),
        None => {
            eprintln!("Could not determine home directory");
            return 1;
        }
    };
    fs::create_dir_all(&hook_dir).ok();

    let hook_path = hook_dir.join("hook.sh");
    let hook_content = r#"#!/bin/bash
# oct hook for OpenCode - rewrites commands through the token proxy
CMD="$1"; shift
case "$CMD" in
  git|gh|docker|kubectl|cargo|pytest|go|ruff|eslint|tsc|ls|cat|find|grep)
    exec oct "$CMD" "$@" ;;
  *)
    exec "$CMD" "$@" ;;
esac
"#;
    if let Err(e) = fs::write(&hook_path, hook_content) {
        eprintln!("Failed to write hook: {e}");
        return 1;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).ok();
    }

    // Write minimal OCT.md for OpenCode context
    let oct_md = r#"# oct - Token Reduction Proxy
Commands are routed through `oct` for compact output.
No action needed - the hook handles this transparently.
"#;
    let oct_md_path = hook_dir.join("OCT.md");
    fs::write(&oct_md_path, oct_md).ok();

    // Try to patch OpenCode config
    if let Some(home) = dirs::home_dir() {
        let config_path = home.join(".opencode.json");
        patch_opencode_config(&config_path);
    }

    println!("Installed:");
    println!("  Hook:   {}", hook_path.display());
    println!("  Config: ~/.oct/OCT.md");
    println!("\nAdd to your shell profile:");
    println!("  export PATH=\"$HOME/.oct:$PATH\"");
    0
}

fn install_local() -> i32 {
    let oct_md = r#"# oct - Token Reduction Proxy
Use `oct` prefix for compact command output:
  oct git status    - compact git status
  oct ls .          - tree-style directory listing
  oct grep "pat" .  - grouped search results
  oct test cargo test - failures only
  oct gain          - token savings analytics
"#;
    if let Err(e) = fs::write("OCT.md", oct_md) {
        eprintln!("Failed to write OCT.md: {e}");
        return 1;
    }
    println!("Created OCT.md in current directory");
    0
}

fn patch_opencode_config(config_path: &PathBuf) {
    // Read existing config or create new one
    let config = if config_path.exists() {
        // Back up first
        let backup = config_path.with_extension("json.bak");
        fs::copy(config_path, &backup).ok();
        println!("  Backup: {}", backup.display());

        fs::read_to_string(config_path).unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    };

    // Parse and inject oct hook reference
    if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&config) {
        let obj = json.as_object_mut().unwrap();
        obj.entry("instructions".to_string())
            .or_insert(serde_json::Value::String(
                "Commands are proxied through oct for token-efficient output.".to_string(),
            ));
        if let Ok(pretty) = serde_json::to_string_pretty(&json) {
            fs::write(config_path, pretty).ok();
        }
    }
}

pub fn uninstall(_global: bool) -> i32 {
    // Remove hook and config
    if let Some(home) = dirs::home_dir() {
        let hook_dir = home.join(".oct");
        if hook_dir.exists() {
            fs::remove_dir_all(&hook_dir).ok();
            println!("Removed {}", hook_dir.display());
        }
    }

    if std::path::Path::new("OCT.md").exists() {
        fs::remove_file("OCT.md").ok();
        println!("Removed OCT.md");
    }

    println!("oct uninstalled");
    0
}
