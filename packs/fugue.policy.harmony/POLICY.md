# Harmony

Start from the key and scale of fugue.policy.theory.

## Make chords

`macro_oscillator.model=chords` plays a four-note chord on each pitch it
receives, built from `macro_oscillator.key` and `macro_oscillator.scale`, so
the scale decides the quality: a seventh in D dorian is Dm7 on D, G7 on G.
`macro_oscillator.harmonics` picks the shape: octave, fifth, sus2, sus4 (0.34),
triad (0.44), sixth, seventh (0.64), ninth, eleventh, six-nine.
`macro_oscillator.timbre` is the inversion, 0.5 for root position. The notes
you send are the roots, so the line is the progression:

```json
{"modules":[
{"id":"clock","type":"clock"},
{"id":"roots","type":"step_sequencer","config":{"root_note":50,"step_count":4,
"pattern":[{"note":0},{"note":5},{"note":10},{"note":7}]}},
{"id":"chords","type":"macro_oscillator","config":{"model":"chords","key":"D","scale":"dorian","harmonics":0.64}}],
"connections":[
{"from":"clock","from_port":"beat","to":"roots","to_port":"clock"},
{"from":"roots","from_port":"frequency","to":"chords","to_port":"frequency"},
{"from":"roots","from_port":"gate","to":"chords","to_port":"trigger"}]}
```

## Carry a progression

To carry a progression under a figure, have `control_scheduler.schedule`
write `step_sequencer.root_note` every four beats. The figure keeps its
intervals, so use one that fits every chord, like root and fifth:

```json
{"modules":[
{"id":"clock","type":"clock"},
{"id":"arp","type":"step_sequencer","config":{"root_note":60,"step_count":2,
"pattern":[{"note":0},{"note":7}]}},
{"id":"plan","type":"control_scheduler","config":{"schedule":[
{"at_step":4,"module":"arp","control":"root_note","value":57},
{"at_step":8,"module":"arp","control":"root_note","value":53}]}}],
"connections":[
{"from":"clock","from_port":"beat","to":"arp","to_port":"clock"},
{"from":"clock","from_port":"beat","to":"plan","to_port":"clock"}]}
```

For a suspension write sus4, then triad, to `macro_oscillator.harmonics`: the
chord changes once, not through the shapes between.

## Re-voice while it plays

`macro_oscillator.timbre` moves all four notes together. To move one voice,
give each chord tone its own `step_sequencer` and write that voice's
`step_sequencer.pattern`: the next step reads it. Keep common tones and move
the rest least: C to Am keeps C and E, and G rises to A (0 4 7 becomes 0 4 9).
One voice per change is fugue.policy.musical-intent's rule; where it lands,
fugue.policy.clean-audio's. A schedule plays once: write
`control_scheduler.schedule` to continue it. Your write to a scheduled
control lasts only until the scheduler's next entry. `divisi` makes no chords:
it fans one line over `divisi.voice_count` voices.

## What does not exist

No chord-symbol input: chords come from a root, scale and shape, not a name
like Dm7. No voice-leading helper: nothing picks inversions or smooth motion.
No shared chord track: each module holds its own. A step holds one note, so
separate voices need a sequencer each.
