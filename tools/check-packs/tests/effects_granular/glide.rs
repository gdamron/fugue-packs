//! The looping delay's tape glide through the pack: passing `glide` and
//! `inertia` through is neutral at their defaults, and a glided move bends
//! the pitch where glide 0 crossfades.

use std::fs;
use std::path::{Path, PathBuf};

use fugue::{ControlValue, RenderEngine};
use serde_json::{json, Value};

use super::rig::*;

/// The steady tone the delay reads, and the analysis window for its pitch.
const TONE: f64 = 1_000.0;
const WINDOW: usize = SECOND / 20;

/// The development without its `glide` and `inertia` controls, as the pack
/// was before it passed them through, written to a temp directory of its own
/// (the caller removes it).
fn development_without_glide(dir: &Path) -> PathBuf {
    let text = fs::read_to_string(development_path()).expect("development reads");
    let mut development: Value = serde_json::from_str(&text).expect("development is JSON");
    let controls = development["controls"].as_array_mut().expect("controls");
    let before = controls.len();
    controls.retain(|c| c["name"] != "glide" && c["name"] != "inertia");
    assert_eq!(before - controls.len(), 2, "the pack passes both through");
    fs::create_dir_all(dir).expect("temp dir");
    let path = dir.join("granular.development.json");
    fs::write(&path, development.to_string()).expect("writes");
    path
}

fn without_glide(source: &Source, config: Value, dir: &Path) -> Value {
    let mut doc = effect(source, config);
    let path = development_without_glide(dir);
    doc["developments"][0]["path"] = json!(path.to_string_lossy());
    doc
}

/// Renders `seconds` with `position` written to each `(second, value)` in
/// `moves`, returning the left output.
fn play(doc: &Value, seconds: usize, moves: &[(usize, f32)]) -> Vec<f32> {
    let mut fx: RenderEngine = engine(doc);
    let mut out = Vec::with_capacity(seconds * SECOND);
    for second in 0..seconds {
        for &(_, value) in moves.iter().filter(|(at, _)| *at == second) {
            set(&fx, "position", ControlValue::Number(value));
        }
        out.extend(render(&mut fx, SECOND).0);
    }
    out
}

fn bits(x: &[f32]) -> Vec<u32> {
    x.iter().map(|v| v.to_bits()).collect()
}

/// The pack's wiring is neutral at the defaults: bit-identical output to the
/// development without the two controls, in every mode and across position
/// moves, and the same with the defaults written into config. (That glide 0
/// sounds as the module did before it had the control is core's guarantee,
/// covered by its own tests.)
#[test]
fn glide_and_inertia_at_their_defaults_change_nothing() {
    let moves = [(2, 0.4), (4, 0.3)];
    let dir = std::env::temp_dir().join(format!("granular-pack-defaults-{}", std::process::id()));
    for mode in ["granular", "stretch", "looping_delay"] {
        let source = Source::noise_at(-6.0);
        let config = json!({ "mode": mode, "position": 0.25, "wet": 1.0 });
        let before = play(&without_glide(&source, config.clone(), &dir), 6, &moves);
        let now = play(&effect(&source, config.clone()), 6, &moves);
        assert!(rms(&now) > 0.01, "{mode}: plays");
        assert!(bits(&now) == bits(&before), "{mode}: defaults differ");
        let mut stated = config;
        stated["glide"] = json!(0.0);
        stated["inertia"] = json!(0.5);
        let stated = play(&effect(&source, stated), 6, &moves);
        assert!(
            bits(&stated) == bits(&before),
            "{mode}: stated defaults differ"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Pitch relative to `TONE` in each window from `from` to `to` seconds,
/// from the spacing of upward zero crossings.
fn speeds(x: &[f32], from: f64, to: f64) -> Vec<f64> {
    let at = |t: f64| (t * f64::from(SR)) as usize;
    let mut speeds = Vec::new();
    let mut start = at(from);
    while start + WINDOW <= at(to) {
        let w = &x[start..start + WINDOW];
        let ups: Vec<f64> = (w.windows(2).enumerate())
            .filter(|(_, p)| p[0] <= 0.0 && p[1] > 0.0)
            .map(|(n, p)| n as f64 + f64::from(p[0] / (p[0] - p[1])))
            .collect();
        if ups.len() >= 3 {
            let span = ups[ups.len() - 1] - ups[0];
            let cycles = (ups.len() - 1) as f64;
            speeds.push(cycles / span * f64::from(SR) / TONE);
        }
        start += WINDOW;
    }
    speeds
}

/// A tone through the looping delay, `position` moved from 0.25 to 0.3 at
/// 3 s (a longer delay), with `glide` and `inertia`; heavy tape takes the
/// longest to settle.
fn moved(glide: f32, inertia: f32) -> Vec<f32> {
    let config = json!({
        "mode": "looping_delay", "density": 0.0, "position": 0.25,
        "glide": glide, "inertia": inertia, "dry": 0.0, "wet": 1.0, "reverb": 0.0
    });
    play(&effect(&Source::sine(TONE, 0.5), config), 13, &[(3, 0.3)])
}

/// With glide on, the delay slides: the tone bends flat while it grows and
/// returns to pitch, on both sides of the default inertia. With glide 0 it
/// crossfades: the pitch never leaves the tone after the jump.
#[test]
fn a_glided_move_bends_where_glide_zero_crossfades() {
    for inertia in [0.0, 0.5, 1.0] {
        let y = moved(2.0, inertia);
        let during = speeds(&y, 3.05, 6.0);
        let lowest = during.iter().copied().fold(f64::MAX, f64::min);
        assert!(lowest < 0.97, "inertia {inertia}: no bend ({lowest})");
        assert!(
            lowest > 0.48,
            "inertia {inertia}: below half speed ({lowest})"
        );
        let settled = speeds(&y, 12.0, 13.0);
        for s in settled {
            assert!((s - 1.0).abs() < 0.015, "inertia {inertia}: settles at {s}");
        }
    }
    let y = moved(0.0, 0.5);
    for s in speeds(&y, 3.05, 13.0) {
        assert!((s - 1.0).abs() < 0.01, "glide 0 bends: {s}");
    }
}
