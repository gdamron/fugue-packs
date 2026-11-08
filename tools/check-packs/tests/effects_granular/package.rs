//! The package: check-packs validation, the manifest and the controls with
//! their defaults.

use std::fs;
use std::process::Command;

use fugue::ControlValue;
use serde_json::{json, Value};

use super::rig::*;

#[test]
fn check_packs_validates_the_repository_with_the_granular_pack() {
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(pack_dir().join("../.."))
        .output()
        .expect("check-packs runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "check-packs failed: {stdout}{stderr}");
}

#[test]
fn manifest_declares_the_development_under_mit() {
    let path = pack_dir().join("fugue.pkg.json");
    let manifest: Value = serde_json::from_str(&fs::read_to_string(&path).expect("manifest"))
        .expect("manifest is JSON");
    assert_eq!(manifest["id"], GRANULAR);
    assert_eq!(manifest["kind"], "development");
    assert_eq!(
        manifest["entry"]["development"],
        "granular.development.json"
    );
    assert_eq!(manifest["license"], "MIT");
    // A date-based YYYY.M.P version, at or after the pack's first release.
    let version = manifest["version"].as_str().expect("version");
    let parts: Vec<u32> = (version.split('.'))
        .map(|p| p.parse().unwrap_or_else(|_| panic!("version {version}")))
        .collect();
    assert!(
        parts.len() == 3 && (parts[0], parts[1], parts[2]) >= (2026, 11, 1),
        "version {version}"
    );
    assert!(pack_dir().join("LICENSE").is_file(), "has a LICENSE");
    fugue::pkg::parse_path(&path).unwrap_or_else(|e| panic!("fugue refuses it: {e}"));
}

#[test]
fn controls_start_at_their_defaults() {
    use ControlValue::{Bool, Number, String as Choice};
    let fx = engine(&effect(&Source::sine(220.0, 0.5), json!({})));
    let expected = [
        ("mode", Choice("granular".into())),
        ("position", Number(0.5)),
        ("size", Number(0.5)),
        ("pitch", Number(0.0)),
        ("density", Number(0.7)),
        ("texture", Number(0.5)),
        ("freeze", Bool(false)),
        ("spread", Number(0.0)),
        ("feedback", Number(0.0)),
        ("sync", Bool(false)),
        ("glide", Number(0.0)),
        ("inertia", Number(0.5)),
        ("dry", Number(1.0)),
        ("wet", Number(0.5)),
        ("reverb", Number(0.25)),
        ("reverb_size", Number(0.7)),
        ("reverb_decay", Number(0.7)),
        ("reverb_damping", Number(0.5)),
    ];
    for (key, want) in expected {
        let got = fx
            .get_control("fx", key)
            .unwrap_or_else(|e| panic!("{key}: {e}"));
        match (&got, &want) {
            (Number(g), Number(w)) => assert!((g - w).abs() <= 1e-6, "{key}: {g}, want {w}"),
            _ => assert_eq!(got, want, "{key}"),
        }
    }
}
