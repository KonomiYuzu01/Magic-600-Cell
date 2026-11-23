//! Compile never runs a script, and it cannot: `c600-log` has no path to `c600-run`, the interpreter, nor to the
//! command layer or the session store that hold one (ADR-D01.81, LP4-12). This test follows the manifests from
//! `c600-log` through every normal and build dependency and fails on a path to those crates or on any dependency
//! that is not a workspace path (no external crate).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const FORBIDDEN: [&str; 3] = ["c600-run", "c600-command", "c600-session"];

/// The path dependencies of a manifest's `[dependencies]`, `[build-dependencies]` and target dependency tables, by
/// name; a dependency that is not a path is returned with `None`.
fn dependencies(manifest: &Path) -> Vec<(String, Option<PathBuf>)> {
    let text = std::fs::read_to_string(manifest).unwrap_or_else(|e| panic!("{}: {e}", manifest.display()));
    let mut out = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            section = line.trim_matches(|c| c == '[' || c == ']').to_owned();
            assert!(
                !section.starts_with("dependencies.") && !section.contains("dependencies."),
                "{}: a dependency table [{section}] this test does not read",
                manifest.display()
            );
            continue;
        }
        let runtime = section == "dependencies"
            || section == "build-dependencies"
            || (section.starts_with("target.") && !section.ends_with("dev-dependencies"));
        if !runtime || !section.ends_with("dependencies") {
            continue;
        }
        let (name, spec) = line.split_once('=').unwrap_or_else(|| panic!("{}: {line}", manifest.display()));
        let name = name.trim().trim_end_matches(".workspace").to_owned();
        let path = spec.split_once("path").and_then(|(_, rest)| {
            let rest = rest.trim_start().strip_prefix('=')?.trim_start().strip_prefix('"')?;
            rest.split_once('"').map(|(path, _)| manifest.parent().unwrap().join(path))
        });
        out.push((name, path));
    }
    out
}

#[test]
fn c600_log_has_no_path_to_the_interpreter_the_command_layer_or_the_session_store() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut pending = vec![("c600-log".to_owned(), root.join("Cargo.toml"), vec!["c600-log".to_owned()])];
    let mut external = BTreeSet::new();
    while let Some((name, manifest, chain)) = pending.pop() {
        if seen.insert(name.clone(), manifest.clone()).is_some() {
            continue;
        }
        for (dependency, path) in dependencies(&manifest) {
            let mut next = chain.clone();
            next.push(dependency.clone());
            assert!(!FORBIDDEN.contains(&dependency.as_str()), "a path to {dependency}: {}", next.join(" -> "));
            match path {
                Some(path) => pending.push((dependency, path.join("Cargo.toml"), next)),
                None => {
                    external.insert(next.join(" -> "));
                }
            }
        }
    }
    assert!(external.is_empty(), "dependencies that are not workspace paths: {external:?}");
    // The closure is what the crate's design says: the kernel replays, the editor services compile scripts.
    for needed in ["c600-kernel", "c600-editor", "c600-digest"] {
        assert!(seen.contains_key(needed), "c600-log does not reach {needed}: {seen:?}");
    }
    for name in seen.keys() {
        assert!(!FORBIDDEN.contains(&name.as_str()), "{name} is reachable");
    }
}
