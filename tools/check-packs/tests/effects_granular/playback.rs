//! Every mode plays, and the output stays bounded.

use serde_json::{json, Value};

use super::rig::*;

#[test]
fn every_mode_plays_a_steady_sine_on_both_sides() {
    for mode in ["granular", "stretch", "looping_delay"] {
        let config = json!({ "dry": 0.0, "wet": 1.0, "reverb": 0.0, "mode": mode });
        let (l, r) = render_doc(&effect(&Source::sine(220.0, 0.5), config), 3 * SECOND);
        let window = 5 * SECOND / 2..3 * SECOND;
        for (side, x) in [("left", &l), ("right", &r)] {
            let level = rms(&x[window.clone()]);
            assert!(level > 0.05, "{mode} {side}: rms {level}");
        }
    }
}

/// Renders `seconds` of 0 dBFS noise and returns the loudest output sample,
/// asserting every sample is finite.
fn loudest(config: Value, seconds: usize) -> f32 {
    let mut fx = engine(&effect(&Source::noise(), config));
    let mut loudest = 0.0f32;
    for _ in 0..seconds {
        let (l, r) = render(&mut fx, SECOND);
        assert!(l.iter().chain(&r).all(|v| v.is_finite()), "finite");
        loudest = loudest.max(peak(&l)).max(peak(&r));
    }
    loudest
}

#[test]
fn dry_plus_wet_at_most_one_stays_within_unity_at_full_feedback() {
    for mode in ["granular", "stretch", "looping_delay"] {
        let config = json!({
            "mode": mode, "dry": 0.5, "wet": 0.5, "reverb": 0.0,
            "feedback": 1.0, "density": 1.0, "texture": 1.0, "spread": 1.0
        });
        let p = loudest(config, 20);
        assert!(p <= 1.0, "{mode}: peak {p}");
    }
}

#[test]
fn everything_at_its_top_stays_finite_and_does_not_run_away() {
    let config = json!({
        "dry": 1.0, "wet": 1.0, "reverb": 1.0, "feedback": 1.0,
        "reverb_size": 1.0, "reverb_decay": 1.0, "reverb_damping": 0.0,
        "density": 1.0, "texture": 1.0
    });
    let p = loudest(config, 30);
    assert!(p <= 16.0, "peak {p}");
}
