//! Inventions around the granular development, rendering and
//! measurements.

use std::path::PathBuf;

use fugue::{ControlValue, RenderEngine};
use serde_json::{json, Value};

pub const SR: u32 = 48_000;
pub const GRANULAR: &str = "fugue.effects.granular";
/// One second, in frames.
pub const SECOND: usize = SR as usize;

pub fn pack_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs")
        .join(GRANULAR)
}

/// The pack's development file, as an absolute path.
pub fn development_path() -> String {
    let path = pack_dir().join("granular.development.json");
    let path = (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    path.to_string_lossy().into_owned()
}

pub fn conn(from: &str, from_port: &str, to: &str, to_port: &str) -> Value {
    json!({ "from": from, "from_port": from_port, "to": to, "to_port": to_port })
}

/// A source: its modules and the `(module, port)` each pack input reads.
#[derive(Clone)]
pub struct Source {
    modules: Vec<Value>,
    connections: Vec<Value>,
    left: (String, String),
    right: (String, String),
}

impl Source {
    /// A sine of `freq` Hz at `amp` peak into both inputs.
    pub fn sine(freq: f64, amp: f64) -> Self {
        let osc = json!({ "waveform": "sine", "frequency": freq });
        let level = json!({ "level": amp });
        Source {
            modules: vec![
                json!({ "id": "sine", "type": "oscillator", "config": osc }),
                json!({ "id": "sine_level", "type": "vca", "config": level }),
            ],
            connections: vec![conn("sine", "audio", "sine_level", "audio")],
            left: ("sine_level".into(), "audio".into()),
            right: ("sine_level".into(), "audio".into()),
        }
    }

    /// Seeded full-scale white noise, a different seed on each input.
    pub fn noise() -> Self {
        let noise = |id: &str, seed: u32| json!({ "id": id, "type": "noise", "config": { "seed": seed, "level": 1.0 } });
        Source {
            modules: vec![noise("nl", 41), noise("nr", 42)],
            connections: Vec::new(),
            left: ("nl".into(), "white".into()),
            right: ("nr".into(), "white".into()),
        }
    }

    /// A 2 kHz sine burst at 0.5 peak for the first `frames` frames, gated
    /// by a 6 bpm clock, so it does not come back within 10 s.
    pub fn burst(frames: usize) -> Self {
        let gate = frames as f64 / (f64::from(SR) * 10.0);
        let clock = json!({ "bpm": 6.0, "gate_length": gate });
        let osc = json!({ "waveform": "sine", "frequency": 2_000.0 });
        Source {
            modules: vec![
                json!({ "id": "burst_clk", "type": "clock", "config": clock }),
                json!({ "id": "burst_osc", "type": "oscillator", "config": osc }),
                json!({ "id": "burst_gate", "type": "vca", "config": {} }),
                json!({ "id": "burst", "type": "vca", "config": { "level": 0.5 } }),
            ],
            connections: vec![
                conn("burst_osc", "audio", "burst_gate", "audio"),
                conn("burst_clk", "beat", "burst_gate", "level"),
                conn("burst_gate", "audio", "burst", "audio"),
            ],
            left: ("burst".into(), "audio".into()),
            right: ("burst".into(), "audio".into()),
        }
    }

    /// The source's modules and connections, its outputs patched into
    /// `to`'s `audio_left` and `audio_right`.
    fn into(&self, to: &str) -> (Vec<Value>, Vec<Value>) {
        let mut connections = self.connections.clone();
        connections.push(conn(&self.left.0, &self.left.1, to, "audio_left"));
        connections.push(conn(&self.right.0, &self.right.1, to, "audio_right"));
        (self.modules.clone(), connections)
    }
}

fn invention(mut modules: Vec<Value>, mut connections: Vec<Value>, to_dac: &str) -> Value {
    modules.push(json!({ "id": "dac", "type": "dac", "config": { "soft_clip": false } }));
    for side in ["audio_left", "audio_right"] {
        connections.push(conn(to_dac, side, "dac", side));
    }
    json!({ "version": "1.0.0", "modules": modules, "connections": connections })
}

/// The invention with the pack as module `fx`, built with `config`.
pub fn effect(source: &Source, config: Value) -> Value {
    let (mut modules, connections) = source.into("fx");
    modules.push(json!({ "id": "fx", "type": "granular_fx", "config": config }));
    let mut doc = invention(modules, connections, "fx");
    doc["developments"] = json!([{ "name": "granular_fx", "path": development_path() }]);
    doc
}

/// The same source through core's bare `granular` module (`fx`), built
/// with the pack's buffer and seed plus `config`.
pub fn bare(source: &Source, mut config: Value) -> Value {
    config["buffer_length"] = json!(4.0);
    config["seed"] = json!(1);
    let (mut modules, connections) = source.into("fx");
    modules.push(json!({ "id": "fx", "type": "granular", "config": config }));
    invention(modules, connections, "fx")
}

/// The source straight to the dac.
pub fn dry(source: &Source) -> Value {
    let mut modules = source.modules.clone();
    modules.push(json!({ "id": "dac", "type": "dac", "config": { "soft_clip": false } }));
    let mut connections = source.connections.clone();
    connections.push(conn(&source.left.0, &source.left.1, "dac", "audio_left"));
    connections.push(conn(&source.right.0, &source.right.1, "dac", "audio_right"));
    json!({ "version": "1.0.0", "modules": modules, "connections": connections })
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

pub fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

pub fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

/// Largest one-sample change.
pub fn max_step(x: &[f32]) -> f32 {
    x.windows(2).fold(0.0, |m, w| m.max((w[1] - w[0]).abs()))
}
