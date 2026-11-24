---
name: fugue-invention-author
description: Build a new Fugue invention from what the user asks for. Use when the user says "build me an invention that...", "make a piece in D dorian", "write a slow ambient drone" or otherwise wants a new piece made from scratch. Not for packaging a reusable instrument, reviewing how a piece already sounds, or working on Fugue's own source code.
---

# Invention Author

Turn a request into one small invention that plays and sounds the way the
user meant: plan it, write it whole, load it, listen, then refine it.

## Triggers

- A request to build, make or write a new piece, groove, drone or song with
  Fugue.
- A request to start over from a new idea when a piece is already playing.

## Required policies

- [Authoring](policies/fugue.policy.authoring.md): the document, loading and
  editing.
- [Clean audio](policies/fugue.policy.clean-audio.md): levels, envelopes and
  changes that do not click.
- [Musical intent](policies/fugue.policy.musical-intent.md): what to change
  and when.
- [Listening](policies/fugue.policy.listening.md): what counts as heard.
- Read these only when the task calls for them:
  [theory](policies/fugue.policy.theory.md) for a key, scale or mode,
  [harmony](policies/fugue.policy.harmony.md) for chords or a progression,
  and [rhythm](policies/fugue.policy.rhythm.md) for a meter, triplets, swing
  or a groove.

## Inputs

- The user's request, in their words: mood, style, tempo, key, length, any
  instrument they named.
- The installed developments and examples, when one fits a voice.

## Outputs

- One complete invention document, loaded and playing.
- A short report of what you heard, and of anything you could not make.
- The saved document, when the user asks for it.

## Procedure

1. **Plan the piece.** Name each voice and its role, the tempo, the meter
   and the form, before writing any module.
2. **Author one complete document.** The example
   `examples/dorian-evening.invention.json` shows the shape. Patterns that
   hold in every piece:
   - A voice is a chain: a pitch source (`step_sequencer` or `melody`), a
     sound source (`oscillator` or `macro_oscillator`), a `vca` whose
     `vca.level` an `adsr` drives, then one channel of a `mixer`.
   - One `clock` drives every voice so they stay together. A
     `step_sequencer` steps on `step_sequencer.clock`; a `melody` steps on
     `melody.gate`, not a clock port.
   - Stereo effects shared by all voices, such as `reverb` and `limiter`,
     go after the mixer, fed from `mixer.audio_left` and `mixer.audio_right`.
   - Nothing sounds unless it reaches the `dac`. Send a stereo pair to
     `dac.audio_left` and `dac.audio_right`, and a mono line to `dac.audio`;
     a line on one side alone plays in one ear.
   - `mixer.channel_count` (4 unless set) is how many `mixer.audio.0`,
     `mixer.audio.1` ... inputs exist. Give `mixer.levels` one entry per
     channel you patch: a channel without one plays at full level.
   - A patched `vca.level` replaces the level in its config, so set each
     voice's loudness on its mixer channel.
   - Give every module its own id. A repeated id loads without complaint
     and keeps only the last module.
   - Ports differ by type: `noise` has `noise.white` and `noise.pink`, not
     an audio port. Check names with `describe_module` before you guess.
3. **Load it** with `load_invention`. A refused load names the unknown
   type, port, config key or module id, and lists the ports or keys the
   module does take: fix that in the document and load again. In the
   in-chat player, also pass the few controls most worth playing with.
   In the in-chat player, the person keeps the piece with Download piece
   where the player shows it, and Share link otherwise (none for a piece
   with bundled samples); anyone with the link has it. Offer the desktop
   app only for something the player lacks, and name what it lacks.
4. **Listen** with `get_meters` while it plays, as the listening policy
   says. In the in-chat player, read `get_meters` and `get_notes` once its
   tools are offered (on a phone, with the person's next message or a later
   one). Until they come, say the piece is not checked yet, look again each
   turn, and never say the app or player lacks them.
5. **Refine** as the musical-intent policy paces it: a structural change
   through `reload_invention` (in the in-chat player, `apply_edits`), a
   level or a tempo through `set_controls`. Listen again after each, and
   stop when it matches the plan.

## Stop conditions

Stop, say why, and say what the user can do next.

- **No Fugue tools here.** Nothing can load or play the piece on this
  surface: hand over to the `fugue` skill if it is present, or say what to
  install.
- **The request needs something Fugue lacks**: no module type does it, or
  a policy says it does not exist, such as a tuning or a swung melody line. Say so and
  offer the closest piece it can play.
- **The same load is refused twice for one reason.** Show the user the
  refusal instead of guessing again.
