//! The core musical policies, `fugue.policy.clean-audio`,
//! `fugue.policy.musical-intent` and `fugue.policy.listening`, checked against
//! the repository's real `packs/`: each is a first-party policy pack small
//! enough to sit whole in an in-graph agent's prompt, and the pulse demos
//! require the real policies they use rather than restating them.

use std::fs;
use std::path::{Path, PathBuf};

use fugue::pkg::{check_policy_entry, parse_path};
use serde_json::Value;

const CLEAN_AUDIO: &str = "fugue.policy.clean-audio";
const MUSICAL_INTENT: &str = "fugue.policy.musical-intent";
const LISTENING: &str = "fugue.policy.listening";
const SKILL: &str = "fugue.demo.pulse-skill";
const AGENT: &str = "fugue.demo.pulse-agent";

/// A policy is included whole in an in-graph agent's prompt.
const MAX_CHARACTERS: usize = 2_500;

fn pack(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs")
        .join(id)
}

/// The policy text of `id`, after checking the pack as the checker does.
fn rules(id: &str) -> String {
    let manifest = parse_path(pack(id).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{id} should validate: {e}"));
    assert_eq!(manifest.kind.as_str(), "policy", "{id}");
    assert_eq!(manifest.version, "2026.11.0", "{id}");
    let entry = check_policy_entry(&pack(id), &manifest)
        .unwrap_or_else(|e| panic!("{id}'s files should pass the check: {e}"));
    fs::read_to_string(entry).unwrap()
}

fn listed_policies(id: &str) -> Vec<String> {
    let text = fs::read_to_string(pack(id).join("fugue.pkg.json")).unwrap();
    let manifest: Value = serde_json::from_str(&text).unwrap();
    manifest["policies"]
        .as_array()
        .unwrap_or_else(|| panic!("{id} should list `policies`"))
        .iter()
        .map(|entry| entry.as_str().unwrap().to_string())
        .collect()
}

/// Each policy is a policy pack, short enough for a prompt, and states the
/// rules the issue lists for it.
#[test]
fn each_core_policy_states_its_rules_in_a_prompt_sized_document() {
    let expected = [
        (CLEAN_AUDIO, ["headroom", "retrigger", "ramp"]),
        (MUSICAL_INTENT, ["one audible", "establish", "small"]),
        (LISTENING, ["clamp", "peak meter", "heard or measured"]),
    ];
    for (id, phrases) in expected {
        let text = rules(id);
        assert!(text.starts_with("# "), "{id} opens with a # title");
        assert!(
            text.chars().count() <= MAX_CHARACTERS,
            "{id} is {} characters, over {MAX_CHARACTERS}",
            text.chars().count()
        );
        let lower = text.to_lowercase();
        for phrase in phrases {
            assert!(lower.contains(phrase), "{id} should state `{phrase}`");
        }
    }
}

/// A rule has one home: the listening policy owns the peak meter and the
/// clamp, so the other two do not state them.
#[test]
fn the_core_policies_do_not_restate_each_others_rules() {
    for id in [CLEAN_AUDIO, MUSICAL_INTENT] {
        let text = rules(id).to_lowercase();
        for owned in ["get_meters", "show_scope", "pre-clamp"] {
            assert!(!text.contains(owned), "{id} should not restate `{owned}`");
        }
    }
    let intent = rules(MUSICAL_INTENT).to_lowercase();
    for owned in ["headroom", "retrigger", "envelope"] {
        assert!(!intent.contains(owned), "musical intent restates `{owned}`");
    }
}

/// Both demos require the real policies they use, and the demo policy is gone.
#[test]
fn the_pulse_demos_require_the_core_policies() {
    assert!(!pack("fugue.demo.pulse-policy").exists());
    for id in [SKILL, AGENT] {
        let listed = listed_policies(id);
        for policy in [CLEAN_AUDIO, MUSICAL_INTENT] {
            assert!(
                listed
                    .iter()
                    .any(|entry| entry.starts_with(&format!("{policy}@"))),
                "{id} should list {policy} in `policies`"
            );
        }
    }
    let skill = fs::read_to_string(pack(SKILL).join("SKILL.md")).unwrap();
    for policy in [CLEAN_AUDIO, MUSICAL_INTENT] {
        assert!(
            skill.contains(&format!("policies/{policy}.md")),
            "the skill should link {policy}"
        );
    }
}
