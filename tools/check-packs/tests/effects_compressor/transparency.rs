//! The dry paths the interface pins: below the threshold the compressor is
//! transparent with no added delay, and `mix` blends dry and compressed as
//! dry + mix × (compressed - dry), so `mix` 0 is the dry input.

use serde_json::json;

use super::rig::*;

/// Seeded white noise, a different seed on each side, within ±`level`.
fn noise_both(level: f64) -> Rig {
    Rig::new()
        .noise("nl", 51, level)
        .noise("nr", 52, level)
        .feed("nl", "white", "audio_left")
        .feed("nr", "white", "audio_right")
}

/// Asserts `wet` equals `dry` at every sample of `range`, within `rel` of
/// the dry peak.
fn assert_dry(what: &str, wet: &[f32], dry: &[f32], range: std::ops::Range<usize>, rel: f32) {
    let bound = rel * peak(&dry[range.clone()]);
    for n in range {
        let err = (wet[n] - dry[n]).abs();
        assert!(
            err <= bound,
            "{what}: sample {n} is {} where the dry input is {} (off {err}, bound {bound})",
            wet[n],
            dry[n]
        );
    }
}

/// "Below the threshold with `makeup` 0, the output equals the input to
/// within float rounding of the dry/wet sum (about 1e-6 relative). No delay
/// is added." Noise within ±0.05 (peak -26 dBFS, under the knee's lower
/// edge at -23 dBFS) on both sides, compared sample by sample with the dry
/// input from the first sample for 2 s. "About 1e-6 relative" is read as
/// 2e-6 of the signal's peak at every sample; a one-sample delay would
/// miss by the noise's own step, about 0.05.
#[test]
fn below_the_threshold_the_output_is_the_input_with_no_delay() {
    let rig = noise_both(0.05);
    let frames = 2 * SR as usize;
    let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, json!({}), frames);
    assert!(
        peak(&dry_l) > 0.04 && peak(&dry_l) <= 0.05,
        "the dry noise peaks at 0.05"
    );
    assert_dry("left", &left, &dry_l, 0..frames, 2e-6);
    assert_dry("right", &right, &dry_r, 0..frames, 2e-6);
}

/// "`mix` 0 gives the dry input, whatever the compression": full-scale
/// noise under heavy settings (threshold -40, ratio 20, knee 0, makeup
/// +12, fast attack) equals the dry input at every sample (within 1e-6 of
/// the peak) with `mix` 0, while the same settings at `mix` 1 change it
/// by more than 6 dB, so the compression was there to hide.
#[test]
fn mix_zero_gives_the_dry_input_whatever_the_compression() {
    let rig = noise_both(1.0);
    let frames = 2 * SR as usize;
    let heavy = |mix: f64| {
        json!({ "threshold": -40.0, "ratio": 20.0, "knee": 0.0, "makeup": 12.0,
                "attack": 0.001, "mix": mix })
    };
    let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, heavy(0.0), frames);
    assert_dry("left at mix 0", &left, &dry_l, 0..frames, 1e-6);
    assert_dry("right at mix 0", &right, &dry_r, 0..frames, 1e-6);

    let (left, _) = render_doc(&rig.effect(heavy(1.0)), frames);
    let g = gain(&left, &dry_l, SETTLE..frames);
    assert!(
        db(g.abs()) < -6.0,
        "at mix 1 the same settings compress (gain {g})"
    );
}

/// `mix` 0.5 is halfway: a steady 0.5 (c = -10.49 dB at the defaults)
/// comes out at 0.5 × (1 + 0.5 × (10^(c/20) - 1)), with c within the
/// curve's 0.11 dB.
#[test]
fn mix_blends_dry_and_compressed_by_its_amount() {
    let rig = Rig::new().constant("a", 0.5).feed_both("a", "audio");
    let frames = SETTLE + SR as usize / 2;
    let ((left, right), (dry_l, dry_r)) = wet_and_dry(&rig, json!({ "mix": 0.5 }), frames);
    let c = default_curve(0.5);
    let blend = |c: f64| 1.0 + 0.5 * (from_db(c) - 1.0);
    let (lo, hi) = (blend(c - CURVE_DB), blend(c + CURVE_DB));
    for (side, wet, dry) in [("left", &left, &dry_l), ("right", &right, &dry_r)] {
        let g = steady_gain(side, wet, dry, SETTLE..frames, 1e-4);
        assert!(
            (lo..=hi).contains(&g),
            "{side}: mix 0.5 gives ×{g}, want ×{} ({lo} to {hi})",
            blend(c)
        );
    }
}
