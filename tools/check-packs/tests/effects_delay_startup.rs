//! A delay pack built with its dry and wet levels at zero is silent from its
//! first sample: starting values land before audio starts, not by a glide.

use fugue::RenderEngine;
use serde_json::{json, Value};

fn conn(from: &str, from_port: &str, to: &str, to_port: &str) -> Value {
    json!({ "from": from, "from_port": from_port, "to": to, "to_port": to_port })
}

fn render_muted(id: &str) -> Vec<f32> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs")
        .join(id)
        .join("delay.development.json");
    let doc = json!({
        "version": "1.0.0",
        "developments": [{ "name": "delay_fx", "path": path.to_string_lossy() }],
        "modules": [
            { "id": "osc", "type": "oscillator", "config": { "waveform": "sine", "frequency": 440.0 } },
            { "id": "fx", "type": "delay_fx", "config": { "dry": 0.0, "wet": 0.0 } },
            { "id": "dac", "type": "dac", "config": { "soft_clip": false } }
        ],
        "connections": [
            conn("osc", "audio", "fx", "audio_left"),
            conn("osc", "audio", "fx", "audio_right"),
            conn("fx", "audio_left", "dac", "audio_left"),
            conn("fx", "audio_right", "dac", "audio_right")
        ]
    });
    let mut engine = RenderEngine::new(48_000);
    engine.load_json(&doc.to_string()).expect("invention loads");
    let mut out = vec![0.0f32; 2 * 4_800];
    engine.render_interleaved(&mut out).expect("renders");
    out
}

#[test]
fn a_pack_built_muted_is_silent_from_its_first_sample() {
    for id in [
        "fugue.effects.stereo-delay",
        "fugue.effects.ping-pong-delay",
    ] {
        let out = render_muted(id);
        let loudest = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert_eq!(loudest, 0.0, "{id} leaks {loudest} in its first 0.1 s");
    }
}
