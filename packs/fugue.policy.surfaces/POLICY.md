# Surfaces

Surfaces play a piece differently. Work out which one you are on first.
How the in-chat player is used is in `fugue.policy.player`; how a piece is
kept or opened, in `fugue.policy.files`.

## Recognize your surface

Go by what you see; stop at the first match. A host may prefix tool
names. Each side lists every tool, including ones it lacks: a tool whose
description begins "Not available here:" only says what to use instead.
Do not call it, and do not count it below.

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
- **Hosted:** play in the in-chat player, as `fugue.policy.player`
  says.
- **Plain chat:** use the preview artifact where the host can run the entry
  skill's script and show an HTML file. It plays with no controls and no
  tools. Otherwise say what to connect (the hosted connector,
  https://mcp.fyug.fm/mcp) or install.

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

## Say where you are

Tell the user which route you are using. The in-chat player and the preview
artifact are limited: they run in the browser with a reduced feature set and
may drop out. Say so, and do not claim features they lack or deny ones they
have. If there is no sound on a phone, the silent switch may be on. Never
change route without saying so: if the player fails or drops out, say that,
and offer the desktop app. On the preview artifact, offer the hosted
connector.
