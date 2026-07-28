# Regenerates docs/images/manage.gif for the README's Manage portals section.
#
# Prerequisites: same as worktree.tape (cargo build --release, run from the
# repo root, fzf on PATH).
#
# Story: stands in ~/code/billing-api, which seed-demo.sh deliberately leaves
# without a portal, so `tp -a api` adds a genuinely new one rather than a
# duplicate. `tp -l` then shows it sitting alongside the two seeded dead
# portals, `tp -p` names them, and `tp -p -f` sweeps them. One tape, two
# features, one story: portals are cheap to add and cheap to clean up.
#
# Prune deliberately has no tape of its own. Its output is static text with
# no interaction, so a standalone GIF of it would be a code block that costs
# a re-record every time the output format changes, while being worse than a
# code block in every other way (not greppable, not copy-pasteable).

Output "docs/images/manage.gif"

Set Shell zsh
Set FontSize 14
Set Width 520
Set Height 750
Set Theme "Tomorrow Night"
Set TypingSpeed 55ms

Source docs/demo/common-setup.tape
Type "cd ~/code/billing-api && clear"
Enter
Sleep 300ms
Show

Sleep 900ms
Type "tp -a api"
Sleep 300ms
Enter
Sleep 1400ms

Type "tp -l"
Sleep 300ms
Enter
Sleep 2200ms

Type "tp -p"
Sleep 300ms
Enter
Sleep 2000ms

Type "tp -p -f"
Sleep 300ms
Enter
Sleep 1800ms

Type "tp -l"
Sleep 300ms
Enter
Sleep 2200ms

Hide
Type "exit"
Enter
Sleep 300ms
