//! The rhythm policy pack, `fugue.policy.rhythm`, checked against the
//! repository's real `packs/` and the pinned core's module registry:
//!
//! - it is a first-party policy pack laid out like the other core policies,
//!   short enough to sit whole in an in-graph agent's prompt;
//! - every module type, config key, control, port and control value it names
//!   exists in the registry, and every fragment it shows loads;
//! - it names the policies that own the rules it leans on instead of
//!   restating them.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use fugue::pkg::{check_policy_entry, parse_path};
use fugue::{
    ControlKind, DescribeModuleQuery, ModuleDescription, ModuleRegistry, RegistryScope,
    RenderEngine,
};
use serde_json::Value;

const POLICY: &str = "fugue.policy.rhythm";
const SR: u32 = 48_000;

/// A policy is included whole in an in-graph agent's prompt.
const MAX_CHARACTERS: usize = 3_000;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pack(id: &str) -> PathBuf {
    repository().join("packs").join(id)
}

fn policy_text() -> String {
    let manifest = parse_path(pack(POLICY).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{POLICY} should validate: {e}"));
    let entry = check_policy_entry(&pack(POLICY), &manifest)
        .unwrap_or_else(|e| panic!("{POLICY}'s files should pass the check: {e}"));
    fs::read_to_string(entry).unwrap()
}

/// The text outside fenced code, and each fenced block's body.
fn split_fences(text: &str) -> (String, Vec<String>) {
    let (mut prose, mut blocks) = (String::new(), Vec::new());
    let mut open: Option<String> = None;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            match open.take() {
                Some(block) => blocks.push(block),
                None => open = Some(String::new()),
            }
        } else if let Some(block) = open.as_mut() {
            block.push_str(line);
            block.push('\n');
        } else {
            prose.push_str(line);
            prose.push('\n');
        }
    }
    assert!(open.is_none(), "a code fence is never closed");
    (prose, blocks)
}

/// The inline `code` spans of `prose`.
fn code_spans(prose: &str) -> Vec<String> {
    prose
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// Everything a type's name can be followed by: its controls (also config
/// keys), config-only keys and ports, and each control's kind.
struct TypeFacts {
    names: BTreeSet<String>,
    controls: Vec<(String, ControlKind)>,
}

fn facts(registry: &ModuleRegistry, module_type: &str) -> TypeFacts {
    let query = DescribeModuleQuery {
        module_type: Some(module_type.into()),
        ..Default::default()
    };
    let info = ModuleDescription::from_registry(registry, SR, RegistryScope::Builtins, &query)
        .unwrap_or_else(|e| panic!("{module_type} should describe: {e:?}"))
        .info;
    let mut names: BTreeSet<String> = info.inputs.iter().chain(&info.outputs).cloned().collect();
    names.extend(info.controls.iter().map(|c| c.key.to_string()));
    names.extend(
        registry
            .config_keys(module_type)
            .iter()
            .map(|k| k.key.to_string()),
    );
    let controls = info.controls.into_iter().map(|c| (c.key, c.kind)).collect();
    TypeFacts { names, controls }
}

/// Whether `value` is one the control's kind accepts.
fn accepts(kind: &ControlKind, value: &str) -> bool {
    match kind {
        ControlKind::Number { min, max, .. } => value
            .parse::<f32>()
            .is_ok_and(|number| (*min..=*max).contains(&number)),
        ControlKind::Bool => matches!(value, "true" | "false"),
        ControlKind::String {
            options: Some(options),
        } => options.iter().any(|o| o == value),
        ControlKind::String { options: None } => true,
    }
}

/// Why `span` names something the registry does not have, if it does not.
/// A span is a module type (`clock`), a `type.name` (a control, config key
/// or port of that type), or `type.control=value`.
fn problem_with(registry: &ModuleRegistry, span: &str) -> Option<String> {
    let (name, value) = match span.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (span, None),
    };
    let Some((module_type, key)) = name.split_once('.') else {
        let known = registry.has_type(name) && value.is_none();
        return (!known).then(|| format!("`{span}` is no module type"));
    };
    if !registry.has_type(module_type) {
        return Some(format!("`{span}`: no module type {module_type}"));
    }
    let facts = facts(registry, module_type);
    if !facts.names.contains(key) {
        return Some(format!("`{span}`: {module_type} has no {key}"));
    }
    let value = value?;
    match facts.controls.iter().find(|(control, _)| control == key) {
        Some((_, kind)) if accepts(kind, value) => None,
        Some(_) => Some(format!("`{span}`: {key} does not take {value}")),
        None => Some(format!("`{span}`: {key} is not a control with a value")),
    }
}

/// The policy as a first-party pack, like the other core policies.
#[test]
fn the_rhythm_policy_is_a_first_party_policy_pack() {
    let manifest = parse_path(pack(POLICY).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{POLICY} should validate: {e}"));
    assert_eq!(manifest.id, POLICY);
    assert_eq!(manifest.kind.as_str(), "policy");
    assert_eq!(manifest.version, "2026.11.0");
    assert_eq!(manifest.license, "MIT");
    let raw: Value =
        serde_json::from_str(&fs::read_to_string(pack(POLICY).join("fugue.pkg.json")).unwrap())
            .unwrap();
    let targets: Vec<&str> = raw["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(targets, ["external-agent", "in-graph-agent"]);
    assert!(pack(POLICY).join("LICENSE").is_file(), "LICENSE is missing");

    let text = policy_text();
    assert!(text.starts_with("# "), "the policy opens with a # title");
    assert!(
        text.lines().filter(|l| l.starts_with("## ")).count() >= 4,
        "each rule is a ## section"
    );
}

/// The README's Policies section lists the pack.
#[test]
fn the_readme_lists_the_policy() {
    let readme = fs::read_to_string(repository().join("README.md")).unwrap();
    let policies = readme
        .split("## Policies")
        .nth(1)
        .expect("a Policies section");
    let section = policies.split("\n## ").next().unwrap();
    assert!(
        section.contains(POLICY),
        "the Policies section omits {POLICY}"
    );
}

/// A policy is included whole in a prompt, so it stays short.
#[test]
fn the_policy_fits_a_prompt() {
    let count = policy_text().chars().count();
    assert!(
        count <= MAX_CHARACTERS,
        "{count} characters, over {MAX_CHARACTERS}"
    );
}

/// Every module type, config key, control, port and value the prose names
/// exists in the pinned core's registry, and it names at least the clock
/// and the sequencers and the module that swings.
#[test]
fn every_name_the_policy_uses_exists_in_the_registry() {
    let registry = ModuleRegistry::default();
    let (prose, _) = split_fences(&policy_text());
    let spans = code_spans(&prose);
    let problems: Vec<String> = spans
        .iter()
        .filter_map(|span| problem_with(&registry, span))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
    for needed in ["clock", "step_sequencer", "cell_sequencer", "drum_map"] {
        assert!(
            spans
                .iter()
                .any(|s| s.split(['.', '=']).next() == Some(needed)),
            "the policy should name {needed}"
        );
    }
}

/// Every fragment the policy shows loads in the pinned core, and what it
/// loads sets nothing aside (an unknown config key is refused, or warned).
#[test]
fn every_fragment_the_policy_shows_loads() {
    let (_, blocks) = split_fences(&policy_text());
    assert!(
        blocks.len() >= 2,
        "the policy shows {} fragments",
        blocks.len()
    );
    for block in blocks {
        let mut document: Value = serde_json::from_str(&block)
            .unwrap_or_else(|e| panic!("a fragment is not JSON ({e}):\n{block}"));
        document["version"] = "1.0.0".into();
        let mut engine = RenderEngine::new(SR);
        engine
            .load_json(&document.to_string())
            .unwrap_or_else(|e| panic!("a fragment fails to load ({e}):\n{block}"));
        let warnings = engine.load_warnings();
        assert!(warnings.is_empty(), "a fragment loads with {warnings:?}");
        let mut out = vec![0.0f32; 2 * 4096];
        engine.render_interleaved(&mut out).expect("renders");
    }
}

/// The checker catches a name the registry lacks.
#[test]
fn a_name_the_registry_lacks_is_caught() {
    let registry = ModuleRegistry::default();
    for span in [
        "clock.beats_per_measure",
        "drum_map.swing=1.5",
        "chance.rate=x7",
        "tempo_clock",
        "clock.bpm=fast",
    ] {
        assert!(problem_with(&registry, span).is_some(), "{span} passed");
    }
    for span in [
        "clock",
        "clock.beat_x4",
        "drum_map.swing=0.3",
        "chance.rate=x3",
    ] {
        assert_eq!(problem_with(&registry, span), None, "{span}");
    }
}

/// A rule has one home: scheduling and one-at-a-time changes are named by
/// their policies, not restated.
#[test]
fn the_policy_names_other_policies_instead_of_restating_them() {
    let text = policy_text();
    for owner in ["fugue.policy.clean-audio", "fugue.policy.musical-intent"] {
        assert!(text.contains(owner), "the policy should name {owner}");
        assert!(pack(owner).is_dir(), "{owner} should be a pack here");
    }
    let lower = text.to_lowercase();
    for owned in [
        "\"kind\"",
        "headroom",
        "retrigger",
        "one at a time",
        "envelope",
    ] {
        assert!(!lower.contains(owned), "the policy restates `{owned}`");
    }
}

/// American English.
#[test]
fn the_policy_uses_american_spelling() {
    let lower = policy_text().to_lowercase();
    for british in [
        "colour",
        "behaviour",
        "centre",
        "rhythmical",
        "labelled",
        "grey",
    ] {
        assert!(!lower.contains(british), "British spelling: {british}");
    }
}
