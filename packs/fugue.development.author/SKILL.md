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

- [Authoring](policies/fugue.policy.authoring.md): writing, loading,
  importing and installing.
- [Clean audio](policies/fugue.policy.clean-audio.md): the levels and
  envelopes a development ships with.

## Inputs

- What to package: a description, or modules of a piece that is playing.
- Where it will be dropped in: the kind of piece and the part it plays.
- Where it will live: a file beside the pieces that use it, or a package.

## Outputs

- A development document whose `inputs`, `outputs` and `controls` are its
  interface, tried in a piece and heard.
- For a package, a `fugue.pkg.json` with its `kind`, `version` and `entry`,
  installed and checked.
- A short note for the user: each port and control, and what its default
  does.

## Procedure

1. Draw the boundary. Keep inside what gives the sound its character; leave
   clocks, sequencing, the mixer and the `dac` to the piece that drops it in.
2. Name the interface after the bundled developments, so one can stand in
   for another: `frequency`, `gate` and `pedal` in and `audio` out for a
   voice, `audio_left` and `audio_right` in and out for a stereo effect.
   List one name more than once to fan it out to several inner ports or
   controls. Expose only what a player would reach for, named for what it
   does (`brightness`, not the filter's cutoff), each control with a `range`
   that keeps every value musical.
3. Make the defaults clean anywhere. The config inside is what a piece gets
   when it sets nothing, so write it there. Route `gate` through `sustain`
   into an `adsr` and a `vca`, so any gate pattern a piece sends is shaped
   as the clean-audio policy asks, and end in a second `vca` whose
   `vca.level` sets a modest output level and is exposed as `level`.
4. Try it where it will be used. Import it into a small piece by `path`, or
   inline as a `definition`, and load it with `load_invention`. Play quick
   repeated notes and a long held one, read `get_meters`, and read the
   instance with `describe_module`. Fix the definition, not the trial piece.
5. Version and package. Write a `fugue.pkg.json` whose `kind` is
   development and whose `entry` names the document. Raise its `version` for
   every change: a `ref` names one exact version, so pieces that import an
   older one keep hearing it. Keep port and control names across versions.
   Install it with `install_package`, find its ref with `list_developments`,
   and confirm the interface with `describe_development`.

## Stop conditions

Stop, say why, and say what the user can do next.

- **The part cannot stand alone.** It needs a clock or another voice to
  sound right: offer to expose that as an input, or leave it in the piece.
- **A change would rename or remove a released port or control.** Pieces
  that move to the new version would break: ask first, and offer to keep
  the old name or to release it under a new package id.
- **It is not clean at its defaults.** Do not package what peaks or clicks
  in the trial; say what you heard and what you will change.
- **It cannot be installed here.** No package tools, or the user declines:
  hand over the document to import by `path`.
