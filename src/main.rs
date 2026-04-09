mod config;
mod fzf;
mod resolve;

use std::process;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};

use config::Config;
use resolve::{
    detect_add_context, expand_tilde,
    resolve_portal, tunnel_worktree_context, AddContext,
};

#[derive(Parser)]
#[command(name = "warp-core", about = "Engine for tp (teleport)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Portal or tunnel name to teleport to
    name: Option<String>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Create a portal or tunnel from the current directory
    Add {
        /// Name for the new portal/tunnel
        name: String,
        /// Force absolute path (portal) even in a repo subdir
        #[arg(long)]
        abs: bool,
    },
    /// Remove a portal or tunnel
    Rm {
        /// Name to remove
        name: String,
    },
    /// List all portals and tunnels
    Ls,
    /// Generate shell completions (hidden)
    #[command(hide = true)]
    Completions {
        shell: Shell,
    },
}

fn cmd_teleport(config: &Config, name: &str) {
    // Check portals first
    if let Some(path) = config.portals.get(name) {
        let resolved = resolve_portal(path);
        if !resolved.exists() {
            eprintln!("Portal '{}' target does not exist: {}", name, resolved.display());
            process::exit(1);
        }
        println!("cd:{}", resolved.display());
        return;
    }

    // Check tunnels
    if let Some(tunnel) = config.tunnels.get(name) {
        let (worktrees, current) = tunnel_worktree_context(tunnel);

        let worktree_root = if let Some(current) = current {
            // Already inside a worktree of this repo
            current
        } else if worktrees.len() == 1 {
            // Only one worktree, use it
            worktrees.into_iter().next().unwrap()
        } else if worktrees.is_empty() {
            eprintln!("No worktrees found for repo: {}", tunnel.repo);
            process::exit(1);
        } else {
            // Multiple worktrees, fzf picker
            let display_lines = fzf::format_worktrees(&worktrees);
            match fzf::pick(&display_lines, "Select worktree:") {
                Some(selected) => {
                    let selected_path = expand_tilde(selected.trim());
                    selected_path
                }
                None => {
                    process::exit(130);
                }
            }
        };

        let target = worktree_root.join(&tunnel.path);
        if !target.exists() {
            eprintln!(
                "Tunnel '{}' target does not exist: {}",
                name,
                target.display()
            );
            process::exit(1);
        }
        println!("cd:{}", target.display());
        return;
    }

    eprintln!("Unknown portal or tunnel: '{}'", name);
    process::exit(1);
}

fn cmd_pick(config: &Config) {
    let entries = fzf::format_entries(config);
    if entries.is_empty() {
        eprintln!("No portals or tunnels configured. Use 'tp add <name>' to create one.");
        process::exit(1);
    }

    let display_lines: Vec<String> = entries.iter().map(|(display, _)| display.clone()).collect();
    let selected = match fzf::pick(&display_lines, "Select portal:") {
        Some(s) => s,
        None => process::exit(130),
    };

    // Find the name from the selected display line
    let name = entries
        .iter()
        .find(|(display, _)| *display == selected)
        .map(|(_, name)| name.clone())
        .expect("selected entry not found");

    cmd_teleport(config, &name);
}

const RESERVED_NAMES: &[&str] = &["add", "rm", "ls", "edit", "help", "completions"];

fn cmd_add(config: &mut Config, name: String, abs: bool) {
    if RESERVED_NAMES.contains(&name.as_str()) {
        eprintln!("'{}' is a reserved command name", name);
        process::exit(1);
    }
    if config.portals.contains_key(&name) || config.tunnels.contains_key(&name) {
        eprintln!("'{}' already exists. Remove it first with 'tp rm {}'.", name, name);
        process::exit(1);
    }

    match detect_add_context(abs) {
        AddContext::Portal(path) => {
            config.add_portal(name.clone(), path);
            config.save();
            println!("Added portal '{}'", name);
        }
        AddContext::Tunnel { repo, path } => {
            config.add_tunnel(name.clone(), repo, path);
            config.save();
            println!("Added tunnel '{}'", name);
        }
    }
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

fn cmd_ls(config: &Config) {
    if config.portals.is_empty() && config.tunnels.is_empty() {
        println!("No portals or tunnels configured. Use 'tp add <name>' to create one.");
        return;
    }

    let entries = fzf::format_entries(config);
    for (display, _) in &entries {
        println!("{}", display);
    }
}

fn main() {
    let cli = Cli::parse();

    let mut config = Config::load();

    match cli.command {
        Some(Commands::Add { name, abs }) => cmd_add(&mut config, name, abs),
        Some(Commands::Rm { name }) => cmd_rm(&mut config, name),
        Some(Commands::Ls) => cmd_ls(&config),
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "warp-core", &mut std::io::stdout());
        }
        None => {
            if let Some(name) = cli.name {
                cmd_teleport(&config, &name);
            } else {
                cmd_pick(&config);
            }
        }
    }
}
