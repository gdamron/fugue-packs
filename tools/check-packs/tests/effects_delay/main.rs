//! Acceptance tests for the `fugue.effects.stereo-delay` and
//! `fugue.effects.ping-pong-delay` development packs, written from their
//! interface spec alone. Each test loads the pack's development file by path
//! into an invention, renders it with fugue's `RenderEngine` at 48 kHz and
//! measures what is heard: echo onsets and sides, levels, the dry path and
//! the output bound.
//!
//! The packs are located relative to this crate (`../../packs/<id>/`), so
//! the suite compiles without them and fails at runtime until they exist.
//!
//! Onsets are the first sample whose magnitude exceeds 1e-4 (the tone
//! lowpass smears an edge, so the peak would drift). Test signals are a
//! 480-sample pulse at 0.5 (a slow clock gate through a vca) and seeded
//! full-scale white noise; outputs reach a dac with soft clip off.
//!
//! Run with `cargo test --release --test effects_delay`.

mod rig;

mod packs;
mod ping_pong;
mod shared;
mod stereo;
