//! "The composed compressor applies its +6 dB makeup."
//!
//! Interface: "below the threshold (c = 0) with `makeup` at +6 dB, the
//! output is the input times about 2: 10^(6/20) ≈ 1.995. Above it, the
//! output is the compressed level raised by 6 dB."

use serde_json::json;

use super::rig::*;

/// 10^(6/20): +6 dB as a linear gain, about 1.9953 (not exactly 2).
fn six_db() -> f64 {
    from_db(6.0)
}

/// Relative tolerance on a makeup gain: the issue pins the gain atom's ×2
/// point to 1e-4 ("+6.02 dB is ×2 ± 1e-4"), and dB to linear is accurate to
/// 1e-5, so 1e-4 relative holds for a +6 dB makeup.
const MAKEUP_REL: f64 = 1e-4;

/// Below the threshold, +6 dB makeup multiplies the input by 1.9953 on each
/// side: a 0.05 sine (-26 dBFS, under the knee's lower edge at -23 dBFS) on
/// the left and a 0.04 one on the right, measured after the detector
/// settles. The output is that gain times the input at every sample (no
/// delay, no other change), within 1e-4 of the peak.
#[test]
fn below_the_threshold_makeup_six_db_multiplies_by_1_9953() {
    let rig = Rig::new()
        .osc("l", "sine", 220.0, 0.05)
        .osc("r", "sine", 330.0, 0.04)
        .feed("l", "audio", "audio_left")
        .feed("r", "audio", "audio_right");
    let frames = SETTLE + SR as usize / 2;
    let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, json!({ "makeup": 6.0 }), frames);
    for (side, wet, dry) in [("left", &left, &dry_l), ("right", &right, &dry_r)] {
        let what = format!("{side}, makeup +6 dB below the threshold");
        let g = steady_gain(&what, wet, dry, SETTLE..frames, MAKEUP_REL);
        let err = (g / six_db() - 1.0).abs();
        assert!(
            err <= MAKEUP_REL,
            "{what}: gain {g}, want {} within {MAKEUP_REL} (off {err})",
            six_db()
        );
    }
}

/// Above the threshold, the makeup raises the compressed level by 6 dB: a
/// steady 0.5 (-6.02 dBFS) on both sides comes out 1.9953 times louder with
/// `makeup` +6 than with 0 (within 1e-4), and at c(-6.02) + 6 dB in all,
/// about -4.49 dB (within the curve's 0.11 dB).
#[test]
fn above_the_threshold_makeup_raises_the_compressed_level_six_db() {
    let rig = Rig::new().constant("a", 0.5).feed_both("a", "audio");
    let frames = SETTLE + SR as usize / 2;
    let measure = |makeup: f64| {
        let config = json!({ "makeup": makeup });
        let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, config, frames);
        let what = format!("makeup {makeup} on a steady 0.5");
        let gl = steady_gain(
            &format!("{what}, left"),
            &left,
            &dry_l,
            SETTLE..frames,
            1e-4,
        );
        let gr = steady_gain(
            &format!("{what}, right"),
            &right,
            &dry_r,
            SETTLE..frames,
            1e-4,
        );
        (gl, gr)
    };
    let (plain_l, plain_r) = measure(0.0);
    let (made_l, made_r) = measure(6.0);
    let want_db = default_curve(0.5) + 6.0;
    for (side, plain, made) in [("left", plain_l, made_l), ("right", plain_r, made_r)] {
        let ratio = made / plain;
        let err = (ratio / six_db() - 1.0).abs();
        assert!(
            err <= MAKEUP_REL,
            "{side}: makeup +6 over 0 is ×{ratio}, want ×{} within {MAKEUP_REL}",
            six_db()
        );
        assert_db(&format!("{side}, makeup +6"), db(made), want_db, CURVE_DB);
    }
}
