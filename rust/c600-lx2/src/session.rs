//! LX-2's session-level conditions for the metered and forced-kill trials (RTE.42 conditions 1 and 2, RTE.43): each
//! trial runs the same command sequence with a session open, and the fault trials are compared with the control.
//!
//! The session is KN5's command layer over M6's journal (`JournalStore`) on a file-system disk in a fresh folder the
//! caller names (`FsDisk`: the product's disk, with the session lock), with the shell and the headless display. The
//! sequence:
//!
//! 1. the solver previews, presents and commits the word [`BEFORE`] from the shell;
//! 2. the trial's job runs in the runtime child given by the caller, through the product's own bridge
//!    (`c600_tools::bridge::Bridge`, caller `Runtime`) and boundary onto the layer. Its proposal of SCENARIO.json's
//!    runtime word becomes the layer's pending preview before the fault. For `forced_kill` the child's process is
//!    killed inside that perform, the barrier;
//! 3. the solver presents and commits the run's preview from the shell;
//! 4. the solver previews, presents and commits the word [`AFTER`].
//!
//! A trial agrees when the job ends with its expected typed outcome, every solver command of steps 3 and 4 is accepted
//! (the session stays open), the journal's records and the state hash equal the control's, and a reopen of the disk
//! replays the same state hash. RTE.43: "unchanged" concerns the fault's effect, so the control runs the same commands
//! without the fault.
//!
//! Not here: conditions 3 and 4 (the renderer gate, LX-4's echo bar and charter E2 during fault windows), which need
//! the shell's frame loop on the owner's machine, and the native faults of option (a).

use std::fs;
use std::path::Path;
use std::sync::Arc;

use c600_check::Profile;
use c600_command::{CommandLayer, HeadlessFrames, Secret, Shell, Status, Table};
use c600_digest::sha256_hex;
use c600_kernel::json;
use c600_kernel::{Kernel, ProtectedSet};
use c600_run::boundary::{Boundary, Store as Shown};
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::value::Value;
use c600_runtime::{Config, Job, JobOptions, Killer, Runtime};
use c600_session::disk::FsDisk;
use c600_session::{journal, JournalStore};
use c600_tools::bridge::{base, word_args, Bridge};
use c600_tools::tool::{BoundaryClass, Manifest, Tool};

use crate::{observe, Observed, Trial, DEADLINE};

/// The word the solver commits before the job.
pub const BEFORE: [i32; 2] = [1, -2];
/// The word the solver commits after the job.
pub const AFTER: [i32; 1] = [3];
/// The run's name: the proposal IDs and the runtime caller carry it.
pub const RUN: &str = "lx2-run";
/// The name of M6's journal file on the disk.
const JOURNAL: &str = "journal";
/// The per-launch secret of the layer and the shell (a fixed test value).
const SECRET: [u8; 32] = [6; 32];
/// The commit chord.
const COMMIT: &str = "Ctrl+Shift+Enter";

/// One trial's session-level record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRecord {
    pub trial: Trial,
    pub observed: Observed,
    /// Every solver command after the job was accepted.
    pub open: bool,
    /// The journal's records after the sequence, as canonical JSON.
    pub journal: Vec<String>,
    /// The state hash (`Digest<labels>`) after the sequence.
    pub state: String,
    /// The state hash a reopen of the disk replays.
    pub replayed: Option<String>,
    /// Child processes started over the trial.
    pub spawns: u64,
}

impl SessionRecord {
    /// Whether the record meets RTE.42 conditions 1 and 2 against `control`, the control trial's record.
    pub fn agrees(&self, control: &SessionRecord) -> bool {
        self.observed == self.trial.expected()
            && self.open
            && self.replayed.as_deref() == Some(self.state.as_str())
            && self.journal == control.journal
            && self.state == control.state
            && !self.journal.is_empty()
    }
}

/// The trial's program as a tool the built-in column admits, with the 600-cell-Full tool's classes and row.
pub fn tool(trial: Trial) -> Result<(Tool, String), String> {
    let source = trial.source();
    let manifest = Manifest {
        tool: "LX-2 fault program",
        candidate: "LX-2 measurement (never shipped)",
        module: "lx2",
        entry: "work",
        classes: &[BoundaryClass::Analysis, BoundaryClass::Checking],
        rows: &["operation.preview"],
        source_sha256: sha256_hex(source.as_bytes()),
        principle_record: false,
    };
    Ok((Tool::new(&source, manifest)?, source))
}

/// The trial's job as the process host sends a tool's run (`c600_tools::process::job`), with the trial's limits.
pub fn job(trial: Trial, source: &str) -> Job {
    let mut options = JobOptions::new(trial.limits());
    options.profile = Profile::ToolAuthoring;
    options.source_ranges = false;
    options.record = false;
    Job {
        sources: vec![source.as_bytes().to_vec()],
        module: "lx2".into(),
        entry: "work".into(),
        args: vec![Value::word(&crate::RUNTIME_WORD)],
        options,
    }
}

/// The boundary as the run's host, with the forced kill at the barrier: the child is killed right after the first
/// perform has reached the layer.
struct KillAtBarrier<'h> {
    inner: &'h mut dyn Host,
    killer: Option<Killer>,
}

impl Host for KillAtBarrier<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.inner.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let answer = self.inner.perform(request);
        if let Some(killer) = self.killer.take() {
            killer.kill();
        }
        answer
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.inner.sink(sink, value, bytes)
    }
}

/// Builds, presents and commits preview `id` from the shell; whether the commit was accepted.
fn commit_preview(layer: &mut CommandLayer<JournalStore>, shell: &mut Shell, id: u64) -> bool {
    if layer.run_jobs() != 1 || HeadlessFrames::after(layer).present(shell, layer, id).is_err() {
        return false;
    }
    matches!(shell.press(layer, COMMIT, false), Ok(Some(receipt)) if matches!(receipt.status, Status::Committed(_)))
}

/// Previews, presents and commits `word` from the shell; whether every step was accepted.
fn commit_word(layer: &mut CommandLayer<JournalStore>, shell: &mut Shell, word: &[i32]) -> bool {
    match shell.invoke(layer, "operation.preview", word_args(word)) {
        Ok(receipt) => match receipt.status {
            Status::Running(id) => commit_preview(layer, shell, id),
            _ => false,
        },
        Err(_) => false,
    }
}

/// The journal's records in the session folder `folder`, as canonical JSON.
fn journal_rows(folder: &Path) -> Vec<String> {
    let Ok(bytes) = fs::read(folder.join(JOURNAL)) else { return Vec::new() };
    let row = |payload: &Vec<u8>| match json::parse(payload) {
        Ok(record) => record.canonical(),
        Err(_) => format!("unparsed {}", sha256_hex(payload)),
    };
    journal::scan(&bytes).payloads.iter().map(row).collect()
}

/// Runs one trial's command sequence against the child binary `runtime`, with the session in `folder`, which must be
/// new or empty.
pub fn run_session_trial(runtime: &Path, kernel: &Arc<Kernel>, folder: &Path, trial: Trial) -> SessionRecord {
    let failed = |observed: Observed, spawns: u64| SessionRecord {
        trial,
        observed,
        open: false,
        journal: Vec::new(),
        state: String::new(),
        replayed: None,
        spawns,
    };
    let (tool, source) = match tool(trial) {
        Ok(admitted) => admitted,
        Err(reason) => return failed(Observed::Other(format!("not admitted: {reason}")), 0),
    };
    if fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_some()) {
        return failed(Observed::Other("the session folder is not empty".into()), 0);
    }
    let store = match FsDisk::new(folder)
        .map_err(|e| format!("{e}"))
        .and_then(|disk| JournalStore::create(Box::new(disk)).map_err(|error| format!("{error:?}")))
    {
        Ok(store) => store,
        Err(error) => return failed(Observed::Other(format!("no session: {error}")), 0),
    };
    let (secret, none) = (Secret::fixed(SECRET), ProtectedSet::none());
    let mut layer = CommandLayer::new(kernel.clone(), Table::h08(), secret, store, "lx2", kernel.solved(), none);
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let mut rt = Runtime::new(Config::new(runtime));
    if let Err(fault) = rt.start() {
        return failed(Observed::Fault(fault.kind, fault.code()), rt.spawns());
    }
    let mut open = commit_word(&mut layer, &mut shell, &BEFORE);
    let read = base(layer.revision());
    let killer = if trial == Trial::ForcedKill { rt.killer() } else { None };
    let job = job(trial, &source);
    let (result, preview) = {
        let mut bridge = Bridge::new(&mut layer, RUN);
        let mut shown = Shown::default();
        let result = {
            let mut boundary = Boundary::new(&mut bridge, &mut shown, tool.context(RUN, &read));
            let mut host = KillAtBarrier { inner: &mut boundary, killer };
            rt.run(&job, &mut host, Some(DEADLINE))
        };
        let preview = bridge.sent.iter().find_map(|sent| match sent.receipt.status {
            Status::Running(id) => Some(id),
            _ => None,
        });
        (result, preview)
    };
    let observed = observe(result);
    open &= preview.is_some_and(|id| commit_preview(&mut layer, &mut shell, id));
    open &= commit_word(&mut layer, &mut shell, &AFTER);
    let spawns = rt.spawns();
    rt.stop();
    let state = layer.snapshot().digest().to_owned();
    drop(layer);
    // The journal as the sequence left it: a reopen journals its own fresh epoch.
    let journal = journal_rows(folder);
    let replayed = FsDisk::new(folder)
        .ok()
        .and_then(|disk| JournalStore::open(Box::new(disk)).ok())
        .and_then(|store| CommandLayer::open(kernel.clone(), Table::h08(), Secret::fixed(SECRET), store).ok())
        .map(|layer| layer.snapshot().digest().to_owned());
    SessionRecord { trial, observed, open, journal, state, replayed, spawns }
}

/// The session-level records as a JSON array for the trial record, each row against `records[0]`, the control. The
/// journal is named by its row count and the SHA-256 of its rows joined by newlines; the folder is never named.
pub fn sessions_json(records: &[SessionRecord]) -> String {
    let Some(control) = records.iter().find(|r| r.trial == Trial::Control) else { return "[]".into() };
    let rows: Vec<String> = records
        .iter()
        .map(|r| {
            format!(
                "    {{\"trial\": \"{}\", \"observed\": \"{}\", \"open\": {}, \"journal_rows\": {}, \"journal_sha256\": \"{}\", \"state\": \"{}\", \"replayed_equal\": {}, \"agrees_with_control\": {}}}",
                r.trial.name(),
                r.observed.describe().replace(['"', '\\'], "_"),
                r.open,
                r.journal.len(),
                sha256_hex(r.journal.join("\n").as_bytes()),
                r.state,
                r.replayed.as_deref() == Some(r.state.as_str()),
                r.agrees(control)
            )
        })
        .collect();
    format!("[\n{}\n  ]", rows.join(",\n"))
}

/// Whether every session-level record agrees with the control among them; true when none ran, false when records ran
/// without a control.
pub fn all_agree(records: &[SessionRecord]) -> bool {
    match records.iter().find(|r| r.trial == Trial::Control) {
        Some(control) => records.iter().all(|r| r.agrees(control)),
        None => records.is_empty(),
    }
}
