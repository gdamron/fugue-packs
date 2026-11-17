//! The entry skill, `fugue.skill.entry`, checked against the repository's
//! real `packs/`: the `/fugue` skill that recognizes the surface, names the
//! route, has the piece authored and plays it through that route, leaving
//! every rule to the four policies it requires.

mod policy_support;
mod skill_support;

use std::process::Command;

use policy_support::{code_spans, flat, pack, policy_text, repository};
use skill_support::{
    assert_body_fits, assert_first_party_skill, assert_names_exist, assert_requires_policies,
    assert_restates_no_policy, assert_thin_shape, section, skill, HOSTED_TOOLS, INSTALL_URL,
    LOCAL_TOOLS,
};

const SKILL: &str = "fugue.skill.entry";
const SURFACES: &str = "fugue.policy.surfaces";
const POLICIES: [&str; 4] = [
    "fugue.policy.authoring",
    SURFACES,
    "fugue.policy.clean-audio",
    "fugue.policy.musical-intent",
];
/// The command the plain chat step runs, from the skill's folder.
const PREVIEW_COMMAND: &str = "python3 scripts/write_preview.py --player fyug-player.html \
    --modules fyug-player-modules.json --invention piece.json --out piece-preview.html";
/// The issue's own limit, below the shared one.
const MAX_BODY_CHARACTERS: usize = 4_000;

/// The pack is a first-party skill for external agents, named `fugue`, and
/// the repository passes check-packs with it, warning about nothing.
#[test]
fn the_entry_skill_is_a_first_party_skill_named_fugue() {
    assert_first_party_skill(SKILL, &["external-agent"]);
    let skill = skill(SKILL);
    assert_eq!(skill.name, "fugue");
    let description = skill.description.to_lowercase();
    for word in [
        "make", "play", "change", "fugue", "fyug", "/fugue", "not for",
    ] {
        assert!(description.contains(word), "the description omits `{word}`");
    }

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
    assert!(!printed.contains(SKILL), "check-packs warns:\n{printed}");
}

/// The four policies are required and linked, and none is restated.
#[test]
fn the_entry_skill_requires_its_policies_without_restating_them() {
    let skill = skill(SKILL);
    assert_requires_policies(&skill, &POLICIES);
    assert_restates_no_policy(&skill);
}

/// Thin shape, within the issue's size limit, naming only real things.
#[test]
fn the_entry_skill_is_thin_short_and_names_only_what_exists() {
    let skill = skill(SKILL);
    assert_thin_shape(&skill, 5);
    assert_body_fits(&skill, MAX_BODY_CHARACTERS);
    assert_names_exist(
        &skill,
        &["/fugue", "fugue.invention.author", PREVIEW_COMMAND],
    );
}

/// Recognize the surface, name the route, author, then play, in that order.
#[test]
fn the_procedure_recognizes_names_authors_and_plays() {
    let skill = skill(SKILL);
    let procedure = flat(section(&skill.body, "Procedure"));
    let at = |needle: &str| {
        procedure
            .find(needle)
            .unwrap_or_else(|| panic!("the procedure should say `{needle}`"))
    };
    let steps = [
        at("1. Recognize the surface as the surfaces policy says"),
        at("2. Tell the user which route"),
        at("3. Author or change the piece as the authoring policy says"),
        at("4. Play it through the route"),
    ];
    assert!(
        steps.windows(2).all(|pair| pair[0] < pair[1]),
        "{procedure}"
    );
    assert!(procedure.contains("`fugue.invention.author` skill is present"));
}

/// Every surface the surfaces policy routes has its step here, carried out
/// by the right server's tools.
#[test]
fn each_surface_in_the_surfaces_policy_has_its_step() {
    let policy = policy_text(SURFACES);
    let labels: Vec<&str> = policy
        .split("\n- **")
        .skip(1)
        .filter_map(|bullet| bullet.split_once(":**").map(|(label, _)| label))
        .collect();
    assert_eq!(labels.len(), 5, "the policy's routes: {labels:?}");

    let skill = skill(SKILL);
    let procedure = section(&skill.body, "Procedure");
    let step = |label: &str| -> String {
        procedure
            .split("\n   - **")
            .find(|bullet| bullet.starts_with(&format!("{label}:**")))
            .map(flat)
            .unwrap_or_else(|| panic!("no step for `{label}`"))
    };
    for label in &labels {
        step(label);
    }

    let local = step("Local");
    let spans = code_spans(&local);
    assert!(!spans.is_empty() && spans.iter().all(|s| LOCAL_TOOLS.contains(&s.as_str())));
    let machine = step("Shell on the user's machine");
    assert!(machine.contains("native install offer") && machine.contains("ask first"));
    assert!(
        machine.contains(INSTALL_URL),
        "the install link is the product's"
    );
    assert!(
        machine.contains("use the local route"),
        "then plays locally"
    );
    let cloud = step("Shell in a cloud session");
    assert!(cloud.contains("`fyug render ") && cloud.contains("--out"));
    assert!(cloud.contains("WAV file"), "the render reaches the user");
    assert!(cloud.contains("Otherwise the preview file"));
    let hosted = step("Hosted");
    let spans = code_spans(&hosted);
    assert_eq!(spans, ["check_invention", "play_preview"]);
    assert!(spans.iter().all(|s| HOSTED_TOOLS.contains(&s.as_str())));
    let plain = step("Plain chat");
    for words in [
        "preview file",
        "host can run this skill's script and show an HTML file",
        "limited preview",
        INSTALL_URL,
        "Never retype, edit or regenerate the player",
        "say so and what to connect or install",
    ] {
        assert!(plain.contains(words), "the plain chat step omits `{words}`");
    }
    assert!(plain.contains(&format!("`{PREVIEW_COMMAND}`")));
    let script = PREVIEW_COMMAND.split(' ').nth(1).unwrap();
    assert!(
        pack(SKILL).join(script).is_file(),
        "{script} ships in the pack"
    );
}

/// Each stop says what the user can do next and never changes route
/// silently.
#[test]
fn the_stop_conditions_cover_the_three_dead_ends() {
    let skill = skill(SKILL);
    let stops = flat(section(&skill.body, "Stop conditions"));
    assert!(stops.contains("what the user can do next"));
    assert!(stops.contains("Never move to another route without telling the user"));
    // Each stop, and what it offers the user next.
    for (stop, next) in [
        ("The user declines the install.", "They can connect"),
        (
            "The piece uses a module type the preview does not support.",
            "Offer to rework the piece",
        ),
        (
            "No route on this surface can play the piece.",
            "what to connect or install",
        ),
    ] {
        let bullet = stops
            .split("- **")
            .find(|bullet| bullet.starts_with(stop))
            .unwrap_or_else(|| panic!("no stop for `{stop}`"));
        assert!(bullet.contains(next), "`{stop}` should offer `{next}`");
    }
    assert!(stops.contains("`play_preview` refuses it by name, and so does the script"));
}
