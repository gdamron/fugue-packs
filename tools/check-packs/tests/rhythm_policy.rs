//! The rhythm policy pack, `fugue.policy.rhythm`, checked against the
//! repository's real `packs/` and the pinned core's module registry:
//!
//! - it is a first-party policy pack laid out like the other core policies,
//!   short enough to sit whole in an in-graph agent's prompt;
//! - every module type, config key, control, port and control value it names
//!   exists in the registry, and every fragment it shows loads;
//! - it names the policies that own the rules it leans on instead of
//!   restating them.

use fugue::ModuleRegistry;

mod policy_support;
use policy_support::*;

const POLICY: &str = "fugue.policy.rhythm";

/// The policy as a first-party pack, like the other core policies.
#[test]
fn the_rhythm_policy_is_a_first_party_policy_pack() {
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
/// exists in the pinned core's registry, and it names at least the clock
/// and the sequencers and the module that swings.
#[test]
fn every_name_the_policy_uses_exists_in_the_registry() {
    assert_names_exist(
        POLICY,
        &["clock", "step_sequencer", "cell_sequencer", "drum_map"],
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
        "clock.beats_per_measure",
        "drum_map.swing=1.5",
        "chance.rate=x7",
        "tempo_clock",
        "clock.bpm=fast",
    ] {
        assert!(problem_with(&registry, span).is_some(), "{span} passed");
    }
    for span in [
        "clock",
        "clock.beat_x4",
        "drum_map.swing=0.3",
        "chance.rate=x3",
    ] {
        assert_eq!(problem_with(&registry, span), None, "{span}");
    }
}

/// A rule has one home: scheduling and one-at-a-time changes are named by
/// their policies, not restated.
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
        ],
    );
}

/// American English.
#[test]
fn the_policy_uses_american_spelling() {
    assert_american_spelling(POLICY, &["rhythmical"]);
}
