# tp — worktree-aware directory bookmarking

> Instant terminal teleportation. Bookmarks that follow you across git worktrees.

[![License: Unlicense](https://img.shields.io/badge/license-Unlicense-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)

---

`tp` manages two kinds of bookmarks:

- **Portals** — fixed destinations (absolute paths). `tp app` always takes you to `~/r/app`.
- **Tunnels** — repo-relative paths that resolve through git worktrees. `tp is` takes you to `python/klaviyo/.../insights_service` inside whichever worktree you're currently in — or lets you pick one if you're not in any.

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
tp app          # teleport to a portal
tp is           # teleport to a tunnel (picks worktree if needed)
tp              # fzf picker over all bookmarks
tp -c app       # teleport then open Claude
tp add myplace  # bookmark cwd (auto-detects portal vs tunnel)
tp rm myplace   # remove a bookmark
tp ls           # list all bookmarks
tp edit         # open config in $EDITOR
```

## How `tp add` works

- Outside a git repo → portal (absolute path)
- At a git repo root → portal (absolute path)
- Inside a git repo subdir → tunnel (repo-relative path)
- `tp add --abs <name>` → force a portal regardless

## How tunnels resolve

When you jump to a tunnel:

1. Already inside a worktree of that repo? Goes there directly.
2. Repo has one worktree? Goes there directly.
3. Repo has multiple worktrees? fzf picker.

Worktree discovery uses `git worktree list`, so it works with any layout — sibling dirs, subdirectories, bare repos.

## Config

`~/.config/tp/portals.toml`:

```toml
[portals]
app = "~/r/app"
shell = "~/shell"

[tunnels.is]
repo = "~/r/k-repo"
path = "python/klaviyo/executive_business_report/insights_service"
```

## How it works

`tp` is a thin zsh function wrapping `warp-core`, a Rust binary. The binary handles all logic and prints a `cd:/path/to/dir` directive; the shell function interprets it and executes the `cd`. The split exists because a subprocess can't change the parent shell's directory.

## License

Public domain — see [LICENSE](LICENSE).
