//! Transparency: the defaults pass the input through bit for bit, a
//! bypassed band is bit-identical to its input (in config, and once a live
//! toggle's crossfade is over), and toggling a bypass crossfades over at
//! least 5 ms without a click.

use serde_json::{json, Value};

use super::rig::*;

/// "With its defaults the EQ is bit-transparent", stereo and mono.
#[test]
fn the_defaults_are_bit_transparent() {
    let frames = 48_000;
    let feed = Feed::noise_pair(1.0);
    let (left, right) = render_doc(&feed.effect(json!({}), 1.0), frames);
    let (dry_left, dry_right) = render_doc(&feed.dry(), frames);
    assert_same("left", &left, &dry_left, 0..frames);
    assert_same("right", &right, &dry_right, 0..frames);

    let mono = Feed::noise_left("nl", 41);
    let (left, _) = render_doc(&mono.effect(json!({}), 1.0), frames);
    let (dry, _) = render_doc(&mono.dry(), frames);
    assert_same("mono", &left, &dry, 0..frames);
}

/// Each band, set so that it would change the signal, and its bypass key.
fn bands() -> Vec<(&'static str, Value, String)> {
    let mut bands = vec![
        (
            "low cut",
            json!({ "low_cut_bypass": false, "low_cut_frequency": 2000.0, "low_cut_slope": "24db" }),
            "low_cut_bypass".to_string(),
        ),
        (
            "low shelf",
            json!({ "low_shelf_frequency": 500.0, "low_shelf_boost": 12.0 }),
            "low_shelf_bypass".to_string(),
        ),
        (
            "high shelf",
            json!({ "high_shelf_frequency": 3000.0, "high_shelf_boost": 12.0 }),
            "high_shelf_bypass".to_string(),
        ),
        (
            "high cut",
            json!({ "high_cut_bypass": false, "high_cut_frequency": 2000.0, "high_cut_slope": "24db" }),
            "high_cut_bypass".to_string(),
        ),
    ];
    for (i, name) in ["bell 0", "bell 1", "bell 2"].into_iter().enumerate() {
        let mut config = json!({});
        config[format!("bell_boost.{i}")] = json!(12.0);
        config[format!("bell_q.{i}")] = json!(2.0);
        bands.push((name, config, format!("bell_bypass.{i}")));
    }
    bands
}

/// For each band: active, it changes the noise; bypassed in config, the
/// output is the input from the first sample; bypassed live, the output is
/// the input again once the crossfade is over (50 ms later).
#[test]
fn a_bypassed_band_is_bit_identical_to_its_input() {
    let feed = Feed::noise_pair(0.1);
    let (lead, tail) = (4_800, 9_600);
    let frames = lead + tail;
    let (dry_left, dry_right) = render_doc(&feed.dry(), frames);
    for (band, active, bypass) in bands() {
        let (left, _) = render_doc(&feed.effect(active.clone(), 1.0), frames);
        assert!(
            first_difference(&left, &dry_left, 0..frames).is_some(),
            "{band}: active, it changes the signal"
        );

        let mut config = active.clone();
        config[&bypass] = json!(true);
        let (left, right) = render_doc(&feed.effect(config, 1.0), frames);
        assert_same(
            &format!("{band} bypassed, left"),
            &left,
            &dry_left,
            0..frames,
        );
        assert_same(
            &format!("{band} bypassed, right"),
            &right,
            &dry_right,
            0..frames,
        );

        let mut fx = engine(&feed.effect(active, 1.0));
        render(&mut fx, lead);
        set(&fx, &bypass, true);
        let (left, right) = render(&mut fx, tail);
        let settled = 2_400..tail;
        let what = format!("{band} bypassed live, 50 ms on");
        assert_same(&what, &left, &dry_left[lead..], settled.clone());
        assert_same(&what, &right, &dry_right[lead..], settled);
    }
}

const TONE: f64 = 1000.3;
const LEAD: usize = 12_000;

/// Bell 1 at +12 dB, q 2, on its 1 kHz centre: a 0.1 sine comes out at 0.4.
fn boosted() -> Value {
    json!({ "bell_boost.1": 12.0, "bell_q.1": 2.0 })
}

/// "Toggling bypass crossfades over at least 5 ms": the output leaves the
/// boosted tone at the toggle and only reaches the dry tone (bit for bit)
/// 5 ms or more later, with no one-sample step larger than the boosted
/// tone's own.
#[test]
fn bypassing_crossfades_over_at_least_5_ms_without_a_click() {
    let feed = Feed::sine_both(TONE, 0.1);
    let tail = 9_600;
    let (wet, _) = render_doc(&feed.effect(boosted(), 1.0), LEAD + tail);
    let (dry, _) = render_doc(&feed.dry(), LEAD + tail);
    let wet_step = max_step(&wet[LEAD - 4800..LEAD]);
    let wet_peak = peak(&wet[LEAD - 4800..LEAD]);
    assert!(wet_peak > 0.35, "the bell boosts the tone ({wet_peak})");

    let mut fx = engine(&feed.effect(boosted(), 1.0));
    let (before, _) = render(&mut fx, LEAD);
    assert_same("before the toggle", &before, &wet, 0..LEAD);
    set(&fx, "bell_bypass.1", true);
    let (after, _) = render(&mut fx, tail);
    let mut out = before;
    out.extend_from_slice(&after);

    let start = first_difference(&out, &wet, LEAD..LEAD + tail).expect("the bypass lands");
    let end = (LEAD..LEAD + tail)
        .rev()
        .find(|&n| out[n].to_bits() != dry[n].to_bits())
        .map_or(start, |n| n + 1);
    let fade = end - start;
    assert!(fade >= 240, "crossfade lasts {fade} samples, under 5 ms");
    assert!(fade <= 2_400, "crossfade lasts {fade} samples, over 50 ms");
    assert!(end + 2_400 <= LEAD + tail, "then holds the dry tone");
    let step = max_step(&out[start - 1..end + 1]);
    assert!(
        step <= 1.1 * wet_step,
        "click: step {step} against the tone's {wet_step}"
    );
    let early = peak(&out[start..start + 96]);
    assert!(
        early > 0.2,
        "2 ms into the fade it is still boosted ({early})"
    );
}

/// Un-bypassing crossfades back in: no click, not yet at full boost 2 ms
/// in, at full boost 50 ms later.
#[test]
fn unbypassing_crossfades_back_without_a_click() {
    let feed = Feed::sine_both(TONE, 0.1);
    let tail = 9_600;
    let mut config = boosted();
    config["bell_bypass.1"] = json!(true);
    let (dry, _) = render_doc(&feed.dry(), LEAD + tail);
    let (wet, _) = render_doc(&feed.effect(boosted(), 1.0), LEAD);
    let wet_step = max_step(&wet[LEAD - 4800..]);

    let mut fx = engine(&feed.effect(config, 1.0));
    let (before, _) = render(&mut fx, LEAD);
    assert_same("bypassed", &before, &dry, 0..LEAD);
    set(&fx, "bell_bypass.1", false);
    let (after, _) = render(&mut fx, tail);
    let start = first_difference(&after, &dry[LEAD..], 0..tail).expect("the band returns");
    let early = peak(&after[start..start + 96]);
    assert!(early < 0.36, "2 ms in it is already at {early}: under 5 ms");
    let late = peak(&after[start + 2_400..start + 2_880]);
    assert!((late - 0.4).abs() < 0.02, "50 ms in it is boosted ({late})");
    let mut joined = dry[LEAD - 1..LEAD].to_vec();
    joined.extend_from_slice(&after);
    let step = max_step(&joined);
    assert!(
        step <= 1.1 * wet_step,
        "click: step {step} against the tone's {wet_step}"
    );
}
