//! The harmony policy pack, `fugue.policy.harmony`, checked against the
//! repository's real `packs/` and the pinned core's module registry:
//!
//! - it is a first-party policy pack laid out like the other core policies,
//!   short enough to sit whole in an in-graph agent's prompt;
//! - every module type, config key, control, port and control value it names
//!   exists in the registry, and every fragment it shows loads;
//! - the chord shapes it places along `macro_oscillator.harmonics` sound the
//!   notes it says they do;
//! - what it says does not exist (a chord or voicing module, a chord on one
//!   sequencer step, a chord from `divisi`) is still true of the pinned core,
//!   so a module that fills the gap fails here;
//! - it names the policies that own the rules it leans on instead of
//!   restating them.

use fugue::{DescribeModuleQuery, ModuleDescription, ModuleRegistry, RegistryScope, RenderEngine};
use serde_json::json;

mod policy_support;
use policy_support::*;

const POLICY: &str = "fugue.policy.harmony";

/// The policy as a first-party pack, like the other core policies.
#[test]
fn the_harmony_policy_is_a_first_party_policy_pack() {
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
/// exists in the pinned core's registry, and it names the chord model's
/// module, the sequencer, the scheduler and the voice allocator.
#[test]
fn every_name_the_policy_uses_exists_in_the_registry() {
    assert_names_exist(
        POLICY,
        &[
            "macro_oscillator",
            "step_sequencer",
            "control_scheduler",
            "divisi",
        ],
        &[],
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
        "macro_oscillator.inversion",
        "macro_oscillator.model=chord",
        "macro_oscillator.harmonics=2",
        "chord_bank",
        "divisi.voices",
        "control_scheduler.steps",
    ] {
        assert!(problem_with(&registry, span).is_some(), "{span} passed");
    }
    for span in [
        "macro_oscillator.model=chords",
        "macro_oscillator.harmonics=0.44",
        "divisi.voice_count",
        "control_scheduler.schedule",
    ] {
        assert_eq!(problem_with(&registry, span), None, "{span}");
    }
}

/// Goertzel magnitude of `samples` at `frequency`.
fn magnitude(samples: &[f32], frequency: f32) -> f32 {
    let w = 2.0 * std::f32::consts::PI * frequency / SR as f32;
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &x in samples {
        let s0 = x + 2.0 * w.cos() * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2).sqrt() / samples.len() as f32
}

/// The left channel of the chords model on D3 in D dorian at `harmonics`,
/// after its first quarter second.
fn chord_at(harmonics: f32) -> Vec<f32> {
    let document = json!({
        "version": "1.0.0",
        "modules": [
            {"id": "chords", "type": "macro_oscillator", "config": {
                "model": "chords", "key": "D", "scale": "dorian",
                "frequency": 146.83, "harmonics": harmonics, "morph": 0.256}},
            {"id": "dac", "type": "dac"}],
        "connections": [
            {"from": "chords", "from_port": "audio", "to": "dac", "to_port": "audio"}]
    });
    let mut engine = RenderEngine::new(SR);
    engine.load_json(&document.to_string()).expect("loads");
    let mut out = vec![0.0f32; 2 * SR as usize];
    engine.render_interleaved(&mut out).expect("renders");
    out.chunks(2).skip(SR as usize / 4).map(|f| f[0]).collect()
}

/// The shapes the policy places along `harmonics` play the notes it says:
/// from D, sus4 has G and no F, triad has F and no G, seventh has C.
#[test]
fn the_chord_shapes_sit_where_the_policy_says() {
    let (f, g, a, b, c, e) = (174.614, 195.998, 220.0, 246.942, 261.626, 329.628);
    // A harmonic of the root that every shape here carries: the reference.
    let reference_of = |samples: &[f32]| magnitude(samples, 293.665);
    let text = policy_text(POLICY);
    for shape in ["sus4 (0.34)", "triad (0.44)", "seventh (0.64)"] {
        assert!(text.contains(shape), "the policy no longer says {shape}");
    }
    let sus4 = chord_at(0.34);
    let triad = chord_at(0.44);
    let seventh = chord_at(0.64);
    let (present, absent) = (0.1, 0.02);
    // What each shape sounds above the root and what only another shape does:
    // the sixth adds B, the ninth E and drops A, so each is told apart.
    for (name, samples, has, lacks) in [
        ("sus4", &sus4, vec![g, a], vec![f]),
        ("triad", &triad, vec![f, a], vec![g, b, c]),
        ("seventh", &seventh, vec![f, a, c], vec![g, b, e]),
    ] {
        let reference = reference_of(samples);
        assert!(reference > 0.0, "{name} is silent");
        for hz in has {
            let at = magnitude(samples, hz);
            assert!(
                at > present * reference,
                "{name} lacks {hz} Hz: {at} against a reference of {reference}"
            );
        }
        for hz in lacks {
            let at = magnitude(samples, hz);
            assert!(
                at < absent * reference,
                "{name} has {hz} Hz: {at} against a reference of {reference}"
            );
        }
    }
}

/// The `harmonics` description lists the shapes in the order the policy does.
#[test]
fn the_shape_order_is_the_registrys() {
    let registry = ModuleRegistry::default();
    let query = DescribeModuleQuery {
        module_type: Some("macro_oscillator".into()),
        ..Default::default()
    };
    let info = ModuleDescription::from_registry(&registry, SR, RegistryScope::Builtins, &query)
        .expect("describes")
        .info;
    let harmonics = info
        .controls
        .iter()
        .find(|c| c.key == "harmonics")
        .expect("harmonics");
    let listed =
        "chord (octave, fifth, sus2, sus4, triad, sixth, seventh, ninth, eleventh, six-nine)";
    assert!(
        harmonics.description.contains(listed),
        "the chord shapes changed: {}",
        harmonics.description
    );
    let text = policy_text(POLICY)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(text.contains(
        "octave, fifth, sus2, sus4 (0.34), triad (0.44), sixth, seventh (0.64), ninth, eleventh, six-nine."
    ));
}

/// What the policy says does not exist still does not, in the pinned core:
/// no module or control for chords, voicings or progressions, a step that
/// holds one note, and a voice allocator that takes one line. If one appears,
/// the policy's last section and its advice are out of date.
#[test]
fn the_gaps_the_policy_names_still_hold() {
    let registry = ModuleRegistry::default();
    let words = [
        "chord",
        "harmon",
        "voicing",
        "voice_lead",
        "progression",
        "inversion",
        "arpegg",
    ];
    for module_type in registry.types() {
        for word in words {
            assert!(
                !module_type.contains(word),
                "{module_type} may fill a gap the policy names"
            );
        }
    }
    // No control or config key on any module is about chords or voicing.
    for module_type in registry.types() {
        let query = DescribeModuleQuery {
            module_type: Some(module_type.into()),
            ..Default::default()
        };
        let controls: Vec<String> =
            ModuleDescription::from_registry(&registry, SR, RegistryScope::Builtins, &query)
                .map(|d| d.info.controls.into_iter().map(|c| c.key).collect())
                .unwrap_or_default();
        let config_keys = registry
            .config_keys(module_type)
            .iter()
            .map(|k| k.key.to_string());
        for key in controls.into_iter().chain(config_keys) {
            for word in words
                .into_iter()
                .filter(|w| *w != "harmon")
                .chain(["numeral", "symbol"])
            {
                assert!(
                    !key.contains(word),
                    "{module_type}.{key} may fill a gap the policy names"
                );
            }
        }
    }
    // Only chance and the chords model hold a key: no shared chord track.
    let holders = types_with_control(&registry, "key");
    assert_eq!(
        holders.iter().map(String::as_str).collect::<Vec<_>>(),
        ["chance", "macro_oscillator"]
    );
    // A step holds one note.
    for step in [
        json!({"note": 0, "notes": [4, 7]}),
        json!({"note": [0, 4, 7]}),
        json!({"chord": "Dm7"}),
    ] {
        let document = json!({
            "version": "1.0.0",
            "modules": [{"id": "seq", "type": "step_sequencer",
                "config": {"step_count": 1, "pattern": [step.clone()]}}],
            "connections": []
        });
        let mut engine = RenderEngine::new(SR);
        assert!(
            engine.load_json(&document.to_string()).is_err(),
            "a step may hold a chord: {step}"
        );
    }
    // divisi takes one line, so it cannot stack a chord.
    let query = DescribeModuleQuery {
        module_type: Some("divisi".into()),
        ..Default::default()
    };
    let info = ModuleDescription::from_registry(&registry, SR, RegistryScope::Builtins, &query)
        .expect("describes")
        .info;
    assert_eq!(info.inputs, ["frequency", "gate", "velocity"]);
    let text = policy_text(POLICY)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for stated in [
        "No chord-symbol input",
        "No voice-leading helper",
        "No shared chord track",
        "A step holds one note",
    ] {
        assert!(text.contains(stated), "the policy no longer says: {stated}");
    }
}

/// A rule has one home: one-at-a-time changes and landing a change are named
/// by their policies, not restated, and theory and rhythm are left to theirs.
#[test]
fn the_policy_names_other_policies_instead_of_restating_them() {
    assert_names_owners_without_restating(
        POLICY,
        &[
            "fugue.policy.theory",
            "fugue.policy.musical-intent",
            "fugue.policy.clean-audio",
        ],
        &[
            "\"kind\"",
            "headroom",
            "retrigger",
            "one at a time",
            "envelope",
            "next_bar",
            "modulat",
            "relative minor",
            "swing",
            "triplet",
            "beats_per_bar",
        ],
    );
}

/// American English.
#[test]
fn the_policy_uses_american_spelling() {
    assert_american_spelling(POLICY, &["harmonise", "favour"]);
}
