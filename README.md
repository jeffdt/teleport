# tp

[![CI](https://github.com/jeffdt/teleport/actions/workflows/ci.yml/badge.svg)](https://github.com/jeffdt/teleport/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built_with-rust-orange.svg)](https://www.rust-lang.org)

Directory portals that cut through worktree sprawl.

## Demo

Add a portal for the current directory:

```bash
> ~/code/authentication-service
$ tp -a auth
Added portal 'auth'
```

Then jump to it from anywhere -- if the repo has multiple worktrees, tp picks one:

```bash
> ~/Downloads
$ tp auth
Select worktree:
  3/3
| ~/code/authentication-service.feature-oauth
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

**Navigate**

```bash
tp                  # pick a portal from a list and jump there
tp auth             # jump to the auth portal
tp .                # open the worktree picker for the repo you're standing in
tp -w auth          # jump to auth and choose a worktree
tp -d auth          # jump to auth, skip the worktree picker
tp -c auth          # jump to auth and open Claude Code
tp -u               # pick a portal nested under the current directory
tp -l -u            # list portals nested under the current directory
```

**Manage portals**

```bash
tp -a [name]        # add a portal for the current directory (auto-named if omitted)
tp -r [name]        # remove a portal (defaults to the one for the current directory)
tp -l               # list all portals
tp -e               # open config in $EDITOR
```

**Maintenance**

```bash
tp -p               # find broken portals (dry-run)
tp -p -f            # remove broken portals
```

## Config

Config lives at `~/.config/tp/config.toml`:

```toml
[settings]
default_nav_mode = "picker"  # "picker" or "direct"

[portals]
auth     = "~/code/authentication-service"
dotfiles = "~/dotfiles"
notes    = "~/Documents/notes"
```

By default, jumping to a portal in a multi-worktree repo opens a picker so you can choose which worktree to land in. If you prefer to always go straight to the stored path without being asked, set `default_nav_mode = "direct"`. You can still override on the fly: `-w` forces the picker, `-d` goes direct.

`tp -a` and `tp -r` manage the portals section for you, but you can always edit the file directly with `tp -e`.

## Development

```sh
cargo build
cargo test
cargo run -- <args>   # test tp-core without installing
```

This repo also ships two Claude Code skills for working on it visually:
`mockup`, for comparing ANSI mockups of a design change before
implementing it, and `live-preview`, for popping the freshly built binary
open in a real tmux window once a feature is done. Both come from the
`tui-utils` plugin:

```
/plugin marketplace add jeffdt/tui-utils
/plugin install tui-utils@tui-utils
```
