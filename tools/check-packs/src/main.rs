use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

use fugue::pkg::{self, EntrySpec, PackageManifest};
use serde_json::Value;

fn main() {
    let mut args = env::args_os().skip(1);
    let root = args.next().map(PathBuf::from).unwrap_or_else(|| ".".into());
    if args.next().is_some() {
        eprintln!("usage: check-packs [repository-root]");
        std::process::exit(2);
    }

    match check_repository(&root) {
        Ok(count) => println!("validated {count} pack manifests"),
        Err(error) => {
            eprintln!("pack check failed: {error}");
            std::process::exit(1);
        }
    }
}

fn check_repository(root: &Path) -> Result<usize, String> {
    check_index(&root.join("index.json"))?;

    let packs_dir = root.join("packs");
    let mut directories = fs::read_dir(&packs_dir)
        .map_err(|error| format!("cannot read {}: {error}", packs_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate {}: {error}", packs_dir.display()))?;
    directories.sort_by_key(|entry| entry.file_name());

    let mut ids = HashSet::new();
    let mut count = 0;
    for directory in directories {
        let file_type = directory
            .file_type()
            .map_err(|error| format!("cannot inspect {}: {error}", directory.path().display()))?;
        if !file_type.is_dir() {
            return Err(format!(
                "{} must contain only package directories",
                packs_dir.display()
            ));
        }

        let pack_root = directory.path();
        let manifest_path = pack_root.join("fugue.pkg.json");
        let manifest = pkg::parse_path(&manifest_path)
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
        let directory_name = directory.file_name();
        let directory_name = directory_name.to_string_lossy();
        if manifest.id != directory_name {
            return Err(format!(
                "{}: manifest id `{}` must match directory name `{directory_name}`",
                manifest_path.display(),
                manifest.id
            ));
        }
        if !ids.insert(manifest.id.clone()) {
            return Err(format!("duplicate package id `{}`", manifest.id));
        }

        check_entry(&pack_root, &manifest)?;
        count += 1;
    }

    if count == 0 {
        return Err("packs directory contains no packages".to_string());
    }
    Ok(count)
}

fn check_index(path: &Path) -> Result<(), String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let index: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    if !index
        .get("packages")
        .is_some_and(serde_json::Value::is_array)
    {
        return Err(format!(
            "{} must contain a top-level `packages` array",
            path.display()
        ));
    }
    Ok(())
}

fn check_entry(pack_root: &Path, manifest: &PackageManifest) -> Result<(), String> {
    let entry = match &manifest.entry {
        EntrySpec::Module { wasm } => wasm,
        EntrySpec::Invention { invention } => invention,
        EntrySpec::Development { development } => development,
        EntrySpec::Skill { skill } => skill,
        EntrySpec::Agent { definition } => definition,
        EntrySpec::SamplePack { samples } => samples,
    };
    let relative = Path::new(entry);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{}: entry `{entry}` must be a relative path inside the package",
            manifest.id
        ));
    }

    let entry_path = pack_root.join(relative);
    let canonical_root = pack_root
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", pack_root.display()))?;
    let canonical_entry = entry_path.canonicalize().map_err(|error| {
        format!(
            "{}: entry `{}` does not exist or cannot be read: {error}",
            manifest.id,
            entry_path.display()
        )
    })?;
    if !canonical_entry.starts_with(&canonical_root) || !canonical_entry.is_file() {
        return Err(format!(
            "{}: entry `{}` must be a file inside the package",
            manifest.id,
            entry_path.display()
        ));
    }
    if canonical_entry
        .extension()
        .and_then(|extension| extension.to_str())
        == Some("json")
    {
        let bytes = fs::read(&canonical_entry)
            .map_err(|error| format!("cannot read {}: {error}", entry_path.display()))?;
        serde_json::from_slice::<Value>(&bytes).map_err(|error| {
            format!(
                "{}: JSON entry `{}` is invalid: {error}",
                manifest.id,
                entry_path.display()
            )
        })?;
    }
    if matches!(manifest.entry, EntrySpec::Skill { .. })
        && canonical_entry.file_name().and_then(|name| name.to_str()) != Some("SKILL.md")
    {
        return Err(format!(
            "{}: skill entry must be named SKILL.md",
            manifest.id
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn repository() -> TempDir {
        let root = tempfile::tempdir().expect("temp repository");
        fs::create_dir(root.path().join("packs")).expect("packs directory");
        fs::write(root.path().join("index.json"), r#"{"packages":[]}"#).expect("registry index");
        root
    }

    fn write_skill(root: &Path, directory: &str, id: &str, entry: &str) {
        let pack = root.join("packs").join(directory);
        fs::create_dir_all(&pack).expect("pack directory");
        let manifest = json!({
            "id": id,
            "version": "0.1.0",
            "kind": "skill",
            "license": "CC0-1.0",
            "authors": [{"name": "Test Author"}],
            "targets": ["external-agent"],
            "entry": {"skill": entry}
        });
        fs::write(
            pack.join("fugue.pkg.json"),
            serde_json::to_vec_pretty(&manifest).expect("manifest JSON"),
        )
        .expect("manifest");
        fs::write(pack.join(entry), "# Test skill\n").expect("entry");
    }

    #[test]
    fn accepts_valid_repository() {
        let root = repository();
        write_skill(
            root.path(),
            "fugue.test.skill",
            "fugue.test.skill",
            "SKILL.md",
        );
        assert_eq!(check_repository(root.path()).unwrap(), 1);
    }

    #[test]
    fn rejects_manifest_id_that_differs_from_directory() {
        let root = repository();
        write_skill(
            root.path(),
            "fugue.test.directory",
            "fugue.test.other",
            "SKILL.md",
        );
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("must match directory name"));
    }

    #[test]
    fn rejects_manifest_that_fugue_considers_invalid() {
        let root = repository();
        write_skill(
            root.path(),
            "fugue.test.skill",
            "fugue.test.skill",
            "SKILL.md",
        );
        let manifest = root.path().join("packs/fugue.test.skill/fugue.pkg.json");
        let invalid = fs::read_to_string(&manifest)
            .unwrap()
            .replace("\"0.1.0\"", "\"not-semver\"");
        fs::write(manifest, invalid).unwrap();
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("invalid version"));
    }

    #[test]
    fn rejects_missing_entry() {
        let root = repository();
        write_skill(
            root.path(),
            "fugue.test.skill",
            "fugue.test.skill",
            "SKILL.md",
        );
        fs::remove_file(root.path().join("packs/fugue.test.skill/SKILL.md")).unwrap();
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("does not exist"));
    }

    #[test]
    fn rejects_entry_that_escapes_package() {
        let root = repository();
        write_skill(
            root.path(),
            "fugue.test.skill",
            "fugue.test.skill",
            "SKILL.md",
        );
        let manifest = root.path().join("packs/fugue.test.skill/fugue.pkg.json");
        let escaped = fs::read_to_string(&manifest)
            .unwrap()
            .replace("SKILL.md", "../outside.md");
        fs::write(manifest, escaped).unwrap();
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("relative path inside the package"));
    }

    #[test]
    fn rejects_skill_entry_with_wrong_name() {
        let root = repository();
        write_skill(
            root.path(),
            "fugue.test.skill",
            "fugue.test.skill",
            "skill.md",
        );
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("must be named SKILL.md"));
    }

    #[test]
    fn rejects_registry_without_packages_array() {
        let root = repository();
        fs::write(root.path().join("index.json"), r#"{"packages":{}}"#).unwrap();
        write_skill(
            root.path(),
            "fugue.test.skill",
            "fugue.test.skill",
            "SKILL.md",
        );
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("top-level `packages` array"));
    }

    #[test]
    fn rejects_invalid_json_entry() {
        let root = repository();
        let pack = root.path().join("packs/fugue.test.agent");
        fs::create_dir_all(&pack).unwrap();
        fs::write(
            pack.join("fugue.pkg.json"),
            serde_json::to_vec_pretty(&json!({
                "id": "fugue.test.agent",
                "version": "0.1.0",
                "kind": "agent",
                "license": "CC0-1.0",
                "authors": [{"name": "Test Author"}],
                "targets": ["in-graph-agent"],
                "entry": {"definition": "agent.json"}
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(pack.join("agent.json"), "{not-json}").unwrap();
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("JSON entry"));
    }
}
