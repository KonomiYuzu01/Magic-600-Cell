//! Shared setup of the R8 tests: the owned engines loaded once from `assets/`, repository files read in place
//! (read-only), sessions on memory disks with the shell stub's commit path, and certificate builders. A builder computes
//! a certificate's claims apart from the checkers: a word's digest here, an end state from a session's own commit.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use c600_checkers::engines::Loaded;
use c600_checkers::{CheckerKind, Engines};
use c600_research::{Admitted, Budget, Cancel, Check, CheckJob, CheckRun, Pinned, Reproduced};
use c600_session::disk::FsDisk;
use c600_session::persist::codec::{canonical, Json};
use c600_session::{At, Envelope, Options, Origin, Receipt, Row, Session, SnapshotHandle, Status, WordSource};

/// Limits no test reaches.
pub const WIDE: Budget = Budget::new(1 << 40, 1 << 40, 1 << 40, 1 << 40);

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The owned engines, loaded once per test binary.
pub fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| Loaded::load(&repository().join("assets")).expect("the model in assets/ loads"))
}

pub fn engines() -> Engines<'static> {
    loaded().engines()
}

/// A repository file, read in place (read-only).
pub fn read(relative: &str) -> Vec<u8> {
    std::fs::read(repository().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

pub fn sha(bytes: &[u8]) -> String {
    c600_digest::sha256_hex(bytes)
}

pub fn hex_of(tag: &str) -> String {
    sha(tag.as_bytes())
}

// --------------------------------------------------------------------------------------------------- sessions

/// A new session of 600-cell-Full in a fresh folder under the test's scratch directory (the memory disk is behind a
/// feature this crate does not enable).
pub fn session() -> Session {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("r8-sessions").join(format!(
        "{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&folder);
    let disk = FsDisk::new(&folder).expect("a scratch folder");
    Session::create(Arc::clone(loaded().kernel()), Box::new(disk), "r8", Options::default())
        .expect("the session is created")
}

/// Delivers a row on the shell's channel, with a token the shell stub mints for it when it is a commit row.
pub fn shell(session: &Session, command: &str, row: Row) -> Receipt {
    let mut envelope = Envelope {
        session: session.id().to_owned(),
        epoch: session.epoch(),
        command: command.to_owned(),
        row,
        token: None,
    };
    if envelope.row.is_commit() {
        envelope.token = Some(session.shell().mint(command, &envelope.row).unwrap_or_else(|e| panic!("no token: {e}")));
    }
    session.invoke(Origin::Shell, envelope)
}

/// Proposes a literal word and executes the preview: one commit from the shell, or its refusal.
pub fn commit(session: &Session, command: &str, letters: &[i32]) -> Receipt {
    let proposal =
        shell(session, &format!("{command}.preview"), Row::Propose { word: WordSource::Literal(letters.to_vec()) });
    let preview = match proposal.status {
        Status::Completed(c) => c.result.and_then(|r| r.preview).expect("a preview ID"),
        other => panic!("the proposal was refused: {other:?}"),
    };
    shell(session, command, Row::Execute { preview })
}

pub fn committed(receipt: &Receipt) {
    assert!(matches!(receipt.status, Status::Committed(_)), "not committed: {:?}", receipt.status);
}

pub fn undo(session: &Session, command: &str) -> Receipt {
    let hint = session.overview().undo.expect("an action to undo");
    shell(session, command, Row::Undo { hint, confirm: false })
}

pub fn protect(session: &Session, command: &str, orbits: &[i64]) -> Receipt {
    shell(session, command, Row::ProtectionSet { orbits: orbits.to_vec(), confirm: true })
}

/// The code of a refused or failed receipt.
pub fn code(receipt: &Receipt) -> &'static str {
    match &receipt.status {
        Status::Refused(e) | Status::Failed(e) => e.code,
        other => panic!("neither refused nor failed: {other:?}"),
    }
}

/// The head revision, opened read-only: the job's engine-owned branch.
pub fn snapshot(session: &Session) -> SnapshotHandle {
    session.open_snapshot(At::Current).expect("a snapshot")
}

pub fn digest(session: &Session) -> String {
    session.overview().revision.digest
}

/// The digest a word ends at from the solved state after `before`, computed by a session's own commits.
pub fn end_digest(before: &[i32], word: &[i32]) -> String {
    let oracle = session();
    if !before.is_empty() {
        committed(&commit(&oracle, "before", before));
    }
    committed(&commit(&oracle, "word", word));
    digest(&oracle)
}

// ------------------------------------------------------------------------------------------------ certificates

/// The start input of a full-alphabet replay: the labels of a snapshot's state.
pub fn labels_record(handle: &SnapshotHandle) -> Vec<u8> {
    let labels = handle.state().labels().iter().map(|&label| Json::Int(i64::from(label))).collect();
    canonical(&Json::object([("labels", Json::List(labels))]))
}

/// The SHA-256 of a full-alphabet word: its letters as little-endian int32 (the kernel's `Digest<literal word>`).
pub fn word_sha(letters: &[i32]) -> String {
    let mut hash = c600_digest::Sha256::new();
    for letter in letters {
        hash.update(&letter.to_le_bytes());
    }
    c600_digest::hex(&hash.finish())
}

fn object(fields: Vec<(&str, Json)>) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

/// A `kn6-expansion/1` certificate of the literal word `letters`, with `extra` top-level fields, and with its replay
/// from `start` (a labels record, with its state's digest) to the stated end digest.
pub fn expansion_with(letters: &[i32], replay: Option<(&[u8], &str, &str)>, extra: Vec<(&str, Json)>) -> Vec<u8> {
    let rule = Json::List(letters.iter().map(|&m| Json::Int(i64::from(m))).collect());
    let mut fields = vec![
        ("format", Json::text("kn6-expansion/1")),
        ("alphabet", Json::text("full")),
        ("rules", Json::List(vec![rule])),
        ("root", Json::Int(0)),
        (
            "claims",
            Json::object([("length", Json::Int(letters.len() as i64)), ("sha256", Json::text(&word_sha(letters)))]),
        ),
    ];
    match replay {
        None => fields.push(("inputs", Json::object([]))),
        Some((start, from, to)) => {
            fields.push(("inputs", Json::object([("start", Json::text(&sha(start)))])));
            let at = |digest: &str| Json::object([("digest", Json::text(digest))]);
            fields.push(("replay", Json::object([("start", at(from)), ("end", at(to))])));
        }
    }
    fields.extend(extra);
    canonical(&object(fields))
}

pub fn expansion(letters: &[i32], replay: Option<(&[u8], &str, &str)>) -> Vec<u8> {
    expansion_with(letters, replay, Vec::new())
}

/// A program of `2 · a · b` letters, `(7, -8)` repeated, with `digest` as its claimed SHA-256.
pub fn long_program(a: u64, b: u64, digest: &str) -> Vec<u8> {
    let reference =
        |rule: i64, times: u64| Json::object([("rule", Json::Int(rule)), ("times", Json::Int(times as i64))]);
    canonical(&object(vec![
        ("format", Json::text("kn6-expansion/1")),
        ("alphabet", Json::text("full")),
        ("inputs", Json::object([])),
        (
            "rules",
            Json::List(vec![
                Json::List(vec![Json::Int(7), Json::Int(-8)]),
                Json::List(vec![reference(0, a)]),
                Json::List(vec![reference(1, b)]),
            ]),
        ),
        ("root", Json::Int(2)),
        ("claims", Json::object([("length", Json::Int((2 * a * b) as i64)), ("sha256", Json::text(digest))])),
    ]))
}

/// The SHA-256 of `(7, -8)` repeated `n` times.
pub fn pairs_digest(n: u64) -> String {
    let mut hash = c600_digest::Sha256::new();
    for _ in 0..n {
        hash.update(&7i32.to_le_bytes());
        hash.update(&(-8i32).to_le_bytes());
    }
    c600_digest::hex(&hash.finish())
}

/// The exact I_a menu record of the W-J fixture, as canonical JSON bytes.
pub fn menu_record() -> Vec<u8> {
    let fixture = read("research/jumbling/fixtures/wj-I_a.json");
    let doc = c600_checkers::claims::document(&fixture).ok().expect("the fixture is JSON");
    doc.get("menu").expect("a menu").canonical().into_bytes()
}

// ------------------------------------------------------------------------------------------------------- jobs

/// An expansion check of a short word with no replay: it passes and charges one unit per letter.
pub fn short_check(letters: &[i32]) -> Check {
    Check::new(CheckerKind::Expansion, expansion(letters, None), Vec::new())
}

/// A long expansion check: `2 · a · b` letters whose claimed digest is wrong, so only a completed check would fail.
pub fn long_check(a: u64, b: u64) -> Check {
    Check::new(CheckerKind::Expansion, long_program(a, b, &hex_of("not the word")), Vec::new())
}

/// A job pinned at a stated start under the solved full identity.
pub fn stated_job(checks: Vec<Check>, budget: Budget) -> CheckJob {
    let identity = c600_session::persist::record::identity_json(&loaded().full_identity());
    CheckJob { checks, input: Pinned::stated("labels", &hex_of("a stated start"), identity), budget }
}

pub fn admit(job: CheckJob) -> Admitted {
    job.admit().expect("the job is admitted")
}

pub fn run(admitted: &Admitted) -> CheckRun {
    admitted.run(&engines(), &Cancel::new()).expect("the record is written")
}

pub fn run_job(job: CheckJob) -> CheckRun {
    run(&admit(job))
}

/// DET.7 on a run's stored bytes, with the sources and captured inputs the run names.
pub fn reproduce(run: &CheckRun) -> Reproduced {
    c600_research::reproduce(&engines(), run.bytes(), &run.available(), &Cancel::new())
}

/// The SHA-256 of a run's exact payload.
pub fn payload_digest(run: &CheckRun) -> String {
    sha(&canonical(run.payload()))
}
