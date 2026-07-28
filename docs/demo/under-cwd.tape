# Regenerates docs/images/under-cwd.gif for the README's Navigate section.
#
# Prerequisites: same as worktree.tape.
#
# Story: `tp -l` from ~ shows all seven seeded portals, then `cd code` and
# `tp -l -u` shrinks the list to the three nested under that directory
# (auth, web, docs). The shrink is the entire point, and it is why
# seed-demo.sh deliberately places dotfiles and notes OUTSIDE ~/code: without
# portals to filter out, `-u` would just reprint the same list and the
# recording would prove nothing. `tp -u` then opens the picker on those three
# and jumps, with the prompt confirming the landing.

Output "docs/images/under-cwd.gif"

Set Shell zsh
Set FontSize 14
Set Width 1200
Set Height 620
Set Theme "Tomorrow Night"
Set TypingSpeed 55ms

Source docs/demo/common-setup.tape
Type "cd ~ && clear"
Enter
Sleep 300ms
Show

Sleep 900ms
Type "tp -l"
Sleep 300ms
Enter
Sleep 2400ms

Type "cd code"
Sleep 300ms
Enter
Sleep 1000ms

Type "tp -l -u"
Sleep 300ms
Enter
Sleep 2400ms

Type "tp -u"
Sleep 300ms
Enter
Sleep 1800ms

# Verified seeded order, top to bottom: auth, docs, web.
# One Down lands on `docs`, which is a single-worktree repo. That matters:
# `auth` is the worktree-enabled repo, so selecting it would chain into a
# second picker and re-tell worktree.gif's story inside this recording.
Down
Sleep 900ms

Enter
Sleep 2200ms

Hide
Type "exit"
Enter
Sleep 300ms
