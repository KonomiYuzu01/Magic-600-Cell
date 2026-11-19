//! View jobs (C9 U8, a draft): programs that the language runtime runs for the one view ([`crate::m6h_run_view`]),
//! the editor's drafts ([`crate::m6h_view_draft`]), and the records that [`crate::m6h_view_result`] reads.
//!
//! - The runtime moves to one worker thread with the first view job ([`Worker`]). The worker runs the jobs in order and
//!   sends each end back. The handle's thread takes the ends inside `m6h_pump` and `m6h_frame`, and only there do they
//!   reach the display host; the worker never touches the handle.
//! - A run serves no host row ([`NoHost`]). c600-run defines no named read-only row of the adopted state (the
//!   `labels` and `positions` rows are c600-tools' and need a command layer), so a program that reads state, publishes
//!   or proposes is refused at admission (no capability is granted) or stopped with `LG3:capability.not_granted`.
//! - A run that completes delivers the draft's reference publication of the revision it was scheduled at, with the
//!   plan dependency `program` set to the hex SHA-256 of its source text, through `Host::deliver`. Its value is not
//!   drawn yet; [`M6hViewResult::value_sha256`] names its canonical bytes.
//! - The `program` dependency follows the editor: the newest digest the shell named, by a run or a draft. A draft
//!   never cancels a run. While a delivered run is being prepared, the dependency keeps that run's digest, and a
//!   draft's digest takes its place once the run has ended: a run of a new text supersedes the draft, and the draft
//!   keeps the view stale until a run of its own text publishes.
//! - A twist replaces the revision a run read: a run not yet delivered ends `KN7:cancelled.revision_replaced` before
//!   the twist's publication supersedes it, and a delivered one ends so in `Host::adopt`.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use c600_digest::{hex, sha256};
use c600_display::plan::CommandId;
use c600_display::{CancelReason, Status as CommandStatus};
use c600_kernel::json::Value;
use c600_run::interp::NoHost;
use c600_run::meter::code::{CANCELLED_BY_USER, CANCELLED_REVISION_REPLACED};
use c600_run::meter::{Interruption, Outcome};
use c600_runtime::parent::RUNTIME_LOST;
use c600_runtime::{Canceller, Ended, Fault, Job, JobOptions, Killer, Refused, Runtime, Stage};

use crate::{
    host_error, Failure, M6hViewResult, ShellHost, M6H_E_RUNTIME, M6H_VIEW_KIND_DRAFT, M6H_VIEW_KIND_RUN,
    M6H_VIEW_STATUS_PENDING, M6H_VIEW_STATUS_PUBLISHED, M6H_VIEW_STATUS_REFUSED, VIEW, VIEW_ALLOWANCE, VIEW_DEADLINE,
    VIEW_LIMITS,
};

/// The plan dependency that names the program a publication came from.
pub const PROGRAM: &str = "program";
/// The records a handle keeps; past it, the oldest finished record goes, else the oldest one not pending.
pub const RECORDS: usize = 1024;
/// The submitted frames a handle remembers until the core confirms them or a newer frame.
pub const FRAME_NOTES: usize = 1024;
/// A draft code, outside chapter 13's register: a newer run superseded this one.
pub const SUPERSEDED: &str = "m6h:cancelled.superseded";

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One view job for the worker: the run's ticket and the job.
pub struct ViewJob {
    pub ticket: u64,
    pub job: Job,
}

/// How the worker ended a job.
pub enum JobEnd {
    Ended(Box<Ended>),
    Fault(Fault),
    /// The job was cancelled before it reached the runtime.
    Skipped,
}

#[derive(Default)]
struct Shared {
    /// The handle is being destroyed: the worker takes no further job.
    closing: bool,
    /// The ticket of the job in the runtime, 0 between jobs, and the handles on its child.
    running: u64,
    killer: Option<Killer>,
    canceller: Option<Canceller>,
    /// Every job up to this ticket is cancelled, with this interruption.
    cancelled: u64,
    why: Option<Interruption>,
    /// The running job whose interruption reached the child.
    interrupted: u64,
}

/// The worker thread that owns the runtime from the first view job on.
pub struct Worker {
    jobs: Option<Sender<ViewJob>>,
    ends: Receiver<(u64, JobEnd)>,
    shared: Arc<Mutex<Shared>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    /// Moves `runtime`, whose child the program `program` starts, to a new worker thread.
    pub fn start(runtime: Runtime, program: PathBuf) -> Result<Worker, String> {
        let (jobs, queue) = mpsc::channel();
        let (sender, ends) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let state = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("c600-shell-view-jobs".into())
            .spawn(move || work(runtime, program, queue, sender, state))
            .map_err(|e| format!("no thread for the view jobs: {e}"))?;
        Ok(Worker { jobs: Some(jobs), ends, shared, thread: Some(thread) })
    }

    /// Queues a job; false when the worker has stopped.
    pub fn submit(&self, job: ViewJob) -> bool {
        self.jobs.as_ref().is_some_and(|jobs| jobs.send(job).is_ok())
    }

    /// Cancels every job up to `ticket`: a queued one is skipped, and the running one is interrupted.
    pub fn cancel(&self, ticket: u64, why: Interruption) {
        let mut s = lock(&self.shared);
        if ticket > s.cancelled {
            s.cancelled = ticket;
            s.why = Some(why);
        }
        interrupt(&mut s);
    }

    /// The jobs that ended since the last call, and whether the worker has stopped. An interruption that found no
    /// run in progress yet is sent again.
    pub fn ends(&self) -> (Vec<(u64, JobEnd)>, bool) {
        interrupt(&mut lock(&self.shared));
        let mut ends = Vec::new();
        loop {
            match self.ends.try_recv() {
                Ok(end) => ends.push(end),
                Err(TryRecvError::Empty) => return (ends, false),
                Err(TryRecvError::Disconnected) => return (ends, true),
            }
        }
    }

    /// The ticket of the job in the runtime, 0 between jobs.
    pub fn running(&self) -> u64 {
        lock(&self.shared).running
    }
}

/// Interrupts the running job if it is cancelled and no interruption reached it yet. The lock keeps the worker from
/// starting another job meanwhile, so the interruption reaches no other run.
fn interrupt(s: &mut Shared) {
    if s.running == 0 || s.running > s.cancelled || s.interrupted == s.running {
        return;
    }
    let why = s.why.unwrap_or(Interruption::ByUser);
    if s.canceller.as_ref().is_some_and(|c| c.cancel(why)) {
        s.interrupted = s.running;
    }
}

impl Drop for Worker {
    /// Ends the child (a job in flight ends as a lost runtime) and waits for the worker. The worker restarts no child
    /// inside a run, and takes no job after this.
    fn drop(&mut self) {
        {
            let mut s = lock(&self.shared);
            s.closing = true;
            if let Some(killer) = s.killer.take() {
                killer.kill();
            }
        }
        drop(self.jobs.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The worker: runs the jobs in order against no host and sends each end back.
fn work(
    mut runtime: Runtime,
    program: PathBuf,
    queue: Receiver<ViewJob>,
    ends: Sender<(u64, JobEnd)>,
    shared: Arc<Mutex<Shared>>,
) {
    for ViewJob { ticket, job } in queue {
        {
            let s = lock(&shared);
            if s.closing {
                break;
            }
            if ticket <= s.cancelled {
                if ends.send((ticket, JobEnd::Skipped)).is_err() {
                    break;
                }
                continue;
            }
        }
        // The worker starts the child itself, outside the lock, so that a destroy never waits for a start.
        runtime.config_mut().program = program.clone();
        if let Err(fault) = runtime.start() {
            if ends.send((ticket, JobEnd::Fault(fault))).is_err() {
                break;
            }
            continue;
        }
        {
            let mut s = lock(&shared);
            if s.closing {
                break;
            }
            if ticket <= s.cancelled {
                drop(s);
                if ends.send((ticket, JobEnd::Skipped)).is_err() {
                    break;
                }
                continue;
            }
            s.running = ticket;
            s.killer = runtime.killer();
            s.canceller = runtime.canceller();
        }
        // No restart inside the run: a child that a destroy killed stays dead, and the run ends as a spawn fault.
        runtime.config_mut().program = PathBuf::new();
        let end = match runtime.run(&job, &mut NoHost, Some(VIEW_DEADLINE)) {
            Ok(ended) => JobEnd::Ended(Box::new(ended)),
            Err(fault) => JobEnd::Fault(fault),
        };
        {
            let mut s = lock(&shared);
            s.running = 0;
            s.killer = None;
            s.canceller = None;
        }
        if ends.send((ticket, end)).is_err() {
            break;
        }
    }
    // Dropping the runtime ends its child.
}

/// What [`crate::m6h_view_result`] reports for one ticket.
#[derive(Clone, Debug)]
pub struct Record {
    pub kind: u32,
    pub status: u32,
    pub revision: u64,
    pub digest: [u8; 32],
    pub frame: u64,
    pub cleared: u64,
    pub value: Option<[u8; 32]>,
    /// A refused run's `{"code", "reason"}` JSON.
    pub refusal: String,
    /// A draft's: the sequence of the last frame submitted before it; only later frames count.
    pub after: u64,
}

impl Record {
    /// Whether no later frame changes the record.
    fn done(&self) -> bool {
        match (self.kind, self.status) {
            (_, M6H_VIEW_STATUS_REFUSED) => true,
            (M6H_VIEW_KIND_DRAFT, _) => self.cleared != 0,
            (_, M6H_VIEW_STATUS_PUBLISHED) => self.frame != 0,
            _ => false,
        }
    }

    pub fn row(&self) -> M6hViewResult {
        M6hViewResult {
            status: self.status,
            kind: self.kind,
            revision: self.revision,
            source_digest: self.digest,
            frame: self.frame,
            cleared_frame: self.cleared,
            has_value: u32::from(self.value.is_some()),
            reserved: 0,
            value_sha256: self.value.unwrap_or([0; 32]),
        }
    }
}

/// A submitted frame: its sequence, the command of the publication it drew (0 for none), and whether that output was
/// stale (`Host::stale`).
#[derive(Clone, Copy, Debug)]
struct FrameNote {
    sequence: u64,
    command: u64,
    stale: bool,
}

/// The run that has not ended yet; a newer run supersedes it.
#[derive(Clone, Debug)]
pub struct PendingRun {
    pub ticket: u64,
    pub revision: u64,
    /// The hex SHA-256 of its source text.
    pub digest: String,
    /// Whether its publication went to `Host::deliver`.
    pub delivered: bool,
}

/// A handle's view jobs.
#[derive(Default)]
pub struct Views {
    pub worker: Option<Worker>,
    pub records: BTreeMap<u64, Record>,
    frames: VecDeque<FrameNote>,
    pub run: Option<PendingRun>,
    /// The `program` value the display host holds, and the newest digest the shell named.
    program: Option<String>,
    editor: Option<String>,
    /// The sequence of the newest submitted frame.
    last_sequence: u64,
    /// A run ended without a publication: the reference publication of the adopted revision may be due again.
    republish: bool,
}

impl Views {
    fn insert(&mut self, ticket: u64, record: Record) {
        self.records.insert(ticket, record);
        if self.records.len() > RECORDS {
            // A pending run's record stays: at most one run is pending, so another record can go.
            let old = self
                .records
                .iter()
                .find(|(_, r)| r.done())
                .or_else(|| self.records.iter().find(|(_, r)| r.status != M6H_VIEW_STATUS_PENDING))
                .map(|(t, _)| *t);
            if let Some(old) = old {
                self.records.remove(&old);
            }
        }
    }

    fn refuse(&mut self, ticket: u64, code: &str, reason: String) {
        if let Some(record) = self.records.get_mut(&ticket) {
            record.status = M6H_VIEW_STATUS_REFUSED;
            let fields = [("code", code.to_owned()), ("reason", reason)];
            let object = fields.into_iter().map(|(k, v)| (k.to_owned(), Value::String(v))).collect();
            record.refusal = Value::Object(object).canonical();
        }
        self.republish = true;
    }

    /// Notes a submitted frame.
    pub fn submitted(&mut self, sequence: u64, command: u64, stale: bool) {
        self.last_sequence = sequence;
        self.frames.push_back(FrameNote { sequence, command, stale });
        while self.frames.len() > FRAME_NOTES {
            self.frames.pop_front();
        }
    }

    /// The core confirmed the frame `sequence`. Older frames not yet confirmed never will be (RM.26), so their notes
    /// go too.
    pub fn confirmed(&mut self, sequence: u64) {
        let Some(note) = self.frames.iter().find(|n| n.sequence == sequence).copied() else { return };
        for (ticket, r) in self.records.iter_mut() {
            match r.kind {
                M6H_VIEW_KIND_RUN => {
                    if r.status == M6H_VIEW_STATUS_PUBLISHED && r.frame == 0 && note.command == *ticket {
                        r.frame = sequence;
                    }
                }
                _ if r.frame == 0 => {
                    if note.stale && sequence > r.after {
                        r.frame = sequence;
                    }
                }
                _ => {
                    if r.cleared == 0 && sequence > r.frame && !note.stale {
                        r.cleared = sequence;
                    }
                }
            }
        }
        while self.frames.front().is_some_and(|n| n.sequence <= sequence) {
            self.frames.pop_front();
        }
    }
}

/// The code and reason of a refusal before the run's first metered operation.
fn refusal(refused: &Refused) -> (String, String) {
    let stage = match refused.stage {
        Stage::Parse => "parse",
        Stage::Lower => "lower",
        Stage::Admit => "admit",
        Stage::Load => "load",
        Stage::Start => "start",
    };
    let code = refused.codes.first().cloned().unwrap_or_else(|| format!("m6h:refused.{stage}"));
    (code, format!("the runtime refused the program at its {stage} stage: {}", refused.detail))
}

impl ShellHost {
    /// [`crate::m6h_run_view`]: schedules a view job for `source` and hands it to the worker.
    pub(crate) fn run_view(&mut self, source: &str, entry: &str) -> Result<u64, Failure> {
        if self.views.worker.is_none() {
            let (runtime, program) = self
                .runtime
                .take()
                .ok_or_else(|| (M6H_E_RUNTIME, "no runtime is loaded: m6h_load_runtime starts it".to_owned()))?;
            self.views.worker = Some(Worker::start(runtime, program).map_err(|e| (M6H_E_RUNTIME, e))?);
        }
        let digest = sha256(source.as_bytes());
        let text = hex(&digest);
        let module = c600_syntax::parse(source.as_bytes()).module.map(|m| m.name.text).unwrap_or_default();
        let ticket = self.next_command;
        self.next_command += 1;
        let receipt = self.host.schedule(CommandId(ticket), VIEW, VIEW_ALLOWANCE);
        if let CommandStatus::Refused(error) | CommandStatus::Failed(error) = receipt.status {
            return Err(host_error(error));
        }
        // The schedule superseded the view's pending command, which may be an older run.
        if let Some(old) = self.views.run.take() {
            self.close_run(old.ticket);
        }
        self.views.insert(
            ticket,
            Record {
                kind: M6H_VIEW_KIND_RUN,
                status: M6H_VIEW_STATUS_PENDING,
                revision: self.revision,
                digest,
                frame: 0,
                cleared: 0,
                value: None,
                refusal: String::new(),
                after: 0,
            },
        );
        self.views.run = Some(PendingRun { ticket, revision: self.revision, digest: text.clone(), delivered: false });
        // The editor names the new text: output of another text is stale until this run publishes.
        self.views.editor = Some(text.clone());
        self.set_program(&text);
        // The draft's run options: the declared limits, no rows kept, the tool-authoring profile and no capability.
        let job = Job {
            sources: vec![source.as_bytes().to_vec()],
            module,
            entry: entry.to_owned(),
            args: Vec::new(),
            options: JobOptions { record: false, ..JobOptions::new(VIEW_LIMITS) },
        };
        let submitted = self.views.worker.as_ref().is_some_and(|w| w.submit(ViewJob { ticket, job }));
        if !submitted {
            self.end_run(RUNTIME_LOST, "the view jobs' worker has stopped".to_owned(), Interruption::ByUser);
        }
        Ok(ticket)
    }

    /// [`crate::m6h_view_draft`]: the editor names `digest`.
    pub(crate) fn view_draft(&mut self, digest: [u8; 32]) -> u64 {
        let text = hex(&digest);
        let ticket = self.next_command;
        self.next_command += 1;
        self.views.editor = Some(text.clone());
        // A delivered run that is being prepared keeps the digest it read until it ends; no draft cancels a run.
        if !self.views.run.as_ref().is_some_and(|r| r.delivered) {
            self.set_program(&text);
        }
        let after = self.views.last_sequence;
        self.views.insert(
            ticket,
            Record {
                kind: M6H_VIEW_KIND_DRAFT,
                status: M6H_VIEW_STATUS_PUBLISHED,
                revision: self.revision,
                digest,
                frame: 0,
                cleared: 0,
                value: None,
                refusal: String::new(),
                after,
            },
        );
        ticket
    }

    fn set_program(&mut self, digest: &str) {
        if self.views.program.as_deref() != Some(digest) {
            self.host.set_dependency(PROGRAM, digest);
            self.views.program = Some(digest.to_owned());
        }
    }

    /// Takes the ends the worker sent and delivers the completed runs (the caller's thread, in `m6h_pump` and
    /// `m6h_frame`).
    pub(crate) fn collect_views(&mut self) {
        let Some(worker) = &self.views.worker else { return };
        let (ends, stopped) = worker.ends();
        for (ticket, end) in ends {
            let Some(run) = self.views.run.clone().filter(|r| r.ticket == ticket && !r.delivered) else { continue };
            let why = Interruption::ByUser;
            let ended = match end {
                JobEnd::Skipped => {
                    self.end_run(SUPERSEDED, "the job was cancelled before it ran".to_owned(), why);
                    continue;
                }
                JobEnd::Fault(fault) => {
                    self.end_run(fault.code(), format!("the runtime faulted: {fault}"), why);
                    continue;
                }
                JobEnd::Ended(ended) => *ended,
            };
            match ended {
                Ended::Refused(refused) => {
                    let (code, reason) = refusal(&refused);
                    self.end_run(&code, reason, why);
                }
                Ended::Ran(ran) if ran.summary.outcome != Outcome::Completed => {
                    let outcome = ran.summary.outcome.name();
                    let code = ran.summary.code.map_or_else(|| format!("m6h:run.{outcome}"), str::to_owned);
                    self.end_run(&code, format!("the run ended {outcome}"), why);
                }
                Ended::Ran(ran) => {
                    if let (Some(record), Some(output)) = (self.views.records.get_mut(&ticket), &ran.output) {
                        record.value = Some(sha256(output));
                    }
                    self.deliver_run(&run);
                }
            }
        }
        if stopped && self.views.run.as_ref().is_some_and(|r| !r.delivered) {
            self.end_run(RUNTIME_LOST, "the view jobs' worker has stopped".to_owned(), Interruption::ByUser);
        }
    }

    /// Delivers the reference publication of the run's revision with the run's `program` dependency.
    fn deliver_run(&mut self, run: &PendingRun) {
        if run.revision != self.revision {
            let reason = format!("engine revision {} replaced revision {}", self.revision, run.revision);
            self.end_run(CANCELLED_REVISION_REPLACED, reason, Interruption::RevisionReplaced);
            return;
        }
        // A run of a new text supersedes the draft: its digest is the program dependency while it is prepared.
        self.set_program(&run.digest);
        let mut publication = self.publication.clone();
        for plan in &mut publication.plans {
            plan.dependencies.insert(PROGRAM.to_owned(), run.digest.clone());
        }
        if let Some(pending) = self.views.run.as_mut() {
            pending.delivered = true;
        }
        let _ = self.host.deliver(CommandId(run.ticket), publication);
    }

    /// Ends the pending run as refused, before the display host ended it, and cancels its job with `why`. The host's
    /// own receipt of the command reads `cancelled(by_user)`; the record carries `code`.
    fn end_run(&mut self, code: &str, reason: String, why: Interruption) {
        let Some(run) = self.views.run.take() else { return };
        self.views.refuse(run.ticket, code, reason);
        self.host.cancel(CommandId(run.ticket));
        if let Some(worker) = &self.views.worker {
            worker.cancel(run.ticket, why);
        }
    }

    /// Ends the record of run `ticket` from its display-host receipt, which is terminal.
    fn close_run(&mut self, ticket: u64) {
        let status = self.host.receipt(CommandId(ticket)).map_or(CommandStatus::UnknownOutcome, |r| r.status);
        let (code, reason, why) = match status {
            CommandStatus::Completed(_) => {
                if let Some(record) = self.views.records.get_mut(&ticket) {
                    record.status = M6H_VIEW_STATUS_PUBLISHED;
                }
                return;
            }
            CommandStatus::Cancelled(CancelReason::Superseded { by }) => {
                (SUPERSEDED.to_owned(), format!("command {} superseded the run", by.0), Interruption::ByUser)
            }
            CommandStatus::Cancelled(CancelReason::ReplacedRevision) => (
                CANCELLED_REVISION_REPLACED.to_owned(),
                "a value the run read was replaced before its publication was installed".to_owned(),
                Interruption::RevisionReplaced,
            ),
            CommandStatus::Cancelled(CancelReason::ByUser) => {
                (CANCELLED_BY_USER.to_owned(), "the run was cancelled".to_owned(), Interruption::ByUser)
            }
            CommandStatus::Cancelled(CancelReason::Budget) => (
                "KN7:budget.output".to_owned(),
                "the publication does not fit the run's output allowance".to_owned(),
                Interruption::ByUser,
            ),
            CommandStatus::Refused(error) | CommandStatus::Failed(error) => {
                (error.code.to_owned(), error.to_string(), Interruption::ByUser)
            }
            CommandStatus::UnknownOutcome | CommandStatus::Accepted | CommandStatus::Running(_) => {
                ("m6h:unknown_outcome".to_owned(), "the run's outcome is unknown".to_owned(), Interruption::ByUser)
            }
        };
        self.views.refuse(ticket, &code, reason);
        if let Some(worker) = &self.views.worker {
            worker.cancel(ticket, why);
        }
    }

    /// A twist adopted a new revision: a pending run that is not delivered yet read the old one and ends now, before
    /// the twist's publication supersedes it. A delivered one ended in `Host::adopt`.
    pub(crate) fn replace_runs(&mut self) {
        if let Some(run) = self.views.run.clone() {
            if !run.delivered {
                let reason = format!("engine revision {} replaced revision {}", self.revision, run.revision);
                self.end_run(CANCELLED_REVISION_REPLACED, reason, Interruption::RevisionReplaced);
            } else if self.host.receipt(CommandId(run.ticket)).is_some_and(|r| r.status.is_terminal()) {
                self.views.run = None;
                self.close_run(run.ticket);
            }
        }
        if self.views.run.is_none() {
            if let Some(editor) = self.views.editor.clone() {
                self.set_program(&editor);
            }
        }
        // The twist publishes the new revision's reference publication itself.
        self.views.republish = false;
    }

    /// After the display host may have ended the pending run: closes its record; once no run is pending, the
    /// editor's digest is the program dependency again, and a run that ended without a publication gets the adopted
    /// revision's reference publication shown again when it superseded that one.
    pub(crate) fn settle_views(&mut self) {
        if let Some(ticket) = self.views.run.as_ref().map(|r| r.ticket) {
            if !self.host.receipt(CommandId(ticket)).is_some_and(|r| r.status.is_terminal()) {
                return;
            }
            self.views.run = None;
            self.close_run(ticket);
        }
        if let Some(editor) = self.views.editor.clone() {
            self.set_program(&editor);
        }
        if std::mem::take(&mut self.views.republish) {
            let shown = self.host.installed(VIEW).is_some_and(|i| i.publication.base.state == self.revision);
            let preparing =
                self.reference.is_some_and(|c| self.host.receipt(c).is_some_and(|r| !r.status.is_terminal()));
            if !shown && !preparing {
                // A lost device answers here; m6h_recover publishes again.
                let _ = self.publish();
            }
        }
    }

    /// The ticket of the view job in the runtime, 0 between jobs or before the first; for tests.
    #[doc(hidden)]
    pub fn running_view_job(&self) -> u64 {
        self.views.worker.as_ref().map_or(0, Worker::running)
    }
}
