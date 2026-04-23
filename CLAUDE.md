# tp (teleport)

Directory teleportation tool with worktree-aware bookmarks called **portals**.

## Architecture

Two components that split along a hard boundary: a subprocess cannot change the parent shell's working directory.

- **`tp-core`** (Rust binary): all logic lives here. Config, path resolution, worktree discovery, fzf pickers. Outputs directives to stdout (`cd:`, `cd+c:`, `edit:`, or plain text) but never performs shell actions itself.
- **`tp`** (zsh function, embedded in the binary via `--init`): pure dispatcher. Calls `tp-core`, pattern-matches on the directive prefix, executes the shell-level action. No branching logic of its own.

## Key concepts

- **Portal**: a named bookmark to any directory. Stored as `name = "~/path"` under `[portals]` in config.
- **Substring matching**: `tp <query>` tries exact name match first, then case-insensitive substring across names and paths. Single match teleports directly; multiple matches open an fzf picker.
- **Worktree awareness**: if a portal's path is inside a git repo with multiple worktrees, tp shows a picker to choose which worktree to resolve through. `-m` skips the picker and goes to the main worktree; `-d` skips it and goes to the stored path directly.
- **Config path**: `~/.config/tp/portals.toml`. Uses `dirs::home_dir().join(".config")` (XDG style), not `dirs::config_dir()` (which returns `~/Library/Application Support` on macOS).

## Key gotchas

- **Shell integration is embedded**: `shell/tp.zsh` is compiled into the binary via `include_str!` and served by `tp-core --init zsh`. There is no separate install step for the shell wrapper. Users add `eval "$(tp-core --init zsh)"` to their `.zshrc`.
- **Directive protocol**: tp-core communicates with the shell function through a line-oriented protocol. Adding a new directive means updating both the Rust `emit_*` call and the `case` statement in `tp.zsh`.
- **fzf is required at runtime**: tp will error with an install hint if fzf is not found. No fallback picker exists.
- **No `clap_complete`**: shell completions are hand-rolled in `tp.zsh` (calls `tp-core -l` and extracts names). The `clap_complete` crate is not a dependency.

## Development

```bash
source "$HOME/.cargo/env"
cargo build                    # build
cargo run -- <args>            # test tp-core without installing (avoids worktree binary collisions)
cargo install --path .         # install to ~/.cargo/bin/
```
