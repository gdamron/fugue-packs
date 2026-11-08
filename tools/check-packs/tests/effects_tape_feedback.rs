//! The tape delay's `feedback` tops out at 1, where the loop holds a quiet
//! repeat at its level instead of letting it creep up into the saturation:
//! with the tone open, quiet bursts in the low band repeat for 30 s within
//! 1 dB of their first echo, and no burst, quiet or loud, at the default
//! tone or open, rises by more than 1 dB. Anything above 1 sounds as 1.
//!
//! The echo alone is heard (`dry` 0, `wet` 1), with wow and flutter at 0 so
//! every repeat lands a whole period apart. Each repeat is read as the RMS
//! of one period, against the first.

#[allow(dead_code)]
#[path = "effects_delay/rig.rs"]
mod rig;
#[allow(dead_code)]
#[path = "effects_tape_delay/tape.rs"]
mod tape;

use fugue::ControlValue;
use serde_json::json;

use rig::{engine, render, Rig, SR};
use tape::{burst_doc, rms, still, TIME};

/// A burst quiet enough that the saturation is linear.
const QUIET: f64 = 0.02;
/// A burst loud enough to reach the saturation.
const LOUD: f64 = 0.3;
/// The top of the `tone` range: the loop as open as it goes.
const OPEN: f64 = 20_000.0;
const DEFAULT_TONE: f64 = 3_500.0;
/// Where the loop gains most: its small-signal peak sits near 800 Hz.
const PEAK_HZ: f64 = 800.0;

/// The RMS of each of `count` repeat periods after one burst of `hz` at
/// `amp` with `feedback` 1 and `tone`. The periods start half a period in,
/// between the burst and its first echo; the burst is silenced then, so it
/// comes only once.
fn repeats(hz: f64, amp: f64, tone: f64, count: usize) -> Vec<f32> {
    let config = still(json!({ "dry": 0.0, "wet": 1.0, "feedback": 1.0, "tone": tone }));
    let mut doc = burst_doc(&Rig::new(), config, amp);
    let modules = doc["modules"].as_array_mut().expect("modules");
    let osc = (modules.iter_mut().find(|m| m["id"] == "burst_osc")).expect("a burst oscillator");
    osc["config"]["frequency"] = json!(hz);
    let mut fx = engine(&doc);
    render(&mut fx, TIME / 2);
    (fx.set_control("burst", "level", ControlValue::Number(0.0))).expect("the burst silences");
    let (left, _) = render(&mut fx, count * TIME);
    let levels: Vec<f32> = left.as_chunks::<TIME>().0.iter().map(|c| rms(c)).collect();
    assert!(levels[0] > amp as f32 / 100.0, "{hz} Hz: the burst echoes");
    levels
}

/// Each repeat's level against the first, in dB.
fn relative_db(levels: &[f32]) -> impl Iterator<Item = (usize, f32)> + '_ {
    let first = levels[0];
    (levels.iter().enumerate()).map(move |(n, level)| (n + 1, 20.0 * (level / first).log10()))
}

/// 30 s of repeats at 0.3 s each.
const HALF_MINUTE: usize = 100;

#[test]
fn at_feedback_1_quiet_low_bursts_repeat_for_30_seconds_within_1_db() {
    for hz in [500.0, PEAK_HZ, 1_000.0] {
        for (n, db) in relative_db(&repeats(hz, QUIET, OPEN, HALF_MINUTE)) {
            assert!(
                db.abs() <= 1.0,
                "{hz} Hz: repeat {n} is {db} dB from the first"
            );
        }
    }
}

#[test]
fn at_feedback_1_no_burst_grows_over_30_seconds() {
    for tone in [DEFAULT_TONE, OPEN] {
        for amp in [QUIET, LOUD] {
            for hz in [500.0, 2_000.0] {
                for (n, db) in relative_db(&repeats(hz, amp, tone, HALF_MINUTE)) {
                    assert!(
                        db <= 1.0,
                        "{hz} Hz at {amp}, tone {tone}: repeat {n} is up {db} dB"
                    );
                }
            }
        }
    }
}

/// The creep a loop gain a hair above 1 would bring shows only over minutes:
/// without the loop's trim the 30 s checks miss it by little, and this one
/// by several dB, so this is the guard that keeps it from coming back.
#[test]
fn at_feedback_1_a_quiet_repeat_does_not_creep_up_over_two_minutes() {
    for hz in [500.0, PEAK_HZ] {
        for (n, db) in relative_db(&repeats(hz, QUIET, OPEN, 4 * HALF_MINUTE)) {
            assert!(db <= 1.0, "{hz} Hz: repeat {n} is up {db} dB");
        }
    }
}

/// Renders 6 s of a burst's repeats with `feedback` configured at `config`,
/// then `live` written once the first echo has passed.
fn heard(config: f64, live: Option<f64>) -> Vec<f32> {
    let config = still(json!({ "dry": 0.0, "wet": 1.0, "feedback": config, "tone": OPEN }));
    let mut fx = engine(&burst_doc(&Rig::new(), config, QUIET));
    let (mut left, _) = render(&mut fx, TIME * 3 / 2);
    if let Some(value) = live {
        (fx.set_control("fx", "feedback", ControlValue::Number(value as f32)))
            .expect("a write is accepted");
    }
    left.extend(render(&mut fx, 6 * SR as usize).0);
    left
}

#[test]
fn feedback_above_1_sounds_as_1() {
    let top = heard(1.0, None);
    assert!(rms(&top[TIME * 15..]) > 1e-3, "the repeats are heard");
    assert_eq!(heard(1.5, None), top, "configured at 1.5");
    let written = heard(0.4, Some(1.0));
    assert_eq!(heard(0.4, Some(1.5)), written, "written 1.5 over 0.4");
    assert_eq!(heard(0.4, Some(2.0)), written, "written 2 over 0.4");
}
