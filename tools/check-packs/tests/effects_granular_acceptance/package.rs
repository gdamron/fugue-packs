//! The package and its control surface: the manifest the spec pins,
//! check-packs validation, every control listed with its default, starting
//! values from the development's config, and live writes by name.

use std::fs;
use std::process::Command;

use fugue::ControlValue;
use serde_json::{json, Value};

use super::rig::*;

#[test]
fn check_packs_validates_the_repository_with_the_granular_pack() {
    assert!(pack_dir().is_dir(), "{GRANULAR} is in packs/");
    let root = pack_dir().join("../..");
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(&root)
        .output()
        .expect("check-packs runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "check-packs failed: {stdout}{stderr}");
}

/// "`id` `fugue.effects.granular`, `version` `2026.11.0`, `kind`
/// `development`, `license` `MIT`, `entry.development`
/// `granular.development.json`; it parses with `fugue::pkg::parse_path`", and
/// the directory holds `LICENSE`.
#[test]
fn manifest_declares_a_development_entry_under_mit_at_this_release() {
    let path = pack_dir().join("fugue.pkg.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let manifest: Value = serde_json::from_str(&text).expect("manifest is JSON");
    assert_eq!(manifest["id"], GRANULAR);
    assert_eq!(manifest["version"], "2026.11.0");
    assert_eq!(manifest["kind"], "development");
    assert_eq!(manifest["license"], "MIT");
    assert_eq!(
        manifest["entry"]["development"],
        "granular.development.json"
    );
    let license = pack_dir().join("LICENSE");
    let license = fs::read_to_string(&license).expect("has a LICENSE");
    assert!(license.contains("MIT License"), "the LICENSE is MIT text");
    fugue::pkg::parse_path(&path).unwrap_or_else(|e| panic!("fugue refuses it: {e}"));
}

fn fx(config: Value) -> fugue::RenderEngine {
    engine(&Feed::noise_pair(0.5).effect(config, 1.0))
}

fn number(engine: &fugue::RenderEngine, key: &str) -> f32 {
    match engine.get_control("fx", key) {
        Ok(ControlValue::Number(n)) => n,
        other => panic!("{key} reads as a number, got {other:?}"),
    }
}

/// Every control with its default (the mixer-level controls are 0-1 in use).
const NUMBERS: [(&str, f32); 12] = [
    ("position", 0.5),
    ("size", 0.5),
    ("pitch", 0.0),
    ("density", 0.7),
    ("texture", 0.5),
    ("spread", 0.0),
    ("feedback", 0.0),
    ("dry", 1.0),
    ("wet", 0.5),
    ("reverb", 0.25),
    ("reverb_size", 0.7),
    ("reverb_decay", 0.7),
];

/// "every control is listed with its default": by name, and read back at
/// its default (numbers to within 1e-6).
#[test]
fn every_control_is_listed_by_name_with_its_default() {
    let engine = fx(json!({}));
    let listed = engine.list_controls(Some("fx")).expect("fx lists controls");
    let keys: Vec<String> = (listed.iter())
        .flat_map(|(_, c)| c.iter().map(|m| m.key.clone()))
        .collect();
    let names = NUMBERS.iter().map(|(k, _)| *k);
    let others = ["reverb_damping", "mode", "freeze", "sync"];
    for key in names.chain(others) {
        assert!(keys.iter().any(|k| k == key), "lists {key}: {keys:?}");
    }
    for (key, want) in NUMBERS {
        let got = number(&engine, key);
        assert!((got - want).abs() <= 1e-6, "{key}: {got}, want {want}");
    }
    assert!((number(&engine, "reverb_damping") - 0.5).abs() <= 1e-6);
    let get = |key: &str| engine.get_control("fx", key).expect("readable");
    assert_eq!(get("mode"), ControlValue::from("granular"));
    assert_eq!(get("freeze"), ControlValue::Bool(false));
    assert_eq!(get("sync"), ControlValue::Bool(false));
}

/// "every control may be given in the development's config as its starting
/// value": each starts there, with its own value.
#[test]
fn every_control_can_start_from_the_config() {
    let engine = fx(json!({
        "mode": "stretch", "position": 0.25, "size": 0.75, "pitch": 7.0, "density": 0.3,
        "texture": 0.8, "freeze": true, "spread": 0.4, "feedback": 0.2, "sync": true,
        "dry": 0.3, "wet": 0.6, "reverb": 0.15, "reverb_size": 0.4, "reverb_decay": 0.9,
        "reverb_damping": 0.1
    }));
    let want = [
        ("position", 0.25),
        ("size", 0.75),
        ("pitch", 7.0),
        ("density", 0.3),
        ("texture", 0.8),
        ("spread", 0.4),
        ("feedback", 0.2),
        ("dry", 0.3),
        ("wet", 0.6),
        ("reverb", 0.15),
        ("reverb_size", 0.4),
        ("reverb_decay", 0.9),
        ("reverb_damping", 0.1),
    ];
    for (key, want) in want {
        let got = number(&engine, key);
        assert!((got - want).abs() <= 1e-6, "{key}: {got}, want {want}");
    }
    let get = |key: &str| engine.get_control("fx", key).expect("readable");
    assert_eq!(get("mode"), ControlValue::from("stretch"));
    assert_eq!((get("freeze"), get("sync")), (true.into(), true.into()));
}

/// "Every control accepts a live write by name": each mode, the bools, and
/// the numbers; the pack goes on rendering finite audio.
#[test]
fn every_control_accepts_a_live_write_by_name() {
    let mut engine = fx(json!({}));
    for mode in ["stretch", "looping_delay", "granular"] {
        set(&engine, "mode", mode);
    }
    for (key, _) in NUMBERS {
        set(&engine, key, 0.3f32);
    }
    set(&engine, "reverb_damping", 0.3f32);
    set(&engine, "pitch", -7.0f32);
    set(&engine, "freeze", true);
    set(&engine, "sync", true);
    let (left, right) = render(&mut engine, SR as usize / 2);
    assert!(left.iter().chain(&right).all(|v| v.is_finite()));
}
