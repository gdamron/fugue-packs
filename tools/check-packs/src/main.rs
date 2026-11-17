use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use fugue::pkg::{self, sample_pack, EntrySpec, PackageManifest, PackageProvider};
use semver::Version;
use serde_json::Value;

fn main() {
    let mut args = env::args_os().skip(1);
    let root = args.next().map(PathBuf::from).unwrap_or_else(|| ".".into());
    if args.next().is_some() {
        eprintln!("usage: check-packs [repository-root]");
        std::process::exit(2);
    }

    match check_repository(&root) {
        Ok(checked) => {
            for warning in &checked.warnings {
                eprintln!("warning: {warning}");
            }
            println!("validated {} pack manifests", checked.count);
        }
        Err(error) => {
            eprintln!("pack check failed: {error}");
            std::process::exit(1);
        }
    }
}

/// A repository that passed: how many packs it holds, and problems worth
/// fixing that do not fail the check.
#[derive(Debug)]
struct Checked {
    count: usize,
    warnings: Vec<String>,
}

fn check_repository(root: &Path) -> Result<Checked, String> {
    check_index(&root.join("index.json"))?;

    let packs_dir = root.join("packs");
    let mut directories = fs::read_dir(&packs_dir)
        .map_err(|error| format!("cannot read {}: {error}", packs_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate {}: {error}", packs_dir.display()))?;
    directories.sort_by_key(|entry| entry.file_name());

    let mut packs = RepositoryPacks::default();
    let mut warnings = Vec::new();
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
        warnings.extend(check_entry(&pack_root, &manifest)?);
        if packs.0.insert(manifest.id.clone(), manifest).is_some() {
            return Err(format!("duplicate package id `{directory_name}`"));
        }
    }

    if packs.0.is_empty() {
        return Err("packs directory contains no packages".to_string());
    }
    for manifest in packs.0.values() {
        pkg::resolve_package(manifest, &packs).map_err(|error| {
            format!("{}: cannot resolve its requirements: {error}", manifest.id)
        })?;
    }
    Ok(Checked {
        count: packs.0.len(),
        warnings,
    })
}

/// The repository's own packs, keyed by id, as a resolution source: every
/// `deps` and `policies` requirement must resolve within the repository.
#[derive(Default)]
struct RepositoryPacks(BTreeMap<String, PackageManifest>);

impl PackageProvider for RepositoryPacks {
    type Acquired = ();

    fn available_versions(&self, id: &str) -> Result<Vec<Version>, Box<dyn Error>> {
        self.0
            .get(id)
            .map(|manifest| Version::parse(&manifest.version).map(|version| vec![version]))
            .transpose()
            .map(Option::unwrap_or_default)
            .map_err(Into::into)
    }

    fn acquire(
        &self,
        id: &str,
        _version: &Version,
    ) -> Result<(PackageManifest, ()), Box<dyn Error>> {
        let manifest = self.0.get(id).ok_or_else(|| format!("no pack `{id}`"))?;
        Ok((manifest.clone(), ()))
    }
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

/// Check a pack's entry, returning warnings that do not fail the check.
fn check_entry(pack_root: &Path, manifest: &PackageManifest) -> Result<Vec<String>, String> {
    let entry = manifest.entry.path();
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
    if matches!(manifest.entry, EntrySpec::SamplePack { .. }) {
        check_sample_pack_entry(pack_root, &canonical_root, &canonical_entry, manifest)?;
    }
    if matches!(manifest.entry, EntrySpec::Policy { .. }) {
        pkg::check_policy_entry(pack_root, manifest)
            .map_err(|error| format!("{}: {error}", manifest.id))?;
    }
    if matches!(manifest.entry, EntrySpec::Skill { .. }) {
        let checked = pkg::check_skill_package(pack_root, manifest)
            .map_err(|error| format!("{}: {error}", manifest.id))?;
        return Ok(checked
            .warnings
            .iter()
            .map(|warning| format!("{}: {warning}", manifest.id))
            .collect());
    }
    Ok(Vec::new())
}

fn check_sample_pack_entry(
    pack_root: &Path,
    canonical_root: &Path,
    entry_path: &Path,
    manifest: &PackageManifest,
) -> Result<(), String> {
    let samples = sample_pack::parse_path(entry_path)
        .map_err(|error| format!("{}: {}: {error}", manifest.id, entry_path.display()))?;
    for file in &samples.files {
        let file_path = pack_root.join(&file.path);
        let canonical_file = file_path.canonicalize().map_err(|error| {
            format!(
                "{}: sample `{}` does not exist or cannot be read: {error}",
                manifest.id, file.path
            )
        })?;
        if !canonical_file.starts_with(canonical_root) || !canonical_file.is_file() {
            return Err(format!(
                "{}: sample `{}` must be a file inside the package",
                manifest.id, file.path
            ));
        }
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
        write_skill_requiring(root, directory, id, entry, None);
    }

    fn write_skill_requiring(
        root: &Path,
        directory: &str,
        id: &str,
        entry: &str,
        policies: Option<&[&str]>,
    ) {
        let pack = root.join("packs").join(directory);
        fs::create_dir_all(&pack).expect("pack directory");
        let mut manifest = json!({
            "id": id,
            "version": "0.1.0",
            "kind": "skill",
            "license": "CC0-1.0",
            "authors": [{"name": "Test Author"}],
            "targets": ["external-agent"],
            "entry": {"skill": entry}
        });
        if let Some(policies) = policies {
            manifest["policies"] = json!(policies);
        }
        fs::write(
            pack.join("fugue.pkg.json"),
            serde_json::to_vec_pretty(&manifest).expect("manifest JSON"),
        )
        .expect("manifest");
        fs::write(
            pack.join(entry),
            "---\nname: test-skill\ndescription: A test skill.\n---\n# Test skill\n",
        )
        .expect("entry");
    }

    fn write_policy(root: &Path, id: &str, entry: &str, text: &str) {
        let pack = root.join("packs").join(id);
        fs::create_dir_all(&pack).expect("pack directory");
        let manifest = json!({
            "id": id,
            "version": "0.1.0",
            "kind": "policy",
            "license": "CC0-1.0",
            "authors": [{"name": "Test Author"}],
            "targets": ["external-agent"],
            "entry": {"policy": entry}
        });
        fs::write(
            pack.join("fugue.pkg.json"),
            serde_json::to_vec_pretty(&manifest).expect("manifest JSON"),
        )
        .expect("manifest");
        fs::write(pack.join(entry), text).expect("entry");
    }

    #[test]
    fn accepts_a_skill_requiring_a_policy_in_the_repository() {
        let root = repository();
        write_policy(root.path(), "fugue.test.rules", "POLICY.md", "# Rules\n");
        write_skill_requiring(
            root.path(),
            "fugue.test.skill",
            "fugue.test.skill",
            "SKILL.md",
            Some(&["fugue.test.rules@^0.1.0"]),
        );
        assert_eq!(check_repository(root.path()).unwrap().count, 2);
    }

    #[test]
    fn rejects_a_policy_requirement_the_repository_cannot_resolve() {
        let root = repository();
        write_skill_requiring(
            root.path(),
            "fugue.test.skill",
            "fugue.test.skill",
            "SKILL.md",
            Some(&["fugue.test.rules@^0.1.0"]),
        );
        let error = check_repository(root.path()).unwrap_err();
        assert!(error.contains("fugue.test.rules"), "{error}");
    }

    #[test]
    fn rejects_a_policy_entry_with_frontmatter() {
        let root = repository();
        write_policy(
            root.path(),
            "fugue.test.rules",
            "POLICY.md",
            "---\nname: rules\n---\n# Rules\n",
        );
        let error = check_repository(root.path()).unwrap_err();
        assert!(error.contains("frontmatter"), "{error}");
    }

    #[test]
    fn the_pulse_demos_lock_each_shared_policy_once() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        check_repository(&root).expect("repository packs validate");
        let policies = ["fugue.policy.clean-audio", "fugue.policy.musical-intent"];
        let requirers = ["fugue.demo.pulse-skill", "fugue.demo.pulse-agent"];
        let mut packs = RepositoryPacks::default();
        for id in requirers.iter().chain(&policies) {
            let manifest = pkg::parse_path(root.join("packs").join(id).join("fugue.pkg.json"))
                .expect("pulse manifest");
            packs.0.insert(id.to_string(), manifest);
        }
        let resolved = pkg::resolve_transitive(&requirers.map(|id| format!("{id}@^0.1.0")), &packs)
            .expect("pulse demos resolve");
        for policy in policies {
            let locked: Vec<_> = resolved
                .iter()
                .filter(|package| package.id == policy)
                .collect();
            assert_eq!(locked.len(), 1, "{policy} is locked once");
            assert_eq!(locked[0].version, "2026.11.0");
        }
        for requirer in requirers {
            let package = resolved
                .iter()
                .find(|package| package.id == requirer)
                .unwrap();
            assert_eq!(
                package.dependencies,
                [
                    "fugue.policy.clean-audio@2026.11.0",
                    "fugue.policy.musical-intent@2026.11.0"
                ],
                "{requirer} requires the real policies"
            );
        }
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
        assert_eq!(check_repository(root.path()).unwrap().count, 1);
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
            .contains("must be a SKILL.md file"));
    }

    fn write_sample_pack(root: &Path, id: &str, samples_json: &str) {
        let pack = root.join("packs").join(id);
        fs::create_dir_all(pack.join("samples")).expect("pack directories");
        let manifest = json!({
            "id": id,
            "version": "0.1.0",
            "kind": "sample-pack",
            "license": "CC0-1.0",
            "authors": [{"name": "Test Author"}],
            "targets": ["external-agent"],
            "requires": {"capabilities": ["fs:read:samples/"]},
            "entry": {"samples": "samples.json"}
        });
        fs::write(
            pack.join("fugue.pkg.json"),
            serde_json::to_vec_pretty(&manifest).expect("manifest JSON"),
        )
        .expect("manifest");
        fs::write(pack.join("samples.json"), samples_json).expect("entry");
        fs::write(pack.join("samples/pulse.wav"), b"RIFF").expect("sample file");
    }

    const VALID_SAMPLES: &str = r#"{
        "license": "CC0-1.0",
        "sample_rate": [48000],
        "files": [{ "path": "samples/pulse.wav" }]
    }"#;

    #[test]
    fn accepts_valid_sample_pack() {
        let root = repository();
        write_sample_pack(root.path(), "fugue.test.samples", VALID_SAMPLES);
        assert_eq!(check_repository(root.path()).unwrap().count, 1);
    }

    #[test]
    fn rejects_sample_pack_entry_failing_schema() {
        let root = repository();
        let invalid = VALID_SAMPLES.replace("[48000]", "[]");
        write_sample_pack(root.path(), "fugue.test.samples", &invalid);
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("sample_rate must declare at least one rate"));
    }

    #[test]
    fn rejects_sample_pack_with_missing_listed_file() {
        let root = repository();
        write_sample_pack(root.path(), "fugue.test.samples", VALID_SAMPLES);
        fs::remove_file(
            root.path()
                .join("packs/fugue.test.samples/samples/pulse.wav"),
        )
        .unwrap();
        assert!(check_repository(root.path())
            .unwrap_err()
            .contains("does not exist"));
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
