//! Acceptance tests for the dry and wet controls of the granular, chorus,
//! flanger, stereo-delay, ping-pong-delay and tape-delay packs. Each loads
//! the pack's development file against the pinned core and checks, through
//! public interfaces only:
//!
//! 1. both controls are described with a 0 to 1 range,
//! 2. a write above 1 lands at 1 and sounds as a write of 1,
//! 3. values inside 0 to 1 are unchanged: defaults and in-range writes read
//!    back as set,
//! 4. the pack version is a later date than the one before the change.
//!
//! The criteria are quoted by number in each test's name or comment.

use std::path::PathBuf;

use fugue::{
    AudioBackend, ControlKind, ControlMeta, ControlValue, DescribeModuleQuery, InventionBuilder,
    RenderEngine,
};
use serde_json::{json, Value};

const SR: u32 = 48_000;
const KEYS: [&str; 2] = ["dry", "wet"];

/// One pack under test.
struct Pack {
    dir: &'static str,
    file: &'static str,
    /// The development alias. It must not be a builtin module type name.
    alias: &'static str,
    /// Default dry and wet levels before the change.
    defaults: (f32, f32),
    /// The version before the change, as (year, month, patch).
    old_version: (u32, u32, u32),
}

const PACKS: [Pack; 6] = [
    Pack {
        dir: "fugue.effects.granular",
        file: "granular.development.json",
        alias: "granular_fx",
        defaults: (1.0, 0.5),
        old_version: (2026, 11, 1),
    },
    Pack {
        dir: "fugue.effects.chorus",
        file: "chorus.development.json",
        alias: "chorus_fx",
        defaults: (0.7, 0.5),
        old_version: (2026, 11, 1),
    },
    Pack {
        dir: "fugue.effects.flanger",
        file: "flanger.development.json",
        alias: "flanger_fx",
        defaults: (0.5, 0.5),
        old_version: (2026, 11, 2),
    },
    Pack {
        dir: "fugue.effects.stereo-delay",
        file: "delay.development.json",
        alias: "stereo_delay_fx",
        defaults: (1.0, 0.5),
        old_version: (2026, 11, 1),
    },
    Pack {
        dir: "fugue.effects.ping-pong-delay",
        file: "delay.development.json",
        alias: "ping_pong_delay_fx",
        defaults: (1.0, 0.5),
        old_version: (2026, 11, 1),
    },
    Pack {
        dir: "fugue.effects.tape-delay",
        file: "delay.development.json",
        alias: "tape_delay_fx",
        defaults: (1.0, 0.5),
        old_version: (2026, 11, 1),
    },
];

impl Pack {
    fn path(&self, file: &str) -> PathBuf {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packs")
            .join(self.dir)
            .join(file);
        (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
    }

    fn default(&self, key: &str) -> f32 {
        if key == "dry" {
            self.defaults.0
        } else {
            self.defaults.1
        }
    }

    /// The pack as module `fx`, fed seeded noise on both inputs when
    /// `source`, into a dac.
    fn doc(&self, source: bool) -> Value {
        let development = self.path(self.file).to_string_lossy().into_owned();
        let mut modules = vec![
            json!({ "id": "fx", "type": self.alias, "config": {} }),
            json!({ "id": "dac", "type": "dac", "config": { "soft_clip": false } }),
        ];
        let mut connections = Vec::new();
        for side in ["audio_left", "audio_right"] {
            connections
                .push(json!({ "from": "fx", "from_port": side, "to": "dac", "to_port": side }));
        }
        if source {
            for (id, seed, side) in [("nl", 41, "audio_left"), ("nr", 42, "audio_right")] {
                modules.push(json!({ "id": id, "type": "noise",
                    "config": { "seed": seed, "level": 0.3 } }));
                connections
                    .push(json!({ "from": id, "from_port": "white", "to": "fx", "to_port": side }));
            }
        }
        json!({
            "version": "1.0.0",
            "developments": [{ "name": self.alias, "path": development }],
            "modules": modules,
            "connections": connections
        })
    }

    fn engine(&self, source: bool) -> RenderEngine {
        let mut engine = RenderEngine::new(SR);
        (engine.load_json(&self.doc(source).to_string()))
            .unwrap_or_else(|err| panic!("{}: loads: {err}", self.dir));
        engine
    }
}

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

fn read(engine: &RenderEngine, key: &str) -> f32 {
    match engine.get_control("fx", key).expect("reads") {
        ControlValue::Number(number) => number,
        other => panic!("{key} reads {other:?}"),
    }
}

fn write(engine: &mut RenderEngine, key: &str, value: f32) {
    (engine.set_control("fx", key, ControlValue::Number(value))).expect("accepted");
}

/// Renders `frames` stereo frames, interleaved.
fn render(engine: &mut RenderEngine, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; frames * 2];
    engine.render_interleaved(&mut out).expect("renders");
    out
}

type Render = Box<dyn FnMut(&mut [f32], &mut [f32]) + Send>;

/// A backend that never pulls audio: enough to describe a live invention.
struct Idle;

impl AudioBackend for Idle {
    fn sample_rate(&self) -> u32 {
        SR
    }
    fn start(&mut self, _render: Render) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn stop(&mut self) {}
}

/// Criterion 1: `describe_module`, for the instance and for the type, and
/// `list_controls` report 0 to 1 for both controls.
#[test]
fn criterion_1_dry_and_wet_are_described_with_a_zero_to_one_range() {
    for pack in &PACKS {
        let invention = serde_json::from_value(pack.doc(false)).expect("parses");
        let (runtime, _) = InventionBuilder::new(SR).build(invention).expect("builds");
        let running = runtime.start_with_backend(Idle).expect("starts");
        let queries = [
            DescribeModuleQuery {
                module_id: Some("fx".into()),
                ..Default::default()
            },
            DescribeModuleQuery {
                module_type: Some(pack.alias.into()),
                config: Some(json!({})),
                ..Default::default()
            },
        ];
        for (which, query) in ["instance", "type"].into_iter().zip(queries) {
            let description = running.describe_module(&query).expect("describes");
            for key in KEYS {
                let (min, max) = bounds(&description.info.controls, key);
                assert!(
                    near(min, 0.0) && near(max, 1.0),
                    "{} {key} described by {which}: {min} to {max}",
                    pack.dir
                );
            }
        }
    }
}

#[test]
fn criterion_1_list_controls_reports_a_zero_to_one_range() {
    for pack in &PACKS {
        let engine = pack.engine(false);
        let controls = engine.list_controls(Some("fx")).expect("lists").remove(0).1;
        for key in KEYS {
            let (min, max) = bounds(&controls, key);
            assert!(
                near(min, 0.0) && near(max, 1.0),
                "{} {key}: {min} to {max}",
                pack.dir
            );
        }
    }
}

/// Criterion 2: a write of 1.5 reads back 1.0. Larger writes are held at
/// the same end, and a write below 0 at the other (the range's lower end).
#[test]
fn criterion_2_a_write_above_the_range_reads_back_at_one() {
    for pack in &PACKS {
        for key in KEYS {
            for (written, held) in [(1.5, 1.0), (10.0, 1.0), (-0.5, 0.0)] {
                let mut engine = pack.engine(false);
                write(&mut engine, key, written);
                render(&mut engine, 512);
                assert!(
                    near(read(&engine, key), held),
                    "{} {key} after writing {written}: {}",
                    pack.dir,
                    read(&engine, key)
                );
            }
        }
    }
}

/// Noise through the pack for two seconds with `key` written to `value`.
fn heard(pack: &Pack, key: &str, value: f32) -> Vec<f32> {
    let mut engine = pack.engine(true);
    write(&mut engine, key, value);
    render(&mut engine, SR as usize * 2)
}

/// Guards the next test: each control is audible in the render window, so
/// a render that matches at 1.0 and 1.5 is a real match and not two
/// renders that never depended on the control.
#[test]
fn dry_and_wet_are_audible_in_the_render_window() {
    for pack in &PACKS {
        for key in KEYS {
            let top = heard(pack, key, 1.0);
            assert!(
                top.iter().any(|s| *s != 0.0),
                "{} {key}: the render is silent",
                pack.dir
            );
            assert!(
                heard(pack, key, 0.5) != top,
                "{} {key}: 0.5 and 1.0 sound the same",
                pack.dir
            );
        }
    }
}

/// Criterion 2: a write of 1.5 sounds as a write of 1.0, to the byte, on
/// every output frame, for noise through the pack.
#[test]
fn criterion_2_a_write_of_one_and_a_half_sounds_as_a_write_of_one() {
    for pack in &PACKS {
        for key in KEYS {
            assert!(
                heard(pack, key, 1.5) == heard(pack, key, 1.0),
                "{} {key}: 1.5 sounds different from 1.0",
                pack.dir
            );
        }
    }
}

/// Criterion 3: each control starts at the level it had before the change.
#[test]
fn criterion_3_defaults_are_unchanged() {
    for pack in &PACKS {
        let engine = pack.engine(false);
        for key in KEYS {
            assert!(
                near(read(&engine, key), pack.default(key)),
                "{} {key} starts at {}",
                pack.dir,
                read(&engine, key)
            );
        }
    }
}

/// Criterion 3: writes inside 0 to 1 read back as written, and the other
/// control keeps its level.
#[test]
fn criterion_3_in_range_writes_read_back_unchanged() {
    for pack in &PACKS {
        for key in KEYS {
            let other = if key == "dry" { "wet" } else { "dry" };
            for value in [0.0, 0.25, 0.5, 0.9, 1.0] {
                let mut engine = pack.engine(false);
                write(&mut engine, key, value);
                render(&mut engine, 512);
                assert!(
                    near(read(&engine, key), value),
                    "{} {key} after writing {value}: {}",
                    pack.dir,
                    read(&engine, key)
                );
                assert!(
                    near(read(&engine, other), pack.default(other)),
                    "{} {other} moved when {key} was written {value}",
                    pack.dir
                );
            }
        }
    }
}

/// Criterion 3: a render with in-range writes is repeatable and follows the
/// write: a second engine given the same writes renders the same bytes.
#[test]
fn criterion_3_in_range_renders_are_repeatable() {
    const FRAMES: usize = SR as usize / 2;
    for pack in &PACKS {
        let heard = || {
            let mut engine = pack.engine(true);
            write(&mut engine, "dry", 0.4);
            write(&mut engine, "wet", 0.8);
            render(&mut engine, FRAMES)
        };
        assert!(heard() == heard(), "{}: renders differ", pack.dir);
    }
}

fn parse_version(text: &str) -> (u32, u32, u32) {
    let parts: Vec<&str> = text.split('.').collect();
    assert_eq!(parts.len(), 3, "{text}: not YYYY.M.P");
    for part in &parts {
        assert!(
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()),
            "{text}: not numeric"
        );
        assert!(
            part.len() == 1 || !part.starts_with('0'),
            "{text}: leading zero"
        );
    }
    assert_eq!(parts[0].len(), 4, "{text}: year is not four digits");
    let number = |part: &str| part.parse::<u32>().expect("numeric");
    let (year, month, patch) = (number(parts[0]), number(parts[1]), number(parts[2]));
    assert!((1..=12).contains(&month), "{text}: month out of 1 to 12");
    (year, month, patch)
}

/// Criterion 4: the manifest version is a valid YYYY.M.P later than the
/// version before the change.
#[test]
fn criterion_4_pack_versions_are_bumped_by_date() {
    for pack in &PACKS {
        let text = std::fs::read_to_string(pack.path("fugue.pkg.json")).expect("manifest reads");
        let manifest: Value = serde_json::from_str(&text).expect("manifest parses");
        let version = manifest["version"].as_str().expect("version is a string");
        let parsed = parse_version(version);
        assert!(
            parsed > pack.old_version,
            "{}: {version} is not later than {:?}",
            pack.dir,
            pack.old_version
        );
    }
}
