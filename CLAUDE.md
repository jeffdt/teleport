# tp (teleport)

Directory teleportation tool with worktree-aware bookmarks called **portals**.

## Architecture

Two components:

- **`warp-core`** (Rust binary): handles config parsing, path resolution, worktree discovery, fzf integration. Outputs directives to stdout: `cd:<path>` (shell should cd) or plain text (shell should print). Never calls `cd` itself.
- **`tp`** (zsh function in `shell/tp.zsh`): calls `warp-core`, interprets directives, executes `cd`. Handles `-c` flag (open Claude after teleporting) and `edit` subcommand.

## Key concepts

- **Portal**: a named bookmark to any directory. Stored as `name = "~/path"` under `[portals]` in config. If the path is inside a git repo with multiple worktrees, tp automatically shows a picker to choose which worktree to resolve through.
- **Substring matching**: `tp <query>` first tries an exact portal name match. If none, it searches portal names and paths for a case-insensitive substring match. A single match teleports directly; multiple matches open an fzf picker filtered to just those portals.
- **Worktree awareness**: at teleport time, tp detects if a portal's path is inside a git repo. If the repo has multiple worktrees, a top-down fzf picker appears with colored `(current)` and `(main)` labels. The current worktree is pre-selected at the top. Use `-m` to skip the picker and go straight to the main worktree.
- **Config**: TOML at `~/.config/tp/portals.toml`. Uses `dirs::home_dir().join(".config")` (XDG style), not `dirs::config_dir()` (which returns `~/Library/Application Support` on macOS).

## Source layout

| File | Responsibility |
|---|---|
| `src/main.rs` | CLI definition (clap), subcommand dispatch, substring matching |
| `src/config.rs` | TOML types (Config), load/save, add/remove |
| `src/resolve.rs` | Tilde expansion, git worktree list, portal worktree context, detect_add_context |
| `src/fzf.rs` | Format table rows, spawn fzf subprocess (ANSI-aware, index-based matching), parse selection |
| `shell/tp.zsh` | Shell wrapper + zsh tab completion |

## Commands

- `tp <query>` teleport to portal by exact name or substring match (with worktree picker if multiple worktrees)
- `tp -m <name>` teleport to main worktree (skip picker)
- `tp` (no args) fzf picker
- `tp add <name>` create portal from cwd
- `tp rm <name>` remove
- `tp ls` list all
- `tp edit` open config in $EDITOR
- `tp -c <name>` teleport then open Claude (composes with -m)

## Development

```bash
source "$HOME/.cargo/env"
cargo build                    # build
cargo install --path .         # install to ~/.cargo/bin/
cp shell/tp.zsh ~/shell/common/tp.zsh  # update shell wrapper
```
