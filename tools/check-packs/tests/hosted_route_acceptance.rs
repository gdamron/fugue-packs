//! The first-party skills and the surfaces policy for the hosted server as it
//! now is, checked as text against the repository's real `packs/`.
//!
//! The hosted server's `load_invention` opens an in-chat player with its own
//! tools; `play_preview` is gone. So no skill or policy in the plugin's set
//! names `play_preview`; the surfaces policy tells the local surface from the
//! hosted one by the tools only one of them has; it lists what the hosted
//! server leaves to the desktop app and when to suggest it; it says how to keep
//! a piece, what a phone changes, and what to do when a call is not approved.
//! The packs that changed carry the new version.
//!
//! Checks look for key nouns and tool names, never whole sentences, so the
//! wording can change without touching them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::{Version, VersionReq};
use serde_json::Value;

/// The plugin's skills; their policies come from their manifests.
const PLUGIN_SKILLS: [&str; 4] = [
    "fugue.skill.entry",
    "fugue.invention.author",
    "fugue.development.author",
    "fugue.review.listener",
];
const SURFACES: &str = "fugue.policy.surfaces";
/// The packs whose text was rewritten, so whose version is new.
const CHANGED: [&str; 4] = [
    SURFACES,
    "fugue.skill.entry",
    "fugue.invention.author",
    "fugue.review.listener",
];
const NEW_VERSION: &str = "2026.11.1";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pack(id: &str) -> PathBuf {
    repository().join("packs").join(id)
}

fn manifest(id: &str) -> Value {
    let text = fs::read_to_string(pack(id).join("fugue.pkg.json"))
        .unwrap_or_else(|e| panic!("{id} has no manifest: {e}"));
    serde_json::from_str(&text).unwrap()
}

/// The ids a skill requires, without their version ranges.
fn required_policies(skill: &str) -> Vec<String> {
    manifest(skill)["policies"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|policy| {
            policy
                .as_str()
                .unwrap()
                .split('@')
                .next()
                .unwrap()
                .to_string()
        })
        .collect()
}

/// The skills and every policy they require, each once.
fn plugin_set() -> Vec<String> {
    let mut set: Vec<String> = PLUGIN_SKILLS.iter().map(|id| id.to_string()).collect();
    for skill in PLUGIN_SKILLS {
        for policy in required_policies(skill) {
            if !set.contains(&policy) {
                set.push(policy);
            }
        }
    }
    set
}

/// Every text file of a pack, as `(relative path, contents)`; the license and
/// anything that is not UTF-8 text are left out.
fn text_files(id: &str) -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let root = pack(id);
    let mut files = Vec::new();
    walk(&root, &mut files);
    files
        .into_iter()
        .filter(|path| path.file_name().unwrap() != "LICENSE")
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            Some((
                path.strip_prefix(&root).unwrap().display().to_string(),
                text,
            ))
        })
        .collect()
}

/// The surfaces policy, lowercased, with its whitespace collapsed.
fn surfaces() -> String {
    let text = fs::read_to_string(pack(SURFACES).join("POLICY.md")).unwrap();
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn assert_mentions(text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            text.contains(needle),
            "the surfaces policy should say `{needle}`:\n{text}"
        );
    }
}

/// The text within `radius` characters of the first `needle`.
fn around<'a>(text: &'a str, needle: &str, radius: usize) -> &'a str {
    let at = text
        .find(needle)
        .unwrap_or_else(|| panic!("the surfaces policy should say `{needle}`:\n{text}"));
    let mut start = at.saturating_sub(radius);
    let mut end = (at + radius).min(text.len());
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    while !text.is_char_boundary(end) {
        end += 1;
    }
    &text[start..end]
}

/// No skill or policy of the plugin names `play_preview`, the hosted tool's
/// old name, in any file the pack ships.
#[test]
fn no_plugin_skill_or_policy_names_play_preview() {
    let mut offenders = Vec::new();
    for id in plugin_set() {
        let files = text_files(&id);
        assert!(!files.is_empty(), "{id} has no text files");
        for (name, text) in files {
            if text.contains("play_preview") {
                offenders.push(format!("{id}/{name}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "still name play_preview: {offenders:?}"
    );
}

/// The plugin's set includes the surfaces policy, so the check above reaches
/// it, and each required policy exists as a pack.
#[test]
fn the_plugin_set_holds_the_surfaces_policy_and_every_pack_exists() {
    let set = plugin_set();
    assert!(set.iter().any(|id| id == SURFACES), "{set:?}");
    for id in &set {
        assert!(
            pack(id).join("fugue.pkg.json").is_file(),
            "{id} is not a pack"
        );
    }
}

/// The packs that changed carry the new version, and every range that points
/// at a pack is still met by the version it has.
#[test]
fn changed_packs_carry_the_new_version_and_every_range_is_met() {
    for id in CHANGED {
        assert_eq!(manifest(id)["version"], NEW_VERSION, "{id}");
    }
    for skill in PLUGIN_SKILLS {
        for entry in manifest(skill)["policies"].as_array().into_iter().flatten() {
            let (policy, range) = entry.as_str().unwrap().split_once('@').unwrap();
            let have = Version::parse(manifest(policy)["version"].as_str().unwrap()).unwrap();
            let want = VersionReq::parse(range).unwrap();
            assert!(
                want.matches(&have),
                "{skill} wants {policy}@{range}, but it is {have}"
            );
        }
    }
}

/// The repository passes the checker, with the changed packs and their
/// versions in place.
#[test]
fn the_check_packs_binary_passes_on_the_repository() {
    let output = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(repository())
        .output()
        .expect("check-packs runs");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "check-packs failed:\n{printed}");
}

/// The local surface is recognized by a tool only the local server has
/// (`reload_invention` or `show_view`), the hosted surface by
/// `check_invention`, and the local one first when both sets are present.
#[test]
fn the_surface_is_recognized_by_tools_only_one_server_has() {
    let text = surfaces();
    assert_mentions(&text, &["reload_invention", "show_view", "check_invention"]);

    let local = text
        .find("reload_invention")
        .min(text.find("show_view"))
        .unwrap();
    let hosted = text.find("check_invention").unwrap();
    assert!(local < hosted, "local is recognized before hosted:\n{text}");

    // Both sets present: the local surface wins.
    let near = around(&text, "check_invention", 300);
    assert!(
        near.contains("both") || near.contains("local first") || near.contains("first"),
        "should say local wins when both sets are present:\n{near}"
    );
    assert!(
        text.contains("both"),
        "should say what to do when both sets are present:\n{text}"
    );
}

/// The policy lists what the hosted server lacks and leaves to the desktop
/// app, and says to suggest the desktop app only when the person asks for one
/// of those.
#[test]
fn it_lists_what_the_hosted_server_leaves_to_the_desktop_app() {
    let text = surfaces();
    assert_mentions(
        &text,
        &[
            "desktop app",
            "phone",
            "library",
            "samples",
            "disk",
            "limit",
        ],
    );
    assert!(
        text.contains("off screen") || text.contains("off-screen") || text.contains("offscreen"),
        "should name music that stops with the player off screen on a phone:\n{text}"
    );
    let ask = around(&text, "desktop app", 400);
    assert!(
        ask.contains("only when") || ask.contains("only if"),
        "should suggest the desktop app only on a request:\n{ask}"
    );
    assert!(text.contains("ask"), "{text}");
}

/// How to keep a piece: the download where the host has one, the share link
/// otherwise; anyone with the link has the piece; the agent never posts a
/// share link itself.
#[test]
fn it_says_how_to_keep_a_piece_and_never_to_post_a_share_link() {
    let text = surfaces();
    assert_mentions(&text, &["download", "share link", "anyone with"]);
    assert!(
        [
            "never post",
            "never share",
            "never send",
            "never paste",
            "not post",
            "do not post"
        ]
        .iter()
        .any(|phrase| text.contains(phrase)),
        "should say the agent never posts a share link:\n{text}"
    );
    let rule = around(&text, "anyone with", 200);
    assert!(rule.contains("link"), "{rule}");
}

/// On a phone the player's tools appear only after the person's next message,
/// so no player tool call is planned in the turn that opens the player.
#[test]
fn it_says_player_tools_arrive_after_the_next_message_on_a_phone() {
    let text = surfaces();
    assert_mentions(&text, &["next message", "phone", "turn"]);
    let near = around(&text, "next message", 400);
    assert!(near.contains("tool"), "{near}");
    assert!(near.contains("player"), "{near}");
}

/// When a call is not run for lack of approval: say what it was about to do,
/// tell the person once how to allow the connector's tools, point to the same
/// control in the player, and do not retry unasked.
#[test]
fn it_says_what_to_do_when_a_call_is_not_approved() {
    let text = surfaces();
    assert_mentions(&text, &["approv", "connector"]);
    let near = around(&text, "approv", 600);
    for needle in ["connector", "once", "player"] {
        assert!(
            near.contains(needle),
            "the approval rule should say `{needle}`:\n{near}"
        );
    }
    assert!(
        ["retry", "try again", "re-run", "rerun", "call again"]
            .iter()
            .any(|phrase| near.contains(phrase)),
        "the approval rule should say not to retry unasked:\n{near}"
    );
}

/// The entry skill still sends a hosted piece through `load_invention`, the
/// one tool that opens the player.
#[test]
fn the_entry_skill_opens_the_player_with_load_invention() {
    let text = fs::read_to_string(pack("fugue.skill.entry").join("SKILL.md")).unwrap();
    assert!(text.contains("load_invention"), "{text}");
    assert!(text.contains("check_invention"), "{text}");
}
