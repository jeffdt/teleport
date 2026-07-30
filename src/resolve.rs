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

/// Resolve the logical working directory from an optional PWD value and a
/// physical fallback. Prefers PWD when it is an absolute path pointing to an
/// existing directory, so symlink paths are preserved.
fn resolve_logical_cwd(pwd: Option<&str>, fallback: PathBuf) -> PathBuf {
    if let Some(pwd) = pwd {
        let p = PathBuf::from(pwd);
        if p.is_absolute() && p.is_dir() {
            return p;
        }
    }
    fallback
}

/// Get the current working directory, preferring the PWD environment variable
/// to preserve symlink paths. Falls back to `std::env::current_dir()` if PWD
/// is unset or invalid.
pub fn logical_cwd() -> PathBuf {
    resolve_logical_cwd(
        std::env::var("PWD").ok().as_deref(),
        std::env::current_dir().expect("could not determine current directory"),
    )
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

/// List a repo's worktrees, skipping the `git worktree list` call entirely
/// when the repo has no linked worktrees (only the one at `toplevel`).
fn worktrees_for_toplevel(toplevel: &Path) -> Vec<PathBuf> {
    let git_path = toplevel.join(".git");
    let may_have_worktrees = if git_path.is_dir() {
        git_path
            .join("worktrees")
            .read_dir()
            .is_ok_and(|mut d| d.next().is_some())
    } else {
        true
    };

    if may_have_worktrees {
        git_worktree_list(toplevel)
    } else {
        vec![toplevel.to_path_buf()]
    }
}

/// Get the worktree list for whatever repo contains `dir`, or `None` if
/// `dir` isn't inside a git repo. `git_toplevel_for` returns the toplevel of
/// whichever worktree `dir` is actually in, so this works the same whether
/// `dir` is in the main worktree or a linked one.
pub fn repo_worktrees_for(dir: &Path) -> Option<Vec<PathBuf>> {
    let toplevel = git_toplevel_for(dir)?;
    Some(worktrees_for_toplevel(&toplevel))
}

/// Find the worktree (if any) that contains `path`, and `path`'s location
/// relative to that worktree's root.
pub fn relative_to_worktree(path: &Path, worktrees: &[PathBuf]) -> Option<(PathBuf, PathBuf)> {
    worktrees
        .iter()
        .find(|wt| path.starts_with(wt))
        .map(|wt| (wt.clone(), path.strip_prefix(wt).unwrap().to_path_buf()))
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
    let toplevel = git_toplevel_for(&expanded)?;

    let relative_path = match (expanded.canonicalize(), toplevel.canonicalize()) {
        (Ok(ce), Ok(ct)) => ce
            .strip_prefix(&ct)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        _ => String::new(),
    };

    let worktrees = worktrees_for_toplevel(&toplevel);

    let main_wt = worktrees.first().cloned().unwrap_or_else(|| toplevel.clone());

    let current = std::env::current_dir().ok().and_then(|cwd| {
        worktrees.iter().find(|wt| cwd.starts_with(wt)).cloned()
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
    fn logical_cwd_prefers_valid_pwd() {
        let fallback = PathBuf::from("/fallback");
        let result = resolve_logical_cwd(Some("/tmp"), fallback);
        assert_eq!(result, PathBuf::from("/tmp"));
    }

    #[test]
    fn logical_cwd_rejects_relative_pwd() {
        let fallback = PathBuf::from("/fallback");
        let result = resolve_logical_cwd(Some("relative/path"), fallback.clone());
        assert_eq!(result, fallback);
    }

    #[test]
    fn logical_cwd_rejects_nonexistent_pwd() {
        let fallback = PathBuf::from("/fallback");
        let result = resolve_logical_cwd(
            Some("/surely/this/does/not/exist/anywhere"),
            fallback.clone(),
        );
        assert_eq!(result, fallback);
    }

    #[test]
    fn logical_cwd_falls_back_when_unset() {
        let fallback = PathBuf::from("/fallback");
        let result = resolve_logical_cwd(None, fallback.clone());
        assert_eq!(result, fallback);
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

    #[test]
    fn relative_to_worktree_root_is_empty() {
        let wt = PathBuf::from("/repo");
        let worktrees = vec![wt.clone()];

        let (root, rel) = relative_to_worktree(&wt, &worktrees).unwrap();

        assert_eq!(root, wt);
        assert_eq!(rel, PathBuf::new());
    }

    #[test]
    fn relative_to_worktree_nested_subdir() {
        let wt = PathBuf::from("/repo");
        let worktrees = vec![wt.clone()];
        let path = PathBuf::from("/repo/services/api");

        let (root, rel) = relative_to_worktree(&path, &worktrees).unwrap();

        assert_eq!(root, wt);
        assert_eq!(rel, PathBuf::from("services/api"));
    }

    #[test]
    fn relative_to_worktree_picks_containing_worktree() {
        let main = PathBuf::from("/repo");
        let feature = PathBuf::from("/repo.feature-oauth");
        let worktrees = vec![main, feature.clone()];
        let path = PathBuf::from("/repo.feature-oauth/services/api");

        let (root, rel) = relative_to_worktree(&path, &worktrees).unwrap();

        assert_eq!(root, feature);
        assert_eq!(rel, PathBuf::from("services/api"));
    }

    #[test]
    fn relative_to_worktree_none_when_unrelated() {
        let worktrees = vec![PathBuf::from("/repo")];
        let path = PathBuf::from("/elsewhere/api");

        assert!(relative_to_worktree(&path, &worktrees).is_none());
    }

    #[test]
    fn relative_to_worktree_no_false_positive_on_shared_prefix() {
        let worktrees = vec![PathBuf::from("/repo")];
        let path = PathBuf::from("/repo2/api");

        assert!(relative_to_worktree(&path, &worktrees).is_none());
    }
}
