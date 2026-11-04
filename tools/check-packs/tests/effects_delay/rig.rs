//! Shared rig: inventions around a pack's development, rendering and
//! signal measurements.

use std::path::PathBuf;

use fugue::{ControlValue, RenderEngine};
use serde_json::{json, Value};

pub const SR: u32 = 48_000;
pub const STEREO: &str = "fugue.effects.stereo-delay";
pub const PING_PONG: &str = "fugue.effects.ping-pong-delay";
pub const BOTH: [&str; 2] = [STEREO, PING_PONG];
/// Magnitude above which a sample counts as heard.
pub const EPS: f32 = 1e-4;
/// Length of the test pulse in samples, and its level.
pub const PULSE_LEN: usize = 480;
pub const PULSE_AMP: f64 = 0.5;

pub fn pack_dir(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs")
        .join(id)
}

/// The pack's development file, as an absolute path.
pub fn development_path(id: &str) -> String {
    let path = pack_dir(id).join("delay.development.json");
    let path = (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    path.to_string_lossy().into_owned()
}

pub fn conn(from: &str, from_port: &str, to: &str, to_port: &str) -> Value {
    json!({ "from": from, "from_port": from_port, "to": to, "to_port": to_port })
}

/// Sources and what they feed. `effect` builds the invention with the pack
/// (module `fx`); `dry` wires the same sources straight to the dac (each
/// `audio_left`/`audio_right` feed to that dac side), the reference for
/// "the dry input".
#[derive(Clone, Default)]
pub struct Rig {
    modules: Vec<Value>,
    feeds: Vec<(String, String, String, String)>,
}

impl Rig {
    pub fn new() -> Self {
        Self::default()
    }

    /// This rig's sources and feeds plus `other`'s.
    pub fn with(&self, other: &Rig) -> Self {
        let mut rig = self.clone();
        rig.modules.extend(other.modules.iter().cloned());
        rig.feeds.extend(other.feeds.iter().cloned());
        rig
    }

    fn link(mut self, from: &str, port: &str, to: &str, input: &str) -> Self {
        self.feeds
            .push((from.into(), port.into(), to.into(), input.into()));
        self
    }

    /// A 480-sample pulse at 0.5 from the first sample, output `<id>.audio`.
    /// Its clock runs at 20 bpm, so it repeats only after 144,000 samples.
    pub fn pulse(mut self, id: &str) -> Self {
        let clk = format!("{id}_clk");
        let gate = PULSE_LEN as f64 / (f64::from(SR) * 3.0);
        let config = json!({ "bpm": 20.0, "gate_length": gate });
        self.modules
            .push(json!({ "id": clk, "type": "clock", "config": config }));
        let config = json!({ "level": PULSE_AMP });
        self.modules
            .push(json!({ "id": id, "type": "vca", "config": config }));
        self.link(&clk, "beat", id, "audio")
    }

    /// Seeded full-scale white noise, output `<id>.white`.
    pub fn noise(mut self, id: &str, seed: u32) -> Self {
        let config = json!({ "seed": seed, "level": 1.0 });
        self.modules
            .push(json!({ "id": id, "type": "noise", "config": config }));
        self
    }

    /// A clock whose `bpm` output carries `bpm`, output `<id>.bpm`.
    pub fn tempo(mut self, id: &str, bpm: f64) -> Self {
        let config = json!({ "bpm": bpm });
        self.modules
            .push(json!({ "id": id, "type": "clock", "config": config }));
        self
    }

    /// Patches `from.port` into the pack's input `input`.
    pub fn feed(self, from: &str, port: &str, input: &str) -> Self {
        self.link(from, port, "fx", input)
    }

    fn doc(&self, mut modules: Vec<Value>, mut connections: Vec<Value>, fx: bool) -> Value {
        modules.extend(self.modules.iter().cloned());
        modules.push(json!({ "id": "dac", "type": "dac", "config": { "soft_clip": false } }));
        for (from, port, to, input) in &self.feeds {
            match (fx, to.as_str(), input.as_str()) {
                (false, "fx", "audio_left" | "audio_right") => {
                    connections.push(conn(from, port, "dac", input))
                }
                (false, "fx", _) => {}
                _ => connections.push(conn(from, port, to, input)),
            }
        }
        json!({ "version": "1.0.0", "modules": modules, "connections": connections })
    }

    /// The invention with the pack as module `fx` (built with `config`),
    /// its outputs scaled by `tap` on the way to the dac (1.0: straight).
    pub fn effect(&self, pack: &str, config: Value, tap: f64) -> Value {
        let mut modules = vec![json!({ "id": "fx", "type": "delay_fx", "config": config })];
        let mut connections = Vec::new();
        for side in ["audio_left", "audio_right"] {
            if tap == 1.0 {
                connections.push(conn("fx", side, "dac", side));
            } else {
                let id = format!("tap_{side}");
                modules.push(json!({ "id": id, "type": "vca", "config": { "level": tap } }));
                connections.push(conn("fx", side, &id, "audio"));
                connections.push(conn(&id, "audio", "dac", side));
            }
        }
        let mut doc = self.doc(modules, connections, true);
        doc["developments"] = json!([{ "name": "delay_fx", "path": development_path(pack) }]);
        doc
    }

    /// The same sources with each pack audio input routed to its dac side.
    pub fn dry(&self) -> Value {
        self.doc(Vec::new(), Vec::new(), false)
    }
}

/// The test pulse patched into both pack inputs.
pub fn pulse_both() -> Rig {
    let rig = Rig::new().pulse("p").feed("p", "audio", "audio_left");
    rig.feed("p", "audio", "audio_right")
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

/// First sample at or after `from` whose magnitude exceeds `EPS`.
pub fn onset(x: &[f32], from: usize) -> Option<usize> {
    (from..x.len()).find(|&n| x[n].abs() > EPS)
}

pub fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

/// Mean level of the pulse (or its echo) that starts at `start`, over its
/// settled middle, clear of the tone filter's edge.
pub fn plateau(x: &[f32], start: usize) -> f32 {
    let span = &x[start + 200..start + 400];
    span.iter().sum::<f32>() / span.len() as f32
}

/// Asserts `a` and `b` agree bit for bit over `range`, naming the first
/// difference.
pub fn assert_same(what: &str, a: &[f32], b: &[f32], range: std::ops::Range<usize>) {
    let first = range.clone().find(|&n| a[n].to_bits() != b[n].to_bits());
    if let Some(n) = first {
        panic!(
            "{what}: sample {n} is {} where the dry input is {}",
            a[n], b[n]
        );
    }
}

pub fn assert_near(what: &str, got: f32, want: f32, tolerance: f32) {
    let ok = (got - want).abs() <= tolerance * want.abs();
    let pct = tolerance * 100.0;
    assert!(ok, "{what}: {got} is not within {pct}% of {want}");
}
