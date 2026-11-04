//! "Time changes glide pitch with no step": a 0.5, 440 Hz sine into both
//! inputs, `dry` 0, `wet` 1, `feedback` 0, `wow` 0 and `flutter` 0, with
//! `time` changed live from 0.3 to 0.15 and back.

use serde_json::json;

use super::rig::*;
use super::tape::*;

const TONE: f64 = 440.0;
/// Pitch windows of 25 ms (11 cycles), short enough to follow a glide.
const WINDOW: usize = 1_200;
/// How long after the change the pitch has to settle back. The spec does
/// not say how fast the head slews; 5 s is generous for a 0.15 s change.
const SETTLE: usize = 5 * SR as usize;

/// Frequency of each `WINDOW` of `x`.
fn pitch_track(x: &[f32]) -> Vec<f64> {
    x.chunks_exact(WINDOW).map(frequency).collect()
}

/// Plays the sine with `time` at `from`, changes it to `to` once the echo
/// is steady, and checks the step bound, the pitch's excursion (rising when
/// the time shortens, falling when it lengthens) and its return to 440 Hz.
fn glide(from: f32, to: f32) {
    let what = format!("time {from} -> {to}");
    let config = still(json!({ "time": from, "dry": 0.0, "wet": 1.0, "feedback": 0.0 }));
    let mut fx = engine(&sine_doc(config, TONE, 0.5, 1.0));
    let lead = SR as usize;
    let (before, _) = render(&mut fx, lead);
    let steady_from = (from * SR as f32) as usize + 4_800;
    let steady = &before[steady_from..];
    let steady_step = max_step(steady);
    let steady_pitch = frequency(steady);
    assert!(
        steady_step > 0.01,
        "{what}: the echo is heard ({steady_step})"
    );
    assert!(
        (steady_pitch / TONE - 1.0).abs() < 0.002,
        "{what}: the steady echo is at {steady_pitch} Hz"
    );

    set(&fx, "time", to);
    let (after, right) = render(&mut fx, SETTLE + SR as usize);
    let diff = (after.iter().zip(&right)).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
    assert!(
        diff <= 1e-6,
        "{what}: both sides hear the same echo ({diff})"
    );

    // "in every 25 ms stretch of the output, the largest one-sample step
    // must not exceed 1.5x the steady echo's largest step x (that
    // stretch's pitch / 440 Hz)". The stretches tile the steady tail's last
    // 25 ms and everything after the change; each takes in the step from
    // the sample before it, so no step falls between two.
    let mut joined = steady[steady.len() - WINDOW..].to_vec();
    joined.extend_from_slice(&after);
    for c in 0..joined.len() / WINDOW {
        let stretch = &joined[(c * WINDOW).saturating_sub(1)..(c + 1) * WINDOW];
        let (step, pitch) = (max_step(stretch), frequency(stretch));
        let bound = 1.5 * steady_step * (pitch / TONE) as f32;
        let at = c * WINDOW;
        assert!(
            step <= bound,
            "{what}: step {step} at {at} exceeds {bound} (pitch {pitch} Hz, steady step {steady_step})"
        );
    }

    let track = pitch_track(&after[..SETTLE]);
    let (lo, hi) = track
        .iter()
        .fold((f64::MAX, 0.0f64), |(lo, hi), f| (lo.min(*f), hi.max(*f)));
    if to < from {
        assert!(hi > TONE * 1.01, "{what}: the pitch rises (peak {hi} Hz)");
    } else {
        assert!(lo < TONE * 0.99, "{what}: the pitch falls (lowest {lo} Hz)");
    }
    let settled = &after[SETTLE..];
    let end_pitch = frequency(settled);
    assert!(
        (end_pitch / TONE - 1.0).abs() < 0.002,
        "{what}: the pitch settles back to 440 Hz, is {end_pitch} Hz"
    );
    let end_track = pitch_track(settled);
    let drift = end_track
        .iter()
        .fold(0.0f64, |m, f| m.max((f / TONE - 1.0).abs()));
    assert!(drift < 0.005, "{what}: and holds there (off by {drift})");
}

#[test]
fn shortening_the_time_raises_the_pitch_without_a_step_then_settles() {
    glide(0.3, 0.15);
}

#[test]
fn lengthening_the_time_lowers_the_pitch_without_a_step_then_settles() {
    glide(0.15, 0.3);
}
