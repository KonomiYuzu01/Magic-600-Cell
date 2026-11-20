//! The runtime child and the model for LX-2's tests.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use c600_kernel::json;
use c600_kernel::Kernel;

/// The `c600-runtime` binary and the features cargo built it with. Cargo gives a test no path to another package's
/// binary, so the first call builds it as the product build does, `-p c600-runtime` alone, with the cargo that runs the
/// test, in the test's profile, and takes the path and the features cargo reports.
fn runtime_build() -> &'static (PathBuf, Vec<String>) {
    static BUILT: OnceLock<(PathBuf, Vec<String>)> = OnceLock::new();
    BUILT.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let mut command = Command::new(cargo);
        command.current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."));
        command.args(["build", "--locked", "--offline", "-p", "c600-runtime", "--bin", "c600-runtime"]);
        command.arg("--message-format=json");
        if !cfg!(debug_assertions) {
            command.arg("--release");
        }
        let out = command.output().expect("cargo starts");
        assert!(out.status.success(), "cargo builds the runtime: {}", String::from_utf8_lossy(&out.stderr));
        let mut executable = out.stdout.split(|&byte| byte == b'\n').filter_map(|line| {
            let message = json::parse(line).ok()?;
            let name = message.get("target")?.get("name")?.as_str()?;
            let reason = message.get("reason")?.as_str()?;
            let executable = message.get("executable")?.as_str()?;
            let features = message.get("features")?.as_array()?.iter().filter_map(|f| f.as_str()).map(String::from);
            (reason == "compiler-artifact" && name == "c600-runtime")
                .then(|| (PathBuf::from(executable), features.collect()))
        });
        executable.next_back().expect("cargo reports the runtime's executable")
    })
}

/// The `c600-runtime` binary, built as the product build does (see [`runtime_features`]).
pub fn runtime() -> PathBuf {
    runtime_build().0.clone()
}

/// The features cargo built [`runtime`]'s binary with.
pub fn runtime_features() -> Vec<String> {
    runtime_build().1.clone()
}

/// The injection child of option (a), `c600-lx2-child`, which cargo builds for this package's tests in the test's
/// profile and with the test's features: on the fault hook only with `--features device`.
pub fn injection() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_c600-lx2-child"))
}

/// The model in `assets/`, loaded once per test binary.
pub fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    Arc::clone(KERNEL.get_or_init(|| {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
        Arc::new(Kernel::load(&assets).unwrap_or_else(|e| panic!("the model in assets/ does not load: {e}")))
    }))
}

/// A fresh folder under Cargo's temporary directory for integration tests, removed first if a run left it.
pub fn fresh(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("c600-lx2-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("a fresh folder");
    path
}
