//! The authoring policy pack, `fugue.policy.authoring`, checked against the
//! repository's real `packs/` through the checker binary and the core's
//! package API:
//!
//! - it holds the authoring contract as a first-party policy pack (kind
//!   `policy`, a markdown `entry.policy`, version `2026.11.1`, both agent
//!   targets, a `#` title and no frontmatter, American English) and the
//!   repository passes check-packs with it;
//! - anything specific to the MCP transport sits under one clearly headed
//!   MCP-only section;
//! - a skill can require the policy by id, and a requirement no version of it
//!   satisfies is refused.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fugue::pkg::{check_policy_entry, parse_path};
use serde_json::{json, Value};

const POLICY: &str = "fugue.policy.authoring";
const VERSION: &str = "2026.11.1";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pack() -> PathBuf {
    repository().join("packs").join(POLICY)
}

fn raw_manifest() -> Value {
    let path = pack().join("fugue.pkg.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{POLICY}: cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{POLICY}: manifest JSON: {e}"))
}

/// The policy's markdown entry, as the manifest names it.
fn policy_text() -> String {
    let entry = raw_manifest()["entry"]["policy"]
        .as_str()
        .unwrap_or_else(|| panic!("{POLICY}: `entry.policy` names no file"))
        .to_string();
    fs::read_to_string(pack().join(&entry))
        .unwrap_or_else(|e| panic!("{POLICY}: cannot read {entry}: {e}"))
}

/// Runs the checker binary on `root`, as CI's "Validate packs" step does.
fn check_packs(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(root)
        .output()
        .expect("check-packs runs")
}

fn report(output: &std::process::Output) -> String {
    format!(
        "status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Markdown headings outside fenced code: (line index, level, text).
fn headings(text: &str) -> Vec<(usize, usize, String)> {
    let mut fenced = false;
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let level = line.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&level) && line[level..].starts_with(' ') {
            found.push((index, level, line[level..].trim().to_string()));
        }
    }
    found
}

/// the pack is a first-party policy pack with the shared
/// content rules: kind `policy`, `entry: { "policy": "<file>.md" }`, date
/// version 2026.11.0, both targets, and an entry the core's policy check
/// accepts.
#[test]
fn the_authoring_policy_is_a_first_party_policy_pack() {
    let manifest = parse_path(pack().join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{POLICY} should validate: {e}"));
    assert_eq!(manifest.id, POLICY);
    assert_eq!(manifest.kind.as_str(), "policy");
    assert_eq!(
        manifest.version, VERSION,
        "first-party policy packs use the date version"
    );

    let raw = raw_manifest();
    let entry = raw["entry"]["policy"].as_str().unwrap_or_default();
    assert!(
        entry.ends_with(".md"),
        "entry.policy should be a markdown file, got {entry:?}"
    );
    let targets: Vec<&str> = raw["targets"]
        .as_array()
        .unwrap_or_else(|| panic!("{POLICY} lists no targets"))
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for target in ["external-agent", "in-graph-agent"] {
        assert!(
            targets.contains(&target),
            "{POLICY} should target {target}: {targets:?}"
        );
    }

    check_policy_entry(&pack(), &manifest)
        .unwrap_or_else(|e| panic!("{POLICY}'s policy file should pass the core check: {e}"));
}

/// the policy file opens with a `#` title and has no
/// frontmatter.
#[test]
fn the_policy_opens_with_a_title_and_has_no_frontmatter() {
    let text = policy_text();
    let first = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    assert!(
        first.starts_with("# "),
        "the policy should open with a `#` title, not {first:?}"
    );
    assert!(
        !text.trim_start().starts_with("---"),
        "the policy has frontmatter"
    );
}

/// the whole repository, the authoring policy included, passes
/// check-packs (`cargo run ... -- .`).
#[test]
fn the_repository_passes_check_packs_with_the_policy() {
    assert!(
        pack().join("fugue.pkg.json").is_file(),
        "{POLICY} is missing"
    );
    let output = check_packs(&repository());
    assert!(
        output.status.success(),
        "check-packs failed:\n{}",
        report(&output)
    );
}

/// the full contract is there. Each rule is its own `##`
/// section, so a contract with a handful of sections is not a stub; and it
/// covers the core of authoring (a whole document, its revision, controls).
#[test]
fn the_policy_holds_the_authoring_contract() {
    let text = policy_text();
    let sections = headings(&text)
        .iter()
        .filter(|(_, level, _)| *level == 2)
        .count();
    assert!(
        sections >= 5,
        "the policy has only {sections} `##` sections"
    );
    for tool in [
        "load_invention",
        "reload_invention",
        "get_invention",
        "expected_revision",
    ] {
        assert!(text.contains(tool), "the policy never mentions {tool}");
    }
}

/// American English. A few British spellings likely in prose
/// about authoring and audio.
#[test]
fn the_policy_uses_american_spelling() {
    const BRITISH: [&str; 22] = [
        "colour",
        "behaviour",
        "favour",
        "honour",
        "centre",
        "analyse",
        "organise",
        "recognise",
        "realise",
        "normalise",
        "optimise",
        "serialise",
        "initialise",
        "synchronise",
        "summarise",
        "licence",
        "labelled",
        "cancelled",
        "modelled",
        "travelled",
        "catalogue",
        "grey",
    ];
    let lower = policy_text().to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| !w.is_empty())
        .collect();
    let found: Vec<&str> = BRITISH
        .iter()
        .copied()
        .filter(|british| {
            words.iter().any(|w| {
                w.starts_with(british) && {
                    // "labelled", "colours", "organised": the stem plus an
                    // inflection.
                    let rest = &w[british.len()..];
                    ["", "s", "d", "ed", "es", "ing", "r", "rs"].contains(&rest)
                }
            })
        })
        .collect();
    assert!(
        found.is_empty(),
        "British spellings in the policy: {found:?}"
    );
}

/// anything specific to the MCP transport (file delivery on
/// the MCP host, `json` as a string) sits under one clearly headed
/// MCP-only section. The section is the one heading that names MCP; it
/// states both MCP-only rules, and the rest of the policy does not mention
/// the MCP host.
#[test]
fn mcp_only_rules_sit_under_one_mcp_section() {
    let text = policy_text();
    let lines: Vec<&str> = text.lines().collect();
    let all = headings(&text);
    let mcp: Vec<_> = all
        .iter()
        .filter(|(_, _, title)| title.contains("MCP"))
        .collect();
    assert_eq!(
        mcp.len(),
        1,
        "expected exactly one heading naming MCP, found {:?}",
        mcp.iter().map(|(_, _, title)| title).collect::<Vec<_>>()
    );
    let &&(start, level, ref title) = mcp.first().unwrap();
    let end = all
        .iter()
        .find(|(index, other, _)| *index > start && *other <= level)
        .map_or(lines.len(), |(index, _, _)| *index);
    let section = lines[start..end].join("\n");
    let outside = [&lines[..start], &lines[end..]].concat().join("\n");

    assert!(
        section.contains("MCP host"),
        "the {title:?} section should say file delivery is on the MCP host"
    );
    assert!(
        section.contains("delivery"),
        "the {title:?} section should cover get_invention file delivery"
    );
    let json_as_string = section
        .lines()
        .any(|line| line.contains("json") && line.to_lowercase().contains("string"));
    assert!(
        json_as_string,
        "the {title:?} section should say `json` is passed as a string"
    );
    assert!(
        !outside.contains("MCP host"),
        "the MCP host is mentioned outside the {title:?} section"
    );
}

/// Writes a skill pack into `root/packs` that requires `requirement`.
fn write_skill_requiring(root: &Path, requirement: &str) {
    let dir = root.join("packs").join("fugue.test.authoring-skill");
    fs::create_dir_all(&dir).unwrap();
    let manifest = json!({
        "id": "fugue.test.authoring-skill",
        "version": "0.1.0",
        "kind": "skill",
        "license": "CC0-1.0",
        "authors": [{ "name": "Test Author" }],
        "targets": ["external-agent"],
        "entry": { "skill": "SKILL.md" },
        "policies": [requirement]
    });
    fs::write(
        dir.join("fugue.pkg.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!(
            "---\nname: test-authoring-skill\ndescription: A skill that requires the authoring policy.\n---\n\n# Test authoring skill\n\nBefore writing a piece, read [authoring](policies/{POLICY}.md).\n"
        ),
    )
    .unwrap();
}

/// A repository holding the real authoring policy pack and nothing else.
fn repository_with_the_policy() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::copy(
        repository().join("index.json"),
        root.path().join("index.json"),
    )
    .unwrap();
    let to = root.path().join("packs").join(POLICY);
    fs::create_dir_all(&to).unwrap();
    for file in fs::read_dir(pack()).unwrap_or_else(|e| panic!("{POLICY} is missing: {e}")) {
        let file = file.unwrap();
        if file.file_type().unwrap().is_file() {
            fs::copy(file.path(), to.join(file.file_name())).unwrap();
        }
    }
    root
}

/// Non-goal boundary, "authoring skills require the policy": a skill can
/// require the policy by id and its date version, and check-packs resolves it.
#[test]
fn a_skill_can_require_the_policy_by_id() {
    let root = repository_with_the_policy();
    write_skill_requiring(root.path(), &format!("{POLICY}@^{VERSION}"));
    let output = check_packs(root.path());
    assert!(
        output.status.success(),
        "requiring {POLICY} failed:\n{}",
        report(&output)
    );
}

/// Its failure edge: a requirement no version of the policy satisfies is
/// refused, naming the policy.
#[test]
fn a_requirement_the_policy_cannot_satisfy_is_refused() {
    let root = repository_with_the_policy();
    write_skill_requiring(root.path(), &format!("{POLICY}@^2099.1.0"));
    let output = check_packs(root.path());
    assert!(
        !output.status.success(),
        "an unsatisfiable requirement passed"
    );
    assert!(
        report(&output).contains(POLICY),
        "the refusal should name {POLICY}:\n{}",
        report(&output)
    );
}

/// The text under `## heading`, up to the next `##` heading, on one line.
fn rule(heading: &str) -> String {
    let text = policy_text();
    let marker = format!("\n## {heading}\n");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("no `## {heading}` rule"))
        + marker.len();
    let rest = &text[start..];
    let body = &rest[..rest.find("\n## ").unwrap_or(rest.len())];
    body.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Structure goes through apply_edits and values through set_controls, on
/// both hosts; apply_edits is no longer offered for retuning values.
#[test]
fn structure_goes_through_apply_edits_and_values_through_set_controls() {
    let edits = rule("Targeted edits to large inventions");
    assert!(edits.contains("On both hosts"), "{edits}");
    assert!(
        edits.contains("structure with apply_edits") && edits.contains("values with set_controls"),
        "{edits}"
    );
    assert!(!flat_text().contains("several starting values retuned together"));
}

fn flat_text() -> String {
    policy_text()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// What a script may do in the hosted in-chat player, as the product ships
/// it (fyug-product docs/hosted-server.md, "Scripts in the player").
#[test]
fn the_scripts_rule_says_what_a_script_does_in_the_player() {
    let scripts = rule("Scripts");
    for phrase in [
        "scripts run, in a sandbox with no network access",
        "runs_in_player",
        "not in the saved, shared or downloaded piece",
        "cannot add a script or agent module",
        "5 s to start and 1 s per tick",
        "overruns or throws is stopped until the piece is loaded again",
        "64 KiB",
        "On the hosted server an agent module does not run",
        "load_invention names them under not_run",
        "nor in get_invention",
        "1,000 graph calls a second and 256 structural edits per hook",
    ] {
        assert!(scripts.contains(phrase), "should say `{phrase}`: {scripts}");
    }
}
