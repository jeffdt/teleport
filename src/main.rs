mod config;
mod fzf;
mod resolve;

use std::process;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};

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
    #[command(subcommand)]
    command: Option<Commands>,

    /// Skip worktree picker, go to main worktree
    #[arg(short = 'm', long = "main", conflicts_with = "direct")]
    main_worktree: bool,

    /// Skip worktree picker, go to the stored path directly
    #[arg(short = 'd', long = "direct", conflicts_with = "main_worktree")]
    direct: bool,

    /// Portal name to teleport to
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

#[derive(clap::Subcommand)]
enum Commands {
    /// Create a portal from the current directory
    Add {
        /// Name for the new portal
        name: String,
    },
    /// Remove a portal
    Rm {
        /// Name to remove
        name: String,
    },
    /// List all portals
    Ls,
    /// Generate shell completions (hidden)
    #[command(hide = true)]
    Completions {
        shell: Shell,
    },
    /// Find and remove broken portals
    Prune {
        /// Actually remove broken portals (default is dry-run)
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
}

fn emit_cd_or_exit(name: &str, target: std::path::PathBuf) {
    if !target.exists() {
        eprintln!("Portal '{}' target does not exist: {}", name, target.display());
        process::exit(1);
    }
    println!("cd:{}", target.display());
}

fn teleport_to_portal(name: &str, path: &str, mode: WorktreeMode) {
    if matches!(mode, WorktreeMode::Direct) {
        emit_cd_or_exit(name, resolve::resolve_portal(path));
        return;
    }

    match portal_worktree_context(path) {
        Some(ctx) if ctx.worktrees.len() > 1 => {
            let worktree_root = if matches!(mode, WorktreeMode::MainOnly) {
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

fn cmd_teleport(config: &Config, query: &str, mode: WorktreeMode) {
    if let Some(path) = config.portals.get(query) {
        teleport_to_portal(query, path, mode);
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
            teleport_to_portal(name, path, mode);
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
                    teleport_to_portal(name, path, mode);
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
            teleport_to_portal(name, path, WorktreeMode::Picker);
        }
        None => process::exit(130),
    }
}

const RESERVED_NAMES: &[&str] = &["add", "rm", "ls", "edit", "help", "completions", "prune"];

fn cmd_add(config: &mut Config, name: String) {
    if RESERVED_NAMES.contains(&name.as_str()) {
        eprintln!("'{}' is a reserved command name", name);
        process::exit(1);
    }
    if config.portals.contains_key(&name) {
        eprintln!("'{}' already exists. Remove it first with 'tp rm {}'.", name, name);
        process::exit(1);
    }

    let path = resolve::detect_add_context();
    config.add_portal(name.clone(), path);
    config.save();
    println!("Added portal '{}'", name);
}

fn cmd_rm(config: &mut Config, name: String) {
    if config.remove(&name) {
        config.save();
        println!("Removed '{}'", name);
    } else {
        eprintln!("'{}' not found", name);
        process::exit(1);
    }
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
        println!("Run 'tp prune -f' to remove them.");
    }
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

    match cli.command {
        Some(Commands::Add { name }) => cmd_add(&mut config, name),
        Some(Commands::Rm { name }) => cmd_rm(&mut config, name),
        Some(Commands::Ls) => cmd_ls(&config),
        Some(Commands::Prune { force }) => cmd_prune(&mut config, force),
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "warp-core", &mut std::io::stdout());
        }
        None => {
            let mode = cli.worktree_mode();
            if let Some(name) = cli.name {
                cmd_teleport(&config, &name, mode);
            } else {
                cmd_pick(&config);
            }
        }
    }
}
