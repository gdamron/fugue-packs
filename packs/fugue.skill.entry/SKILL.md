---
name: fugue
description: Make, play or change music with Fugue (also called Fyug), on any surface. Use when the user asks to compose or play a piece, hear an example, change the music that is playing, or keep or share a piece, or types /fugue. Not for working on Fugue's own source code.
---

# Fugue

The way in to making music with Fugue: find where you are, say how the piece
will play, and play it that way.

## Triggers

- A request to make, play, hear or change music with Fugue or Fyug.
- `/fugue`, alone or followed by a request.

## Required policies

- Before anything else, read [surfaces](policies/fugue.policy.surfaces.md)
  to find your surface and its route.
- Before opening the in-chat player, read
  [the player](policies/fugue.policy.player.md).
- Before opening a .fyug file, or when asked how to keep or share a piece,
  read [files](policies/fugue.policy.files.md).
- Before writing or changing a piece, or adding or editing a script module,
  read [authoring](policies/fugue.policy.authoring.md).
- Before choosing sounds, levels or changes, read
  [clean audio](policies/fugue.policy.clean-audio.md) and
  [musical intent](policies/fugue.policy.musical-intent.md).

## Inputs

- The user's request and the piece playing or named.
- The tools and shell you can see: they decide the surface.

## Outputs

- One sentence naming the route, before playing.
- The piece playing on that route, or a stop saying what the user can do
  next.

## Procedure

1. Recognize the surface as the surfaces policy says.
2. Tell the user which route you will use.
3. Author or change the piece as the authoring policy says. When the
   `fugue.invention.author` skill is present, hand the authoring to it.
4. Play it through the route for the surface:
   - **Local:** the local tools, `load_invention` or `reload_invention`, then
     `show_view` where the host shows views.
   - **Shell on the user's machine:** the native install offer: ask first,
     and on a yes point the user to https://github.com/Ilusiv-media/fyug#install
     to install it themselves. When the local tools appear, use the local route.
   - **Shell in a cloud session:** an offline render where Fyug is already
     installed, `fyug render piece.json --out piece.wav --seconds 60` (or
     `--to-end` when the piece ends itself), then hand the user the WAV file
     and say it is a recording, not live. Otherwise the preview file below.
   - **Hosted:** the in-chat player, as the player policy says:
     `check_invention`, then `load_invention`, then the player's own tools.
     On a phone they may come a message or more later:
     until then, say levels are unread, and never call them missing.
     Asked how to keep it, point to Download piece or Download recording
     where the player shows them, else Share link where there is one. Offer
     the desktop app only for something the player lacks: name what it lacks.
   - **Plain chat:** the preview file, when the host can run this skill's
     script and show an HTML file. Save the piece, then run
     `python3 scripts/write_preview.py --player fyug-player.html --modules fyug-player-modules.json --invention piece.json --out piece-preview.html`
     from this skill's folder, where the player and module list come with it.
     Hand the user the file: say it is a limited preview that may drop out,
     tapping Play starts it, and link the desktop install,
     https://github.com/Ilusiv-media/fyug#install. Never retype, edit or
     regenerate the player yourself; if the script or either file is
     missing, say so and what to connect or install.

## Stop conditions

Stop, say why, and say what the user can do next. Never move to another
route without telling the user first.

- **The user declines the install.** Install nothing. They can connect the
  hosted connector the surfaces policy names, or install later.
- **The piece uses a module type the player or preview does not support.**
  `load_invention` refuses it by name, and so does the script, writing no
  file. Rework the piece without it. Offer to install Fyug
  (`get_status` gives the link) only when the request needs that type.
- **No route on this surface can play the piece.** Say what is missing and
  what to connect or install; do not pretend it played.
