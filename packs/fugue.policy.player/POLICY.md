# The In-Chat Player

How a piece is opened, read and changed in the hosted server's in-chat
player, on the web and on a phone. Which surface you are on is in
`fugue.policy.surfaces`; keeping a piece is in `fugue.policy.files`.

## The in-chat player

Check the document with check_invention and fix what it reports, then open
it with load_invention. Prefer bundled instruments (list_developments), and
pass as controls the two to six values most worth playing with. The person
taps Play.

While it is open, the player offers tools for the piece it holds.
Read its levels (get_meters) and notes (get_notes) before calling it ready,
and name what you find: silence, clipping, a part out of range. Change the
loaded piece through them, set_controls for values and apply_edits for its
structure, rather than sending it again. A value the person moved
(list_changed_controls) stays as they left it.

On the web the player sits beside the chat, keeps playing out of view and
offers downloads. On a phone sound stops when the player is off screen, and
each call may wait for the person's OK. On a phone the player's tools may
not be offered once load_invention returns: they come with the person's
next message or a later one; plan no player tool call in this turn and do
not resend the piece. Say it is loaded, ask them to tap Play. Until they
are listed, say its levels are not read yet, look again each turn,
and never say the app or player lacks them. Where the host offers no player
tools at all, send a changed document to load_invention.

When a player tool call comes back not run for lack of approval, say what
you were about to do and point to the same control in the player. Tell the
person once that they can set the connector's tools to always allow in the
app. Do not retry unless they ask.

## Edit its structure

The player's apply_edits takes add_module, remove_module, swap_module,
connect, disconnect and set_control, with no `when` or `ttl_ms`: an edit
lands as soon as it is sent. A swap keeps the module's id, name and the
wires the new type has ports for, takes the config you give (none when
left out), and `swapped` lists the wires it removed. Its set_control sets
a value the document keeps, such as a new module's starting level; play
values through set_controls. Both hold a value as the local tools do: a
number outside its control's range is set to the nearer limit and reported
clamped, and one refused write changes nothing.

## Read the notes

Call get_notes with `bars`, how many bars to list (1 to 64; 4 is a good
first look), and start_bar to begin later than bar 0. A part with random
modules lists one possible playing (chance). Script and agent modules' notes
are not listed; the answer names those modules under not_run, though a script
the player runs can still change what plays. When the answer is cut short,
pass its cursor with the same arguments for the rest.
