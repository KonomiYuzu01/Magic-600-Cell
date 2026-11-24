//! Shared setup of the exit sweep: the edition of claim bindings, the owned engines, the archive's files read in place
//! (read-only) and pinned by the edition's digests, the preflight host, and the I_a menu checked against the edition's
//! identity block.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use c600_archive::bindings::{Bindings, FilePin, Host, InputState};
use c600_checkers::archive;
use c600_checkers::engines::Loaded;
use c600_checkers::Engines;

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn sha(bytes: &[u8]) -> String {
    c600_digest::sha256_hex(bytes)
}

/// Edition 1 of the claim bindings, with its pinned digest.
pub fn bindings() -> &'static Bindings {
    static BINDINGS: OnceLock<Bindings> = OnceLock::new();
    BINDINGS.get_or_init(Bindings::edition_1)
}

/// The owned engines, loaded once per test binary from `assets/`.
pub fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| Loaded::load(&repository().join("assets")).expect("the model in assets/ loads"))
}

pub fn engines() -> Engines<'static> {
    loaded().engines()
}

/// A repository file's bytes, read in place; `None` when the checkout does not hold it.
pub fn read(relative: &str) -> Option<Vec<u8>> {
    std::fs::read(repository().join(relative)).ok()
}

/// The SHA-256 of a checkout file, computed once; `None` when the checkout does not hold it.
pub fn checkout_digest(relative: &str) -> Option<String> {
    static DIGESTS: OnceLock<Mutex<BTreeMap<String, Option<String>>>> = OnceLock::new();
    let cache = DIGESTS.get_or_init(Default::default);
    if let Some(found) = cache.lock().unwrap().get(relative) {
        return found.clone();
    }
    let digest = read(relative).map(|bytes| sha(&bytes));
    cache.lock().unwrap().insert(relative.to_owned(), digest.clone());
    digest
}

/// A stored input's bytes, read in place once and checked against the edition's pin. A file the checkout does not
/// hold, or holds with other bytes, is a test failure: preflight answered it `Present`.
pub fn pinned(file: &FilePin) -> Arc<Vec<u8>> {
    static FILES: OnceLock<Mutex<BTreeMap<String, Arc<Vec<u8>>>>> = OnceLock::new();
    let cache = FILES.get_or_init(Default::default);
    if let Some(found) = cache.lock().unwrap().get(&file.path) {
        return Arc::clone(found);
    }
    let bytes = read(&file.path).unwrap_or_else(|| panic!("{}: not in the checkout", file.path));
    assert_eq!(sha(&bytes), file.sha256, "{}: not the pinned bytes", file.path);
    assert_eq!(bytes.len() as u64, file.bytes, "{}: not the pinned size", file.path);
    let bytes = Arc::new(bytes);
    cache.lock().unwrap().insert(file.path.clone(), Arc::clone(&bytes));
    bytes
}

/// LB5.14's host: the KN6 registry of `c600-checkers` (a sub-checker by its ID, with its source digest) and the
/// checkout's actual bytes (an input is held when the checkout's file has the pinned SHA-256).
pub struct Checkout;

impl Host for Checkout {
    fn checker(&self, id: &str) -> Option<String> {
        archive::find(id).map(|sub| sub.checker.sources.identity().source)
    }

    fn input(&self, file: &FilePin) -> InputState {
        match checkout_digest(&file.path) {
            None => InputState::Missing,
            Some(digest) if digest == file.sha256 => InputState::Present,
            Some(_) => InputState::Mismatch,
        }
    }
}

/// The W-J fixture the checkers' own tests read the I_a menu record from.
pub const MENU_FIXTURE: &str = "research/jumbling/fixtures/wj-I_a.json";

/// The exact I_a menu record, as canonical JSON bytes, built as the checkers' own tests build it and accepted only when
/// the jumbling engine reads from it the edition's identity block (model, menu identity, menu order and contract).
pub fn menu() -> &'static [u8] {
    static MENU: OnceLock<Vec<u8>> = OnceLock::new();
    MENU.get_or_init(|| {
        let fixture = read(MENU_FIXTURE).expect("the W-J I_a fixture");
        let doc = c600_checkers::claims::document(&fixture).ok().expect("the fixture is JSON");
        let menu = doc.get("menu").expect("a menu").canonical().into_bytes();
        let block = loaded().jumbling_identity(&menu).expect("the engine reads the menu record");
        let edition = c600_checkers::claims::document(bindings().identity.as_bytes()).ok().expect("the identity block");
        let field = |key: &str| edition.get(key).and_then(|v| v.as_str()).map(str::to_owned);
        assert_eq!(field("puzzle").as_deref(), Some("jumbling"));
        assert_eq!(field("model_id"), Some(block.model_id.clone()), "the menu's model is the edition's");
        assert_eq!(field("menu_id"), block.menu_id.clone(), "the menu's identity is the edition's");
        assert_eq!(field("menu_order"), block.menu_order.clone(), "the menu's order is the edition's");
        assert_eq!(field("contract_rev"), block.contract_rev.clone(), "the menu's contract is the edition's");
        menu
    })
}

/// The directory part of a repository path, with its trailing slash.
pub fn directory(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| &path[..dir.len() + 1])
}

/// The file name part of a repository path.
pub fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}
