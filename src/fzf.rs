use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::config::Config;
use crate::resolve::{collapse_tilde, expand_tilde};

/// Format all portals and tunnels for the main fzf picker.
pub fn format_entries(config: &Config) -> Vec<(String, String)> {
    let mut entries: Vec<(String, String)> = Vec::new();

    let name_width = config
        .portals
        .keys()
        .chain(config.tunnels.keys())
        .map(|k| k.len())
        .max()
        .unwrap_or(0);

    for (name, path) in &config.portals {
        let display = format!(
            "  {:<width$}  portal   {}",
            name,
            path,
            width = name_width
        );
        entries.push((display, name.clone()));
    }

    for (name, tunnel) in &config.tunnels {
        let repo_basename = expand_tilde(&tunnel.repo)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let short_path = if tunnel.path.len() > 40 {
            let parts: Vec<&str> = tunnel.path.split('/').collect();
            if parts.len() > 3 {
                format!("{}/.../{}", parts[0], parts[parts.len() - 1])
            } else {
                tunnel.path.clone()
            }
        } else {
            tunnel.path.clone()
        };

        let display = format!(
            "  {:<width$}  tunnel   {} -> {}",
            name,
            repo_basename,
            short_path,
            width = name_width
        );
        entries.push((display, name.clone()));
    }

    entries
}

/// Format worktree paths for the worktree picker.
pub fn format_worktrees(worktrees: &[PathBuf]) -> Vec<String> {
    worktrees
        .iter()
        .map(|wt| collapse_tilde(wt))
        .collect()
}

/// Spawn fzf with the given lines and prompt. Returns the selected line or None.
pub fn pick(lines: &[String], prompt: &str) -> Option<String> {
    let fzf = Command::new("fzf")
        .args(["--height=~50%", "--border", &format!("--prompt={} ", prompt)])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn();

    let mut child = match fzf {
        Ok(c) => c,
        Err(e) => {
            eprintln!("fzf is required but not found: {e}");
            eprintln!("Install it with: brew install fzf");
            std::process::exit(1);
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        for line in lines {
            let _ = writeln!(stdin, "{}", line);
        }
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let selected = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if selected.is_empty() {
        None
    } else {
        Some(selected)
    }
}
