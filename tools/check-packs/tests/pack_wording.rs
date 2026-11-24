//! Wording no pack may ship, and the one-home rules, checked across every
//! Markdown and JSON file under `packs/`: no format conversion, no default for
//! `get_notes`' bars, no claim that scripts never run on the hosted server, the
//! `.fyug` rule stated only in the files policy, and the development author
//! naming the desktop app only for reuse.

mod policy_support;

use std::fs;
use std::path::Path;

use policy_support::{flat, policy_text, repository};

/// Every Markdown and JSON file under `packs/`, as (path relative to
/// `packs/`, text). A preview script is code, and a word in its code is no
/// claim to a person.
fn all_pack_text() -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("md" | "json")
            ) {
                let name = path.strip_prefix(root).unwrap().display().to_string();
                out.push((name, fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let root = repository().join("packs");
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out
}

fn lower(text: &str) -> String {
    flat(text).to_lowercase()
}

/// The sentences of `text`, its lines joined, split after ". ", "! " or "? ".
fn sentences(text: &str) -> Vec<String> {
    let text = flat(text);
    let mut out = Vec::new();
    let mut start = 0;
    for (i, _) in text.match_indices(['.', '!', '?']) {
        if text[i + 1..].starts_with(' ') {
            out.push(text[start..=i].to_string());
            start = i + 2;
        }
    }
    out.push(text[start.min(text.len())..].to_string());
    out
}

#[test]
fn no_pack_text_talks_of_converting_formats_or_ffmpeg() {
    for (path, text) in all_pack_text() {
        let text = text.to_lowercase();
        for word in ["ffmpeg", "convert"] {
            assert!(!text.contains(word), "{path} says `{word}`");
        }
    }
}

#[test]
fn the_retuned_together_example_is_gone_from_every_pack() {
    for (path, text) in all_pack_text() {
        assert!(
            !lower(&text).contains("several starting values retuned together"),
            "{path} still has the phrase"
        );
    }
}

/// The text's paragraphs and list items, each on one line.
fn blocks(text: &str) -> Vec<String> {
    text.split("\n\n")
        .flat_map(|paragraph| paragraph.split("\n- "))
        .map(flat)
        .collect()
}

#[test]
fn no_pack_claims_get_notes_has_a_default_number_of_bars() {
    for (path, text) in all_pack_text() {
        for block in blocks(&text).iter().filter(|b| b.contains("get_notes")) {
            for sentence in sentences(block) {
                let sentence = sentence.to_lowercase();
                assert!(
                    !(sentence.contains("default")
                        && (sentence.contains("bar") || sentence.contains("get_notes"))),
                    "{path} claims a default for get_notes: {sentence}"
                );
            }
        }
    }
}

#[test]
fn no_pack_says_scripts_never_run_on_the_hosted_server() {
    for (path, text) in all_pack_text() {
        let text = lower(&text);
        for claim in [
            "scripts never run",
            "scripts do not run",
            "scripts don't run",
            "script modules never run",
            "script modules do not run",
            "script modules don't run",
        ] {
            assert!(!text.contains(claim), "{path} says `{claim}`");
        }
    }
}

#[test]
fn no_policy_but_the_files_policy_states_the_fyug_rule() {
    for (path, text) in all_pack_text() {
        if !path.ends_with("POLICY.md") || path.starts_with("fugue.policy.files/") {
            continue;
        }
        let text = lower(&text);
        assert!(
            !(text.contains(".fyug") && text.contains("never read")),
            "{path} restates the .fyug rule"
        );
    }
    let files = lower(&policy_text("fugue.policy.files"));
    assert!(files.contains(".fyug") && files.contains("never read"));
}

#[test]
fn the_development_author_names_the_desktop_app_only_for_reuse() {
    let text =
        fs::read_to_string(repository().join("packs/fugue.development.author/SKILL.md")).unwrap();
    let naming: Vec<String> = sentences(&text)
        .into_iter()
        .filter(|s| s.to_lowercase().contains("desktop app"))
        .collect();
    assert!(!naming.is_empty(), "the desktop app is never named");
    for sentence in naming {
        assert!(
            sentence.to_lowercase().contains("reuse"),
            "the desktop app is named outside a reuse offer: {sentence}"
        );
    }
}
