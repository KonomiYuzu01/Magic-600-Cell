//! C9's session fixture (charter E3, runbook U6; a draft): an existing session for the product shell's cold start, "with
//! an existing session and runtime loaded". The product build writes it once with the `c600-shell-session-fixture`
//! binary ([`write`]), and the launcher gives each run a fresh copy of its session folder, which the host reopens
//! ([`crate::m6h_open_session`] with the fixture, [`crate::turns::Turns::reopen`]).
//!
//! The fixture is a session made through the shell's own turns: a new session of one puzzle at the solved state (with
//! Jumble on, `jumble.on` committed at its start), the twist keys of [`KEYS`] pressed through the input router and
//! committed through the command layer, and the correctness control run at the close, which must pass. Its folder
//! holds:
//! - `session/`: the session's journal (with its `lock` file, which `FsDisk` never removes);
//! - `fixture.json` ([`RECORD`]): the format [`FORMAT`], the puzzle, the keys, the committed presses' arguments, the
//!   engine revision that shows the last of them ([`Puzzle::opened_at`] plus the presses), the head's digest, and the
//!   control's record. A record without a puzzle is a Jumble-off one, as before the jumbled puzzle had fixtures.
//!
//! Draft: the presses are a fixed list of the router's three twist keys and their inverses on the one cap the router
//! grips; the bar names no session size.
//!
//! ADR-D02 B5 adds a word fixture ([`write_word`]): a jumble-on session whose letters `[cap, item]` of the pinned menu
//! are committed as `jumble.twist.menu` through the shell's route ([`Turns::twist_letter`]), at any cap, so that a
//! session can open at one of D02's sample states. Its record adds the word and the head's state digests
//! ([`Turns::state_digests`]), by which `renderer/shell/build_d02_fixtures.py` checks it against the sample and the
//! shell binds the sample to the session. Nothing here is Windows/DirectX, input or performance evidence.

use std::path::Path;
use std::sync::Arc;

use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

use crate::turns::{Press, Puzzle, Turns};

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
    /// The session's puzzle.
    pub puzzle: Puzzle,
    /// The arguments of every committed press, in order.
    pub committed: Vec<Value>,
    /// The digest of the session's head after them.
    pub digest: String,
    /// The engine revision that shows that head: [`Puzzle::opened_at`] plus the presses.
    pub revision: u64,
}

/// Reads a fixture's record from `path`; refuses another format, an unknown puzzle, a malformed record, or a revision
/// that does not count the presses.
pub fn read(path: &Path) -> Result<Fixture, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("the session fixture: {error}"))?;
    let record = json::parse(&bytes).map_err(|error| format!("the session fixture is not JSON: {error:?}"))?;
    if record.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(format!("the session fixture is not {FORMAT}"));
    }
    let puzzle = match record.get("puzzle") {
        None => Puzzle::JumbleOff,
        Some(name) => name.as_str().and_then(Puzzle::from_name).ok_or("the session fixture's puzzle is unknown")?,
    };
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
    if revision != puzzle.opened_at() + committed.len() as u64 {
        return Err(format!(
            "the session fixture's revision {revision} does not count its {} presses",
            committed.len()
        ));
    }
    Ok(Fixture { puzzle, committed, digest, revision })
}

/// Makes a Jumble-off fixture: [`write_for`] with [`Puzzle::JumbleOff`].
pub fn write(kernel: Arc<Kernel>, assets: &Path, menu_file: &Path, out: &Path, keys: &[&str]) -> Result<Value, String> {
    write_for(Puzzle::JumbleOff, kernel, assets, menu_file, out, keys)
}

/// Makes a fixture of `puzzle` in the new folder `out` (it must not exist): a new session in `out/session` with the
/// twist menu of `menu_file` pinned (with Jumble on, `jumble.on` committed at its start), each chord of `keys` pressed
/// and released (each must commit), and the control at the close, which must pass. Writes `out/fixture.json` and
/// answers its record.
pub fn write_for(
    puzzle: Puzzle,
    kernel: Arc<Kernel>,
    assets: &Path,
    menu_file: &Path,
    out: &Path,
    keys: &[&str],
) -> Result<Value, String> {
    let mut turns = start(puzzle, kernel, assets, menu_file, out)?;
    for chord in keys {
        match turns.key_down(chord, false)? {
            Press::Committed => {}
            press => return Err(format!("`{chord}` did not commit a twist: {press:?}")),
        }
        turns.key_up(chord)?;
    }
    let keys = keys.iter().map(|key| Value::String((*key).to_owned())).collect();
    close(turns, puzzle, out, keys, Vec::new())
}

/// Makes a jumble-on fixture of a word (ADR-D02 B5) in the new folder `out`: [`write_for`]'s session with Jumble on,
/// with each letter `[cap, item]` of `letters` committed as `jumble.twist.menu` through the shell's route
/// ([`Turns::twist_letter`]) in place of router keys (each must commit), and the control at the close, which must
/// pass. The record's `keys` is empty; it adds the `word` and the head's `state_digests`.
pub fn write_word(
    kernel: Arc<Kernel>,
    assets: &Path,
    menu_file: &Path,
    out: &Path,
    letters: &[[u64; 2]],
) -> Result<Value, String> {
    let mut turns = start(Puzzle::JumbleOn, kernel, assets, menu_file, out)?;
    for (index, &[cap, item]) in letters.iter().enumerate() {
        let cap = u32::try_from(cap).map_err(|_| format!("letter {index}: {cap} is not a cap"))?;
        turns.twist_letter(cap, item).map_err(|error| format!("letter {index}: {error}"))?;
    }
    let digests = turns.state_digests()?;
    let word = letters.iter().map(|&[cap, item]| Value::Array(vec![Value::Int(cap as i64), Value::Int(item as i64)]));
    let extra = vec![("word", Value::Array(word.collect())), ("state_digests", digests)];
    close(turns, Puzzle::JumbleOn, out, Vec::new(), extra)
}

/// A new session of `puzzle` in `out/session`, with `out` made new (it must not exist).
fn start(puzzle: Puzzle, kernel: Arc<Kernel>, assets: &Path, menu_file: &Path, out: &Path) -> Result<Turns, String> {
    if out.exists() {
        return Err("the fixture folder already exists; a fixture is never replaced".to_owned());
    }
    std::fs::create_dir_all(out).map_err(|error| format!("the fixture folder: {error}"))?;
    Turns::open(kernel, assets, &out.join(SESSION), menu_file, puzzle)
}

/// Runs the control at the close, which must pass, and writes `out/fixture.json` with `extra` beside its fields.
fn close(
    mut turns: Turns,
    puzzle: Puzzle,
    out: &Path,
    keys: Vec<Value>,
    extra: Vec<(&str, Value)>,
) -> Result<Value, String> {
    let digest = turns.head()?.digest();
    let committed = turns.committed().to_vec();
    let revision = puzzle.opened_at() + committed.len() as u64;
    let control = turns.check(Some((revision, digest.clone())));
    if control.get("pass") != Some(&Value::Bool(true)) {
        return Err(format!("the fixture's correctness control failed: {}", control.canonical()));
    }
    let fields = [
        ("format", Value::String(FORMAT.to_owned())),
        ("session", Value::String(SESSION.to_owned())),
        ("puzzle", Value::String(puzzle.name().to_owned())),
        ("keys", Value::Array(keys)),
        ("committed", Value::Array(committed)),
        ("revision", Value::Int(revision as i64)),
        ("digest", Value::String(digest)),
        ("control", control),
    ];
    let record = Value::Object(fields.into_iter().chain(extra).map(|(key, value)| (key.to_owned(), value)).collect());
    std::fs::write(out.join(RECORD), record.canonical() + "\n")
        .map_err(|error| format!("the fixture's record: {error}"))?;
    Ok(record)
}
