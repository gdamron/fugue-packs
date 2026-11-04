//! "Stereo, linked: two identical chains ... every control moving both
//! sides. For mono, feed `audio_left` and take `audio_left`."

use serde_json::{json, Value};

use super::rig::*;

/// Every band active and away from its defaults.
fn shaped() -> Value {
    json!({
        "low_cut_bypass": false, "low_cut_frequency": 80.0, "low_cut_slope": "24db",
        "low_shelf_boost": 4.0, "low_shelf_frequency": 150.0, "low_shelf_q": 1.2,
        "bell_boost.0": -6.0, "bell_q.0": 3.0, "bell_frequency.0": 400.0,
        "bell_boost.1": 5.0, "bell_q.1": 0.8,
        "bell_boost.2": -3.0, "bell_q.2": 6.0, "bell_frequency.2": 6000.0,
        "high_shelf_boost": -4.0, "high_shelf_frequency": 9000.0, "high_shelf_q": 0.5,
        "high_cut_bypass": false, "high_cut_frequency": 15000.0
    })
}

/// The same noise in both inputs gives the same output on both sides, bit
/// for bit, while every control is moved live (a control that moved one
/// side only would split them).
#[test]
fn every_live_control_moves_both_sides() {
    let mut fx = engine(&Feed::noise_both(7).effect(shaped(), 0.25));
    let check = |fx: &mut fugue::RenderEngine, what: &str| {
        let (left, right) = render(fx, 1_920);
        assert!(peak(&left) > 0.0, "{what}: heard");
        assert_same(what, &right, &left, 0..left.len());
    };
    check(&mut fx, "configured");
    let moves: Vec<(String, fugue::ControlValue)> = vec![
        ("low_cut_frequency".into(), 200.0.into()),
        ("low_cut_slope".into(), "12db".into()),
        ("low_shelf_frequency".into(), 300.0.into()),
        ("low_shelf_boost".into(), (-8.0).into()),
        ("low_shelf_q".into(), 2.0.into()),
        ("bell_frequency.0".into(), 700.0.into()),
        ("bell_boost.0".into(), 9.0.into()),
        ("bell_q.0".into(), 10.0.into()),
        ("bell_frequency.1".into(), 2500.0.into()),
        ("bell_boost.1".into(), (-10.0).into()),
        ("bell_q.1".into(), 4.0.into()),
        ("bell_frequency.2".into(), 11000.0.into()),
        ("bell_boost.2".into(), 7.0.into()),
        ("bell_q.2".into(), 0.5.into()),
        ("high_shelf_frequency".into(), 5000.0.into()),
        ("high_shelf_boost".into(), 6.0.into()),
        ("high_shelf_q".into(), 1.5.into()),
        ("high_cut_frequency".into(), 7000.0.into()),
        ("high_cut_slope".into(), "24db".into()),
        ("low_cut_bypass".into(), true.into()),
        ("low_shelf_bypass".into(), true.into()),
        ("bell_bypass.0".into(), true.into()),
        ("bell_bypass.1".into(), true.into()),
        ("bell_bypass.2".into(), true.into()),
        ("high_shelf_bypass".into(), true.into()),
        ("high_cut_bypass".into(), true.into()),
    ];
    for (key, value) in moves {
        (fx.set_control("fx", &key, value)).unwrap_or_else(|e| panic!("fx.{key}: {e}"));
        check(&mut fx, &format!("after {key}"));
    }
}

/// The sides are independent identical chains: with different noise in
/// each, the left output is what the left noise alone gives in `audio_left`
/// (the mono use), and the right output is what the right noise gives there.
#[test]
fn each_side_is_the_mono_chain_of_its_own_input() {
    let frames = 24_000;
    let (left, right) = render_doc(&Feed::noise_pair(1.0).effect(shaped(), 0.25), frames);
    let (dry, _) = render_doc(&Feed::noise_pair(1.0).dry(), frames);
    assert!(
        first_difference(&left, &dry, 0..frames).is_some(),
        "the EQ shapes the noise"
    );
    let (mono_left, _) = render_doc(&Feed::noise_left("nl", 41).effect(shaped(), 0.25), frames);
    let (mono_right, _) = render_doc(&Feed::noise_left("nr", 42).effect(shaped(), 0.25), frames);
    assert_same("left against mono", &left, &mono_left, 0..frames);
    assert_same("right against mono", &right, &mono_right, 0..frames);
}
