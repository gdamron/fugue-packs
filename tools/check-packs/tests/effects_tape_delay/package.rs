//! The package and its control surface: the manifest the interface pins,
//! check-packs validation, the controls with their defaults (and no
//! `glide`), and starting levels that land before audio starts.

use std::fs;
use std::process::Command;

use fugue::ControlValue;
use serde_json::{json, Value};

use super::rig::*;
use super::tape::*;

#[test]
fn check_packs_validates_the_repository_with_the_tape_pack() {
    assert!(pack_dir(TAPE).is_dir(), "{TAPE} is in packs/");
    let root = pack_dir(TAPE).join("../..");
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(&root)
        .output()
        .expect("check-packs runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "check-packs failed: {stdout}{stderr}");
}

/// "`fugue.pkg.json` (kind `development`, entry `delay.development.json`,
/// MIT, version `2026.11.1`) and `LICENSE`".
#[test]
fn manifest_declares_a_development_entry_under_mit_at_this_release() {
    let path = pack_dir(TAPE).join("fugue.pkg.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let manifest: Value = serde_json::from_str(&text).expect("manifest is JSON");
    assert_eq!(manifest["id"], TAPE);
    assert_eq!(manifest["kind"], "development");
    assert_eq!(manifest["entry"]["development"], "delay.development.json");
    assert_eq!(manifest["license"], "MIT");
    assert_eq!(manifest["version"], "2026.11.1");
    assert!(pack_dir(TAPE).join("LICENSE").is_file(), "has a LICENSE");
    fugue::pkg::parse_path(&path).unwrap_or_else(|e| panic!("fugue refuses it: {e}"));
}

/// The controls the interface lists, with their defaults.
fn expected() -> Vec<(&'static str, ControlValue)> {
    use ControlValue::{Bool, Number, String as Choice};
    vec![
        ("time", Number(0.3)),
        ("division", Choice("off".into())),
        ("bpm", Number(120.0)),
        ("feedback", Number(0.4)),
        ("tone", Number(3500.0)),
        ("dry", Number(1.0)),
        ("wet", Number(0.5)),
        ("wow", Number(0.25)),
        ("flutter", Number(0.2)),
        ("hold", Bool(false)),
    ]
}

/// Each control is listed by name and starts at its default; "There is no
/// `glide` control". Numbers are compared to within 1e-6.
#[test]
fn controls_are_exposed_by_name_with_their_defaults_and_no_glide() {
    let engine = engine(&noise_both().effect(TAPE, json!({}), 1.0));
    let listed = engine.list_controls(Some("fx")).expect("fx lists controls");
    let keys: Vec<String> = listed
        .iter()
        .flat_map(|(_, c)| c.iter().map(|m| m.key.clone()))
        .collect();
    for (key, default) in expected() {
        assert!(keys.iter().any(|k| k == key), "lists {key}: {keys:?}");
        let value = engine.get_control("fx", key).expect("readable");
        match (&value, &default) {
            (ControlValue::Number(got), ControlValue::Number(want)) => {
                assert!((got - want).abs() <= 1e-6, "{key}: {got}, want {want}")
            }
            _ => assert_eq!(value, default, "{key} default"),
        }
    }
    assert!(
        !keys.iter().any(|k| k == "glide"),
        "no glide control: {keys:?}"
    );
}

/// Every control accepts a live write by its exposed name, at the top of
/// its range where the interface gives one.
#[test]
fn every_control_accepts_a_live_write() {
    let fx = engine(&noise_both().effect(TAPE, json!({}), 1.0));
    set(&fx, "division", "quarter_note");
    set(&fx, "time", 0.5f32);
    set(&fx, "bpm", 1000.0f32);
    set(&fx, "feedback", 2.0f32);
    set(&fx, "tone", 18_000.0f32);
    set(&fx, "dry", 2.0f32);
    set(&fx, "wet", 2.0f32);
    set(&fx, "wow", 1.0f32);
    set(&fx, "flutter", 1.0f32);
    set(&fx, "hold", true);
}

/// "Both are mixer levels, as in the shared controls": a pack built with
/// `dry` and `wet` at 0 is silent from its first sample (starting values
/// land before audio starts, not by a glide).
#[test]
fn a_pack_built_muted_is_silent_from_its_first_sample() {
    let config = json!({ "dry": 0.0, "wet": 0.0 });
    let (left, right) = render_doc(&sine_doc(config, 440.0, 1.0, 1.0), 2 * TIME);
    let loudest = peak(&left).max(peak(&right));
    assert_eq!(loudest, 0.0, "leaks {loudest}");
}
