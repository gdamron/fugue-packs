//! Click-free control changes: a live write moves the sound without a step.

use fugue::RenderEngine;
use serde_json::{json, Value};

use super::rig::*;

/// Time to settle after a write, and the length of a steady window.
const SETTLE: usize = 2 * SR as usize;
const WINDOW: usize = SR as usize / 2;
/// "the 100 ms after the write".
const JUMP: usize = SR as usize / 10;

type Sides = (Vec<f32>, Vec<f32>);

/// A steady stretch of output: controls constant for long enough to settle.
fn steady(fx: &mut RenderEngine) -> Sides {
    render(fx, SETTLE);
    render(fx, WINDOW)
}

/// Writes `key = value` and asserts "the largest sample step in the 100 ms
/// after the write is no more than 1.5x the larger of the steady-state
/// largest steps (controls constant) before and after the change", on both
/// sides. Returns the steady output after the change.
fn assert_glides(
    what: &str,
    fx: &mut RenderEngine,
    key: &str,
    value: Value,
    before: &Sides,
) -> Sides {
    match value {
        Value::Bool(b) => set(fx, key, b),
        Value::String(s) => set(fx, key, s.as_str()),
        other => set(fx, key, other.as_f64().expect("a number") as f32),
    }
    let jump = render(fx, JUMP);
    let after = steady(fx);
    let sides = [
        ("left", &before.0, &jump.0, &after.0),
        ("right", &before.1, &jump.1, &after.1),
    ];
    for (side, before, jump, after) in sides {
        let steady_step = max_step(before).max(max_step(after));
        assert!(steady_step > 1e-3, "{what} {side}: heard ({steady_step})");
        // The step from the last sample before the write is part of it.
        let span = [&before[before.len() - 1..], jump].concat();
        let step = max_step(&span);
        assert!(
            step <= 1.5 * steady_step,
            "{what} {side}: step {step} after the write, steady {steady_step}"
        );
    }
    after
}

/// A 220 Hz sine at 0.5 into the pack built with `config`, halved on the
/// way to the dac, settled.
fn playing(config: Value) -> (RenderEngine, Sides) {
    let mut fx = engine(&Feed::sine(220.0, 0.5).effect(config, 0.5));
    let before = steady(&mut fx);
    (fx, before)
}

/// A jump of `key` between 0 and 1, each way, with the other controls at
/// their defaults.
fn assert_jumps_glide(key: &str) {
    for (from, to) in [(0.0, 1.0), (1.0, 0.0)] {
        let (mut fx, before) = playing(json!({ key: from }));
        assert_glides(
            &format!("{key} {from} -> {to}"),
            &mut fx,
            key,
            json!(to),
            &before,
        );
    }
}

#[test]
fn a_jump_of_dry_glides() {
    assert_jumps_glide("dry");
}

#[test]
fn a_jump_of_wet_glides() {
    assert_jumps_glide("wet");
}

#[test]
fn a_jump_of_reverb_glides() {
    assert_jumps_glide("reverb");
}

/// In `stretch` with `dry` 0, `wet` 1, `reverb` 0, "toggling `freeze` on and
/// off ... keep[s] the largest step within 1.5x steady state".
#[test]
fn toggling_freeze_in_stretch_glides() {
    let (mut fx, before) =
        playing(json!({ "mode": "stretch", "dry": 0.0, "wet": 1.0, "reverb": 0.0 }));
    let frozen = assert_glides("freeze on", &mut fx, "freeze", json!(true), &before);
    assert_glides("freeze off", &mut fx, "freeze", json!(false), &frozen);
}

/// Likewise "changing `mode` to `looping_delay`".
#[test]
fn changing_mode_from_stretch_to_looping_delay_glides() {
    let (mut fx, before) =
        playing(json!({ "mode": "stretch", "dry": 0.0, "wet": 1.0, "reverb": 0.0 }));
    assert_glides("mode", &mut fx, "mode", json!("looping_delay"), &before);
}
