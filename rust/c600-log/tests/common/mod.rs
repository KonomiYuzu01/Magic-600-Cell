//! Shared test setup: the model, the fixtures and what 0.4 returned for them.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| {
        Kernel::load(&repository().join("assets")).unwrap_or_else(|e| panic!("the model in assets/ does not load: {e}"))
    })
}

pub fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub fn expected() -> &'static Value {
    static EXPECTED: OnceLock<Value> = OnceLock::new();
    EXPECTED.get_or_init(|| json::parse(&fixture("expected.json")).expect("expected.json is JSON"))
}

pub fn text<'a>(value: &'a Value, path: &[&str]) -> &'a str {
    walk(value, path).as_str().unwrap_or_else(|| panic!("{path:?} is not a text"))
}

pub fn int(value: &Value, path: &[&str]) -> i64 {
    walk(value, path).as_i64().unwrap_or_else(|| panic!("{path:?} is not an integer"))
}

pub fn walk<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(value, |v, key| v.get(key).unwrap_or_else(|| panic!("{path:?}: no {key}")))
}

pub fn sha256(bytes: &[u8]) -> String {
    c600_digest::sha256_hex(bytes)
}
