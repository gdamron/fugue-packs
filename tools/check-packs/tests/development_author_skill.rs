//! The development author skill, `fugue.development.author`, checked against
//! the repository's real `packs/`: the skill that packages a reusable voice
//! or subgraph as a development, leaving every rule to the authoring and
//! clean-audio policies, with an example development that drops in cleanly.

mod policy_support;
mod skill_support;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use fugue::{Invention, RenderEngine};
use policy_support::{flat, pack, repository, SR};
use serde_json::{json, Value};
use skill_support::{
    assert_body_fits, assert_first_party_skill, assert_names_exist, assert_requires_policies,
    assert_restates_no_policy, assert_thin_shape, format_fields, interface_names, section, skill,
    DEVELOPMENT_TOOLS, MAX_BODY_CHARACTERS,
};

const SKILL: &str = "fugue.development.author";
const POLICIES: [&str; 2] = ["fugue.policy.authoring", "fugue.policy.clean-audio"];
const DEVELOPMENT: &str = "examples/glass-mallet.development.json";
const TRIAL: &str = "examples/glass-mallet-trial.json";
/// The bundled developments whose interface the skill tells authors to share.
const VOICES: [&str; 5] = ["piano", "marimba", "vibraphone", "pluck", "pad"];
const EFFECTS: [(&str, &str); 3] = [
    ("fugue.effects.eq", "eq.development.json"),
    ("fugue.effects.stereo-delay", "delay.development.json"),
    ("fugue.effects.chorus", "chorus.development.json"),
];

fn voice(name: &str) -> PathBuf {
    pack(&format!("fugue.instruments.{name}")).join("voice.json")
}

fn example(file: &str) -> PathBuf {
    pack(SKILL).join(file)
}

/// The pack is a first-party skill for both targets, and the repository
/// passes check-packs with it, warning about nothing.
#[test]
fn the_development_author_is_a_first_party_skill() {
    assert_first_party_skill(SKILL, &["external-agent", "in-graph-agent"]);
    let skill = skill(SKILL);
    assert_eq!(skill.name, "fugue-development-author");
    let description = skill.description.to_lowercase();
    for word in [
        "reusable",
        "development",
        "instrument",
        "use when",
        "not for",
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

/// Both policies are required and linked, and neither is restated.
#[test]
fn the_development_author_requires_its_policies_without_restating_them() {
    let skill = skill(SKILL);
    assert_requires_policies(&skill, &POLICIES);
    assert_restates_no_policy(&skill);
}

/// Thin shape and size, and every span is a real tool, module type, control,
/// format field or interface name of a bundled development.
#[test]
fn the_development_author_is_thin_short_and_names_only_what_exists() {
    let skill = skill(SKILL);
    assert_thin_shape(&skill, 5);
    assert_body_fits(&skill, MAX_BODY_CHARACTERS);

    let manifest = pack("fugue.instruments.pluck").join("fugue.pkg.json");
    let mut names: BTreeSet<String> = format_fields(&manifest);
    names.extend(interface_names(&voice("piano")));
    names.extend(interface_names(&pack(EFFECTS[0].0).join(EFFECTS[0].1)));
    names.extend(interface_names(&example(DEVELOPMENT)));
    let mut others: Vec<&str> = names.iter().map(String::as_str).collect();
    others.extend(DEVELOPMENT_TOOLS);
    others.push("fugue.pkg.json");
    assert_names_exist(&skill, &others);
}

/// Boundary, interface, clean defaults, trial, then version and package.
#[test]
fn the_procedure_covers_interface_naming_defaults_and_versioning() {
    let skill = skill(SKILL);
    let procedure = flat(section(&skill.body, "Procedure"));
    let at = |needle: &str| {
        procedure
            .find(needle)
            .unwrap_or_else(|| panic!("the procedure should say `{needle}`"))
    };
    let steps = [
        at("1. Draw the boundary"),
        at("2. Name the interface after the bundled developments"),
        at("3. Make the defaults clean anywhere"),
        at("4. Try it where it will be used"),
        at("5. Version and package"),
    ];
    assert!(
        steps.windows(2).all(|pair| pair[0] < pair[1]),
        "{procedure}"
    );
    for needle in [
        "fan it out",
        "with a `range`",
        "as the clean-audio policy asks",
        "quick repeated notes",
        "Raise its `version` for every change",
        "Keep port and control names across versions",
    ] {
        at(needle);
    }
}

/// The names the skill gives a voice and a stereo effect are the ones every
/// bundled instrument and the bundled effects expose.
#[test]
fn the_shared_interface_is_the_bundled_developments_own() {
    let voice_names = ["frequency", "gate", "pedal", "audio"];
    for name in VOICES {
        let names = interface_names(&voice(name));
        for port in voice_names {
            assert!(names.contains(port), "{name} has no `{port}`");
        }
    }
    for (id, file) in EFFECTS {
        let document: Invention =
            Invention::from_json(&std::fs::read_to_string(pack(id).join(file)).unwrap()).unwrap();
        for side in ["audio_left", "audio_right"] {
            assert!(document.inputs.iter().any(|i| i.name == side), "{id} in");
            assert!(document.outputs.iter().any(|o| o.name == side), "{id} out");
        }
    }
    let skill = skill(SKILL);
    let procedure = flat(section(&skill.body, "Procedure"));
    assert!(procedure.contains(
        "`frequency`, `gate` and `pedal` in and `audio` out for a voice, \
         `audio_left` and `audio_right` in and out for a stereo effect"
    ));
}

/// The example development follows the skill: the voice interface, a range
/// on every control, an exposed output level that starts modest, and an
/// envelope behind a sustain on the gate.
#[test]
fn the_example_development_follows_the_skill() {
    let raw: Value =
        serde_json::from_str(&std::fs::read_to_string(pack(SKILL).join("fugue.pkg.json")).unwrap())
            .unwrap();
    assert_eq!(raw["examples"], json!([DEVELOPMENT, TRIAL]));

    let text = std::fs::read_to_string(example(DEVELOPMENT)).unwrap();
    let development = Invention::from_json(&text).unwrap();
    assert_eq!(
        interface_names(&example(DEVELOPMENT)),
        [
            "frequency",
            "gate",
            "pedal",
            "audio",
            "brightness",
            "decay",
            "release",
            "level"
        ]
        .into_iter()
        .map(String::from)
        .collect()
    );
    assert!(development.controls.iter().all(|c| c.range.is_some()));
    let type_of = |id: &str| {
        let module = development.modules.iter().find(|m| m.id == id).unwrap();
        (module.module_type.as_str(), &module.config)
    };
    let gate = development
        .inputs
        .iter()
        .find(|i| i.name == "gate")
        .unwrap();
    assert_eq!(type_of(&gate.to).0, "sustain");
    let level = development
        .controls
        .iter()
        .find(|c| c.name == "level")
        .unwrap();
    let (trim, config) = type_of(&level.module);
    assert_eq!(trim, "vca");
    assert!(
        config["level"].as_f64().unwrap() <= 0.5,
        "a modest output level"
    );
    assert!(!development.modules.iter().any(|m| m.module_type == "dac"));
}

/// The rendered peak of `engine` over `seconds`, failing on a jump between
/// samples large enough to click.
fn peak_without_clicks(engine: &mut RenderEngine, seconds: usize) -> f32 {
    let mut out = vec![0.0f32; 2 * SR as usize * seconds];
    engine.render_interleaved(&mut out).expect("renders");
    for side in 0..2 {
        let channel: Vec<f32> = out.iter().skip(side).step_by(2).copied().collect();
        let jump = channel
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(jump < 0.1, "a jump of {jump} between samples");
    }
    out.iter().fold(0.0f32, |peak, s| peak.max(s.abs()))
}

/// The trial example loads from its file, importing the development by its
/// relative path, and plays audibly with headroom and no clicks.
#[test]
fn the_trial_example_loads_and_plays_cleanly() {
    let invention = Invention::from_file(example(TRIAL).to_str().unwrap()).expect("parses");
    let mut engine = RenderEngine::new(SR);
    engine.load_invention(invention).expect("the trial loads");
    let warnings = engine.load_warnings();
    assert!(warnings.is_empty(), "the trial loads with {warnings:?}");
    let peak = peak_without_clicks(&mut engine, 4);
    assert!((0.01..0.5).contains(&peak), "peak {peak}");
}

/// Dropped straight into a dac at its defaults, with fast gates, the
/// development stays below full scale and does not click on retriggers.
#[test]
fn the_example_development_is_clean_at_its_defaults_on_fast_gates() {
    let path = example(DEVELOPMENT);
    let document = json!({
        "version": "1.0.0",
        "developments": [{ "name": "mallet", "path": path.to_str().unwrap() }],
        "modules": [
            { "id": "clock", "type": "clock", "config": { "bpm": 240.0 } },
            { "id": "voice", "type": "mallet" },
            { "id": "dac", "type": "dac", "config": { "soft_clip": false } }
        ],
        "connections": [
            { "from": "clock", "from_port": "beat_x2", "to": "voice", "to_port": "gate" },
            { "from": "voice", "from_port": "audio", "to": "dac", "to_port": "audio_left" },
            { "from": "voice", "from_port": "audio", "to": "dac", "to_port": "audio_right" }
        ]
    });
    let mut engine = RenderEngine::new(SR);
    engine.load_json(&document.to_string()).expect("loads");
    assert!(engine.load_warnings().is_empty());
    let peak = peak_without_clicks(&mut engine, 3);
    assert!((0.05..0.6).contains(&peak), "peak {peak}");
}

/// Each stop says what the user can do next.
#[test]
fn the_stop_conditions_cover_the_four_dead_ends() {
    let skill = skill(SKILL);
    let stops = flat(section(&skill.body, "Stop conditions"));
    assert!(stops.contains("say what the user can do next"));
    for (stop, next) in [
        (
            "The part cannot stand alone.",
            "offer to expose that as an input",
        ),
        (
            "A change would rename or remove a released port or control.",
            "ask first",
        ),
        ("It is not clean at its defaults.", "Do not package"),
        ("It cannot be installed here.", "import by `path`"),
    ] {
        let bullet = stops
            .split("- **")
            .find(|bullet| bullet.starts_with(stop))
            .unwrap_or_else(|| panic!("no stop for `{stop}`"));
        assert!(bullet.contains(next), "`{stop}` should offer `{next}`");
    }
}
