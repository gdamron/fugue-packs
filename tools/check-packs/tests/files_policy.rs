//! The files policy, `fugue.policy.files`, checked against the repository's
//! real `packs/`: how the person keeps or shares a piece from the in-chat
//! player, and how a downloaded `.fyug` file is opened. Split from the
//! surfaces policy so an agent opens it only when a file is in play.

mod policy_support;

use std::fs;

use policy_support::{british_spellings, flat, pack, policy_text, repository};
use serde_json::Value;

const POLICY: &str = "fugue.policy.files";
/// The same limit as the other prompt-sized policies.
const MAX_CHARACTERS: usize = 3_000;

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
fn the_files_policy_is_a_small_external_agent_policy_pack() {
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

/// A piece is kept by the player's two download buttons where there are
/// any, its share link otherwise; the agent never posts the link; a phone
/// piece with samples cannot be kept, and the policy says so. Nothing is
/// converted: the buttons save what the person keeps.
#[test]
fn a_piece_is_kept_by_its_two_downloads_or_a_share_link_never_posted() {
    let section = section(&policy_text(POLICY), "Keep a piece");
    for phrase in [
        "where it shows them: Download piece (a file on disk",
        "Download recording",
        "Point to Share link otherwise",
        "anyone with the link has the piece",
        "Never post a share link yourself",
        "bundled samples has no Share link",
        "cannot be kept",
    ] {
        assert!(section.contains(phrase), "should say `{phrase}`: {section}");
    }
    let lower = policy_text(POLICY).to_lowercase();
    for word in ["ffmpeg", "convert"] {
        assert!(!lower.contains(word), "the policy says `{word}`");
    }
}

/// A `.fyug` is opened by its path with `load_invention`, never read as
/// text, since it is compressed; the hosted tools open no file.
#[test]
fn a_downloaded_piece_is_opened_by_its_path_never_read() {
    let section = section(&policy_text(POLICY), "Open a downloaded piece");
    for phrase in [
        ".fyug",
        "compressed",
        "load_invention by its path",
        "never read it as text",
        "The hosted tools open no file",
    ] {
        assert!(section.contains(phrase), "should say `{phrase}`: {section}");
    }
}
