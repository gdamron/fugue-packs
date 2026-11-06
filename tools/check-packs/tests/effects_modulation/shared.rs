//! What both packs share: layout and manifest, the control surface, the
//! stereo offset, tempo sync and the level at defaults.

use std::fs;
use std::process::Command;

use fugue::ControlValue;
use fugue::ControlValue::{Bool, Number, String as Choice};
use serde_json::{json, Value};

use super::rig::*;

// -- Layout and manifest ----------------------------------------------------

#[test]
fn check_packs_validates_the_repository_with_both_packs() {
    let root = repository_root();
    let out = Command::new(env!("CARGO_BIN_EXE_check-packs"))
        .arg(&root)
        .output()
        .expect("check-packs runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "check-packs failed: {stdout}{stderr}");
    for pack in BOTH {
        assert!(pack.dir().is_dir(), "{} is in packs/", pack.id);
    }
}

/// Id, kind `development`, the development file named `<name>.development.json`,
/// MIT, a LICENSE, and a date-based version YYYY.M.P (a year from 2026, a
/// month 1-12, a patch, no leading zeros).
#[test]
fn manifests_declare_a_development_entry_with_a_date_based_version() {
    for pack in BOTH {
        let path = pack.dir().join("fugue.pkg.json");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let manifest: Value = serde_json::from_str(&text).expect("manifest is JSON");
        assert_eq!(manifest["id"], pack.id);
        assert_eq!(manifest["kind"], "development", "{}", pack.id);
        assert_eq!(manifest["entry"]["development"], pack.file, "{}", pack.id);
        assert_eq!(manifest["license"], "MIT", "{}", pack.id);
        assert!(
            pack.dir().join("LICENSE").is_file(),
            "{} has a LICENSE",
            pack.id
        );
        let version = manifest["version"].as_str().expect("a version string");
        let parts: Vec<&str> = version.split('.').collect();
        let numbers: Vec<u32> = (parts.iter())
            .map(|p| {
                p.parse()
                    .unwrap_or_else(|_| panic!("{} version {version}", pack.id))
            })
            .collect();
        assert!(parts.len() == 3, "{}: {version} is YYYY.M.P", pack.id);
        assert!(parts.iter().all(|p| !p.starts_with('0') || *p == "0"));
        assert!((2026..2100).contains(&numbers[0]), "{}: {version}", pack.id);
        assert!((1..=12).contains(&numbers[1]), "{}: {version}", pack.id);
        fugue::pkg::parse_path(&path)
            .unwrap_or_else(|e| panic!("{}: fugue refuses it: {e}", pack.id));
        let development = fs::read_to_string(pack.dir().join(pack.file)).expect("development file");
        serde_json::from_str::<Value>(&development).expect("the development is JSON");
    }
}

// -- Controls ----------------------------------------------------------------

/// (name, default, a different value inside the range), per the interface.
fn controls(pack: Pack) -> Vec<(&'static str, ControlValue, ControlValue)> {
    let mut all = vec![
        ("time", Number(pack_time(pack)), Number(0.004)),
        (
            "rate",
            Number(if pack.id == CHORUS.id { 0.8 } else { 0.25 }),
            Number(1.5),
        ),
        ("division", Choice("off".into()), Choice("8th_note".into())),
        ("bpm", Number(120.0), Number(90.0)),
        ("depth", Number(0.5), Number(0.8)),
        (pack.offset, Number(0.25), Number(0.5)),
        (
            "dry",
            Number(if pack.id == CHORUS.id { 0.7 } else { 0.5 }),
            Number(0.3),
        ),
        ("wet", Number(0.5), Number(0.9)),
    ];
    if pack.id == FLANGER.id {
        all.push(("feedback", Number(0.5), Number(0.7)));
        all.push(("negative_feedback", Bool(false), Bool(true)));
    }
    all
}

fn pack_time(pack: Pack) -> f32 {
    if pack.id == CHORUS.id {
        0.012
    } else {
        0.002
    }
}

fn near(a: &ControlValue, b: &ControlValue) -> bool {
    match (a, b) {
        (Number(x), Number(y)) => (x - y).abs() <= 1e-5 * y.abs().max(1e-3),
        _ => a == b,
    }
}

fn silent_rig() -> Rig {
    (Rig::new().sine("s", FREQ_TEST, 0.1)).feed_both("s", "audio")
}

const FREQ_TEST: f64 = 440.0;

/// Every pinned control is listed by name and reads its default.
#[test]
fn controls_are_exposed_by_name_with_their_defaults() {
    for pack in BOTH {
        let engine = engine(&silent_rig().effect(pack, json!({}), 1.0));
        let listed = engine.list_controls(Some("fx")).expect("fx lists controls");
        let keys: Vec<_> = (listed.iter())
            .flat_map(|(_, c)| c.iter().map(|m| m.key.clone()))
            .collect();
        for (key, default, _) in controls(pack) {
            assert!(
                keys.iter().any(|k| k == key),
                "{} lists {key}: {keys:?}",
                pack.id
            );
            let got = engine.get_control("fx", key).expect("readable");
            assert!(
                near(&got, &default),
                "{}.{key} is {got:?}, want {default:?}",
                pack.id
            );
        }
    }
}

/// Each control accepts a live write and a config value by its name, and
/// reads it back.
#[test]
fn controls_accept_writes_and_config_by_name() {
    for pack in BOTH {
        let mut fx = engine(&silent_rig().effect(pack, json!({}), 1.0));
        render(&mut fx, 1_024);
        let mut config = json!({});
        for (key, _, value) in controls(pack) {
            set(&fx, key, value.clone());
            config[key] = match &value {
                Number(v) => json!(v),
                Choice(v) => json!(v),
                Bool(v) => json!(v),
            };
        }
        render(&mut fx, SR as usize / 10);
        let started = engine(&silent_rig().effect(pack, config, 1.0));
        for (key, _, value) in controls(pack) {
            let got = fx.get_control("fx", key).expect("readable");
            assert!(
                near(&got, &value),
                "{}.{key} reads {got:?} after writing {value:?}",
                pack.id
            );
            let got = started.get_control("fx", key).expect("readable");
            assert!(
                near(&got, &value),
                "{}.{key} starts at {got:?} from config {value:?}",
                pack.id
            );
        }
    }
}

// -- Stereo offset -----------------------------------------------------------

fn noise_rig(level: f64) -> Rig {
    Rig::new().noise("n", 5, level).feed_both("n", "white")
}

/// "`spread` 0 makes the sides identical for identical inputs", and the
/// same of the flanger's `stereo_phase`; at the default 0.25 they differ.
#[test]
fn a_zero_offset_gives_identical_sides_and_the_default_does_not() {
    let frames = 2 * SR as usize;
    for pack in BOTH {
        let config = json!({ pack.offset: 0.0 });
        let (left, right) = render_doc(&noise_rig(0.3).effect(pack, config, 1.0), frames);
        let worst = (left.iter().zip(&right)).fold(0.0f32, |m, (l, r)| m.max((l - r).abs()));
        assert!(
            worst <= 1e-6,
            "{} {} 0: sides differ by {worst}",
            pack.id,
            pack.offset
        );
        assert!(rms(&left) > 0.05, "{}: heard", pack.id);

        let (left, right) = render_doc(&noise_rig(0.3).effect(pack, json!({}), 1.0), frames);
        let worst = (left.iter().zip(&right)).fold(0.0f32, |m, (l, r)| m.max((l - r).abs()));
        assert!(
            worst > 0.01,
            "{}: default offset separates the sides ({worst})",
            pack.id
        );
    }
}

/// "each side's voices read their own side": noise on one input is heard
/// on that output only.
#[test]
fn each_side_reads_its_own_input() {
    for pack in BOTH {
        for (input, heard, silent) in [("audio_left", 0, 1), ("audio_right", 1, 0)] {
            let rig = Rig::new().noise("n", 5, 0.3).feed("n", "white", input);
            let (left, right) = render_doc(&rig.effect(pack, json!({}), 1.0), SR as usize);
            let sides = [left, right];
            assert!(rms(&sides[heard]) > 0.05, "{} {input}: heard", pack.id);
            assert!(
                peak(&sides[silent]) <= EPS,
                "{} {input}: other side silent",
                pack.id
            );
        }
    }
}

// -- Tempo sync --------------------------------------------------------------

fn tone() -> Rig {
    (Rig::new().sine("s", 1_000.0, 0.3)).feed_both("s", "audio")
}

fn sweep_render(rig: &Rig, pack: Pack, config: Value) -> (Vec<f32>, Vec<f32>) {
    render_doc(&rig.effect(pack, config, 1.0), 3 * SR as usize)
}

/// A 1 kHz tone is a whole number of cycles in 0.5 s, so a sweep that
/// repeats every 0.5 s makes the output repeat every 24,000 samples.
fn lag_difference(x: &[f32], lag: usize) -> f32 {
    let later = x[lag..].to_vec();
    relative_difference(x, &later, 12_000..x.len() - lag)
}

/// "A tempo patched to `bpm` with `division` set syncs the sweep: at 120
/// bpm with `quarter_note` the sweep repeats every 0.5 s": the output
/// repeats at 0.5 s and not at 0.3 s (nor, for the flanger's single voice
/// per side, at 0.25 s), and equals the unsynced sweep at 2 Hz. The
/// `bpm` control is set to 60 to show the patched tempo takes precedence,
/// and `rate` to 3.7 to show it is ignored while `division` is set.
#[test]
fn a_patched_tempo_syncs_the_sweep_to_the_division() {
    let patched = tone().tempo("tempo", 120.0).feed("tempo", "bpm", "bpm");
    for pack in BOTH {
        let config = json!({ "division": "quarter_note", "bpm": 60.0, "rate": 3.7 });
        let synced = sweep_render(&patched, pack, config);
        let free = sweep_render(&tone(), pack, json!({ "rate": 2.0 }));
        let lags: &[usize] = if pack.id == FLANGER.id {
            &[12_000, 14_400]
        } else {
            &[14_400]
        };
        for (side, x, y) in [("left", &synced.0, &free.0), ("right", &synced.1, &free.1)] {
            let what = format!("{} {side}", pack.id);
            let repeat = lag_difference(x, 24_000);
            assert!(
                repeat < 0.01,
                "{what}: repeats every 0.5 s (differs by {repeat})"
            );
            for lag in lags {
                let moved = lag_difference(x, *lag);
                assert!(
                    moved > 0.1,
                    "{what}: moves between lag {lag} samples ({moved})"
                );
            }
            let apart = relative_difference(x, y, 12_000..x.len());
            assert!(
                apart < 0.05,
                "{what}: equals the 2 Hz sweep (differs by {apart})"
            );
        }
    }
}

/// The `bpm` control sets the tempo while the input is unpatched: 90 bpm
/// with `quarter_note` is 1.5 Hz.
#[test]
fn the_bpm_control_sets_the_tempo_when_none_is_patched() {
    for pack in BOTH {
        let config = json!({ "division": "quarter_note", "bpm": 90.0 });
        let synced = sweep_render(&tone(), pack, config);
        let free = sweep_render(&tone(), pack, json!({ "rate": 1.5 }));
        let apart = relative_difference(&synced.0, &free.0, 12_000..synced.0.len());
        assert!(
            apart < 0.05,
            "{}: equals the 1.5 Hz sweep (differs by {apart})",
            pack.id
        );
    }
}

// -- Level at defaults -------------------------------------------------------

/// "At default settings, the RMS of L+R is within 3 dB of the mono input's",
/// read as the RMS of the mono fold (L+R)/2 against the mono input fed to
/// both sides. The input is pink noise (equal energy per octave, as music
/// has), and white noise too for the chorus. Last 4 s of 6 s.
///
/// White noise is not asked of the flanger: its dry and delayed copies are
/// uncorrelated and its two sides' delays differ, so a fold of defaults
/// (dry 0.5, wet 0.5 at feedback 0.5, sides a quarter cycle apart) gives
/// about -3.8 dB there by the arithmetic, a property of the design and not
/// of the criterion's intent.
#[test]
fn the_mono_fold_at_defaults_is_within_3_db_of_the_mono_input() {
    let frames = 6 * SR as usize;
    let window = 2 * SR as usize..frames;
    let cases = [(CHORUS, "pink"), (CHORUS, "white"), (FLANGER, "pink")];
    for (pack, port) in cases {
        for seed in [7, 8] {
            let rig = (Rig::new().noise("n", seed, 0.1)).feed_both("n", port);
            let (input, _) = render_doc(&rig.dry(), frames);
            let (left, right) = render_doc(&rig.effect(pack, json!({}), 1.0), frames);
            let fold: Vec<f32> = left
                .iter()
                .zip(&right)
                .map(|(l, r)| (l + r) / 2.0)
                .collect();
            let db = 20.0 * (rms(&fold[window.clone()]) / rms(&input[window.clone()])).log10();
            assert!(
                db.abs() <= 3.0,
                "{} on {port} (seed {seed}): fold is {db:.2} dB of the input",
                pack.id
            );
        }
    }
}
