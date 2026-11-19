//! C9's session fixture (charter E3, runbook U6; a draft): an existing session for the product shell's cold start, "with
//! an existing session and runtime loaded". The product build writes it once with the `c600-shell-session-fixture`
//! binary ([`write`]), and the launcher gives each run a fresh copy of its session folder, which the host reopens
//! ([`crate::m6h_open_session`] with the fixture, [`crate::turns::Turns::reopen`]).
//!
//! The fixture is a session made through the shell's own turns: a new session at the solved state, the twist keys of
//! [`KEYS`] pressed through the input router and committed through the command layer, and the correctness control run
//! at the close, which must pass. Its folder holds:
//! - `session/`: the session's journal (with its `lock` file, which `FsDisk` never removes);
//! - `fixture.json` ([`RECORD`]): the format [`FORMAT`], the keys, the committed presses' arguments, the engine revision
//!   that shows the last of them ([`crate::ENGINE_REVISION`] plus the presses), the state's digest, and the control's
//!   record.
//!
//! Draft: the presses are a fixed list of the router's three twist keys and their inverses on the one cap the router
//! grips; the bar names no session size. Nothing here is Windows/DirectX, input or performance evidence.

use std::path::Path;
use std::sync::Arc;

use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

use crate::turns::{Press, Turns};
use crate::ENGINE_REVISION;

/// The record's format.
pub const FORMAT: &str = "magic600-c9-session-fixture-v1";
/// The session folder's name in the fixture's folder.
pub const SESSION: &str = "session";
/// The record's file name in the fixture's folder.
pub const RECORD: &str = "fixture.json";
/// The presses of the product's fixture (draft): the router's twist keys and their inverses (Shift), 24 presses.
pub const KEYS: [&str; 24] = [
    "KeyJ",
    "KeyK",
    "KeyL",
    "Shift+KeyJ",
    "KeyK",
    "KeyL",
    "KeyJ",
    "Shift+KeyK",
    "KeyL",
    "KeyL",
    "KeyJ",
    "Shift+KeyL",
    "KeyK",
    "KeyJ",
    "KeyJ",
    "Shift+KeyK",
    "KeyL",
    "KeyK",
    "Shift+KeyJ",
    "KeyL",
    "KeyK",
    "KeyJ",
    "Shift+KeyL",
    "KeyK",
];

/// What the host needs of a fixture to reopen its session.
#[derive(Clone, Debug, PartialEq)]
pub struct Fixture {
    /// The arguments of every committed press, in order.
    pub committed: Vec<Value>,
    /// The digest of the session's state after them.
    pub digest: String,
    /// The engine revision that shows that state: [`ENGINE_REVISION`] plus the presses.
    pub revision: u64,
}

/// Reads a fixture's record from `path`; refuses another format, a malformed record, or a revision that does not
/// count the presses.
pub fn read(path: &Path) -> Result<Fixture, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("the session fixture: {error}"))?;
    let record = json::parse(&bytes).map_err(|error| format!("the session fixture is not JSON: {error:?}"))?;
    if record.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(format!("the session fixture is not {FORMAT}"));
    }
    let committed = record
        .get("committed")
        .and_then(Value::as_array)
        .filter(|presses| presses.iter().all(|press| press.as_object().is_some()))
        .ok_or("the session fixture's committed presses are not a list of objects")?
        .to_vec();
    let digest = record.get("digest").and_then(Value::as_str).ok_or("the session fixture has no digest")?.to_owned();
    let revision = record
        .get("revision")
        .and_then(Value::as_i64)
        .and_then(|revision| u64::try_from(revision).ok())
        .ok_or("the session fixture has no revision")?;
    if revision != ENGINE_REVISION + committed.len() as u64 {
        return Err(format!(
            "the session fixture's revision {revision} does not count its {} presses",
            committed.len()
        ));
    }
    Ok(Fixture { committed, digest, revision })
}

/// Makes a fixture in the new folder `out` (it must not exist): a new session in `out/session` with the twist menu of
/// `menu_file` pinned, each chord of `keys` pressed and released (each must commit), and the control at the close,
/// which must pass. Writes `out/fixture.json` and answers its record.
pub fn write(kernel: Arc<Kernel>, assets: &Path, menu_file: &Path, out: &Path, keys: &[&str]) -> Result<Value, String> {
    if out.exists() {
        return Err("the fixture folder already exists; a fixture is never replaced".to_owned());
    }
    std::fs::create_dir_all(out).map_err(|error| format!("the fixture folder: {error}"))?;
    let mut turns = Turns::open(kernel, assets, &out.join(SESSION), menu_file)?;
    for chord in keys {
        match turns.key_down(chord, false)? {
            Press::Committed => {}
            press => return Err(format!("`{chord}` did not commit a twist: {press:?}")),
        }
        turns.key_up(chord)?;
    }
    let digest = turns.state()?.digest().to_owned();
    let revision = ENGINE_REVISION + keys.len() as u64;
    let control = turns.check(Some((revision, digest.clone())));
    if control.get("pass") != Some(&Value::Bool(true)) {
        return Err(format!("the fixture's correctness control failed: {}", control.canonical()));
    }
    let record = json::object([
        ("format", Value::String(FORMAT.to_owned())),
        ("session", Value::String(SESSION.to_owned())),
        ("keys", Value::Array(keys.iter().map(|key| Value::String((*key).to_owned())).collect())),
        ("committed", Value::Array(turns.committed().to_vec())),
        ("revision", Value::Int(revision as i64)),
        ("digest", Value::String(digest)),
        ("control", control),
    ]);
    std::fs::write(out.join(RECORD), record.canonical() + "\n")
        .map_err(|error| format!("the fixture's record: {error}"))?;
    Ok(record)
}
