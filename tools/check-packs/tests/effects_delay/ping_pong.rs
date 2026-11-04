//! `fugue.effects.ping-pong-delay`: one time for both heads, echoes
//! alternating left, right, left.

use serde_json::json;

use super::rig::*;

/// "an impulse given to both inputs gives a first echo on the left only at
/// 12,000 samples, the second on the right only at about 24,000 (+ up to a
/// sample per outside-loop pass), the third on the left at about 36,000,
/// each echo about x `feedback` from the one before", and "a signal patched
/// to both inputs enters at unity" (the first echo is the pulse at the `wet`
/// level, 0.5). Assumption: "about" is within 15% for levels, and within 2
/// samples per pass for onsets.
#[test]
fn echoes_alternate_left_right_left_and_decay_by_feedback() {
    let (left, right) = render_doc(&pulse_both().effect(PING_PONG, json!({}), 1.0), 40_000);
    let start = onset(&left, 0).expect("the dry pulse");
    assert_eq!(onset(&right, 0), Some(start), "dry is heard on both sides");
    let after_dry = start + PULSE_LEN + 100;

    let first = onset(&left, after_dry).expect("a first echo on the left");
    assert_eq!(first, start + 12_000, "first echo, pulse at {start}");
    let second = onset(&right, after_dry).expect("a second echo on the right");
    assert!(
        (start + 24_000..=start + 24_002).contains(&second),
        "second echo at {second}"
    );
    let third = onset(&left, first + 2_000).expect("a third echo on the left");
    assert!(
        (start + 36_000..=start + 36_004).contains(&third),
        "third echo at {third}"
    );
    // The onsets above leave each side silent between its echoes; the right
    // stays silent through the third.
    assert!(
        peak(&right[second + 2_000..third + 2_000]) <= EPS,
        "right is silent at the third"
    );

    let (e1, e2, e3) = (
        plateau(&left, first),
        plateau(&right, second),
        plateau(&left, third),
    );
    assert_near("first echo level", e1, PULSE_AMP as f32 * 0.5, 0.15);
    assert_near("second echo level", e2, e1 * 0.35, 0.15);
    assert_near("third echo level", e3, e2 * 0.35, 0.15);
}

/// The `bpm` input sets the tempo for both heads (the control, at 60, is
/// ignored): at 100 bpm a 1/8 is 14,400 samples, left then right.
#[test]
fn the_bpm_input_retimes_both_heads() {
    let patched = Rig::new().tempo("tempo", 100.0).feed("tempo", "bpm", "bpm");
    let doc = pulse_both()
        .with(&patched)
        .effect(PING_PONG, json!({ "bpm": 60.0 }), 1.0);
    let (left, right) = render_doc(&doc, 32_000);
    let start = onset(&left, 0).expect("the dry pulse");
    let after_dry = start + PULSE_LEN + 100;
    assert_eq!(onset(&left, after_dry), Some(start + 14_400), "left echo");
    let second = onset(&right, after_dry).expect("a right echo");
    assert!(
        (start + 28_800..=start + 28_802).contains(&second),
        "right echo at {second}"
    );
}
