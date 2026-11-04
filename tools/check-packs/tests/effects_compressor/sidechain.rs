//! "A sidechained kick ducks the input by the curve's amount."
//!
//! Interface: "with `sidechain_mix` 1, a kick (or any key) on `sidechain`
//! sets x. While it plays, the audio is pulled down by c(x_kick), whatever
//! the audio's own level; when it stops, the audio recovers with
//! `release`." With the defaults (threshold -20, ratio 4, knee 6) the
//! curve's amount is c(x) at the key's settled level, which for a steady
//! key is its peak.

use serde_json::json;

use super::rig::*;

/// Keys: (description, rig adding module `k`, settled peak).
fn keys() -> Vec<(&'static str, Rig, f64)> {
    vec![
        ("a steady 1.0 key", Rig::new().constant("k", 1.0), 1.0),
        (
            "a sustained 60 Hz square kick at 0.8",
            Rig::new().osc("k", "square", 60.0, 0.8),
            0.8,
        ),
    ]
}

/// Audio: (description, peak) of a 440 Hz sine on both sides; 0.05 is
/// below the threshold on its own, 0.3 (-10.5 dBFS) above it.
const AUDIO: [f64; 2] = [0.05, 0.3];

fn with_audio(key: &Rig, amp: f64) -> Rig {
    let audio = Rig::new()
        .osc("a", "sine", 440.0, amp)
        .feed_both("a", "audio");
    key.with(&audio).feed("k", "audio", "sidechain")
}

/// With `sidechain_mix` 1, each key ducks the audio by c(x_key) on both
/// sides (-15 dB for 1.0, -13.55 dB for 0.8), within the curve's 0.11 dB,
/// whatever the audio's own level, and holds that gain steadily.
#[test]
fn a_sidechained_kick_ducks_the_input_by_the_curves_amount() {
    let frames = SETTLE + SR as usize / 2;
    for (key_name, key, key_peak) in keys() {
        let want = default_curve(key_peak);
        for amp in AUDIO {
            let rig = with_audio(&key, amp);
            let config = json!({ "sidechain_mix": 1.0 });
            let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, config, frames);
            for (side, wet, dry) in [("left", &left, &dry_l), ("right", &right, &dry_r)] {
                let what = format!("{key_name}, audio at {amp}, {side}");
                let g = steady_gain(&what, wet, dry, SETTLE..frames, 1e-3);
                assert_db(&what, db(g), want, CURVE_DB);
            }
        }
    }
}

/// With `sidechain_mix` at its default 0, the same key does not duck: audio
/// below the threshold passes at unity (within 1e-4) with the key playing.
#[test]
fn with_sidechain_mix_zero_the_key_is_ignored() {
    let frames = SETTLE + SR as usize / 2;
    for (key_name, key, _) in keys() {
        let rig = with_audio(&key, 0.05);
        let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, json!({}), frames);
        for (side, wet, dry) in [("left", &left, &dry_l), ("right", &right, &dry_r)] {
            let what = format!("{key_name} at sidechain_mix 0, {side}");
            let g = steady_gain(&what, wet, dry, SETTLE..frames, 1e-4);
            assert!((g - 1.0).abs() <= 1e-4, "{what}: gain {g}, want 1");
        }
    }
}

/// When the kick stops, the audio recovers with `release` (default 0.1 s).
/// The key is 1.0 for the first second of every 3 s; the audio a steady
/// 0.05 (below the threshold) so the gain reads at every sample.
///
/// - While the key plays: c(0 dBFS) = -15 dB.
/// - One release time after it stops, the detector has fallen to e^-1 of
///   1.0 (the follower pins this point to ±10% in time, so between
///   e^(-1/0.9) and e^(-1/1.1)): the gain lies between c at those levels,
///   widened by the curve's 0.11 dB (about -9.2 to -7.6 dB).
/// - The gain rises monotonically (to within 1e-5) through the recovery.
/// - One second (10 release times) after it stops the detector is below
///   -79 dBFS, under the knee, so the audio is back at unity (1e-4).
#[test]
fn when_the_kick_stops_the_audio_recovers_with_release() {
    let stop = SR as usize; // 1 s of a 3 s beat
    let tau = SR as usize / 10; // release 0.1 s
    let rig = Rig::new()
        .gated("k", 1.0, 1.0)
        .constant("a", 0.05)
        .feed_both("a", "audio")
        .feed("k", "audio", "sidechain");
    let frames = stop + SR as usize + 1_000;
    let config = json!({ "sidechain_mix": 1.0 });
    let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, config, frames);
    for (side, wet, dry) in [("left", &left, &dry_l), ("right", &right, &dry_r)] {
        let g = |n: usize| f64::from(wet[n]) / f64::from(dry[n]);
        let ducked = steady_gain(side, wet, dry, SETTLE..stop - 100, 1e-3);
        assert_db(
            &format!("{side}, while the kick plays"),
            db(ducked),
            default_curve(1.0),
            CURVE_DB,
        );

        // A faster release (0.9τ) leaves a lower level, so less reduction.
        let fast = curve(db((-1.0f64 / 0.9).exp()), T, R, W);
        let slow = curve(db((-1.0f64 / 1.1).exp()), T, R, W);
        let (low, high) = (slow.min(fast) - CURVE_DB, slow.max(fast) + CURVE_DB);
        let at = db(g(stop + tau));
        assert!(
            (low..=high).contains(&at),
            "{side}: one release time after the kick stops the gain is {at:.3} dB, want {low:.3} to {high:.3}"
        );

        for n in stop + 2..stop + SR as usize {
            assert!(
                g(n) >= g(n - 1) - 1e-5,
                "{side}: the recovery falls back at sample {n}: {} after {}",
                g(n),
                g(n - 1)
            );
        }
        let back = g(stop + SR as usize);
        assert!(
            (back - 1.0).abs() <= 1e-4,
            "{side}: 1 s after the kick stops the gain is {back}, want 1"
        );
    }
}
