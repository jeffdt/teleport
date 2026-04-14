use std::path::{Path, PathBuf};
use std::process::Command;

/// Expand ~ to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .expect("could not determine home directory")
            .join(rest)
    } else if path == "~" {
        dirs::home_dir().expect("could not determine home directory")
    } else {
        PathBuf::from(path)
    }
}

/// Collapse a path back to tilde form for display.
pub fn collapse_tilde(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = path.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}

/// Get the git toplevel for a specific directory.
pub fn git_toplevel_for(dir: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["-C", &dir.display().to_string(), "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?.trim().to_string();
    Some(PathBuf::from(path))
}

/// Get the path relative to the repo root for a specific directory.
/// Uses `git rev-parse --show-prefix` which handles worktree indirection correctly.
pub fn git_show_prefix(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &dir.display().to_string(), "rev-parse", "--show-prefix"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let prefix = String::from_utf8(output.stdout).ok()?.trim().to_string();
    // git returns trailing slash for subdirs, strip it
    Some(prefix.trim_end_matches('/').to_string())
}

/// Get the git toplevel for the current directory, if any.
pub fn git_toplevel() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?.trim().to_string();
    Some(PathBuf::from(path))
}

/// Get all worktrees for a repo.
pub fn git_worktree_list(repo_path: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(["-C", &repo_path.display().to_string(), "worktree", "list", "--porcelain"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return vec![repo_path.to_path_buf()],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .collect()
}

/// Resolve a portal to an absolute path.
pub fn resolve_portal(path: &str) -> PathBuf {
    expand_tilde(path)
}

/// Context for resolving a portal that lives inside a git repo.
pub struct PortalContext {
    pub worktrees: Vec<PathBuf>,
    pub main_worktree: PathBuf,
    pub current_worktree: Option<PathBuf>,
    pub relative_path: String,
}

/// Detect if a portal path is inside a git repo and gather worktree context.
/// Returns None if the path is not inside a git repo.
pub fn portal_worktree_context(portal_path: &str) -> Option<PortalContext> {
    let expanded = expand_tilde(portal_path);

    // Find the repo root for this path
    let toplevel = git_toplevel_for(&expanded)?;

    // Relative path within the repo (empty string if at repo root).
    // Uses git show-prefix to handle worktree indirection correctly.
    let relative_path = git_show_prefix(&expanded).unwrap_or_default();

    // Get worktree list from this repo (works from any worktree)
    let worktrees = git_worktree_list(&toplevel);
    let main_wt = worktrees.first().cloned().unwrap_or_else(|| toplevel.clone());

    // Detect current worktree using already-fetched list (avoids a second git subprocess)
    let current = git_toplevel().and_then(|cwd_toplevel| {
        worktrees.iter().find(|wt| **wt == cwd_toplevel).cloned()
    });

    Some(PortalContext {
        worktrees,
        main_worktree: main_wt,
        current_worktree: current,
        relative_path,
    })
}

/// A worktree with display metadata for the picker.
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub is_main: bool,
    pub is_current: bool,
}

/// Sort worktrees for the picker: current first, then main, then rest in creation order.
pub fn sorted_worktrees(
    worktrees: &[PathBuf],
    main_worktree: &Path,
    current_worktree: Option<&Path>,
) -> Vec<WorktreeInfo> {
    let mut result: Vec<WorktreeInfo> = Vec::new();

    // Current worktree first (if we're inside one)
    if let Some(current) = current_worktree {
        if worktrees.iter().any(|wt| wt.as_path() == current) {
            result.push(WorktreeInfo {
                path: current.to_path_buf(),
                is_main: current == main_worktree,
                is_current: true,
            });
        }
    }

    // Main worktree (if not already added as current)
    if !result.iter().any(|info| info.path == main_worktree) {
        result.push(WorktreeInfo {
            path: main_worktree.to_path_buf(),
            is_main: true,
            is_current: false,
        });
    }

    // Remaining worktrees in creation order
    for wt in worktrees {
        if !result.iter().any(|info| info.path == *wt) {
            result.push(WorktreeInfo {
                path: wt.clone(),
                is_main: false,
                is_current: false,
            });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorted_worktrees_current_first() {
        let main = PathBuf::from("/repo");
        let wt_a = PathBuf::from("/repo.wt-a");
        let wt_b = PathBuf::from("/repo.wt-b");
        let worktrees = vec![main.clone(), wt_a.clone(), wt_b.clone()];

        let sorted = sorted_worktrees(&worktrees, &main, Some(&wt_a));

        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].path, wt_a);
        assert!(sorted[0].is_current);
        assert!(!sorted[0].is_main);
        assert_eq!(sorted[1].path, main);
        assert!(sorted[1].is_main);
        assert!(!sorted[1].is_current);
        assert_eq!(sorted[2].path, wt_b);
        assert!(!sorted[2].is_main);
        assert!(!sorted[2].is_current);
    }

    #[test]
    fn sorted_worktrees_current_is_main() {
        let main = PathBuf::from("/repo");
        let wt_a = PathBuf::from("/repo.wt-a");
        let worktrees = vec![main.clone(), wt_a.clone()];

        let sorted = sorted_worktrees(&worktrees, &main, Some(&main));

        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].path, main);
        assert!(sorted[0].is_current);
        assert!(sorted[0].is_main);
        assert_eq!(sorted[1].path, wt_a);
    }

    #[test]
    fn sorted_worktrees_no_current() {
        let main = PathBuf::from("/repo");
        let wt_a = PathBuf::from("/repo.wt-a");
        let worktrees = vec![main.clone(), wt_a.clone()];

        let sorted = sorted_worktrees(&worktrees, &main, None);

        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].path, main);
        assert!(sorted[0].is_main);
        assert!(!sorted[0].is_current);
    }
}
