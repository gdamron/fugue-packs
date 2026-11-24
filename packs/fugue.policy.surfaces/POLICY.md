# Surfaces

Surfaces play a piece differently. Work out which one you are on first.

## Recognize your surface

Go by what you see; stop at the first match. A host may prefix tool
names.

1. The local Fyug tools are present (reload_invention, show_view): local.
   When both sets of tools are present, this one wins.
2. The hosted tools are present (check_invention) without the local ones:
   hosted. Its load_invention opens an in-chat player.
3. A shell is present, but no Fyug tools: shell.
4. None of these: plain chat.

A shell is on the user's machine or in a cloud agent session, such as Claude
Code on the web or Codex cloud, which has no speakers. If you cannot tell,
ask.

## Take the route for your surface

- **Local:** play and edit through the local tools, and use views
  (show_view) where the host shows them. Never use the in-chat player or the
  preview artifact.
- **Shell on the user's machine:** offer the native install and wait for the
  answer. The install procedure is not part of this policy.
- **Shell in a cloud session:** do not install for playback. Render offline
  with fyug render where fyug is already present, or use the preview artifact
  where the host shows one. If neither applies, say what is missing.
- **Hosted:** play in the in-chat player, below.
- **Plain chat:** use the preview artifact where the host can run the entry
  skill's script and show an HTML file. It plays with no controls and no
  tools. Otherwise say what to connect (the hosted connector,
  https://mcp.fyug.fm/mcp) or install.

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
each call may wait for the person's OK. If the player's tools are not
offered once load_invention returns, as on a phone, they come with the
person's next message or a later one: plan no player tool call in this turn
and do not send the piece again. Say it is loaded and ask them to tap Play.
Until they come, say it is not checked yet, look again each turn, and never
say the app or player lacks them. Where the host offers no player
tools at all, send a changed document to load_invention.

When a player tool call comes back not run for lack of approval, say what
you were about to do and point to the same control in the player. Tell the
person once that they can set the connector's tools to always allow in the
app. Do not retry unless they ask.

## What the hosted server leaves to the desktop app

Suggest the desktop app only when the person asks for one of these, and name
it, or when the player fails:

- music that keeps playing in the background, where a phone's player stops
  off screen
- a library of installed packages
- their own samples, file, RTMP or YouTube outputs, or hardware output
- a module type the player does not build, or a piece past the hosted limits;
  load_invention refuses either by name
- an agent working on a piece with no player open

Otherwise do not raise it on this route. get_status gives the install URL.

## Keep a piece

Asked how to keep a piece or send it to a friend, point to Download piece
where the player shows it (a file on disk), and to Share link otherwise. Say
that anyone with the link has the piece. Never post a share link yourself:
the person decides who gets it. A piece with bundled samples has no Share
link, so on a phone it cannot be kept: say so, and point to the web or the
desktop app.

## Say where you are

Tell the user which route you are using. The in-chat player and the preview
artifact are limited: they run in the browser with a reduced feature set and
may drop out. Say so, and do not claim features they lack or deny ones they
have. If there is no sound on a phone, the silent switch may be on. Never
change route without saying so: if the player fails or drops out, say that,
and offer the desktop app. On the preview artifact, offer the hosted
connector.
