//! "every control exposed by name": the nine controls the interface pins
//! are listed on the development by name, start at their defaults, and
//! accept a live write by that name.

use serde_json::json;

use super::rig::*;

/// (name, default, a value inside the range to write).
const CONTROLS: [(&str, f32, f32); 9] = [
    ("threshold", -20.0, -35.0),
    ("ratio", 4.0, 10.0),
    ("knee", 6.0, 0.0),
    ("attack", 0.01, 0.05),
    ("release", 0.1, 0.4),
    ("sidechain_mix", 0.0, 1.0),
    ("highpass", 0.0, 120.0),
    ("makeup", 0.0, 6.0),
    ("mix", 1.0, 0.5),
];

fn number(engine: &fugue::RenderEngine, key: &str) -> f32 {
    let value = (engine.get_control("fx", key)).unwrap_or_else(|e| panic!("fx.{key}: {e}"));
    (value.as_number()).unwrap_or_else(|e| panic!("fx.{key} is a number: {e}"))
}

/// Each pinned control is listed under its name and reads its default
/// (to f32 rounding: 1e-6 relative, so 0.01 s compares as 0.01).
#[test]
fn every_control_is_listed_by_name_at_its_default() {
    let rig = Rig::new()
        .osc("a", "sine", 220.0, 0.1)
        .feed_both("a", "audio");
    let engine = engine(&rig.effect(json!({})));
    let listed = engine.list_controls(Some("fx")).expect("fx lists controls");
    let keys: Vec<_> = listed
        .iter()
        .flat_map(|(_, c)| c.iter().map(|m| m.key.clone()))
        .collect();
    for (key, default, _) in CONTROLS {
        assert!(keys.iter().any(|k| k == key), "fx lists {key}: {keys:?}");
        let got = number(&engine, key);
        assert!(
            (got - default).abs() <= 1e-6 * default.abs().max(1.0),
            "fx.{key} starts at {got}, want {default}"
        );
    }
}

/// A write by each name is accepted live and reads back.
#[test]
fn every_control_accepts_a_live_write_by_name() {
    let rig = Rig::new()
        .osc("a", "sine", 220.0, 0.1)
        .feed_both("a", "audio");
    let mut engine = engine(&rig.effect(json!({})));
    render(&mut engine, 1_024);
    for (key, _, value) in CONTROLS {
        set(&engine, key, value);
    }
    render(&mut engine, SR as usize / 10);
    for (key, _, value) in CONTROLS {
        let got = number(&engine, key);
        assert!(
            (got - value).abs() <= 1e-6 * value.abs().max(1.0),
            "fx.{key} reads {got} after a write of {value}"
        );
    }
}

/// Each control given in the module's config is its starting value.
#[test]
fn every_control_can_start_from_config_by_name() {
    let rig = Rig::new()
        .osc("a", "sine", 220.0, 0.1)
        .feed_both("a", "audio");
    let mut config = json!({});
    for (key, _, value) in CONTROLS {
        config[key] = json!(value);
    }
    let engine = engine(&rig.effect(config));
    for (key, _, value) in CONTROLS {
        let got = number(&engine, key);
        assert!(
            (got - value).abs() <= 1e-6 * value.abs().max(1.0),
            "fx.{key} starts at {got} from a config of {value}"
        );
    }
}
