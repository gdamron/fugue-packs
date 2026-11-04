//! The bands' responses through the pack's controls: a few of the issue's
//! bell cases (±0.5 dB at the centre, ±1 dB at the band edges), the shelves'
//! plateaus (±0.5 dB), and the cuts at 12 and 24 dB/oct. Expected values are
//! the RBJ Audio EQ Cookbook's, designed with the same f0, gain and Q.
//! Tones sit slightly off round frequencies, so no period is a whole number
//! of samples.

use serde_json::json;

use super::rig::*;

/// Nudges a round frequency off a whole-sample period.
fn off(f: f64) -> f64 {
    f * 1.000_37
}

fn check(what: &str, got: f64, want: f64, tolerance: f64) {
    let ok = (got - want).abs() <= tolerance;
    assert!(ok, "{what}: {got:.2} dB, want {want:.2} ± {tolerance} dB");
}

/// "Bells at ±12 dB (100 Hz, 1 kHz, 8 kHz; Q 0.7, 2, 8)": one case per bell
/// and per Q, each set through that bell's controls. Band edges are the
/// bandpass half-power points f0 (sqrt(1 + 1/4Q²) ± 1/2Q).
#[test]
fn bells_match_the_cookbook_at_centre_and_band_edges() {
    for (bell, f0, boost, q) in [
        (0, 100.0, 12.0, 0.7f64),
        (1, 1000.0, -12.0, 2.0),
        (2, 8000.0, 12.0, 8.0),
    ] {
        let mut config = json!({});
        config[format!("bell_frequency.{bell}")] = json!(f0);
        config[format!("bell_boost.{bell}")] = json!(boost);
        config[format!("bell_q.{bell}")] = json!(q);
        let what = format!("bell {bell} at {f0} Hz, {boost} dB, q {q}");
        let centre = gain_db(config.clone(), off(f0));
        check(&format!("{what}, centre"), centre, boost, 0.5);
        let (root, half) = ((1.0 + 1.0 / (4.0 * q * q)).sqrt(), 1.0 / (2.0 * q));
        for edge in [f0 * (root - half), f0 * (root + half)] {
            let want = rbj_db(Rbj::Peaking, f0, boost, q, edge);
            let got = gain_db(config.clone(), edge);
            check(&format!("{what}, edge {edge:.1} Hz"), got, want, 1.0);
        }
    }
}

/// "Shelves within ±0.5 dB at their plateau", with the half-way point at
/// the shelf frequency and the far side untouched.
#[test]
fn shelves_reach_their_plateau() {
    for boost in [12.0, -12.0] {
        let config = json!({ "low_shelf_frequency": 200.0, "low_shelf_boost": boost });
        let what = format!("low shelf at 200 Hz, {boost} dB");
        check(
            &format!("{what}, plateau"),
            gain_db(config.clone(), 30.7),
            boost,
            0.5,
        );
        let mid = rbj_db(
            Rbj::LowShelf,
            200.0,
            boost,
            std::f64::consts::FRAC_1_SQRT_2,
            off(200.0),
        );
        check(
            &format!("{what}, at 200 Hz"),
            gain_db(config.clone(), off(200.0)),
            mid,
            1.0,
        );
        check(
            &format!("{what}, at 5 kHz"),
            gain_db(config, off(5000.0)),
            0.0,
            0.5,
        );

        let config = json!({ "high_shelf_frequency": 3000.0, "high_shelf_boost": boost });
        let what = format!("high shelf at 3 kHz, {boost} dB");
        check(
            &format!("{what}, plateau"),
            gain_db(config.clone(), off(18_000.0)),
            boost,
            0.5,
        );
        let mid = rbj_db(
            Rbj::HighShelf,
            3000.0,
            boost,
            std::f64::consts::FRAC_1_SQRT_2,
            off(3000.0),
        );
        check(
            &format!("{what}, at 3 kHz"),
            gain_db(config.clone(), off(3000.0)),
            mid,
            1.0,
        );
        check(
            &format!("{what}, at 150 Hz"),
            gain_db(config, off(150.0)),
            0.0,
            0.5,
        );
    }
}

/// A Butterworth 12 dB/oct cut is 3 dB down at its frequency and about
/// 24 dB down two octaves into its stopband; `24db` at least doubles that
/// in dB. Both leave the passband two octaves the other side untouched.
fn cut(band: &str, cutoff: f64, stop: f64, pass: f64) {
    let config = |slope: &str| {
        let mut config = json!({});
        config[format!("{band}_bypass")] = json!(false);
        config[format!("{band}_frequency")] = json!(cutoff);
        config[format!("{band}_slope")] = json!(slope);
        config
    };
    let what = format!("{band} at {cutoff} Hz");
    check(
        &format!("{what}, 12db at cutoff"),
        gain_db(config("12db"), off(cutoff)),
        -3.0,
        1.0,
    );
    let gentle = gain_db(config("12db"), stop);
    check(&format!("{what}, 12db at {stop} Hz"), gentle, -24.0, 3.0);
    let steep = gain_db(config("24db"), stop);
    assert!(
        steep <= -42.0,
        "{what}, 24db at {stop} Hz: only {steep:.1} dB"
    );
    assert!(
        steep <= gentle - 18.0,
        "{what}: 24db ({steep:.1}) is not steeper than 12db ({gentle:.1})"
    );
    for slope in ["12db", "24db"] {
        check(
            &format!("{what}, {slope} at {pass} Hz"),
            gain_db(config(slope), pass),
            0.0,
            0.5,
        );
    }
}

#[test]
fn the_low_cut_rolls_off_at_12_or_24_db_per_octave() {
    cut("low_cut", 1000.0, off(250.0), off(16_000.0));
}

#[test]
fn the_high_cut_rolls_off_at_12_or_24_db_per_octave() {
    cut("high_cut", 1000.0, off(4000.0), off(62.5));
}
