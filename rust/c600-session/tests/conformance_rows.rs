//! LP2 conformance rows (chapter 12, section 15.3) that the session store carries: the session halves of CT-ONE-01
//! and CT-ONE-02, the session rows of CT-KN5-10 and the token half of CT-KN5-03. Each test's marker names what it runs
//! and what it does not run (PHS.62, PHS.63).

mod common;

use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};
use std::thread;

use c600_kernel::{codes, Derivation, Derived, Kernel, ProtectedSet, Snapshot, ORBITS};
use c600_run::interp::Loaded;
use c600_run::meter::Amounts;
use c600_run::provenance::Json;
use c600_run::value::Value;
use c600_session::disk::MemDisk;
use c600_session::{
    code, ActionHint, At, Envelope, Input, JobId, JobKind, JobSpec, Lane, Origin, Overview, PreviewInfo, Receipt, Row,
    Session, SnapshotHandle, Status, WordSource,
};
use common::*;

/// The number of variants of `c600_session::Row`.
const VARIANTS: usize = 10;

/// The rows' names, in [`variant`]'s order.
const NAMES: [&str; VARIANTS] = [
    "operation.preview",
    "operation.execute",
    "session.undo",
    "session.redo",
    "session.switch_line",
    "session.checkpoint",
    "workspace.set",
    "protection.set",
    "job.submit",
    "job.stop",
];

/// A row's place in [`NAMES`]. The match names every variant and every field, with no wildcard: a new row or a new
/// field breaks the build here, and the tests below must then be extended to it.
fn variant(row: &Row) -> usize {
    match row {
        Row::Propose { word: _ } => 0,
        Row::Execute { preview: _ } => 1,
        Row::Undo { hint: _, confirm: _ } => 2,
        Row::Redo { hint: _, confirm: _ } => 3,
        Row::SwitchLine { line: _ } => 4,
        Row::Checkpoint { name: _ } => 5,
        Row::WorkspaceSet { record: _, payload: _, provenance: _ } => 6,
        Row::ProtectionSet { orbits: _, confirm: _ } => 7,
        Row::JobSubmit(_) => 8,
        Row::JobStop { job: _ } => 9,
    }
}

/// Whether a row changes the puzzle state, classified here by an exhaustive match rather than by `Row::is_commit`.
fn commit_row(row: &Row) -> bool {
    match row {
        Row::Execute { .. } | Row::Undo { .. } | Row::Redo { .. } | Row::SwitchLine { .. } => true,
        Row::Propose { .. }
        | Row::Checkpoint { .. }
        | Row::WorkspaceSet { .. }
        | Row::ProtectionSet { .. }
        | Row::JobSubmit(_)
        | Row::JobStop { .. } => false,
    }
}

/// What the session does with one field of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Use {
    /// Letters the kernel replays from the current state.
    Letters,
    /// A name, an ID or a hint the session resolves against its own objects.
    Reference,
    /// A flag or a set of orbits that parameterizes the row.
    Parameter,
    /// A value the workspace stores. A proposal can read it only as a list of letters.
    Stored,
    /// A checked program and its arguments, run on a job lane that reaches no commit row.
    Program,
    /// A snapshot a job reads. It is never adopted.
    ReadOnlyInput,
    /// A state or a digest the session would adopt. ONE.26 and KN5.13 forbid it, so no field may be classified here.
    #[allow(dead_code)]
    Adopt,
}

/// Every field of a row with its use. Every variant and every field is named with its type, with no wildcard: a new
/// row, a new field or a field of another type breaks the build here and must be classified.
fn uses(row: &Row) -> Vec<(&'static str, Use)> {
    match row {
        Row::Propose { word } => match word {
            WordSource::Literal(letters) => {
                let _: &Vec<i32> = letters;
                vec![("word", Use::Letters)]
            }
            WordSource::Record(name) => {
                let _: &String = name;
                vec![("word", Use::Reference)]
            }
        },
        Row::Execute { preview } => {
            let _: &u64 = preview;
            vec![("preview", Use::Reference)]
        }
        Row::Undo { hint, confirm } | Row::Redo { hint, confirm } => {
            let _: (&ActionHint, &bool) = (hint, confirm);
            vec![("hint", Use::Reference), ("confirm", Use::Parameter)]
        }
        Row::SwitchLine { line } => {
            let _: &usize = line;
            vec![("line", Use::Reference)]
        }
        Row::Checkpoint { name } => {
            let _: &String = name;
            vec![("name", Use::Reference)]
        }
        Row::WorkspaceSet { record, payload, provenance } => {
            let _: (&String, &Json, &Vec<u8>) = (record, payload, provenance);
            vec![("record", Use::Reference), ("payload", Use::Stored), ("provenance", Use::Stored)]
        }
        Row::ProtectionSet { orbits, confirm } => {
            let _: (&Vec<i64>, &bool) = (orbits, confirm);
            vec![("orbits", Use::Parameter), ("confirm", Use::Parameter)]
        }
        Row::JobSubmit(JobSpec { program, program_digest, module, function, args, inputs, lane, kind, budget }) => {
            let _: (&Arc<Loaded>, &String, &String, &String) = (program, program_digest, module, function);
            let _: (&Vec<Value>, &Lane, &JobKind, &Amounts) = (args, lane, kind, budget);
            // The program digest names the program; it is not a state.
            let mut out = vec![
                ("program", Use::Program),
                ("program_digest", Use::Reference),
                ("module", Use::Reference),
                ("function", Use::Reference),
                ("args", Use::Program),
                ("lane", Use::Parameter),
                ("kind", Use::Parameter),
                ("budget", Use::Parameter),
            ];
            for input in inputs {
                match input {
                    Input::Current => out.push(("inputs", Use::ReadOnlyInput)),
                    Input::Handle(handle) => {
                        let _: &SnapshotHandle = handle;
                        out.push(("inputs", Use::ReadOnlyInput));
                    }
                }
            }
            out
        }
        Row::JobStop { job } => {
            let _: &JobId = job;
            vec![("job", Use::Reference)]
        }
    }
}

/// An envelope from the shell with a token the shell stub minted for it.
fn minted(session: &Session, command: &str, row: Row) -> Envelope {
    let mut e = envelope(session, command, row);
    e.token = Some(session.shell().mint(command, &e.row).unwrap_or_else(|e| panic!("no token: {e}")));
    e
}

/// What a refused delivery must leave as it was.
fn observed(session: &Session, disk: &MemDisk) -> (Overview, Option<PreviewInfo>, u64) {
    (session.overview(), session.preview(), disk.writes())
}

/// Delivers an envelope on the shell's channel that must be refused with this code and change nothing.
fn refused(session: &Session, disk: &MemDisk, envelope: Envelope, expected: &str) -> Receipt {
    refused_on(session, disk, Origin::Shell, envelope, expected)
}

/// [`refused`] on the channel of `origin`.
fn refused_on(session: &Session, disk: &MemDisk, origin: Origin, envelope: Envelope, expected: &str) -> Receipt {
    let before = observed(session, disk);
    let receipt = session.invoke(origin, envelope);
    assert!(matches!(receipt.status, Status::Refused(_)), "{receipt:?}");
    assert_eq!(code(&receipt), expected, "{receipt:?}");
    assert_eq!(observed(session, disk), before, "a refusal changed the session");
    receipt
}

/// A receipt of a row that ran.
fn ran(receipt: &Receipt) {
    assert!(matches!(receipt.status, Status::Committed(_) | Status::Completed(_)), "{receipt:?}");
}

/// A session where every row can run from the shell: two lines, an action to undo and one to redo, and a fresh
/// pending preview.
fn staged() -> (Session, MemDisk) {
    let (session, disk) = session();
    committed(&commit(&session, "a", &[1]));
    committed(&undo(&session, "u1"));
    committed(&commit(&session, "b", &[2]));
    committed(&commit(&session, "c", &[3]));
    committed(&undo(&session, "u2"));
    propose(&session, "p", literal(&[4]));
    let o = session.overview();
    assert!(
        o.lines == 2 && o.active_line == 1 && o.undo.is_some() && o.redo.is_some() && o.pending_preview.is_some(),
        "{o:?}"
    );
    (session, disk)
}

/// One row of each variant, in [`NAMES`]' order, each valid in a [`staged`] session.
fn samples(session: &Session) -> Vec<Row> {
    let o = session.overview();
    vec![
        Row::Propose { word: literal(&[4]) },
        Row::Execute { preview: o.pending_preview.expect("a pending preview") },
        Row::Undo { hint: o.undo.expect("an action to undo"), confirm: false },
        Row::Redo { hint: o.redo.expect("an action to redo"), confirm: false },
        Row::SwitchLine { line: 0 },
        Row::Checkpoint { name: "k".into() },
        Row::WorkspaceSet { record: "r".into(), payload: letters(&[1, 2]), provenance: manual() },
        Row::ProtectionSet { orbits: vec![0], confirm: false },
        Row::JobSubmit(job("count", vec![Value::int(3)], vec![Input::Current], Lane::Program, JobKind::Query)),
        Row::JobStop { job: JobId { epoch: session.epoch(), n: 1 << 40 } },
    ]
}

/// The kernel's own derivation of a word from a state, under no protection.
fn derived(kernel: &Kernel, base: &Snapshot, letters: &[i64]) -> Derived {
    let recipe = kernel.word(letters).unwrap_or_else(|r| panic!("not a word: {r:?}"));
    match kernel.derive(base, &recipe, &ProtectedSet::none()) {
        Ok(Derivation::Derived(d)) => d,
        other => panic!("the kernel did not derive the word: {other:?}"),
    }
}

/// The session's current state.
fn current(session: &Session) -> Snapshot {
    session.open_snapshot(At::Current).expect("the current state").state().clone()
}

/// The writes one `operation.execute` makes.
fn execute_writes() -> u64 {
    let kinds = kinds();
    writes_of(kinds.iter().find(|k| k.name == "operation.execute").expect("the execute kind"))
}

/// CT-ONE-01 (part: the session half: an exhaustive match over every `c600_session::Row` variant and field finds none
/// that takes a state or a digest to adopt; workspace records shaped like a digest or a label list, proposed as words,
/// are refused or give the kernel's own derivation of a word from the current state, and a commit of that preview and a
/// job reading a snapshot of another state adopt neither the record's state nor the snapshot; the runtime's half is
/// `ct_one_01_no_operation_takes_a_state_or_a_digest_to_adopt` in `c600-run/tests/tables.rs`, and the command layer's
/// H-08 rows are `no_row_takes_a_state_or_a_digest_to_adopt` in `c600-command/tests/layer.rs`)
#[test]
fn ct_one_01_no_row_takes_a_state_or_a_digest_to_adopt() {
    // The enumeration: one row of each variant, and a job whose input is an opened snapshot.
    let (staged_session, _staged_disk) = staged();
    let mut rows = samples(&staged_session);
    let handle = staged_session.open_snapshot(At::Current).expect("a snapshot");
    rows.push(Row::JobSubmit(job("state", vec![], vec![Input::Handle(handle)], Lane::Program, JobKind::Query)));
    let mut seen = [false; VARIANTS];
    for row in &rows {
        // Can fail: the enumerated rows are the ones that exist, under their own names.
        assert_eq!(row.name(), NAMES[variant(row)]);
        seen[variant(row)] = true;
        for (field, using) in uses(row) {
            assert_ne!(using, Use::Adopt, "{} takes its {field} to adopt", row.name());
        }
    }
    assert!(seen.iter().all(|&s| s), "a row variant was not enumerated: {seen:?}");

    // Records shaped like a digest or a label list of a state the session does not hold, proposed as words.
    let (session, disk) = session();
    let kernel = kernel();
    let start = current(&session);
    let target = derived(&kernel, &start, &[1]);
    let target_digest = target.state.digest().to_owned();
    assert_ne!(target_digest, start.digest(), "the target is a state the session does not hold");
    let labels: Vec<i64> = target.state.labels().iter().map(|&l| i64::from(l)).collect();
    let digest_bytes: Vec<i64> = target_digest.bytes().map(i64::from).collect();
    let prefix: Vec<i64> = labels.iter().copied().filter(|l| (1..=1200).contains(l)).take(3).collect();
    assert_eq!(prefix.len(), 3, "the label list holds legal letters");
    // Each record, its payload, and the word it holds or the code that refuses it.
    let cases: Vec<(&str, Json, Result<Vec<i64>, &str>)> = vec![
        ("digest", Json::Text(target_digest.clone()), Err(codes::NAMED_OBJECT_INVALID)),
        ("labels", letters(&labels), Err(codes::RECIPE_SHAPE)),
        ("digest-bytes", letters(&digest_bytes), Ok(digest_bytes.clone())),
        ("labels-prefix", letters(&prefix), Ok(prefix.clone())),
    ];
    let (mut refusals, mut words) = (0, 0);
    for (record, payload, holds) in cases {
        ran(&set_record(&session, &format!("set {record}"), record, payload));
        let before = observed(&session, &disk);
        let row = Row::Propose { word: WordSource::Record(record.to_owned()) };
        let receipt = shell(&session, &format!("propose {record}"), row);
        // Never an adopted state: a proposal changes no revision and records no action.
        let after = session.overview();
        assert_eq!((&after.revision, after.order), (&before.0.revision, before.0.order), "{record}");
        assert_eq!(after.revision.digest, start.digest(), "{record}");
        match (&receipt.status, &holds) {
            (Status::Refused(e), Err(expected)) => {
                assert_eq!(e.code, *expected, "{record}");
                assert_eq!(observed(&session, &disk), before, "{record}: a refused proposal changed the session");
                refusals += 1;
            }
            (Status::Completed(_), Ok(word)) => {
                let expected = derived(&kernel, &start, word);
                let preview = session.preview().expect("the preview");
                assert_eq!(preview.start, start.digest(), "{record}");
                assert_eq!(preview.end.as_deref(), Some(expected.state.digest()), "{record}: the kernel's derivation");
                assert_eq!(preview.literal, expected.word, "{record}");
                assert_ne!(preview.end.as_deref(), Some(target_digest.as_str()), "{record}: the record's state");
                words += 1;
            }
            (status, holds) => panic!("{record}: {status:?}, expected {holds:?}"),
        }
    }
    // Can fail: the planted refusals were refused and the planted words gave previews.
    assert_eq!((refusals, words), (2, 2));

    // A commit of the record's preview adopts the kernel's own end state.
    let before = session.overview();
    let preview = before.pending_preview.expect("the labels-prefix preview");
    let receipt = session.invoke(Origin::Shell, minted(&session, "execute labels-prefix", Row::Execute { preview }));
    let post = committed(&receipt).post.clone();
    assert_eq!(post, derived(&kernel, &start, &prefix).state.digest());
    assert_ne!(post, target_digest);
    assert_eq!(session.overview().order, before.order + 1);

    // A job reads a snapshot of another state; the session does not adopt it.
    let earlier = session.open_snapshot(At::Revision(before.revision.clone())).expect("the earlier revision");
    assert_eq!(earlier.state().digest(), start.digest());
    let now = session.overview();
    let spec = job("state", vec![], vec![Input::Handle(earlier)], Lane::Program, JobKind::Query);
    let job = submit(&session, "read the earlier state", spec);
    session.wait_job(job).expect("the job's result");
    let after = session.overview();
    assert_eq!((after.revision, after.order), (now.revision, now.order), "a job's input is read, never adopted");
}

/// CT-ONE-02 (part: the session half: by an exhaustive enumeration of `c600_session::Row`, every commit row, `Redo`
/// included, delivered on a program's channel without a token and with a token the shell minted for that very command
/// and row, is refused as `KN5:capability.commit_row_refused` and changes neither `overview()` nor `preview()`, and no
/// other row a program delivers commits; the runtime's half is
/// `ct_kn5_02_no_operation_returns_a_token_or_commits_in_any_profile` in `c600-run/tests/tables.rs`, and the command
/// layer's and the overlay's rows are in `c600-command/tests/profile.rs`). CT-KN5-02 (part: the session's rows: no
/// `c600_session::Row` a program delivers commits, with or without a token the shell minted, and a session receipt has
/// no field that could carry a token; the runtime's half is in `c600-run/tests/tables.rs` and the command layer's in
/// `c600-command/tests/profile.rs`). CT-ONE-23 (whole, with `no_runtime_request_changes_a_line` in `branches.rs` for
/// the job hosts): the staged session has two lines and an undone action, so a program's execute would start a new
/// line, its switch would change the active line and its undo or redo would change a line; each is refused, and the
/// lines stay as they were.
#[test]
fn ct_one_02_a_program_delivers_no_commit_row() {
    let mut seen = [false; VARIANTS];
    let mut commit_rows = Vec::new();
    for index in 0..VARIANTS {
        let (session, disk) = staged();
        let row = samples(&session).swap_remove(index);
        assert_eq!((variant(&row), row.name()), (index, NAMES[index]));
        assert_eq!(commit_row(&row), row.is_commit(), "{}", row.name());
        seen[index] = true;
        if !commit_row(&row) {
            let before = session.overview();
            let receipt = from_program(&session, "program", row);
            assert!(!matches!(receipt.status, Status::Committed(_)), "{receipt:?}");
            // No field of a receipt could carry a token: one added here stops this from compiling.
            let Receipt { command: _, request: _, job: _, status: _ } = &receipt;
            if let Status::Running(job) = receipt.status {
                session.wait_job(job).expect("the job's result");
            }
            let after = session.overview();
            assert_eq!((after.revision, after.order), (before.revision, before.order));
            continue;
        }
        commit_rows.push(row.name());
        let bare = envelope(&session, "bare", row.clone());
        refused_on(&session, &disk, Origin::Program, bare, code::COMMIT_ROW_REFUSED);
        let mut with_token = envelope(&session, "minted", row.clone());
        with_token.token = Some(session.shell().mint("minted", &row).unwrap_or_else(|e| panic!("no token: {e}")));
        refused_on(&session, &disk, Origin::Program, with_token, code::COMMIT_ROW_REFUSED);
        // Can fail: the same row, delivered by the shell with a token of its own, commits.
        let order = session.overview().order;
        committed(&session.invoke(Origin::Shell, minted(&session, "shell", row)));
        assert_eq!(session.overview().order, order + 1);
    }
    assert!(seen.iter().all(|&s| s), "a row variant was not enumerated: {seen:?}");
    assert_eq!(commit_rows, ["operation.execute", "session.undo", "session.redo", "session.switch_line"]);
}

/// CT-KN5-10 (part: R-05 for the session rows: with every orbit protected, every `c600_session::Row` variant is
/// delivered by an exhaustive enumeration; `Execute` and `SwitchLine` are refused with the protection code and change
/// nothing, an `Undo` or a `Redo` that would release protection and a `protection.set` release are refused until
/// confirmed, and no row moves a protected orbit; the H-08 table's 175 rows run against their restrictions in
/// `c600-command/tests/r05_rows.rs`; not run: an `Undo` or a `Redo` of a move refused with the protection code, which
/// M6 cannot reach: protection changes share the one undo order, so what is on top of either stack was done under the
/// protection now in force)
#[test]
fn ct_kn5_10_every_session_row_keeps_every_protected_orbit() {
    let (session, disk) = session();
    committed(&commit(&session, "a", &[1]));
    committed(&undo(&session, "u1"));
    committed(&commit(&session, "b", &[2]));
    assert_eq!((session.overview().lines, session.overview().active_line), (2, 1));
    let all: Vec<i64> = (0..ORBITS as i64).collect();
    let every: Vec<usize> = (0..ORBITS).collect();
    ran(&protect(&session, "protect every orbit", &all));
    assert_eq!(session.overview().protected, every);
    let held = digest(&session);
    let mut seen = [false; VARIANTS];
    let mut deliver = |row: &Row| seen[variant(row)] = true;
    let kept = |session: &Session, protected: &[usize]| {
        let o = session.overview();
        assert_eq!((o.revision.digest.as_str(), o.protected.as_slice()), (held.as_str(), protected));
    };

    // operation.preview: a protection conflict still gets a preview, with no end state (KN5.9).
    let row = Row::Propose { word: literal(&[1]) };
    deliver(&row);
    let receipt = shell(&session, "p1", row);
    ran(&receipt);
    let p1 = session.preview().expect("p1");
    assert_eq!(p1.end, None, "a protected orbit would move");
    kept(&session, &every);

    // operation.execute and session.switch_line: refused with the protection code.
    let row = Row::Execute { preview: p1.id };
    deliver(&row);
    refused(&session, &disk, minted(&session, "x1", row), codes::ORBIT_WOULD_MOVE);
    let row = Row::SwitchLine { line: 0 };
    deliver(&row);
    refused(&session, &disk, minted(&session, "x2", row), codes::ORBIT_WOULD_MOVE);

    // session.undo of the protection: it would release every orbit, so it needs confirmation.
    let row = Row::Undo { hint: session.overview().undo.expect("the protection"), confirm: false };
    deliver(&row);
    refused(&session, &disk, minted(&session, "x3", row), code::CONFIRMATION_REQUIRED);

    // session.checkpoint and workspace.set: no orbit moves.
    let row = Row::Checkpoint { name: "k".into() };
    deliver(&row);
    ran(&shell(&session, "k", row));
    kept(&session, &every);
    let row = Row::WorkspaceSet { record: "r".into(), payload: letters(&[1]), provenance: manual() };
    deliver(&row);
    ran(&shell(&session, "r", row));
    kept(&session, &every);

    // A preview of the record's word is refused at execute like a literal one.
    let p2 = propose(&session, "p2", WordSource::Record("r".into()));
    assert_eq!(session.preview().expect("p2").end, None);
    refused(&session, &disk, minted(&session, "x4", Row::Execute { preview: p2 }), codes::ORBIT_WOULD_MOVE);

    // session.undo and session.redo of the record: no orbit moves.
    let row = Row::Undo { hint: session.overview().undo.expect("the record"), confirm: false };
    ran(&shell(&session, "u-r", row));
    kept(&session, &every);
    let row = Row::Redo { hint: session.overview().redo.expect("the record"), confirm: false };
    deliver(&row);
    ran(&shell(&session, "r-r", row));
    kept(&session, &every);

    // job.submit and job.stop: a job reaches no commit row.
    let row = Row::JobSubmit(job("count", vec![Value::int(3)], vec![Input::Current], Lane::Program, JobKind::Query));
    deliver(&row);
    let admitted = shell(&session, "j", row);
    let job = admitted.job.expect("an admitted job");
    finished(&session, &admitted);
    kept(&session, &every);
    let row = Row::JobStop { job };
    deliver(&row);
    let stopped = shell(&session, "js", row);
    assert!(!matches!(stopped.status, Status::Committed(_)), "{stopped:?}");
    kept(&session, &every);

    // protection.set: a release needs its confirmation.
    let row = Row::ProtectionSet { orbits: vec![], confirm: false };
    deliver(&row);
    refused(&session, &disk, envelope(&session, "x5", row), code::CONFIRMATION_REQUIRED);
    // Can fail: every variant was delivered while every orbit was protected.
    assert!(seen.iter().all(|&s| s), "a row variant was not delivered: {seen:?}");

    // A confirmed release, an undo that protects again and a redo that releases again, which needs confirmation.
    ran(&protect(&session, "release", &[]));
    kept(&session, &[]);
    ran(&undo(&session, "u-p"));
    kept(&session, &every);
    let hint = session.overview().redo.expect("the release");
    refused(&session, &disk, minted(&session, "x6", Row::Redo { hint, confirm: false }), code::CONFIRMATION_REQUIRED);
    kept(&session, &every);
    ran(&shell(&session, "r-p", Row::Redo { hint, confirm: true }));
    kept(&session, &[]);

    // Can fail: with the orbits released, the rows refused above move the state.
    committed(&shell(&session, "s0", Row::SwitchLine { line: 0 }));
    assert_ne!(digest(&session), held);
    let moved = digest(&session);
    committed(&commit(&session, "m", &[1]));
    assert_ne!(digest(&session), moved);
}

/// CT-KN5-03 (part: the token half of R-18: one minted token; `Execute` is delivered from 20 threads under distinct
/// command IDs with that token, and as repeats of its own command ID, first while the commit is held in its write and
/// then racing its landing: exactly one commit, `overview().order` up by one, every other delivery refused with the
/// token-used, commit-in-flight or preview-replaced code, every repeat answered from the stored outcome; not run: the
/// shell's held-key repeat guard (chapter 08 section 7, step 1), which exists only in `c600-command` and waits for
/// KN5's placement)
#[test]
fn ct_kn5_03_one_token_commits_once_under_concurrent_deliveries() {
    const THREADS: usize = 20;
    let (session, disk) = session();
    let p = propose(&session, "p", literal(&[1]));
    let end = session.preview().and_then(|q| q.end).expect("p's end state");
    let (before, writes) = (session.overview(), disk.writes());
    // The one token. It is minted before the hold: the stub reads the session core, which the commit holds.
    let c = minted(&session, "C", Row::Execute { preview: p });
    let under = |prefix: &str| -> Vec<Envelope> {
        (0..THREADS)
            .map(|i| {
                let mut e = c.clone();
                e.command = format!("{prefix}{i}");
                e
            })
            .collect()
    };
    let (held_ids, landing_ids) = (under("D"), under("E"));
    let (held_gate, landing_gate) = (Barrier::new(THREADS), Barrier::new(THREADS + 1));

    disk.hold_write(disk.writes());
    let (first, held, landing) = thread::scope(|s| {
        let (session, c) = (&session, &c);
        let worker = s.spawn(move || session.invoke(Origin::Shell, c.clone()));
        disk.wait_held();
        // Wave 1: C was admitted and its commit waits in its write.
        let gate = &held_gate;
        let handles: Vec<_> = held_ids
            .iter()
            .cloned()
            .map(|other| {
                let repeat = c.clone();
                s.spawn(move || {
                    gate.wait();
                    (session.invoke(Origin::Shell, other), session.invoke(Origin::Shell, repeat))
                })
            })
            .collect();
        let held: Vec<(Receipt, Receipt)> = handles.into_iter().map(|h| h.join().expect("a delivery")).collect();
        // Wave 2: the release races the deliveries.
        let gate = &landing_gate;
        let handles: Vec<_> = landing_ids
            .iter()
            .cloned()
            .map(|other| {
                let repeat = c.clone();
                s.spawn(move || {
                    gate.wait();
                    (session.invoke(Origin::Shell, other), session.invoke(Origin::Shell, repeat))
                })
            })
            .collect();
        gate.wait();
        disk.release();
        let landing: Vec<(Receipt, Receipt)> = handles.into_iter().map(|h| h.join().expect("a delivery")).collect();
        (worker.join().expect("the commit"), held, landing)
    });

    assert_eq!(committed(&first).post, end, "the commit adopts the preview's own end state");
    let allowed = [code::TOKEN_USED, code::COMMIT_IN_FLIGHT, code::PREVIEW_REPLACED];
    for (other, repeat) in held.iter().chain(&landing) {
        assert!(matches!(other.status, Status::Refused(_)), "{other:?}");
        assert!(allowed.contains(&code(other)), "{other:?}");
        // Chapter 08 section 7: step 3 refuses a token seen before, ahead of steps 4 and 5.
        assert_eq!(code(other), code::TOKEN_USED, "{other:?}");
        assert!(repeat.status == Status::Accepted || *repeat == first, "{repeat:?}");
        assert_eq!(repeat.request, first.request);
    }
    // Can fail: every repeat of the first wave met the commit in flight, so the deliveries raced it.
    assert!(held.iter().all(|(_, repeat)| repeat.status == Status::Accepted), "{held:?}");

    // Exactly one commit.
    let receipts = std::iter::once(&first).chain(held.iter().chain(&landing).flat_map(|(o, r)| [o, r]));
    let commits: BTreeSet<&str> =
        receipts.filter(|r| matches!(r.status, Status::Committed(_))).map(|r| r.command.as_str()).collect();
    assert_eq!(commits, BTreeSet::from(["C"]));
    let after = session.overview();
    assert_eq!(after.order, before.order + 1, "exactly one commit");
    assert_eq!((after.revision.digest.as_str(), after.commit_busy), (end.as_str(), false));
    assert_eq!(session.preview(), None, "the commit discarded p");
    assert_eq!(disk.writes(), writes + execute_writes());

    // Every delivery is answered from its stored outcome.
    assert_eq!(session.invoke(Origin::Shell, c.clone()), first);
    assert_eq!(session.outcome("C", &first.request), first);
    let stored: Vec<&Receipt> = held.iter().chain(&landing).map(|(other, _)| other).collect();
    for (e, receipt) in held_ids.iter().chain(&landing_ids).zip(stored) {
        assert_eq!(session.invoke(Origin::Shell, e.clone()), *receipt);
        assert_eq!(session.outcome(&e.command, &receipt.request), *receipt);
    }
    assert_eq!(session.overview().order, before.order + 1);
}
