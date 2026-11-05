//! Render tests for the `fugue.effects.granular` development pack: the
//! manifest, the dry and wet ends of its mix, the soft limit on its output,
//! the reverb send's tail, every mode playing and glides on the mix
//! controls. Each test loads the pack's development file by path into an
//! invention and renders it with fugue's `RenderEngine` at 48 kHz.
//!
//! Run with `cargo test --release --test effects_granular`.

#[allow(dead_code)]
mod rig;

mod mix;
mod package;
mod playback;
