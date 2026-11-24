//! The checker on temporary repositories: a skill links a policy only from a
//! sentence that opens with when to read it, and links only the policies its
//! manifest requires. Each case runs the `check-packs` binary.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};
use tempfile::TempDir;

/// The words a pointer's sentence may open with, each naming a moment.
const TRIGGER_WORDS: [&str; 7] = ["Before", "When", "While", "Whenever", "After", "Once", "If"];

const REQUIRED: &str = "fugue.test.required-policy";
const UNREQUIRED: &str = "fugue.test.unrequired-policy";
const SKILL_ID: &str = "fugue.test.pointer-skill";

struct Run {
    success: bool,
    output: String,
}

fn check(root: &Path) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(root)
        .output()
        .expect("check-packs runs");
    Run {
        success: out.status.success(),
        output: format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}

fn write_pack(repo: &Path, id: &str, manifest: &Value, files: &[(&str, &str)]) {
    let dir = repo.join("packs").join(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("fugue.pkg.json"),
        serde_json::to_string_pretty(manifest).unwrap(),
    )
    .unwrap();
    for (name, content) in files {
        fs::write(dir.join(name), content).unwrap();
    }
}

fn policy_pack(repo: &Path, id: &str) {
    write_pack(
        repo,
        id,
        &json!({
            "id": id,
            "version": "0.1.0",
            "kind": "policy",
            "license": "CC0-1.0",
            "authors": [{ "name": "Example Engineer" }],
            "description": "Shared rules.",
            "targets": ["external-agent"],
            "entry": { "policy": "POLICY.md" }
        }),
        &[("POLICY.md", "# Rules\n\nKeep headroom.\n")],
    );
}

/// A repository with two policy packs and one skill whose manifest requires
/// `requires` and whose body is `body`.
fn repository_with(requires: &[&str], body: &str) -> TempDir {
    let repo = tempfile::tempdir().unwrap();
    fs::write(repo.path().join("index.json"), r#"{"packages":[]}"#).unwrap();
    fs::create_dir(repo.path().join("packs")).unwrap();
    policy_pack(repo.path(), REQUIRED);
    policy_pack(repo.path(), UNREQUIRED);
    let policies: Vec<String> = requires.iter().map(|id| format!("{id}@^0.1.0")).collect();
    let mut manifest = json!({
        "id": SKILL_ID,
        "version": "0.1.0",
        "kind": "skill",
        "license": "CC0-1.0",
        "authors": [{ "name": "Example Engineer" }],
        "description": "A skill under test.",
        "targets": ["external-agent"],
        "entry": { "skill": "SKILL.md" }
    });
    if !policies.is_empty() {
        manifest["policies"] = json!(policies);
    }
    let skill = format!(
        "---\nname: pointer-skill\ndescription: Shapes a phrase.\n---\n\n# Skill\n\n{body}\n"
    );
    write_pack(repo.path(), SKILL_ID, &manifest, &[("SKILL.md", &skill)]);
    repo
}

fn link(id: &str) -> String {
    format!("policies/{id}.md")
}

fn assert_fails_naming(repo: &TempDir, id: &str) {
    let run = check(repo.path());
    assert!(!run.success, "the run should fail: {}", run.output);
    assert!(
        run.output.contains(id),
        "the output should name {id}: {}",
        run.output
    );
}

fn assert_passes(repo: &TempDir) {
    let run = check(repo.path());
    assert!(run.success, "the run should pass: {}", run.output);
}

#[test]
fn a_link_in_a_list_item_that_opens_with_the_policy_name_fails_naming_it() {
    let body = format!("- [Rules]({}): how a phrase is shaped.", link(REQUIRED));
    assert_fails_naming(&repository_with(&[REQUIRED], &body), REQUIRED);
}

#[test]
fn a_link_in_a_sentence_that_opens_with_another_word_fails_naming_the_policy() {
    let body = format!(
        "Read the [rules]({}) before shaping a phrase.",
        link(REQUIRED)
    );
    assert_fails_naming(&repository_with(&[REQUIRED], &body), REQUIRED);
}

#[test]
fn a_trigger_word_in_an_earlier_sentence_does_not_cover_the_link() {
    let body = format!(
        "Before you start, breathe. The [rules]({}) shape a phrase.",
        link(REQUIRED)
    );
    assert_fails_naming(&repository_with(&[REQUIRED], &body), REQUIRED);
}

#[test]
fn a_trigger_word_in_the_middle_of_the_sentence_does_not_count() {
    let body = format!(
        "Read the [rules]({}) when shaping a phrase.",
        link(REQUIRED)
    );
    assert_fails_naming(&repository_with(&[REQUIRED], &body), REQUIRED);
}

#[test]
fn a_link_to_a_policy_the_manifest_does_not_require_fails_naming_it() {
    let body = format!(
        "Before shaping a phrase, read the [rules]({}). When it ends, read the [extras]({}).",
        link(REQUIRED),
        link(UNREQUIRED)
    );
    assert_fails_naming(&repository_with(&[REQUIRED], &body), UNREQUIRED);
}

#[test]
fn a_link_in_a_skill_that_requires_no_policy_fails_naming_it() {
    let body = format!(
        "Before shaping a phrase, read the [rules]({}).",
        link(UNREQUIRED)
    );
    assert_fails_naming(&repository_with(&[], &body), UNREQUIRED);
}

#[test]
fn a_required_policy_linked_in_a_trigger_sentence_passes() {
    let body = format!(
        "Before shaping a phrase, read the [rules]({}).",
        link(REQUIRED)
    );
    assert_passes(&repository_with(&[REQUIRED], &body));
}

#[test]
fn each_trigger_word_opens_a_passing_sentence() {
    for word in TRIGGER_WORDS {
        let body = format!(
            "{word} shaping a phrase, read the [rules]({}).",
            link(REQUIRED)
        );
        let repo = repository_with(&[REQUIRED], &body);
        let run = check(repo.path());
        assert!(run.success, "{word}: {}", run.output);
    }
}

#[test]
fn a_trigger_sentence_after_another_sentence_in_the_paragraph_passes() {
    let body = format!(
        "Keep it short. When shaping a phrase, read the [rules]({}). Then play it.",
        link(REQUIRED)
    );
    assert_passes(&repository_with(&[REQUIRED], &body));
}

#[test]
fn a_trigger_sentence_in_a_list_item_or_wrapped_over_lines_passes() {
    let wrapped = format!(
        "Before shaping a phrase,\nread the\n[rules]({}).",
        link(REQUIRED)
    );
    assert_passes(&repository_with(&[REQUIRED], &wrapped));
    let bullet = format!("- If it is a phrase, read the [rules]({}).", link(REQUIRED));
    assert_passes(&repository_with(&[REQUIRED], &bullet));
    let numbered = format!("1. After a phrase, read the [rules]({}).", link(REQUIRED));
    assert_passes(&repository_with(&[REQUIRED], &numbered));
}
