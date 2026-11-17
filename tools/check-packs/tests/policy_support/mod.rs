//! Checks shared by the tests of the first-party reference policies
//! (`fugue.policy.rhythm`, `fugue.policy.theory`, ...), run against the
//! repository's real `packs/` and the pinned core's module registry.
//!
//! Each policy's test file calls these with its own pack id and the names it
//! must teach; what is specific to one policy stays in its file.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use fugue::pkg::{check_policy_entry, parse_path};
use fugue::{
    ControlKind, DescribeModuleQuery, ModuleDescription, ModuleRegistry, RegistryScope,
    RenderEngine,
};
use serde_json::Value;

pub const SR: u32 = 48_000;

/// A policy is included whole in an in-graph agent's prompt.
pub const MAX_CHARACTERS: usize = 3_000;

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn pack(id: &str) -> PathBuf {
    repository().join("packs").join(id)
}

pub fn policy_text(id: &str) -> String {
    let manifest = parse_path(pack(id).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{id} should validate: {e}"));
    let entry = check_policy_entry(&pack(id), &manifest)
        .unwrap_or_else(|e| panic!("{id}'s files should pass the check: {e}"));
    fs::read_to_string(entry).unwrap()
}

/// The text outside fenced code, and each fenced block's body.
pub fn split_fences(text: &str) -> (String, Vec<String>) {
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
pub fn code_spans(prose: &str) -> Vec<String> {
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

/// The options of the control `module_type.key`, if it is a choice.
pub fn control_options(
    registry: &ModuleRegistry,
    module_type: &str,
    key: &str,
) -> Option<Vec<String>> {
    match facts(registry, module_type)
        .controls
        .into_iter()
        .find(|(control, _)| control == key)?
        .1
    {
        ControlKind::String {
            options: Some(options),
        } => Some(options),
        _ => None,
    }
}

/// The module types that declare a control or config key named `key`. A
/// type that cannot be described without config of its own (a sink that
/// needs a URL) is skipped.
pub fn types_with_control(registry: &ModuleRegistry, key: &str) -> BTreeSet<String> {
    registry
        .types()
        .filter(|module_type| {
            let query = DescribeModuleQuery {
                module_type: Some((*module_type).into()),
                ..Default::default()
            };
            let described =
                ModuleDescription::from_registry(registry, SR, RegistryScope::Builtins, &query);
            let control =
                described.is_ok_and(|d| d.info.controls.iter().any(|control| control.key == key));
            control
                || registry
                    .config_keys(module_type)
                    .iter()
                    .any(|k| k.key == key)
        })
        .map(str::to_string)
        .collect()
}

/// Why `span` names something the registry does not have, if it does not.
/// A span is a module type (`clock`), a `type.name` (a control, config key
/// or port of that type), or `type.control=value`.
pub fn problem_with(registry: &ModuleRegistry, span: &str) -> Option<String> {
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
pub fn assert_first_party_policy(id: &str) {
    let manifest = parse_path(pack(id).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{id} should validate: {e}"));
    assert_eq!(manifest.id, id);
    assert_eq!(manifest.kind.as_str(), "policy");
    assert_eq!(manifest.version, "2026.11.0");
    assert_eq!(manifest.license, "MIT");
    let raw: Value =
        serde_json::from_str(&fs::read_to_string(pack(id).join("fugue.pkg.json")).unwrap())
            .unwrap();
    let targets: Vec<&str> = raw["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(targets, ["external-agent", "in-graph-agent"]);
    assert!(pack(id).join("LICENSE").is_file(), "LICENSE is missing");

    let text = policy_text(id);
    assert!(text.starts_with("# "), "the policy opens with a # title");
    assert!(
        text.lines().filter(|l| l.starts_with("## ")).count() >= 4,
        "each rule is a ## section"
    );
}

/// The README's Policies section lists the pack.
pub fn assert_readme_lists(id: &str) {
    let readme = fs::read_to_string(repository().join("README.md")).unwrap();
    let policies = readme
        .split("## Policies")
        .nth(1)
        .expect("a Policies section");
    let section = policies.split("\n## ").next().unwrap();
    assert!(section.contains(id), "the Policies section omits {id}");
}

/// A policy is included whole in a prompt, so it stays short.
pub fn assert_fits_prompt(id: &str) {
    let count = policy_text(id).chars().count();
    assert!(
        count <= MAX_CHARACTERS,
        "{count} characters, over {MAX_CHARACTERS}"
    );
}

/// Every module type, config key, control, port and value the prose names
/// exists in the pinned core's registry, and it names each type in `needed`.
/// A span in `tools` (a tool or argument such as `set_controls`) is allowed
/// when the authoring policy names it too.
pub fn assert_names_exist(id: &str, needed: &[&str], tools: &[&str]) {
    let registry = ModuleRegistry::default();
    let authoring = policy_text("fugue.policy.authoring");
    for tool in tools {
        assert!(
            authoring.contains(tool),
            "{tool} is not named by fugue.policy.authoring"
        );
    }
    let (prose, _) = split_fences(&policy_text(id));
    let spans = code_spans(&prose);
    let problems: Vec<String> = spans
        .iter()
        .filter(|span| !tools.contains(&span.as_str()))
        .filter_map(|span| problem_with(&registry, span))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
    for needed in needed {
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
pub fn assert_fragments_load(id: &str, at_least: usize) {
    let (_, blocks) = split_fences(&policy_text(id));
    assert!(
        blocks.len() >= at_least,
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

/// A rule has one home: the policy names each owner in `owners` (a pack
/// here) and restates none of `owned`, matched in lower case.
pub fn assert_names_owners_without_restating(id: &str, owners: &[&str], owned: &[&str]) {
    let text = policy_text(id);
    for owner in owners {
        assert!(text.contains(owner), "the policy should name {owner}");
        assert!(pack(owner).is_dir(), "{owner} should be a pack here");
    }
    let lower = text.to_lowercase();
    for owned in owned {
        assert!(!lower.contains(owned), "the policy restates `{owned}`");
    }
}

/// American English: none of `extra` nor the common British spellings.
pub fn assert_american_spelling(id: &str, extra: &[&str]) {
    let lower = policy_text(id).to_lowercase();
    for british in ["colour", "behaviour", "centre", "labelled", "grey"]
        .into_iter()
        .chain(extra.iter().copied())
    {
        assert!(!lower.contains(british), "British spelling: {british}");
    }
}
