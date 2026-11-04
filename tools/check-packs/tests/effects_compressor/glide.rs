//! "Control changes glide; none clicks."
//!
//! The audio is a constant, so the output is that constant times the
//! compressor's gain and any click is a step in the gain itself, with no
//! waveform to hide in. A change made with no glide puts its whole
//! excursion into one sample; the atoms pin their glides at about 2.7 ms
//! (128 frames) or 20 ms, which put a few percent of it into each sample.
//! The bound: no one-sample step exceeds a tenth of the output's whole
//! excursion around the change, leaving room for a glide shaped in dB or
//! as a one-pole rather than a straight line.

use serde_json::json;

use super::rig::*;

/// Frames kept from before the change, all steady.
const LEAD: usize = 100;

/// Asserts the largest step in `x` is at most a tenth of its excursion,
/// and that the excursion is at least `min_change` (the change is heard).
fn assert_glides(what: &str, x: &[f32], min_change: f32) {
    let (lo, hi) = x
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let excursion = hi - lo;
    assert!(
        excursion >= min_change,
        "{what}: the change is heard (output moves {excursion}, want at least {min_change})"
    );
    let step = max_step(x);
    let at = x.windows(2).position(|w| (w[1] - w[0]).abs() == step);
    assert!(
        step <= excursion / 10.0 + 1e-6,
        "{what}: a step of {step} at {at:?} in an excursion of {excursion} clicks"
    );
}

/// Renders `rig` through the pack built with `config` until settled,
/// writes `key` = `value` live, renders 0.5 s more, and checks both sides.
fn change(
    what: &str,
    rig: &Rig,
    config: serde_json::Value,
    key: &str,
    value: f32,
    min_change: f32,
) {
    let mut fx = engine(&rig.effect(config));
    let (bl, br) = render(&mut fx, SETTLE);
    set(&fx, key, value);
    let (al, ar) = render(&mut fx, SR as usize / 2);
    for (side, before, after) in [("left", bl, al), ("right", br, ar)] {
        let mut joined = before[SETTLE - LEAD..].to_vec();
        joined.extend_from_slice(&after);
        assert_glides(
            &format!("{what}: {key} -> {value}, {side}"),
            &joined,
            min_change,
        );
    }
}

fn steady(amp: f64) -> Rig {
    Rig::new().constant("a", amp).feed_both("a", "audio")
}

/// Curve and output controls on a steady 0.5 (-6.02 dBFS, c = -10.49 dB).
/// Each write moves the output by at least 0.01 (the knee's by more than
/// 0.004 on a steady 0.1 at the threshold, where it matters).
#[test]
fn curve_makeup_and_mix_changes_glide_without_a_click() {
    let cases: [(&str, f64, &str, f32, f32); 9] = [
        ("steady 0.5", 0.5, "threshold", -40.0, 0.05),
        ("steady 0.5", 0.5, "threshold", 0.0, 0.05),
        ("steady 0.5", 0.5, "ratio", 20.0, 0.01),
        ("steady 0.5", 0.5, "ratio", 1.0, 0.1),
        ("steady 0.1", 0.1, "knee", 0.0, 0.004),
        ("steady 0.5", 0.5, "makeup", 12.0, 0.1),
        ("steady 0.5", 0.5, "makeup", -24.0, 0.1),
        ("steady 0.5", 0.5, "mix", 0.0, 0.1),
        ("steady 0.5", 0.5, "highpass", 500.0, 0.1),
    ];
    for (what, amp, key, value, min_change) in cases {
        change(what, &steady(amp), json!({}), key, value, min_change);
    }
}

/// `sidechain_mix` 0 -> 1 with a steady 1.0 key: the audio (a steady 0.05,
/// below the threshold) is ducked by -15 dB without a click.
#[test]
fn switching_to_the_sidechain_glides_without_a_click() {
    let rig = steady(0.05)
        .with(&Rig::new().constant("k", 1.0))
        .feed("k", "audio", "sidechain");
    change(
        "steady 0.05, key 1.0",
        &rig,
        json!({}),
        "sidechain_mix",
        1.0,
        0.02,
    );
}

/// `attack` and `release` changed while the detector moves: a 1.0 key for
/// 1 s of every 3 s ducks a steady 0.05. `release` goes 0.1 -> 1.0 50 ms
/// after the key stops (mid-recovery) and `attack` 0.01 -> 0.2 2 ms after
/// it returns at 3 s (mid-attack). From the second sample on (the first is
/// the audio's own onset), no step exceeds a tenth of the ducking.
#[test]
fn attack_and_release_changes_mid_movement_do_not_click() {
    let rig = steady(0.05)
        .with(&Rig::new().gated("k", 1.0, 1.0))
        .feed("k", "audio", "sidechain");
    let mut fx = engine(&rig.effect(json!({ "sidechain_mix": 1.0 })));
    let sr = SR as usize;
    let mut out: [Vec<f32>; 2] = [Vec::new(), Vec::new()];
    let mut run = |fx: &mut fugue::RenderEngine, frames: usize| {
        let (l, r) = render(fx, frames);
        out[0].extend(l);
        out[1].extend(r);
    };
    run(&mut fx, sr + sr / 20);
    set(&fx, "release", 1.0f32);
    run(&mut fx, 3 * sr + sr / 500 - (sr + sr / 20));
    set(&fx, "attack", 0.2f32);
    run(&mut fx, sr);
    for (side, x) in ["left", "right"].iter().zip(&out) {
        assert_glides(&format!("attack and release, {side}"), &x[1..], 0.02);
    }
}
