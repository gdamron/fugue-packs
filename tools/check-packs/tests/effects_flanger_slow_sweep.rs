//! The flanger at the bottom of its `rate` range, 0.02 Hz: one sweep takes
//! 50 s, and the delay glides through it without holding or jumping.
//!
//! The sweep is read from what is heard. A 100 Hz sine runs through the
//! flanger wet only, with no feedback, so the output is the input delayed
//! by the swept delay. The dac's right side carries the input itself, so
//! the phase of each 10 ms window (one cycle of the sine) of the left side
//! against the right gives that window's delay, whatever the oscillator's
//! own rounding: the swing of 3 ms is under a third of a cycle, so the
//! phase never wraps.

use std::f64::consts::TAU;
use std::path::PathBuf;

use fugue::RenderEngine;
use serde_json::json;

const SR: u32 = 48_000;
const RATE: f32 = 0.02;
const TONE: f64 = 100.0;
/// One cycle of the tone.
const WINDOW: usize = (SR as f64 / TONE) as usize;
/// Long enough for the delay to cross its center twice after the start.
const SECONDS: usize = 54;

fn development_path() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs/fugue.effects.flanger/flanger.development.json");
    let path = (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    path.to_string_lossy().into_owned()
}

/// The left output of the flanger at `RATE`, full depth and wet only, fed
/// a sine of `TONE`, and that sine itself.
fn render() -> (Vec<f32>, Vec<f32>) {
    let config = json!({ "rate": RATE, "depth": 1.0, "feedback": 0.0, "dry": 0.0, "wet": 1.0 });
    let doc = json!({
        "version": "1.0.0",
        "developments": [{ "name": "flanger", "path": development_path() }],
        "modules": [
            { "id": "osc", "type": "oscillator",
              "config": { "waveform": "sine", "frequency": TONE } },
            { "id": "fx", "type": "flanger", "config": config },
            { "id": "dac", "type": "dac", "config": { "soft_clip": false } }
        ],
        "connections": [
            { "from": "osc", "from_port": "audio", "to": "fx", "to_port": "audio_left" },
            { "from": "osc", "from_port": "audio", "to": "fx", "to_port": "audio_right" },
            { "from": "fx", "from_port": "audio_left", "to": "dac", "to_port": "audio_left" },
            { "from": "osc", "from_port": "audio", "to": "dac", "to_port": "audio_right" }
        ]
    });
    let mut engine = RenderEngine::new(SR);
    (engine.load_json(&doc.to_string())).unwrap_or_else(|err| panic!("loads: {err}"));
    let mut out = vec![0.0f32; SECONDS * SR as usize * 2];
    engine.render_interleaved(&mut out).expect("renders");
    let left = out.iter().step_by(2).copied().collect();
    let right = out.iter().skip(1).step_by(2).copied().collect();
    (left, right)
}

/// The phase of one window of the tone against the absolute time.
fn phase(window: &[f32], start: usize) -> f64 {
    let (mut i, mut q) = (0.0, 0.0);
    for (k, &y) in window.iter().enumerate() {
        let phase = TAU * TONE * (start + k) as f64 / f64::from(SR);
        i += f64::from(y) * phase.sin();
        q += f64::from(y) * phase.cos();
    }
    q.atan2(i)
}

/// The delay of each window of `out` behind `input`, in seconds, up to a
/// whole number of cycles.
fn delay_track(out: &[f32], input: &[f32]) -> Vec<f64> {
    let mut track = Vec::new();
    let mut last: Option<f64> = None;
    let windows = out.chunks_exact(WINDOW).zip(input.chunks_exact(WINDOW));
    for (w, (window, dry)) in windows.enumerate() {
        let mut lag = phase(dry, w * WINDOW) - phase(window, w * WINDOW);
        if let Some(prev) = last {
            lag -= TAU * ((lag - prev) / TAU).round();
        }
        last = Some(lag);
        track.push(lag / (TAU * TONE));
    }
    track
}

/// Window times (s, interpolated) where `track` crosses `center`.
fn crossings(track: &[f64], center: f64) -> Vec<f64> {
    let seconds = WINDOW as f64 / f64::from(SR);
    let mut found = Vec::new();
    for (n, pair) in track.windows(2).enumerate() {
        let (a, b) = (pair[0] - center, pair[1] - center);
        if a != 0.0 && a.signum() != b.signum() {
            found.push((n as f64 + a / (a - b)) * seconds);
        }
    }
    found
}

#[test]
fn a_sweep_at_the_bottom_of_the_rate_range_takes_50_seconds_and_glides() {
    let (out, input) = render();
    // Skip the first half second: the line fills and the controls settle.
    let skip = SR as usize / 2 / WINDOW;
    let track = &delay_track(&out, &input)[skip..];
    let (lo, hi) = track
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &d| (lo.min(d), hi.max(d)));
    // Full depth swings the delay 1.5 ms either way.
    let swing = hi - lo;
    assert!(
        (swing - 3e-3).abs() < 0.1 * 3e-3,
        "the delay swings {swing} s, want about 3 ms"
    );

    // Consecutive crossings of the center are half a sweep apart.
    let center = (hi + lo) / 2.0;
    let found = crossings(track, center);
    assert_eq!(found.len(), 2, "two crossings in {SECONDS} s: {found:?}");
    let period = 2.0 * (found[1] - found[0]);
    let want = 1.0 / f64::from(RATE);
    assert!(
        (period - want).abs() <= 0.01 * want,
        "one sweep takes {period} s, want {want} s +/- 1%"
    );

    // No stepping: from one window to the next the delay's slope changes
    // by at most 0.1 us (0.005 of a sample). A smooth 50 s sweep changes it
    // by about 0.002 us; a held window or a jump of a sample changes it by
    // 20 us.
    let bend = track
        .windows(3)
        .fold(0.0f64, |m, w| m.max((w[2] - 2.0 * w[1] + w[0]).abs()));
    assert!(bend < 1e-7, "the delay's slope jumps by {bend} s");

    // And the audio has no step a smooth delay of the sine would not have.
    let tone_step = 2.0 * (std::f64::consts::PI * TONE / f64::from(SR)).sin();
    let step = out[SR as usize / 2..]
        .windows(2)
        .fold(0.0f32, |m, w| m.max((w[1] - w[0]).abs()));
    assert!(
        f64::from(step) <= 1.05 * tone_step,
        "a step of {step} exceeds the sine's own {tone_step}"
    );
}
