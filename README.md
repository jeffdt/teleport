# tp

[![CI](https://github.com/jeffdt/teleport/actions/workflows/ci.yml/badge.svg)](https://github.com/jeffdt/teleport/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)

A directory teleportation tool for the terminal. Create portals in your favorite directories and jump to them instantly, with built-in git worktree support.

> **Heads up:** tp is under active development. Commands, config format, and behavior may change without notice, and there are no guarantees of backwards compatibility between versions. Once the core workflow settles, stability will be prioritized.

## Install

Requires Rust and fzf.

```bash
brew install fzf
cargo install --path .
```

Add to your `~/.zshrc`:

```zsh
eval "$(tp-core --init zsh)"
```

## Usage

```bash
tp blog         # teleport to a portal by exact name
tp dot          # substring match: jumps directly if one match, picker if multiple
tp              # fzf picker over all portals
tp -m blog      # skip worktree picker, go straight to main worktree
tp -d blog      # skip worktree picker, go to the stored path directly (experimental)
tp -c blog      # teleport then open Claude Code
tp -a myplace   # create a portal from current directory (auto-names from basename if omitted)
tp -r myplace   # remove a portal (removes by cwd match if name omitted)
tp -l           # list all portals
tp -e           # open config in $EDITOR
tp -p           # find broken portals (dry-run)
tp -p -f        # remove broken portals
```

## Worktree support

If a portal points inside a git repo that has multiple worktrees, tp shows an fzf picker so you can choose which worktree to resolve through. The current worktree is pre-selected at the top, with colored `(current)` and `(main)` labels. If the repo has only one worktree, tp goes there directly.

Use `-m` to skip the picker and always land in the main worktree.

## Config

Stored at `~/.config/tp/portals.toml`:

```toml
[portals]
dotfiles = "~/dotfiles"
blog = "~/projects/blog"
notes = "~/Documents/notes"
```

## How it works

`tp` is a zsh function that calls `tp-core` (the Rust binary). The binary handles config, path resolution, worktree discovery, and fzf integration, then outputs directives to stdout: `cd:/path` (change directory), `cd+c:/path` (change directory and open Claude), or `edit:/path` (open file in `$EDITOR`). The shell function interprets these and executes the corresponding shell-level action. This split exists because a subprocess cannot change the parent shell's working directory.
