//! The session: revisions, previews, commits, undo and redo, branches, workspace records, receipts and the replay that
//! rebuilds them from the journal.
//!
//! Every durable action is planned first and adopted after its frame is durable. The live path and the replay at open
//! build the same plan from the same state and adopt it the same way, so a reopened session is the session that wrote
//! the journal. Lock order: `admin`, then `core`. A commit takes the commit lane under `admin`, releases it, runs under
//! `core`, then releases `core` before it takes `admin` again to free the lane, fence jobs and store its receipt.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use c600_digest::sha256_hex;
use c600_kernel::{codes, Derivation, Effect, IdentityBlock, Kernel, ProtectedSet, Snapshot, Verdict, SLOTS};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::provenance::{self, Json};

use crate::disk::Disk;
use crate::jobs::{self, DiffCache, Jobs};
use crate::store::{Fate, Store};
use crate::token::{Binding, PreviewBinding, Secret, Token};
use crate::types::{
    code, ActionHint, At, CommitRecord, Completion, CompletionKind, Envelope, Error, EventId, Input, JobId, JobResult,
    Mode, Origin, Overview, PreviewInfo, Receipt, RecordValue, ResultRef, RevisionRef, Row, SnapshotHandle, Status,
    WordSource,
};
use crate::wire::{self, Act, Frame, Header, Stored};

/// The longest word the kernel derives in one step (KN2): a longer undo or switch path is refused (a draft limit).
const MAX_WORD: usize = 100_000;
/// The derived states kept besides the current one.
const CACHED_STATES: usize = 8;

/// How a session runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// The declared capacity of the program lane (KN7.2).
    pub program_capacity: usize,
}

impl Default for Options {
    fn default() -> Options {
        Options { program_capacity: 2 }
    }
}

/// One open session of one engine epoch. Clones share the session.
#[derive(Clone)]
pub struct Session(Arc<Inner>);

pub(crate) struct Inner {
    pub(crate) kernel: Arc<Kernel>,
    pub(crate) id: String,
    pub(crate) epoch: u64,
    identity: IdentityBlock,
    secret: Secret,
    pub(crate) options: Options,
    admin: Mutex<Admin>,
    changed: Condvar,
    core: Mutex<Core>,
    pub(crate) diffs: DiffCache,
    next_nonce: AtomicU64,
    next_handle: AtomicU64,
}

/// What the engine keeps about invocations: receipts, used token nonces, the commit lane and the jobs.
#[derive(Default)]
pub(crate) struct Admin {
    /// Receipts by command ID: durable ones rebuilt from the journal, transient ones for this epoch (KN5.23).
    receipts: HashMap<String, Receipt>,
    used: HashSet<u64>,
    commit_busy: bool,
    pub(crate) jobs: Jobs,
}

/// A node of the move tree: a state the session reached by a move. Node `0` is the start.
struct Node {
    parent: EventId,
    letters: Arc<[i32]>,
    digest: String,
    depth: u64,
}

/// One value of the `state` counter: the head it names, its digest, and the `workspace` and `protection` counters
/// seen while it was current.
struct Rev {
    head: EventId,
    digest: String,
    workspace: (u64, u64),
    protection: (u64, u64),
}

struct Rec {
    version: u64,
    value: Option<RecordValue>,
}

/// What an undoable action changed, so that undo and redo can restore either side.
#[derive(Clone)]
enum Change {
    /// The head and the active line before and after.
    Head {
        before: (EventId, usize),
        after: (EventId, usize),
    },
    Record {
        name: String,
        before: Option<RecordValue>,
        after: Option<RecordValue>,
    },
    Protection {
        before: Vec<usize>,
        after: Vec<usize>,
    },
}

/// KN0.8's preview dependencies.
struct Deps {
    epoch: u64,
    state: u64,
    protection: u64,
    draft: String,
}

struct Preview {
    id: u64,
    letters: Arc<[i32]>,
    literal: String,
    source: WordSource,
    reads: Vec<(String, String, u64)>,
    deps: Deps,
    start: String,
    end: Option<Snapshot>,
    effect: Effect,
}

#[derive(Clone)]
struct Checkpoint {
    object: String,
    revision: RevisionRef,
}

struct Core {
    session: String,
    epoch: u64,
    identity: IdentityBlock,
    store: Store,
    /// The latest durable action's order.
    order: u64,
    nodes: HashMap<EventId, Node>,
    /// The tip of each line; `active` is the line the head is on.
    lines: Vec<EventId>,
    active: usize,
    head: EventId,
    current: Snapshot,
    state: u64,
    workspace: u64,
    protection: u64,
    /// Indexed by the `state` counter.
    revisions: Vec<Rev>,
    records: BTreeMap<String, Rec>,
    orbits: Vec<usize>,
    protected: ProtectedSet,
    preview: Option<Preview>,
    next_preview: u64,
    undo: Vec<u64>,
    redo: Vec<u64>,
    changes: HashMap<u64, Change>,
    checkpoints: BTreeMap<String, Checkpoint>,
    cache: VecDeque<(EventId, Snapshot)>,
    read_only: Option<&'static str>,
}

/// What one durable action does.
struct Plan {
    act: Act,
    post: String,
    kind: PlanKind,
}

enum PlanKind {
    /// The head moves: to a new node made by a move (with whether it extends the active line), or to a known node.
    Head {
        head: EventId,
        active: usize,
        state: Snapshot,
        node: Option<(Node, bool)>,
        stack: Stack,
    },
    Record {
        name: String,
        value: Option<RecordValue>,
        stack: Stack,
    },
    Protection {
        orbits: Vec<usize>,
        set: ProtectedSet,
        stack: Stack,
    },
    Checkpoint {
        name: String,
        object: String,
    },
}

enum Stack {
    Push(Change),
    Undo,
    Redo,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn refusal(r: c600_kernel::Refusal) -> Error {
    Error::new(r.code, r.message)
}

fn kernel_error(e: c600_kernel::Error) -> Error {
    match e {
        c600_kernel::Error::Refusal(r) => refusal(r),
        c600_kernel::Error::Certificate(c) => Error::new(code::STATE_HASH_MISMATCH, c.0),
    }
}

fn wide(letters: &[i32]) -> Vec<i64> {
    letters.iter().map(|&m| i64::from(m)).collect()
}

fn io_error(e: std::io::Error) -> Error {
    Error::new(code::DISK_FULL, format!("the session storage failed: {e}"))
}

/// A state object's bytes: the labels as little-endian 32-bit integers, so its name is the state's digest.
fn object_bytes(state: &Snapshot) -> Vec<u8> {
    state.labels().iter().flat_map(|label| label.to_le_bytes()).collect()
}

fn object_labels(bytes: &[u8]) -> Option<Vec<i32>> {
    if bytes.len() != 4 * SLOTS {
        return None;
    }
    Some(bytes.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

/// The letters a record holds: a list of integers that fit a letter.
fn record_letters(payload: &Json) -> Option<Vec<i32>> {
    match payload {
        Json::List(items) => {
            items.iter().map(|item| if let Json::Int(n) = item { i32::try_from(*n).ok() } else { None }).collect()
        }
        _ => None,
    }
}

impl Session {
    /// Creates a session on an empty disk: the journal starts with the session's header, at the solved state. A disk
    /// that holds a whole or a damaged frame is refused before anything is written; the torn remnant of a header
    /// whose append did not finish is cut off.
    pub fn create(kernel: Arc<Kernel>, disk: Box<dyn Disk>, id: &str, options: Options) -> Result<Session, Error> {
        let mut opened = Store::read_journal(disk).map_err(io_error)?;
        if !opened.records.is_empty() || opened.damaged.is_some() {
            return Err(Error::new(code::PERSISTENCE_FORMAT, "the disk already holds a journal"));
        }
        opened.settle().map_err(io_error)?;
        let mut store = opened.store;
        let header = Header {
            session: id.to_owned(),
            model: kernel.identity().model_id,
            start: kernel.solved().digest().into(),
        };
        let bytes = wire::checked(&wire::header_json(&header))?;
        store.append(&bytes).map_err(|fate| match fate {
            Fate::RolledBack => Error::new(code::DISK_FULL, "the session header was not written"),
            Fate::Unknown => Error::new(code::COMMIT_FATE, "nothing proves whether the session header was written"),
        })?;
        Ok(Session::new(kernel, store, id.to_owned(), options))
    }

    /// Opens the session a disk holds and replays its journal under a new epoch. A damaged frame or a frame that does
    /// not replay puts the session into read-only recovery (KN5.23).
    pub fn open(kernel: Arc<Kernel>, disk: Box<dyn Disk>, options: Options) -> Result<Session, Error> {
        let opened = Store::open(disk).map_err(io_error)?;
        let mut records = opened.records.into_iter();
        let header = match records.next().as_deref().and_then(wire::read_record) {
            Some(Stored::Header(header)) => header,
            _ => return Err(Error::new(code::UNKNOWN_SESSION, "the disk holds no session journal")),
        };
        if header.model != kernel.identity().model_id {
            return Err(Error::new(codes::IDENTITY_MISMATCH, "the session belongs to another model"));
        }
        if header.start != kernel.solved().digest() {
            return Err(Error::new(code::PERSISTENCE_FORMAT, "the session does not start at the solved state"));
        }
        let session = Session::new(kernel, opened.store, header.session, options);
        {
            let inner = &*session.0;
            let mut admin = lock(&inner.admin);
            let mut core = lock(&inner.core);
            for payload in records {
                let replayed = match wire::read_record(&payload) {
                    Some(Stored::Frame(frame)) => core.replay(&inner.kernel, &frame).map(|()| frame),
                    _ => Err("not an action record".into()),
                };
                match replayed {
                    Ok(frame) => {
                        let status = core.durable_status(&frame, sha256_hex(&payload));
                        let receipt =
                            Receipt { command: frame.command.clone(), request: frame.request, job: None, status };
                        admin.receipts.insert(frame.command, receipt);
                    }
                    Err(_) => {
                        core.read_only = Some(code::COMMIT_FATE);
                        break;
                    }
                }
            }
            if opened.damaged.is_some() {
                core.read_only = Some(code::COMMIT_FATE);
            }
        }
        Ok(session)
    }

    fn new(kernel: Arc<Kernel>, store: Store, id: String, options: Options) -> Session {
        let secret = Secret::fresh();
        let derived = secret.derive("epoch");
        let epoch = u64::from_le_bytes([
            derived[0], derived[1], derived[2], derived[3], derived[4], derived[5], derived[6], derived[7],
        ]);
        let identity = kernel.identity();
        let solved = kernel.solved();
        let start = solved.digest().to_owned();
        let mut nodes = HashMap::new();
        nodes.insert(0, Node { parent: 0, letters: Arc::from(Vec::new()), digest: start.clone(), depth: 0 });
        let core = Core {
            session: id.clone(),
            epoch,
            identity: identity.clone(),
            store,
            order: 0,
            nodes,
            lines: vec![0],
            active: 0,
            head: 0,
            current: solved,
            state: 0,
            workspace: 0,
            protection: 0,
            revisions: vec![Rev { head: 0, digest: start, workspace: (0, 0), protection: (0, 0) }],
            records: BTreeMap::new(),
            orbits: Vec::new(),
            protected: ProtectedSet::none(),
            preview: None,
            next_preview: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            changes: HashMap::new(),
            checkpoints: BTreeMap::new(),
            cache: VecDeque::new(),
            read_only: None,
        };
        Session(Arc::new(Inner {
            kernel,
            id,
            epoch,
            identity,
            secret,
            options,
            admin: Mutex::new(Admin::default()),
            changed: Condvar::new(),
            core: Mutex::new(core),
            diffs: DiffCache::default(),
            next_nonce: AtomicU64::new(1),
            next_handle: AtomicU64::new(1),
        }))
    }

    pub(crate) fn inner(&self) -> &Inner {
        &self.0
    }

    pub fn id(&self) -> &str {
        &self.0.id
    }

    pub fn epoch(&self) -> u64 {
        self.0.epoch
    }

    pub fn identity(&self) -> &IdentityBlock {
        &self.0.identity
    }

    /// The shell stub: it mints user-action tokens. Only the shell holds it; no runtime operation reaches it.
    /// Superseded by c600-command's shell and layer over [`crate::JournalStore`]; removed once its tests run there.
    pub fn shell(&self) -> Shell<'_> {
        Shell(self)
    }

    /// Delivers one invocation and returns its receipt. A job's receipt is `running`; its terminal receipt comes from
    /// [`Session::wait_job`] or [`Session::outcome`].
    /// Superseded by c600-command's `CommandLayer::invoke` over [`crate::JournalStore`]; removed once its tests run there.
    pub fn invoke(&self, envelope: Envelope) -> Receipt {
        let inner = &*self.0;
        let request = wire::request_digest(&envelope);
        let answer =
            |status| Receipt { command: envelope.command.clone(), request: request.clone(), job: None, status };
        if envelope.session != inner.id {
            return answer(Status::Refused(Error::new(code::UNKNOWN_SESSION, "another session")));
        }
        let mut admin = lock(&inner.admin);
        // KN5.21, LI.6: a command ID with a receipt is answered from it before any other gate, also when the delivery
        // is a retry from an earlier epoch: an identical request gets the stored receipt, other content is a reuse.
        if let Some(stored) = admin.receipts.get(&envelope.command) {
            if stored.request == request {
                return stored.clone();
            }
            return answer(Status::Refused(Error::new(
                code::COMMAND_ID_REUSED,
                "the command ID names another request",
            )));
        }
        if envelope.epoch != inner.epoch {
            return answer(Status::Refused(Error::new(code::EPOCH_CHANGED, "another engine epoch")));
        }
        admin.receipts.insert(envelope.command.clone(), answer(Status::Accepted));
        if let Err(e) = inner.admit(&mut admin, &envelope) {
            return inner.finish(&mut admin, answer(Status::Refused(e)));
        }
        match envelope.row {
            Row::JobSubmit(spec) => return jobs::submit(self, admin, &envelope.command, &request, spec),
            Row::JobStop { job } => return jobs::stop(self, admin, &envelope.command, &request, job),
            _ => {}
        }
        let commit = envelope.row.is_commit();
        drop(admin);
        let (status, replaced) = lock(&inner.core).run(&inner.kernel, &envelope, &request);
        let mut admin = lock(&inner.admin);
        if commit {
            admin.commit_busy = false;
        }
        if let Some(state) = replaced {
            admin.jobs.fence(state);
        }
        inner.finish(&mut admin, answer(status))
    }

    /// LI.32's outcome lookup: the stored receipt of a command, `unknown_outcome` when none is known.
    pub fn outcome(&self, command: &str, request: &str) -> Receipt {
        let admin = lock(&self.0.admin);
        let status = match admin.receipts.get(command) {
            Some(stored) if stored.request == request => return stored.clone(),
            Some(_) => Status::Refused(Error::new(code::COMMAND_ID_REUSED, "the command ID names another request")),
            None => Status::UnknownOutcome,
        };
        Receipt { command: command.to_owned(), request: request.to_owned(), job: None, status }
    }

    /// R-13's status, answered from memory: it reads no storage, whatever the history holds.
    pub fn overview(&self) -> Overview {
        let admin = lock(&self.0.admin);
        let core = lock(&self.0.core);
        Overview {
            revision: core.revision(),
            order: core.order,
            read_only: core.read_only,
            pending_preview: core.preview.as_ref().map(|p| p.id),
            lines: core.lines.len(),
            active_line: core.active,
            checkpoints: core.checkpoints.len(),
            undo: core.hint(false),
            redo: core.hint(true),
            protected: core.orbits.clone(),
            commit_busy: admin.commit_busy,
            jobs_running: admin.jobs.running(),
        }
    }

    /// The pending preview, if any.
    pub fn preview(&self) -> Option<PreviewInfo> {
        let core = lock(&self.0.core);
        core.preview.as_ref().map(|p| PreviewInfo {
            id: p.id,
            literal: p.literal.clone(),
            start: p.start.clone(),
            end: p.end.as_ref().map(|s| s.digest().to_owned()),
            fresh: core.fresh(p),
        })
    }

    /// Opens a read-only snapshot of the current revision, of an earlier revision of this epoch, or of a checkpoint.
    pub fn open_snapshot(&self, at: At) -> Result<SnapshotHandle, Error> {
        let inner = &*self.0;
        let mut core = lock(&inner.core);
        let (revision, state) = core.locate(&inner.kernel, at)?;
        Ok(SnapshotHandle { id: inner.next_handle.fetch_add(1, Ordering::SeqCst), revision, state })
    }

    /// Waits until a job is terminal and returns its result.
    pub fn wait_job(&self, job: JobId) -> Result<JobResult, Error> {
        let inner = &*self.0;
        let mut admin = lock(&inner.admin);
        loop {
            match admin.jobs.result(inner.epoch, job)? {
                Some(result) => return Ok(result),
                None => admin = inner.changed.wait(admin).unwrap_or_else(|e| e.into_inner()),
            }
        }
    }

    /// A job's result if it is terminal, `None` while it runs.
    pub fn job_result(&self, job: JobId) -> Result<Option<JobResult>, Error> {
        lock(&self.0.admin).jobs.result(self.0.epoch, job)
    }

    /// A finished job's result as it applies to a revision: refused unless the revision has the job's input state
    /// (KN7.3). An equal digest after an undo is a match.
    pub fn job_result_at(&self, job: JobId, revision: &RevisionRef) -> Result<JobResult, Error> {
        let result = self.wait_job(job)?;
        if result.input != revision.digest {
            return Err(Error::new(code::JOB_INPUT_DIGEST, "the job ran on another state"));
        }
        Ok(result)
    }

    /// Empties the derived-query cache. A recomputation is charged the same as a cache hit (KN7.8).
    pub fn evict_cache(&self) {
        self.0.diffs.clear();
    }

    /// The number of cached whole-state query answers.
    pub fn cached_queries(&self) -> usize {
        self.0.diffs.len()
    }
}

impl Inner {
    /// The gates before a row runs: what a program may deliver (KN5.15, ONE.77), the token (KN5.14) and the commit
    /// lane (KN5.17).
    fn admit(&self, admin: &mut Admin, envelope: &Envelope) -> Result<(), Error> {
        if envelope.origin == Origin::Program {
            if envelope.row.is_commit() {
                return Err(Error::new(code::COMMIT_ROW_REFUSED, "a program cannot deliver a commit row"));
            }
            if !matches!(envelope.row, Row::JobSubmit(_) | Row::JobStop { .. }) {
                return Err(Error::new(NOT_GRANTED, "the row is not granted to a program"));
            }
        }
        if envelope.row.is_commit() {
            let token = envelope
                .token
                .as_ref()
                .filter(|token| self.secret.verify(token))
                .ok_or_else(|| Error::new(code::TOKEN_INVALID, "no valid user-action token"))?;
            // Step 3 comes before step 4 (chapter 08 section 7): a token refused at the commit lane was seen and stays
            // used, and a new attempt needs a new token.
            if !admin.used.insert(token.nonce) {
                return Err(Error::new(code::TOKEN_USED, "the token was used"));
            }
            if admin.commit_busy {
                return Err(Error::new(code::COMMIT_IN_FLIGHT, "a commit is in flight"));
            }
            admin.commit_busy = true;
        }
        Ok(())
    }

    /// Stores a receipt as the command's answer and wakes every waiter.
    pub(crate) fn finish(&self, admin: &mut Admin, receipt: Receipt) -> Receipt {
        admin.receipts.insert(receipt.command.clone(), receipt.clone());
        self.changed.notify_all();
        receipt
    }

    pub(crate) fn admin(&self) -> MutexGuard<'_, Admin> {
        lock(&self.admin)
    }

    /// A job input: the current revision, or an opened snapshot of this epoch.
    pub(crate) fn capture(&self, input: &Input) -> Result<SnapshotHandle, Error> {
        match input {
            Input::Current => {
                let core = lock(&self.core);
                let (revision, state) = (core.revision(), core.current.clone());
                Ok(SnapshotHandle { id: self.next_handle.fetch_add(1, Ordering::SeqCst), revision, state })
            }
            Input::Handle(handle) if handle.revision.session != self.id => {
                Err(Error::new(code::UNKNOWN_SESSION, "a snapshot of another session"))
            }
            Input::Handle(handle) if handle.revision.epoch != self.epoch => {
                Err(Error::new(code::EPOCH_CHANGED, "a snapshot of another engine epoch"))
            }
            Input::Handle(handle) => Ok(handle.clone()),
        }
    }

    /// The current revision.
    pub(crate) fn revision(&self) -> RevisionRef {
        lock(&self.core).revision()
    }
}

/// The shell stub (KN5.14): it mints a token for one commit invocation the shell is about to deliver.
/// Superseded by c600-command's `Shell` over [`crate::JournalStore`]; removed once its tests run there.
pub struct Shell<'a>(&'a Session);

impl Shell<'_> {
    /// Mints a token for one commit invocation. It binds the `state` and `protection` revisions the action is resolved
    /// against (LI.6): a word commit binds its preview's own dependencies, another commit row the current revisions.
    /// The stub reads them under the session core, so it waits while a commit is being written.
    pub fn mint(&self, command: &str, row: &Row) -> Result<Token, Error> {
        let inner = &*self.0 .0;
        if !row.is_commit() {
            return Err(Error::new(code::TOKEN_INVALID, "only a commit row takes a token"));
        }
        let (state, protection, preview) = {
            let core = lock(&inner.core);
            match row {
                Row::Execute { preview } => match &core.preview {
                    Some(p) if p.id == *preview && core.fresh(p) => {
                        let bound = PreviewBinding { id: p.id, literal: p.literal.clone() };
                        (p.deps.state, p.deps.protection, Some(bound))
                    }
                    _ => return Err(Error::new(code::NO_FRESH_PREVIEW, "no fresh preview with this ID")),
                },
                _ => (core.state, core.protection, None),
            }
        };
        let binding = Binding {
            session: inner.id.clone(),
            epoch: inner.epoch,
            command: command.to_owned(),
            row: row.name(),
            args: wire::args_digest(row),
            identity: inner.identity.clone(),
            state,
            protection,
            preview,
        };
        let nonce = inner.next_nonce.fetch_add(1, Ordering::SeqCst);
        let mac = inner.secret.mac(&binding, nonce);
        Ok(Token { binding, nonce, mac })
    }
}

impl Core {
    fn revision(&self) -> RevisionRef {
        RevisionRef {
            session: self.session.clone(),
            epoch: self.epoch,
            identity: self.identity.clone(),
            mode: Mode::JumbleOff,
            head: self.head,
            state: self.state,
            workspace: self.workspace,
            protection: self.protection,
            digest: self.current.digest().to_owned(),
        }
    }

    fn hint(&self, redo: bool) -> Option<ActionHint> {
        let stack = if redo { &self.redo } else { &self.undo };
        stack.last().map(|&target| ActionHint { order: self.order, target })
    }

    /// KN0.8: a preview is fresh while its epoch, state, protection and draft (the literal word, the resolved
    /// argument and every record it read) are unchanged. Workspace edits it did not read leave it fresh.
    fn fresh(&self, p: &Preview) -> bool {
        p.deps.epoch == self.epoch
            && p.deps.state == self.state
            && p.deps.protection == self.protection
            && self.draft(&p.letters, &p.source, &p.reads) == p.deps.draft
    }

    fn draft(&self, letters: &[i32], source: &WordSource, reads: &[(String, String, u64)]) -> String {
        let now: Vec<(String, String, u64)> = reads
            .iter()
            .map(|(name, _, _)| match self.records.get(name) {
                Some(Rec { version, value: Some(value) }) => (name.clone(), wire::content_digest(value), *version),
                Some(Rec { version, value: None }) => (name.clone(), String::new(), *version),
                None => (name.clone(), String::new(), 0),
            })
            .collect();
        wire::draft_digest(letters, source, &now)
    }

    fn run(&mut self, kernel: &Kernel, envelope: &Envelope, request: &str) -> (Status, Option<u64>) {
        if self.read_only.is_some() {
            return (
                Status::Refused(Error::new(code::READ_ONLY_RECOVERY, "the session is in read-only recovery")),
                None,
            );
        }
        let planned = match &envelope.row {
            Row::Propose { word } => return (self.propose(kernel, word), None),
            Row::Execute { .. } => self.plan_execute(kernel, envelope),
            Row::Undo { hint, confirm } => self
                .check_resolved(envelope)
                .and_then(|()| self.plan_step(kernel, *hint, *confirm, false, &self.protected)),
            Row::Redo { hint, confirm } => self
                .check_resolved(envelope)
                .and_then(|()| self.plan_step(kernel, *hint, *confirm, true, &self.protected)),
            Row::SwitchLine { line } => {
                self.check_resolved(envelope).and_then(|()| self.plan_switch(kernel, *line, &self.protected))
            }
            Row::Checkpoint { name } => Ok(self.plan_checkpoint(name)),
            Row::WorkspaceSet { record, payload, provenance } => {
                self.plan_record(record, payload.clone(), provenance.clone())
            }
            Row::ProtectionSet { orbits, confirm } => self.plan_protection(kernel, orbits, *confirm),
            Row::JobSubmit(_) | Row::JobStop { .. } => unreachable!("job rows run in the jobs module"),
        };
        match planned {
            Ok(plan) => self.write(plan, &envelope.command, request, envelope.row.name()),
            Err(e) => (Status::Refused(e), None),
        }
    }

    /// KN5.9: a preview of a literal word, or of the letters a workspace record holds. A protection conflict still
    /// gets a preview, which `operation.execute` refuses; a mechanically refused word builds none and leaves the
    /// pending preview as it was (KN5.11).
    fn propose(&mut self, kernel: &Kernel, word: &WordSource) -> Status {
        let refused = |code, detail: &str| Status::Refused(Error::new(code, detail));
        let (letters, reads) = match word {
            WordSource::Literal(letters) => (letters.clone(), Vec::new()),
            WordSource::Record(name) => match self.records.get(name) {
                Some(Rec { version, value: Some(value) }) => match record_letters(&value.payload) {
                    Some(letters) => (letters, vec![(name.clone(), wire::content_digest(value), *version)]),
                    None => return refused(codes::NAMED_OBJECT_INVALID, "the record does not hold a list of letters"),
                },
                _ => return refused(codes::NAMED_OBJECT_INVALID, "no such record"),
            },
        };
        let recipe = match kernel.word(&wide(&letters)) {
            Ok(recipe) => recipe,
            Err(r) => return Status::Refused(refusal(r)),
        };
        let (literal, end, effect) = match kernel.derive(&self.current, &recipe, &self.protected) {
            Ok(Derivation::Derived(d)) => (d.word, Some(d.state), d.effect),
            Ok(Derivation::Refused(r)) if r.refusal.code == codes::ORBIT_WOULD_MOVE => (r.word, None, r.effect),
            Ok(Derivation::Refused(r)) => return Status::Refused(refusal(r.refusal)),
            Err(e) => return Status::Refused(kernel_error(e)),
        };
        self.next_preview += 1;
        let id = self.next_preview;
        let draft = wire::draft_digest(&letters, word, &reads);
        self.preview = Some(Preview {
            id,
            letters: letters.into(),
            literal: literal.clone(),
            source: word.clone(),
            reads,
            deps: Deps { epoch: self.epoch, state: self.state, protection: self.protection, draft },
            start: self.current.digest().to_owned(),
            end,
            effect,
        });
        let result = ResultRef { job: None, preview: Some(id), action: None, digest: literal };
        Status::Completed(Completion {
            kind: CompletionKind::PreviewReady,
            revision: self.revision(),
            result: Some(result),
        })
    }

    /// The token's binding must name this invocation (KN5.14).
    fn check_binding(&self, envelope: &Envelope) -> Result<(), Error> {
        let binding = &envelope.token.as_ref().expect("a commit row is admitted with a token").binding;
        let preview = match (&envelope.row, &binding.preview) {
            (Row::Execute { preview }, Some(bound)) => bound.id == *preview,
            (Row::Execute { .. }, None) => false,
            (_, bound) => bound.is_none(),
        };
        let same = binding.session == self.session
            && binding.epoch == self.epoch
            && binding.command == envelope.command
            && binding.row == envelope.row.name()
            && binding.args == wire::args_digest(&envelope.row)
            && binding.identity == self.identity
            && preview;
        if same {
            Ok(())
        } else {
            Err(Error::new(code::TOKEN_MISMATCH, "the token was minted for another invocation"))
        }
    }

    /// LI.6 for a commit without a preview: the token binds this invocation (step 5), and the action is delivered at
    /// the `state` and `protection` revisions it was resolved against (step 6).
    fn check_resolved(&self, envelope: &Envelope) -> Result<(), Error> {
        self.check_binding(envelope)?;
        let binding = &envelope.token.as_ref().expect("a commit row is admitted with a token").binding;
        if binding.state == self.state && binding.protection == self.protection {
            Ok(())
        } else {
            Err(Error::new(code::REVISION_CHANGED, "the revision changed since the action was resolved"))
        }
    }

    /// KN5.16's checks in chapter 08's order (steps 5 to 8): the token binds this invocation and the pending preview
    /// with its literal, the preview is fresh, the state is the one it was built on, and no protected orbit moves. The
    /// commit publishes the preview's own state.
    fn plan_execute(&self, kernel: &Kernel, envelope: &Envelope) -> Result<Plan, Error> {
        self.check_binding(envelope)?;
        let binding = &envelope.token.as_ref().expect("a commit row is admitted with a token").binding;
        let bound = binding.preview.as_ref().expect("the binding of an execute names its preview");
        let preview = match &self.preview {
            Some(p) if p.id == bound.id => p,
            _ => return Err(Error::new(code::PREVIEW_REPLACED, "the preview on screen was replaced")),
        };
        if bound.literal != preview.literal
            || (binding.state, binding.protection) != (preview.deps.state, preview.deps.protection)
        {
            return Err(Error::new(code::TOKEN_MISMATCH, "the token binds another word"));
        }
        if !self.fresh(preview) {
            return Err(Error::new(code::REVISION_CHANGED, "the revision changed since the preview"));
        }
        if self.current.digest() != preview.start {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the state is not the preview's start"));
        }
        match (&preview.end, kernel.protection(&preview.effect, &self.protected).verdict) {
            (Some(end), Verdict::Clear) => Ok(self.plan_move(preview.letters.clone(), end.clone())),
            _ => Err(Error::new(codes::ORBIT_WOULD_MOVE, c600_kernel::PROTECTED_MESSAGE)),
        }
    }

    fn plan_move(&self, letters: Arc<[i32]>, end: Snapshot) -> Plan {
        let head = self.order + 1;
        let extends = self.lines[self.active] == self.head;
        let active = if extends { self.active } else { self.lines.len() };
        let depth = self.nodes[&self.head].depth + 1;
        let node = Node { parent: self.head, letters: letters.clone(), digest: end.digest().to_owned(), depth };
        let change = Change::Head { before: (self.head, self.active), after: (head, active) };
        Plan {
            act: Act::Move { letters },
            post: end.digest().to_owned(),
            kind: PlanKind::Head { head, active, state: end, node: Some((node, extends)), stack: Stack::Push(change) },
        }
    }

    /// An undo or a redo named by its hint (LI.9): refused as stale unless the hint names the latest action and the
    /// stack's top.
    fn plan_step(
        &self,
        kernel: &Kernel,
        hint: ActionHint,
        confirm: bool,
        redo: bool,
        protected: &ProtectedSet,
    ) -> Result<Plan, Error> {
        let stack = if redo { &self.redo } else { &self.undo };
        match stack.last() {
            Some(&target) if hint.order == self.order && hint.target == target => {
                self.plan_inverse(kernel, target, confirm, redo, protected)
            }
            Some(_) => Err(Error::new(code::UNDO_SUPERSEDED, "the hint names a superseded action")),
            None => Err(Error::new(code::UNDO_SUPERSEDED, "there is nothing to step to")),
        }
    }

    /// Restores one side of an undoable action. A head change replays the path between the two nodes under the
    /// current protection and must arrive at the recorded state; a release of protected orbits needs confirmation.
    fn plan_inverse(
        &self,
        kernel: &Kernel,
        target: u64,
        confirm: bool,
        redo: bool,
        protected: &ProtectedSet,
    ) -> Result<Plan, Error> {
        let change = self.changes.get(&target).ok_or_else(|| Error::new(code::UNDO_SUPERSEDED, "unknown action"))?;
        let act = if redo { Act::Redo { target } } else { Act::Undo { target } };
        let stack = if redo { Stack::Redo } else { Stack::Undo };
        let post = self.current.digest().to_owned();
        match change {
            Change::Head { before, after } => {
                let (head, active) = if redo { *after } else { *before };
                let state = self.walk(kernel, head, protected)?;
                let post = state.digest().to_owned();
                Ok(Plan { act, post, kind: PlanKind::Head { head, active, state, node: None, stack } })
            }
            Change::Record { name, before, after } => {
                let value = if redo { after } else { before }.clone();
                Ok(Plan { act, post, kind: PlanKind::Record { name: name.clone(), value, stack } })
            }
            Change::Protection { before, after } => {
                let (from, to) = if redo { (before, after) } else { (after, before) };
                if !confirm && from.iter().any(|orbit| !to.contains(orbit)) {
                    return Err(Error::new(
                        code::CONFIRMATION_REQUIRED,
                        "releasing a protected orbit needs confirmation",
                    ));
                }
                let set = kernel.protected_set(&to.iter().map(|&o| o as i64).collect::<Vec<_>>()).map_err(refusal)?;
                Ok(Plan { act, post, kind: PlanKind::Protection { orbits: to.clone(), set, stack } })
            }
        }
    }

    /// ONE.75: the head moves to another line's tip, and that line becomes the active one.
    fn plan_switch(&self, kernel: &Kernel, line: usize, protected: &ProtectedSet) -> Result<Plan, Error> {
        let head = *self.lines.get(line).ok_or_else(|| Error::new(codes::NAMED_OBJECT_INVALID, "no such line"))?;
        let state = self.walk(kernel, head, protected)?;
        let change = Change::Head { before: (self.head, self.active), after: (head, line) };
        Ok(Plan {
            act: Act::Switch { line },
            post: state.digest().to_owned(),
            kind: PlanKind::Head { head, active: line, state, node: None, stack: Stack::Push(change) },
        })
    }

    fn plan_checkpoint(&self, name: &str) -> Plan {
        let object = self.current.digest().to_owned();
        Plan {
            act: Act::Checkpoint { name: name.to_owned(), object: object.clone() },
            post: object.clone(),
            kind: PlanKind::Checkpoint { name: name.to_owned(), object },
        }
    }

    /// `workspace.set`: the provenance must be a valid FP.17 block (PS1.16).
    fn plan_record(&self, name: &str, payload: Json, provenance: Vec<u8>) -> Result<Plan, Error> {
        provenance::read(&provenance).map_err(|code| Error::new(code, "the provenance is not a valid block"))?;
        let value = RecordValue { payload, provenance };
        let before = self.records.get(name).and_then(|r| r.value.clone());
        let change = Change::Record { name: name.to_owned(), before, after: Some(value.clone()) };
        Ok(Plan {
            act: Act::Record { name: name.to_owned(), value: Some(value.clone()) },
            post: self.current.digest().to_owned(),
            kind: PlanKind::Record { name: name.to_owned(), value: Some(value), stack: Stack::Push(change) },
        })
    }

    fn plan_protection(&self, kernel: &Kernel, orbits: &[i64], confirm: bool) -> Result<Plan, Error> {
        let set = kernel.protected_set(orbits).map_err(refusal)?;
        let to = set.orbits().to_vec();
        if !confirm && self.orbits.iter().any(|orbit| !to.contains(orbit)) {
            return Err(Error::new(code::CONFIRMATION_REQUIRED, "releasing a protected orbit needs confirmation"));
        }
        let change = Change::Protection { before: self.orbits.clone(), after: to.clone() };
        Ok(Plan {
            act: Act::Protection { orbits: to.clone() },
            post: self.current.digest().to_owned(),
            kind: PlanKind::Protection { orbits: to, set, stack: Stack::Push(change) },
        })
    }

    /// The path word from the head to a node: the inverses of the moves up to their common ancestor, then the moves
    /// down to the node.
    fn path(&self, to: EventId) -> Vec<i32> {
        let (mut a, mut b) = (self.head, to);
        let (mut up, mut down) = (Vec::new(), Vec::new());
        while self.nodes[&a].depth > self.nodes[&b].depth {
            up.push(a);
            a = self.nodes[&a].parent;
        }
        while self.nodes[&b].depth > self.nodes[&a].depth {
            down.push(b);
            b = self.nodes[&b].parent;
        }
        while a != b {
            up.push(a);
            a = self.nodes[&a].parent;
            down.push(b);
            b = self.nodes[&b].parent;
        }
        let mut letters = Vec::new();
        for n in up {
            letters.extend(self.nodes[&n].letters.iter().rev().map(|&m| -m));
        }
        for n in down.iter().rev() {
            letters.extend_from_slice(&self.nodes[n].letters);
        }
        letters
    }

    /// The state at a node, reached from the current state by the path word under a protected set.
    fn walk(&self, kernel: &Kernel, to: EventId, protected: &ProtectedSet) -> Result<Snapshot, Error> {
        let letters = self.path(to);
        if letters.is_empty() {
            return Ok(self.current.clone());
        }
        if letters.len() > MAX_WORD {
            return Err(Error::new(codes::RECIPE_SHAPE, "the path is longer than one word"));
        }
        let recipe = kernel.word(&wide(&letters)).map_err(refusal)?;
        match kernel.derive(&self.current, &recipe, protected).map_err(kernel_error)? {
            Derivation::Derived(d) if d.state.digest() == self.nodes[&to].digest => Ok(d.state),
            Derivation::Derived(_) => Err(Error::new(code::STATE_HASH_MISMATCH, "the path does not reach the state")),
            Derivation::Refused(r) => Err(refusal(r.refusal)),
        }
    }

    /// The counters after a plan: the head, `state`, `workspace` and `protection`.
    fn after(&self, plan: &Plan) -> (EventId, u64, u64, u64) {
        match &plan.kind {
            PlanKind::Head { head, .. } => (*head, self.state + 1, self.workspace, self.protection),
            PlanKind::Record { .. } | PlanKind::Checkpoint { .. } => {
                (self.head, self.state, self.workspace + 1, self.protection)
            }
            PlanKind::Protection { .. } => (self.head, self.state, self.workspace + 1, self.protection + 1),
        }
    }

    fn frame(&self, plan: &Plan, command: &str, request: &str, row: &str, epoch: u64) -> Frame {
        let (head, state, workspace, protection) = self.after(plan);
        Frame {
            order: self.order + 1,
            command: command.to_owned(),
            request: request.to_owned(),
            row: row.to_owned(),
            epoch,
            act: plan.act.clone(),
            pre: self.current.digest().to_owned(),
            post: plan.post.clone(),
            head,
            state,
            workspace,
            protection,
        }
    }

    /// Makes a plan durable, then adopts it. A checkpoint's state object is written before the frame that cites it,
    /// unless the disk already holds it intact, so an object lost or damaged since an earlier checkpoint of the same
    /// state is written again. Returns the receipt status and, for a head change, the replaced `state` counter.
    fn write(&mut self, plan: Plan, command: &str, request: &str, row: &str) -> (Status, Option<u64>) {
        let frame = self.frame(&plan, command, request, row, self.epoch);
        let bytes = match wire::checked(&wire::frame_json(&frame)) {
            Ok(bytes) => bytes,
            Err(e) => return (Status::Refused(e), None),
        };
        if let PlanKind::Checkpoint { object, .. } = &plan.kind {
            let state = object_bytes(&self.current);
            let held = matches!(self.store.get_object(object), Ok(Some(stored)) if stored == state);
            if !held && self.store.put_object(object, &state).is_err() {
                return (Status::Failed(Error::new(code::DISK_FULL, "the state object was not written")), None);
            }
        }
        match self.store.append(&bytes) {
            Ok(()) => {}
            Err(Fate::RolledBack) => {
                return (Status::Failed(Error::new(code::DISK_FULL, "the action was not written")), None);
            }
            Err(Fate::Unknown) => {
                self.read_only = Some(code::COMMIT_FATE);
                return (Status::UnknownOutcome, None);
            }
        }
        let replaced = matches!(plan.kind, PlanKind::Head { .. }).then_some(self.state);
        self.adopt(plan, &frame);
        (self.durable_status(&frame, sha256_hex(&bytes)), replaced)
    }

    /// A durable action's receipt status: commit rows are `committed`, workspace rows `completed(workspace)`.
    fn durable_status(&self, frame: &Frame, digest: String) -> Status {
        let mut revision = self.revision();
        revision.epoch = frame.epoch;
        match frame.row.as_str() {
            "operation.execute" | "session.undo" | "session.redo" | "session.switch_line" => {
                Status::Committed(CommitRecord {
                    event: frame.order,
                    pre: frame.pre.clone(),
                    post: frame.post.clone(),
                    revision,
                })
            }
            _ => Status::Completed(Completion {
                kind: CompletionKind::Workspace,
                revision,
                result: Some(ResultRef { job: None, preview: None, action: Some(frame.order), digest }),
            }),
        }
    }

    /// Adopts a durable plan. Counters: a head change advances `state` and discards the pending preview; a record
    /// write advances `workspace` and the record's version; a protection change advances `workspace` and
    /// `protection`; a checkpoint advances `workspace` (KN0.2).
    fn adopt(&mut self, plan: Plan, frame: &Frame) {
        self.order = frame.order;
        match plan.kind {
            PlanKind::Head { head, active, state, node, stack } => {
                if let Some((node, extends)) = node {
                    self.nodes.insert(head, node);
                    if extends {
                        self.lines[active] = head;
                    } else {
                        self.lines.push(head);
                    }
                }
                self.head = head;
                self.active = active;
                self.state += 1;
                let digest = state.digest().to_owned();
                self.remember(head, state.clone());
                self.current = state;
                self.revisions.push(Rev {
                    head,
                    digest,
                    workspace: (self.workspace, self.workspace),
                    protection: (self.protection, self.protection),
                });
                self.preview = None;
                self.step(frame.order, stack);
            }
            PlanKind::Record { name, value, stack } => {
                let record = self.records.entry(name).or_insert(Rec { version: 0, value: None });
                record.version += 1;
                record.value = value;
                self.workspace += 1;
                self.step(frame.order, stack);
            }
            PlanKind::Protection { orbits, set, stack } => {
                self.orbits = orbits;
                self.protected = set;
                self.workspace += 1;
                self.protection += 1;
                self.step(frame.order, stack);
            }
            PlanKind::Checkpoint { name, object } => {
                self.workspace += 1;
                let revision = self.revision();
                self.checkpoints.insert(name, Checkpoint { object, revision });
            }
        }
        let rev = &mut self.revisions[self.state as usize];
        rev.workspace.1 = self.workspace;
        rev.protection.1 = self.protection;
    }

    fn step(&mut self, order: u64, stack: Stack) {
        match stack {
            Stack::Push(change) => {
                self.changes.insert(order, change);
                self.undo.push(order);
                for dropped in self.redo.drain(..) {
                    self.changes.remove(&dropped);
                }
            }
            Stack::Undo => {
                if let Some(target) = self.undo.pop() {
                    self.redo.push(target);
                }
            }
            Stack::Redo => {
                if let Some(target) = self.redo.pop() {
                    self.undo.push(target);
                }
            }
        }
    }

    fn remember(&mut self, node: EventId, state: Snapshot) {
        self.cache.retain(|(n, _)| *n != node);
        self.cache.push_back((node, state));
        while self.cache.len() > CACHED_STATES {
            self.cache.pop_front();
        }
    }

    /// Replays one stored frame: the plan it describes must reproduce the frame exactly.
    fn replay(&mut self, kernel: &Kernel, frame: &Frame) -> Result<(), String> {
        if frame.order != self.order + 1 {
            return Err(format!("order {} after {}", frame.order, self.order));
        }
        let none = ProtectedSet::none();
        let fail = |e: Error| e.to_string();
        let plan = match &frame.act {
            Act::Move { letters } => {
                let recipe = kernel.word(&wide(letters)).map_err(|r| fail(refusal(r)))?;
                match kernel.derive(&self.current, &recipe, &none).map_err(|e| fail(kernel_error(e)))? {
                    Derivation::Derived(d) => self.plan_move(letters.clone(), d.state),
                    Derivation::Refused(r) => return Err(fail(refusal(r.refusal))),
                }
            }
            Act::Undo { target } if self.undo.last() == Some(target) => {
                self.plan_inverse(kernel, *target, true, false, &none).map_err(fail)?
            }
            Act::Redo { target } if self.redo.last() == Some(target) => {
                self.plan_inverse(kernel, *target, true, true, &none).map_err(fail)?
            }
            Act::Undo { .. } | Act::Redo { .. } => return Err("an undo or redo of another action".into()),
            Act::Switch { line } => self.plan_switch(kernel, *line, &none).map_err(fail)?,
            Act::Checkpoint { name, object } if *object == self.current.digest() => self.plan_checkpoint(name),
            Act::Checkpoint { .. } => return Err("a checkpoint of another state".into()),
            Act::Record { name, value: Some(value) } => {
                self.plan_record(name, value.payload.clone(), value.provenance.clone()).map_err(fail)?
            }
            Act::Record { value: None, .. } => return Err("a record write without a value".into()),
            Act::Protection { orbits } => {
                let orbits: Vec<i64> = orbits.iter().map(|&o| o as i64).collect();
                self.plan_protection(kernel, &orbits, true).map_err(fail)?
            }
        };
        if self.frame(&plan, &frame.command, &frame.request, &frame.row, frame.epoch) != *frame {
            return Err(format!("frame {} does not match its replay", frame.order));
        }
        self.adopt(plan, frame);
        Ok(())
    }

    /// Resolves where a snapshot is opened. A revision must be one this session reached in this epoch, with
    /// counters it had. A checkpoint opens from its state object when the object matches its digest; a lost or
    /// damaged object is not used, and the state is replayed along the checkpoint's path from the start instead
    /// (PS1.29), which must reach the checkpoint's digest.
    fn locate(&mut self, kernel: &Kernel, at: At) -> Result<(RevisionRef, Snapshot), Error> {
        match at {
            At::Current => Ok((self.revision(), self.current.clone())),
            At::Revision(r) => {
                if r.session != self.session {
                    return Err(Error::new(code::UNKNOWN_SESSION, "a revision of another session"));
                }
                if r.epoch != self.epoch {
                    return Err(Error::new(code::EPOCH_CHANGED, "a revision of another engine epoch"));
                }
                if r.identity != self.identity || r.mode != Mode::JumbleOff {
                    return Err(Error::new(codes::IDENTITY_MISMATCH, "a revision of another identity or mode"));
                }
                let known = usize::try_from(r.state).ok().and_then(|s| self.revisions.get(s)).is_some_and(|rev| {
                    rev.head == r.head
                        && rev.digest == r.digest
                        && (rev.workspace.0..=rev.workspace.1).contains(&r.workspace)
                        && (rev.protection.0..=rev.protection.1).contains(&r.protection)
                });
                if !known {
                    return Err(Error::new(code::REVISION_GONE, "no such revision"));
                }
                let state = self.state_at(kernel, r.head)?;
                Ok((r, state))
            }
            At::Checkpoint(name) => {
                let checkpoint = self
                    .checkpoints
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| Error::new(codes::NAMED_OBJECT_INVALID, "no such checkpoint"))?;
                let stored = match self.store.get_object(&checkpoint.object) {
                    Ok(Some(bytes)) if sha256_hex(&bytes) == checkpoint.object => {
                        object_labels(&bytes).and_then(|labels| kernel.state_from_labels(labels))
                    }
                    _ => None,
                };
                let state = match stored {
                    Some(state) => state,
                    None => self.state_at(kernel, checkpoint.revision.head)?,
                };
                if state.digest() != checkpoint.object {
                    return Err(Error::new(code::DIGEST_MISMATCH, "the checkpoint's state does not match its digest"));
                }
                Ok((checkpoint.revision, state))
            }
        }
    }

    /// The state at a node: from the nearest known state on its way to the start, replaying the moves down to it.
    fn state_at(&mut self, kernel: &Kernel, node: EventId) -> Result<Snapshot, Error> {
        let mut chain = Vec::new();
        let mut n = node;
        let mut state = loop {
            if n == self.head {
                break self.current.clone();
            }
            if let Some((_, state)) = self.cache.iter().find(|(m, _)| *m == n) {
                break state.clone();
            }
            if n == 0 {
                break kernel.solved();
            }
            chain.push(n);
            n = self.nodes[&n].parent;
        };
        for n in chain.iter().rev() {
            let node = &self.nodes[n];
            let recipe = kernel.word(&wide(&node.letters)).map_err(refusal)?;
            state = match kernel.derive(&state, &recipe, &ProtectedSet::none()).map_err(kernel_error)? {
                Derivation::Derived(d) if d.state.digest() == node.digest => d.state,
                _ => return Err(Error::new(code::STATE_HASH_MISMATCH, "a stored move does not reach its state")),
            };
        }
        self.remember(node, state.clone());
        Ok(state)
    }
}
