//! The chorus and flanger `rate` and the compressor `mix` declare their own
//! ranges, narrower than the controls they reach: each pack, loaded against
//! the pinned core, lists and describes the range, holds every write and
//! configured value within it, and starts at its old default.

use std::path::PathBuf;

use fugue::{
    AudioBackend, ControlKind, ControlMeta, ControlValue, DescribeModuleQuery, InventionBuilder,
    RenderEngine,
};
use serde_json::{json, Value};

const SR: u32 = 48_000;

/// A pack's ranged control: the declared range and its default.
struct Ranged {
    pack: &'static str,
    kind: &'static str,
    file: &'static str,
    key: &'static str,
    range: (f32, f32),
    default: f32,
}

const RANGED: [Ranged; 3] = [
    Ranged {
        pack: "fugue.effects.chorus",
        kind: "chorus",
        file: "chorus.development.json",
        key: "rate",
        range: (0.05, 10.0),
        default: 0.8,
    },
    Ranged {
        pack: "fugue.effects.flanger",
        kind: "flanger",
        file: "flanger.development.json",
        key: "rate",
        range: (0.05, 10.0),
        default: 0.25,
    },
    Ranged {
        pack: "fugue.effects.compressor",
        kind: "compressor",
        file: "compressor.development.json",
        key: "mix",
        range: (0.0, 1.0),
        default: 1.0,
    },
];

fn development_path(pack: &str, file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs")
        .join(pack)
        .join(file);
    let path = (path.canonicalize()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    path.to_string_lossy().into_owned()
}

/// The pack as module `fx`, built with `config`, fed silence into a dac.
fn doc(pack: &str, kind: &str, file: &str, config: Value) -> Value {
    json!({
        "version": "1.0.0",
        "developments": [{ "name": kind, "path": development_path(pack, file) }],
        "modules": [
            { "id": "fx", "type": kind, "config": config },
            { "id": "dac", "type": "dac", "config": { "soft_clip": false } }
        ],
        "connections": [
            { "from": "fx", "from_port": "audio_left", "to": "dac", "to_port": "audio_left" },
            { "from": "fx", "from_port": "audio_right", "to": "dac", "to_port": "audio_right" }
        ]
    })
}

fn engine(doc: &Value) -> RenderEngine {
    let mut engine = RenderEngine::new(SR);
    (engine.load_json(&doc.to_string())).unwrap_or_else(|err| panic!("loads: {err}"));
    engine
}

fn bounds(controls: &[ControlMeta], key: &str) -> (f32, f32) {
    let meta = (controls.iter().find(|c| c.key == key)).unwrap_or_else(|| panic!("no {key}"));
    match meta.kind {
        ControlKind::Number { min, max, .. } => (min, max),
        _ => panic!("{key} is not a number control"),
    }
}

fn read(engine: &RenderEngine, key: &str) -> f32 {
    let value = engine.get_control("fx", key).expect("reads");
    match value {
        ControlValue::Number(number) => number,
        other => panic!("{key} reads {other:?}"),
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-6 * b.abs().max(1.0)
}

#[test]
fn each_pack_lists_its_declared_range_and_starts_at_its_default() {
    for Ranged {
        pack,
        kind,
        file,
        key,
        range,
        default,
    } in RANGED
    {
        let engine = engine(&doc(pack, kind, file, json!({})));
        let controls = engine.list_controls(Some("fx")).expect("lists").remove(0).1;
        let (min, max) = bounds(&controls, key);
        assert!(
            near(min, range.0) && near(max, range.1),
            "{pack} {key}: {min} to {max}"
        );
        assert!(near(read(&engine, key), default), "{pack} {key} default");
    }
}

#[test]
fn writes_outside_the_range_read_back_at_its_ends() {
    for Ranged {
        pack,
        kind,
        file,
        key,
        range,
        ..
    } in RANGED
    {
        let mut engine = engine(&doc(pack, kind, file, json!({})));
        for (written, held) in [(range.1 * 10.0, range.1), (range.0 - 1.0, range.0)] {
            engine
                .set_control("fx", key, ControlValue::Number(written))
                .expect("accepted");
            let mut out = vec![0.0f32; 512];
            engine.render_interleaved(&mut out).expect("renders");
            assert!(
                near(read(&engine, key), held),
                "{pack} {key} after writing {written}"
            );
        }
    }
}

#[test]
fn a_configured_value_outside_the_range_is_held_within_it() {
    for Ranged {
        pack,
        kind,
        file,
        key,
        range,
        ..
    } in RANGED
    {
        let engine = engine(&doc(pack, kind, file, json!({ key: range.1 * 10.0 })));
        assert!(
            near(read(&engine, key), range.1),
            "{pack} {key} configured high"
        );
    }
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

#[test]
fn describe_module_reports_the_range_for_the_instance_and_the_type() {
    for Ranged {
        pack,
        kind,
        file,
        key,
        range,
        ..
    } in RANGED
    {
        let invention = serde_json::from_value(doc(pack, kind, file, json!({}))).expect("parses");
        let (runtime, _) = InventionBuilder::new(SR).build(invention).expect("builds");
        let running = runtime.start_with_backend(Idle).expect("starts");
        let queries = [
            DescribeModuleQuery {
                module_id: Some("fx".into()),
                ..Default::default()
            },
            DescribeModuleQuery {
                module_type: Some(kind.into()),
                config: Some(json!({})),
                ..Default::default()
            },
        ];
        for query in queries {
            let description = running.describe_module(&query).expect("describes");
            let (min, max) = bounds(&description.info.controls, key);
            assert!(
                near(min, range.0) && near(max, range.1),
                "{pack} {key}: {min} to {max}"
            );
        }
    }
}
