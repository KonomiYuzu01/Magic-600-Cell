//! The product-build check of option (a) (top integrator, 18 November 2026): the injection device `c600-lx2-child`,
//! its ceiling allocator and its use of c600-runtime's fault hook stay out of the product. No package of the language
//! workspace or of `renderer/bridge` other than c600-lx2 declares a dependency on c600-lx2, so no product binary can
//! link the device; nothing under c600-runtime's source names a global allocator, so the product's runtime child runs
//! on the system allocator; c600-runtime's fault hook (its feature `lx2-faults`) is turned on by no dependency and by
//! no feature but c600-lx2's `device`, which no default turns on, and the runtime child built as the product builds it
//! has the hook off; and the unsafe code of the device stays in its bin. Source evidence, and one build of the
//! runtime child.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use c600_kernel::json;

const PACKAGE: &str = "c600-lx2";
const RUNTIME: &str = "c600-runtime";
const HOOK: &str = "lx2-faults";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// A workspace package as `cargo metadata --no-deps` lists it.
struct Package {
    name: String,
    /// Each declared dependency, of every kind, with the features it turns on.
    dependencies: Vec<(String, Vec<String>)>,
    /// Each feature with what it turns on.
    features: Vec<(String, Vec<String>)>,
}

/// The strings of a JSON array.
fn strings(value: Option<&json::Value>) -> Vec<String> {
    let listed = value.and_then(|v| v.as_array()).expect("an array of strings");
    listed.iter().map(|s| s.as_str().expect("a string").to_owned()).collect()
}

/// `cargo metadata --no-deps` of `manifest`: each workspace package.
fn packages(manifest: &Path) -> Vec<Package> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--offline", "--locked", "--manifest-path"])
        .arg(manifest)
        .output()
        .expect("cargo starts");
    assert!(out.status.success(), "cargo metadata of {}: {}", manifest.display(), String::from_utf8_lossy(&out.stderr));
    let metadata = json::parse(&out.stdout).expect("cargo metadata prints JSON");
    let listed = metadata.get("packages").and_then(|p| p.as_array()).expect("cargo metadata lists packages");
    let name = |value: &json::Value| value.get("name").and_then(|n| n.as_str()).expect("a name").to_owned();
    listed
        .iter()
        .map(|package| {
            let dependencies = package
                .get("dependencies")
                .and_then(|d| d.as_array())
                .expect("a package lists its dependencies")
                .iter()
                .map(|d| (name(d), strings(d.get("features"))))
                .collect();
            let features = package
                .get("features")
                .and_then(|f| f.as_object())
                .expect("a package lists its features")
                .iter()
                .map(|(feature, enables)| (feature.to_string(), strings(Some(enables))))
                .collect();
            Package { name: name(package), dependencies, features }
        })
        .collect()
}

/// Both workspaces' manifests: the language workspace and the shell's bridge.
fn manifests() -> [PathBuf; 2] {
    let root = repository();
    [root.join("rust").join("Cargo.toml"), root.join("renderer").join("bridge").join("Cargo.toml")]
}

#[test]
fn no_package_but_this_one_depends_on_the_device_package() {
    for manifest in manifests() {
        let listed = packages(&manifest);
        assert!(!listed.is_empty(), "{} lists no package", manifest.display());
        let dependents: Vec<&str> = listed
            .iter()
            .filter(|p| p.name != PACKAGE && p.dependencies.iter().any(|(d, _)| d == PACKAGE))
            .map(|p| p.name.as_str())
            .collect();
        assert!(dependents.is_empty(), "{} has packages that depend on {PACKAGE}: {dependents:?}", manifest.display());
    }
    let workspace = packages(&manifests()[0]);
    assert!(workspace.iter().any(|p| p.name == PACKAGE), "the check runs over the workspace that holds {PACKAGE}");
    assert!(workspace.iter().any(|p| p.name == RUNTIME), "and over the product's runtime");
}

#[test]
fn the_hook_reaches_the_runtime_only_through_this_packages_device_feature_which_no_default_turns_on() {
    let hook = format!("{RUNTIME}/{HOOK}");
    let weak = format!("{RUNTIME}?/{HOOK}");
    let mut routes = Vec::new();
    for manifest in manifests() {
        for package in packages(&manifest) {
            for (dependency, features) in &package.dependencies {
                assert!(
                    !(dependency == RUNTIME && features.iter().any(|f| f == HOOK)),
                    "{} turns on {RUNTIME}'s {HOOK} in its dependency",
                    package.name
                );
            }
            for (feature, enables) in &package.features {
                if enables.iter().any(|e| *e == hook || *e == weak) {
                    routes.push(format!("{}/{feature}", package.name));
                }
                if feature == "default" {
                    assert!(
                        !enables.iter().any(|e| *e == hook || *e == weak || e == HOOK || e == "device"),
                        "{}'s default features turn the hook on: {enables:?}",
                        package.name
                    );
                }
            }
        }
    }
    assert_eq!(routes, [format!("{PACKAGE}/device")], "the one feature that turns the hook on");
}

#[test]
fn the_runtime_child_built_as_the_product_builds_it_has_no_hook() {
    let features = common::runtime_features();
    assert!(!features.iter().any(|f| f == HOOK), "the runtime child was built with {features:?}");
}

/// Every `.rs` file under `folder`, recursively, with its text.
fn sources(folder: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).unwrap_or_else(|e| panic!("{} reads: {e}", next.display())) {
            let path = entry.expect("a folder entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()));
                found.push((path, text));
            }
        }
    }
    found
}

#[test]
fn the_products_runtime_names_no_global_allocator_and_keeps_unsafe_code_out() {
    let runtime = sources(&repository().join("rust").join(RUNTIME).join("src"));
    assert!(runtime.iter().any(|(path, _)| path.ends_with("lib.rs")), "c600-runtime's source is read");
    for (path, text) in &runtime {
        assert!(!text.contains("global_allocator"), "{} names a global allocator", path.display());
    }
    for file in ["lib.rs", "main.rs"] {
        let (_, text) = runtime.iter().find(|(path, _)| path.ends_with(file)).expect("c600-runtime's crate roots");
        assert!(text.contains("#![forbid(unsafe_code)]"), "c600-runtime's {file} forbids unsafe code");
    }
}

#[test]
fn the_device_alone_holds_the_allocator_the_hook_and_the_unsafe_code_of_this_package() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let device = source.join("bin").join("c600-lx2-child.rs");
    for (path, text) in sources(&source) {
        if path == device {
            assert!(text.contains("#[global_allocator]") && text.contains("c600_runtime::child::main()"));
            assert!(text.contains("c600_runtime::child::main_with(native::point)"));
            // C9 U7: the device keeps Windows Error Reporting out of its own crashes, first thing, per process only.
            assert!(text.contains("fn main() {\n    quiet_error_reporting();\n"));
            assert!(text.contains("SetErrorMode(GetErrorMode() | SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX)"));
            assert!(!text.contains("WerAddExcludedApplication") && !text.contains("RegSetValue"));
        } else {
            assert!(!text.contains("global_allocator"), "{} names a global allocator", path.display());
            assert!(!text.contains("main_with("), "{} calls the fault hook", path.display());
        }
    }
    for file in ["lib.rs", "main.rs"] {
        let text = fs::read_to_string(source.join(file)).expect("the crate roots read");
        assert!(text.contains("#![forbid(unsafe_code)]"), "c600-lx2's {file} forbids unsafe code");
    }
}
