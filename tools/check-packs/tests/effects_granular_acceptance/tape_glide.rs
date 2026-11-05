//! Tape glide through the pack: `glide` and `inertia` reach the looping
//! delay, a moved `position` bends the pitch while the delay slides, and
//! the defaults change nothing.

use fugue::ControlValue;
use serde_json::{json, Value};

use super::rig::*;

/// The steady tone, and the pitch-measurement window (100 cycles).
const TONE: f64 = 997.0;
const WINDOW: usize = SR as usize / 10;
/// Time on the old delay before the move, and time rendered after it.
const BEFORE: usize = 3 * SR as usize;
const AFTER: usize = 12 * SR as usize;

/// The pitch of `x` as a multiple of the tone: positive-going zero
/// crossings, interpolated, counted from the first to the last in `x`.
fn speed(x: &[f32]) -> f64 {
    let mut crossings = Vec::new();
    for (n, pair) in x.windows(2).enumerate() {
        if pair[0] < 0.0 && pair[1] >= 0.0 {
            crossings.push(n as f64 + f64::from(-pair[0]) / f64::from(pair[1] - pair[0]));
        }
    }
    assert!(
        crossings.len() > 10,
        "a tone is heard ({} cycles)",
        crossings.len()
    );
    let cycles = (crossings.len() - 1) as f64;
    cycles * f64::from(SR) / (crossings[crossings.len() - 1] - crossings[0]) / TONE
}

/// The speed in each 100 ms window.
fn speeds(x: &[f32]) -> Vec<f64> {
    x.chunks_exact(WINDOW).map(speed).collect()
}

/// The looping delay at `position` 0.25 with a steady tone in, only the
/// delay heard, built with `config` on top of that.
fn looping_delay(config: Value) -> fugue::RenderEngine {
    let base = json!({
        "mode": "looping_delay", "density": 0.0, "dry": 0.0, "wet": 1.0,
        "reverb": 0.0, "position": 0.25
    });
    engine(&Feed::sine(TONE, 0.5).effect(with(&base, config), 1.0))
}

/// Settles on the old delay, moves `position` to 0.3 and returns the speed
/// in each 100 ms window of the output after the move, left then right.
fn move_speeds(fx: &mut fugue::RenderEngine) -> (Vec<f64>, Vec<f64>) {
    render(fx, BEFORE);
    set(fx, "position", 0.3f32);
    let (left, right) = render(fx, AFTER);
    (speeds(&left), speeds(&right))
}

/// The second after which the speed stays within 1.5% of 1.
fn settled_at(speeds: &[f64]) -> f64 {
    let last = speeds.iter().rposition(|s| (s - 1.0).abs() > 0.015);
    last.map_or(0.0, |n| (n + 1) as f64 * WINDOW as f64 / f64::from(SR))
}

/// "a live move of `position` from 0.25 to 0.3 with `glide` 2 bends the
/// tone's pitch flat while the delay grows (its read speed dips below
/// 0.97x, never below 0.5x), and then returns to pitch (... allow about 8 s
/// after the move at `inertia` 1)". Returns the slowest speed and the
/// settling time, the worse of the two sides.
fn assert_bends_flat_and_returns(what: &str, config: Value) -> (f64, f64) {
    let mut fx = looping_delay(config);
    let (left, right) = move_speeds(&mut fx);
    let mut worst = (f64::MAX, 0.0f64);
    for (side, speeds) in [("left", &left), ("right", &right)] {
        let slowest = speeds.iter().copied().fold(f64::MAX, f64::min);
        assert!(
            (0.49..0.97).contains(&slowest),
            "{what} {side}: slowest speed {slowest}"
        );
        let settled = settled_at(speeds);
        assert!(
            settled <= 8.5,
            "{what} {side}: back at pitch after {settled} s"
        );
        let end = &speeds[speeds.len() - 10..];
        assert!(
            end.iter().all(|s| (s - 1.0).abs() <= 0.015),
            "{what} {side}: speeds {end:?} in the last second"
        );
        worst = (worst.0.min(slowest), worst.1.max(settled));
    }
    worst
}

#[test]
fn a_position_move_with_glide_bends_the_pitch_flat_then_returns() {
    assert_bends_flat_and_returns("glide 2", json!({ "glide": 2.0 }));
}

#[test]
fn the_pitch_bends_and_returns_at_every_inertia() {
    for inertia in [0.0, 1.0] {
        let config = json!({ "glide": 2.0, "inertia": inertia });
        assert_bends_flat_and_returns(&format!("inertia {inertia}"), config);
    }
}

#[test]
fn live_writes_of_glide_and_inertia_do_the_same_as_config() {
    let mut fx = looping_delay(json!({}));
    set(&fx, "glide", 2.0f32);
    set(&fx, "inertia", 1.0f32);
    let (left, _) = move_speeds(&mut fx);
    let slowest = left.iter().copied().fold(f64::MAX, f64::min);
    assert!((0.49..0.97).contains(&slowest), "slowest speed {slowest}");
    assert!(settled_at(&left) <= 8.5, "settles within about 8 s");
}

/// "heavier tape settles slower" (and, per the module, bends more gently).
#[test]
fn a_heavier_tape_bends_less_and_settles_slower() {
    let light = assert_bends_flat_and_returns("inertia 0", json!({ "glide": 2.0, "inertia": 0.0 }));
    let heavy = assert_bends_flat_and_returns("inertia 1", json!({ "glide": 2.0, "inertia": 1.0 }));
    assert!(
        heavy.0 > light.0,
        "slowest speed {} heavy, {} light",
        heavy.0,
        light.0
    );
    assert!(
        heavy.1 > light.1,
        "settles in {} s heavy, {} s light",
        heavy.1,
        light.1
    );
}

/// "with `glide` 0 the same move keeps the pitch (it crossfades)", whatever
/// the `inertia`.
#[test]
fn without_glide_the_same_move_keeps_the_pitch() {
    for inertia in [0.0, 0.5, 1.0] {
        let mut fx = looping_delay(json!({ "glide": 0.0, "inertia": inertia }));
        let (left, right) = move_speeds(&mut fx);
        for (side, speeds) in [("left", &left), ("right", &right)] {
            let worst = (speeds.iter()).fold(0.0f64, |m, s| m.max((s - 1.0).abs()));
            assert!(
                worst <= 0.02,
                "inertia {inertia} {side}: speed off by {worst}"
            );
        }
    }
}

/// "Giving `glide` 0 and `inertia` 0.5 in config is bit-identical to leaving
/// them out, in every mode", through live moves of `position`.
#[test]
fn glide_zero_and_inertia_half_in_config_change_nothing_in_every_mode() {
    let feed = Feed::noise_pair(0.5);
    for mode in ["granular", "stretch", "looping_delay"] {
        let bare = json!({ "mode": mode });
        let given = json!({ "mode": mode, "glide": 0.0, "inertia": 0.5 });
        let mut plain = engine(&feed.effect(bare, 1.0));
        let mut explicit = engine(&feed.effect(given, 1.0));
        for position in [0.5f32, 0.3, 0.7, 0.2] {
            let (a, b) = (
                render(&mut plain, SR as usize),
                render(&mut explicit, SR as usize),
            );
            let bits = |x: &[f32]| x.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&a.0), bits(&b.0), "{mode} left before {position}");
            assert_eq!(bits(&a.1), bits(&b.1), "{mode} right before {position}");
            assert!(peak(&a.0) > 0.01, "{mode} is heard");
            set(&plain, "position", position);
            set(&explicit, "position", position);
        }
    }
}

/// "Live writes of `glide` and `inertia` are accepted by name": across
/// their ranges, read back, and the pack keeps rendering finite audio.
#[test]
fn glide_and_inertia_accept_live_writes_by_name() {
    let mut fx = looping_delay(json!({}));
    for (glide, inertia) in [
        (0.0f32, 0.0f32),
        (0.5, 0.25),
        (1.0, 0.5),
        (2.0, 1.0),
        (0.0, 0.5),
    ] {
        set(&fx, "glide", glide);
        set(&fx, "inertia", inertia);
        let (left, right) = render(&mut fx, SR as usize / 4);
        assert!(left.iter().chain(&right).all(|v| v.is_finite()));
        for (key, want) in [("glide", glide), ("inertia", inertia)] {
            match fx.get_control("fx", key) {
                Ok(ControlValue::Number(got)) => {
                    assert!((got - want).abs() <= 1e-6, "{key}: {got}, want {want}")
                }
                other => panic!("{key} reads as a number, got {other:?}"),
            }
        }
    }
}
