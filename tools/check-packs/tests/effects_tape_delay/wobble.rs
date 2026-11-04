//! "Wow and flutter move the pitch: a steady 1 kHz sine's echo has a
//! constant period with both at 0. With `wow` at 1, or `flutter` at 1, the
//! period varies measurably, for example zero-crossing intervals spread over
//! more than 1%."
//!
//! The echo alone is heard (`dry` 0, `wet` 1, `feedback` 0). The periods
//! are measured over 2.5 s of the steady echo, more than one cycle of the
//! slow (~0.55 Hz) wobble. "Spread" is (longest - shortest) / mean period.

use serde_json::json;

use super::rig::*;
use super::tape::*;

const TONE: f64 = 1_000.0;

/// The left echo's steady 2.5 s, and checks the right hears the same.
fn echo(wow: f64, flutter: f64) -> Vec<f32> {
    let config = json!({
        "dry": 0.0, "wet": 1.0, "feedback": 0.0, "wow": wow, "flutter": flutter
    });
    let from = TIME + 4_800;
    let to = from + 5 * SR as usize / 2;
    let (left, right) = render_doc(&sine_doc(config, TONE, 0.1, 1.0), to);
    let diff = (left.iter().zip(&right)).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
    assert!(
        diff <= 1e-6,
        "wow {wow} flutter {flutter}: sides differ by {diff}"
    );
    left[from..to].to_vec()
}

/// "With both at 0 the delay time holds still": the echo is a 1 kHz sine
/// whose period does not vary.
#[test]
fn with_wow_and_flutter_at_zero_the_period_is_constant() {
    let x = echo(0.0, 0.0);
    let f = frequency(&x);
    assert!((f / TONE - 1.0).abs() < 0.001, "the echo is at {f} Hz");
    let s = spread(&x);
    assert!(s < 0.002, "the period spreads by {:.3}%", s * 100.0);
}

#[test]
fn wow_at_full_depth_wavers_the_pitch() {
    let s = spread(&echo(1.0, 0.0));
    assert!(s > 0.01, "wow 1: the period spreads by {:.3}%", s * 100.0);
}

#[test]
fn flutter_at_full_depth_wavers_the_pitch() {
    let s = spread(&echo(0.0, 1.0));
    assert!(
        s > 0.01,
        "flutter 1: the period spreads by {:.3}%",
        s * 100.0
    );
}

/// The default depths (`wow` 0.25, `flutter` 0.2) waver the pitch too, if
/// less: the spread is above the still head's.
#[test]
fn the_default_depths_waver_the_pitch_a_little() {
    let config = json!({ "dry": 0.0, "wet": 1.0, "feedback": 0.0 });
    let from = TIME + 4_800;
    let to = from + 5 * SR as usize / 2;
    let (left, _) = render_doc(&sine_doc(config, TONE, 0.1, 1.0), to);
    let s = spread(&left[from..to]);
    assert!(
        s > 0.002,
        "defaults: the period spreads by {:.3}%",
        s * 100.0
    );
}
