//! The output bound under full-scale noise, and self-oscillation.
//!
//! The outputs reach the dac through a 0.5 tap (`effect(.., 0.5)`), so the
//! dac cannot hide an overshoot; levels below are scaled back by 2.

use fugue::ControlValue;
use serde_json::{json, Value};

use super::rig::*;
use super::tape::*;

/// Renders `seconds` of 0 dBFS white noise in both inputs and asserts both
/// outputs stay finite and within ±1.0 throughout.
fn assert_bounded(what: &str, config: Value, seconds: usize) {
    let mut fx = engine(&noise_both().effect(TAPE, config, 0.5));
    for second in 0..seconds {
        let (left, right) = render(&mut fx, SR as usize);
        for (side, x) in [("left", &left), ("right", &right)] {
            assert!(x.iter().all(|v| v.is_finite()), "{what} {side}: finite");
            let p = peak(x) * 2.0;
            assert!(
                p <= 1.0 + 1e-6,
                "{what} {side}: peak {p} in second {second}"
            );
        }
    }
}

/// "With `dry` 0, `wet` 1 and `feedback` 1.1, 0 dBFS white noise in both
/// inputs for 60 s keeps both outputs within ±1.0 and finite."
#[test]
fn noise_for_a_minute_at_feedback_1_1_stays_within_unity() {
    let config = json!({ "dry": 0.0, "wet": 1.0, "feedback": 1.1 });
    assert_bounded("feedback 1.1", config, 60);
}

/// "The same holds at `feedback` 2", at the default `tone` and a bright one.
#[test]
fn noise_for_a_minute_at_feedback_2_stays_within_unity() {
    let config = json!({ "dry": 0.0, "wet": 1.0, "feedback": 2.0 });
    assert_bounded("feedback 2", config, 60);
    let bright = json!({ "dry": 0.0, "wet": 1.0, "feedback": 2.0, "tone": 20_000.0 });
    assert_bounded("feedback 2, tone 20000", bright, 10);
}

/// "With `dry` + `wet` <= 1, the output stays within ±1.0 for inputs within
/// ±1.0": 10 s of full-scale noise at a few splits and feedbacks, with wow
/// and flutter at full depth.
#[test]
fn dry_plus_wet_at_most_one_keeps_the_output_within_unity() {
    for (dry, wet, feedback) in [(0.5, 0.5, 1.1), (0.3, 0.7, 0.4), (0.7, 0.3, 2.0)] {
        let what = format!("dry {dry} wet {wet} feedback {feedback}");
        let config = json!({
            "dry": dry, "wet": wet, "feedback": feedback,
            "tone": 18_000.0, "wow": 1.0, "flutter": 1.0
        });
        assert_bounded(&what, config, 10);
    }
}

/// Plays a 0.2 s noise burst into both inputs with `dry` 0 and `wet` 1,
/// silences it, and returns the next 6 s of (left, right), scaled back from
/// the tap.
fn after_burst(feedback: f64) -> (Vec<f32>, Vec<f32>) {
    let config = json!({ "dry": 0.0, "wet": 1.0, "feedback": feedback });
    let mut fx = engine(&noise_both().effect(TAPE, config, 0.5));
    render(&mut fx, SR as usize / 5);
    for id in ["nl", "nr"] {
        (fx.set_control(id, "level", ControlValue::Number(0.0)))
            .unwrap_or_else(|err| panic!("{id} silences: {err}"));
    }
    let (left, right) = render(&mut fx, 6 * SR as usize);
    // With `dry` 0 nothing is heard until the first echo at 0.3 s, 0.1 s
    // into what follows; the burst reached the pack if it echoes.
    let echo = peak(&left[..SR as usize / 2]) * 2.0;
    assert!(
        echo > 0.01,
        "feedback {feedback}: the burst echoes ({echo})"
    );
    let scale = |x: Vec<f32>| x.into_iter().map(|v| v * 2.0).collect::<Vec<_>>();
    (scale(left), scale(right))
}

/// "at `feedback` 1.1, after a burst of input stops, the echoes do not die
/// away (5 s later the output is still clearly audible, above -40 dBFS) and
/// stay within ±1.0". Read as the RMS over the half second starting 5 s
/// after the burst. At `feedback` 0.5 the same burst has died away by then
/// (repeats decay by about `feedback`), so the measure tells the two apart.
#[test]
fn at_feedback_1_1_the_echoes_self_oscillate_but_stay_bounded() {
    let window = 5 * SR as usize..5 * SR as usize + SR as usize / 2;
    let (left, right) = after_burst(1.1);
    for (side, x) in [("left", &left), ("right", &right)] {
        assert!(x.iter().all(|v| v.is_finite()), "{side}: finite");
        let p = peak(x);
        assert!(p <= 1.0 + 1e-6, "{side}: peak {p}");
        let level = rms(&x[window.clone()]);
        let db = 20.0 * level.log10();
        assert!(db > -40.0, "{side}: 5 s after the burst it is at {db} dBFS");
    }

    let (left, _) = after_burst(0.5);
    let db = 20.0 * rms(&left[window]).log10();
    assert!(
        db < -40.0,
        "feedback 0.5: the echoes die away, still {db} dBFS"
    );
}
