use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::resolve::collapse_tilde;

const MAX_DISPLAYED_NAMES: usize = 3;

/// Format portal entries for display, grouping portals that share the same path.
/// Returns (display_line, portal_name) pairs where portal_name is the first
/// alphabetical name in each group (used as the lookup key).
pub fn format_portal_entries(
    portals: &std::collections::BTreeMap<String, String>,
    prefix: &str,
) -> Vec<(String, String)> {
    let mut by_path: std::collections::BTreeMap<&str, Vec<&str>> =
        std::collections::BTreeMap::new();
    // BTreeMap iteration is sorted by key, so names within each group arrive alphabetically.
    for (name, path) in portals {
        by_path.entry(path.as_str()).or_default().push(name.as_str());
    }

    let mut grouped: Vec<(String, String, String)> = by_path
        .into_iter()
        .map(|(path, names)| {
            let display_names = if names.len() > MAX_DISPLAYED_NAMES {
                let shown = &names[..MAX_DISPLAYED_NAMES];
                let extra = names.len() - MAX_DISPLAYED_NAMES;
                format!("{} +{} more", shown.join(", "), extra)
            } else {
                names.join(", ")
            };
            let key = names[0].to_string();
            (display_names, path.to_string(), key)
        })
        .collect();
    grouped.sort_by(|a, b| a.2.cmp(&b.2));

    let name_width = grouped.iter().map(|(dn, _, _)| dn.len()).max().unwrap_or(0);

    grouped
        .into_iter()
        .map(|(display_names, path, key)| {
            let display = format!(
                "  {}{:<width$}  {}",
                prefix, display_names, path,
                width = name_width
            );
            (display, key)
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

/// Format broken portal entries for prune output. Returns display lines with aligned columns.
pub fn format_prune_entries(portals: &[(String, String)]) -> Vec<String> {
    let name_width = portals.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    portals
        .iter()
        .map(|(name, path)| format!("  {:<width$}  {}", name, path, width = name_width))
        .collect()
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
    fn portal_entries_dedup_same_path() {
        let portals = [
            ("insights".to_string(), "~/r/k-repo/insights_service".to_string()),
            ("is".to_string(), "~/r/k-repo/insights_service".to_string()),
            ("app".to_string(), "~/r/app".to_string()),
        ]
        .into_iter()
        .collect();

        let entries = format_portal_entries(&portals, "* ");
        assert_eq!(entries.len(), 2);

        let grouped = entries.iter().find(|(d, _)| d.contains("insights")).unwrap();
        assert!(grouped.0.contains("insights, is"));
        assert!(grouped.0.contains("~/r/k-repo/insights_service"));
        assert_eq!(grouped.1, "insights");

        let singleton = entries.iter().find(|(d, _)| d.contains("app")).unwrap();
        assert!(singleton.0.contains("app"));
        assert_eq!(singleton.1, "app");
    }

    #[test]
    fn portal_entries_cap_names_at_three() {
        let portals = [
            ("a".to_string(), "~/same".to_string()),
            ("b".to_string(), "~/same".to_string()),
            ("c".to_string(), "~/same".to_string()),
            ("d".to_string(), "~/same".to_string()),
            ("e".to_string(), "~/same".to_string()),
        ]
        .into_iter()
        .collect();

        let entries = format_portal_entries(&portals, "* ");
        assert_eq!(entries.len(), 1);
        let display = &entries[0].0;
        assert!(display.contains("a, b, c"));
        assert!(display.contains("+2 more"));
        // Portal names "d" and "e" must not appear as listed names (only a, b, c shown)
        let name_section = display.split("  ~/").next().unwrap_or(display);
        assert!(!name_section.contains(", d") && !name_section.contains("d,"));
        assert!(!name_section.contains(", e") && !name_section.contains("e,"));
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

    #[test]
    fn format_prune_entries_aligns_columns() {
        let portals = vec![
            ("short".to_string(), "~/a".to_string()),
            ("much-longer".to_string(), "~/b".to_string()),
        ];

        let lines = format_prune_entries(&portals);
        assert_eq!(lines.len(), 2);
        // Both paths should start at the same column
        let pos_a = lines[0].find("~/a").unwrap();
        let pos_b = lines[1].find("~/b").unwrap();
        assert_eq!(pos_a, pos_b);
    }

    #[test]
    fn format_prune_entries_empty() {
        let portals: Vec<(String, String)> = vec![];
        let lines = format_prune_entries(&portals);
        assert!(lines.is_empty());
    }
}
