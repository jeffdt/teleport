use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Tunnel;

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

/// Find the main worktree for a repo (first entry from git worktree list).
pub fn main_worktree(repo_path: &Path) -> PathBuf {
    git_worktree_list(repo_path)
        .into_iter()
        .next()
        .unwrap_or_else(|| repo_path.to_path_buf())
}

/// Determine if cwd is inside one of the repo's worktrees.
/// Returns the matching worktree root if found.
pub fn current_worktree_for_repo(repo_path: &Path) -> Option<PathBuf> {
    let toplevel = git_toplevel()?;
    let worktrees = git_worktree_list(repo_path);
    worktrees.into_iter().find(|wt| *wt == toplevel)
}

/// Resolve a portal to an absolute path.
pub fn resolve_portal(path: &str) -> PathBuf {
    expand_tilde(path)
}

/// Build the context needed to resolve a tunnel.
/// Returns: (list of worktree roots, current worktree if inside one).
pub fn tunnel_worktree_context(tunnel: &Tunnel) -> (Vec<PathBuf>, Option<PathBuf>) {
    let repo_path = expand_tilde(&tunnel.repo);
    let worktrees = git_worktree_list(&repo_path);
    let current = current_worktree_for_repo(&repo_path);
    (worktrees, current)
}

/// Determine what kind of entry `tp add` should create from the current directory.
pub enum AddContext {
    Portal(String),
    Tunnel { repo: String, path: String },
}

pub fn detect_add_context(force_abs: bool) -> AddContext {
    let cwd = env::current_dir().expect("could not determine current directory");
    let cwd_str = collapse_tilde(&cwd);

    if force_abs {
        return AddContext::Portal(cwd_str);
    }

    let toplevel = match git_toplevel() {
        Some(t) => t,
        None => return AddContext::Portal(cwd_str),
    };

    // At repo root: portal
    if cwd == toplevel {
        return AddContext::Portal(cwd_str);
    }

    // In a subdir: tunnel
    let rel_path = cwd
        .strip_prefix(&toplevel)
        .expect("cwd should be under toplevel")
        .display()
        .to_string();

    // Find the main worktree to store as the repo path
    let main_wt = main_worktree(&toplevel);
    let repo_str = collapse_tilde(&main_wt);

    AddContext::Tunnel {
        repo: repo_str,
        path: rel_path,
    }
}
