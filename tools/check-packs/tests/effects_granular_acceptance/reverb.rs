//! The reverb send: fed by the granular output (before `wet`), heard at
//! `reverb` with no dry of its own.

use serde_json::{json, Value};

use super::rig::*;

const BURST: usize = SR as usize / 10;

/// Plays 100 ms of a 2 kHz sine at 0.5 into both inputs, then silence for
/// 1.5 s, at `dry` 0 and `position` 0 with `config` on top. Returns
/// (burst, tail) per side.
fn burst_then_silence(config: Value) -> [(Vec<f32>, Vec<f32>); 2] {
    let config = with(&json!({ "dry": 0.0, "position": 0.0 }), config);
    let mut fx = engine(&Feed::sine(2000.0, 0.5).effect(config, 1.0));
    let burst = render(&mut fx, BURST);
    set_on(&fx, "sine_level", "level", 0.0f32);
    [burst, render(&mut fx, 3 * SR as usize / 2)]
}

/// "At `reverb` 0 the output falls silent (|x| <= 1e-4) within 100 ms after
/// the burst ends"; the burst itself is heard.
#[test]
fn without_reverb_the_output_falls_silent_within_100_ms() {
    let [burst, tail] = burst_then_silence(json!({ "wet": 1.0, "reverb": 0.0 }));
    assert!(
        peak(&burst.0) > 0.05 && peak(&burst.1) > 0.05,
        "the burst is heard"
    );
    for (side, x) in [("left", &tail.0), ("right", &tail.1)] {
        let loudest = peak(&x[BURST..]);
        assert!(loudest <= 1e-4, "{side}: {loudest} after 100 ms of silence");
    }
}

/// "at `reverb` 0.5 (default size/decay) the output RMS from 0.5 s to 1 s
/// after the burst ends is above 1e-2".
#[test]
fn with_reverb_a_tail_rings_on_after_the_burst() {
    let [_, tail] = burst_then_silence(json!({ "wet": 1.0, "reverb": 0.5 }));
    for (side, x) in [("left", &tail.0), ("right", &tail.1)] {
        let level = rms(&x[SR as usize / 2..SR as usize]);
        assert!(level > 1e-2, "{side}: RMS {level} from 0.5 s to 1 s");
    }
}

/// "with `wet` 0 the reverb return alone is heard (non-silent) while at
/// `wet` 0, `reverb` 0 it is silent": the reverb is fed before `wet`.
#[test]
fn the_reverb_is_fed_before_wet_so_it_is_heard_with_wet_at_zero() {
    let [_, tail] = burst_then_silence(json!({ "wet": 0.0, "reverb": 0.5 }));
    for (side, x) in [("left", &tail.0), ("right", &tail.1)] {
        let level = rms(&x[SR as usize / 2..SR as usize]);
        assert!(level > 1e-3, "{side}: only the return, RMS {level}");
    }

    let [burst, tail] = burst_then_silence(json!({ "wet": 0.0, "reverb": 0.0 }));
    for (side, x) in [
        ("left", [&burst.0, &tail.0]),
        ("right", [&burst.1, &tail.1]),
    ] {
        let loudest = peak(x[0]).max(peak(x[1]));
        assert!(loudest <= 1e-4, "{side}: wet 0, reverb 0 leaks {loudest}");
    }
}
