//! The surfaces policy, `fugue.policy.surfaces`, checked against the
//! repository's real `packs/`: a first-party policy pack for external agents
//! that says how to recognize the surface, which route plays a piece there,
//! what the hosted server leaves to the desktop app, and to name the route in
//! use. How the in-chat player is used is `fugue.policy.player`'s, and how a
//! piece is kept or opened is `fugue.policy.files`'s: the three were one
//! policy, split so an agent opens only the one its task calls for. It stays
//! prompt-sized and leaves the authoring contract to `fugue.policy.authoring`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fugue::pkg::{check_policy_entry, parse_path};
use serde_json::Value;

const POLICY: &str = "fugue.policy.surfaces";
const AUTHORING: &str = "fugue.policy.authoring";
/// The policies the surfaces policy was split into, itself among them.
const SPLIT: [&str; 3] = [POLICY, "fugue.policy.player", "fugue.policy.files"];
/// The surfaces policy's `##` rules before the split, each now in exactly one
/// of `SPLIT`.
const RULES_BEFORE_THE_SPLIT: [&str; 6] = [
    "Recognize your surface",
    "Take the route for your surface",
    "The in-chat player",
    "What the hosted server leaves to the desktop app",
    "Keep a piece",
    "Say where you are",
];
/// The limit from before the split, not raised; the surfaces policy now
/// holds far less.
const MAX_CHARACTERS: usize = 4_600;

/// The in-chat player's tools: the hosted server's `load_invention` opens a
/// page that offers them, with the local tools' names (`crates/hosted/src/
/// player/tools.js` in fyug-product at d10777d1).
const PLAYER_TOOLS: [&str; 11] = [
    "list_controls",
    "get_control",
    "set_control",
    "set_controls",
    "list_changed_controls",
    "get_notes",
    "get_meters",
    "apply_edits",
    "get_invention",
    "pause_invention",
    "resume_invention",
];

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
    let parts: Vec<u32> = manifest
        .version
        .split('.')
        .map(|part| {
            part.parse()
                .unwrap_or_else(|_| panic!("{id}: {}", manifest.version))
        })
        .collect();
    assert!(
        parts.len() == 3 && parts[0] >= 2026,
        "{id}: {}",
        manifest.version
    );
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
/// tools, then hosted tools, then a shell, then none of these. Both servers
/// offer load_invention, so each is told apart by a tool only it has: the
/// local one by reload_invention and show_view, the hosted one by
/// check_invention.
#[test]
fn the_surface_is_recognized_in_order_from_observable_tools() {
    let text = rules(POLICY);
    let section = flat(section(&text, "Recognize your surface"));
    let local = position(
        &section,
        "1. The local Fyug tools are present (reload_invention, show_view)",
    );
    let hosted = position(
        &section,
        "2. The hosted tools are present (check_invention)",
    );
    let shell = position(&section, "3. A shell is present");
    let none = position(&section, "4. None of these");
    assert!(
        local < hosted && hosted < shell && shell < none,
        "{section}"
    );
    assert!(section.contains("no Fyug tools"));
    assert!(!section.contains("play_preview"), "the old hosted tool");
}

/// Each surface's tools alone, and both sets together, land on one surface:
/// the first rule whose tools are present, as the policy reads them.
#[test]
fn each_set_of_tools_lands_on_its_surface_and_both_land_on_local() {
    let text = rules(POLICY);
    let section = flat(section(&text, "Recognize your surface"));
    // The tools each numbered rule names, in order.
    let starts: Vec<usize> = [
        "1. The local",
        "2. The hosted",
        "3. A shell is present",
        "4. None of these",
        "A shell is on",
    ]
    .iter()
    .map(|marker| position(&section, marker))
    .collect();
    let rules: Vec<(String, Vec<&str>)> = starts
        .windows(2)
        .map(|pair| {
            let rule = section[pair[0]..pair[1]].to_string();
            let tools = ["reload_invention", "show_view", "check_invention"]
                .into_iter()
                .filter(|tool| rule.contains(tool))
                .collect();
            (rule, tools)
        })
        .collect();
    let surface = |present: &[&str]| -> usize {
        rules
            .iter()
            .position(|(_, tools)| !tools.is_empty() && tools.iter().all(|t| present.contains(t)))
            .unwrap_or(2)
    };
    let local = [
        "load_invention",
        "reload_invention",
        "show_view",
        "get_status",
    ];
    let hosted = [
        "load_invention",
        "check_invention",
        "get_status",
        "set_controls",
    ];
    let both: Vec<&str> = local.iter().chain(&hosted).copied().collect();
    assert_eq!(surface(&local), 0, "local tools alone: {rules:?}");
    assert_eq!(surface(&hosted), 1, "hosted tools alone: {rules:?}");
    assert_eq!(surface(&both), 0, "both sets: {rules:?}");
    assert!(
        rules[0]
            .0
            .contains("When both sets of tools are present, this one wins"),
        "{rules:?}"
    );
    assert!(rules[1].0.contains("without the local ones"), "{rules:?}");
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
    assert!(local.contains("show_view") && local.contains("Never use the in-chat player"));

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
    assert!(hosted.contains("in-chat player") && hosted.contains("fugue.policy.player"));

    let chat = bullet("Plain chat");
    assert!(chat.contains("preview artifact") && chat.contains("entry skill's script"));
    assert!(chat.contains("say what to connect"));
}

/// What the hosted server leaves to the desktop app is listed, and the app is
/// suggested only on a request for one of them.
#[test]
fn what_the_hosted_server_leaves_to_the_desktop_app_is_listed() {
    let text = rules(POLICY);
    let section = flat(section(
        &text,
        "What the hosted server leaves to the desktop app",
    ))
    .to_lowercase();
    assert!(section.contains(
        "suggest the desktop app only when the person asks for one of these, and name it, or when the player fails"
    ));
    for lacking in [
        "keeps playing in the background, where a phone's player stops off screen",
        "a library of installed packages",
        "their own samples",
        "file, rtmp or youtube outputs",
        "hardware output",
        "a module type the player does not build",
        "past the hosted limits",
        "no player open",
        "get_status",
    ] {
        assert!(section.contains(lacking), "should list `{lacking}`");
    }
    assert!(
        section.contains("otherwise do not raise it on this route"),
        "should keep the app out of the player otherwise"
    );
    // Download piece saves the person's file, so saving is not the app's.
    assert!(!section.contains("saving"));
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
        "offer the desktop app",
        "On the preview artifact, offer the hosted connector",
        "Never change route without saying so",
    ] {
        assert!(section.contains(phrase), "should say `{phrase}`");
    }
}

/// A rule has one home: the surfaces policy and the two split from it name a
/// tool the authoring policy owns only to route to it (the local tools that
/// tell the surface, and the player's tools the hosted route uses, which
/// share the local names), and copy no sentence from it.
#[test]
fn the_split_policies_do_not_restate_the_authoring_policy() {
    let authoring = rules(AUTHORING);
    let words = |text: &str| -> BTreeSet<String> {
        text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .filter(|word| {
                word.contains('_') && word.chars().all(|c| c.is_ascii_lowercase() || c == '_')
            })
            .map(str::to_string)
            .collect()
    };
    let routed: BTreeSet<String> = [
        "load_invention",
        "reload_invention",
        "show_view",
        "list_developments",
    ]
    .into_iter()
    .chain(PLAYER_TOOLS)
    .map(String::from)
    .collect();
    // Headings are dropped first, or one would stay glued to the front of the
    // first sentence under it and hide a copy of that sentence.
    let prose: String = authoring
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for id in SPLIT {
        let text = rules(id);
        let shared: BTreeSet<String> = words(&text)
            .intersection(&words(&authoring))
            .cloned()
            .collect();
        let unrouted: Vec<&String> = shared.difference(&routed).collect();
        assert!(
            unrouted.is_empty(),
            "{id} shares tools with the authoring policy: {unrouted:?}"
        );
        let text = flat(&text);
        for sentence in flat(&prose).split(". ") {
            if sentence.split(' ').count() >= 6 {
                assert!(!text.contains(sentence), "{id} restates: {sentence}");
            }
        }
    }
}

/// The README's Policies section lists the pack.
#[test]
fn the_readme_lists_the_surfaces_policy() {
    let readme = fs::read_to_string(repository().join("README.md")).unwrap();
    for id in SPLIT {
        assert!(readme.contains(id), "the README omits {id}");
    }
}

/// Each rule of the policy before the split is in exactly one of the
/// policies it was split into, and each of those is a pack under its limit.
#[test]
fn each_rule_from_before_the_split_has_one_home() {
    let texts: Vec<String> = SPLIT.iter().map(|id| rules(id)).collect();
    for rule in RULES_BEFORE_THE_SPLIT {
        let heading = format!("\n## {rule}\n");
        let homes: Vec<&str> = SPLIT
            .iter()
            .zip(&texts)
            .filter(|(_, text)| text.contains(&heading))
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(homes.len(), 1, "`{rule}` is in {homes:?}");
    }
    for (id, text) in SPLIT.iter().zip(&texts) {
        let count = text.chars().count();
        assert!(count <= MAX_CHARACTERS, "{id}: {count} characters");
    }
}
