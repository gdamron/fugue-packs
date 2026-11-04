//! Behaviour both packs share: the output bound, `wet` 0, and the control
//! surface reached by its exposed names.

use fugue::ControlValue;
use serde_json::json;

use super::rig::*;

fn noise_rig() -> Rig {
    Rig::new().noise("nl", 31).noise("nr", 32).feed("nl", "white", "audio_left").feed(
        "nr",
        "white",
        "audio_right",
    )
}

/// "Before the first echo, the output is exactly the dry input", each input
/// on its own side: noise is bit-identical to the same noise wired straight
/// to the dac until each side's first echo.
#[test]
fn output_before_the_first_echo_is_exactly_the_dry_input() {
    let (dry_left, dry_right) = render_doc(&noise_rig().dry(), 24_000);
    assert!(peak(&dry_left) > 0.5, "the reference noise is heard");
    for (pack, left_echo, right_echo) in [(STEREO, 18_000, 24_000), (PING_PONG, 12_000, 12_000)] {
        let (left, right) = render_doc(&noise_rig().effect(pack, json!({}), 1.0), 24_000);
        assert_same(&format!("{pack} left"), &left, &dry_left, 0..left_echo);
        assert_same(&format!("{pack} right"), &right, &dry_right, 0..right_echo);
    }
}

/// "Output stays within +/-1.0 for inputs within +/-1.0 at any control
/// setting": full-scale noise on both inputs, `feedback` 1.0, `wet` 1.0 and
/// `dry` at its default 1.0, for 10 s, at the default `tone` and a bright
/// one. The outputs reach the dac through a 0.5 tap so the dac cannot hide
/// an overshoot.
#[test]
fn output_stays_within_unity_at_full_feedback_and_wet() {
    for pack in BOTH {
        for tone in [4_000.0, 20_000.0] {
            let config = json!({ "feedback": 1.0, "wet": 1.0, "tone": tone });
            let mut engine = engine(&noise_rig().effect(pack, config, 0.5));
            for second in 0..10 {
                let (left, right) = render(&mut engine, SR as usize);
                for (side, x) in [("left", &left), ("right", &right)] {
                    assert!(x.iter().all(|v| v.is_finite()), "{pack}: finite");
                    let p = peak(x) * 2.0;
                    assert!(
                        p <= 1.0 + 1e-6,
                        "{pack} tone {tone}: {side} peak {p} in second {second}"
                    );
                }
            }
        }
    }
}

/// "dry passes at unity when `wet` is 0": with `wet` 0 the output is the
/// dry input exactly, over 3 s, well past the echoes.
#[test]
fn with_wet_at_zero_the_output_is_exactly_the_dry_input() {
    let frames = 3 * SR as usize;
    let (dry_left, dry_right) = render_doc(&noise_rig().dry(), frames);
    for pack in BOTH {
        let doc = noise_rig().effect(pack, json!({ "wet": 0.0, "feedback": 0.9 }), 1.0);
        let (left, right) = render_doc(&doc, frames);
        assert_same(&format!("{pack} left"), &left, &dry_left, 0..frames);
        assert_same(&format!("{pack} right"), &right, &dry_right, 0..frames);
    }
}

/// The shared controls, with their pinned defaults, and each pack's own.
fn expected(pack: &str) -> Vec<(&'static str, ControlValue)> {
    use ControlValue::{Bool, Number, String as Choice};
    let mut controls = vec![
        ("bpm", Number(120.0)),
        ("feedback", Number(0.35)),
        ("tone", Number(4000.0)),
        ("dry", Number(1.0)),
        ("wet", Number(0.5)),
        ("glide", Choice("crossfade".into())),
        ("hold", Bool(false)),
    ];
    if pack == STEREO {
        controls.extend([
            ("time_left", Number(0.375)),
            ("time_right", Number(0.5)),
            ("division_left", Choice("dotted_8th_note".into())),
            ("division_right", Choice("quarter_note".into())),
        ]);
    } else {
        controls.extend([("time", Number(0.25)), ("division", Choice("8th_note".into()))]);
    }
    controls
}

/// The exposed controls are listed by name and start at their defaults.
#[test]
fn controls_are_exposed_by_name_with_their_defaults() {
    for pack in BOTH {
        let engine = engine(&noise_rig().effect(pack, json!({}), 1.0));
        let listed = engine.list_controls(Some("fx")).expect("fx lists controls");
        let keys: Vec<_> = listed.iter().flat_map(|(_, c)| c.iter().map(|m| &m.key)).collect();
        for (key, default) in expected(pack) {
            assert!(keys.iter().any(|k| *k == key), "{pack} lists {key}: {keys:?}");
            let value = engine.get_control("fx", key).expect("readable");
            assert_eq!(value, default, "{pack}.{key} default");
        }
    }
}

/// Setting controls by their exposed names works live: a division turned
/// `off` uses its time (0.1 s = 4,800 samples), and `dry` and `wet` at 0
/// silence the output. Every shared control accepts a write.
#[test]
fn controls_set_by_name_change_what_is_heard() {
    for pack in BOTH {
        let mut fx = engine(&pulse_both().effect(pack, json!({}), 1.0));
        let times: &[(&str, f32)] = if pack == STEREO {
            &[("time_left", 0.1), ("time_right", 0.15)]
        } else {
            &[("time", 0.1)]
        };
        for (time, seconds) in times {
            set(&fx, &time.replace("time", "division"), "off");
            set(&fx, time, *seconds);
        }
        set(&fx, "tone", 20_000.0);
        let (left, right) = render(&mut fx, 12_000);
        let start = onset(&left, 0).expect("the dry pulse");
        let after = start + PULSE_LEN + 100;
        assert_eq!(onset(&left, after), Some(start + 4_800), "{pack}: left");
        let right_echo = onset(&right, after).expect("a right echo");
        let want = start + if pack == STEREO { 7_200 } else { 9_600 };
        let msg = format!("{pack}: right echo at {right_echo}, want {want}");
        assert!((want..=want + 2).contains(&right_echo), "{msg}");

        set(&fx, "feedback", 0.5);
        set(&fx, "bpm", 90.0);
        set(&fx, "glide", "slew");
        set(&fx, "hold", false);
        set(&fx, "dry", 0.0);
        set(&fx, "wet", 0.0);
        let (left, right) = render(&mut fx, SR as usize);
        let half = SR as usize / 2;
        let tail = peak(&left[half..]).max(peak(&right[half..]));
        assert!(tail <= EPS, "{pack}: dry and wet at 0 are silent, peak {tail}");
    }
}
