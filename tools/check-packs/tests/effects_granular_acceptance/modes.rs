//! Every mode plays.

use serde_json::json;

use super::rig::*;

/// "with a steady sine (e.g. 220 Hz at 0.5) in both inputs, `dry` 0, `wet`
/// 1, `reverb` 0, each `mode` at otherwise default controls gives output RMS
/// above 0.05 on both sides between 2.5 s and 3 s (the default `position`
/// 0.5 reads 2 s back in the 4 s buffer)."
#[test]
fn each_mode_is_heard_on_both_sides_at_default_controls() {
    let feed = Feed::sine(220.0, 0.5);
    for mode in ["granular", "stretch", "looping_delay"] {
        let config = json!({ "mode": mode, "dry": 0.0, "wet": 1.0, "reverb": 0.0 });
        let (left, right) = render_doc(&feed.effect(config, 1.0), 3 * SR as usize);
        let window = 5 * SR as usize / 2..;
        for (side, x) in [("left", &left), ("right", &right)] {
            let level = rms(&x[window.clone()]);
            assert!(level > 0.05, "{mode} {side}: RMS {level} from 2.5 s to 3 s");
        }
    }
}
