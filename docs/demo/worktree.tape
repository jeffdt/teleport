# Regenerates docs/images/worktree.gif for the README's Demo section.
#
# Prerequisites:
#   - cargo build --release   (the tape records target/release/tp-core,
#     not whatever brew last installed)
#   - Run from the repo root: vhs docs/demo/worktree.tape
#   - fzf on PATH
#
# No tmux anywhere: tp has no popup, status line, or pane chrome, so this
# uses the direct-pty shape rather than a nested isolated server. fzf runs
# with --height=~50% (src/fzf.rs), so the picker draws inline instead of
# taking the alternate screen. That means no clear-before-launch dance is
# needed, and the picker visibly collapses back into the transcript on close.
#
# Story: opens at ~/Downloads, somewhere unrelated to any repo, because that
# is the premise: portals work from anywhere. `tp auth` brings up three
# worktrees, and the selection walks past main onto feature-oauth rather than
# taking the first row, so the recording shows a choice being made instead of
# an enter-key reflex. The payoff is the prompt: its first line changes from
# ~/Downloads to the feature worktree, which proves the claim with no
# narration.
#
# Picker order as seeded (verified by hand, see the plan's Task 2 Step 5):
#   ~/code/authentication-service                (main)
#   ~/code/authentication-service.feature-oauth
#   ~/code/authentication-service.pr-review

Output "docs/images/worktree.gif"

Set Shell zsh
Set FontSize 14
Set Width 1200
Set Height 600
Set Theme "Tomorrow Night"
Set TypingSpeed 55ms

Source docs/demo/common-setup.tape
Type "cd ~/Downloads && clear"
Enter
Sleep 300ms
Show

Sleep 900ms
Type "tp auth"
Sleep 400ms
Enter
Sleep 1800ms

# Verified seeded order, top to bottom:
#   ~/code/authentication-service                (main)
#   ~/code/authentication-service.feature-oauth
#   ~/code/authentication-service.pr-review
# One Down moves off main onto feature-oauth. Two would overshoot to
# pr-review. Re-verify this order if seed-demo.sh's worktree setup changes.
Down
Sleep 900ms

Enter
Sleep 2200ms

Hide
Type "exit"
Enter
Sleep 300ms
