//! Inventions around the granular development, rendering, and measurements.

use std::path::PathBuf;

use fugue::{ControlValue, RenderEngine};
use serde_json::{json, Value};

pub const SR: u32 = 48_000;
pub const GRANULAR: &str = "fugue.effects.granular";

pub fn pack_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs")
        .join(GRANULAR)
}

/// The pack's development file, as an absolute path.
fn development_path() -> String {
    let path = pack_dir().join("granular.development.json");
    let path = (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    path.to_string_lossy().into_owned()
}

fn conn(from: &str, from_port: &str, to: &str, to_port: &str) -> Value {
    json!({ "from": from, "from_port": from_port, "to": to, "to_port": to_port })
}

/// `base` with the keys of the object `extra` set on it.
pub fn with(base: &Value, extra: Value) -> Value {
    let mut merged = base.clone();
    for (key, value) in extra.as_object().expect("an object") {
        merged[key] = value.clone();
    }
    merged
}

/// Sources and which input each feeds, as `(module, port)` per side, and
/// clocks patched into the effect's `trigger` or `clock` input.
#[derive(Clone)]
pub struct Feed {
    modules: Vec<Value>,
    wires: Vec<Value>,
    left: (String, String),
    right: (String, String),
    clocks: Vec<(String, String)>,
}

fn noise(id: &str, seed: u32, level: f64) -> Value {
    json!({ "id": id, "type": "noise", "config": { "seed": seed, "level": level } })
}

impl Feed {
    /// Seeded white noise at `level`, a different seed on each side.
    pub fn noise_pair(level: f64) -> Self {
        Feed {
            modules: vec![noise("nl", 41, level), noise("nr", 42, level)],
            wires: vec![],
            left: ("nl".into(), "white".into()),
            right: ("nr".into(), "white".into()),
            clocks: vec![],
        }
    }

    /// The `noise_pair` brought `decibels` up with a `gain` module on each
    /// side, so the input can run past full scale (noise levels stop at 1).
    pub fn hot_noise_pair(level: f64, decibels: f64) -> Self {
        let mut feed = Self::noise_pair(level);
        for (side, source) in [("l", &mut feed.left), ("r", &mut feed.right)] {
            let id = format!("hot_{side}");
            let config = json!({ "decibels": decibels });
            feed.modules
                .push(json!({ "id": id, "type": "gain", "config": config }));
            feed.wires.push(conn(&source.0, &source.1, &id, "audio"));
            *source = (id, "audio".into());
        }
        feed
    }

    /// One full-scale noise into both inputs.
    pub fn noise_both() -> Self {
        let source = ("n".to_string(), "white".to_string());
        Feed {
            modules: vec![noise("n", 43, 1.0)],
            left: source.clone(),
            right: source,
            ..Self::noise_pair(0.0)
        }
    }

    /// A sine of `freq` Hz at `amp` peak into both inputs. Its level is
    /// module `sine_level`, so writing 0 ends a burst.
    pub fn sine(freq: f64, amp: f64) -> Self {
        let osc = json!({ "waveform": "sine", "frequency": freq });
        let source = ("sine_level".to_string(), "audio".to_string());
        Feed {
            modules: vec![
                json!({ "id": "sine", "type": "oscillator", "config": osc }),
                json!({ "id": "sine_level", "type": "vca", "config": { "level": amp } }),
            ],
            wires: vec![conn("sine", "audio", "sine_level", "audio")],
            left: source.clone(),
            right: source,
            ..Self::noise_pair(0.0)
        }
    }

    /// The same feed with a clock at `bpm` patched into `port`.
    pub fn clocked(&self, port: &str, bpm: f64) -> Self {
        let id = format!("clock_{port}");
        let mut feed = self.clone();
        let config = json!({ "bpm": bpm });
        feed.modules
            .push(json!({ "id": id, "type": "clock", "config": config }));
        feed.clocks.push((id, port.into()));
        feed
    }

    /// The invention with `node` (`(id, type, config)`) between the feed and
    /// the dac, or the feed straight to the dac. `tap` scales the outputs on
    /// the way to the dac, so the dac cannot hide an overshoot.
    fn build(&self, node: Option<(&str, &str, Value)>, devs: Option<Value>, tap: f64) -> Value {
        let mut modules = self.modules.clone();
        modules.push(json!({ "id": "dac", "type": "dac", "config": { "soft_clip": false } }));
        let mut connections = self.wires.clone();
        let sides = [("audio_left", &self.left), ("audio_right", &self.right)];
        for (side, (from, port)) in sides {
            let Some((id, kind, config)) = &node else {
                connections.push(conn(from, port, "dac", side));
                continue;
            };
            if side == "audio_left" {
                modules.push(json!({ "id": id, "type": kind, "config": config }));
            }
            connections.push(conn(from, port, id, side));
            if tap == 1.0 {
                connections.push(conn(id, side, "dac", side));
            } else {
                let tap_id = format!("tap_{side}");
                modules.push(json!({ "id": tap_id, "type": "vca", "config": { "level": tap } }));
                connections.push(conn(id, side, &tap_id, "audio"));
                connections.push(conn(&tap_id, "audio", "dac", side));
            }
        }
        if let Some((id, ..)) = &node {
            for (clock, port) in &self.clocks {
                connections.push(conn(clock, "beat", id, port));
            }
        }
        let mut doc = json!({ "version": "1.0.0", "modules": modules, "connections": connections });
        if let Some(devs) = devs {
            doc["developments"] = devs;
        }
        doc
    }

    /// The invention with the pack as module `fx` (built with `config`).
    pub fn effect(&self, config: Value, tap: f64) -> Value {
        let devs = json!([{ "name": "granular_fx", "path": development_path() }]);
        self.build(Some(("fx", "granular_fx", config)), Some(devs), tap)
    }

    /// The same feed through core's bare `granular` module `g`, built with
    /// the pack's fixed `buffer_length` 4.0 and `seed` 1 and `config`.
    pub fn bare(&self, config: &Value) -> Value {
        let config = with(config, json!({ "buffer_length": 4.0, "seed": 1 }));
        self.build(Some(("g", "granular", config)), None, 1.0)
    }

    /// The feed straight to the dac: "the input".
    pub fn dry(&self) -> Value {
        self.build(None, None, 1.0)
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

pub fn set_on(engine: &RenderEngine, id: &str, key: &str, value: impl Into<ControlValue>) {
    (engine.set_control(id, key, value.into()))
        .unwrap_or_else(|err| panic!("{id}.{key} accepts the value: {err}"));
}

/// A live write to the pack, module `fx`.
pub fn set(engine: &RenderEngine, key: &str, value: impl Into<ControlValue>) {
    set_on(engine, "fx", key, value);
}

/// The pack's static soft limit on each output, in f32:
/// `0.5469 * clamp(kg * x, -1, 1) + 0.453 * clamp(tg * x, -1, 1)` with
/// `kg` +3.011048 dB and `tg` -6.0206 dB.
pub fn limit(x: f32) -> f32 {
    let gain = |decibels: f64| 10f64.powf(decibels / 20.0) as f32;
    let (kg, tg) = (gain(3.011048), gain(-6.0206));
    0.5469 * (kg * x).clamp(-1.0, 1.0) + 0.453 * (tg * x).clamp(-1.0, 1.0)
}

pub fn limited(x: &[f32]) -> Vec<f32> {
    x.iter().map(|v| limit(*v)).collect()
}

/// Asserts `a` and `b` agree within `tolerance` per sample over their
/// length, naming the first difference.
pub fn assert_near(what: &str, a: &[f32], b: &[f32], tolerance: f32) {
    assert_eq!(a.len(), b.len(), "{what}: lengths");
    if let Some(n) = (0..a.len()).find(|&n| (a[n] - b[n]).abs() > tolerance) {
        panic!(
            "{what}: sample {n} is {} where the reference is {} (tolerance {tolerance})",
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
