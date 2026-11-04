//! `fugue.effects.stereo-delay`: two independent heads, dotted 1/8 on the
//! left and 1/4 on the right at 120 bpm.

use serde_json::json;

use super::rig::*;

/// "an impulse on `audio_left` gives a first echo on the left only, 18,000
/// samples later at 48 kHz (dotted 1/8), and nothing on the right; an
/// impulse on `audio_right` gives a first echo on the right only at 24,000
/// samples (1/4)".
#[test]
fn each_side_echoes_on_its_own_side_only_at_its_division() {
    for (input, delay) in [("audio_left", 18_000), ("audio_right", 24_000)] {
        let rig = Rig::new().pulse("p").feed("p", "audio", input);
        let (left, right) = render_doc(&rig.effect(STEREO, json!({}), 1.0), 30_000);
        let (heard, other) = if input == "audio_left" { (left, right) } else { (right, left) };
        let start = onset(&heard, 0).expect("the dry pulse is heard");
        let echo = onset(&heard, start + PULSE_LEN + 100);
        assert_eq!(echo, Some(start + delay), "{input}: first echo, pulse at {start}");
        assert!(peak(&other) <= EPS, "{input}: the other side is silent, peak {}", peak(&other));
    }
}

/// "Echo k (k >= 1) on a side is at k x time + (k - 1) samples, each x
/// `feedback` from the one before ... with `tone` high an impulse's echoes
/// decay by about `feedback` per repeat", heard at the `wet` level.
/// Assumption: "about" is within 10%, and later onsets fall within 2 samples
/// of k x time + (k - 1) (the lowpass may smear an onset).
#[test]
fn repeats_are_spaced_time_plus_one_and_decay_by_feedback() {
    for (feedback, wet) in [(0.35f32, 0.8f32), (0.6, 1.0)] {
        let config = json!({ "feedback": feedback, "wet": wet, "tone": 20_000.0 });
        let rig = Rig::new().pulse("p").feed("p", "audio", "audio_left");
        let (left, _) = render_doc(&rig.effect(STEREO, config, 1.0), 76_000);
        let start = onset(&left, 0).expect("the dry pulse");
        let mut from = start + PULSE_LEN + 100;
        let mut want_level = plateau(&left, start) * wet;
        for k in 1..=4 {
            let want = start + k * 18_000 + (k - 1);
            let at = onset(&left, from).unwrap_or_else(|| panic!("echo {k} is heard"));
            let slack = if k == 1 { 0 } else { 2 };
            let msg = format!("feedback {feedback}: echo {k} at {at}, want {want}");
            assert!(at >= want && at <= want + slack, "{msg}");
            let heard = plateau(&left, at);
            assert_near(&format!("{msg}, level"), heard, want_level, 0.1);
            want_level = heard * feedback;
            from = at + PULSE_LEN + 1_000;
        }
    }
}

/// "`bpm` ... while patched it sets the tempo for every delay head": at 100
/// bpm a dotted 1/8 is 21,600 samples and a 1/4 is 28,800, with the `bpm`
/// control at 60 (the input wins). The `bpm` control alone retimes too.
#[test]
fn the_bpm_input_and_control_retime_both_heads() {
    let rig = pulse_both();
    let patched = Rig::new().tempo("tempo", 100.0).feed("tempo", "bpm", "bpm");
    let cases = [
        ("bpm input", rig.with(&patched).effect(STEREO, json!({ "bpm": 60.0 }), 1.0)),
        ("bpm control", rig.effect(STEREO, json!({ "bpm": 100.0 }), 1.0)),
    ];
    for (what, doc) in cases {
        let (left, right) = render_doc(&doc, 32_000);
        let start = onset(&left, 0).expect("the dry pulse");
        let after = start + PULSE_LEN + 100;
        assert_eq!(onset(&left, after), Some(start + 21_600), "{what}: left");
        assert_eq!(onset(&right, after), Some(start + 28_800), "{what}: right");
    }
}
