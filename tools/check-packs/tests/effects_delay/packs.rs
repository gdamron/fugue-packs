//! "Both packs pass check-packs validation", and the manifest the interface
//! pins: kind `development`, entry `delay.development.json`, license MIT,
//! version 2026.11.0, a LICENSE file.

use std::fs;
use std::process::Command;

use serde_json::Value;

use super::rig::*;

#[test]
fn check_packs_validates_the_repository_with_both_packs() {
    let root = pack_dir(STEREO).join("../..");
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(&root)
        .output()
        .expect("check-packs runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "check-packs failed: {stdout}{stderr}");
    for id in BOTH {
        assert!(pack_dir(id).is_dir(), "{id} is in packs/");
    }
}

#[test]
fn manifests_declare_a_development_entry_under_mit_at_this_release() {
    for id in BOTH {
        let path = pack_dir(id).join("fugue.pkg.json");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let manifest: Value = serde_json::from_str(&text).expect("manifest is JSON");
        assert_eq!(manifest["id"], id);
        assert_eq!(manifest["kind"], "development", "{id}");
        assert_eq!(manifest["entry"]["development"], "delay.development.json", "{id}");
        assert_eq!(manifest["license"], "MIT", "{id}");
        assert_eq!(manifest["version"], "2026.11.0", "{id}");
        assert!(pack_dir(id).join("LICENSE").is_file(), "{id} has a LICENSE");
        fugue::pkg::parse_path(&path).unwrap_or_else(|e| panic!("{id}: fugue refuses it: {e}"));
    }
}
