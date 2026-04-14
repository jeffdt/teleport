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
eval "$(warp-core --init zsh)"
```

## Usage

```bash
tp blog         # teleport to a portal by exact name
tp dot          # substring match: jumps directly if one match, picker if multiple
tp              # fzf picker over all portals
tp -m blog      # skip worktree picker, go straight to main worktree
tp -c blog      # teleport then open Claude Code
tp add myplace  # create a portal from current directory
tp rm myplace   # remove a portal
tp ls           # list all portals
tp edit         # open config in $EDITOR
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

`tp` is a zsh function that calls `warp-core` (the Rust binary). The binary handles config, path resolution, worktree discovery, and fzf integration, then outputs a `cd:/path` directive. The shell function interprets it and runs `cd`. This split exists because a subprocess cannot change the parent shell's working directory.
