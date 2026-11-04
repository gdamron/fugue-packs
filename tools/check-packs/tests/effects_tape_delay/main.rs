//! Acceptance tests for the `fugue.effects.tape-delay` development pack,
//! written from its interface spec alone. Each test loads the pack's
//! development file by path into an invention, renders it with fugue's
//! `RenderEngine` at 48 kHz and measures what is heard: the dry path, echo
//! timing and level, repeats, the pitch of the echo while the time changes
//! or wobbles, and the output bound.
//!
//! The shared delay rig (`tests/effects_delay/rig.rs`) builds the
//! inventions; `tape` adds sine sources and pitch measurements. The pack is
//! located relative to this crate (`../../packs/<id>/`), so the suite
//! compiles without it and fails at runtime until it exists.
//!
//! Run with `cargo test --release --test effects_tape_delay`.

#[allow(dead_code)]
#[path = "../effects_delay/rig.rs"]
mod rig;
#[allow(dead_code)]
mod tape;

mod bounded;
mod echoes;
mod glide;
mod package;
mod wobble;
