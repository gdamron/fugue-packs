# Fugue Packs

First-party content for [Fugue](https://github.com/gdamron/fugue), a platform
for creating interactive and generative music.

Each directory under `packs/` is an independently versioned package with a
`fugue.pkg.json` manifest and the assets named by its `entry` field:

```text
packs/
  fugue.demo.pulse-skill/
    fugue.pkg.json
    SKILL.md
```

The initial reference collection demonstrates the five content package kinds:

| Package | Manifest kind | Entry |
| --- | --- | --- |
| `fugue.demo.pulse-skill` | `skill` | `SKILL.md` |
| `fugue.demo.pulse-agent` | `agent` | `agent.json` |
| `fugue.demo.pulse-samples` | `sample-pack` | `samples.json` |
| `fugue.demo.pulse-development` | `development` | `pulse.development.json` |
| `fugue.demo.pulse-study` | `invention` | `pulse.invention.json` |

The `agent.json` and `samples.json` payloads are illustrative until their
kind-specific schemas are finalized. Their package manifests use Fugue's
current authoritative schema.

## Check the repository

Install a stable Rust toolchain, then run:

```sh
cargo run --locked --manifest-path tools/check-packs/Cargo.toml -- .
```

The checker validates every manifest with the Fugue library, verifies package
layout and entry paths, and checks the top-level registry stub. CI also runs the
checker's unit tests, formatting, and Clippy.

## Registry stub

`index.json` intentionally contains an empty `packages` array. Package
versions, git references, and integrity hashes will be published there once
the installer can address individual packages in this monorepo.

See [CONTRIBUTING.md](CONTRIBUTING.md) before proposing a pack and
[LICENSING.md](LICENSING.md) for the repository's licensing model.

## Policies

A `policy` pack holds rules that skills and agent definitions require by id
instead of restating them: one Markdown file, opened with a `#` title and no
frontmatter. `fugue.policy.authoring` is the full contract for creating,
editing and conducting an invention through the `fugue` tools; the MCP
server's instructions summarize it. `fugue.policy.clean-audio`,
`fugue.policy.musical-intent` and `fugue.policy.listening` are the core
musical policies: protecting the sound, making musical choices, and judging by
ear. The pulse demos require the first two. `fugue.policy.rhythm` is the
rhythm reference: meter, grid, triplets and polyrhythm, swing and groove, tied
to the clock, sequencer and drum map modules that realize them.
`fugue.policy.surfaces` is for external agents only: how to tell which surface
they are on, which route plays a piece there, how the hosted server's in-chat
player is used on the web and on a phone, how a piece is kept, and what the
hosted server leaves to the desktop app.
`fugue.policy.theory` is the theory reference: choosing a mode, spelling a scale
for the sequencers, keeping `chance` and `melody` in key, and changing key while
a piece plays.
`fugue.policy.harmony` is the harmony reference: building chords with the macro
oscillator or one sequencer per voice, carrying a progression, re-voicing a
playing piece one voice at a time, and what has no module.

## Skills

A `skill` pack holds one `SKILL.md`: a thin procedure an agent host loads as
a skill, which requires the policies it follows instead of restating them.
`fugue.skill.entry` is the `fugue` skill, the `/fugue` entry point for
external agents: it works out which surface the agent is on, says which route
will play the piece, and has it authored and played through that route.
Its `scripts/write_preview.py` (Python 3 standard library only) puts a piece
into a release's single-file browser player for surfaces with no engine and
no hosted connector; the player and its supported-modules list come from the
release, not from this repository.

`fugue.development.author` packages a reusable voice or subgraph as a
development, which requires the authoring and clean-audio policies: exposed
ports and controls, names, version, and defaults that stay clean wherever it
is dropped in, with an example development to start from.

`fugue.invention.author` builds a new invention from a request, for external
and in-graph agents: it plans the piece, writes one complete document, loads
it, listens and refines it, and ships a small example invention.

`fugue.review.listener` is a listening critique for external agents: it
hears the loaded piece through the local audio preview and meters (or, in the
hosted in-chat player, measures its levels and notes), says what it found
against the piece's intent, and proposes one change.

## Release musical content

`fugue.instruments.piano`, `fugue.instruments.marimba`,
`fugue.instruments.vibraphone`, `fugue.instruments.pluck`, and
`fugue.instruments.pad` are the reusable instruments bundled with the daemon.
`fugue.effects.stereo-delay`, `fugue.effects.ping-pong-delay` and
`fugue.effects.tape-delay` are the bundled delay effects, built from core's
`delay` module (the tape delay adds `drive` saturation and wow and flutter).
`fugue.effects.compressor` is the bundled compressor, stereo-linked with a
sidechain input, built from core's `follower`, `gain_curve` and `gain` atoms.
`fugue.effects.eq` is the bundled parametric EQ: low and high cuts, two
shelves and three bells per side, each one of core's `filter` modes.
`fugue.effects.granular` is the bundled granular texture processor, core's
`granular` module with dry/wet levels and a send to core's `reverb`; its
looping delay can glide to a new length tape-style.
`fugue.effects.chorus` and `fugue.effects.flanger` are the bundled
modulation effects, swept core `delay`s driven by tempo-syncable `lfo`s (the
flanger feeds back through a `gain` that can invert).
`fugue.starter.bwv772` is the bundled playable invention: its score remains a
separate attributed asset and its arrangement depends on the exact piano
package version. Their canonical definitions and license notices live here.

The CLI release generates a hash-pinned snapshot from a pinned tag of this
repository at build time and embeds it, so no copy of these definitions lives
in the host repository. Change package versions when changing content;
installed versions are immutable. Bundling does not require publishing the
registry index.
