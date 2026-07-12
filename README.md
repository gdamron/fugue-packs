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

`index.json` intentionally contains an empty `packages` array. FUG-100 owns
publishing package versions, git references, and integrity hashes after the
installer can address individual packages in this monorepo.

See [CONTRIBUTING.md](CONTRIBUTING.md) before proposing a pack and
[LICENSING.md](LICENSING.md) for the repository's licensing model.
