#!/usr/bin/env bash
# Seeds a throwaway $HOME for the docs/demo tapes and prints its path.
#
# Usage (from a tape, repo root as cwd):
#   export HOME=$(docs/demo/seed-demo.sh)
#
# Everything is disposable: a fresh mktemp -d per recording, no shared
# sandbox to keep in sync (contrast boomerang's seed-issues.sh, which resets
# state in a real GitHub repo).
#
# tp resolves BOTH its config path (src/config.rs) and its tilde-collapsing
# display logic (src/resolve.rs) through dirs::home_dir(), so redirecting HOME
# isolates config and display in one move. Side benefit: repos seeded under
# the scratch home render on screen as clean ~/code/... paths.
#
# This script points HOME at the scratch dir before doing any git work, so the
# seeded repos are hermetic: no global hooks, no templatedir, no commit
# signing. That also means git has no committer identity, so every git call
# below must pass one inline or the commits fail outright.
set -euo pipefail

# `cd ... && pwd -P` resolves the scratch dir to its physical path. On macOS
# mktemp -d hands back /var/folders/..., but /var is a symlink to /private/var,
# and `git worktree list` reports canonicalized paths. Without this, $HOME and
# git disagree, tp's collapse_tilde cannot strip the prefix, and the worktree
# picker renders raw /private/var/folders/... paths on screen instead of
# ~/code/... entries.
home=$(cd "$(mktemp -d)" && pwd -P)
export HOME="$home"

git_demo() {
  git -c user.name="tp demo" \
      -c user.email="demo@example.com" \
      -c commit.gpgsign=false \
      -c init.defaultBranch=main \
      -c advice.detachedHead=false \
      "$@"
}

make_repo() {
  local dir="$home/code/$1"
  mkdir -p "$dir"
  git_demo -C "$dir" init -q
  printf '# %s\n' "$1" > "$dir/README.md"
  git_demo -C "$dir" add -A
  git_demo -C "$dir" commit -q -m "Initial commit"
}

mkdir -p "$home/code" "$home/dotfiles" "$home/Documents/notes" "$home/Downloads"

make_repo authentication-service
make_repo web-dashboard
make_repo docs-site
# billing-api deliberately has NO seeded portal. manage.tape stands in it to
# run `tp -a api`, so that tape adds a genuinely new portal rather than a
# duplicate of a seeded one.
make_repo billing-api

# Real worktrees, not fabricated directories: tp shells out to a real
# `git worktree list` (src/resolve.rs).
git_demo -C "$home/code/authentication-service" \
  worktree add -q -b feature-oauth "$home/code/authentication-service.feature-oauth"
git_demo -C "$home/code/authentication-service" \
  worktree add -q -b pr-review "$home/code/authentication-service.pr-review"

mkdir -p "$home/.config/tp"
cat > "$home/.config/tp/config.toml" <<'EOF'
[settings]
default_nav_mode = "picker"

[portals]
auth     = "~/code/authentication-service"
web      = "~/code/web-dashboard"
docs     = "~/code/docs-site"
dotfiles = "~/dotfiles"
notes    = "~/Documents/notes"
payments = "~/archive/payments-service"
spike    = "~/archive/spike-oauth"
EOF

# payments and spike point at directories that are never created, so `tp -p`
# has two corpses to find. Two rather than one so the swept output reads as
# plural.
#
# They sit under ~/archive rather than ~/code on purpose. `-u` is a pure path
# prefix match with no existence check (src/main.rs), so broken portals under
# ~/code would pass the filter and `tp -l -u` would return five rows instead
# of the three under-cwd.tape is built around. ~/archive itself is never
# created either; only the paths' prefix matters here.

# The tapes run `zsh -f` (no rc files, so a themed prompt cannot fight
# `Set Theme`), which means the prompt must be set explicitly. It is written
# here rather than typed in the tape so no vhs escape-sequence handling can
# mangle the embedded newlines. The shape matches the README's code blocks,
# and it is load-bearing: the entire claim of a tp GIF is "you are now
# somewhere else", and the %~ line is the evidence.
cat > "$home/.tp-demo-prompt" <<'EOF'
PROMPT='
> %~
$ '
EOF

echo "$home"
