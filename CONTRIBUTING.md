# Contributing packs

Fugue packs are music-making materials. Contributions should be useful to a
composer, clearly described, and small enough to understand and audition.

## Package layout

Create one directory at `packs/<package-id>/`. The directory name must exactly
match the `id` in its `fugue.pkg.json` manifest. Every package must include:

- a manifest accepted by Fugue's package validator;
- the file named by the manifest's kind-specific `entry` field;
- an SPDX license identifier and at least one author in the manifest;
- only assets that the contributor has the right to publish.

Package IDs use lowercase reverse-DNS form such as `fugue.composing.harmony`.
Package versions use semantic versioning. Entry paths must be relative, remain
inside the package directory, and name a file. Skill entries must be named
`SKILL.md`.

Agent-definition and sample-index payloads are provisional. Until their
kind-specific schemas land, keep those payloads minimal and mark assumptions in
the package README or description.

## Before opening a pull request

Run the same checks as CI:

```sh
cargo fmt --manifest-path tools/check-packs/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/check-packs/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/check-packs/Cargo.toml
cargo run --locked --manifest-path tools/check-packs/Cargo.toml -- .
```

Do not edit `index.json` merely to publish a package. Registry publication and
integrity metadata follow a separate release workflow.
