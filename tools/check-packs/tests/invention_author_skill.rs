//! The authoring skill, `fugue.invention.author`, checked against the
//! repository's real `packs/` and the pinned core: the skill an agent uses to
//! build a new invention, leaving every rule to the seven policies it
//! requires and keeping only the module-graph patterns the authoring policy
//! does not state, each checked here against the core it names.

mod policy_support;
mod skill_support;

use std::fs;
use std::process::Command;

use fugue::pkg::parse_path;
use fugue::RenderEngine;
use policy_support::{flat, pack, repository, SR};
use skill_support::{
    assert_body_fits, assert_first_party_skill, assert_names_exist, assert_requires_policies,
    assert_restates_no_policy, assert_thin_shape, section, skill, LOCAL_TOOLS, MAX_BODY_CHARACTERS,
};

const SKILL: &str = "fugue.invention.author";
const EXAMPLE: &str = "examples/dorian-evening.invention.json";
/// Read only when the task calls for them.
const MUSIC_POLICIES: [&str; 3] = [
    "fugue.policy.theory",
    "fugue.policy.harmony",
    "fugue.policy.rhythm",
];
const POLICIES: [&str; 7] = [
    "fugue.policy.authoring",
    "fugue.policy.clean-audio",
    "fugue.policy.musical-intent",
    "fugue.policy.listening",
    MUSIC_POLICIES[0],
    MUSIC_POLICIES[1],
    MUSIC_POLICIES[2],
];

/// A first-party skill for both kinds of agent, and the repository passes
/// check-packs with it, warning about nothing.
#[test]
fn the_authoring_skill_is_a_first_party_skill_for_both_agents() {
    assert_first_party_skill(SKILL, &["external-agent", "in-graph-agent"]);
    let skill = skill(SKILL);
    assert_eq!(skill.name, "fugue-invention-author");
    let description = skill.description.to_lowercase();
    for words in ["build me an invention", "new piece", "use when", "not for"] {
        assert!(
            description.contains(words),
            "the description omits `{words}`"
        );
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

/// All seven policies are required and linked, none is restated, and the
/// three music references are read only when the task calls for them.
#[test]
fn the_authoring_skill_requires_seven_policies_without_restating_them() {
    let skill = skill(SKILL);
    assert_requires_policies(&skill, &POLICIES);
    assert_restates_no_policy(&skill);

    let required = flat(section(&skill.body, "Required policies"));
    let when = required
        .find("only when the task calls for them")
        .expect("the music policies are read only when needed");
    for policy in POLICIES {
        let link = required
            .find(&format!("](policies/{policy}.md)"))
            .unwrap_or_else(|| panic!("{policy} is not linked"));
        let conditional = MUSIC_POLICIES.contains(&policy);
        assert_eq!(link > when, conditional, "{policy} is in the wrong place");
    }
}

/// Thin shape, within the shared size limit, naming only real things.
#[test]
fn the_authoring_skill_is_thin_short_and_names_only_what_exists() {
    let skill = skill(SKILL);
    assert_thin_shape(&skill, 5);
    assert_body_fits(&skill, MAX_BODY_CHARACTERS);
    assert_names_exist(&skill, &["fugue", EXAMPLE]);
}

/// Plan, author one complete document, load, listen, refine, in that order.
#[test]
fn the_procedure_plans_authors_loads_listens_and_refines() {
    let skill = skill(SKILL);
    let procedure = flat(section(&skill.body, "Procedure"));
    let at = |needle: &str| {
        procedure
            .find(needle)
            .unwrap_or_else(|| panic!("the procedure should say `{needle}`"))
    };
    let steps = [
        at("1. **Plan the piece.**"),
        at("2. **Author one complete document.**"),
        at("3. **Load it** with `load_invention`"),
        at("4. **Listen** with `get_meters`"),
        at("5. **Refine**"),
    ];
    assert!(
        steps.windows(2).all(|pair| pair[0] < pair[1]),
        "{procedure}"
    );
    for tool in ["load_invention", "reload_invention"] {
        assert!(LOCAL_TOOLS.contains(&tool), "{tool} is a local tool");
    }
    assert!(procedure.contains(&format!("`{EXAMPLE}`")));
}

/// Every example the manifest ships loads in the pinned core with no
/// warnings and plays, with its peak under full scale.
#[test]
fn the_example_invention_loads_and_plays_with_headroom() {
    let manifest = parse_path(pack(SKILL).join("fugue.pkg.json")).unwrap();
    let examples = manifest.examples.expect("the skill ships examples");
    assert_eq!(examples, [EXAMPLE]);
    for example in examples {
        let text = fs::read_to_string(pack(SKILL).join(&example)).unwrap();
        let (left, right) = peaks(&text, 8);
        assert!(left > 0.05 && right > 0.05, "{example} is silent");
        assert!(
            left < 1.0 && right < 1.0,
            "{example} peaks at {left}, {right}"
        );
    }
}

/// The pitfalls the skill states hold in the pinned core, so a core change
/// that ends one fails here instead of leaving the skill wrong.
#[test]
fn the_pitfalls_the_skill_states_hold_in_the_core() {
    let tone = r#"{"id":"tone","type":"oscillator","config":{"frequency":220}}"#;
    let to_mixer = |port: &str| {
        format!(
            r#"{{"modules":[{tone},{{"id":"mix","type":"mixer","config":{{"levels":[0.25,0.25,0.25],"master":0.8}}}},{{"id":"dac","type":"dac"}}],
            "connections":[{{"from":"tone","from_port":"audio","to":"mix","to_port":"{port}"}},
            {{"from":"mix","from_port":"audio_left","to":"dac","to_port":"audio"}}]}}"#
        )
    };
    // A mixer channel without a level plays at full level.
    let listed = peaks(&to_mixer("audio.0"), 1).0;
    let unlisted = peaks(&to_mixer("audio.3"), 1).0;
    assert!((unlisted / listed - 4.0).abs() < 0.1, "{listed} {unlisted}");
    // Past `channel_count` there is no input.
    assert!(load(&to_mixer("audio.4")).is_err());

    // A patched vca level replaces the configured one.
    let enveloped = |level: f32| {
        format!(
            r#"{{"modules":[{tone},{{"id":"clock","type":"clock"}},{{"id":"env","type":"adsr"}},
            {{"id":"amp","type":"vca","config":{{"level":{level}}}}},{{"id":"dac","type":"dac"}}],
            "connections":[{{"from":"clock","from_port":"beat","to":"env","to_port":"gate"}},
            {{"from":"env","from_port":"envelope","to":"amp","to_port":"level"}},
            {{"from":"tone","from_port":"audio","to":"amp","to_port":"audio"}},
            {{"from":"amp","from_port":"audio","to":"dac","to_port":"audio"}}]}}"#
        )
    };
    assert_eq!(peaks(&enveloped(0.0), 1), peaks(&enveloped(1.0), 1));

    // A mono line on one side of the dac plays in one ear; on `audio`, both.
    let to_dac = |port: &str| {
        format!(
            r#"{{"modules":[{tone},{{"id":"dac","type":"dac"}}],
            "connections":[{{"from":"tone","from_port":"audio","to":"dac","to_port":"{port}"}}]}}"#
        )
    };
    let (left, right) = peaks(&to_dac("audio_left"), 1);
    assert!(left > 0.5 && right == 0.0);
    let (left, right) = peaks(&to_dac("audio"), 1);
    assert!(left > 0.5 && right > 0.5);

    // A repeated id loads without a warning and keeps the last module.
    let repeated = |second: &str, port: &str| {
        format!(
            r#"{{"modules":[{tone},{{"id":"tone","type":"{second}"}},{{"id":"dac","type":"dac"}}],
            "connections":[{{"from":"tone","from_port":"{port}","to":"dac","to_port":"audio"}}]}}"#
        )
    };
    let engine = load(&repeated("noise", "white")).expect("a repeated id loads");
    assert!(engine.load_warnings().is_empty());
    assert!(
        load(&repeated("noise", "audio")).is_err(),
        "the oscillator is gone"
    );

    // A melody steps on its gate; noise has no audio port.
    let melody = r#"{"modules":[{"id":"clock","type":"clock"},{"id":"tune","type":"melody"}],
        "connections":[{"from":"clock","from_port":"beat","to":"tune","to_port":"clock"}]}"#;
    let refused = load(melody).err().expect("a melody has no clock port");
    assert!(
        refused.contains("gate"),
        "the refusal lists the ports: {refused}"
    );
    let key =
        r#"{"modules":[{"id":"tone","type":"oscillator","config":{"freq":220}}],"connections":[]}"#;
    let refused = load(key).err().expect("an unknown config key is refused");
    assert!(
        refused.contains("frequency"),
        "the refusal lists the keys: {refused}"
    );
    assert!(load(&to_dac("audio").replace("oscillator", "noise")).is_err());
}

/// The document loaded in the pinned core, or why not.
fn load(document: &str) -> Result<RenderEngine, String> {
    let mut document: serde_json::Value =
        serde_json::from_str(document).unwrap_or_else(|e| panic!("not JSON ({e}): {document}"));
    document["version"] = "1.0.0".into();
    let mut engine = RenderEngine::new(SR);
    engine
        .load_json(&document.to_string())
        .map_err(|e| e.to_string())?;
    Ok(engine)
}

/// The left and right peaks of `seconds` of the document's output.
fn peaks(document: &str, seconds: usize) -> (f32, f32) {
    let mut engine = load(document).unwrap_or_else(|e| panic!("fails to load ({e})"));
    let warnings = engine.load_warnings();
    assert!(warnings.is_empty(), "loads with {warnings:?}");
    let mut out = vec![0.0f32; 2 * 512];
    let (mut left, mut right) = (0.0f32, 0.0f32);
    for _ in 0..(seconds * SR as usize).div_ceil(512) {
        engine.render_interleaved(&mut out).expect("renders");
        for frame in out.chunks(2) {
            left = left.max(frame[0].abs());
            right = right.max(frame[1].abs());
        }
    }
    (left, right)
}
