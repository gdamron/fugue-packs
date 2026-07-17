# fugue.demo.808-kit

A synthesized 808-style drum kit — the reference `sample-pack` for the
`samples.json` entry schema (see `src/pkg/SAMPLE_PACK.md` in the `fugue`
repository).

## Contents

Six one-shots (`kick`, `snare`, `clap`, `hat-closed`, `hat-open`,
`cowbell`) and a one-bar 120 BPM loop assembled from them. The loop
declares eighth-note slice points (`beat-1` … `and-4`) for slicing
modules.

All audio is 48 kHz, 16-bit mono WAV, peak-normalized to −1 dBFS with
faded tails.

## Provenance and licensing

Every sample is synthesized from scratch using classic analog drum-machine
recipes (sine-sweep kick, tone-plus-noise snare, square-bank metals,
noise-burst clap). No third-party recordings are included. Released under
CC0-1.0, per the manifest.
