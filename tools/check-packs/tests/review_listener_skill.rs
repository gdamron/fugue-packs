//! The listening critique skill, `fugue.review.listener`, checked against
//! the repository's real `packs/`: it hears the piece through the local
//! server's audio preview and meters, judges it against musical intent and
//! proposes one change, leaving every rule to the four policies it requires.

mod policy_support;
mod skill_support;

use std::process::Command;

use policy_support::{code_spans, flat, repository};
use skill_support::{
    assert_body_fits, assert_first_party_skill, assert_names_exist, assert_requires_policies,
    assert_restates_no_policy, assert_thin_shape, section, skill, HOSTED_TOOLS, LISTENING_TOOLS,
    LOCAL_TOOLS, MAX_BODY_CHARACTERS, PLAYER_TOOLS,
};

const SKILL: &str = "fugue.review.listener";
const POLICIES: [&str; 5] = [
    "fugue.policy.listening",
    "fugue.policy.musical-intent",
    "fugue.policy.clean-audio",
    "fugue.policy.surfaces",
    "fugue.policy.player",
];

/// A local server tool, as the product lists it.
fn is_local_tool(name: &str) -> bool {
    LOCAL_TOOLS.contains(&name) || LISTENING_TOOLS.contains(&name)
}

/// In the in-chat player there is no audio preview: the critique comes from
/// the player's levels and notes, and says it was measured, not heard.
#[test]
fn in_the_in_chat_player_the_critique_is_measured_not_heard() {
    let skill = skill(SKILL);
    let procedure = flat(section(&skill.body, "Procedure"));
    for phrase in [
        "In the in-chat player, read `get_meters` and `preview_notes` from the player instead",
        "skip to step 4",
        "measured, not heard",
        "On any other route, stop",
    ] {
        assert!(
            procedure.contains(phrase),
            "should say `{phrase}`: {procedure}"
        );
    }
    for tool in ["get_meters", "preview_notes"] {
        assert!(PLAYER_TOOLS.contains(&tool), "{tool} is a player tool");
    }
    assert!(!skill.description.contains("local Fyug tools are missing"));
}

/// The pack is a first-party skill for external agents with a host-safe
/// name, and the repository passes check-packs with it, warning about
/// nothing.
#[test]
fn the_listener_is_a_first_party_skill_for_external_agents() {
    assert_first_party_skill(SKILL, &["external-agent"]);
    let skill = skill(SKILL);
    assert_eq!(skill.name, "fugue-review-listener");
    let description = skill.description.to_lowercase();
    for word in ["listen", "review", "critique", "mix", "use when", "not for"] {
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

/// The policies are required and linked, each by a pointer that says when
/// to read it, and none is restated.
#[test]
fn the_listener_requires_its_policies_without_restating_them() {
    let skill = skill(SKILL);
    assert_requires_policies(&skill, &POLICIES);
    skill_support::assert_pointers_say_when(&skill);
    assert_restates_no_policy(&skill);
}

/// Thin shape, within the shared size limit, naming only real things.
#[test]
fn the_listener_is_thin_short_and_names_only_what_exists() {
    let skill = skill(SKILL);
    assert_thin_shape(&skill, 5);
    assert_body_fits(&skill, MAX_BODY_CHARACTERS);
    assert_names_exist(&skill, &[]);
}

/// Surface, meters, preview, judgment against intent, one change: in that
/// order, each carried out by the local server's tools.
#[test]
fn the_procedure_measures_previews_judges_and_proposes_one_change() {
    let skill = skill(SKILL);
    let procedure = flat(section(&skill.body, "Procedure"));
    let at = |needle: &str| {
        procedure
            .find(needle)
            .unwrap_or_else(|| panic!("the procedure should say `{needle}`"))
    };
    let steps = [
        at("1. Recognize the surface as the surfaces policy says"),
        at("2. Read `get_meters`"),
        at("3. Get the bounded audio preview"),
        at("4. Say what you heard against the musical-intent policy"),
        at("5. Propose one change"),
    ];
    assert!(
        steps.windows(2).all(|pair| pair[0] < pair[1]),
        "{procedure}"
    );
    assert!(procedure.contains("`show_view` with view \"scope\""));
    // The preview is `show_view` with view "audio", which renders only a
    // paused piece: pause, render, resume.
    let preview = [
        at("`pause_invention`"),
        at("`show_view` with view \"audio\""),
        at("`resume_invention`"),
    ];
    assert!(
        preview.windows(2).all(|pair| pair[0] < pair[1]),
        "{procedure}"
    );
    assert!(procedure.contains("10 seconds"), "the preview's default");
    assert!(procedure.contains("at most 30"), "the preview's own cap");
    // The user's music is theirs: ask before pausing it, and resume only
    // what was paused here.
    assert!(at("ask before pausing it") < preview[0]);
    assert!(procedure.contains("`resume_invention` only if you paused it"));
    let spans = code_spans(&procedure);
    assert!(!spans.is_empty());
    for span in &spans {
        assert!(is_local_tool(span), "`{span}` is no local server tool");
    }
}

/// The stops, each with what the user can do next; whether a preview
/// exists is the surfaces policy's call.
#[test]
fn the_stop_conditions_cover_a_missing_preview_and_an_unheard_one() {
    let skill = skill(SKILL);
    let stops = flat(section(&skill.body, "Stop conditions"));
    assert!(stops.contains("what the user can do next"));
    for (stop, next) in [
        (
            "No audio preview and no player tools on this surface.",
            "The surfaces policy decides",
        ),
        ("Any other preview refusal.", "Say the reason"),
        ("Nothing is loaded.", "open it in the player"),
        ("Nothing is loaded.", "next message or a later one"),
        ("Nothing is loaded.", "never that the tools are missing"),
        (
            "The piece cannot be rendered offline.",
            "Critique from the meters and scope alone",
        ),
        (
            "The user does not want the music paused.",
            "Critique from the meters and scope alone",
        ),
        (
            "Your host cannot play the WAV to you.",
            "Do not claim to have heard it",
        ),
    ] {
        let bullet = stops
            .split("- **")
            .find(|bullet| bullet.starts_with(stop))
            .unwrap_or_else(|| panic!("no stop for `{stop}`"));
        assert!(bullet.contains(next), "`{stop}` should say `{next}`");
    }
    for span in code_spans(&stops) {
        assert!(
            is_local_tool(&span) || HOSTED_TOOLS.contains(&span.as_str()),
            "`{span}` is no product tool"
        );
    }
}
