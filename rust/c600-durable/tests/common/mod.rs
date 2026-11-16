//! Helpers shared by the C7 tests: the kernel, sessions on memory disks and in fresh directories, envelopes from the
//! shell or a program, the commit path through M6's shell stub, the fixture's policy pins, a record read back from
//! the journal, and checked programs for jobs.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::check;
use c600_kernel::Kernel;
use c600_run::boundary::Pins;
use c600_run::interp::{load, Loaded};
use c600_run::meter::Amounts;
use c600_run::provenance::{self, Json, Pin};
use c600_run::value::Value;
use c600_session::disk::MemDisk;
use c600_session::journal;
use c600_session::{
    CommitRecord, Envelope, Input, JobKind, JobSpec, Lane, Options, Origin, Overview, Receipt, RecordValue, Row,
    Session, Status, WordSource,
};
use c600_syntax::{lower, parse};

pub const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The name of M6's journal file (`c600-session`'s `store.rs`; a draft layout).
pub const JOURNAL: &str = "journal";

/// The kernel of the model in `assets/`, loaded once per test binary.
pub fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    Arc::clone(KERNEL.get_or_init(|| {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
        Arc::new(Kernel::load(&assets).unwrap_or_else(|e| panic!("the model in assets/ does not load: {e}")))
    }))
}

/// A new session on a fresh memory disk. The test keeps a clone of the disk to crash and inspect it.
pub fn session() -> (Session, MemDisk) {
    let disk = MemDisk::new();
    let session =
        Session::create(kernel(), Box::new(disk.clone()), "s1", Options::default()).expect("the session is created");
    (session, disk)
}

/// Opens the session a memory disk holds, under a new engine epoch.
pub fn reopen(disk: &MemDisk) -> Session {
    Session::open(kernel(), Box::new(disk.clone()), Options::default()).expect("the session reopens")
}

/// A fresh directory under Cargo's temporary directory for integration tests, removed before use and when dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("c600-durable-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a fresh directory");
        TempDir(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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

/// The commit record of a committed receipt.
pub fn committed(receipt: &Receipt) -> &CommitRecord {
    match &receipt.status {
        Status::Committed(record) => record,
        other => panic!("not committed: {other:?}"),
    }
}

/// The durable action a completed workspace row made.
pub fn action(receipt: &Receipt) -> u64 {
    match &receipt.status {
        Status::Completed(c) => c.result.as_ref().and_then(|r| r.action).expect("a durable action"),
        other => panic!("not completed: {other:?}"),
    }
}

/// The code of a refused or failed receipt.
pub fn code(receipt: &Receipt) -> &'static str {
    match &receipt.status {
        Status::Refused(e) | Status::Failed(e) => e.code,
        other => panic!("neither refused nor failed: {other:?}"),
    }
}

/// What survives a reopen: the overview without the epoch and the pending preview.
pub fn durable(session: &Session) -> Overview {
    let mut overview = session.overview();
    overview.revision.epoch = 0;
    overview.pending_preview = None;
    overview
}

/// FP.18's fixture capability contract, held by the policy in these tests.
pub fn contract() -> Pin {
    Pin::of("fixture-1", b"c7 fixture capability contract 1\n")
}

pub fn checker() -> Pin {
    Pin::of("m1-draft", b"c600-check fixture\n")
}

pub fn pins() -> Pins {
    Pins { contracts: vec![contract()], checkers: vec![checker()] }
}

/// A hand-entered value's block (FP.17, BND.23).
pub fn manual() -> Vec<u8> {
    provenance::canonical(&provenance::manual(&contract()))
}

/// A record payload that holds letters, the form M6's `operation.preview` reads.
pub fn word_payload(letters: &[i32]) -> Json {
    Json::List(letters.iter().map(|&l| Json::Int(i64::from(l))).collect())
}

/// A `workspace.set` from the shell, with a hand-entered block.
pub fn set_record(session: &Session, command: &str, record: &str, letters: &[i32]) -> Receipt {
    let row = Row::WorkspaceSet { record: record.to_owned(), payload: word_payload(letters), provenance: manual() };
    shell(session, command, row)
}

/// The value the last `record` action of a journal sets for a record, read from the journal's bytes in M6's draft
/// frame form (`c600-session`'s `wire.rs`). Undo and redo actions are not followed. M6 has no public record reader
/// (crate documentation, Drafts: reading a record back).
pub fn journal_record(bytes: &[u8], name: &str) -> Option<RecordValue> {
    let text = |value: Option<&Json>| value.and_then(|v| v.as_text().map(str::to_owned));
    let mut found = None;
    for payload in journal::scan(bytes).payloads {
        let Some(act) = provenance::parse(&payload).and_then(|frame| frame.get("act").cloned()) else {
            continue;
        };
        if text(act.get("type")).as_deref() != Some("record") || text(act.get("name")).as_deref() != Some(name) {
            continue;
        }
        found = match act.get("value") {
            Some(Json::Null) | None => None,
            Some(value) => Some(RecordValue {
                payload: value.get("payload")?.clone(),
                provenance: text(value.get("provenance"))?.into_bytes(),
            }),
        };
    }
    found
}

/// The canonical encoding of a value.
pub fn encoded(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    c600_run::encode::encode(value, &mut bytes).expect("a canonical value");
    bytes
}

/// The job programs, in one module `m`: a state query and a workspace write.
pub const PROGRAMS: &str = "\
edition \"6cs-draft-1\";
module m requires [];

export fn state() -> Nat effects [snapshot] {
    return read_snapshot<Nat>(\"state\");
}

export fn keep(w: Word) -> Unit effects [workspace] {
    return write_workspace<Word>(\"saved\", w);
}
";

/// The checked and loaded job programs.
pub fn programs() -> Arc<Loaded> {
    static LOADED: OnceLock<Arc<Loaded>> = OnceLock::new();
    Arc::clone(LOADED.get_or_init(|| {
        let file = parse(PROGRAMS.as_bytes());
        assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
        let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
        let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        Arc::new(load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}")))
    }))
}

/// A job of one exported function of the job programs, on the current revision.
pub fn job(function: &str, args: Vec<Value>, kind: JobKind) -> JobSpec {
    JobSpec {
        program: programs(),
        program_digest: "c7-job-programs".to_owned(),
        module: "m".to_owned(),
        function: function.to_owned(),
        args,
        inputs: vec![Input::Current],
        lane: Lane::Program,
        kind,
        budget: WIDE,
    }
}

/// The receipt a job's command ends with, once the job is terminal.
pub fn finished(session: &Session, admitted: &Receipt) -> Receipt {
    let job = admitted.job.expect("an admitted job");
    session.wait_job(job).expect("the job's result");
    session.outcome(&admitted.command, &admitted.request)
}
