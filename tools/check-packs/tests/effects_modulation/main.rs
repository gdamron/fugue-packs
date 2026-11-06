//! Acceptance tests for the `fugue.effects.chorus` and
//! `fugue.effects.flanger` development packs, written from their interface
//! spec alone. Each test loads the pack's development file by path into an
//! invention, renders it with fugue's `RenderEngine` at 48 kHz and measures
//! what is heard.
//!
//! The packs are located relative to this crate (`../../packs/<id>/`), so
//! the suite compiles without them and fails at runtime until they exist.
//!
//! What the measurements mean:
//!
//! - No zipper: a modulation click is a step between neighbouring samples
//!   that the dry input does not have, so the largest sample-to-sample step
//!   of the output is compared with the largest step of the dry input
//!   (bound 1.5x). The input is a 1 kHz sine, whose steps are all the same
//!   size, so the dry maximum is exact and a jump of even a couple of
//!   samples in a delay shows. The wet and dry levels in those tests keep
//!   the legitimate steps (the dry share plus the voices, each scaled by
//!   how fast its delay moves) inside the bound, so only a discontinuity
//!   exceeds it; the arithmetic is in `zipper.rs`.
//! - Level: broadband pink noise (equal energy per octave, like music)
//!   fed to both sides, measured as the RMS of the mono fold (L+R)/2.
//! - Bounded: full-scale sources through a 0.5 tap so the dac cannot hide
//!   an overshoot.
//!
//! Run with `cargo test --release --test effects_modulation`.

mod rig;

mod chorus;
mod flanger;
mod shared;
mod zipper;
