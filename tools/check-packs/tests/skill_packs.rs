//! check-packs runs the skill package checks on every skill pack: a problem
//! fails the run and is named, a warning is printed and the run still
//! succeeds. Each case builds a temporary repository and runs the built
//! binary against it; the shipped repository is checked as it stands.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use tempfile::TempDir;

const POLICY: &str = "fugue.test.listed-policy";
const MINIMAL_INVENTION: &str = r#"{"modules": [], "connections": []}"#;

struct Run {
    success: bool,
    /// Standard output and standard error together: warnings and failures may
    /// be printed on either.
    output: String,
    stderr: String,
}

fn check(root: &Path) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(root)
        .output()
        .expect("check-packs runs");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    Run {
        success: out.status.success(),
        output: format!("{stdout}\n{stderr}"),
        stderr,
    }
}

fn shipped_repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// An empty repository: an index and a `packs/` directory.
fn repository() -> TempDir {
    let repo = tempfile::tempdir().unwrap();
    fs::write(repo.path().join("index.json"), r#"{"packages":[]}"#).unwrap();
    fs::create_dir(repo.path().join("packs")).unwrap();
    repo
}

fn write_pack(repo: &Path, id: &str, manifest: &Value, files: &[(&str, &str)]) -> PathBuf {
    let dir = repo.join("packs").join(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("fugue.pkg.json"),
        serde_json::to_string_pretty(manifest).unwrap(),
    )
    .unwrap();
    for (name, content) in files {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    dir
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

fn skill_manifest(id: &str, extra: Value) -> Value {
    let mut manifest = json!({
        "id": id,
        "version": "0.1.0",
        "kind": "skill",
        "license": "CC0-1.0",
        "authors": [{ "name": "Example Engineer" }],
        "description": "A skill under test.",
        "targets": ["external-agent"],
        "entry": { "skill": "SKILL.md" }
    });
    for (key, value) in extra.as_object().unwrap() {
        manifest[key] = value.clone();
    }
    manifest
}

fn skill_text(name_line: &str, description_line: &str, body: &str) -> String {
    format!("---\n{name_line}\n{description_line}\n---\n\n{body}\n")
}

fn good_skill() -> String {
    skill_text(
        "name: pack-skill",
        "description: Shapes a phrase.",
        "# Skill",
    )
}

const SKILL_ID: &str = "fugue.test.pack-skill";

/// A repository with one skill pack holding `skill` as its `SKILL.md`.
fn repository_with_skill(skill: &str, extra: Value, files: &[(&str, &str)]) -> TempDir {
    let repo = repository();
    let mut all = vec![("SKILL.md", skill)];
    all.extend_from_slice(files);
    write_pack(
        repo.path(),
        SKILL_ID,
        &skill_manifest(SKILL_ID, extra),
        &all,
    );
    repo
}

fn assert_fails_naming(run: &Run, what: &[&str]) {
    assert!(!run.success, "the run should fail: {}", run.output);
    for needle in what {
        assert!(
            run.stderr.contains(needle),
            "the failure should name `{needle}`, but says: {}",
            run.stderr
        );
    }
}

// ---------------------------------------------------------------------------
// The shipped repository, and a clean temporary one.
// ---------------------------------------------------------------------------

#[test]
fn the_shipped_repository_passes_including_the_pulse_skill() {
    let run = check(&shipped_repository());
    assert!(
        run.success,
        "the shipped repository should pass: {}",
        run.output
    );
    assert!(
        shipped_repository()
            .join("packs/fugue.demo.pulse-skill/fugue.pkg.json")
            .is_file(),
        "the pulse skill is still a pack in the repository"
    );
}

#[test]
fn a_well_formed_skill_pack_passes() {
    let repo = repository_with_skill(&good_skill(), json!({}), &[]);
    let run = check(repo.path());
    assert!(run.success, "{}", run.output);
}

// ---------------------------------------------------------------------------
// Problems fail the run and are named.
// ---------------------------------------------------------------------------

#[test]
fn a_skill_pack_with_no_frontmatter_fails_the_run_naming_the_problem() {
    let repo = repository_with_skill("# A heading and no frontmatter\n", json!({}), &[]);
    assert_fails_naming(&check(repo.path()), &["SKILL.md"]);
}

#[test]
fn a_skill_pack_with_no_description_fails_the_run_naming_the_field() {
    let skill = "---\nname: pack-skill\n---\n\nBody\n";
    let repo = repository_with_skill(skill, json!({}), &[]);
    assert_fails_naming(&check(repo.path()), &["description"]);
}

#[test]
fn a_skill_pack_with_an_unsafe_name_fails_the_run_naming_the_field() {
    let skill = skill_text("name: Pack Skill", "description: Shapes a phrase.", "Body");
    let repo = repository_with_skill(&skill, json!({}), &[]);
    assert_fails_naming(&check(repo.path()), &["name"]);
}

#[cfg(unix)]
#[test]
fn a_skill_pack_whose_entry_is_a_symlink_fails_the_run_naming_the_path() {
    let repo = repository();
    let dir = write_pack(
        repo.path(),
        SKILL_ID,
        &skill_manifest(SKILL_ID, json!({})),
        &[("real/SKILL.md", &good_skill())],
    );
    // The target is also named SKILL.md, so only the link itself is wrong.
    std::os::unix::fs::symlink("real/SKILL.md", dir.join("SKILL.md")).unwrap();
    assert_fails_naming(&check(repo.path()), &["SKILL.md"]);
}

#[test]
fn a_missing_example_fails_the_run_naming_the_path() {
    let repo = repository_with_skill(
        &good_skill(),
        json!({ "examples": ["examples/absent.json"] }),
        &[],
    );
    assert_fails_naming(&check(repo.path()), &["examples/absent.json"]);
}

#[test]
fn an_example_that_is_not_an_invention_fails_the_run_naming_the_path() {
    let repo = repository_with_skill(
        &good_skill(),
        json!({ "examples": ["examples/bad.json"] }),
        &[("examples/bad.json", r#"{"title": "not an invention"}"#)],
    );
    assert_fails_naming(&check(repo.path()), &["examples/bad.json"]);
}

#[test]
fn a_skill_pack_with_valid_examples_passes() {
    let repo = repository_with_skill(
        &good_skill(),
        json!({ "examples": ["examples/song.json"] }),
        &[("examples/song.json", MINIMAL_INVENTION)],
    );
    let run = check(repo.path());
    assert!(run.success, "{}", run.output);
}

#[test]
fn examples_on_a_pack_that_is_not_a_skill_fail_the_run_naming_the_field() {
    let repo = repository();
    write_pack(
        repo.path(),
        POLICY,
        &json!({
            "id": POLICY,
            "version": "0.1.0",
            "kind": "policy",
            "license": "CC0-1.0",
            "authors": [{ "name": "Example Engineer" }],
            "description": "Shared rules.",
            "targets": ["external-agent"],
            "examples": ["examples/song.json"],
            "entry": { "policy": "POLICY.md" }
        }),
        &[
            ("POLICY.md", "# Rules\n"),
            ("examples/song.json", MINIMAL_INVENTION),
        ],
    );
    assert_fails_naming(&check(repo.path()), &["examples"]);
}

#[test]
fn a_bad_skill_fails_the_run_even_when_other_packs_are_fine() {
    let repo = repository();
    policy_pack(repo.path(), POLICY);
    write_pack(
        repo.path(),
        SKILL_ID,
        &skill_manifest(SKILL_ID, json!({})),
        &[("SKILL.md", "no frontmatter here\n")],
    );
    assert_fails_naming(&check(repo.path()), &["SKILL.md"]);
}

// ---------------------------------------------------------------------------
// A listed policy link prints no warning (skill_policy_links.rs covers the
// links that fail the run).
// ---------------------------------------------------------------------------

#[test]
fn a_link_to_a_listed_policy_prints_no_warning() {
    let repo = repository();
    policy_pack(repo.path(), POLICY);
    let link = format!("policies/{POLICY}.md");
    let skill = skill_text(
        "name: pack-skill",
        "description: Shapes a phrase.",
        &format!("Before shaping a phrase, read the [rules]({link})."),
    );
    write_pack(
        repo.path(),
        SKILL_ID,
        &skill_manifest(
            SKILL_ID,
            json!({ "policies": [format!("{POLICY}@^0.1.0")] }),
        ),
        &[("SKILL.md", &skill)],
    );
    let run = check(repo.path());
    assert!(run.success, "{}", run.output);
    assert!(
        !run.output.contains(&link),
        "nothing warns about a listed policy: {}",
        run.output
    );
}
