//! Helpers shared by the session tests: the kernel, sessions on memory disks, envelopes delivered by the shell or by
//! a program, the commit path through the shell stub, provenance blocks and checked programs.

#![allow(dead_code)]

use std::path::Path;
use std::sync::{Arc, OnceLock};

use c600_check::check;
use c600_kernel::Kernel;
use c600_run::interp::{load, Loaded};
use c600_run::meter::Amounts;
use c600_run::provenance::{self, Json, Pin};
use c600_session::disk::MemDisk;
use c600_session::{
    CommitRecord, Envelope, Input, JobId, JobKind, JobSpec, Lane, Options, Origin, Overview, Receipt, Row, Session,
    Status, WordSource,
};
use c600_syntax::{lower, parse};

pub const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The kernel of the model in `assets/`, loaded once per test binary.
pub fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    Arc::clone(KERNEL.get_or_init(|| {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
        Arc::new(Kernel::load(&assets).unwrap_or_else(|e| panic!("the model in assets/ does not load: {e}")))
    }))
}

/// A new session on a fresh memory disk. The test keeps a clone of the disk to inject failures and crash it.
pub fn session() -> (Session, MemDisk) {
    session_with(Options::default())
}

pub fn session_with(options: Options) -> (Session, MemDisk) {
    let disk = MemDisk::new();
    let session = Session::create(kernel(), Box::new(disk.clone()), "s1", options).expect("the session is created");
    (session, disk)
}

/// Opens the session a memory disk holds under a new engine epoch.
pub fn reopen(disk: &MemDisk) -> Session {
    Session::open(kernel(), Box::new(disk.clone()), Options::default()).expect("the session reopens")
}

pub fn envelope(session: &Session, command: &str, origin: Origin, row: Row) -> Envelope {
    Envelope {
        session: session.id().to_owned(),
        epoch: session.epoch(),
        command: command.to_owned(),
        origin,
        row,
        token: None,
    }
}

/// Delivers a row from the shell, with a token the shell stub mints for it when it is a commit row.
pub fn shell(session: &Session, command: &str, row: Row) -> Receipt {
    let mut e = envelope(session, command, Origin::Shell, row);
    if e.row.is_commit() {
        e.token = Some(session.shell().mint(command, &e.row).unwrap_or_else(|e| panic!("no token: {e}")));
    }
    session.invoke(e)
}

/// Delivers a row as a program would: through its host, never with a token.
pub fn from_program(session: &Session, command: &str, row: Row) -> Receipt {
    session.invoke(envelope(session, command, Origin::Program, row))
}

/// Proposes a word and returns the preview's ID.
pub fn propose(session: &Session, command: &str, word: WordSource) -> u64 {
    match shell(session, command, Row::Propose { word }).status {
        Status::Completed(c) => c.result.and_then(|r| r.preview).expect("a preview ID"),
        other => panic!("the proposal was refused: {other:?}"),
    }
}

pub fn literal(letters: &[i32]) -> WordSource {
    WordSource::Literal(letters.to_vec())
}

/// Proposes a literal word and executes the preview: one commit, from the shell.
pub fn commit(session: &Session, command: &str, letters: &[i32]) -> Receipt {
    let preview = propose(session, &format!("{command}.preview"), literal(letters));
    shell(session, command, Row::Execute { preview })
}

pub fn undo(session: &Session, command: &str) -> Receipt {
    let hint = session.overview().undo.expect("an action to undo");
    shell(session, command, Row::Undo { hint, confirm: false })
}

pub fn redo(session: &Session, command: &str) -> Receipt {
    let hint = session.overview().redo.expect("an action to redo");
    shell(session, command, Row::Redo { hint, confirm: false })
}

/// The commit record of a committed receipt.
pub fn committed(receipt: &Receipt) -> &CommitRecord {
    match &receipt.status {
        Status::Committed(record) => record,
        other => panic!("not committed: {other:?}"),
    }
}

/// The code of a refused or failed receipt.
pub fn code(receipt: &Receipt) -> &'static str {
    match &receipt.status {
        Status::Refused(e) | Status::Failed(e) => e.code,
        other => panic!("neither refused nor failed: {other:?}"),
    }
}

/// The digest of the current state.
pub fn digest(session: &Session) -> String {
    session.overview().revision.digest
}

/// What survives a reopen: the overview without the epoch and the pending preview.
pub fn durable(session: &Session) -> Overview {
    let mut overview = session.overview();
    overview.revision.epoch = 0;
    overview.pending_preview = None;
    overview
}

/// One kind of durable action: what makes it possible, and the action under test (command `act`).
pub struct Kind {
    pub name: &'static str,
    pub setup: fn(&Session),
    pub act: fn(&Session) -> Receipt,
}

/// The seven kinds of durable action.
pub fn kinds() -> Vec<Kind> {
    vec![
        Kind {
            name: "operation.execute",
            setup: |s| {
                propose(s, "p", literal(&[1, 2]));
            },
            act: |s| {
                let preview = s.preview().expect("a pending preview").id;
                shell(s, "act", Row::Execute { preview })
            },
        },
        Kind {
            name: "session.undo",
            setup: |s| {
                commit(s, "c", &[1]);
            },
            act: |s| undo(s, "act"),
        },
        Kind {
            name: "session.redo",
            setup: |s| {
                commit(s, "c", &[1]);
                undo(s, "u");
            },
            act: |s| redo(s, "act"),
        },
        Kind {
            name: "session.switch_line",
            setup: |s| {
                commit(s, "c", &[1]);
                undo(s, "u");
                commit(s, "c2", &[2]);
            },
            act: |s| shell(s, "act", Row::SwitchLine { line: 0 }),
        },
        Kind {
            name: "session.checkpoint",
            setup: |s| {
                commit(s, "c", &[1]);
            },
            act: |s| shell(s, "act", Row::Checkpoint { name: "k".into() }),
        },
        Kind { name: "workspace.set", setup: |_| {}, act: |s| set_record(s, "act", "r", letters(&[1, 2])) },
        Kind { name: "protection.set", setup: |_| {}, act: |s| protect(s, "act", &[0, 1]) },
    ]
}

/// The writes an action of this kind makes: its frame and the frame's sync, after a checkpoint's new state object
/// and the object's sync.
pub fn writes_of(kind: &Kind) -> u64 {
    if kind.name == "session.checkpoint" {
        4
    } else {
        2
    }
}

/// Damages one byte: it changes and does not become zero, so the damage never looks like the zero fill of a torn
/// append.
pub fn flip(bytes: &mut [u8], at: usize) {
    let mask = 1u8 << (at % 8);
    bytes[at] ^= if bytes[at] == mask { 0x81 } else { mask };
}

/// FP.17's manual block under a fixture capability contract.
pub fn manual() -> Vec<u8> {
    provenance::canonical(&provenance::manual(&Pin::of("fixture-1", b"m6 fixture capability contract\n")))
}

/// A record payload that holds letters.
pub fn letters(list: &[i64]) -> Json {
    Json::List(list.iter().map(|&n| Json::Int(n)).collect())
}

pub fn set_record(session: &Session, command: &str, record: &str, payload: Json) -> Receipt {
    shell(session, command, Row::WorkspaceSet { record: record.to_owned(), payload, provenance: manual() })
}

pub fn protect(session: &Session, command: &str, orbits: &[i64]) -> Receipt {
    shell(session, command, Row::ProtectionSet { orbits: orbits.to_vec(), confirm: true })
}

pub const HEADER: &str = "edition \"6cs-draft-1\";\nmodule m requires [file];\n\n";

/// The test programs, in one module `m`. `tree` does 2^limit calls in bounded memory: at a limit of 60 it does not end
/// within any test.
pub const PROGRAMS: &str = "\
fn tree(level: Nat, limit: Nat) -> Nat effects [] {
    return match exact_less(level, limit) {
        true => exact_add(tree(exact_add(level, 1), limit), tree(exact_add(level, 1), limit));
        _ => 1;
    };
}

export fn count(limit: Nat) -> Nat effects [] {
    return tree(0, limit);
}

export fn state() -> Nat effects [snapshot] {
    return read_snapshot<Nat>(\"state\");
}

export fn moved() -> Nat effects [snapshot] {
    return read_snapshot<Nat>(\"moved\");
}

export fn diff() -> Nat effects [snapshot] {
    return read_snapshot<Nat>(\"diff\");
}

export fn compare() -> Nat effects [snapshot] {
    let a = read_snapshot<Nat>(\"diff\");
    let b = read_snapshot<Nat>(\"moved\");
    let c = read_snapshot<Nat>(\"inputs\");
    return exact_add(exact_add(a, b), c);
}

export fn long_query() -> Nat effects [snapshot] {
    let s = read_snapshot<Nat>(\"state\");
    return exact_add(s, tree(0, 60));
}

export fn peek() -> Word effects [file] {
    return file_read<Word>(\"journal\");
}

export fn save(w: Word) -> Unit effects [file] {
    let a = file_write(\"journal\", w);
    return ();
}

export fn keep(w: Word) -> Unit effects [workspace] {
    return write_workspace<Word>(\"saved\", w);
}

export fn offer(w: Word) -> Text effects [preview] {
    return receipt_id(propose(w, target(\"t\")));
}
";

/// The checked and loaded test programs.
pub fn programs() -> Arc<Loaded> {
    static LOADED: OnceLock<Arc<Loaded>> = OnceLock::new();
    Arc::clone(LOADED.get_or_init(|| {
        let source = format!("{HEADER}{PROGRAMS}");
        let file = parse(source.as_bytes());
        assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
        let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
        let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        Arc::new(load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}")))
    }))
}

/// A job of one exported function of the test programs.
pub fn job(
    function: &str,
    args: Vec<c600_run::value::Value>,
    inputs: Vec<Input>,
    lane: Lane,
    kind: JobKind,
) -> JobSpec {
    JobSpec {
        program: programs(),
        program_digest: "test-programs".to_owned(),
        module: "m".to_owned(),
        function: function.to_owned(),
        args,
        inputs,
        lane,
        kind,
        budget: WIDE,
    }
}

/// Submits a job from the shell and returns its ID.
pub fn submit(session: &Session, command: &str, spec: JobSpec) -> JobId {
    let receipt = shell(session, command, Row::JobSubmit(spec));
    match receipt.status {
        Status::Running(job) => job,
        other => panic!("the job was not admitted: {other:?}"),
    }
}

pub fn stop(session: &Session, command: &str, job: JobId) -> Receipt {
    shell(session, command, Row::JobStop { job })
}

/// The output bytes of a natural number a job returns.
pub fn nat(n: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    c600_run::encode::encode(&c600_run::value::Value::int(n), &mut bytes).expect("a canonical natural");
    bytes
}

/// Submits a job from the shell and returns its admission receipt, whatever it is.
pub fn try_submit(session: &Session, command: &str, spec: JobSpec) -> Receipt {
    shell(session, command, Row::JobSubmit(spec))
}

/// The receipt a job's command ends with, once the job is terminal.
pub fn finished(session: &Session, admitted: &Receipt) -> Receipt {
    let job = admitted.job.expect("an admitted job");
    session.wait_job(job).expect("the job's result");
    session.outcome(&admitted.command, &admitted.request)
}
