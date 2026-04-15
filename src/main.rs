mod config;
mod fzf;
mod resolve;

use std::process;

use clap::Parser;

use config::Config;
use resolve::{portal_worktree_context, sorted_worktrees};

#[derive(Clone, Copy)]
enum WorktreeMode {
    Picker,
    MainOnly,
    Direct,
}

#[derive(Parser)]
#[command(name = "warp-core", version, about = "Engine for tp (teleport)")]
struct Cli {
    /// Add a portal for the current directory
    #[arg(short = 'a', long = "add", conflicts_with_all = ["remove", "list", "edit", "prune"])]
    add: bool,

    /// Remove a portal
    #[arg(short = 'r', long = "rm", conflicts_with_all = ["add", "list", "edit", "prune"])]
    remove: bool,

    /// List all portals
    #[arg(short = 'l', long = "ls", conflicts_with_all = ["add", "remove", "edit", "prune"])]
    list: bool,

    /// Open config in editor
    #[arg(short = 'e', long = "edit", conflicts_with_all = ["add", "remove", "list", "prune"])]
    edit: bool,

    /// Find and remove broken portals (dry-run by default, use with -f to remove)
    #[arg(short = 'p', long = "prune", conflicts_with_all = ["add", "remove", "list", "edit"])]
    prune: bool,

    /// Actually remove broken portals (use with -p)
    #[arg(short = 'f', long = "force", requires = "prune")]
    force: bool,

    /// Print shell integration code for the given shell
    #[arg(long)]
    init: Option<String>,

    /// Skip worktree picker, go to main worktree
    #[arg(short = 'm', long = "main", conflicts_with_all = ["add", "remove", "list", "edit", "prune", "direct"])]
    main_worktree: bool,

    /// Skip worktree picker, go to the stored path directly (experimental)
    #[arg(short = 'd', long = "direct", conflicts_with_all = ["add", "remove", "list", "edit", "prune", "main_worktree"])]
    direct: bool,

    /// Open Claude after teleporting
    #[arg(short = 'c', long = "claude", conflicts_with_all = ["add", "remove", "list", "edit", "prune"])]
    claude: bool,

    /// Portal name or teleport query
    name: Option<String>,
}

impl Cli {
    fn worktree_mode(&self) -> WorktreeMode {
        if self.direct {
            WorktreeMode::Direct
        } else if self.main_worktree {
            WorktreeMode::MainOnly
        } else {
            WorktreeMode::Picker
        }
    }
}

fn emit_cd_or_exit(name: &str, target: std::path::PathBuf, claude: bool) {
    if !target.exists() {
        eprintln!("Portal '{}' target does not exist: {}", name, target.display());
        process::exit(1);
    }
    let prefix = if claude { "cd+c" } else { "cd" };
    println!("{}:{}", prefix, target.display());
}

fn teleport_to_portal(name: &str, path: &str, mode: WorktreeMode, claude: bool) {
    if matches!(mode, WorktreeMode::Direct) {
        emit_cd_or_exit(name, resolve::expand_tilde(path), claude);
        return;
    }

    let Some(ctx) = portal_worktree_context(path) else {
        emit_cd_or_exit(name, resolve::expand_tilde(path), claude);
        return;
    };

    let worktree_root = if ctx.worktrees.len() > 1 && matches!(mode, WorktreeMode::Picker) {
        let sorted = sorted_worktrees(
            &ctx.worktrees,
            &ctx.main_worktree,
            ctx.current_worktree.as_deref(),
        );
        let entries = fzf::format_worktree_entries(&sorted);
        let display_lines: Vec<String> = entries.iter().map(|(d, _)| d.clone()).collect();
        match fzf::pick(&display_lines, "Select worktree:") {
            Some(idx) => entries[idx].1.clone(),
            None => process::exit(130),
        }
    } else {
        ctx.main_worktree
    };

    emit_cd_or_exit(name, worktree_root.join(&ctx.relative_path), claude);
}

fn pick_and_teleport(
    portals: &std::collections::BTreeMap<String, String>,
    mode: WorktreeMode,
    claude: bool,
) {
    let entries = fzf::format_portal_entries(portals, "* ");
    let display_lines: Vec<String> = entries.iter().map(|(d, _)| d.clone()).collect();
    match fzf::pick(&display_lines, "Teleport:") {
        Some(idx) => {
            let name = &entries[idx].1;
            let path = portals.get(name).unwrap();
            teleport_to_portal(name, path, mode, claude);
        }
        None => process::exit(130),
    }
}

/// Find portals whose name or path contains the query as a case-insensitive substring.
fn find_matching_portals<'a>(config: &'a Config, query: &str) -> Vec<(&'a String, &'a String)> {
    let query_lower = query.to_lowercase();
    config
        .portals
        .iter()
        .filter(|(name, path)| {
            name.to_lowercase().contains(&query_lower)
                || path.to_lowercase().contains(&query_lower)
        })
        .collect()
}

fn cmd_teleport(config: &Config, query: &str, mode: WorktreeMode, claude: bool) {
    if let Some(path) = config.portals.get(query) {
        teleport_to_portal(query, path, mode, claude);
        return;
    }

    let matches = find_matching_portals(config, query);

    match matches.len() {
        0 => {
            eprintln!("No portal matching '{}'", query);
            process::exit(1);
        }
        1 => {
            let (name, path) = matches[0];
            teleport_to_portal(name, path, mode, claude);
        }
        _ => {
            let filtered: std::collections::BTreeMap<String, String> = matches
                .iter()
                .map(|(n, p)| ((*n).clone(), (*p).clone()))
                .collect();
            pick_and_teleport(&filtered, mode, claude);
        }
    }
}

fn cmd_pick(config: &Config) {
    if config.portals.is_empty() {
        eprintln!("No portals configured. Use 'tp -a <name>' to create one.");
        process::exit(1);
    }
    pick_and_teleport(&config.portals, WorktreeMode::Picker, false);
}

fn cmd_add(config: &mut Config, name: Option<String>) {
    let cwd = resolve::logical_cwd();

    let name = match name {
        Some(n) => n,
        None => cwd
            .file_name()
            .expect("current directory has no name")
            .to_str()
            .expect("directory name is not valid UTF-8")
            .to_string(),
    };

    if config.portals.contains_key(&name) {
        eprintln!(
            "Portal '{}' already exists. Use 'tp -a <name>' to specify a different name.",
            name
        );
        process::exit(1);
    }

    let path = resolve::collapse_tilde(&cwd);
    config.add_portal(name.clone(), path);
    config.save();
    println!("Added portal '{}'", name);
}

fn cmd_rm(config: &mut Config, name: Option<String>) {
    let name = match name {
        Some(n) => n,
        None => {
            let cwd = std::env::current_dir().expect("could not determine current directory");
            let matches: Vec<_> = config
                .portals
                .iter()
                .filter(|(_, path)| {
                    resolve::expand_tilde(path)
                        .canonicalize()
                        .ok()
                        .as_ref()
                        == Some(&cwd)
                })
                .map(|(name, _)| name.clone())
                .collect();

            match matches.len() {
                0 => {
                    eprintln!("No portal points to this directory");
                    process::exit(1);
                }
                1 => matches.into_iter().next().unwrap(),
                _ => {
                    eprintln!(
                        "Multiple portals point to this directory: {}. Specify which one with 'tp -r <name>'.",
                        matches.join(", ")
                    );
                    process::exit(1);
                }
            }
        }
    };

    if config.remove(&name) {
        config.save();
        println!("Removed portal '{}'", name);
    } else {
        eprintln!("Portal '{}' not found", name);
        process::exit(1);
    }
}

fn cmd_edit() {
    println!("edit:{}", Config::path().display());
}

fn cmd_prune(config: &mut Config, force: bool) {
    let broken = config.broken_portals();

    if broken.is_empty() {
        println!("All portals are valid.");
        return;
    }

    let lines = fzf::format_prune_entries(&broken);
    let noun = if broken.len() == 1 { "portal" } else { "portals" };

    if force {
        for (name, _) in &broken {
            config.remove(name);
        }
        config.save();
        println!("Removed {} broken {}:", broken.len(), noun);
    } else {
        println!("Found {} broken {}:", broken.len(), noun);
    }

    for line in &lines {
        println!("{}", line);
    }

    if !force {
        println!("Run 'tp -p -f' to remove them.");
    }
}

fn cmd_ls(config: &Config) {
    if config.portals.is_empty() {
        println!("No portals configured. Use 'tp -a <name>' to create one.");
        return;
    }

    let entries = fzf::format_portal_entries(&config.portals, "");
    for (display, _) in &entries {
        println!("{}", display);
    }
}

fn main() {
    let cli = Cli::parse();

    if let Some(shell) = &cli.init {
        match shell.as_str() {
            "zsh" => {
                print!("{}", include_str!("../shell/tp.zsh"));
                return;
            }
            _ => {
                eprintln!("Unsupported shell: {}. Supported: zsh", shell);
                process::exit(1);
            }
        }
    }

    let mut config = Config::load();

    if cli.add {
        cmd_add(&mut config, cli.name);
    } else if cli.remove {
        cmd_rm(&mut config, cli.name);
    } else if cli.list {
        cmd_ls(&config);
    } else if cli.edit {
        cmd_edit();
    } else if cli.prune {
        cmd_prune(&mut config, cli.force);
    } else if let Some(ref name) = cli.name {
        let mode = cli.worktree_mode();
        cmd_teleport(&config, name, mode, cli.claude);
    } else {
        cmd_pick(&config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_zsh_outputs_shell_function() {
        let shell_code = include_str!("../shell/tp.zsh");
        assert!(shell_code.contains("tp()"));
        assert!(shell_code.contains("compdef _tp tp"));
    }

    #[test]
    fn auto_name_from_basename() {
        let path = "/Users/jeff/code/teleport";
        let name = std::path::Path::new(path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(name, "teleport");
    }

    #[test]
    fn auto_name_from_basename_nested() {
        let path = "/Users/jeff/code/my-project/sub";
        let name = std::path::Path::new(path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(name, "sub");
    }

    #[test]
    fn find_portal_by_path_single_match() {
        let mut config = Config::default();
        config.portals.insert("myproject".to_string(), "~/code/myproject".to_string());
        config.portals.insert("notes".to_string(), "~/Documents/notes".to_string());

        let cwd = resolve::expand_tilde("~/code/myproject");
        let matches: Vec<_> = config
            .portals
            .iter()
            .filter(|(_, path)| resolve::expand_tilde(path) == cwd)
            .collect();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "myproject");
    }

    #[test]
    fn find_portal_by_path_no_match() {
        let mut config = Config::default();
        config.portals.insert("notes".to_string(), "~/Documents/notes".to_string());

        let cwd = resolve::expand_tilde("~/code/other");
        let matches: Vec<_> = config
            .portals
            .iter()
            .filter(|(_, path)| resolve::expand_tilde(path) == cwd)
            .collect();

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn find_portal_by_path_multiple_matches() {
        let mut config = Config::default();
        config.portals.insert("proj".to_string(), "~/code/myproject".to_string());
        config.portals.insert("proj2".to_string(), "~/code/myproject".to_string());

        let cwd = resolve::expand_tilde("~/code/myproject");
        let matches: Vec<_> = config
            .portals
            .iter()
            .filter(|(_, path)| resolve::expand_tilde(path) == cwd)
            .collect();

        assert_eq!(matches.len(), 2);
    }
}
