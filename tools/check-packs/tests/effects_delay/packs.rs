//! "Both packs pass check-packs validation", and the manifest the interface
//! pins: kind `development`, entry `delay.development.json`, license MIT,
//! a date-based YYYY.M.P version not earlier than 2026.11.1, a LICENSE file.

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

/// A date-based version YYYY.M.P (a year from 2026, a month 1-12, a patch, no
/// leading zeros) not earlier than 2026.11.1.
fn assert_date_version_not_before_2026_11_1(what: &str, version: &str) {
    let parts: Vec<&str> = version.split('.').collect();
    assert!(parts.len() == 3, "{what}: {version} is YYYY.M.P");
    assert!(
        parts.iter().all(|p| !p.starts_with('0') || *p == "0"),
        "{what}: {version} has a leading zero"
    );
    let numbers: Vec<u32> = (parts.iter())
        .map(|p| {
            p.parse()
                .unwrap_or_else(|_| panic!("{what} version {version}"))
        })
        .collect();
    assert!((2026..2100).contains(&numbers[0]), "{what}: {version}");
    assert!((1..=12).contains(&numbers[1]), "{what}: {version}");
    assert!(
        (numbers[0], numbers[1], numbers[2]) >= (2026, 11, 1),
        "{what}: {version} is earlier than 2026.11.1"
    );
}

#[test]
fn manifests_declare_a_development_entry_under_mit_at_this_release() {
    for id in BOTH {
        let path = pack_dir(id).join("fugue.pkg.json");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let manifest: Value = serde_json::from_str(&text).expect("manifest is JSON");
        assert_eq!(manifest["id"], id);
        assert_eq!(manifest["kind"], "development", "{id}");
        assert_eq!(
            manifest["entry"]["development"], "delay.development.json",
            "{id}"
        );
        assert_eq!(manifest["license"], "MIT", "{id}");
        let version = manifest["version"].as_str().expect("a version string");
        assert_date_version_not_before_2026_11_1(id, version);
        assert!(pack_dir(id).join("LICENSE").is_file(), "{id} has a LICENSE");
        fugue::pkg::parse_path(&path).unwrap_or_else(|e| panic!("{id}: fugue refuses it: {e}"));
    }
}
