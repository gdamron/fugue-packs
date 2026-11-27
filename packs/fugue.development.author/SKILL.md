---
name: fugue-development-author
description: Package a voice, effect or part of a Fugue piece as a reusable development that other pieces import and drop in. Use when the user asks to make a reusable instrument or development, turn part of a piece into one, or change a development's ports, controls, defaults or version. Not for writing a whole piece, or for using a development that already exists.
---

# Development author

Turn a voice or a subgraph into a development: a document with named inputs,
outputs and controls that any piece can import and use as a module type. It
will play in pieces you never hear, so its interface and its defaults must be
right on their own.

## Triggers

- A request to make a reusable instrument, voice, effect or subgraph, or to
  package one as a development.
- A request to change a development's ports, controls, defaults or version.

## Required policies

- Before writing, importing or installing the development, or adding or
  editing a script module, read [authoring](policies/fugue.policy.authoring.md).
- Before choosing the levels and envelopes it ships with, read
  [clean audio](policies/fugue.policy.clean-audio.md).
- Before trying it in the in-chat player (the hosted tools), read
  [the player](policies/fugue.policy.player.md).
- Before opening a .fyug file, or when asked how to keep the piece, read
  [files](policies/fugue.policy.files.md).

## Inputs

- What to package: a description, or modules of a piece that is playing.
- Where it will be dropped in: the kind of piece and the part it plays.
- Your tools. With the local ones (`reload_invention` among them) it lives
  in a file beside its pieces, or a package. With only the hosted ones (no
  local `reload_invention`), in the piece's document.

## Outputs

- A development document whose `inputs`, `outputs` and `controls` are its
  interface, tried in a piece and heard.
- With the local tools, for a package, a `fugue.pkg.json`, installed and
  confirmed with `describe_development`. With only the hosted tools, the
  piece that holds it, open in the in-chat player.
- A short note for the user: each port and control, and what its default
  does.

## Procedure

1. Draw the boundary. Keep inside what gives the sound its character; leave
   clocks, sequencing, the mixer and the `dac` to the piece that drops it in.
2. Name the interface after the bundled developments, so one can stand in
   for another: `frequency`, `gate` and `pedal` in and `audio` out for a
   voice, `audio_left` and `audio_right` in and out for a stereo effect.
   List an input or control name more than once to fan it out to several
   inner ports or controls. Expose only what a player would reach for, named
   for what it does (`brightness`, not the filter's cutoff), and give each
   number control a `range`, within the inner control's own, that keeps
   every value musical.
3. Make the defaults clean anywhere. The config inside is what a piece gets
   when it sets nothing, so write it there. Route `gate` and `pedal`
   through `sustain`, as the bundled voices do, then into an `adsr` and a
   `vca`, so every note has an envelope as the clean-audio policy asks.
   End in a second `vca` whose `vca.level` sets a modest output level and
   is exposed as `level`.
4. Try it where it will be used. With the local tools, import it into a
   small piece by `path`, or inline as a `definition`, and load it with
   `load_invention`. With only the hosted tools, write it inline as a
   `definition` in the piece's own `developments`, check the piece with
   `check_invention`, then open it with `load_invention`. Play quick
   repeated notes and a long held one and read `get_meters`; with the local
   tools, also read the instance with `describe_module`. Fix the
   definition, not the trial piece.
5. Version and package, with the local tools. Write a `fugue.pkg.json` like
   a bundled instrument's: `id`, `version`, `kind` development, `license`,
   `authors`, `targets`, and an `entry` naming the document. Raise its
   `version` for every change: a `ref` names one exact version, so pieces
   that import an older one keep hearing it. Keep port and control names
   across versions. Install it with `install_package`, find its ref with
   `list_developments`, and confirm the interface with
   `describe_development`. With only the hosted tools there is nothing to
   install: say plainly that the instrument lives only in that piece, and
   that Download piece keeps it inside. Mention the desktop app only when
   the person asks to reuse it in other pieces, where it can become a
   package.

## Stop conditions

Stop, say why, and say what the user can do next.

- **The part cannot stand alone.** It needs a clock or another voice to
  sound right: offer to expose that as an input, or leave it in the piece.
- **A change would rename or remove a released port or control.** Pieces
  that move to the new version would break: ask first, and offer to keep
  the old name or to release it under a new package id.
- **It is not clean at its defaults.** Do not package what peaks or clicks
  in the trial; say what you heard and what you will change.
- **It cannot be installed here.** No `install_package` is offered, or the
  user declines: hand over the document to import by `path`. With
  only the hosted tools this is no stop: it stays in the piece.
