//! No pack names a local tool that left the listing because `apply_edits`,
//! `inspect_invention` and `show_view` cover it, checked across every file
//! under `packs/`. The single-step edits and the swap are also `apply_edits`
//! ops, so a paragraph that names `apply_edits` may name them as ops.

mod policy_support;

use std::fs;
use std::path::Path;

use policy_support::repository;

/// The local tools that left the listing.
const REMOVED_TOOLS: &[&str] = &[
    "add_module",
    "remove_module",
    "connect",
    "disconnect",
    "swap_module",
    "list_modules",
    "list_connections",
    "show_scope",
    "refresh_status",
];

/// The removed tools that are also `apply_edits` ops.
const EDIT_OPS: &[&str] = &[
    "add_module",
    "remove_module",
    "connect",
    "disconnect",
    "swap_module",
];

/// Words that are English as well as tool names: these count only as code,
/// in backticks, quoted or called, as in `connect`, "connect" or connect(...).
const ENGLISH: &[&str] = &["connect", "disconnect"];

/// Every text file under `packs/`, as (path relative to `packs/`, text).
fn all_pack_text() -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if let Ok(text) = fs::read_to_string(&path) {
                let name = path.strip_prefix(root).unwrap().display().to_string();
                out.push((name, text));
            }
        }
    }
    let root = repository().join("packs");
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out
}

/// Whether `paragraph` names `tool`: as a whole word, or, for a word that is
/// English too, only as code. A host's prefix ending in `__`, as in
/// `mcp__fugue__list_modules`, still names the tool.
fn names(paragraph: &str, tool: &str) -> bool {
    let bytes = paragraph.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    paragraph.match_indices(tool).any(|(at, _)| {
        let end = at + tool.len();
        let prefixed = paragraph[..at].ends_with("__");
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(end).copied();
        if (!prefixed && before.is_some_and(is_word)) || after.is_some_and(is_word) {
            return false;
        }
        let quoted = matches!(before, Some(b'`' | b'"' | b'\''));
        !ENGLISH.contains(&tool) || prefixed || quoted || after == Some(b'(')
    })
}

/// The removed tools `text` names, paragraph by paragraph. A paragraph that
/// names `apply_edits` may name its ops.
fn removed_tools_named(text: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    let text = text.replace("\r\n", "\n");
    for paragraph in text.split("\n\n") {
        let ops_allowed = names(paragraph, "apply_edits");
        for tool in REMOVED_TOOLS {
            if ops_allowed && EDIT_OPS.contains(tool) {
                continue;
            }
            if names(paragraph, tool) && !found.contains(tool) {
                found.push(*tool);
            }
        }
    }
    found
}

#[test]
fn no_pack_names_a_removed_tool() {
    let mut failures = Vec::new();
    for (path, text) in all_pack_text() {
        let found = removed_tools_named(&text);
        if !found.is_empty() {
            failures.push(format!("{path} names {found:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The check finds a removed tool named as a tool, and only then.
#[test]
fn the_check_finds_a_removed_tool_and_only_a_removed_tool() {
    assert_eq!(
        removed_tools_named("Keep add_module and remove_module for small changes."),
        ["add_module", "remove_module"]
    );
    assert_eq!(
        removed_tools_named("swap_module replaces one module's type in place."),
        ["swap_module"]
    );
    assert_eq!(
        removed_tools_named("Read it with list_connections.\n\nOr `show_scope`."),
        ["list_connections", "show_scope"]
    );
    assert_eq!(removed_tools_named("Call `connect` first."), ["connect"]);
    assert_eq!(
        removed_tools_named("Then disconnect(a, b)."),
        ["disconnect"]
    );
    assert_eq!(removed_tools_named("Call \"connect\" first."), ["connect"]);
    assert_eq!(
        removed_tools_named("Call mcp__fugue__list_modules or fugue.show_scope."),
        ["list_modules", "show_scope"]
    );
    assert_eq!(
        removed_tools_named("apply_edits edits.\r\n\r\nThen add_module."),
        ["add_module"]
    );
    // Ops in a paragraph that names apply_edits, English words and longer
    // names are not the tools.
    assert!(removed_tools_named("apply_edits takes add_module and `connect` edits.").is_empty());
    assert!(removed_tools_named("Its swap_module edit, in apply_edits.").is_empty());
    assert!(removed_tools_named("Say what to connect or install.").is_empty());
    assert!(removed_tools_named("Use list_modules_by_type or my_show_scope.").is_empty());
    // An op outside an apply_edits paragraph still counts.
    assert_eq!(
        removed_tools_named("apply_edits edits.\n\nThen add_module."),
        ["add_module"]
    );
}
