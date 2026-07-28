# Shared hidden setup for the docs/demo tapes. Sourced, not copied, so the
# setup has one home instead of three that drift.
#
# Ends still inside Hide, with no `cd` and no `Show`: each tape supplies its
# own starting directory, its own `clear`, and its own `Show`. Sourcing a
# tape inlines its commands, so the Hide opened here stays in effect across
# the boundary (verified: sourced setup does not render).
#
# Two vhs parser gotchas, both learned the hard way:
#   - Do NOT rename this file to start with an underscore. vhs's lexer
#     rejects `Source _setup.tape` with "Invalid command: setup.tape".
#   - The Source path resolves against the process working directory, not
#     the sourcing tape's directory, which is why every tape spells it
#     `docs/demo/common-setup.tape` and must be run from the repo root.
#     That matches how `Output "docs/images/..."` already behaves.

Hide
Type "exec zsh -f"
Enter
Sleep 500ms
Type 'unset FZF_DEFAULT_OPTS FZF_DEFAULT_COMMAND'
Enter
Sleep 200ms
Type 'TP_REPO=$PWD; export HOME=$($TP_REPO/docs/demo/seed-demo.sh) || exit 1'
Enter
Sleep 2s
Type '[ -d "$HOME/code" ] || exit 1'
Enter
Sleep 200ms
Type '[ -x "$TP_REPO/target/release/tp-core" ] || exit 1'
Enter
Sleep 200ms
Type 'export PATH=$TP_REPO/target/release:$PATH'
Enter
Sleep 200ms
Type 'eval "$(tp-core --init zsh)"'
Enter
Sleep 400ms
Type "source $HOME/.tp-demo-prompt"
Enter
Sleep 200ms
