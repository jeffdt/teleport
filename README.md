# tp

[![CI](https://github.com/jeffdt/teleport/actions/workflows/ci.yml/badge.svg)](https://github.com/jeffdt/teleport/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Directory teleportation with worktree-aware bookmarks.

## Demo

Drop a portal anywhere, jump there instantly from anywhere else:

```bash
$ cd ~/code/user-authentication-service
$ tp -a auth
Added portal 'auth'

$ cd ~
$ tp auth
~/code/user-authentication-service $
```

<!-- gif: add portal and teleport -->

For git repos with multiple worktrees, `tp` shows a picker so you can choose which worktree to land in:

```bash
$ tp auth
  auth  ~/code/user-authentication-service        (main)
> auth  ~/code/user-authentication-service-feat   (current)
```

<!-- gif: worktree picker in action -->

## Install

Requires [rustup](https://rustup.rs) and [fzf](https://github.com/junegunn/fzf).

```bash
cargo install --git https://github.com/jeffdt/teleport
```

Add to your `~/.zshrc`:

```zsh
eval "$(tp-core --init zsh)"
```

## How it works

**Portals** are named bookmarks stored in `~/.config/tp/portals.toml`. Add one from any directory with `tp -a <name>`, then `tp <name>` jumps there from anywhere in your shell. Type just `tp` to open a fuzzy picker over all portals, or a partial name to narrow it down -- exact match wins outright, otherwise a picker opens.

If a portal's target is inside a git repo with multiple worktrees, tp shows a picker so you can choose which worktree to resolve through. The current worktree is pre-selected at the top. Use `-m` to skip straight to the main worktree, or `-d` to go to the stored path directly.

## Usage

```bash
tp                  # fzf picker over all portals
tp auth             # teleport by name (exact or substring match)
tp -a auth          # add a portal for the current directory
tp -a               # add a portal, auto-named from the directory basename
tp -r auth          # remove a portal by name
tp -r               # remove the portal pointing to the current directory
tp -m auth          # skip worktree picker, go to main worktree
tp -d auth          # skip worktree picker, go to stored path directly
tp -c auth          # teleport then open Claude Code
tp -l               # list all portals
tp -e               # open config in $EDITOR
tp -p               # find broken portals (dry-run)
tp -p -f            # remove broken portals
```

## Config

Portals are stored at `~/.config/tp/portals.toml`:

```toml
[portals]
auth    = "~/code/user-authentication-service"
dotfiles = "~/dotfiles"
notes   = "~/Documents/notes"
```

You can edit this directly (`tp -e`) or manage portals through `tp -a` and `tp -r`.
