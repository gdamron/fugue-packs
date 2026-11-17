//! Names from the document and manifest formats that a skill may show in
//! code spans, taken from the pinned core rather than listed by hand.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fugue::pkg::parse_path;
use fugue::Invention;
use serde_json::{json, Value};

/// Every field name an invention or development document carries, as the
/// core writes one that uses each kind of development import and every
/// part of a development's interface, and every field of a package manifest
/// as the core writes the manifest at `manifest`.
pub fn format_fields(manifest: &Path) -> BTreeSet<String> {
    let inner = json!({
        "modules": [{ "id": "amp", "type": "vca" }],
        "connections": [],
        "inputs": [{ "name": "audio", "to": "amp", "to_port": "audio" }],
        "outputs": [{ "name": "audio", "from": "amp", "from_port": "audio" }],
        "controls": [{
            "name": "level", "module": "amp", "control": "level",
            "range": { "min": 0.0, "max": 1.0 }
        }]
    });
    let document = json!({
        "developments": [
            { "name": "by_path", "path": "voice.json" },
            { "name": "inline", "definition": inner },
            { "name": "by_ref", "ref": { "package": "an.id", "version": "1.0.0" } }
        ],
        "modules": [{ "id": "dac", "type": "dac" }],
        "connections": [{ "from": "a", "from_port": "b", "to": "c", "to_port": "d" }]
    });
    let invention = Invention::from_json(&document.to_string()).expect("the document parses");
    let mut fields = BTreeSet::new();
    keys(&serde_json::to_value(&invention).unwrap(), &mut fields);

    let manifest = parse_path(manifest).expect("the manifest validates");
    keys(&serde_json::to_value(&manifest).unwrap(), &mut fields);
    fields
}

/// The names of a development document's exposed inputs, outputs and
/// controls.
pub fn interface_names(document: &Path) -> BTreeSet<String> {
    let text = fs::read_to_string(document).unwrap();
    let invention = Invention::from_json(&text)
        .unwrap_or_else(|e| panic!("{} should parse: {e}", document.display()));
    assert!(invention.is_development(), "{}", document.display());
    let inputs = invention.inputs.iter().map(|i| i.name.clone());
    let outputs = invention.outputs.iter().map(|o| o.name.clone());
    let controls = invention.controls.iter().map(|c| c.name.clone());
    inputs.chain(outputs).chain(controls).collect()
}

fn keys(value: &Value, into: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                into.insert(key.clone());
                keys(value, into);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys(item, into)),
        _ => {}
    }
}
