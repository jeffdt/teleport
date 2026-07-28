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

## Packaging and distribution

tp ships as a prebuilt binary through a personal Homebrew tap:

- A `v*` git tag triggers `release.yml`, which builds the `aarch64-apple-darwin`
  binary and attaches it to the GitHub Release as `tp-core-aarch64-apple-darwin`.
- `jeffdt/homebrew-tap` carries `Formula/tp.rb`, a binary formula that
  downloads that asset by pinned `sha256`. Install with
  `brew install jeffdt/tap/tp`. The installed binary is named `tp-core`; the
  `tp` shell function itself is not packaged (see "Shell integration is
  embedded" above).

### Cutting a release

**Every push to `main` that changes shipped behavior must also cut a release.**
Users install via Homebrew, which only ever sees tagged release binaries, never
`main`. A commit on `main` with no accompanying release is invisible to anyone
who runs `brew upgrade`: the code is "shipped" in git but not to users. So
unless a change is purely internal (docs, tests, CI, scratch under `specs/` or
`plans/`), finish the job by running the steps below in the same session: bump,
tag, wait for CI, and update the tap. Don't leave `main` ahead of the latest
release.

Shipped changes reach `main` via PR, and the version bump rides in that PR.
Once it has merged, cut the tag and update the tap. The tap is a separate
repo, `jeffdt/homebrew-tap`; clone it if it isn't already checked out.
`.claude/skills/cutting-a-release/release.sh` expects it at
`~/code/homebrew-tap`; set `TP_TAP_DIR` if it lives elsewhere.

`.claude/skills/cutting-a-release/release.sh` automates the mechanical
steps:

1. On the feature branch, before opening the PR:
   `.claude/skills/cutting-a-release/release.sh bump <patch|minor|major>`.
   Reads the current version from `Cargo.toml`, applies the bump, refreshes
   `Cargo.lock` (`cargo build --release`), and commits. That commit rides in
   the PR as usual. Picking `patch` vs `minor` vs `major` is the one call
   the script doesn't make for you -- same judgment as always (a bug fix is
   patch, new user-facing behavior like `tp .` is minor).
2. After the PR merges: `git checkout main && git pull`, then
   `.claude/skills/cutting-a-release/release.sh cut`. It reads the version
   already on `main` (no bump decision left -- that was step 1), tags and
   pushes `vX.Y.Z`, waits for
   `release.yml` (which builds and attaches a single asset named
   **`tp-core-aarch64-apple-darwin`** to the GitHub Release), downloads and
   hashes that asset, updates and validates `jeffdt/homebrew-tap`'s
   `Formula/tp.rb`, pushes the tap, and runs `brew update && brew upgrade
   jeffdt/tap/tp` locally, ending on a confirmed `tp-core --version`. It
   refuses to run off `main`, with a dirty tree, or against a tag that
   already exists, rather than guessing.

The formula carries `depends_on arch: :arm64` and `depends_on :macos` and a
top-level `url` (the version is scanned from the URL, e.g.
`.../download/vX.Y.Z/tp-core-...`; there is no separate `version` line) so the
tap's `brew test-bot` CI passes -- keep that shape by hand if editing the
formula outside the script. `release.sh cut` only ever rewrites the `url` and
`sha256` lines.

Currently Apple Silicon only. Supporting Intel means adding
`x86_64-apple-darwin` to the release matrix, an Intel branch in the formula,
and updating `release.sh`'s asset handling.
