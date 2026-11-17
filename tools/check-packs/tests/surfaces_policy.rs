//! The surfaces policy, `fugue.policy.surfaces`, checked against the
//! repository's real `packs/`: a first-party policy pack for external agents
//! that says how to recognize the surface, which route plays a piece there,
//! what the browser preview lacks and to name the route in use. It stays
//! prompt-sized and leaves the authoring contract to `fugue.policy.authoring`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fugue::pkg::{check_policy_entry, parse_path};
use serde_json::Value;

const POLICY: &str = "fugue.policy.surfaces";
const AUTHORING: &str = "fugue.policy.authoring";
const MAX_CHARACTERS: usize = 3_000;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pack(id: &str) -> PathBuf {
    repository().join("packs").join(id)
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

/// The text under `heading`, up to the next `##` heading or the end.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let start = text
        .find(&format!("\n## {heading}\n"))
        .unwrap_or_else(|| panic!("no `## {heading}` section"))
        + 1;
    let rest = &text[start + 3..];
    &rest[..rest.find("\n## ").unwrap_or(rest.len())]
}

/// The text with its line breaks and indents collapsed to single spaces.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn position(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("the policy should say `{needle}`"))
}

/// The pack is a first-party policy pack for external agents only, with a
/// license file, a `#` title and no frontmatter, and the repository passes
/// check-packs with it.
#[test]
fn the_surfaces_policy_is_an_external_agent_policy_pack() {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string(pack(POLICY).join("fugue.pkg.json")).unwrap())
            .unwrap();
    assert_eq!(raw["license"], "MIT");
    assert_eq!(raw["targets"], serde_json::json!(["external-agent"]));
    assert!(pack(POLICY).join("LICENSE").is_file(), "ships a LICENSE");

    let text = rules(POLICY);
    assert!(text.starts_with("# "), "opens with a # title");
    assert!(!text.starts_with("---"), "has no frontmatter");

    let output = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(repository())
        .output()
        .expect("check-packs runs");
    assert!(
        output.status.success(),
        "check-packs failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The policy sits whole in a prompt.
#[test]
fn the_surfaces_policy_fits_the_size_limit() {
    let count = rules(POLICY).chars().count();
    assert!(
        count <= MAX_CHARACTERS,
        "{count} characters, over {MAX_CHARACTERS}"
    );
}

/// Surfaces are recognized from what the agent can observe, in order: local
/// tools, then only hosted tools, then a shell, then none of these.
#[test]
fn the_surface_is_recognized_in_order_from_observable_tools() {
    let text = rules(POLICY);
    let section = section(&text, "Recognize your surface");
    let local = position(section, "load_invention");
    let hosted = position(section, "check_invention");
    assert!(section.contains("play_preview"));
    let shell = position(section, "A shell is present");
    let none = position(section, "None of these");
    assert!(
        local < hosted && hosted < shell && shell < none,
        "{section}"
    );
    assert!(section.contains("Only the hosted tools"));
    assert!(section.contains("no Fyug tools"));
}

/// Each surface gets its route, and the routes use real tool names.
#[test]
fn each_surface_has_one_route() {
    let text = rules(POLICY);
    let routes = section(&text, "Take the route for your surface");
    let bullet = |label: &str| -> String {
        routes
            .split("\n- **")
            .find(|route| route.starts_with(&format!("{label}:**")))
            .map(flat)
            .unwrap_or_else(|| panic!("no route for `{label}`"))
    };

    let local = bullet("Local");
    assert!(local.contains("show_view") && local.contains("Never use the browser preview"));

    let machine = bullet("Shell on the user's machine");
    assert!(machine.contains("offer the native install"));
    assert!(
        machine.contains("not part of this policy"),
        "names the route only"
    );

    let cloud = bullet("Shell in a cloud session");
    assert!(cloud.contains("do not install for playback"));
    assert!(cloud.contains("fyug render") && cloud.contains("preview artifact"));

    let hosted = bullet("Hosted");
    assert!(position(&hosted, "check_invention") < position(&hosted, "play_preview"));
    assert!(hosted.contains("get_status"));

    let chat = bullet("Plain chat");
    assert!(chat.contains("preview artifact") && chat.contains("entry skill's script"));
    assert!(chat.contains("say what to connect"));
}

/// What no local engine cannot do is listed.
#[test]
fn what_the_preview_lacks_is_listed() {
    let text = rules(POLICY);
    let section = flat(section(&text, "Know what the browser preview lacks")).to_lowercase();
    for lacking in [
        "file, rtmp or youtube outputs",
        "hardware output",
        "live controls",
        "saving",
        "supported list",
    ] {
        assert!(section.contains(lacking), "should list `{lacking}`");
    }
}

/// The preview is called limited, the route is named, the native route is
/// offered and a change of route is never silent.
#[test]
fn the_agent_names_its_route_and_never_changes_it_silently() {
    let text = rules(POLICY);
    let section = flat(section(&text, "Say where you are"));
    for phrase in [
        "limited",
        "may drop out",
        "reduced feature set",
        "Tell the user which route you are using",
        "offer it",
        "Never change route without saying so",
    ] {
        assert!(section.contains(phrase), "should say `{phrase}`");
    }
}

/// A rule has one home: the surfaces policy names a tool the authoring
/// policy owns only to route to it (`load_invention`, `show_view`), and
/// copies no sentence from it.
#[test]
fn the_surfaces_policy_does_not_restate_the_authoring_policy() {
    let surfaces = rules(POLICY);
    let authoring = rules(AUTHORING);
    let words = |text: &str| -> BTreeSet<String> {
        text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .filter(|word| {
                word.contains('_') && word.chars().all(|c| c.is_ascii_lowercase() || c == '_')
            })
            .map(str::to_string)
            .collect()
    };
    let routed: BTreeSet<String> = ["load_invention", "show_view"].map(String::from).into();
    let shared: BTreeSet<String> = words(&surfaces)
        .intersection(&words(&authoring))
        .cloned()
        .collect();
    assert_eq!(shared, routed, "tools it shares with the authoring policy");

    let surfaces = flat(&surfaces);
    // Headings are dropped first, or one would stay glued to the front of the
    // first sentence under it and hide a copy of that sentence.
    let prose: String = authoring
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for sentence in flat(&prose).split(". ") {
        if sentence.split(' ').count() >= 6 {
            assert!(!surfaces.contains(sentence), "restates: {sentence}");
        }
    }
}

/// The README's Policies section lists the pack.
#[test]
fn the_readme_lists_the_surfaces_policy() {
    let readme = fs::read_to_string(repository().join("README.md")).unwrap();
    assert!(readme.contains(POLICY));
}
