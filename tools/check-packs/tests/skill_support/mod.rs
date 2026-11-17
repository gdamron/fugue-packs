//! Checks shared by the tests of the first-party skills, run against the
//! repository's real `packs/`, the pinned core's skill package check and
//! module registry, and the product tool names in `tools`.
//!
//! A skill's test file declares `mod policy_support; mod skill_support;` and
//! calls these with its own pack id; what is specific to one skill (its
//! routes, its procedure) stays in its file.

#![allow(dead_code)]

mod tools;

use std::collections::BTreeSet;
use std::fs;

use fugue::pkg::{check_skill_package, parse_path};
use fugue::ModuleRegistry;
use serde_json::Value;

use tools::is_command;
// Each skill's test uses some of these, not all.
#[allow(unused_imports)]
pub use tools::{HOSTED_TOOLS, INSTALL_URL, LISTENING_TOOLS, LOCAL_TOOLS};

use crate::policy_support::{
    assert_text_fragments_load, british_spellings, code_spans, pack, policy_text, problem_with,
    repository, restated_sentences, split_fences,
};

/// The most characters a skill's body may hold, unless its issue sets less.
pub const MAX_BODY_CHARACTERS: usize = 5_000;

/// The sections of the thin skill shape, and nothing else.
pub const SECTIONS: [&str; 6] = [
    "Triggers",
    "Required policies",
    "Inputs",
    "Outputs",
    "Procedure",
    "Stop conditions",
];

/// A skill pack that passed the core skill package check with no warnings.
pub struct Skill {
    pub name: String,
    pub description: String,
    /// The text after the frontmatter.
    pub body: String,
    /// The ids in the manifest's `policies`, without their version ranges.
    pub policies: Vec<String>,
}

pub fn skill(id: &str) -> Skill {
    let manifest = parse_path(pack(id).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{id} should validate: {e}"));
    assert_eq!(manifest.kind.as_str(), "skill", "{id}");
    let checked = check_skill_package(&pack(id), &manifest)
        .unwrap_or_else(|e| panic!("{id} should pass the skill check: {e}"));
    let warnings: Vec<String> = checked.warnings.iter().map(ToString::to_string).collect();
    assert!(warnings.is_empty(), "{id} warns: {warnings:#?}");

    let text = fs::read_to_string(&checked.entry).unwrap();
    let (frontmatter, body) = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .expect("SKILL.md opens with frontmatter");
    let description = frontmatter
        .lines()
        .find_map(|line| line.strip_prefix("description: "))
        .expect("a one-line description")
        .to_string();
    let policies = manifest
        .policies
        .iter()
        .flatten()
        .map(|policy| policy.split('@').next().unwrap().to_string())
        .collect();
    Skill {
        name: checked.name,
        description,
        body: body.to_string(),
        policies,
    }
}

/// The text under `## heading` in `body`, up to the next `##` heading.
pub fn section<'a>(body: &'a str, heading: &str) -> &'a str {
    let marker = format!("\n## {heading}\n");
    let start = body
        .find(&marker)
        .unwrap_or_else(|| panic!("no `## {heading}` section"))
        + marker.len();
    let rest = &body[start..];
    &rest[..rest.find("\n## ").unwrap_or(rest.len())]
}

/// A first-party skill pack: date version, MIT with a LICENSE file, the
/// given targets, and listed in the README's Skills section.
pub fn assert_first_party_skill(id: &str, targets: &[&str]) {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string(pack(id).join("fugue.pkg.json")).unwrap())
            .unwrap();
    assert_eq!(raw["version"], "2026.11.0");
    assert_eq!(raw["license"], "MIT");
    assert_eq!(raw["targets"], serde_json::json!(targets));
    assert!(pack(id).join("LICENSE").is_file(), "LICENSE is missing");

    let readme = fs::read_to_string(repository().join("README.md")).unwrap();
    let skills = readme
        .split("\n## Skills\n")
        .nth(1)
        .expect("a Skills section");
    let section = skills.split("\n## ").next().unwrap();
    assert!(section.contains(id), "the Skills section omits {id}");
}

/// The body has the thin shape: an `#` title and intro, then exactly the
/// `SECTIONS`, with a numbered procedure of at most `max_steps` steps.
pub fn assert_thin_shape(skill: &Skill, max_steps: usize) {
    let headings: BTreeSet<&str> = skill
        .body
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect();
    assert_eq!(headings, BTreeSet::from(SECTIONS), "the skill's sections");
    assert!(
        !skill.body.lines().any(|line| line.starts_with("### ")),
        "no subsections"
    );
    let steps: Vec<usize> = section(&skill.body, "Procedure")
        .lines()
        .filter_map(|line| line.split_once(". ")?.0.parse().ok())
        .collect();
    let expected: Vec<usize> = (1..=steps.len()).collect();
    assert!(!steps.is_empty(), "the procedure is a numbered list");
    assert_eq!(steps, expected, "the procedure counts from 1");
    assert!(steps.len() <= max_steps, "{} steps", steps.len());
}

/// The body, frontmatter aside, holds at most `max` characters (and never
/// more than `MAX_BODY_CHARACTERS`).
pub fn assert_body_fits(skill: &Skill, max: usize) {
    let count = skill.body.chars().count();
    let max = max.min(MAX_BODY_CHARACTERS);
    assert!(count <= max, "{count} characters, over {max}");
}

/// The manifest requires exactly `expected`, each a pack here, and the body
/// links each as `policies/<package-id>.md` and links no other policy.
pub fn assert_requires_policies(skill: &Skill, expected: &[&str]) {
    let listed: BTreeSet<&str> = skill.policies.iter().map(String::as_str).collect();
    assert_eq!(listed, expected.iter().copied().collect(), "`policies`");
    for policy in expected {
        assert!(pack(policy).is_dir(), "{policy} should be a pack here");
    }
    let linked: BTreeSet<&str> = skill
        .body
        .split("](policies/")
        .skip(1)
        .filter_map(|rest| rest.split_once(".md)").map(|(id, _)| id))
        .collect();
    assert_eq!(linked, listed, "the policies the body links");
}

/// The body copies no sentence of six words or more from a policy it
/// requires: it points at the rule instead.
pub fn assert_restates_no_policy(skill: &Skill) {
    for policy in &skill.policies {
        let copied = restated_sentences(&skill.body, &policy_text(policy));
        assert!(copied.is_empty(), "restates {policy}: {copied:#?}");
    }
}

/// Every code span in the prose is a product tool or command (`tools`), one
/// of `others` (a skill id, a slash command), or a module type, `type.key`
/// or `type.control=value` the registry has; every fragment loads; and the
/// spelling is American.
pub fn assert_names_exist(skill: &Skill, others: &[&str]) {
    let registry = ModuleRegistry::default();
    let (prose, _) = split_fences(&skill.body);
    let known = |span: &str| {
        LOCAL_TOOLS.contains(&span)
            || HOSTED_TOOLS.contains(&span)
            || LISTENING_TOOLS.contains(&span)
            || others.contains(&span)
            || is_command(span)
    };
    let problems: Vec<String> = code_spans(&prose)
        .iter()
        .filter(|span| !known(span))
        .filter_map(|span| problem_with(&registry, span))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
    assert_text_fragments_load(&skill.body, 0);
    let british = british_spellings(&skill.body, &[]);
    assert!(british.is_empty(), "British spelling: {british:?}");
}
