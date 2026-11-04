//! The dry path, the first echo's timing and level, and the repeats.

use serde_json::{json, Value};

use super::rig::*;
use super::tape::*;

/// "Before the first echo the output is exactly the dry input", with
/// `audio_left` heard on the left only and `audio_right` on the right only:
/// noise is bit-identical to the same noise wired straight to the dac. At
/// the defaults `wow` (±1 ms) and `flutter` (±0.08 ms) may bring the first
/// echo up to ~52 samples early, so the comparison stops 64 short of 0.3 s;
/// with both at 0 it runs to the full `time`. The echo is then heard.
#[test]
fn output_before_the_first_echo_is_exactly_the_dry_input() {
    let frames = TIME + 2_000;
    let (dry_left, dry_right) = render_doc(&noise_both().dry(), frames);
    assert!(peak(&dry_left) > 0.5, "the reference noise is heard");
    for (what, config, until) in [
        ("defaults", json!({}), TIME - 64),
        ("no wobble", still(json!({})), TIME),
    ] {
        let (left, right) = render_doc(&noise_both().effect(TAPE, config, 1.0), frames);
        assert_same(&format!("{what}: left"), &left, &dry_left, 0..until);
        assert_same(&format!("{what}: right"), &right, &dry_right, 0..until);
        let echo = TIME + LATENCY + 200;
        let differs = (until..echo).any(|n| left[n].to_bits() != dry_left[n].to_bits());
        assert!(differs, "{what}: an echo is heard by sample {echo}");
    }
}

/// Renders a tone burst (0.05 peak) into both inputs and returns the dry
/// burst's onset and the first echo's, checking the echo is the same on
/// both sides and, with `feedback` 0, the only one: once it ends, what
/// follows is under 1% of its peak.
fn first_echo(what: &str, doc: &Value, frames: usize) -> (usize, usize) {
    let (left, right) = render_doc(doc, frames);
    let diff = (left.iter().zip(&right)).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
    assert!(diff <= 1e-6, "{what}: left and right differ by {diff}");
    let start = onset(&left, 0).expect("the dry burst");
    let echo = onset(&left, start + BURST_LEN + 100).expect("a first echo");
    let after = echo + BURST_LEN + 500;
    let loudest = peak(&left[echo..after]);
    let rest = peak(&left[after.min(frames)..]);
    assert!(
        rest <= 0.01 * loudest,
        "{what}: one echo only (peak {loudest}), then {rest} after {after}"
    );
    (start, echo)
}

/// "with `wow` and `flutter` at 0, an impulse given to both inputs gives one
/// first echo, the same on both sides ... `time` x sample rate later, plus up
/// to 32 frames ... At 120 bpm with `division` `quarter_note`, that is
/// 24,000 samples (+ up to 32)". The probe is a tone burst, which carries no
/// DC ("Probe echoes with single-sample impulses or tone bursts"). The `bpm`
/// input sets the tempo for the head (the control at 60 is ignored): at 100
/// bpm a 1/4 is 28,800 samples.
#[test]
fn the_first_echo_comes_after_time_plus_at_most_32_frames() {
    let base = still(json!({ "feedback": 0.0, "tone": 18_000.0 }));
    let with = |extra: Value| {
        let mut config = base.clone();
        for (k, v) in extra.as_object().expect("object") {
            config[k] = v.clone();
        }
        config
    };
    let tempo = Rig::new().tempo("tempo", 100.0).feed("tempo", "bpm", "bpm");
    let cases = [
        ("time 0.3", Rig::new(), with(json!({})), TIME),
        ("time 0.1", Rig::new(), with(json!({ "time": 0.1 })), 4_800),
        (
            "quarter note at 120 bpm",
            Rig::new(),
            with(json!({ "division": "quarter_note" })),
            24_000,
        ),
        (
            "quarter note at a patched 100 bpm",
            tempo,
            with(json!({ "division": "quarter_note", "bpm": 60.0 })),
            28_800,
        ),
    ];
    for (what, rig, config, delay) in cases {
        let doc = burst_doc(&rig, config, 0.05);
        let (start, echo) = first_echo(what, &doc, delay + 4_000);
        let want = start + delay..=start + delay + LATENCY;
        assert!(
            want.contains(&echo),
            "{what}: echo at {echo}, want {want:?}"
        );
    }
}

/// "the first echo of a quiet, low tone (e.g. 200 Hz at 0.01 amplitude in
/// both inputs) is within ±1 dB of the input x `wet`": the echo's RMS, over
/// 50 cycles clear of its onset, on each side, at the default `wet` (0.5)
/// and at 1.5. `dry` 0 and `feedback` 0 leave the first echo alone.
#[test]
fn a_quiet_low_tone_echoes_at_unity_times_wet() {
    for wet in [0.5f32, 1.5] {
        let config = json!({ "dry": 0.0, "feedback": 0.0, "wet": wet });
        let (left, right) = render_doc(&sine_doc(config, 200.0, 0.01, 1.0), 2 * TIME);
        let want = 0.01 / 2f32.sqrt() * wet;
        for (side, x) in [("left", &left), ("right", &right)] {
            let heard = rms(&x[TIME + 2_400..2 * TIME]);
            let db = 20.0 * (heard / want).log10();
            assert!(
                db.abs() <= 1.0,
                "wet {wet} {side}: echo RMS {heard}, want {want} (±1 dB), off by {db} dB"
            );
        }
    }
}

/// "at `feedback` 0.4 and `tone` high (e.g. 18000), the repeats of a quiet
/// impulse fall by about x 0.4 each, spaced `time` plus up to 33 frames".
/// A 0.02 tone burst (no DC); the first echo at its level x `wet` (0.5).
/// Assumptions: "about" is within 12% (about 1 dB), and the onset reading
/// may land up to 2 samples early on a quieter repeat.
#[test]
fn repeats_of_a_quiet_impulse_decay_by_feedback() {
    let config = still(json!({ "feedback": 0.4, "tone": 18_000.0 }));
    let (left, right) = render_doc(&burst_doc(&Rig::new(), config, 0.02), 4 * TIME + 4_000);
    let diff = (left.iter().zip(&right)).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
    assert!(
        diff <= 1e-6,
        "the echoes are the same on both sides, off by {diff}"
    );
    let start = onset(&left, 0).expect("the dry burst");
    let first = onset(&left, start + BURST_LEN + 100).expect("a first echo");
    assert!(
        (start + TIME..=start + TIME + LATENCY).contains(&first),
        "first echo at {first}, burst at {start}"
    );
    let dry = burst_level(&left, start);
    assert_near(
        "first echo level",
        burst_level(&left, first),
        dry * 0.5,
        0.12,
    );
    let mut prev = first;
    for k in 2..=4 {
        let at = onset(&left, prev + BURST_LEN + 1_000).unwrap_or_else(|| panic!("echo {k}"));
        let gap = at - prev;
        assert!(
            (TIME - 2..=TIME + LATENCY + 1).contains(&gap),
            "echo {k} is {gap} samples after echo {}",
            k - 1
        );
        let (level, before) = (burst_level(&left, at), burst_level(&left, prev));
        assert_near(&format!("echo {k} level"), level, before * 0.4, 0.12);
        prev = at;
    }
}
