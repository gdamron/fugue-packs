//! Acceptance tests for the `fugue.effects.compressor` development pack,
//! written from its interface spec alone. Each test loads the pack's
//! development file by path into an invention, renders it with fugue's
//! `RenderEngine` at 48 kHz and measures what is heard against the same
//! sources wired straight to the dac (the dry input).
//!
//! Steady levels use steady signals: a constant (a long clock gate through
//! a vca), a sustained square, or a sine as the compressed audio. A steady
//! signal's detector level is its peak magnitude, so once the detector has
//! settled (0.5 s, 50 attack time constants at the default 0.01 s) the gain
//! is fixed by the curve: 10^((c(x) + makeup)/20).
//!
//! The pack is located relative to this crate (`../../packs/<id>/`), so the
//! suite compiles without it and fails at runtime until it exists.
//!
//! Run with `cargo test --release --test effects_compressor`.

mod rig;

mod controls;
mod glide;
mod makeup;
mod package;
mod sidechain;
mod stereo;
mod transparency;
