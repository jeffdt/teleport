# tp

[![CI](https://github.com/jeffdt/teleport/actions/workflows/ci.yml/badge.svg)](https://github.com/jeffdt/teleport/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Directory portals that cut through worktree sprawl.

## Demo

```bash
$ cd ~/code/authentication-service
$ tp -a auth
Added portal 'auth'

$ tp auth
Select worktree:
  3/3
| ~/code/authentication-service.feature-oauth   (current)
  ~/code/authentication-service                 (main)
  ~/code/authentication-service.pr-review
```

<!-- gif: worktree picker in action -->

## Install

Requires [fzf](https://github.com/junegunn/fzf).

```bash
brew install jeffdt/tap/tp
brew install fzf  # if you don't have it already
```

Add to your `~/.zshrc`:

```zsh
eval "$(tp-core --init zsh)"
```

> Apple Silicon only for now. If you have Rust installed, `cargo install --git https://github.com/jeffdt/teleport` works on any platform.

## How it works

**Portals** are named shortcuts to directories. `tp -a <name>` drops one wherever you are; `tp <name>` takes you there from anywhere. Type just `tp` to open a fuzzy picker, or a partial name to narrow it down.

The real power is worktree awareness. If a portal points inside a git repo with multiple worktrees -- common when running parallel agents or juggling feature branches -- tp shows a picker so you land in the right one. One portal per repo, not one per worktree.

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
auth     = "~/code/authentication-service"
dotfiles = "~/dotfiles"
notes   = "~/Documents/notes"
```

You can edit this directly (`tp -e`) or manage portals through `tp -a` and `tp -r`.
