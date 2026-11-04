//! The package and its control surface: the manifest, check-packs
//! validation, the controls with their defaults, live writes, and starting
//! values given in config.

use std::fs;
use std::process::Command;

use fugue::ControlValue;
use serde_json::{json, Value};

use super::rig::*;

#[test]
fn check_packs_validates_the_repository_with_the_eq_pack() {
    assert!(pack_dir().is_dir(), "{EQ} is in packs/");
    let root = pack_dir().join("../..");
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(&root)
        .output()
        .expect("check-packs runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "check-packs failed: {stdout}{stderr}");
}

#[test]
fn manifest_declares_a_development_entry_under_mit_at_this_release() {
    let path = pack_dir().join("fugue.pkg.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let manifest: Value = serde_json::from_str(&text).expect("manifest is JSON");
    assert_eq!(manifest["id"], EQ);
    assert_eq!(manifest["kind"], "development");
    assert_eq!(manifest["entry"]["development"], "eq.development.json");
    assert_eq!(manifest["license"], "MIT");
    assert_eq!(manifest["version"], "2026.11.0");
    let targets = manifest["targets"].as_array().expect("targets");
    for target in ["external-agent", "in-graph-agent"] {
        assert!(targets.iter().any(|t| t == target), "targets {target}");
    }
    assert!(pack_dir().join("LICENSE").is_file(), "has a LICENSE");
    fugue::pkg::parse_path(&path).unwrap_or_else(|e| panic!("fugue refuses it: {e}"));
}

/// The controls the interface lists, with their defaults.
fn expected() -> Vec<(String, ControlValue)> {
    use ControlValue::{Bool, Number, String as Choice};
    let mut controls = vec![
        ("low_cut_frequency".into(), Number(30.0)),
        ("low_cut_slope".into(), Choice("12db".into())),
        ("low_cut_bypass".into(), Bool(true)),
        ("low_shelf_frequency".into(), Number(100.0)),
        ("low_shelf_boost".into(), Number(0.0)),
        (
            "low_shelf_q".into(),
            Number(std::f32::consts::FRAC_1_SQRT_2),
        ),
        ("low_shelf_bypass".into(), Bool(false)),
        ("high_shelf_frequency".into(), Number(8000.0)),
        ("high_shelf_boost".into(), Number(0.0)),
        (
            "high_shelf_q".into(),
            Number(std::f32::consts::FRAC_1_SQRT_2),
        ),
        ("high_shelf_bypass".into(), Bool(false)),
        ("high_cut_frequency".into(), Number(18_000.0)),
        ("high_cut_slope".into(), Choice("12db".into())),
        ("high_cut_bypass".into(), Bool(true)),
    ];
    for (i, freq) in [250.0, 1000.0, 4000.0].into_iter().enumerate() {
        controls.push((format!("bell_frequency.{i}"), Number(freq)));
        controls.push((format!("bell_boost.{i}"), Number(0.0)));
        controls.push((format!("bell_q.{i}"), Number(1.0)));
        controls.push((format!("bell_bypass.{i}"), Bool(false)));
    }
    controls
}

fn assert_value(key: &str, got: &ControlValue, want: &ControlValue) {
    match (got, want) {
        (ControlValue::Number(g), ControlValue::Number(w)) => {
            assert!(
                (g - w).abs() <= 1e-4 * w.abs().max(1.0),
                "{key}: {g}, want {w}"
            )
        }
        _ => assert_eq!(got, want, "{key}"),
    }
}

#[test]
fn controls_are_exposed_by_name_with_their_defaults() {
    let engine = engine(&Feed::noise_pair(1.0).effect(json!({}), 1.0));
    let listed = engine.list_controls(Some("fx")).expect("fx lists controls");
    let keys: Vec<String> = listed
        .iter()
        .flat_map(|(_, c)| c.iter().map(|m| m.key.clone()))
        .collect();
    for (key, default) in expected() {
        assert!(keys.contains(&key), "lists {key}: {keys:?}");
        let value = engine.get_control("fx", &key).expect("readable");
        assert_value(&key, &value, &default);
    }
}

/// Every control takes a live write by its exposed name, at the ends of the
/// ranges the interface gives.
#[test]
fn every_control_accepts_a_live_write() {
    let mut fx = engine(&Feed::noise_pair(0.1).effect(json!({}), 1.0));
    render(&mut fx, 480);
    for (key, default) in expected() {
        match default {
            ControlValue::Bool(on) => set(&fx, &key, !on),
            ControlValue::String(_) => set(&fx, &key, "24db"),
            ControlValue::Number(_) if key.contains("frequency") => {
                set(&fx, &key, 20.0f32);
                set(&fx, &key, 20_000.0f32);
            }
            ControlValue::Number(_) if key.contains("boost") => {
                set(&fx, &key, -24.0f32);
                set(&fx, &key, 24.0f32);
            }
            ControlValue::Number(_) => {
                set(&fx, &key, 0.1f32);
                set(&fx, &key, 18.0f32);
            }
        }
    }
    let (left, right) = render(&mut fx, 4800);
    assert!(left.iter().chain(&right).all(|v| v.is_finite()), "finite");
}

/// "A control can also be given a starting value in the module's config."
#[test]
fn config_gives_starting_values() {
    let config = json!({
        "bell_boost.1": 6.0, "bell_q.2": 4.0, "low_cut_bypass": false,
        "low_cut_slope": "24db", "high_shelf_frequency": 5000.0
    });
    let engine = engine(&Feed::noise_pair(0.1).effect(config, 1.0));
    use ControlValue::{Bool, Number, String as Choice};
    for (key, want) in [
        ("bell_boost.1", Number(6.0)),
        ("bell_q.2", Number(4.0)),
        ("low_cut_bypass", Bool(false)),
        ("low_cut_slope", Choice("24db".into())),
        ("high_shelf_frequency", Number(5000.0)),
    ] {
        let got = engine.get_control("fx", key).expect("readable");
        assert_value(key, &got, &want);
    }
}
