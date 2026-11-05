//! The mix: dry and wet ends against the input and the bare module, and
//! the reverb send's tail.

use serde_json::json;

use super::rig::*;

/// Below the knee the limit passes the input: the dry end is the input to
/// within rounding.
#[test]
fn wet_and_reverb_at_zero_pass_the_input_below_the_knee() {
    let source = Source::noise_at(-3.2);
    let (want_l, want_r) = render_doc(&dry(&source), SECOND);
    let config = json!({ "wet": 0.0, "reverb": 0.0 });
    let (l, r) = render_doc(&effect(&source, config), SECOND);
    for (side, got, want) in [("left", &l, &want_l), ("right", &r, &want_r)] {
        assert!(peak(want) > 0.68, "{side}: the input reaches the knee");
        let off = got
            .iter()
            .zip(want.iter())
            .fold(0.0f32, |m, (g, w)| m.max((g - w).abs()));
        assert!(off <= 1e-6, "{side}: strays {off} from the input");
        assert!(peak(got) > 0.68, "{side}: heard at the knee");
    }
}

/// A source 8 dB over full scale (peaks near 2.5), dry alone: every sample is the limit of
/// the input's, with no delay, and nothing passes the ceiling.
#[test]
fn a_hot_input_follows_the_soft_limit_without_latency() {
    let source = Source::noise_at(8.0);
    let (want_l, want_r) = render_doc(&dry(&source), SECOND);
    let config = json!({ "wet": 0.0, "reverb": 0.0 });
    let (l, r) = render_doc(&effect(&source, config), SECOND);
    assert_limited("left", &l, &want_l);
    assert_limited("right", &r, &want_r);
    assert!(peak(&l).max(peak(&r)) < 1.0, "under the ceiling");
}

#[test]
fn dry_and_reverb_at_zero_are_the_bare_granular_module_limited() {
    let source = Source::sine(220.0, 0.5);
    for mode in ["granular", "stretch", "looping_delay"] {
        // Every grain control away from its default and from the others, so
        // a crossed alias shows.
        let grains = json!({
            "mode": mode, "position": 0.3, "size": 0.62, "pitch": 7.0,
            "density": 0.85, "texture": 0.4, "spread": 0.6
        });
        let (want_l, want_r) = render_doc(&bare(&source, grains.clone()), 3 * SECOND);
        let mut config = grains;
        config["dry"] = json!(0.0);
        config["wet"] = json!(1.0);
        config["reverb"] = json!(0.0);
        let (l, r) = render_doc(&effect(&source, config), 3 * SECOND);
        assert_limited(&format!("{mode} left"), &l, &want_l);
        assert_limited(&format!("{mode} right"), &r, &want_r);
    }
}

/// A 100 ms burst read at the newest audio: without the reverb the grains
/// stop with it; the reverb return rings on, and is heard alone at `wet` 0.
#[test]
fn the_reverb_send_adds_a_tail_after_the_grains_stop() {
    let tail = |wet: f64, reverb: f64| {
        let config = json!({ "dry": 0.0, "wet": wet, "reverb": reverb, "position": 0.0 });
        let (l, _) = render_doc(&effect(&Source::burst(4_800), config), 2 * SECOND);
        rms(&l[SECOND / 2..SECOND])
    };
    assert_eq!(tail(1.0, 0.0), 0.0, "the grains stop with the burst");
    assert!(tail(1.0, 0.5) > 0.02, "the reverb rings on");
    assert!(tail(0.0, 0.5) > 0.02, "the return is heard alone");
    assert_eq!(tail(0.0, 0.0), 0.0, "nothing at wet and reverb 0");
}

/// The larger of the two sides' largest steps.
fn both((left, right): (Vec<f32>, Vec<f32>)) -> f32 {
    max_step(&left).max(max_step(&right))
}

/// Each mix control jumped live from 0 to 1 and back glides: no step larger
/// than 1.5 times the steady signal's on either side of the change. The
/// sine's period does not divide the write times, so a step in `dry` would
/// not hide at a zero crossing.
#[test]
fn mix_controls_glide_when_changed_live() {
    for key in ["dry", "wet", "reverb"] {
        let mut fx = engine(&effect(&Source::sine(233.0, 0.25), json!({ key: 0.0 })));
        render(&mut fx, 3 * SECOND);
        let mut before = both(render(&mut fx, SECOND / 2));
        for to in [1.0f32, 0.0] {
            set(&fx, key, to);
            let change = render(&mut fx, SECOND / 10);
            render(&mut fx, SECOND);
            let after = both(render(&mut fx, SECOND / 2));
            let step = both(change);
            let bound = 1.5 * before.max(after);
            assert!(step <= bound, "{key} -> {to}: step {step} over {bound}");
            before = after;
        }
    }
}
