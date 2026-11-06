//! The "no zipper" measurement shared by both packs: a 1 s sweep of `rate`
//! and `depth`, written live from the first chunk to the last, with the
//! largest sample-to-sample step of the output held to 1.5x the dry
//! input's.
//!
//! Why this is a fair bound for a correct, smoothly interpolated delay: a
//! voice that reads the input at a delay moving at `v` samples per sample
//! reproduces the input at `|1 - v|` times its slope, so its steps are at
//! most `|1 - v|` times the dry input's largest. The chorus's two voices
//! per side sweep in opposite directions, so the output steps are at most
//! `dry + wet (|1 - v| + |1 + v|)` times the dry input's, and the flanger's
//! one voice gives `dry + wet |1 - v|`. The cases pick `dry` and `wet` so
//! that stays near or under 1 at the fastest the case's sweep moves a
//! delay, leaving the 1.5x for interpolation error. A delay that steps by
//! a few samples (a zipper from block-rate or unsmoothed writes) puts
//! `slope x jump` into one sample and breaks the bound; the sine's steps
//! are all the same size, so the dry maximum is exact.

use serde_json::{json, Value};

use super::rig::*;

/// The input: a 1 kHz sine, peak `AMP` (a dry step of about 0.065).
pub const FREQ: f64 = 1_000.0;
/// Frames rendered before the sweep, with the controls at its start.
const PRIME: usize = 12_000;

/// One 1 s sweep: `rate` and `depth` move linearly in time from the first
/// value to the second (`rate` geometrically when `log_rate`).
pub struct Sweep {
    pub name: &'static str,
    pub rate: (f32, f32),
    pub depth: (f32, f32),
    pub log_rate: bool,
    pub dry: f32,
    pub wet: f32,
}

impl Sweep {
    fn at(&self, t: f32) -> (f32, f32) {
        let rate = if self.log_rate {
            self.rate.0 * (self.rate.1 / self.rate.0).powf(t)
        } else {
            self.rate.0 + (self.rate.1 - self.rate.0) * t
        };
        (rate, self.depth.0 + (self.depth.1 - self.depth.0) * t)
    }
}

/// The largest steps of both sides against the dry input's.
pub fn assert_bounded_steps(what: &str, (left, right): (&[f32], &[f32]), dry: &[f32]) {
    let limit = 1.5 * max_step(dry);
    assert!(limit > 0.01, "{what}: the dry reference moves ({limit})");
    for (side, x) in [("left", left), ("right", right)] {
        let step = max_step(x);
        let at = x.windows(2).position(|w| (w[1] - w[0]).abs() == step);
        assert!(
            step <= limit,
            "{what}, {side}: a step of {step} at {at:?} exceeds 1.5x the dry input's {}",
            limit / 1.5
        );
        assert!(
            rms(x) > 0.1 * rms(dry),
            "{what}, {side}: the output is heard (rms {})",
            rms(x)
        );
    }
}

/// The dry sine's last `SR` frames after `PRIME` frames.
fn dry_window(amp: f64) -> Vec<f32> {
    let rig = Rig::new().sine("s", FREQ, amp).feed_both("s", "audio");
    let (dry, _) = render_doc(&rig.dry(), PRIME + SR as usize);
    dry[PRIME..].to_vec()
}

/// Sweeps `pack` over `sweep` on a sine of `amp`, with `config` holding
/// any other starting values.
pub fn check_sweep(pack: Pack, sweep: &Sweep, amp: f64, config: Value) {
    let rig = Rig::new().sine("s", FREQ, amp).feed_both("s", "audio");
    let (rate, depth) = sweep.at(0.0);
    let mut config = config;
    for (key, value) in [
        ("rate", rate),
        ("depth", depth),
        ("dry", sweep.dry),
        ("wet", sweep.wet),
    ] {
        config[key] = json!(value);
    }
    let mut fx = engine(&rig.effect(pack, config, 1.0));
    render(&mut fx, PRIME);
    let out = render_chunked(&mut fx, SR as usize, |pos, fx| {
        let (rate, depth) = sweep.at(pos as f32 / SR as f32);
        set(fx, "rate", rate);
        set(fx, "depth", depth);
    });
    let what = format!("{} sweep '{}'", pack.id, sweep.name);
    assert_bounded_steps(&what, (&out.0, &out.1), &dry_window(amp));
}

pub const SLOW_FAST: (f32, f32) = (0.001, 100.0);
pub const FAST_SLOW: (f32, f32) = (100.0, 0.001);
pub const FULL: (f32, f32) = (0.0, 1.0);
pub const FULL_DOWN: (f32, f32) = (1.0, 0.0);

/// The rig itself, with no pack: the dry reference is a steady sine of the
/// expected step, and rendering in odd chunks gives the same samples as
/// one render.
#[test]
fn the_dry_reference_is_steady_and_chunked_rendering_matches_whole() {
    let rig = Rig::new().sine("s", FREQ, 0.5).feed_both("s", "audio");
    let (whole, _) = render_doc(&rig.dry(), 20_000);
    let step = 2.0 * 0.5 * (std::f64::consts::PI * FREQ / f64::from(SR)).sin();
    let got = f64::from(max_step(&whole[1_000..]));
    assert!(
        (got - step).abs() < 0.01 * step,
        "dry step {got}, want {step}"
    );
    let (chunked, _) = render_chunked(&mut engine(&rig.dry()), 20_000, |_, _| {});
    assert_eq!(whole, chunked);
}
