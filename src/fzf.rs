use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::resolve::collapse_tilde;

/// Format portal entries for display. Returns (display_line, portal_name) pairs.
/// Use `prefix` to distinguish contexts: e.g. `"* "` for picker, `""` for `tp ls`.
pub fn format_portal_entries(
    portals: &std::collections::BTreeMap<String, String>,
    prefix: &str,
) -> Vec<(String, String)> {
    let name_width = portals.keys().map(|k| k.len()).max().unwrap_or(0);

    portals
        .iter()
        .map(|(name, path)| {
            let display = format!(
                "  {}{:<width$}  {}",
                prefix, name, path,
                width = name_width
            );
            (display, name.clone())
        })
        .collect()
}

/// Format worktree entries for the picker with colored labels.
/// Returns (display_line, actual_path) pairs.
pub fn format_worktree_entries(worktrees: &[crate::resolve::WorktreeInfo]) -> Vec<(String, PathBuf)> {
    const GREEN: &str = "\x1b[32m";
    const BLUE: &str = "\x1b[34m";
    const RESET: &str = "\x1b[0m";

    worktrees
        .iter()
        .map(|info| {
            let path_str = collapse_tilde(&info.path);
            let label = match (info.is_current, info.is_main) {
                (true, true) => format!("  {GREEN}(current, main){RESET}"),
                (true, false) => format!("  {GREEN}(current){RESET}"),
                (false, true) => format!("  {BLUE}(main){RESET}"),
                (false, false) => String::new(),
            };
            let display = format!("  {path_str}{label}");
            (display, info.path.clone())
        })
        .collect()
}

/// Strip ANSI escape codes from a string for comparison.
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
        } else {
            result.push(c);
        }
    }
    result
}

/// Spawn fzf with the given lines and prompt. Returns the index of the selected line or None.
pub fn pick(lines: &[String], prompt: &str) -> Option<usize> {
    let fzf = Command::new("fzf")
        .args(["--height=~50%", "--layout=reverse", "--ansi", "--border", &format!("--prompt={} ", prompt)])
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

    let selected = String::from_utf8(output.stdout).ok()?.trim_end().to_string();
    if selected.is_empty() {
        return None;
    }

    let stripped = strip_ansi(&selected);
    lines.iter().position(|l| strip_ansi(l) == stripped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::WorktreeInfo;

    #[test]
    fn portal_entries_with_star_prefix() {
        let portals = [("notes".to_string(), "~/Documents/notes".to_string())]
            .into_iter()
            .collect();

        let entries = format_portal_entries(&portals, "* ");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].0.contains("*"));
        assert!(entries[0].0.contains("notes"));
    }

    #[test]
    fn worktree_entries_show_labels() {
        let worktrees = vec![
            WorktreeInfo {
                path: PathBuf::from("/Users/jeff/r/k-repo.wt-auth"),
                is_main: false,
                is_current: true,
            },
            WorktreeInfo {
                path: PathBuf::from("/Users/jeff/r/k-repo"),
                is_main: true,
                is_current: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/Users/jeff/r/k-repo.wt-other"),
                is_main: false,
                is_current: false,
            },
        ];

        let entries = format_worktree_entries(&worktrees);
        assert_eq!(entries.len(), 3);

        // Current worktree has (current) label
        assert!(entries[0].0.contains("(current)"));
        assert!(!entries[0].0.contains("(main)"));
        assert_eq!(entries[0].1, PathBuf::from("/Users/jeff/r/k-repo.wt-auth"));

        // Main worktree has (main) label
        assert!(entries[1].0.contains("(main)"));
        assert!(!entries[1].0.contains("(current)"));

        // Neither has no labels
        assert!(!entries[2].0.contains("(current)"));
        assert!(!entries[2].0.contains("(main)"));
    }

    #[test]
    fn worktree_entries_both_labels() {
        let worktrees = vec![
            WorktreeInfo {
                path: PathBuf::from("/Users/jeff/r/k-repo"),
                is_main: true,
                is_current: true,
            },
        ];

        let entries = format_worktree_entries(&worktrees);
        assert!(entries[0].0.contains("(current, main)"));
    }
}
