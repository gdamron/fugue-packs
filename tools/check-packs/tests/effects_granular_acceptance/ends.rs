//! The two ends of the mix and the silent corner: with `wet` and `reverb` 0
//! the pack is its input through the soft limit; with `dry` and `reverb` 0
//! and `wet` 1 it is the bare granular module through the soft limit; with
//! every level 0 it is silent. The mix also forwards `trigger` and `clock`
//! to the module.

use serde_json::{json, Value};

use super::rig::*;

const FRAMES: usize = 4 * SR as usize;

/// "with `wet` 0 and `reverb` 0 (dry at its default 1), each output is
/// `limit(input)` from the first sample: for an input whose peak stays below
/// the knee (e.g. -3.2 dBFS) it equals the input to within 1e-6", whatever
/// else is set.
#[test]
fn with_wet_and_reverb_off_each_output_is_its_input_below_the_knee() {
    let feeds = [Feed::noise_pair(0.69), Feed::sine(220.0, 0.5)];
    for (feed, extra) in feeds
        .iter()
        .zip([json!({}), json!({ "mode": "stretch", "feedback": 1.0 })])
    {
        let config = with(&json!({ "wet": 0.0, "reverb": 0.0 }), extra);
        let (left, right) = render_doc(&feed.effect(config.clone(), 1.0), FRAMES / 4);
        let (in_left, in_right) = render_doc(&feed.dry(), FRAMES / 4);
        assert!(peak(&in_left) > 0.1, "the input is audible");
        assert!(peak(&in_left).max(peak(&in_right)) < 0.7, "below the knee");
        assert_near(&format!("left {config}"), &left, &in_left, 1e-6);
        assert_near(&format!("right {config}"), &right, &in_right, 1e-6);
    }
}

/// "for a hot input (e.g. noise brought 8 dB over full scale with a `gain`
/// module) it is `limit(input)` sample for sample (within 1e-5), with no
/// added delay."
#[test]
fn with_wet_and_reverb_off_a_hot_input_is_limited_sample_for_sample() {
    let feed = Feed::hot_noise_pair(1.0, 8.0);
    for extra in [json!({}), json!({ "mode": "stretch", "feedback": 1.0 })] {
        let config = with(&json!({ "wet": 0.0, "reverb": 0.0 }), extra);
        let (left, right) = render_doc(&feed.effect(config.clone(), 1.0), FRAMES / 4);
        let (in_left, in_right) = render_doc(&feed.dry(), FRAMES / 4);
        assert!(
            peak(&in_left) > 1.5,
            "the input is hot ({})",
            peak(&in_left)
        );
        assert_near(&format!("left {config}"), &left, &limited(&in_left), 1e-5);
        assert_near(
            &format!("right {config}"),
            &right,
            &limited(&in_right),
            1e-5,
        );
    }
}

/// Renders the pack at `dry` 0, `wet` 1, `reverb` 0 with `controls` and the
/// bare module with the same, and asserts the pack's sides are `limit()` of
/// the bare module's within 1e-5.
fn assert_wet_end_matches(what: &str, feed: &Feed, controls: Value) {
    let ends = json!({ "dry": 0.0, "wet": 1.0, "reverb": 0.0 });
    let (left, right) = render_doc(&feed.effect(with(&controls, ends), 1.0), FRAMES);
    let (ref_left, ref_right) = render_doc(&feed.bare(&controls), FRAMES);
    let heard = peak(&ref_left[3 * SR as usize..]).min(peak(&ref_right[3 * SR as usize..]));
    assert!(heard > 0.01, "{what}: the bare module is heard ({heard})");
    assert_near(&format!("{what}, left"), &left, &limited(&ref_left), 1e-5);
    assert_near(
        &format!("{what}, right"),
        &right,
        &limited(&ref_right),
        1e-5,
    );
}

/// "with `dry` 0, `wet` 1, `reverb` 0, the output equals `limit()` of an
/// invention with core's bare `granular` module (config `buffer_length` 4.0,
/// `seed` 1, same other controls) fed the same input, sample for sample
/// within 1e-5", at defaults and with the controls moved, in each mode.
#[test]
fn with_dry_and_reverb_off_the_output_is_the_bare_granular_module() {
    let feed = Feed::noise_pair(0.8);
    let moved = json!({ "position": 0.2, "size": 0.3, "pitch": -3.0, "density": 0.9,
        "texture": 0.8, "spread": 0.6 });
    assert_wet_end_matches("defaults", &feed, json!({}));
    assert_wet_end_matches("moved controls", &feed, moved.clone());
    for mode in ["stretch", "looping_delay"] {
        let controls = with(&moved, json!({ "mode": mode, "position": 0.2 }));
        assert_wet_end_matches(mode, &feed, controls);
    }
}

/// `feedback` reaches the module. The bare module's `feedback` differs a
/// little over its first four seconds between a config start and a live
/// write before the first sample, so the pack's output (a start value
/// forwarded to the module) must equal the bare module's under one of the
/// two (through the limit), and differ from no feedback.
#[test]
fn feedback_reaches_the_module() {
    let feed = Feed::noise_pair(0.8);
    let controls = json!({ "position": 0.2, "size": 0.3, "density": 0.9 });
    let with_feedback = with(&controls, json!({ "feedback": 0.4 }));
    let ends = json!({ "dry": 0.0, "wet": 1.0, "reverb": 0.0 });
    let pack = render_doc(&feed.effect(with(&with_feedback, ends), 1.0), FRAMES);
    let from_config = render_doc(&feed.bare(&with_feedback), FRAMES);
    let mut live = engine(&feed.bare(&controls));
    set_on(&live, "g", "feedback", 0.4f32);
    let from_write = render(&mut live, FRAMES);
    let none = render_doc(&feed.bare(&controls), FRAMES);
    let same = |a: &[f32], b: &[f32]| (0..a.len()).all(|n| a[n].to_bits() == b[n].to_bits());
    let limited_same = |a: &[f32], b: &[f32]| {
        let b = limited(b);
        (0..a.len()).all(|n| (a[n] - b[n]).abs() <= 1e-5)
    };
    for (side, got, config, write, off) in [
        ("left", &pack.0, &from_config.0, &from_write.0, &none.0),
        ("right", &pack.1, &from_config.1, &from_write.1, &none.1),
    ] {
        assert!(
            !same(config, off),
            "{side}: feedback changes the bare module"
        );
        assert!(
            limited_same(got, config) || limited_same(got, write),
            "{side}: feedback 0.4 is the module's"
        );
    }
}

/// The pack's `trigger` and `clock` inputs reach the module's inputs of the
/// same names: a clock on `trigger` fires grains (density 0.5 has no
/// automatic ones), and a clock on `clock` with `sync` on gates them.
#[test]
fn trigger_and_clock_reach_the_module() {
    let feed = Feed::noise_pair(0.8);
    let on_trigger = feed.clocked("trigger", 90.0);
    let on_clock = feed.clocked("clock", 120.0);
    let triggered = json!({ "density": 0.5, "position": 0.2, "size": 0.1 });
    let synced = json!({ "density": 0.9, "position": 0.2, "size": 0.1, "sync": true });
    for (what, feed, controls) in [
        ("trigger", on_trigger, triggered),
        ("clock", on_clock, synced),
    ] {
        let ends = json!({ "dry": 0.0, "wet": 1.0, "reverb": 0.0 });
        let (left, right) = render_doc(&feed.effect(with(&controls, ends), 1.0), FRAMES);
        let (ref_left, ref_right) = render_doc(&feed.bare(&controls), FRAMES);
        assert!(peak(&ref_left) > 0.01, "{what}: the bare module is heard");
        assert_near(&format!("{what}, left"), &left, &limited(&ref_left), 1e-5);
        assert_near(
            &format!("{what}, right"),
            &right,
            &limited(&ref_right),
            1e-5,
        );
    }
}

/// "with `dry`, `wet` and `reverb` all 0 in config, the output is silent
/// (exactly 0) from the first sample."
#[test]
fn built_muted_it_is_silent_from_the_first_sample() {
    let config = json!({ "dry": 0.0, "wet": 0.0, "reverb": 0.0 });
    for feed in [Feed::noise_pair(0.8), Feed::sine(220.0, 0.5)] {
        let (left, right) = render_doc(&feed.effect(config.clone(), 1.0), FRAMES / 4);
        assert_eq!(peak(&left).max(peak(&right)), 0.0, "leaks");
    }
}
