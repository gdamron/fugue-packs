//! Acceptance tests for the tape delay's `feedback` control, written from
//! the spec alone. The pack is loaded by path against the pinned core and
//! judged by what is described, read back and heard:
//!
//! 1. `feedback` is described with a 0 to 1 range and starts at 0.4,
//! 2. a write of 1.5, live or configured, lands at 1.0 and sounds as 1.0,
//! 3. at `feedback` 1 a quiet tone burst repeats at unity (within 1 dB),
//! 4. at `feedback` 1 no repeat rises more than 1 dB above the first,
//! 5. at `feedback` 0 only the first echo is heard,
//! 6. the pack version is a date-based YYYY.M.P later than 2026.11.2.
//!
//! The criteria are quoted by number in each test's name or comment. The
//! shared delay rig (`tests/effects_delay/rig.rs`) builds the inventions.
//!
//! Run with `cargo test --release --test effects_tape_feedback_acceptance`.

#[allow(dead_code)]
#[path = "effects_delay/rig.rs"]
mod rig;

use fugue::{
    AudioBackend, ControlKind, ControlMeta, ControlValue, DescribeModuleQuery, InventionBuilder,
    RenderEngine,
};
use rig::*;
use serde_json::{json, Value};

const TAPE: &str = "fugue.effects.tape-delay";
const ALIAS: &str = "delay_fx";
/// The default `time`, 0.3 s: one repeat period in samples.
const T: usize = 14_400;
/// The burst length in samples, as in the tape delay suite.
const BURST_LEN: usize = 480;

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-6 * b.abs().max(1.0)
}

fn bounds(controls: &[ControlMeta], key: &str) -> (f32, f32) {
    let meta = (controls.iter().find(|c| c.key == key)).unwrap_or_else(|| panic!("no {key}"));
    match meta.kind {
        ControlKind::Number { min, max, .. } => (min, max),
        _ => panic!("{key} is not a number control"),
    }
}

fn feedback_of(engine: &RenderEngine) -> f32 {
    match engine.get_control("fx", "feedback").expect("reads") {
        ControlValue::Number(number) => number,
        other => panic!("feedback reads {other:?}"),
    }
}

/// A backend that never pulls audio: enough to describe a live invention.
struct Idle;

impl AudioBackend for Idle {
    fn sample_rate(&self) -> u32 {
        SR
    }
    fn start(
        &mut self,
        _render: Box<dyn FnMut(&mut [f32], &mut [f32]) + Send>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn stop(&mut self) {}
}

fn bare() -> Value {
    Rig::new().effect(TAPE, json!({}), 1.0)
}

/// Criterion 1: "describe_development / list_controls / describe_module ...
/// report `feedback` with a range of 0-1", for the instance and the type.
#[test]
fn criterion_1_describe_module_reports_feedback_from_zero_to_one() {
    let invention = serde_json::from_value(bare()).expect("parses");
    let (runtime, _) = InventionBuilder::new(SR).build(invention).expect("builds");
    let running = runtime.start_with_backend(Idle).expect("starts");
    let by_instance = DescribeModuleQuery {
        module_id: Some("fx".into()),
        ..Default::default()
    };
    let by_type = DescribeModuleQuery {
        module_type: Some(ALIAS.into()),
        config: Some(json!({})),
        ..Default::default()
    };
    for (which, query) in [("instance", by_instance), ("type", by_type)] {
        let description = running.describe_module(&query).expect("describes");
        let (min, max) = bounds(&description.info.controls, "feedback");
        assert!(
            near(min, 0.0) && near(max, 1.0),
            "feedback described by {which}: {min} to {max}"
        );
    }
}

#[test]
fn criterion_1_list_controls_reports_feedback_from_zero_to_one() {
    let fx = engine(&bare());
    let controls = fx.list_controls(Some("fx")).expect("lists").remove(0).1;
    let (min, max) = bounds(&controls, "feedback");
    assert!(near(min, 0.0) && near(max, 1.0), "feedback: {min} to {max}");
}

/// Criterion 1: "The default stays 0.4."
#[test]
fn criterion_1_default_stays_at_point_four() {
    assert!(near(feedback_of(&engine(&bare())), 0.4));
}

/// Criterion 2: "A write of 1.5 to `feedback`, live ...: it reads back 1.0".
#[test]
fn criterion_2_a_live_write_of_one_and_a_half_reads_back_one() {
    let mut fx = engine(&bare());
    set(&fx, "feedback", 1.5f32);
    render(&mut fx, 512);
    assert!(near(feedback_of(&fx), 1.0), "reads {}", feedback_of(&fx));
}

/// Criterion 2: "... or configured, lands at 1.0: it reads back 1.0".
#[test]
fn criterion_2_a_configured_one_and_a_half_reads_back_one() {
    let fx = engine(&Rig::new().effect(TAPE, json!({ "feedback": 1.5 }), 1.0));
    assert!(near(feedback_of(&fx), 1.0), "reads {}", feedback_of(&fx));
}

/// Two seconds of seeded noise through the pack (dry 0, wet 1), with
/// `feedback` configured or written live.
fn heard(feedback: f64, live: bool) -> Vec<f32> {
    let config = if live {
        json!({ "dry": 0.0, "wet": 1.0 })
    } else {
        json!({ "dry": 0.0, "wet": 1.0, "feedback": feedback })
    };
    let rig = Rig::new().noise("nl", 41).noise("nr", 42);
    let rig = rig.feed("nl", "white", "audio_left");
    let rig = rig.feed("nr", "white", "audio_right");
    let mut fx = engine(&rig.effect(TAPE, config, 0.5));
    if live {
        set(&fx, "feedback", feedback as f32);
    }
    let (left, right) = render(&mut fx, 2 * SR as usize);
    left.into_iter().chain(right).collect()
}

/// Criterion 2: "it renders the same as 1.0", to the byte, live and
/// configured. The guard shows the render depends on `feedback`.
#[test]
fn criterion_2_a_write_of_one_and_a_half_renders_as_one() {
    let one = heard(1.0, true);
    assert!(one.iter().any(|v| v.abs() > 1e-3), "the render is heard");
    assert!(heard(0.5, true) != one, "0.5 and 1.0 render differently");
    assert!(heard(1.5, true) == one, "live 1.5 renders unlike 1.0");
    assert!(
        heard(1.5, false) == one,
        "configured 1.5 renders unlike 1.0"
    );
    assert!(
        heard(1.0, false) == one,
        "configured 1.0 renders unlike live"
    );
}

/// One tone burst of `freq` Hz at `amp` peak in both inputs, from the first
/// sample, through the pack at feedback 1 (dry 0, wet 1, no wow or flutter)
/// with `tone` and `feedback`. The 20 bpm clock would retrigger it every
/// 3 s, so the burst is silenced once it has played. Returns the left output.
fn burst_render(freq: f64, amp: f64, tone: f64, feedback: f64, frames: usize) -> Vec<f32> {
    let config = json!({ "dry": 0.0, "wet": 1.0, "feedback": feedback, "tone": tone,
        "wow": 0.0, "flutter": 0.0 });
    let mut doc = Rig::new().effect(TAPE, config, 1.0);
    let gate = BURST_LEN as f64 / (f64::from(SR) * 3.0);
    let clock = json!({ "bpm": 20.0, "gate_length": gate });
    let osc = json!({ "waveform": "sine", "frequency": freq });
    let modules = doc["modules"].as_array_mut().expect("modules");
    modules.push(json!({ "id": "clk", "type": "clock", "config": clock }));
    modules.push(json!({ "id": "osc", "type": "oscillator", "config": osc }));
    modules.push(json!({ "id": "gate", "type": "vca", "config": {} }));
    modules.push(json!({ "id": "burst", "type": "vca", "config": { "level": amp } }));
    let wires = doc["connections"].as_array_mut().expect("connections");
    wires.push(conn("osc", "audio", "gate", "audio"));
    wires.push(conn("clk", "beat", "gate", "level"));
    wires.push(conn("gate", "audio", "burst", "audio"));
    wires.push(conn("burst", "audio", "fx", "audio_left"));
    wires.push(conn("burst", "audio", "fx", "audio_right"));
    let mut fx = engine(&doc);
    let mut left = render(&mut fx, 2 * BURST_LEN).0;
    (fx.set_control("burst", "level", ControlValue::Number(0.0))).expect("silences the burst");
    left.extend(render(&mut fx, frames - 2 * BURST_LEN).0);
    assert!(left.iter().all(|v| v.is_finite()), "the output is finite");
    left
}

/// Level of each of `repeats` repeats in dB relative to the first. A repeat's
/// level is the RMS over one repeat period (`T` samples). Each period is
/// centered on the energy of the repeat before, plus `T`, so a few frames of
/// latency per pass cannot walk a repeat out of its window.
fn repeat_levels(freq: f64, amp: f64, tone: f64, feedback: f64, repeats: usize) -> Vec<f64> {
    let frames = (repeats + 1) * T + 40_000;
    let x = burst_render(freq, amp, tone, feedback, frames);
    let mut center = T;
    let mut levels = Vec::new();
    while levels.len() < repeats && center + T / 2 <= x.len() {
        let window = &x[center - T / 2..center + T / 2];
        let energy: f64 = window.iter().map(|v| f64::from(v * v)).sum();
        levels.push((energy / T as f64).sqrt());
        let first = center - T / 2;
        let moment: f64 = (window.iter().enumerate())
            .map(|(n, v)| (first + n) as f64 * f64::from(v * v))
            .sum();
        center = if energy > 1e-20 {
            (moment / energy) as usize
        } else {
            center
        } + T;
    }
    assert_eq!(levels.len(), repeats, "{repeats} repeats fit in the render");
    assert!(
        levels[0] > 1e-4,
        "the first repeat is heard ({})",
        levels[0]
    );
    let first = levels[0];
    levels.iter().map(|l| 20.0 * (l / first).log10()).collect()
}

fn describe(levels: &[f64]) -> String {
    let (lo, hi) = (levels.iter()).fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    format!("{lo:.2} to {hi:.2} dB against the first")
}

/// Criterion 3: "every repeat over 30 s (100 repeats) is within +-1 dB of
/// the first", at tone 20000 and a 0.02 peak burst.
fn assert_unity(freq: f64) {
    let levels = repeat_levels(freq, 0.02, 20_000.0, 1.0, 100);
    let bad = levels.iter().position(|d| d.abs() > 1.0);
    assert!(
        bad.is_none(),
        "{freq} Hz: repeat {bad:?} of 100 strays, {}",
        describe(&levels)
    );
}

#[test]
fn criterion_3_unity_at_500_hz() {
    assert_unity(500.0);
}

#[test]
fn criterion_3_unity_at_800_hz() {
    assert_unity(800.0);
}

#[test]
fn criterion_3_unity_at_1_khz() {
    assert_unity(1_000.0);
}

/// Criterion 4: "over 30 s, no repeat rises more than +1 dB above the
/// first. Decay is allowed." For `tone` and `amp`, at 500 Hz and 2 kHz.
fn assert_no_growth(tone: f64, amp: f64) {
    for freq in [500.0, 2_000.0] {
        let levels = repeat_levels(freq, amp, tone, 1.0, 100);
        let top = levels.iter().cloned().fold(f64::MIN, f64::max);
        let what = format!("tone {tone}, burst {amp}, {freq} Hz");
        assert!(top <= 1.0, "{what}: a repeat rises, {}", describe(&levels));
    }
}

#[test]
fn criterion_4_no_growth_at_default_tone_with_a_quiet_burst() {
    assert_no_growth(3_500.0, 0.02);
}

#[test]
fn criterion_4_no_growth_at_default_tone_with_a_loud_burst() {
    assert_no_growth(3_500.0, 0.3);
}

#[test]
fn criterion_4_no_growth_at_open_tone_with_a_quiet_burst() {
    assert_no_growth(20_000.0, 0.02);
}

#[test]
fn criterion_4_no_growth_at_open_tone_with_a_loud_burst() {
    assert_no_growth(20_000.0, 0.3);
}

/// Criterion 4: "quiet 0.02 bursts at 500 Hz and 800 Hz with tone 20000,
/// over 120 s (400 repeats), with no rise above +1 dB."
#[test]
fn criterion_4_no_growth_over_two_minutes() {
    for freq in [500.0, 800.0] {
        let levels = repeat_levels(freq, 0.02, 20_000.0, 1.0, 400);
        let top = levels.iter().cloned().fold(f64::MIN, f64::max);
        assert!(
            top <= 1.0,
            "{freq} Hz over 120 s rises, {}",
            describe(&levels)
        );
    }
}

/// Criterion 5: "assert that feedback 0 leaves only the first echo": once
/// the first echo has rung out (0.2 s on), nothing is heard for the rest of
/// 5 s, past the point where an unsilenced clock would retrigger the burst.
#[test]
fn criterion_5_feedback_zero_leaves_only_the_first_echo() {
    let x = burst_render(800.0, 0.3, 20_000.0, 0.0, 5 * SR as usize);
    let echo = peak(&x[T..T + 2_000]);
    assert!(echo > 0.01, "the first echo is heard ({echo})");
    let later = peak(&x[T + 9_600..]);
    assert!(later < 1e-5, "a second echo is heard ({later})");
}

/// Criterion 6: "The pack version is a date-based YYYY.M.P, later than
/// 2026.11.2": any such version from 2026.11.3 on.
#[test]
fn criterion_6_pack_version_is_date_based_and_later_than_2026_11_2() {
    let path = pack_dir(TAPE).join("fugue.pkg.json");
    let text = std::fs::read_to_string(&path).expect("manifest reads");
    let manifest: Value = serde_json::from_str(&text).expect("manifest parses");
    let version = manifest["version"].as_str().expect("version is a string");
    let parts: Vec<&str> = version.split('.').collect();
    assert_eq!(parts.len(), 3, "{version}: not YYYY.M.P");
    for part in &parts {
        assert!(
            part.bytes().all(|b| b.is_ascii_digit()),
            "{version}: not numeric"
        );
        assert!(
            part.len() == 1 || !part.starts_with('0'),
            "{version}: leading zero"
        );
    }
    assert_eq!(parts[0].len(), 4, "{version}: year is not four digits");
    let n: Vec<u32> = parts.iter().map(|p| p.parse().expect("numeric")).collect();
    assert!((1..=12).contains(&n[1]), "{version}: month out of 1 to 12");
    assert!((n[0], n[1], n[2]) > (2026, 11, 2), "{version} is not later");
}
