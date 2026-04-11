# tp

A directory teleportation tool for the terminal. Navigate to bookmarked directories instantly, with special support for git worktrees.

## What it does

`tp` manages two kinds of bookmarks:

- **Portals**: fixed destinations (absolute paths). `tp notes` always takes you to `~/notes`.
- **Tunnels**: repo-relative destinations that resolve through git worktrees. `tp api` takes you to `src/api` inside whichever worktree of your project repo you're currently in (or lets you pick one if you're not in any).

## Install

Requires Rust and fzf.

```bash
brew install fzf
cargo install --path .
```

Copy the shell wrapper to somewhere your `.zshrc` sources:

```bash
cp shell/tp.zsh ~/your/shell/config/tp.zsh
```

## Usage

```bash
tp notes        # teleport to a portal
tp api          # teleport to a tunnel (picks worktree if needed)
tp              # fzf picker over all bookmarks
tp -c notes     # teleport then open Claude
tp add myplace  # bookmark current directory (auto-detects portal vs tunnel)
tp rm myplace   # remove a bookmark
tp ls           # list all bookmarks
tp edit         # open config in $EDITOR
```

## How `tp add` works

- Outside a git repo: creates a portal (absolute path)
- At a git repo root: creates a portal (absolute path)
- Inside a git repo subdir: creates a tunnel (repo-relative path)
- `tp add --abs <name>`: forces a portal regardless of context

## How tunnels resolve

When you `tp` to a tunnel:

1. If you're already inside a worktree of that repo, it uses that worktree
2. If the repo has only one worktree, it goes there directly
3. If the repo has multiple worktrees, it shows an fzf picker

Worktree discovery uses `git worktree list`, so it works with any worktree layout (sibling dirs, subdirectories, bare repo setups).

## Config

Stored at `~/.config/tp/portals.toml`:

```toml
[portals]
notes = "~/notes"
shell = "~/shell"

[tunnels.api]
repo = "~/projects/my-app"
path = "src/api"
```

## How it works

`tp` is a thin zsh function that calls `warp-core` (the Rust binary). The binary handles all logic and outputs a `cd:/path/to/dir` directive. The shell function interprets it and executes the `cd`. This split is necessary because a subprocess cannot change the parent shell's working directory.
