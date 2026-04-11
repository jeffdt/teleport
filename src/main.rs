mod config;
mod fzf;
mod history;
mod resolve;

use std::process;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};

use config::Config;
use resolve::{
    current_worktree_for_repo, expand_tilde, portal_worktree_context, sorted_worktrees,
};

#[derive(Parser)]
#[command(name = "warp-core", about = "Engine for tp (teleport)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Skip worktree picker, go to main worktree
    #[arg(short = 'm', long = "main")]
    main_worktree: bool,

    /// Portal name to teleport to
    name: Option<String>,
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
    /// Log a directory visit (called by shell chpwd hook)
    #[command(hide = true)]
    Log {
        /// Absolute path of the directory visited
        path: String,
    },
}

fn resolve_frecent_path(entry: &history::FrecencyEntry) -> Option<std::path::PathBuf> {
    match &entry.repo {
        Some(repo) => {
            let repo_path = expand_tilde(repo);
            let worktree = current_worktree_for_repo(&repo_path)
                .unwrap_or_else(|| resolve::main_worktree(&repo_path));
            let target = worktree.join(&entry.path);
            if target.exists() { Some(target) } else { None }
        }
        None => {
            let target = expand_tilde(&entry.path);
            if target.exists() { Some(target) } else { None }
        }
    }
}

fn cmd_teleport(config: &Config, name: &str, main_only: bool) {
    if let Some(path) = config.portals.get(name) {
        // Check if portal is inside a git repo (defer existence check until after resolution)
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
                        Some(selected) => entries
                            .iter()
                            .find(|(d, _)| *d == selected)
                            .map(|(_, p)| p.clone())
                            .expect("selected entry not found"),
                        None => process::exit(130),
                    }
                };

                let target = if ctx.relative_path.is_empty() {
                    worktree_root
                } else {
                    worktree_root.join(&ctx.relative_path)
                };

                if !target.exists() {
                    eprintln!(
                        "Portal '{}' target does not exist in selected worktree: {}",
                        name,
                        target.display()
                    );
                    process::exit(1);
                }
                println!("cd:{}", target.display());
                return;
            }
            Some(ctx) if ctx.worktrees.len() == 1 => {
                // Single worktree; resolve through it
                let target = if ctx.relative_path.is_empty() {
                    ctx.worktrees.into_iter().next().unwrap()
                } else {
                    ctx.worktrees.into_iter().next().unwrap().join(&ctx.relative_path)
                };
                if !target.exists() {
                    eprintln!(
                        "Portal '{}' target does not exist: {}",
                        name,
                        target.display()
                    );
                    process::exit(1);
                }
                println!("cd:{}", target.display());
                return;
            }
            _ => {
                // Not a git repo or no worktrees; use resolved path directly
                let resolved = resolve::resolve_portal(path);
                if !resolved.exists() {
                    eprintln!("Portal '{}' target does not exist: {}", name, resolved.display());
                    process::exit(1);
                }
                println!("cd:{}", resolved.display());
                return;
            }
        }
    }

    // Frecency fallback: try substring match
    let tokens: Vec<&str> = name.split_whitespace().collect();
    let matches = history::find_by_substring(&tokens);

    for entry in &matches {
        if let Some(resolved) = resolve_frecent_path(entry) {
            println!("cd:{}", resolved.display());
            return;
        }
        let _ = history::remove_entry(&entry.path, entry.repo.as_deref());
    }

    eprintln!("No match for '{}'", name);
    process::exit(1);
}

enum PickerEntry {
    Bookmark(String),
    Frecent { path: String, repo: Option<String> },
    Separator,
}

fn cmd_pick(config: &Config) {
    let bookmark_entries = fzf::format_bookmark_entries(config);
    let frecent_raw = history::top_frecent(50);

    let bookmark_paths: std::collections::HashSet<std::path::PathBuf> = config
        .portals
        .values()
        .map(|p| resolve::canonicalize_path(&expand_tilde(p)))
        .collect();

    let frecent_filtered: Vec<_> = frecent_raw
        .iter()
        .filter(|entry| {
            let resolved = resolve_frecent_path(entry);
            match resolved {
                Some(p) => !bookmark_paths.contains(&resolve::canonicalize_path(&p)),
                None => false,
            }
        })
        .collect();

    if bookmark_entries.is_empty() && frecent_filtered.is_empty() {
        eprintln!("No portals or frecent directories. Use 'tp add <name>' to create one.");
        process::exit(1);
    }

    let name_width = config
        .portals
        .keys()
        .map(|k| k.len())
        .max()
        .unwrap_or(8);

    let frecent_entries = fzf::format_frecent_entries(&frecent_filtered, name_width);

    let mut display_lines: Vec<String> = Vec::new();
    let mut line_map: Vec<PickerEntry> = Vec::new();

    for (display, name) in &bookmark_entries {
        display_lines.push(display.clone());
        line_map.push(PickerEntry::Bookmark(name.clone()));
    }

    if !bookmark_entries.is_empty() && !frecent_entries.is_empty() {
        let separator = "  -".to_string() + &"-".repeat(60);
        display_lines.push(separator);
        line_map.push(PickerEntry::Separator);
    }

    for (display, entry_ref) in &frecent_entries {
        display_lines.push(display.clone());
        line_map.push(PickerEntry::Frecent {
            path: entry_ref.path.clone(),
            repo: entry_ref.repo.clone(),
        });
    }

    let selected = match fzf::pick(&display_lines, "Teleport:") {
        Some(s) => s,
        None => process::exit(130),
    };

    let idx = display_lines
        .iter()
        .position(|l| *l == selected)
        .expect("selected entry not found");

    match &line_map[idx] {
        PickerEntry::Bookmark(name) => cmd_teleport(config, name, false),
        PickerEntry::Frecent { path, repo } => {
            let entry = history::FrecencyEntry {
                path: path.clone(),
                repo: repo.clone(),
                effective_score: 0.0,
            };
            match resolve_frecent_path(&entry) {
                Some(resolved) => println!("cd:{}", resolved.display()),
                None => {
                    let _ = history::remove_entry(path, repo.as_deref());
                    eprintln!("Path no longer exists, removed from history");
                    process::exit(1);
                }
            }
        }
        PickerEntry::Separator => process::exit(130),
    }
}

const RESERVED_NAMES: &[&str] = &["add", "rm", "ls", "edit", "help", "completions", "log"];

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

fn cmd_ls(config: &Config) {
    if config.portals.is_empty() {
        println!("No portals configured. Use 'tp add <name>' to create one.");
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
        Some(Commands::Add { name }) => cmd_add(&mut config, name),
        Some(Commands::Rm { name }) => cmd_rm(&mut config, name),
        Some(Commands::Ls) => cmd_ls(&config),
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "warp-core", &mut std::io::stdout());
        }
        Some(Commands::Log { path }) => {
            let _ = history::record_visit(&path);
        }
        None => {
            if let Some(name) = cli.name {
                cmd_teleport(&config, &name, cli.main_worktree);
            } else {
                cmd_pick(&config);
            }
        }
    }
}
