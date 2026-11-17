# Rhythm

Decide the meter, the grid and the feel before writing notes. Every pulse
comes from a `clock`.

## Count the bar

Set `clock.beats_per_bar=3` for waltz time, or 7 for 7/8 with the beat an
eighth; the default 0 keeps no bars. The clock then reports `clock.bar` and
`clock.beat_in_bar`, and a change scheduled for the next bar (see
fugue.policy.clean-audio) lands on your bars. Size a looping sequencer's
`cell_sequencer.step_count` or `step_sequencer.step_count` to a whole number
of bars on its grid, or it drifts against the bar line.

## Choose the grid

Patch a sequencer's `step_sequencer.clock` from the output with one edge per
step: with a quarter-note beat, `clock.beat_x2` for eighths and
`clock.beat_x4` for sixteenths. Keep
`clock.bpm` at the notated tempo instead of running it fast, and leave rests
where a fine grid has nothing to say. Echoes sit on the grid too: patch
`clock.bpm` into `delay.bpm` and set `delay.division=dotted_8th_note`.

## Triplets and polyrhythm

The clock has no triplet output. For 3 against 2, run a second clock at 1.5
times the tempo, each driving its own sequencer from `clock.beat`; move both
`clock.bpm` together:

```json
{"modules":[
{"id":"two","type":"clock","config":{"bpm":120}},
{"id":"three","type":"clock","config":{"bpm":180}},
{"id":"a","type":"step_sequencer","config":{"step_count":2}},
{"id":"b","type":"step_sequencer","config":{"step_count":3}}],
"connections":[
{"from":"two","from_port":"beat","to":"a","to_port":"clock"},
{"from":"three","from_port":"beat","to":"b","to_port":"clock"}]}
```

For polymeter keep one clock and give each sequencer its own step count: 5
against 4 realign every 20 steps. Within one module, `chance.rate=x3` puts
three steps in each pulse, and `lfo.division=8th_triplet` times a sweep in
triplets.

## Swing and groove

Only `drum_map` swings. `drum_map.swing` delays the off-beat sixteenths: 0 is
straight, 0.3 gentle, 2/3 a triplet feel. The sequencers play a straight
grid, so do not promise a swung melody. Its bar is always 16 sixteenths, so
use it in 4/4 only.

```json
{"modules":[
{"id":"clock","type":"clock","config":{"bpm":96,"beats_per_bar":4}},
{"id":"drums","type":"drum_map","config":{"swing":0.3}}],
"connections":[
{"from":"clock","from_port":"beat_x4","to":"drums","to_port":"clock"}]}
```

Move `drum_map.x` from straight to syncopated to broken and `drum_map.y`
from sparse to driving; `drum_map.chaos` varies bars, and a fixed
`drum_map.seed` repeats them. `chance.jitter` loosens gates by up to half a
step, so keep it small. Change feel as fugue.policy.musical-intent directs.
