//! The pack's layout as the interface pins it: `packs/fugue.effects.compressor/`
//! with its development in `compressor.development.json`, kind
//! `development`, and the repository still validates with it.

use std::fs;
use std::process::Command;

use serde_json::Value;

use super::rig::*;

#[test]
fn check_packs_validates_the_repository_with_the_compressor() {
    assert!(pack_dir().is_dir(), "{PACK} is in packs/");
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
fn the_manifest_declares_a_development_entry() {
    let path = pack_dir().join("fugue.pkg.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let manifest: Value = serde_json::from_str(&text).expect("manifest is JSON");
    assert_eq!(manifest["id"], PACK);
    assert_eq!(manifest["kind"], "development");
    assert_eq!(
        manifest["entry"]["development"],
        "compressor.development.json"
    );
    fugue::pkg::parse_path(&path).unwrap_or_else(|e| panic!("fugue refuses it: {e}"));
}
