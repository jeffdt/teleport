# tp (teleport)

Directory teleportation tool with two bookmark types: **portals** (absolute paths) and **tunnels** (git repo-relative paths that resolve through worktrees).

## Architecture

Two components:

- **`warp-core`** (Rust binary): handles config parsing, path resolution, worktree discovery, fzf integration. Outputs directives to stdout: `cd:<path>` (shell should cd) or plain text (shell should print). Never calls `cd` itself.
- **`tp`** (zsh function in `shell/tp.zsh`): calls `warp-core`, interprets directives, executes `cd`. Handles `-c` flag (open Claude after teleporting) and `edit` subcommand.

## Key concepts

- **Portal**: absolute path bookmark. Stored as `name = "~/path"` under `[portals]` in config.
- **Tunnel**: repo-relative bookmark. Stored with `repo` (absolute path to main worktree) and `path` (relative subdir) under `[tunnels.<name>]`. Resolves through any worktree of that repo via `git worktree list`.
- **Config**: TOML at `~/.config/tp/portals.toml`. Uses `dirs::home_dir().join(".config")` (XDG style), not `dirs::config_dir()` (which returns `~/Library/Application Support` on macOS).

## Source layout

| File | Responsibility |
|---|---|
| `src/main.rs` | CLI definition (clap), subcommand dispatch |
| `src/config.rs` | TOML types (Config, Tunnel), load/save, add/remove |
| `src/resolve.rs` | Tilde expansion, git worktree list, tunnel resolution, detect_add_context |
| `src/fzf.rs` | Format table rows, spawn fzf subprocess, parse selection |
| `shell/tp.zsh` | Shell wrapper + zsh tab completion |

## Commands

- `tp <name>` teleport to portal or tunnel
- `tp` (no args) fzf picker
- `tp add <name>` create from cwd (auto-detects portal vs tunnel)
- `tp add --abs <name>` force absolute portal
- `tp rm <name>` remove
- `tp ls` list all
- `tp edit` open config in $EDITOR
- `tp -c <name>` teleport then open Claude

## Development

```bash
source "$HOME/.cargo/env"
cargo build                    # build
cargo install --path .         # install to ~/.cargo/bin/
cp shell/tp.zsh ~/shell/common/tp.zsh  # update shell wrapper
```