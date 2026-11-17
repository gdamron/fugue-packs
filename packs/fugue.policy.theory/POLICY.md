# Theory

Choose the key and scale first. Pitch is a MIDI note: 60 is middle C, 62 is D.

## Choose the mode by its color

Each mode has one note that sets it apart: dorian is minor with a raised
sixth, phrygian minor with a lowered second, lydian major with a raised
fourth, mixolydian major with a lowered seventh. Pick the mode for the mood
and put that note on a strong beat. `chance.scale` takes major (Ionian),
dorian, phrygian, lydian, mixolydian, minor (Aeolian), locrian,
harmonic_minor, melodic_minor (rising form), major_pentatonic,
minor_pentatonic, blues, whole_tone and chromatic.

## Spell the scale in a sequencer

A sequencer has no scale. Each step's note is a semitone offset from
`step_sequencer.root_note` (also `cell_sequencer.root_note`), so you write
the scale's offsets yourself. D dorian is 62 with 0 2 3 5 7 9 10:

```json
{"modules":[
{"id":"seq","type":"step_sequencer","config":{"root_note":62,"step_count":8,
"pattern":[{"note":0},{"note":2},{"note":3},{"note":5},{"note":7},{"note":5},{"note":3},{"note":2}]}}],
"connections":[]}
```

## Let chance or melody stay in key

`chance.key` and `chance.scale` quantize its pitches, but only at
`chance.steps` 0.5 or more; at 1 they narrow to tonic, fifth and third.
`chance.low_note` and `chance.range` set the register. `melody` takes any
scale as semitone offsets in `melody.degrees` from `melody.root_note`; raise
`melody.note_weights` on the tonic and fifth:

```json
{"modules":[
{"id":"c","type":"chance","config":{"key":"D","scale":"dorian","steps":0.7,"low_note":50,"range":19}},
{"id":"m","type":"melody","config":{"root_note":62,"degrees":[0,2,3,5,7,9,10],"note_weights":[3,1,1,1,2,1,1]}}],
"connections":[]}
```

## Modulate while it plays

There is no shared key: each module holds its own. Write them in one
`set_controls` call, each with the same `when`.

To transpose, write `step_sequencer.root_note`, `cell_sequencer.root_note`,
`melody.root_note`, `chance.key` and, for chords, `macro_oscillator.key`; up
a fifth is +7. To change mode, write `chance.scale`, `macro_oscillator.scale`,
`melody.degrees`, or the notes of `step_sequencer.pattern` or
`cell_sequencer.cells`. The relative minor keeps every note and moves the
tonic: from C major, `chance.key=A` and `chance.scale=minor`; sequencer notes
and `melody.degrees` need respelling from the new root.

Keys a fifth apart share six of seven notes, so they sound closest. Writing a
sequencer's root note retunes the sounding note at once; `chance` and
`melody` change on their next step. Which moment to land on is
fugue.policy.clean-audio's, and how much to change at once is
fugue.policy.musical-intent's.

## What does not exist

Nothing quantizes an incoming pitch or CV to a scale: only `chance` snaps, and
only its own draws. There is no tuning control: pitch is twelve-tone equal at
A 440, and sequencers and `melody` take whole semitones. For a microtonal
pitch, set `oscillator.frequency` in Hz directly.
