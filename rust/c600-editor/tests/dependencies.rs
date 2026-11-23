//! EV1.9 (part: a linked service runs no user program; LP4-1's acceptance): the editor crate's dependency closure,
//! read from the manifests, holds the syntax, checker, IR and digest crates only, so no service has a path to the
//! interpreter (`c600-run`) or the command layer (`c600-command`).

use std::collections::BTreeSet;
use std::path::Path;

/// The entries of a manifest's dependency and build-dependency tables, its target tables among them: each name, and
/// the directory of a crate of this workspace that it names by its relative path, or `None` for a crate from the
/// registry. A dependency's own table section, such as `[dependencies.name]`, fails the test, since this reader does
/// not follow one.
fn dependencies(manifest: &str) -> Vec<(String, Option<String>)> {
    let mut entries = Vec::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            assert!(!line.contains("dependencies."), "a dependency's own table section, not followed here: {line}");
            let target = line.starts_with("[target.")
                && (line.ends_with(".dependencies]") || line.ends_with(".build-dependencies]"));
            inside = line == "[dependencies]" || line == "[build-dependencies]" || target;
            continue;
        }
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line.split_once('=').expect("a dependency line");
        let directory = value.split_once("path = \"../").and_then(|(_, rest)| rest.split_once('"'));
        entries.push((name.trim().to_owned(), directory.map(|(d, _)| d.to_owned())));
    }
    entries
}

#[test]
fn the_editor_services_have_no_path_to_the_interpreter_or_the_command_layer() {
    let rust = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut closure = BTreeSet::new();
    let mut pending = vec!["c600-editor".to_owned()];
    while let Some(name) = pending.pop() {
        let manifest = std::fs::read_to_string(rust.join(&name).join("Cargo.toml")).unwrap();
        for (dependency, directory) in dependencies(&manifest) {
            let Some(directory) = directory else {
                assert!(!dependency.starts_with("c600"), "{name} takes {dependency} from the registry");
                continue;
            };
            assert_eq!(directory, dependency, "{name}'s manifest");
            if closure.insert(dependency.clone()) {
                pending.push(dependency);
            }
        }
    }
    let allowed: BTreeSet<String> = ["c600-check", "c600-digest", "c600-ir", "c600-syntax"].map(String::from).into();
    assert_eq!(closure, allowed, "the editor services' path dependencies changed (EV1.9)");
    for forbidden in ["c600-run", "c600-command", "c600-runtime", "c600-tools", "c600-module", "c600-session"] {
        assert!(!closure.contains(forbidden), "{forbidden}");
    }
}
