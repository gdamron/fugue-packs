//! Tape-delay additions to the shared rig: sources patched into both pack
//! inputs, and level and pitch measurements of what is heard.

use serde_json::{json, Value};

use super::rig::*;

pub const TAPE: &str = "fugue.effects.tape-delay";
/// The default `time`, 0.3 s, in samples at 48 kHz.
pub const TIME: usize = 14_400;
/// Frames the oversampled saturation may add to an echo ("plus up to 32
/// frames").
pub const LATENCY: usize = 32;
/// The probe burst: 480 samples, 20 whole cycles of a 2 kHz sine, so it
/// carries no DC for the saturation's DC blocker to leave behind.
pub const BURST_LEN: usize = 480;
pub const BURST_HZ: f64 = 2_000.0;

/// `config` with `wow` and `flutter` at 0, so the delay time holds still.
pub fn still(mut config: Value) -> Value {
    config["wow"] = json!(0.0);
    config["flutter"] = json!(0.0);
    config
}

/// Seeded full-scale white noise, a different seed on each input.
pub fn noise_both() -> Rig {
    Rig::new()
        .noise("nl", 41)
        .noise("nr", 42)
        .feed("nl", "white", "audio_left")
        .feed("nr", "white", "audio_right")
}

/// The pack with a sine of `freq` Hz and `amp` peak patched into both
/// inputs (oscillator `sine` through vca `sine_level`).
pub fn sine_doc(config: Value, freq: f64, amp: f64, tap: f64) -> Value {
    let mut doc = Rig::new().effect(TAPE, config, tap);
    let modules = doc["modules"].as_array_mut().expect("modules");
    let osc = json!({ "waveform": "sine", "frequency": freq });
    modules.push(json!({ "id": "sine", "type": "oscillator", "config": osc }));
    let level = json!({ "level": amp });
    modules.push(json!({ "id": "sine_level", "type": "vca", "config": level }));
    let connections = doc["connections"].as_array_mut().expect("connections");
    connections.push(conn("sine", "audio", "sine_level", "audio"));
    connections.push(conn("sine_level", "audio", "fx", "audio_left"));
    connections.push(conn("sine_level", "audio", "fx", "audio_right"));
    doc
}

/// `rig`'s sources plus a tone burst at `amp` peak into both inputs: a
/// 2 kHz sine (from phase 0) gated for `BURST_LEN` samples from the first
/// sample by a 20 bpm clock, so it repeats only after 144,000 samples.
pub fn burst_doc(rig: &Rig, config: Value, amp: f64) -> Value {
    let mut doc = rig.effect(TAPE, config, 1.0);
    let modules = doc["modules"].as_array_mut().expect("modules");
    let gate = BURST_LEN as f64 / (f64::from(SR) * 3.0);
    let clock = json!({ "bpm": 20.0, "gate_length": gate });
    modules.push(json!({ "id": "burst_clk", "type": "clock", "config": clock }));
    let osc = json!({ "waveform": "sine", "frequency": BURST_HZ });
    modules.push(json!({ "id": "burst_osc", "type": "oscillator", "config": osc }));
    modules.push(json!({ "id": "burst_gate", "type": "vca", "config": {} }));
    let level = json!({ "level": amp });
    modules.push(json!({ "id": "burst", "type": "vca", "config": level }));
    let connections = doc["connections"].as_array_mut().expect("connections");
    connections.push(conn("burst_osc", "audio", "burst_gate", "audio"));
    connections.push(conn("burst_clk", "beat", "burst_gate", "level"));
    connections.push(conn("burst_gate", "audio", "burst", "audio"));
    connections.push(conn("burst", "audio", "fx", "audio_left"));
    connections.push(conn("burst", "audio", "fx", "audio_right"));
    doc
}

/// RMS of the burst (or its echo) heard from `start`, over 12 whole cycles
/// in its middle, clear of its edges.
pub fn burst_level(x: &[f32], start: usize) -> f32 {
    rms(&x[start + 96..start + 384])
}

pub fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

/// Largest one-sample change.
pub fn max_step(x: &[f32]) -> f32 {
    x.windows(2).fold(0.0, |m, w| m.max((w[1] - w[0]).abs()))
}

/// Rising zero crossings, interpolated between samples.
pub fn crossings(x: &[f32]) -> Vec<f64> {
    let mut at = Vec::new();
    for n in 1..x.len() {
        let (a, b) = (f64::from(x[n - 1]), f64::from(x[n]));
        if a < 0.0 && b >= 0.0 {
            at.push((n - 1) as f64 + a / (a - b));
        }
    }
    at
}

/// Intervals between rising zero crossings, in samples.
pub fn periods(x: &[f32]) -> Vec<f64> {
    crossings(x).windows(2).map(|w| w[1] - w[0]).collect()
}

/// Mean frequency over `x`, from its zero crossings.
pub fn frequency(x: &[f32]) -> f64 {
    let at = crossings(x);
    assert!(at.len() >= 3, "a tone is heard ({} crossings)", at.len());
    let mean = (at[at.len() - 1] - at[0]) / (at.len() - 1) as f64;
    f64::from(SR) / mean
}

/// Spread of the periods: (longest - shortest) / mean.
pub fn spread(x: &[f32]) -> f64 {
    let p = periods(x);
    assert!(p.len() >= 10, "a tone is heard ({} periods)", p.len());
    let mean = p.iter().sum::<f64>() / p.len() as f64;
    let (lo, hi) = p
        .iter()
        .fold((f64::MAX, 0.0f64), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    (hi - lo) / mean
}
