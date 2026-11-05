//! The output bound under full-scale noise, including at `feedback` 1.

use serde_json::{json, Value};

use super::rig::*;

/// Renders `seconds` of 0 dBFS white noise in both inputs and asserts both
/// outputs stay finite and within ±`bound` throughout. The outputs reach
/// the dac through a tap of 1/`scale` (a power of two, so exact), so the dac
/// cannot hide an overshoot of up to `scale`.
fn assert_bounded(what: &str, config: Value, seconds: usize, bound: f32, scale: f64) {
    let mut fx = engine(&Feed::noise_both().effect(config, 1.0 / scale));
    for second in 0..seconds {
        let (left, right) = render(&mut fx, SR as usize);
        for (side, x) in [("left", &left), ("right", &right)] {
            assert!(x.iter().all(|v| v.is_finite()), "{what} {side}: finite");
            let p = peak(x) * scale as f32;
            assert!(
                p <= bound + 1e-4,
                "{what} {side}: peak {p} in second {second}"
            );
        }
    }
}

/// "With `reverb` 0 and `dry` + `wet` <= 1, inputs within ±1 (e.g. 0 dBFS
/// white noise) keep both outputs within ±1.0, including at `feedback` 1 for
/// 60 s", at a few splits of the level.
#[test]
fn dry_plus_wet_at_most_one_stays_within_unity_for_a_minute_at_feedback_1() {
    for (dry, wet) in [(0.0, 1.0), (0.5, 0.5), (1.0, 0.0), (0.3, 0.7)] {
        let config = json!({ "dry": dry, "wet": wet, "reverb": 0.0, "feedback": 1.0 });
        assert_bounded(&format!("dry {dry} wet {wet}"), config, 60, 1.0, 4.0);
    }
}

/// The same bound in each mode, and with the controls moved to where a
/// grain engine is busiest (full density, spread and size, the largest
/// pitch shift), for ten seconds.
#[test]
fn the_bound_holds_in_every_mode_and_with_busy_controls() {
    for mode in ["granular", "stretch", "looping_delay"] {
        for extra in [
            json!({}),
            json!({ "density": 1.0, "spread": 1.0, "size": 1.0, "pitch": 24.0, "texture": 1.0 }),
        ] {
            let config = with(
                &json!({ "mode": mode, "dry": 0.5, "wet": 0.5, "reverb": 0.0, "feedback": 1.0 }),
                extra.clone(),
            );
            assert_bounded(&format!("{mode} {extra}"), config, 10, 1.0, 4.0);
        }
    }
}

/// "With every level at 1 and the reverb at its largest and least damped
/// (`reverb_size` 1, `reverb_decay` 1, `reverb_damping` 0), `feedback` 1,
/// 60 s of 0 dBFS noise keeps both outputs finite and within ±16."
#[test]
fn everything_at_the_top_for_a_minute_stays_finite_and_within_16() {
    let top = json!({ "dry": 1.0, "wet": 1.0, "reverb": 1.0, "reverb_size": 1.0,
        "reverb_decay": 1.0, "reverb_damping": 0.0, "feedback": 1.0 });
    assert_bounded("granular", top.clone(), 60, 16.0, 32.0);
    for mode in ["stretch", "looping_delay"] {
        assert_bounded(mode, with(&top, json!({ "mode": mode })), 20, 16.0, 32.0);
    }
}
