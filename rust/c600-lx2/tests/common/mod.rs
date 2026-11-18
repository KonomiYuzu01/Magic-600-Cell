//! The runtime child and the model for LX-2's tests.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use c600_kernel::json;
use c600_kernel::Kernel;

/// The `c600-runtime` binary. Cargo gives a test no path to another package's binary, so the first call builds it
/// with the cargo that runs the test, in the test's profile, and takes the path cargo reports.
pub fn runtime() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
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
                (reason == "compiler-artifact" && name == "c600-runtime").then(|| PathBuf::from(executable))
            });
            executable.next_back().expect("cargo reports the runtime's executable")
        })
        .clone()
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
