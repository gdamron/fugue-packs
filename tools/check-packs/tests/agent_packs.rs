//! check-packs runs the agent package checks on every agent pack, and
//! resolves each agent as a host would; the shipped pulse agent composes its
//! skill and the shared policies, each once.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fugue::pkg::{self, PromptSource};
use serde_json::{json, Value};
use tempfile::TempDir;

const AGENT: &str = "fugue.test.agent";
const SKILL: &str = "fugue.test.skill";

fn shipped_packs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs")
}

/// Run check-packs on `root`: whether it passed, and everything it printed.
fn check(root: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(root)
        .output()
        .expect("check-packs runs");
    let output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), output)
}

fn write_pack(repo: &Path, id: &str, manifest: Value, files: &[(&str, &str)]) {
    let dir = repo.join("packs").join(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("fugue.pkg.json"), manifest.to_string()).unwrap();
    for (name, content) in files {
        fs::write(dir.join(name), content).unwrap();
    }
}

fn manifest(id: &str, kind: &str, targets: &[&str], extra: Value) -> Value {
    let entry = match kind {
        "agent" => json!({"definition": "agent.json"}),
        _ => json!({"skill": "SKILL.md"}),
    };
    let mut manifest = json!({
        "id": id, "version": "0.1.0", "kind": kind, "license": "CC0-1.0",
        "authors": [{"name": "Example Engineer"}], "targets": targets, "entry": entry,
    });
    for (key, value) in extra.as_object().unwrap() {
        manifest[key] = value.clone();
    }
    manifest
}

/// A repository holding an agent with `definition` that composes one skill
/// targeting `skill_targets`.
fn repository(definition: &str, skill_targets: &[&str]) -> TempDir {
    let repo = tempfile::tempdir().unwrap();
    fs::write(repo.path().join("index.json"), r#"{"packages":[]}"#).unwrap();
    write_pack(
        repo.path(),
        SKILL,
        manifest(SKILL, "skill", skill_targets, json!({})),
        &[(
            "SKILL.md",
            "---\nname: test-skill\ndescription: Shapes a phrase.\n---\n\n# Skill\n",
        )],
    );
    write_pack(
        repo.path(),
        AGENT,
        manifest(
            AGENT,
            "agent",
            &["in-graph-agent"],
            json!({"skills": [format!("{SKILL}@^0.1.0")]}),
        ),
        &[("agent.json", definition)],
    );
    repo
}

#[test]
fn the_shipped_pulse_agent_composes_each_policy_once() {
    let packs = shipped_packs();
    let agent = pkg::resolve_agent_package(&packs.join("fugue.demo.pulse-agent"), &packs)
        .expect("the pulse agent resolves");
    let parts: Vec<_> = agent
        .parts
        .iter()
        .map(|part| (part.source, part.package.as_str()))
        .collect();
    assert_eq!(
        parts,
        [
            (PromptSource::Persona, "fugue.demo.pulse-agent"),
            (PromptSource::Policy, "fugue.policy.clean-audio"),
            (PromptSource::Policy, "fugue.policy.musical-intent"),
            (PromptSource::Skill, "fugue.demo.pulse-skill"),
        ]
    );
    for heading in ["# Clean Audio", "# Musical Intent", "# Pulse Study"] {
        assert_eq!(
            agent.system_prompt.matches(heading).count(),
            1,
            "{heading} appears once"
        );
    }
    assert!(!agent.system_prompt.contains("name: fugue-demo-pulse"));
    assert_eq!(agent.model, None);
    assert_eq!(agent.tools, ["fugue.set_control"]);
}

#[test]
fn a_good_agent_passes_and_passes_again() {
    let repo = repository(r#"{"system_prompt": "Persona."}"#, &["in-graph-agent"]);
    for run in 0..2 {
        let (passed, output) = check(repo.path());
        assert!(passed, "run {run}: {output}");
    }
}

#[test]
fn refuses_a_definition_field_naming_it_and_the_file() {
    for field in ["skills", "policies", "kind"] {
        let definition = json!({"system_prompt": "Persona.", field: []}).to_string();
        let repo = repository(&definition, &["in-graph-agent"]);
        let (passed, output) = check(repo.path());
        assert!(!passed, "{field}: {output}");
        assert!(
            output.contains(&format!("`{field}`")) && output.contains("agent.json"),
            "{output}"
        );
    }
}

#[test]
fn refuses_a_missing_or_empty_prompt() {
    for definition in [r#"{"default_tools": []}"#, r#"{"system_prompt": " "}"#] {
        let repo = repository(definition, &["in-graph-agent"]);
        let (passed, output) = check(repo.path());
        assert!(!passed && output.contains("`system_prompt`"), "{output}");
    }
}

#[test]
fn refuses_a_missing_definition_naming_the_path() {
    let repo = repository("{}", &["in-graph-agent"]);
    fs::remove_file(repo.path().join("packs").join(AGENT).join("agent.json")).unwrap();
    let (passed, output) = check(repo.path());
    assert!(!passed && output.contains("agent.json"), "{output}");
}

#[test]
fn refuses_a_skill_for_external_agents_only_naming_it() {
    let repo = repository(r#"{"system_prompt": "Persona."}"#, &["external-agent"]);
    let (passed, output) = check(repo.path());
    assert!(
        !passed && output.contains(SKILL) && output.contains("in-graph-agent"),
        "{output}"
    );
}
