---
name: fugue
description: Make, play or change music with Fugue (also called Fyug), on any surface. Use when the user asks to compose or play a piece, hear an example, or change the music that is playing, or types /fugue. Not for working on Fugue's own source code.
---

# Fugue

The way in to making music with Fugue. Work out where you are, say how the
piece will play, then make it and play it that way, so the user does not
have to pick a backend.

## Triggers

- A request to make, play, hear or change music with Fugue or Fyug.
- `/fugue`, alone or followed by a request.

## Required policies

- [Surfaces](policies/fugue.policy.surfaces.md): which surface you are on and
  which route plays the piece there.
- [Authoring](policies/fugue.policy.authoring.md): writing, loading and
  changing a piece.
- [Clean audio](policies/fugue.policy.clean-audio.md) and
  [musical intent](policies/fugue.policy.musical-intent.md): how it sounds.

## Inputs

- The user's request, in their words.
- The tools and shell you can see: they decide the surface.
- The piece that is playing or named, if there is one.

## Outputs

- Before playing, one sentence naming the route.
- The piece, playing on that route, or a stop that tells the user what they
  can do next.

## Procedure

1. Recognize the surface as the surfaces policy says.
2. Tell the user which route you will use.
3. Author or change the piece as the authoring policy says. When the
   `fugue.invention.author` skill is present, hand the authoring to it.
4. Play it through the route for the surface:
   - **Local:** the local tools, `load_invention` or `reload_invention`, then
     `show_view` where the host shows views.
   - **Shell on the user's machine:** the native install offer. The guided
     install is not published yet: until it is, ask first, and on a yes
     point the user to https://github.com/Ilusiv-media/fyug#install to
     install it themselves. When the local tools appear, use the local route.
   - **Shell in a cloud session:** an offline render where Fyug is already
     installed, `fyug render piece.json --out piece.wav --seconds 60` (or
     `--to-end` when the piece ends itself), then hand the user the WAV file
     and say it is a recording, not live. Otherwise the preview file below.
   - **Hosted:** `check_invention`, fix what it reports, then `play_preview`.
     The user taps Play.
   - **Plain chat:** the preview file, when the host can run this skill's
     script and show an HTML file. Save the piece, then run
     `python3 scripts/write_preview.py --player fyug-player.html --modules fyug-player-modules.json --invention piece.json --out piece-preview.html`
     with the release's player and module list that come with this skill.
     Hand the user the file: say it is a limited preview that may drop out,
     tapping Play starts it, and link the desktop install,
     https://github.com/Ilusiv-media/fyug#install. Never retype, edit or
     regenerate the player yourself; if the script or either file is
     missing, say so and what to connect or install.

## Stop conditions

Stop, say why, and say what the user can do next. Never move to another
route without telling the user first.

- **The user declines the install.** Install nothing. They can connect the
  hosted connector the surfaces policy names to hear a preview, or install
  later.
- **The piece uses a module type the preview does not support.**
  `play_preview` refuses it by name, and so does the script, writing no
  file. Offer to rework the piece without it, or to play it in full after
  installing Fyug (`get_status` gives the link).
- **No route on this surface can play the piece.** Say what is missing and
  what to connect or install; do not pretend it played.
