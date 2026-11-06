//! Shared rig: inventions around a pack's development, rendering and
//! signal measurements.

use std::path::PathBuf;

use fugue::{ControlValue, RenderEngine};
use serde_json::{json, Value};

pub const SR: u32 = 48_000;
/// Magnitude above which a sample counts as heard.
pub const EPS: f32 = 1e-4;

/// A pack under test: its id and development file.
#[derive(Clone, Copy)]
pub struct Pack {
    pub id: &'static str,
    pub file: &'static str,
    /// The control that offsets the right side's sweep from the left's.
    pub offset: &'static str,
}

pub const CHORUS: Pack = Pack {
    id: "fugue.effects.chorus",
    file: "chorus.development.json",
    offset: "spread",
};
pub const FLANGER: Pack = Pack {
    id: "fugue.effects.flanger",
    file: "flanger.development.json",
    offset: "stereo_phase",
};
pub const BOTH: [Pack; 2] = [CHORUS, FLANGER];

impl Pack {
    pub fn dir(&self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packs")
            .join(self.id)
    }

    /// The development file, as an absolute path.
    pub fn development_path(&self) -> String {
        let path = self.dir().join(self.file);
        let path = (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        path.to_string_lossy().into_owned()
    }
}

/// The repository root, whether or not the packs exist yet.
pub fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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
    connections: Vec<Value>,
    feeds: Vec<(String, String, String)>,
}

impl Rig {
    pub fn new() -> Self {
        Self::default()
    }

    fn level(mut self, id: &str, source: &str, port: &str, amp: f64) -> Self {
        let config = json!({ "level": amp });
        self.modules
            .push(json!({ "id": id, "type": "vca", "config": config }));
        self.connections.push(conn(source, port, id, "audio"));
        self
    }

    /// A sine of `freq` Hz and `amp` peak, output `<id>.audio`.
    pub fn sine(mut self, id: &str, freq: f64, amp: f64) -> Self {
        let osc = format!("{id}_osc");
        let config = json!({ "waveform": "sine", "frequency": freq });
        self.modules
            .push(json!({ "id": osc, "type": "oscillator", "config": config }));
        self.level(id, &osc, "audio", amp)
    }

    /// Seeded noise, uniform within +/-`level`; outputs `<id>.white` and
    /// `<id>.pink`.
    pub fn noise(mut self, id: &str, seed: u32, level: f64) -> Self {
        let config = json!({ "seed": seed, "level": level });
        self.modules
            .push(json!({ "id": id, "type": "noise", "config": config }));
        self
    }

    /// A constant `amp` from the first sample for 59 s (a 1 bpm clock's
    /// gate at 0.99 of the beat), output `<id>.audio`.
    pub fn constant(mut self, id: &str, amp: f64) -> Self {
        let clk = format!("{id}_clk");
        let config = json!({ "bpm": 1.0, "gate_length": 0.99 });
        self.modules
            .push(json!({ "id": clk, "type": "clock", "config": config }));
        self.level(id, &clk, "beat", amp)
    }

    /// A clock whose `bpm` output carries `bpm`, module `<id>`.
    pub fn tempo(mut self, id: &str, bpm: f64) -> Self {
        let config = json!({ "bpm": bpm });
        self.modules
            .push(json!({ "id": id, "type": "clock", "config": config }));
        self
    }

    /// Patches `from.port` into the pack's input `input`.
    pub fn feed(mut self, from: &str, port: &str, input: &str) -> Self {
        self.feeds.push((from.into(), port.into(), input.into()));
        self
    }

    /// Patches `from.port` into both audio inputs.
    pub fn feed_both(self, from: &str, port: &str) -> Self {
        (self.feed(from, port, "audio_left")).feed(from, port, "audio_right")
    }

    fn doc(&self, mut modules: Vec<Value>, mut connections: Vec<Value>, fx: bool) -> Value {
        modules.extend(self.modules.iter().cloned());
        connections.extend(self.connections.iter().cloned());
        modules.push(json!({ "id": "dac", "type": "dac", "config": { "soft_clip": false } }));
        for (from, port, input) in &self.feeds {
            match (fx, input.as_str()) {
                (true, _) => connections.push(conn(from, port, "fx", input)),
                (false, "audio_left" | "audio_right") => {
                    connections.push(conn(from, port, "dac", input))
                }
                (false, _) => {}
            }
        }
        json!({ "version": "1.0.0", "modules": modules, "connections": connections })
    }

    /// The invention with the pack as module `fx` (built with `config`),
    /// its outputs scaled by `tap` on the way to the dac (1.0: straight).
    pub fn effect(&self, pack: Pack, config: Value, tap: f64) -> Value {
        let mut modules = vec![json!({ "id": "fx", "type": "mod_fx", "config": config })];
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
        doc["developments"] = json!([{ "name": "mod_fx", "path": pack.development_path() }]);
        doc
    }

    /// The same sources with each pack audio input routed to its dac side.
    pub fn dry(&self) -> Value {
        self.doc(Vec::new(), Vec::new(), false)
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

/// Chunk lengths that are not multiples of any block size, so that the
/// edges of control writes land at every offset in a block.
pub const CHUNKS: [usize; 7] = [37, 61, 64, 91, 130, 50, 113];

/// Renders `frames` in chunks cycling through `CHUNKS`; `before` runs ahead
/// of each chunk with the frame it starts at (to write controls).
pub fn render_chunked(
    engine: &mut RenderEngine,
    frames: usize,
    mut before: impl FnMut(usize, &RenderEngine),
) -> (Vec<f32>, Vec<f32>) {
    let (mut left, mut right) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
    let mut chunk = 0;
    while left.len() < frames {
        before(left.len(), engine);
        let n = CHUNKS[chunk % CHUNKS.len()].min(frames - left.len());
        let (l, r) = render(engine, n);
        left.extend(l);
        right.extend(r);
        chunk += 1;
    }
    (left, right)
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

/// The largest difference between neighbouring samples.
pub fn max_step(x: &[f32]) -> f32 {
    x.windows(2).fold(0.0, |m, w| m.max((w[1] - w[0]).abs()))
}

/// RMS of the difference of `a` and `b` over `range`, relative to the RMS
/// of `a` there.
pub fn relative_difference(a: &[f32], b: &[f32], range: std::ops::Range<usize>) -> f32 {
    let diff: Vec<f32> = range.clone().map(|n| a[n] - b[n]).collect();
    rms(&diff) / rms(&a[range])
}
