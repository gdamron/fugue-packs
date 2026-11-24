//! A skill's links to its policies. An agent opens a linked policy from disk
//! only when the skill tells it the moment the policy applies, so every link
//! sits in a sentence that opens with when to read it ("Before opening the
//! in-chat player, read ..."), not only with what the policy is about. A
//! skill links exactly the policies its manifest requires.

use std::collections::BTreeSet;

/// The words a pointer's sentence may open with, each naming a moment.
pub const TRIGGER_WORDS: [&str; 7] = ["Before", "When", "While", "Whenever", "After", "Once", "If"];

/// The marker every policy link starts with: `[name](policies/<id>.md)`.
const LINK: &str = "](policies/";

/// What is wrong with the policy links of a skill's text, given the ids its
/// manifest requires (version ranges already removed). Empty when nothing is.
pub fn problems(text: &str, required: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut linked = BTreeSet::new();
    for block in blocks(text) {
        let mut rest = block.as_str();
        let mut offset = 0;
        while let Some(found) = rest.find(LINK) {
            let at = offset + found;
            let after = &block[at + LINK.len()..];
            let Some(id) = link_id(after) else {
                let what = if after.contains(')') {
                    "does not end in .md"
                } else {
                    "is not closed"
                };
                problems.push(format!("a policy link {what}: `{}`", excerpt(&block)));
                break;
            };
            if !required.contains(&id) {
                problems.push(format!(
                    "links {id}, which its manifest's `policies` does not require"
                ));
            }
            if !opens_with_trigger(sentence_before(&block, at)) {
                problems.push(format!(
                    "links {id} without saying when to open it: start the sentence \
                     with one of {} (\"Before opening a .fyug file, read ...\"), not \
                     `{}`",
                    TRIGGER_WORDS.join(", "),
                    excerpt(sentence_before(&block, at))
                ));
            }
            linked.insert(id);
            offset = at + LINK.len();
            rest = &block[offset..];
        }
    }
    for id in required {
        if !linked.contains(id) {
            problems.push(format!("requires {id} but never links it"));
        }
    }
    problems
}

/// The policy id of a link target, given the text after `](policies/`: up
/// to the link's closing `)`, without a title, an `#anchor` or `.md`.
fn link_id(after: &str) -> Option<String> {
    let (target, _) = after.split_once(')')?;
    let target = target.split([' ', '#']).next().unwrap_or_default();
    target.strip_suffix(".md").map(str::to_string)
}

/// The text's headings, paragraphs and list items outside its frontmatter
/// and fenced code, each with its line breaks and indents collapsed to single
/// spaces and its list or heading marker removed.
fn blocks(text: &str) -> Vec<String> {
    let body = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(text, |(_, body)| body);
    let mut blocks: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    let mut fenced = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            blocks.extend(current.take());
            continue;
        }
        if fenced || trimmed.is_empty() {
            blocks.extend(current.take());
            continue;
        }
        if trimmed.starts_with('#') {
            blocks.extend(current.take());
            blocks.push(trimmed.trim_start_matches('#').trim().to_string());
            continue;
        }
        match list_item(trimmed) {
            Some(item) => {
                blocks.extend(current.take());
                current = Some(item.to_string());
            }
            None => match current.as_mut() {
                Some(block) => {
                    block.push(' ');
                    block.push_str(trimmed);
                }
                None => current = Some(trimmed.to_string()),
            },
        }
    }
    blocks.extend(current);
    blocks
}

/// The item's text when `line` starts a list item (`- `, `* ` or `12. `).
fn list_item(line: &str) -> Option<&str> {
    if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return Some(item);
    }
    let (number, item) = line.split_once(". ")?;
    (!number.is_empty() && number.chars().all(|c| c.is_ascii_digit())).then_some(item)
}

/// The sentence of `block` that the byte offset `at` falls in, up to `at`.
fn sentence_before(block: &str, at: usize) -> &str {
    let before = &block[..at];
    let start = [". ", "! ", "? "]
        .iter()
        .filter_map(|end| before.rfind(end).map(|i| i + end.len()))
        .max()
        .unwrap_or(0);
    &before[start..]
}

/// Whether the sentence's first word, past any bold or italic markers,
/// names a moment.
fn opens_with_trigger(sentence: &str) -> bool {
    let word = sentence
        .trim_start_matches(['*', '_', ' '])
        .split(|c: char| !c.is_alphabetic())
        .next()
        .unwrap_or_default();
    TRIGGER_WORDS.contains(&word)
}

fn excerpt(text: &str) -> String {
    let short: String = text.chars().take(80).collect();
    if short.len() < text.len() {
        format!("{short}...")
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn required(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    const FRONT: &str = "---\nname: t\ndescription: A test.\n---\n# T\n\n";

    #[test]
    fn accepts_a_link_whose_sentence_says_when() {
        let text = format!(
            "{FRONT}## Required policies\n\n- Before opening a file, read\n  \
             [files](policies/fugue.test.files.md).\n- When it plays, read \
             [a](policies/fugue.test.a.md) and [b](policies/fugue.test.b.md).\n"
        );
        let found = problems(
            &text,
            &required(&["fugue.test.files", "fugue.test.a", "fugue.test.b"]),
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn rejects_a_link_that_says_only_what_the_policy_is_about() {
        let text = format!("{FRONT}- [Files](policies/fugue.test.files.md): keeping a piece.\n");
        let found = problems(&text, &required(&["fugue.test.files"]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("without saying when"), "{found:?}");
    }

    #[test]
    fn the_trigger_must_open_the_sentence_holding_the_link() {
        let text = format!(
            "{FRONT}Before you start, look around. Then follow \
             [files](policies/fugue.test.files.md) throughout.\n"
        );
        let found = problems(&text, &required(&["fugue.test.files"]));
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn rejects_a_link_to_a_policy_the_manifest_does_not_require() {
        let text = format!("{FRONT}- Before playing, read [x](policies/fugue.test.x.md).\n");
        let found = problems(&text, &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("does not require"), "{found:?}");
    }

    #[test]
    fn rejects_a_required_policy_the_skill_never_links() {
        let found = problems(FRONT, &required(&["fugue.test.x"]));
        assert_eq!(found, ["requires fugue.test.x but never links it"]);
    }

    #[test]
    fn reads_the_id_past_an_anchor_or_a_title() {
        let text = format!(
            "{FRONT}- Before playing, read [a](policies/fugue.test.a.md#keep).\n\
             - When it ends, read [b](policies/fugue.test.b.md \"Rules\").\n"
        );
        let found = problems(&text, &required(&["fugue.test.a", "fugue.test.b"]));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn checks_a_link_in_a_heading() {
        let text = format!("{FRONT}## [Rules](policies/fugue.test.x.md)\n");
        let found = problems(&text, &required(&["fugue.test.x"]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("without saying when"), "{found:?}");
    }

    #[test]
    fn names_a_link_that_does_not_end_in_md() {
        let text = format!("{FRONT}- Before playing, read [x](policies/fugue.test.x).\n");
        let found = problems(&text, &required(&["fugue.test.x"]));
        assert!(found[0].contains("does not end in .md"), "{found:?}");
    }

    #[test]
    fn ignores_links_in_fenced_code() {
        let text = format!("{FRONT}```\n[x](policies/fugue.test.x.md)\n```\n");
        assert!(problems(&text, &[]).is_empty());
    }
}
