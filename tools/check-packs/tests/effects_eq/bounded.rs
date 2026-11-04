//! "Modulated parameters must not zipper or go unstable": fast live moves of
//! every band's frequency (and the bells' q and boost) at the top q stay
//! finite and within ±4 for input within ±0.25.
//!
//! The outputs reach the dac through a 0.125 tap, so nothing past the pack
//! could hide an overshoot; levels below are scaled back by 8.

use serde_json::json;

use super::rig::*;

/// Frames between moves: with blocks of 24 frames, a frequency flipping
/// between its extremes every block is a 1 kHz square wave of requests.
const STEP: usize = 24;

#[test]
fn fast_live_moves_at_max_q_stay_finite_and_bounded() {
    let config = json!({
        "low_cut_bypass": false, "low_cut_slope": "24db",
        "high_cut_bypass": false, "high_cut_slope": "24db",
        "bell_boost.0": 12.0, "bell_boost.1": 12.0, "bell_boost.2": 12.0,
        "bell_q.0": 18.0, "bell_q.1": 18.0, "bell_q.2": 18.0,
        "low_shelf_q": 18.0, "high_shelf_q": 18.0
    });
    let mut fx = engine(&Feed::noise_pair(0.25).effect(config, 0.125));
    fx.set_block_size(STEP);
    let mut loudest = 0.0f32;
    for step in 0..2 * SR as usize / STEP {
        let up = step % 2 == 0;
        let (lo, hi) = if up {
            (20.0f32, 16_000.0f32)
        } else {
            (16_000.0, 20.0)
        };
        set(&fx, "bell_frequency.0", lo);
        set(&fx, "bell_frequency.1", hi);
        set(&fx, "bell_frequency.2", if step % 4 < 2 { lo } else { hi });
        set(&fx, "low_cut_frequency", if up { 20.0f32 } else { 2_000.0 });
        set(
            &fx,
            "high_cut_frequency",
            if up { 16_000.0f32 } else { 2_000.0 },
        );
        set(&fx, "low_shelf_frequency", lo);
        set(&fx, "high_shelf_frequency", hi);
        if step % 8 == 0 {
            let q = if step % 16 == 0 { 18.0f32 } else { 0.1 };
            set(&fx, "bell_q.1", q);
            set(
                &fx,
                "bell_boost.2",
                if step % 16 == 0 { -12.0f32 } else { 12.0 },
            );
        }
        let (left, right) = render(&mut fx, STEP);
        for (side, x) in [("left", &left), ("right", &right)] {
            assert!(
                x.iter().all(|v| v.is_finite()),
                "{side}: not finite at move {step}"
            );
            let p = peak(x) * 8.0;
            assert!(p <= 4.0, "{side}: {p} at move {step}");
            loudest = loudest.max(p);
        }
    }
    assert!(loudest > 0.01, "the noise is heard ({loudest})");
}
