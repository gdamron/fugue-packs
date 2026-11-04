//! Stereo linking: "The detector is stereo-linked, following the louder
//! side at each sample, and one gain change applies to both sides."

use serde_json::json;

use super::rig::*;

/// A steady 0.5 (-6.02 dBFS, c = -10.49 dB) on one side and a quiet 0.05
/// sine (below the threshold on its own) on the other: both sides get the
/// loud side's gain, c(-6.02) within the curve's 0.11 dB, and the two
/// gains agree within 1e-4 relative (one gain). Checked both ways round.
#[test]
fn the_louder_side_sets_one_gain_for_both_sides() {
    let frames = SETTLE + SR as usize / 2;
    let want = default_curve(0.5);
    for (loud, quiet) in [("audio_left", "audio_right"), ("audio_right", "audio_left")] {
        let rig = Rig::new()
            .constant("loud", 0.5)
            .osc("quiet", "sine", 440.0, 0.05)
            .feed("loud", "audio", loud)
            .feed("quiet", "audio", quiet);
        let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, json!({}), frames);
        let gl = steady_gain(
            &format!("loud {loud}: left"),
            &left,
            &dry_l,
            SETTLE..frames,
            1e-3,
        );
        let gr = steady_gain(
            &format!("loud {loud}: right"),
            &right,
            &dry_r,
            SETTLE..frames,
            1e-3,
        );
        assert_db(&format!("loud {loud}: left"), db(gl), want, CURVE_DB);
        assert_db(&format!("loud {loud}: right"), db(gr), want, CURVE_DB);
        assert!(
            (gl / gr - 1.0).abs() <= 1e-4,
            "loud {loud}: one gain for both sides, left ×{gl}, right ×{gr}"
        );
    }
}

/// "following the louder side at each sample": in-phase 100 Hz squares,
/// 0.5 on the left and 0.25 on the right, are compressed by c of the
/// louder, c(-6.02 dBFS) = -10.49 dB, on both sides. A detector on their
/// sum (0.75, c = -13.1 dB) or their mean (0.375, c = -8.6 dB) misses by
/// well over the curve's 0.11 dB.
#[test]
fn the_detector_follows_the_louder_side_not_their_sum() {
    let frames = SETTLE + SR as usize / 2;
    let rig = Rig::new()
        .osc("l", "square", 100.0, 0.5)
        .osc("r", "square", 100.0, 0.25)
        .feed("l", "audio", "audio_left")
        .feed("r", "audio", "audio_right");
    let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, json!({}), frames);
    let want = default_curve(0.5);
    let gl = steady_gain("left", &left, &dry_l, SETTLE..frames, 1e-3);
    let gr = steady_gain("right", &right, &dry_r, SETTLE..frames, 1e-3);
    assert_db("left", db(gl), want, CURVE_DB);
    assert_db("right", db(gr), want, CURVE_DB);
}
