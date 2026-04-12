use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::config::Config;
use crate::history::FrecencyEntry;
use crate::resolve::{collapse_tilde, expand_tilde};

/// Format all portals for `tp ls`.
pub fn format_entries(config: &Config) -> Vec<(String, String)> {
    let name_width = config
        .portals
        .keys()
        .map(|k| k.len())
        .max()
        .unwrap_or(0);

    config
        .portals
        .iter()
        .map(|(name, path)| {
            let display = format!("  {:<width$}  {}", name, path, width = name_width);
            (display, name.clone())
        })
        .collect()
}

/// Format bookmarks for the fzf picker with star prefix.
pub fn format_bookmark_entries(config: &Config) -> Vec<(String, String)> {
    let name_width = config
        .portals
        .keys()
        .map(|k| k.len())
        .max()
        .unwrap_or(0);

    config
        .portals
        .iter()
        .map(|(name, path)| {
            let display = format!(
                "  * {:<width$}  {}",
                name, path,
                width = name_width
            );
            (display, name.clone())
        })
        .collect()
}

/// Reference data for a frecent entry selected from the picker.
pub struct FrecencyEntryRef {
    pub path: String,
    pub repo: Option<String>,
}

/// Format frecent entries for the fzf picker (no star, with score).
pub fn format_frecent_entries(
    entries: &[&FrecencyEntry],
    name_width: usize,
) -> Vec<(String, FrecencyEntryRef)> {
    entries
        .iter()
        .map(|entry| {
            let display_path = match &entry.repo {
                Some(repo) => {
                    let repo_basename = expand_tilde(repo)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    format!("{} -> {}", repo_basename, entry.path)
                }
                None => entry.path.clone(),
            };

            let label = entry.path.rsplit('/').next().unwrap_or(&entry.path);

            let display = format!(
                "    {:<width$}           {:<40} {:>5.1}",
                label,
                display_path,
                entry.effective_score,
                width = name_width,
            );

            let ref_data = FrecencyEntryRef {
                path: entry.path.clone(),
                repo: entry.repo.clone(),
            };

            (display, ref_data)
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
    use crate::history::FrecencyEntry;
    use crate::resolve::WorktreeInfo;

    #[test]
    fn frecent_entries_formatted_with_score() {
        let entries = vec![
            FrecencyEntry {
                path: "~/Documents/notes".to_string(),
                repo: None,
                effective_score: 3.2,
            },
            FrecencyEntry {
                path: "src/components".to_string(),
                repo: Some("~/r/app".to_string()),
                effective_score: 2.8,
            },
        ];
        let entry_refs: Vec<&FrecencyEntry> = entries.iter().collect();

        let formatted = format_frecent_entries(&entry_refs, 10);
        assert_eq!(formatted.len(), 2);

        // Non-repo entry
        assert!(formatted[0].0.contains("~/Documents/notes"));
        assert!(formatted[0].0.contains("3.2"));
        assert!(!formatted[0].0.contains("*"));

        // Repo entry shows repo basename
        assert!(formatted[1].0.contains("app"));
        assert!(formatted[1].0.contains("src/components"));
    }

    #[test]
    fn bookmark_entries_have_star() {
        let config = Config {
            portals: [("notes".to_string(), "~/Documents/notes".to_string())]
                .into_iter()
                .collect(),
        };

        let entries = format_bookmark_entries(&config);
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
