//! Acceptance tests for the `fugue.effects.granular` development pack,
//! written from its interface spec alone: core's stereo `granular` module
//! with dry and wet levels and a send to core's `reverb`.
//!
//! Each test loads the pack's development file by path into an invention,
//! renders it with fugue's `RenderEngine` at 48 kHz and measures what is
//! heard: the dry end and the wet end (the input, and the bare module,
//! through the pack's soft limit), silence when muted, the reverb tail, every mode playing, the
//! output bound, and the step size of live control changes. The pack is
//! located relative to this crate (`../../packs/<id>/`), so the suite
//! compiles without it and fails at runtime until it exists.
//!
//! Run with `cargo test --release --test effects_granular_acceptance`.

mod rig;

mod bounded;
mod ends;
mod glide;
mod modes;
mod package;
mod reverb;
