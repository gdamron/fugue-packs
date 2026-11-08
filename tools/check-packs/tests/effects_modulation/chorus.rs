//! `fugue.effects.chorus`: "Sweeping rate and depth over the full range in
//! 1 s: the largest step is no more than 1.5x the dry input's."
//!
//! Each side's two voices move opposite ways around `time` (12 ms), at most
//! `depth` x 4 ms, so at `rate` 10 Hz (the top of its range) and `depth` 1
//! a voice's delay moves 0.25 samples per sample. Cases that keep the
//! product of rate and depth lower use `dry` 0.4 and `wet` 0.3 (levels
//! summing to 1.0 over a side's two voices); the cases that take both to
//! their maximum together use `dry` 0.25 and `wet` 0.1 so the legitimate
//! steps stay inside the bound.

use super::rig::*;
use super::zipper::*;

/// The declared `rate` range, slowest to fastest and back, so a sweep
/// spans the whole range and no write lands beyond it.
const SLOW_FAST: (f32, f32) = (0.05, 10.0);
const FAST_SLOW: (f32, f32) = (10.0, 0.05);

fn sweep(
    name: &'static str,
    rate: (f32, f32),
    depth: (f32, f32),
    log_rate: bool,
    (dry, wet): (f32, f32),
) -> Sweep {
    Sweep {
        name,
        rate,
        depth,
        log_rate,
        dry,
        wet,
    }
}

const BALANCED: (f32, f32) = (0.4, 0.3);
const LIGHT_WET: (f32, f32) = (0.25, 0.1);

/// Rate across its range one way while depth crosses its range the other
/// (the delay never moves faster than 0.063 samples per sample), linear and
/// geometric in rate.
#[test]
fn sweeping_rate_up_and_depth_down_makes_no_step() {
    let cases = [
        sweep("rate up, depth down", SLOW_FAST, FULL_DOWN, false, BALANCED),
        sweep(
            "rate up log, depth down",
            SLOW_FAST,
            FULL_DOWN,
            true,
            BALANCED,
        ),
    ];
    for case in &cases {
        check_sweep(CHORUS, case, 0.5, serde_json::json!({}));
    }
}

#[test]
fn sweeping_rate_down_and_depth_up_makes_no_step() {
    let cases = [
        sweep("rate down, depth up", FAST_SLOW, FULL, false, BALANCED),
        sweep("rate down log, depth up", FAST_SLOW, FULL, true, BALANCED),
    ];
    for case in &cases {
        check_sweep(CHORUS, case, 0.5, serde_json::json!({}));
    }
}

/// Rate and depth both at their maximum together at one end of the sweep.
#[test]
fn sweeping_rate_and_depth_together_to_their_maxima_makes_no_step() {
    let cases = [
        sweep("both up", SLOW_FAST, FULL, false, LIGHT_WET),
        sweep("both down", FAST_SLOW, FULL_DOWN, false, LIGHT_WET),
        sweep("both up log", SLOW_FAST, FULL, true, LIGHT_WET),
    ];
    for case in &cases {
        check_sweep(CHORUS, case, 0.5, serde_json::json!({}));
    }
}

/// The sweep also holds with `spread` at 0 and at 1 (a whole cycle apart
/// is the same as together) and `time` short (6 ms, where depth 1 swings a voice down to 2 ms) and
/// at its long end.
#[test]
fn sweeping_holds_at_the_ends_of_spread_and_time() {
    let case = sweep("rate up, depth down", SLOW_FAST, FULL_DOWN, false, BALANCED);
    for config in [
        serde_json::json!({ "spread": 0.0 }),
        serde_json::json!({ "spread": 1.0 }),
        serde_json::json!({ "time": 0.006 }),
        serde_json::json!({ "time": 0.04 }),
    ] {
        check_sweep(CHORUS, &case, 0.5, config);
    }
}
