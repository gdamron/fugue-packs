//! The in-chat player policy, `fugue.policy.player`, checked against the
//! repository's real `packs/`: how a piece is opened, read and changed in the
//! hosted server's in-chat player, on the web and on a phone. Split from the
//! surfaces policy so an agent opens it only when it uses the player.

mod policy_support;

use std::fs;

use policy_support::{british_spellings, flat, pack, policy_text, repository};
use serde_json::Value;

const POLICY: &str = "fugue.policy.player";
/// The same limit as the other prompt-sized policies.
const MAX_CHARACTERS: usize = 3_000;

/// The in-chat player's tools: the hosted server's `load_invention` opens a
/// page that offers them, with the local tools' names (`crates/hosted/src/
/// player/tools.js` in fyug-product at 5990afad).
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

/// The text under `## heading`, up to the next `##` heading or the end.
fn section(text: &str, heading: &str) -> String {
    let marker = format!("\n## {heading}\n");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("no `## {heading}` section"))
        + marker.len();
    let rest = &text[start..];
    flat(&rest[..rest.find("\n## ").unwrap_or(rest.len())])
}

/// A first-party policy pack for external agents, prompt-sized, in American
/// English, listed in the README.
#[test]
fn the_player_policy_is_a_small_external_agent_policy_pack() {
    let raw: Value =
        serde_json::from_str(&fs::read_to_string(pack(POLICY).join("fugue.pkg.json")).unwrap())
            .unwrap();
    assert_eq!(raw["license"], "MIT");
    assert_eq!(raw["targets"], serde_json::json!(["external-agent"]));
    assert!(pack(POLICY).join("LICENSE").is_file(), "ships a LICENSE");
    let text = policy_text(POLICY);
    assert!(text.starts_with("# "), "opens with a # title");
    let count = text.chars().count();
    assert!(count <= MAX_CHARACTERS, "{count} characters");
    assert!(british_spellings(&text, &[]).is_empty());
    let readme = fs::read_to_string(repository().join("README.md")).unwrap();
    assert!(readme.contains(POLICY), "the README omits {POLICY}");
}

/// The player: check, then load with bundled instruments and a few controls;
/// read the levels and notes before calling it ready; change it through its
/// tools, with a full document only where none are offered; the person's
/// values stay. Every tool it names is the hosted server's or the player's.
#[test]
fn the_in_chat_player_is_loaded_read_and_changed_through_its_tools() {
    let section = section(&policy_text(POLICY), "The in-chat player");
    assert!(section.find("check_invention").unwrap() < section.find("load_invention").unwrap());
    for phrase in [
        "bundled instruments (list_developments)",
        "two to six values",
        "get_meters",
        "get_notes",
        "before calling it ready",
        "silence, clipping, a part out of range",
        "set_controls for values and apply_edits",
        "rather than sending it again",
        "offers no player tools at all, send a changed document to load_invention",
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

/// The phone: no player tool in the opening turn; the tools may come with the
/// next message or a later one, and until then its levels are unread and the
/// tools are looked for again, never called missing; and a call not approved
/// is explained once and not retried.
#[test]
fn a_phone_waits_for_the_next_message_and_an_unapproved_call_is_not_retried() {
    let section = section(&policy_text(POLICY), "The in-chat player");
    for phrase in [
        "On the web the player sits beside the chat",
        "offers downloads",
        "On a phone the player's tools may not be offered once load_invention returns: they come with the person's next message or a later one",
        "plan no player tool call in this turn and do not resend the piece",
        "ask them to tap Play",
        "Until they are listed, say its levels are not read yet, look again each turn, and never say the app or player lacks them",
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

/// The player's get_notes needs `bars` (its schema requires it), and this
/// policy is the one place that says so, so a default shipping later is a
/// one-line change here.
#[test]
fn get_notes_is_called_with_bars_and_only_this_policy_says_how() {
    let notes = section(&policy_text(POLICY), "Read the notes");
    for phrase in ["get_notes with `bars`", "1 to 64", "cursor"] {
        assert!(notes.contains(phrase), "should say `{phrase}`: {notes}");
    }
    let packs = fs::read_dir(repository().join("packs")).unwrap();
    for entry in packs.map(Result::unwrap) {
        let id = entry.file_name().to_string_lossy().to_string();
        if id == POLICY {
            continue;
        }
        for file in ["POLICY.md", "SKILL.md"] {
            let Ok(text) = fs::read_to_string(entry.path().join(file)) else {
                continue;
            };
            let text = flat(&text);
            assert!(
                !text.contains("`bars`") && !text.contains("get_notes with"),
                "{id} says how to call get_notes; only {POLICY} does"
            );
        }
    }
}
