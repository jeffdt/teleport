mod config;
mod fzf;
mod resolve;

use std::process;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};

use config::Config;
use resolve::{portal_worktree_context, sorted_worktrees};

#[derive(Parser)]
#[command(name = "warp-core", version, about = "Engine for tp (teleport)")]
struct Cli {
    /// Add a portal for the current directory
    #[arg(short = 'a', long = "add", conflicts_with_all = ["remove", "list", "edit", "completions"])]
    add: bool,

    /// Remove a portal
    #[arg(short = 'r', long = "rm", conflicts_with_all = ["add", "list", "edit", "completions"])]
    remove: bool,

    /// List all portals
    #[arg(short = 'l', long = "ls", conflicts_with_all = ["add", "remove", "edit", "completions"])]
    list: bool,

    /// Open config in editor
    #[arg(short = 'e', long = "edit", conflicts_with_all = ["add", "remove", "list", "completions"])]
    edit: bool,

    /// Skip worktree picker, go to main worktree
    #[arg(short = 'm', long = "main")]
    main_worktree: bool,

    /// Open Claude after teleporting
    #[arg(short = 'c', long = "claude")]
    claude: bool,

    /// Generate shell completions
    #[arg(long = "completions", conflicts_with_all = ["add", "remove", "list", "edit"])]
    completions: Option<Shell>,

    /// Portal name or teleport query
    name: Option<String>,
}

fn emit_cd_or_exit(name: &str, target: std::path::PathBuf) {
    if !target.exists() {
        eprintln!("Portal '{}' target does not exist: {}", name, target.display());
        process::exit(1);
    }
    println!("cd:{}", target.display());
}

/// Teleport to a known portal by name, handling worktree resolution.
fn teleport_to_portal(name: &str, path: &str, main_only: bool) {
    match portal_worktree_context(path) {
        Some(ctx) if ctx.worktrees.len() > 1 => {
            let worktree_root = if main_only {
                ctx.main_worktree
            } else {
                let sorted = sorted_worktrees(
                    &ctx.worktrees,
                    &ctx.main_worktree,
                    ctx.current_worktree.as_deref(),
                );
                let entries = fzf::format_worktree_entries(&sorted);
                let display_lines: Vec<String> =
                    entries.iter().map(|(d, _)| d.clone()).collect();

                match fzf::pick(&display_lines, "Select worktree:") {
                    Some(idx) => entries[idx].1.clone(),
                    None => process::exit(130),
                }
            };

            let target = if ctx.relative_path.is_empty() {
                worktree_root
            } else {
                worktree_root.join(&ctx.relative_path)
            };
            emit_cd_or_exit(name, target);
        }
        Some(ctx) if ctx.worktrees.len() == 1 => {
            let wt = ctx.worktrees.into_iter().next().unwrap();
            let target = if ctx.relative_path.is_empty() {
                wt
            } else {
                wt.join(&ctx.relative_path)
            };
            emit_cd_or_exit(name, target);
        }
        _ => {
            emit_cd_or_exit(name, resolve::resolve_portal(path));
        }
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

fn cmd_teleport(config: &Config, query: &str, main_only: bool, _claude: bool) {
    if let Some(path) = config.portals.get(query) {
        teleport_to_portal(query, path, main_only);
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
            teleport_to_portal(name, path, main_only);
        }
        _ => {
            let filtered: std::collections::BTreeMap<String, String> = matches
                .iter()
                .map(|(n, p)| ((*n).clone(), (*p).clone()))
                .collect();
            let entries = fzf::format_portal_entries(&filtered, "* ");
            let display_lines: Vec<String> = entries.iter().map(|(d, _)| d.clone()).collect();

            match fzf::pick(&display_lines, "Teleport:") {
                Some(idx) => {
                    let name = &entries[idx].1;
                    let path = config.portals.get(name).unwrap();
                    teleport_to_portal(name, path, main_only);
                }
                None => process::exit(130),
            }
        }
    }
}

fn cmd_pick(config: &Config) {
    let entries = fzf::format_portal_entries(&config.portals, "* ");

    if entries.is_empty() {
        eprintln!("No portals configured. Use 'tp add <name>' to create one.");
        process::exit(1);
    }

    let display_lines: Vec<String> = entries.iter().map(|(d, _)| d.clone()).collect();

    match fzf::pick(&display_lines, "Teleport:") {
        Some(idx) => {
            let name = &entries[idx].1;
            let path = config.portals.get(name).unwrap();
            teleport_to_portal(name, path, false);
        }
        None => process::exit(130),
    }
}

fn cmd_add(config: &mut Config, name: Option<String>) {
    let name = match name {
        Some(n) => n,
        None => {
            let cwd = std::env::current_dir().expect("could not determine current directory");
            let basename = cwd
                .file_name()
                .expect("current directory has no name")
                .to_str()
                .expect("directory name is not valid UTF-8")
                .to_string();
            basename
        }
    };

    if config.portals.contains_key(&name) {
        eprintln!(
            "Portal '{}' already exists. Use 'tp -a <name>' to specify a different name.",
            name
        );
        process::exit(1);
    }

    let path = resolve::detect_add_context();
    config.add_portal(name.clone(), path);
    config.save();
    println!("Added portal '{}'", name);
}

#[cfg(test)]
mod tests {
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
}

fn cmd_rm(config: &mut Config, name: Option<String>) {
    let name = name.expect("Portal name required for rm");
    if config.remove(&name) {
        config.save();
        println!("Removed '{}'", name);
    } else {
        eprintln!("'{}' not found", name);
        process::exit(1);
    }
}

fn cmd_edit() {
    println!("edit:{}", Config::path().display());
}

fn cmd_ls(config: &Config) {
    if config.portals.is_empty() {
        println!("No portals configured. Use 'tp add <name>' to create one.");
        return;
    }

    let entries = fzf::format_portal_entries(&config.portals, "");
    for (display, _) in &entries {
        println!("{}", display);
    }
}

fn main() {
    let cli = Cli::parse();

    let mut config = Config::load();

    if let Some(shell) = cli.completions {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "warp-core", &mut std::io::stdout());
    } else if cli.add {
        cmd_add(&mut config, cli.name);
    } else if cli.remove {
        cmd_rm(&mut config, cli.name);
    } else if cli.list {
        cmd_ls(&config);
    } else if cli.edit {
        cmd_edit();
    } else if let Some(name) = cli.name {
        cmd_teleport(&config, &name, cli.main_worktree, cli.claude);
    } else {
        cmd_pick(&config);
    }
}
