//! Inventions around the EQ development, rendering, and measurements.

use std::path::PathBuf;

use fugue::{ControlValue, RenderEngine};
use serde_json::{json, Value};

pub const SR: u32 = 48_000;
pub const EQ: &str = "fugue.effects.eq";

pub fn pack_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs")
        .join(EQ)
}

/// The pack's development file, as an absolute path.
pub fn development_path() -> String {
    let path = pack_dir().join("eq.development.json");
    let path = (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    path.to_string_lossy().into_owned()
}

fn conn(from: &str, from_port: &str, to: &str, to_port: &str) -> Value {
    json!({ "from": from, "from_port": from_port, "to": to, "to_port": to_port })
}

/// Sources and which pack input each feeds: `(module, port)` per side.
#[derive(Clone, Default)]
pub struct Feed {
    modules: Vec<Value>,
    left: Option<(String, String)>,
    right: Option<(String, String)>,
}

fn noise(id: &str, seed: u32, level: f64) -> Value {
    json!({ "id": id, "type": "noise", "config": { "seed": seed, "level": level } })
}

impl Feed {
    /// Seeded white noise at `level`, a different seed on each side.
    pub fn noise_pair(level: f64) -> Self {
        Feed {
            modules: vec![noise("nl", 41, level), noise("nr", 42, level)],
            left: Some(("nl".into(), "white".into())),
            right: Some(("nr".into(), "white".into())),
        }
    }

    /// One full-scale noise (seed `seed`) into both inputs.
    pub fn noise_both(seed: u32) -> Self {
        let source = Some(("n".to_string(), "white".to_string()));
        Feed {
            modules: vec![noise("n", seed, 1.0)],
            left: source.clone(),
            right: source,
        }
    }

    /// Mono: full-scale noise (seed `seed`, module `id`) into `audio_left`
    /// only.
    pub fn noise_left(id: &str, seed: u32) -> Self {
        Feed {
            modules: vec![noise(id, seed, 1.0)],
            left: Some((id.into(), "white".into())),
            right: None,
        }
    }

    /// A sine of `freq` Hz at `amp` peak into both inputs.
    pub fn sine_both(freq: f64, amp: f64) -> Self {
        let osc = json!({ "waveform": "sine", "frequency": freq });
        let level = json!({ "level": amp });
        Feed {
            modules: vec![
                json!({ "id": "sine", "type": "oscillator", "config": osc }),
                json!({ "id": "sine_level", "type": "vca", "config": level }),
            ],
            left: Some(("sine_level".into(), "audio".into())),
            right: Some(("sine_level".into(), "audio".into())),
        }
    }

    fn sources(&self) -> (Vec<Value>, Vec<Value>) {
        let mut connections = Vec::new();
        if self.modules.iter().any(|m| m["id"] == "sine") {
            connections.push(conn("sine", "audio", "sine_level", "audio"));
        }
        let mut modules = self.modules.clone();
        modules.push(json!({ "id": "dac", "type": "dac", "config": { "soft_clip": false } }));
        (modules, connections)
    }

    fn sides(&self) -> [(&'static str, &Option<(String, String)>); 2] {
        [("audio_left", &self.left), ("audio_right", &self.right)]
    }

    /// The invention with the pack as module `fx` (built with `config`),
    /// its outputs scaled by `tap` on the way to the dac (1.0: straight).
    pub fn effect(&self, config: Value, tap: f64) -> Value {
        let (mut modules, mut connections) = self.sources();
        modules.push(json!({ "id": "fx", "type": "eq", "config": config }));
        for (side, source) in self.sides() {
            if let Some((from, port)) = source {
                connections.push(conn(from, port, "fx", side));
            }
            if tap == 1.0 {
                connections.push(conn("fx", side, "dac", side));
            } else {
                let id = format!("tap_{side}");
                modules.push(json!({ "id": id, "type": "vca", "config": { "level": tap } }));
                connections.push(conn("fx", side, &id, "audio"));
                connections.push(conn(&id, "audio", "dac", side));
            }
        }
        json!({
            "version": "1.0.0",
            "developments": [{ "name": "eq", "path": development_path() }],
            "modules": modules,
            "connections": connections
        })
    }

    /// The same sources, each routed straight to the dac side whose pack
    /// input it feeds: "the dry input".
    pub fn dry(&self) -> Value {
        let (modules, mut connections) = self.sources();
        for (side, source) in self.sides() {
            if let Some((from, port)) = source {
                connections.push(conn(from, port, "dac", side));
            }
        }
        json!({ "version": "1.0.0", "modules": modules, "connections": connections })
    }
}

pub fn engine(doc: &Value) -> RenderEngine {
    let mut engine = RenderEngine::new(SR);
    (engine.load_json(&doc.to_string())).unwrap_or_else(|err| panic!("invention loads: {err}"));
    engine
}

/// Renders `frames` and returns the (left, right) outputs.
pub fn render(engine: &mut RenderEngine, frames: usize) -> (Vec<f32>, Vec<f32>) {
    let mut out = vec![0.0f32; frames * 2];
    engine.render_interleaved(&mut out).expect("renders");
    let left = out.iter().step_by(2).copied().collect();
    let right = out.iter().skip(1).step_by(2).copied().collect();
    (left, right)
}

pub fn render_doc(doc: &Value, frames: usize) -> (Vec<f32>, Vec<f32>) {
    render(&mut engine(doc), frames)
}

pub fn set(engine: &RenderEngine, key: &str, value: impl Into<ControlValue>) {
    (engine.set_control("fx", key, value.into()))
        .unwrap_or_else(|err| panic!("fx.{key} accepts the value: {err}"));
}

/// First sample in `range` where `a` and `b` differ bit for bit.
pub fn first_difference(a: &[f32], b: &[f32], range: std::ops::Range<usize>) -> Option<usize> {
    range
        .into_iter()
        .find(|&n| a[n].to_bits() != b[n].to_bits())
}

/// Asserts `a` and `b` agree bit for bit over `range`, naming the first
/// difference.
pub fn assert_same(what: &str, a: &[f32], b: &[f32], range: std::ops::Range<usize>) {
    if let Some(n) = first_difference(a, b, range) {
        panic!(
            "{what}: sample {n} is {} where the reference is {}",
            a[n], b[n]
        );
    }
}

pub fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

pub fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

/// Largest one-sample change.
pub fn max_step(x: &[f32]) -> f32 {
    x.windows(2).fold(0.0, |m, w| m.max((w[1] - w[0]).abs()))
}

/// Gain in dB of the pack built with `config` at `freq` Hz: the RMS of a
/// steady 0.1-peak sine through it against the same sine dry, over 0.5 s
/// after 0.25 s to settle (left side).
pub fn gain_db(config: Value, freq: f64) -> f64 {
    let feed = Feed::sine_both(freq, 0.1);
    let frames = 36_000;
    let (wet, _) = render_doc(&feed.effect(config, 1.0), frames);
    let (dry, _) = render_doc(&feed.dry(), frames);
    assert!(wet.iter().all(|v| v.is_finite()), "finite at {freq} Hz");
    let window = 12_000..frames;
    20.0 * (rms(&wet[window.clone()]) / rms(&dry[window])).log10()
}

/// The RBJ Audio EQ Cookbook responses the bands follow.
#[derive(Clone, Copy, Debug)]
pub enum Rbj {
    Peaking,
    LowShelf,
    HighShelf,
}

/// Magnitude in dB at `f` of the RBJ cookbook biquad `kind` designed with
/// `f0`, `db_gain` and `q` (the Q form) at 48 kHz.
pub fn rbj_db(kind: Rbj, f0: f64, db_gain: f64, q: f64, f: f64) -> f64 {
    let a = 10f64.powf(db_gain / 40.0);
    let w0 = 2.0 * std::f64::consts::PI * f0 / f64::from(SR);
    let (cw, alpha) = (w0.cos(), w0.sin() / (2.0 * q));
    let sa = 2.0 * a.sqrt() * alpha;
    let (b, den) = match kind {
        Rbj::Peaking => (
            [1.0 + alpha * a, -2.0 * cw, 1.0 - alpha * a],
            [1.0 + alpha / a, -2.0 * cw, 1.0 - alpha / a],
        ),
        Rbj::LowShelf => (
            [
                a * ((a + 1.0) - (a - 1.0) * cw + sa),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cw),
                a * ((a + 1.0) - (a - 1.0) * cw - sa),
            ],
            [
                (a + 1.0) + (a - 1.0) * cw + sa,
                -2.0 * ((a - 1.0) + (a + 1.0) * cw),
                (a + 1.0) + (a - 1.0) * cw - sa,
            ],
        ),
        Rbj::HighShelf => (
            [
                a * ((a + 1.0) + (a - 1.0) * cw + sa),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cw),
                a * ((a + 1.0) + (a - 1.0) * cw - sa),
            ],
            [
                (a + 1.0) - (a - 1.0) * cw + sa,
                2.0 * ((a - 1.0) - (a + 1.0) * cw),
                (a + 1.0) - (a - 1.0) * cw - sa,
            ],
        ),
    };
    let w = 2.0 * std::f64::consts::PI * f / f64::from(SR);
    let mag = |c: [f64; 3]| {
        let re = c[0] + c[1] * w.cos() + c[2] * (2.0 * w).cos();
        let im = -(c[1] * w.sin() + c[2] * (2.0 * w).sin());
        (re * re + im * im).sqrt()
    };
    20.0 * (mag(b) / mag(den)).log10()
}
