//! The product-build check of option (a) (top integrator, 18 November 2026): the injection device `c600-lx2-child`
//! and its ceiling allocator stay out of the product. No package of the language workspace or of `renderer/bridge`
//! other than c600-lx2 declares a dependency on c600-lx2, so no product binary can link the device; nothing under
//! c600-runtime's source names a global allocator, so the product's runtime child runs on the system allocator; and the
//! unsafe code of the device stays in its bin. Source evidence only.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use c600_kernel::json;

const PACKAGE: &str = "c600-lx2";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// `cargo metadata --no-deps` of `manifest`: each workspace package's name with the names of its declared
/// dependencies, of every kind.
fn packages(manifest: &Path) -> Vec<(String, Vec<String>)> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--offline", "--locked", "--manifest-path"])
        .arg(manifest)
        .output()
        .expect("cargo starts");
    assert!(out.status.success(), "cargo metadata of {}: {}", manifest.display(), String::from_utf8_lossy(&out.stderr));
    let metadata = json::parse(&out.stdout).expect("cargo metadata prints JSON");
    let listed = metadata.get("packages").and_then(|p| p.as_array()).expect("cargo metadata lists packages");
    listed
        .iter()
        .map(|package| {
            let name = package.get("name").and_then(|n| n.as_str()).expect("a package has a name").to_owned();
            let dependencies = package
                .get("dependencies")
                .and_then(|d| d.as_array())
                .expect("a package lists its dependencies")
                .iter()
                .map(|d| d.get("name").and_then(|n| n.as_str()).expect("a dependency has a name").to_owned())
                .collect();
            (name, dependencies)
        })
        .collect()
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
fn no_package_but_this_one_depends_on_the_device_package() {
    let root = repository();
    for manifest in [root.join("rust").join("Cargo.toml"), root.join("renderer").join("bridge").join("Cargo.toml")] {
        let listed = packages(&manifest);
        assert!(!listed.is_empty(), "{} lists no package", manifest.display());
        let dependents: Vec<&str> = listed
            .iter()
            .filter(|(name, dependencies)| name != PACKAGE && dependencies.iter().any(|d| d == PACKAGE))
            .map(|(name, _)| name.as_str())
            .collect();
        assert!(dependents.is_empty(), "{} has packages that depend on {PACKAGE}: {dependents:?}", manifest.display());
    }
    let workspace = packages(&root.join("rust").join("Cargo.toml"));
    assert!(
        workspace.iter().any(|(name, _)| name == PACKAGE),
        "the check runs over the workspace that holds {PACKAGE}"
    );
    assert!(workspace.iter().any(|(name, _)| name == "c600-runtime"), "and over the product's runtime");
}

#[test]
fn the_products_runtime_names_no_global_allocator_and_keeps_unsafe_code_out() {
    let runtime = sources(&repository().join("rust").join("c600-runtime").join("src"));
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
fn the_device_alone_holds_the_allocator_and_the_unsafe_code_of_this_package() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let device = source.join("bin").join("c600-lx2-child.rs");
    for (path, text) in sources(&source) {
        if path == device {
            assert!(text.contains("#[global_allocator]") && text.contains("c600_runtime::child::main()"));
        } else {
            assert!(!text.contains("global_allocator"), "{} names a global allocator", path.display());
        }
    }
    for file in ["lib.rs", "main.rs"] {
        let text = fs::read_to_string(source.join(file)).expect("the crate roots read");
        assert!(text.contains("#![forbid(unsafe_code)]"), "c600-lx2's {file} forbids unsafe code");
    }
}
