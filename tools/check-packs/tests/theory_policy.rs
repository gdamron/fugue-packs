//! The theory policy pack, `fugue.policy.theory`, checked against the
//! repository's real `packs/` and the pinned core's module registry:
//!
//! - it is a first-party policy pack laid out like the other core policies,
//!   short enough to sit whole in an in-graph agent's prompt;
//! - every module type, config key, control, port and control value it names
//!   exists in the registry, and every fragment it shows loads;
//! - what it says does not exist (a quantizer module, a shared key) is still
//!   true of the pinned core, so a module that fills the gap fails here;
//! - it names the policies that own the rules it leans on instead of
//!   restating them.

use fugue::ModuleRegistry;

mod policy_support;
use policy_support::*;

const POLICY: &str = "fugue.policy.theory";

/// The policy as a first-party pack, like the other core policies.
#[test]
fn the_theory_policy_is_a_first_party_policy_pack() {
    assert_first_party_policy(POLICY);
}

/// The README's Policies section lists the pack.
#[test]
fn the_readme_lists_the_policy() {
    assert_readme_lists(POLICY);
}

/// A policy is included whole in a prompt, so it stays short.
#[test]
fn the_policy_fits_a_prompt() {
    assert_fits_prompt(POLICY);
}

/// Every module type, config key, control, port and value the prose names
/// exists in the pinned core's registry, and it names the sequencers and the
/// two modules that stay in key.
#[test]
fn every_name_the_policy_uses_exists_in_the_registry() {
    assert_names_exist(
        POLICY,
        &["step_sequencer", "cell_sequencer", "chance", "melody"],
        &["set_controls", "when"],
    );
}

/// Every fragment the policy shows loads in the pinned core, and what it
/// loads sets nothing aside (an unknown config key is refused, or warned).
#[test]
fn every_fragment_the_policy_shows_loads() {
    assert_fragments_load(POLICY, 2);
}

/// The checker catches a name the registry lacks.
#[test]
fn a_name_the_registry_lacks_is_caught() {
    let registry = ModuleRegistry::default();
    for span in [
        "chance.mode_name",
        "melody.scale",
        "chance.scale=dorian_flat",
        "chance.steps=2",
        "quantizer",
        "step_sequencer.root_note=200",
    ] {
        assert!(problem_with(&registry, span).is_some(), "{span} passed");
    }
    for span in [
        "chance.scale=dorian",
        "chance.steps=0.5",
        "melody.degrees",
        "step_sequencer.root_note=62",
    ] {
        assert_eq!(problem_with(&registry, span), None, "{span}");
    }
}

/// The scales the prose lists are ones `chance.scale` takes, as written
/// (outside code spans, so the span check does not see them): the list that
/// ends "and chromatic" is read from the prose, and each name is checked.
#[test]
fn the_scales_the_policy_lists_exist() {
    let registry = ModuleRegistry::default();
    let (prose, _) = split_fences(&policy_text(POLICY));
    let prose = prose.replace('\n', " ");
    let list = prose
        .split("`chance.scale` takes ")
        .nth(1)
        .and_then(|rest| rest.split_once("chromatic"))
        .expect("the policy lists the scales `chance.scale` takes, ending in chromatic")
        .0;
    let mut names: Vec<String> = list
        .replace(" and", ",")
        .split(',')
        .filter_map(|part| part.split_whitespace().next().map(str::to_string))
        .collect();
    names.push("chromatic".into());
    assert!(names.len() >= 12, "read {names:?}");
    for name in &names {
        let span = format!("chance.scale={name}");
        assert_eq!(problem_with(&registry, &span), None, "{span}");
    }
    let options = control_options(&registry, "chance", "scale").expect("chance.scale is a choice");
    for option in options {
        assert!(
            names.contains(&option),
            "the policy omits the scale {option}"
        );
    }
}

/// What the policy says does not exist still does not, in the pinned core:
/// no quantizer module, and no module but `chance` and the chord model of
/// `macro_oscillator` holds a key or scale. If one appears, the policy's last
/// section and its modulation advice are out of date.
#[test]
fn the_gaps_the_policy_names_still_hold() {
    let registry = ModuleRegistry::default();
    for module_type in registry.types() {
        assert!(
            !module_type.contains("quantiz") && !module_type.contains("tuning"),
            "{module_type} may fill a gap the policy names"
        );
    }
    for key in ["key", "scale", "tuning", "transpose"] {
        let holders = types_with_control(&registry, key);
        let expected: &[&str] = match key {
            "key" | "scale" => &["chance", "macro_oscillator"],
            _ => &[],
        };
        assert_eq!(
            holders.iter().map(String::as_str).collect::<Vec<_>>(),
            expected,
            "modules with a `{key}` control changed"
        );
    }
    let text = policy_text(POLICY);
    assert!(text.contains("There is no shared key"));
}

/// A rule has one home: scheduling and one-at-a-time changes are named by
/// their policies, not restated, and harmony and rhythm are left to theirs.
#[test]
fn the_policy_names_other_policies_instead_of_restating_them() {
    assert_names_owners_without_restating(
        POLICY,
        &["fugue.policy.clean-audio", "fugue.policy.musical-intent"],
        &[
            "\"kind\"",
            "headroom",
            "retrigger",
            "one at a time",
            "envelope",
            "next_bar",
            "progression",
            "voice leading",
            "swing",
            "triplet",
        ],
    );
}

/// American English.
#[test]
fn the_policy_uses_american_spelling() {
    assert_american_spelling(POLICY, &["tonalise", "harmonise", "favour"]);
}
