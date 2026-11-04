//! Shared rig: inventions around the compressor's development, rendering,
//! the pinned static curve, and gain measurements.

use std::path::PathBuf;

use fugue::{ControlValue, RenderEngine};
use serde_json::{json, Value};

pub const SR: u32 = 48_000;
pub const PACK: &str = "fugue.effects.compressor";
/// Frames rendered before a steady measurement: 0.5 s, 50 attack time
/// constants at the default attack (0.01 s), far past any 2.7 ms or 20 ms
/// control glide.
pub const SETTLE: usize = SR as usize / 2;
/// Tolerance on a steady gain in dB: the curve is pinned to within ±0.1 dB
/// of its design (`gain_curve`), and dB to linear to 1e-5 relative (about
/// 1e-4 dB); 0.11 dB covers both.
pub const CURVE_DB: f64 = 0.11;

/// The pinned defaults: threshold, ratio, knee in dB.
pub const T: f64 = -20.0;
pub const R: f64 = 4.0;
pub const W: f64 = 6.0;

pub fn pack_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs")
        .join(PACK)
}

/// The pack's development file, as an absolute path.
pub fn development_path() -> String {
    let path = pack_dir().join("compressor.development.json");
    let path = (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    path.to_string_lossy().into_owned()
}

pub fn conn(from: &str, from_port: &str, to: &str, to_port: &str) -> Value {
    json!({ "from": from, "from_port": from_port, "to": to, "to_port": to_port })
}

/// Sources and the pack inputs they feed. `effect` builds the invention
/// with the pack as module `fx`; `dry` wires the same sources straight to
/// the dac (each `audio_left`/`audio_right` feed to that dac side, the
/// `sidechain` dropped), the reference for "the input".
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

    /// This rig's sources and feeds plus `other`'s.
    pub fn with(&self, other: &Rig) -> Self {
        let mut rig = self.clone();
        rig.modules.extend(other.modules.iter().cloned());
        rig.connections.extend(other.connections.iter().cloned());
        rig.feeds.extend(other.feeds.iter().cloned());
        rig
    }

    fn level(mut self, id: &str, source: &str, port: &str, amp: f64) -> Self {
        let config = json!({ "level": amp });
        self.modules
            .push(json!({ "id": id, "type": "vca", "config": config }));
        self.connections.push(conn(source, port, id, "audio"));
        self
    }

    /// A gate of 1 from the first sample, held `on` seconds of every 3 s
    /// (a 20 bpm clock), at `amp`: output `<id>.audio`.
    pub fn gated(mut self, id: &str, amp: f64, on: f64) -> Self {
        let clk = format!("{id}_clk");
        let config = json!({ "bpm": 20.0, "gate_length": on / 3.0 });
        self.modules
            .push(json!({ "id": clk, "type": "clock", "config": config }));
        self.level(id, &clk, "beat", amp)
    }

    /// A constant `amp` from the first sample, for 59 s (a 1 bpm clock's
    /// gate at 0.99 of the beat): output `<id>.audio`.
    pub fn constant(mut self, id: &str, amp: f64) -> Self {
        let clk = format!("{id}_clk");
        let config = json!({ "bpm": 1.0, "gate_length": 0.99 });
        self.modules
            .push(json!({ "id": clk, "type": "clock", "config": config }));
        self.level(id, &clk, "beat", amp)
    }

    /// An oscillator of `waveform` at `freq` Hz and `amp` peak: output
    /// `<id>.audio`.
    pub fn osc(mut self, id: &str, waveform: &str, freq: f64, amp: f64) -> Self {
        let osc = format!("{id}_osc");
        let config = json!({ "waveform": waveform, "frequency": freq });
        self.modules
            .push(json!({ "id": osc, "type": "oscillator", "config": config }));
        self.level(id, &osc, "audio", amp)
    }

    /// Seeded white noise, uniform within ±`level`: output `<id>.white`.
    pub fn noise(mut self, id: &str, seed: u32, level: f64) -> Self {
        let config = json!({ "seed": seed, "level": level });
        self.modules
            .push(json!({ "id": id, "type": "noise", "config": config }));
        self
    }

    /// Patches `from.port` into the pack's input `input`.
    pub fn feed(mut self, from: &str, port: &str, input: &str) -> Self {
        self.feeds.push((from.into(), port.into(), input.into()));
        self
    }

    /// `from.port` into both audio inputs.
    pub fn feed_both(self, from: &str, port: &str) -> Self {
        self.feed(from, port, "audio_left")
            .feed(from, port, "audio_right")
    }

    fn doc(&self, mut modules: Vec<Value>, mut connections: Vec<Value>, fx: bool) -> Value {
        modules.extend(self.modules.iter().cloned());
        modules.push(json!({ "id": "dac", "type": "dac", "config": { "soft_clip": false } }));
        connections.extend(self.connections.iter().cloned());
        for (from, port, input) in &self.feeds {
            if fx {
                connections.push(conn(from, port, "fx", input));
            } else if input != "sidechain" {
                connections.push(conn(from, port, "dac", input));
            }
        }
        json!({ "version": "1.0.0", "modules": modules, "connections": connections })
    }

    /// The invention with the pack as module `fx` (built with `config`),
    /// its outputs straight to the dac.
    pub fn effect(&self, config: Value) -> Value {
        let modules = vec![json!({ "id": "fx", "type": "compressor", "config": config })];
        let connections = vec![
            conn("fx", "audio_left", "dac", "audio_left"),
            conn("fx", "audio_right", "dac", "audio_right"),
        ];
        let mut doc = self.doc(modules, connections, true);
        doc["developments"] = json!([{ "name": "compressor", "path": development_path() }]);
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

/// Renders `rig` through the pack (with `config`) and dry, `frames` each:
/// ((wet left, wet right), (dry left, dry right)).
pub type Sides = (Vec<f32>, Vec<f32>);
pub fn wet_and_dry(rig: &Rig, config: Value, frames: usize) -> (Sides, Sides) {
    (
        render_doc(&rig.effect(config), frames),
        render_doc(&rig.dry(), frames),
    )
}

pub fn set(engine: &RenderEngine, key: &str, value: impl Into<ControlValue>) {
    (engine.set_control("fx", key, value.into()))
        .unwrap_or_else(|err| panic!("fx.{key} accepts the value: {err}"));
}

pub fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

/// Largest one-sample step in `x`.
pub fn max_step(x: &[f32]) -> f32 {
    x.windows(2).fold(0.0, |m, w| m.max((w[1] - w[0]).abs()))
}

pub fn db(level: f64) -> f64 {
    20.0 * level.log10()
}

pub fn from_db(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

/// The pinned curve's gain change c(x) = y - x in dB, for a detector level
/// `x` dBFS, threshold `t`, ratio `r` and knee `w`.
pub fn curve(x: f64, t: f64, r: f64, w: f64) -> f64 {
    let y = if 2.0 * (x - t) < -w {
        x
    } else if w > 0.0 && 2.0 * (x - t).abs() <= w {
        x + (1.0 / r - 1.0) * (x - t + w / 2.0).powi(2) / (2.0 * w)
    } else {
        t + (x - t) / r
    };
    y - x
}

/// c(x) at the default threshold, ratio and knee, for a linear level.
pub fn default_curve(level: f64) -> f64 {
    curve(db(level), T, R, W)
}

/// The least-squares gain from `dry` to `wet` over `range`.
pub fn gain(wet: &[f32], dry: &[f32], range: std::ops::Range<usize>) -> f64 {
    let (mut wd, mut dd) = (0.0f64, 0.0f64);
    for n in range {
        wd += f64::from(wet[n]) * f64::from(dry[n]);
        dd += f64::from(dry[n]) * f64::from(dry[n]);
    }
    assert!(dd > 0.0, "the dry input is heard");
    wd / dd
}

/// Asserts that over `range` `wet` is `dry` times one steady gain, within
/// `rel` of the dry peak at every sample, and returns that gain.
pub fn steady_gain(
    what: &str,
    wet: &[f32],
    dry: &[f32],
    range: std::ops::Range<usize>,
    rel: f64,
) -> f64 {
    let g = gain(wet, dry, range.clone());
    let bound = rel * f64::from(peak(&dry[range.clone()]));
    for n in range {
        let err = (f64::from(wet[n]) - g * f64::from(dry[n])).abs();
        assert!(
            err <= bound,
            "{what}: sample {n} is {} where a steady gain {g} gives {} (off by {err}, bound {bound})",
            wet[n],
            g * f64::from(dry[n])
        );
    }
    g
}

/// Asserts a gain in dB is within `tol` dB of `want`.
pub fn assert_db(what: &str, got_db: f64, want_db: f64, tol: f64) {
    assert!(
        (got_db - want_db).abs() <= tol,
        "{what}: gain {got_db:.4} dB, want {want_db:.4} dB ± {tol}"
    );
}
