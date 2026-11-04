//! Acceptance tests for the `fugue.effects.eq` development pack, written
//! from its spec alone: seven bands in series per side (low cut, low shelf,
//! three bells, high shelf, high cut), each with a bypass, linked stereo,
//! transparent at its defaults.
//!
//! Each test loads the pack's development file by path into an invention,
//! renders it with fugue's `RenderEngine` at 48 kHz and measures what is
//! heard: bit-identity with the dry input (defaults, bypasses), the bypass
//! crossfade, the two sides' link, the bands' responses (steady-tone RMS
//! against the dry tone, compared with the RBJ cookbook) and the output bound
//! under fast live moves. The pack is located relative to this crate
//! (`../../packs/<id>/`), so the suite compiles without it and fails at
//! runtime until it exists.
//!
//! Run with `cargo test --release --test effects_eq`.

mod rig;

mod bounded;
mod bypass;
mod package;
mod response;
mod stereo;
