//! `fugue.effects.flanger`: the 1 s sweep of `rate` and `depth`, a fast
//! sweep of `feedback` through both polarities, and the bound at maximum
//! feedback.
//!
//! The flanger's delay moves `depth` x 1.5 ms around `time`, so at `rate`
//! 100 Hz and `depth` 1 it moves 0.94 samples per sample. The sweeps run
//! with `feedback` 0 (so the wet path is one pass through the line) and
//! `dry` 0.4, `wet` 0.25, except one run at the default `feedback` 0.5
//! with `dry` and `wet` lowered to match the comb's boost.

use serde_json::{json, Value};

use super::rig::*;
use super::zipper::*;

const OPEN: (f32, f32) = (0.4, 0.25);
const COMB: (f32, f32) = (0.2, 0.2);

fn sweep(
    name: &'static str,
    rate: (f32, f32),
    depth: (f32, f32),
    log_rate: bool,
    (dry, wet): (f32, f32),
) -> Sweep {
    Sweep {
        name,
        rate,
        depth,
        log_rate,
        dry,
        wet,
    }
}

#[test]
fn sweeping_rate_and_depth_over_their_ranges_makes_no_step() {
    let cases = [
        sweep("rate up, depth down", SLOW_FAST, FULL_DOWN, false, OPEN),
        sweep("rate down, depth up", FAST_SLOW, FULL, false, OPEN),
        sweep("both up", SLOW_FAST, FULL, false, OPEN),
        sweep("both down", FAST_SLOW, FULL_DOWN, false, OPEN),
        sweep("both up, log rate", SLOW_FAST, FULL, true, OPEN),
    ];
    for case in &cases {
        check_sweep(FLANGER, case, 0.5, json!({ "feedback": 0.0 }));
    }
}

/// The same with the default `feedback` 0.5 recirculating (the wet path's
/// comb boosts a low tone up to 2x), on a gentler pair of levels.
#[test]
fn sweeping_with_default_feedback_makes_no_step() {
    let case = sweep("rate up, depth down", SLOW_FAST, FULL_DOWN, false, COMB);
    check_sweep(FLANGER, &case, 0.5, json!({ "feedback": 0.5 }));
    let case = sweep("rate down, depth up", FAST_SLOW, FULL, false, COMB);
    check_sweep(FLANGER, &case, 0.5, json!({ "feedback": 0.5 }));
}

/// `feedback` as a triangle of 0 to 0.8 with a period of `period` seconds
/// for 1 s, written ahead of every chunk (edges at every offset in the
/// engine's blocks), with `negative_feedback` flipped at each trough (zero
/// feedback) or at each peak (0.8) of it. The input's 0.15 peak keeps the
/// comb's resonant boost (up to 5x at 0.8) under the line's +/-1.
fn feedback_sweep(what: &str, period: f32, flip_at_peak: bool) {
    const AMP: f64 = 0.15;
    let rig = Rig::new().sine("s", FREQ, AMP).feed_both("s", "audio");
    let config = json!({ "feedback": 0.0, "negative_feedback": false, "dry": 0.2, "wet": 0.15 });
    let mut fx = engine(&rig.effect(FLANGER, config, 1.0));
    render(&mut fx, 12_000);
    let mut negative = false;
    let out = render_chunked(&mut fx, SR as usize, |pos, fx| {
        let t = pos as f32 / SR as f32;
        let cycle = t / period;
        let triangle = 1.0 - (2.0 * cycle.fract() - 1.0).abs();
        set(fx, "feedback", 0.8 * triangle);
        let flips = if flip_at_peak { cycle + 0.5 } else { cycle };
        let want = (flips as usize) % 2 == 1;
        if want != negative {
            negative = want;
            set(fx, "negative_feedback", negative);
        }
    });
    let (dry, _) = render_doc(&rig.dry(), 12_000 + SR as usize);
    assert_bounded_steps(
        &format!("flanger feedback sweep '{what}'"),
        (&out.0, &out.1),
        &dry[12_000..],
    );
}

#[test]
fn sweeping_feedback_through_both_polarities_makes_no_step() {
    feedback_sweep("period 0.2 s, flipped at zero feedback", 0.2, false);
    feedback_sweep("period 0.2 s, flipped at 0.8 feedback", 0.2, true);
    feedback_sweep("period 0.05 s, flipped at 0.8 feedback", 0.05, true);
}

/// A source of full scale: white noise, an independent seed per side, a
/// full-scale 1 kHz sine, and a constant 1.0 (feedback at 1 sums it up
/// the fastest).
fn full_scale(kind: &str) -> Rig {
    let rig = Rig::new();
    match kind {
        "noise" => (rig.noise("nl", 11, 1.0))
            .noise("nr", 12, 1.0)
            .feed("nl", "white", "audio_left")
            .feed("nr", "white", "audio_right"),
        "sine" => (rig.sine("s", FREQ, 1.0)).feed_both("s", "audio"),
        _ => (rig.constant("c", 1.0)).feed_both("c", "audio"),
    }
}

/// Renders `seconds` in whole seconds through a 0.5 tap and asserts the
/// output is finite and within +/-1.0 (the tap cannot hide an overshoot).
fn assert_within_unity(what: &str, fx: &mut fugue::RenderEngine, seconds: usize) {
    for second in 0..seconds {
        let (left, right) = render(fx, SR as usize);
        for (side, x) in [("left", &left), ("right", &right)] {
            assert!(x.iter().all(|v| v.is_finite()), "{what}: {side} is finite");
            let p = peak(x) * 2.0;
            assert!(
                p <= 1.0 + 1e-6,
                "{what}: {side} peak {p} in second {second}"
            );
        }
    }
}

/// "At max feedback, 0 dBFS noise stays within +/-1.0": feedback 1.0 with
/// the default levels (dry 0.5, wet 0.5), both polarities, at the shortest
/// `time` 0.0005 s, the default 0.002 s and the longest 0.02 s, for 5 s.
/// Noise is also run at full `depth` (which clamps the delay at its
/// shortest); the sine and the constant run at the default depth.
#[test]
fn full_scale_input_stays_within_unity_at_maximum_feedback() {
    for kind in ["noise", "sine", "constant"] {
        for time in [0.0005, 0.002, 0.02] {
            for negative in [false, true] {
                let depths: &[f64] = if kind == "noise" { &[0.5, 1.0] } else { &[0.5] };
                for depth in depths {
                    let config = full_scale_config(time, negative, *depth);
                    let mut fx = engine(&full_scale(kind).effect(FLANGER, config, 0.5));
                    let what = format!("{kind}, time {time}, negative {negative}, depth {depth}");
                    assert_within_unity(&what, &mut fx, 5);
                }
            }
        }
    }
}

fn full_scale_config(time: f64, negative: bool, depth: f64) -> Value {
    json!({ "feedback": 1.0, "time": time, "negative_feedback": negative, "depth": depth })
}

/// Toggling the polarity live at maximum feedback, every 0.37 s (off the
/// block grid), on full-scale noise at the shortest and default `time`.
#[test]
fn toggling_polarity_at_maximum_feedback_stays_within_unity() {
    for time in [0.0005, 0.002] {
        let config = full_scale_config(time, false, 0.5);
        let mut fx = engine(&full_scale("noise").effect(FLANGER, config, 0.5));
        let mut negative = false;
        let (left, right) = render_chunked(&mut fx, 5 * SR as usize, |pos, fx| {
            let want = (pos as f32 / (0.37 * SR as f32)) as usize % 2 == 1;
            if want != negative {
                negative = want;
                set(fx, "negative_feedback", negative);
            }
        });
        for (side, x) in [("left", &left), ("right", &right)] {
            let what = format!("toggled, time {time}: {side}");
            assert!(x.iter().all(|v| v.is_finite()), "{what} is finite");
            let p = peak(x) * 2.0;
            assert!(p <= 1.0 + 1e-6, "{what}: peak {p}");
        }
    }
}
