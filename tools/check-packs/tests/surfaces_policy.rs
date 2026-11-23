//! The surfaces policy, `fugue.policy.surfaces`, checked against the
//! repository's real `packs/`: a first-party policy pack for external agents
//! that says how to recognize the surface, which route plays a piece there,
//! how the hosted server's in-chat player is used on the web and on a phone,
//! what the hosted server leaves to the desktop app, how a piece is kept, and
//! to name the route in use. It stays prompt-sized and leaves the authoring
//! contract to `fugue.policy.authoring`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fugue::pkg::{check_policy_entry, parse_path};
use serde_json::Value;

const POLICY: &str = "fugue.policy.surfaces";
const AUTHORING: &str = "fugue.policy.authoring";
/// Room for the hosted route: the in-chat player, its web and phone
/// differences, approvals, keeping a piece and what is left to the desktop
/// app. It was 3,000 when the hosted route was a one-line preview.
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
    assert!(hosted.contains("in-chat player"));

    let chat = bullet("Plain chat");
    assert!(chat.contains("preview artifact") && chat.contains("entry skill's script"));
    assert!(chat.contains("say what to connect"));
}

/// The player: check, then load with bundled instruments and a few controls;
/// read the levels and notes before calling it ready; change it through its
/// tools, with a full document only where none are offered; the person's
/// values stay. Every tool it names is the hosted server's or the player's.
#[test]
fn the_in_chat_player_is_loaded_read_and_changed_through_its_tools() {
    let text = rules(POLICY);
    let section = flat(section(&text, "The in-chat player"));
    assert!(position(&section, "check_invention") < position(&section, "load_invention"));
    for phrase in [
        "bundled instruments (list_developments)",
        "two to six values",
        "get_meters",
        "get_notes",
        "before calling it ready",
        "silence, clipping, a part out of range",
        "set_controls for values and apply_edits",
        "rather than sending it again",
        "offers no player tools even then, send a changed document to load_invention",
        "list_changed_controls",
    ] {
        assert!(section.contains(phrase), "should say `{phrase}`: {section}");
    }
    let hosted_own = ["check_invention", "load_invention", "list_developments"];
    for word in section.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if word.contains('_') {
            assert!(
                hosted_own.contains(&word) || PLAYER_TOOLS.contains(&word),
                "`{word}` is not the hosted server's or the player's"
            );
        }
    }
}

/// The phone: no player tool in the opening turn, the levels and notes after
/// the next message, never "checked" early; and a call not approved is
/// explained once and not retried.
#[test]
fn a_phone_waits_for_the_next_message_and_an_unapproved_call_is_not_retried() {
    let text = rules(POLICY);
    let section = flat(section(&text, "The in-chat player"));
    for phrase in [
        "On the web the player sits beside the chat",
        "offers downloads",
        "If the player's tools are not offered once load_invention returns, as on a phone, they come with the person's next message",
        "plan no player tool call in this turn and do not send the piece again",
        "ask them to tap Play",
        "Never call it checked before you have read them",
        "On a phone sound stops when the player is off screen",
        "say what you were about to do",
        "point to the same control in the player",
        "Tell the person once",
        "always allow",
        "Do not retry unless they ask",
    ] {
        assert!(section.contains(phrase), "should say `{phrase}`: {section}");
    }
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
        "saving a piece to disk",
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
}

/// A piece is kept by its download where there is one, its share link
/// otherwise; the agent never posts the link; a phone piece with samples
/// cannot be kept, and the policy says so.
#[test]
fn a_piece_is_kept_by_download_or_share_link_never_posted() {
    let text = rules(POLICY);
    let section = flat(section(&text, "Keep a piece"));
    for phrase in [
        "Download piece where the player shows it",
        "Share link otherwise",
        "anyone with the link has the piece",
        "Never post a share link yourself",
        "bundled samples has no Share link",
        "cannot be kept",
    ] {
        assert!(section.contains(phrase), "should say `{phrase}`: {section}");
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
        "offer the desktop app",
        "On the preview artifact, offer the hosted connector",
        "Never change route without saying so",
    ] {
        assert!(section.contains(phrase), "should say `{phrase}`");
    }
}

/// A rule has one home: the surfaces policy names a tool the authoring
/// policy owns only to route to it (the local tools that tell the surface,
/// and the player's tools the hosted route uses, which share the local
/// names), and copies no sentence from it.
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
    let shared: BTreeSet<String> = words(&surfaces)
        .intersection(&words(&authoring))
        .cloned()
        .collect();
    let unrouted: Vec<&String> = shared.difference(&routed).collect();
    assert!(
        unrouted.is_empty(),
        "tools it shares with the authoring policy: {unrouted:?}"
    );

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
