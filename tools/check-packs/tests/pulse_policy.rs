//! The pulse demos share one policy pack, `fugue.demo.pulse-policy`: it holds
//! the shared rules, and the skill and the agent list it in `policies` instead
//! of restating them. Checked against the repository's real `packs/`.

use std::fs;
use std::path::{Path, PathBuf};

use fugue::pkg::{check_policy_entry, parse_path};
use serde_json::Value;

const POLICY: &str = "fugue.demo.pulse-policy";
const SKILL: &str = "fugue.demo.pulse-skill";
const AGENT: &str = "fugue.demo.pulse-agent";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pack(id: &str) -> PathBuf {
    repository().join("packs").join(id)
}

fn raw_manifest(id: &str) -> Value {
    let text = fs::read_to_string(pack(id).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{id}: cannot read its manifest: {e}"));
    serde_json::from_str(&text).unwrap()
}

/// The `policies` a pack's manifest lists.
fn listed_policies(id: &str) -> Vec<String> {
    raw_manifest(id)["policies"]
        .as_array()
        .unwrap_or_else(|| panic!("{id} should list `policies`"))
        .iter()
        .map(|entry| entry.as_str().unwrap().to_string())
        .collect()
}

/// The policy pack exists, is a policy, and holds the shared
/// rules the two demos used to restate.
#[test]
fn the_pulse_policy_pack_holds_the_shared_rules() {
    let manifest = parse_path(pack(POLICY).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("the policy pack should validate: {e}"));
    assert_eq!(manifest.id, POLICY);
    assert_eq!(manifest.kind.as_str(), "policy");
    assert_eq!(manifest.version, "0.1.0", "demo packs stay at 0.1.0");

    let entry = check_policy_entry(&pack(POLICY), &manifest)
        .unwrap_or_else(|e| panic!("the policy pack's files should pass the check: {e}"));
    let rules = fs::read_to_string(entry).unwrap().to_lowercase();
    assert!(
        rules.contains("headroom"),
        "the policy states the headroom rule"
    );
    assert!(
        rules.contains("at a time"),
        "the policy states the one-change-at-a-time rule"
    );
}

/// Both demos list the policy, and no longer restate its rules.
#[test]
fn the_skill_and_the_agent_list_the_policy_and_no_longer_restate_it() {
    for id in [SKILL, AGENT] {
        assert!(
            listed_policies(id)
                .iter()
                .any(|entry| entry.starts_with(&format!("{POLICY}@"))),
            "{id} should list {POLICY} in `policies`"
        );
    }

    let skill = fs::read_to_string(pack(SKILL).join("SKILL.md"))
        .unwrap()
        .to_lowercase();
    assert!(
        !skill.contains("change one dimension at a time"),
        "the skill should not restate the one-change rule"
    );
    let agent = fs::read_to_string(pack(AGENT).join("agent.json"))
        .unwrap()
        .to_lowercase();
    for restated in ["protect headroom", "one audible musical decision at a time"] {
        assert!(
            !agent.contains(restated),
            "the agent definition should not restate `{restated}`"
        );
    }
}
