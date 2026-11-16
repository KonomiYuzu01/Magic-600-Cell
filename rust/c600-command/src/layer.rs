//! The command layer for one session: invocations, receipts, the preview slot, the commit steps, undo and redo.
//!
//! Chapter 08 section 7's commit steps run in [`CommandLayer::invoke`] for `operation.execute`, in this order: the
//! retry lookup (KN5.21, before lane admission or a token is consumed); the envelope's epoch, the caller's rights and
//! the argument form; the token's authenticator, session, epoch and nonce (step 3); commit-lane admission (step 4);
//! the token's binding to this invocation, and its preview against the pending one and its literal (step 5);
//! `fresh(p)` (step 6); the head hash (step 7); the protection recheck (step 8); the durable transaction (step 9);
//! adoption (step 10). Step 1 is the shell's gate and step 2 its minting ([`crate::Shell`]). A refusal at any step
//! leaves every digest and counter unchanged (CT-ONE-11).
//!
//! `session.undo` and `session.redo` take LI.9's hint and a token and use the commit lane with no `not_busy` gate
//! (KN5.20). An undo derives the inverse of the action's recipe from the current state through the kernel, under the
//! protection set in force (LI.26's normal permission path), and adopts the result only when it equals the action's
//! pre-state digest; a redo derives the recipe again and checks the post-state digest. The journal keeps recipes and
//! digests, not states. Every commit, undo and redo advances the state revision and cancels the jobs that read the
//! revision it replaces, except file jobs (KN5.18).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Effect, Kernel, ProtectedSet, Protection, Recipe, Snapshot, Step, SupportRow, Verdict};

use crate::receipt::{
    code, digest, ActionHint, CancelReason, CommitRecord, Completion, CompletionKind, Direction, Error, PreviewDeps,
    Receipt, ResultRef, Revision, Status,
};
use crate::store::{Fate, MemoryStore, Store, Transaction, TransactionKind};
use crate::table::{Kind, Lane, Overlay, Row, Table};
use crate::token::{args_digest, Secret, Token, TokenBinding};

/// The mode of every revision here: the jumbling engine joins when M5 is on main.
pub const MODE: &str = "jumble_off";
/// A star certificate failure has no registered code yet (draft).
pub const CERTIFICATE: &str = "KN2:consistency.star_certificate";

/// Who invokes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Caller {
    /// The shell, for a solver's press or click.
    Shell,
    /// The language runtime, for one run. It never holds a token.
    Runtime { run: String },
}

/// LI.5's envelope. `args` holds the absolute arguments, without the token.
#[derive(Clone, Debug)]
pub struct Invocation {
    pub command: String,
    pub row: String,
    pub args: Value,
    pub token: Option<Token>,
    pub caller: Caller,
    /// The originating engine epoch. The request digest covers it, and a transport retry keeps the whole envelope, so
    /// a retry finds its receipt after a restart.
    pub epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewState {
    Building,
    Ready,
    Presented,
    Committed,
    Replaced,
    Cancelled,
    Discarded,
}

/// The engine's preview object (KN5.9 to KN5.11).
#[derive(Clone, Debug)]
pub struct Preview {
    pub id: u64,
    /// The command that built it.
    pub command: String,
    pub caller: Caller,
    pub deps: PreviewDeps,
    /// `Digest<literal word>`.
    pub literal: String,
    pub state: PreviewState,
    pub base: Snapshot,
    /// `None` when a protected orbit would move: the preview is still built, and execute refuses it (KN5.9).
    pub end: Option<Snapshot>,
    pub recipe: Recipe,
    pub effect: Effect,
    pub protection: Protection,
}

fn rows_json(rows: &[SupportRow]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json::object([
                    ("orbit", Value::Int(row.orbit as i64)),
                    ("stickers", Value::Int(row.stickers as i64)),
                    ("pieces", Value::Int(row.pieces as i64)),
                ])
            })
            .collect(),
    )
}

impl Preview {
    /// The preview's typed result: what the shell shows, and what its result digest covers.
    pub fn summary(&self) -> Value {
        let verdict = match self.protection.verdict {
            Verdict::Clear => "clear",
            Verdict::Moves => "moves",
        };
        json::object([
            ("id", Value::Int(self.id as i64)),
            ("literal", Value::String(self.literal.clone())),
            ("deps", self.deps.to_json()),
            ("base", Value::String(self.base.digest().to_owned())),
            ("end", self.end.as_ref().map_or(Value::Null, |s| Value::String(s.digest().to_owned()))),
            ("net_sha256", Value::String(self.effect.net_sha256.clone())),
            ("net_moved_stickers", Value::Int(self.effect.moved_stickers() as i64)),
            ("net_moved_pieces", Value::Int(self.effect.moved_pieces as i64)),
            ("primitive_count", Value::Int(self.effect.primitive_count as i64)),
            ("support", rows_json(&self.effect.support)),
            ("conflicts", rows_json(&self.protection.conflicts)),
            ("verdict", Value::String(verdict.into())),
        ])
    }
}

/// A language-defined row's run. The layer has no interpreter, so a host outside this crate takes the job, runs the
/// entry and reports the result.
#[derive(Clone, Debug)]
pub struct ProgramJob {
    pub job: u64,
    pub command: String,
    pub row: String,
    pub kind: Kind,
    pub args: Value,
    pub caller: Caller,
    /// The revision the job reads.
    pub revision: Revision,
    pub overlay: Overlay,
}

enum JobKind {
    Preview { recipe: Recipe, draft: String, caller: Caller },
    Program(ProgramJob),
}

struct Job {
    id: u64,
    command: String,
    lane: Lane,
    kind: JobKind,
}

/// A journal entry. The journal keeps the recipe and both digests.
#[derive(Clone, Debug)]
struct Action {
    order: u64,
    event: String,
    recipe: Recipe,
    pre: String,
    post: String,
}

enum Stacks {
    Commit(Action),
    Undo,
    Redo(Action),
}

/// Everything step 10 applies, computed before step 9.
struct Adoption {
    transaction: Transaction,
    state: Snapshot,
    stacks: Stacks,
    /// The preview a word commit commits.
    preview: Option<u64>,
}

fn refused(code: &'static str, detail: impl Into<String>) -> Result<(Option<u64>, Status), Error> {
    Err(Error::new(code, detail))
}

fn kernel_error(error: c600_kernel::Error) -> Error {
    match error {
        c600_kernel::Error::Refusal(refusal) => refusal.into(),
        c600_kernel::Error::Certificate(error) => Error::new(CERTIFICATE, error.0),
    }
}

fn nat(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_i64).and_then(|n| u64::try_from(n).ok())
}

/// The command layer for one session.
pub struct CommandLayer<S: Store = MemoryStore> {
    kernel: Arc<Kernel>,
    table: Table,
    secret: Secret,
    store: S,
    protected: ProtectedSet,
    revision: Revision,
    current: Snapshot,
    /// The current durable action-order number (LI.9).
    order: u64,
    events: u64,
    jobs_issued: u64,
    jobs: VecDeque<Job>,
    running: BTreeMap<u64, ProgramJob>,
    program_capacity: usize,
    slot: Option<Preview>,
    history: BTreeMap<u64, PreviewState>,
    done: Vec<Action>,
    undone: Vec<Action>,
    used: BTreeSet<u64>,
    receipts: BTreeMap<String, (String, Receipt)>,
    /// Command IDs whose transient receipts a restart lost, with their request digests.
    lost: BTreeMap<String, String>,
    commit_held: bool,
    recovery: Option<Error>,
    limbo: Option<Adoption>,
    generation: u64,
}

impl<S: Store> CommandLayer<S> {
    /// A layer for `session`, at `initial` with the protected set `protected`, in epoch 1.
    pub fn new(
        kernel: Arc<Kernel>,
        table: Table,
        secret: Secret,
        store: S,
        session: &str,
        initial: Snapshot,
        protected: ProtectedSet,
    ) -> CommandLayer<S> {
        let revision = Revision {
            session: session.to_owned(),
            epoch: 1,
            identity: kernel.identity().model_id,
            mode: MODE,
            head: "e0".to_owned(),
            state: 0,
            workspace: 0,
            protection: 0,
            digest: initial.digest().to_owned(),
        };
        CommandLayer {
            kernel,
            table,
            secret,
            store,
            protected,
            revision,
            current: initial,
            order: 0,
            events: 0,
            jobs_issued: 0,
            jobs: VecDeque::new(),
            running: BTreeMap::new(),
            program_capacity: 1,
            slot: None,
            history: BTreeMap::new(),
            done: Vec::new(),
            undone: Vec::new(),
            used: BTreeSet::new(),
            receipts: BTreeMap::new(),
            lost: BTreeMap::new(),
            commit_held: false,
            recovery: None,
            limbo: None,
            generation: 0,
        }
    }

    pub fn kernel(&self) -> &Arc<Kernel> {
        &self.kernel
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    /// Registers a language-defined row (KN5.3).
    pub fn register_overlay(
        &mut self,
        id: &str,
        kind: Kind,
        arguments: &[(&str, &str)],
        overlay: Overlay,
    ) -> Result<(), Error> {
        self.table.register_overlay(id, kind, arguments, overlay).map(|_| ())
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.current
    }

    pub fn protected(&self) -> &ProtectedSet {
        &self.protected
    }

    /// The pending preview.
    pub fn preview(&self) -> Option<&Preview> {
        self.slot.as_ref()
    }

    /// The state of any preview this epoch built.
    pub fn preview_state(&self, id: u64) -> Option<PreviewState> {
        match &self.slot {
            Some(p) if p.id == id => Some(p.state),
            _ if self.jobs.iter().any(|job| job.id == id && job.lane == Lane::Exclusive) => {
                Some(PreviewState::Building)
            }
            _ => self.history.get(&id).copied(),
        }
    }

    /// Advances whenever availability may change: a preview's creation, presentation, cancellation or replacement, a
    /// lane's admission or release, an adoption, a protection change, a restart (CT-KN5-12).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether the session is in read-only recovery, and why.
    pub fn recovery(&self) -> Option<&Error> {
        self.recovery.as_ref()
    }

    /// Sets how many program jobs the program lane holds (LI.22's host setting).
    pub fn set_program_capacity(&mut self, capacity: usize) {
        self.program_capacity = capacity.max(1);
    }

    /// The current preview dependencies for a draft digest (KN0.8).
    pub fn deps(&self, draft: &str) -> PreviewDeps {
        PreviewDeps {
            epoch: self.revision.epoch,
            identity: self.revision.identity.clone(),
            mode: MODE,
            state: self.revision.state,
            protection: self.revision.protection,
            draft: draft.to_owned(),
        }
    }

    /// `kernel.undo_hint`: the current order and the eligible target for `direction`, read together (LI.9).
    pub fn undo_hint(&self, direction: Direction) -> Option<ActionHint> {
        let stack = match direction {
            Direction::Undo => &self.done,
            Direction::Redo => &self.undone,
        };
        stack.last().map(|action| ActionHint { order: self.order, target: action.order })
    }

    /// `kernel.outcome(command, request)` (KN5.23, LI.30): the stored receipt, with no side effect and no lane. A
    /// command stored with another request digest answers `refused(KN5:input.command_id_reused)` and leaves the stored
    /// record unchanged; a transient receipt a restart lost answers `unknown_outcome` (CT-ONE-21); an unknown command
    /// answers `None`.
    pub fn outcome(&self, command: &str, request: &str) -> Option<Receipt> {
        let (stored, receipt) = match self.receipts.get(command) {
            Some((stored, receipt)) => (stored, Some(receipt)),
            None => (self.lost.get(command)?, None),
        };
        let receipt = if stored != request {
            let error = Error::new(code::COMMAND_ID_REUSED, "this command ID was used for other content");
            Receipt {
                command: command.to_owned(),
                request: request.to_owned(),
                job: None,
                status: Status::Refused(error),
            }
        } else if let Some(receipt) = receipt {
            receipt.clone()
        } else {
            Receipt { command: command.to_owned(), request: stored.clone(), job: None, status: Status::UnknownOutcome }
        };
        Some(receipt)
    }

    /// The adapter's report that the built-in chain drew the pending preview (RM.26, ONE.41).
    pub fn present(&mut self, id: u64) -> Result<&Preview, Error> {
        match &mut self.slot {
            Some(p) if p.id == id && matches!(p.state, PreviewState::Ready | PreviewState::Presented) => {
                if p.state == PreviewState::Ready {
                    p.state = PreviewState::Presented;
                    self.generation += 1;
                }
                Ok(self.slot.as_ref().expect("the slot holds the preview"))
            }
            _ => Err(Error::new(code::NO_FRESH_PREVIEW, format!("preview {id} is not the pending ready preview"))),
        }
    }

    /// Holds or releases the commit lane, as an unsettled commit of another invocation would (test hook).
    pub fn hold_commit_lane(&mut self, held: bool) {
        if self.commit_held != held {
            self.commit_held = held;
            self.generation += 1;
        }
    }

    /// Changes the protected set and advances the protection revision, as a protection row's commit would. The
    /// protection rows are not built yet; this is their stand-in.
    pub fn set_protected(&mut self, protected: ProtectedSet) {
        self.protected = protected;
        self.revision.protection += 1;
        self.generation += 1;
    }

    /// Replaces the head state without any revision change, as a corrupted head would (fault rehearsal for step 7).
    #[doc(hidden)]
    pub fn fault_replace_head(&mut self, state: Snapshot) {
        self.current = state;
    }

    /// Ends the epoch and starts the next with a new per-launch secret (CT-ONE-21). Previews, jobs and handles end;
    /// durable mutation receipts stay; other receipts become `unknown_outcome`; no token of the old epoch is valid.
    /// A transaction whose fate was unknown is adopted if the store holds it durably, and dropped otherwise.
    pub fn restart(&mut self, secret: Secret) {
        let epoch = self.revision.epoch + 1;
        self.secret = secret;
        self.used.clear();
        if let Some(p) = self.slot.take() {
            self.history.insert(p.id, PreviewState::Discarded);
        }
        self.jobs.clear();
        self.running.clear();
        self.commit_held = false;
        self.recovery = None;
        let mut resolved = None;
        if let Some(adoption) = self.limbo.take() {
            if self.store.durable(&adoption.transaction.event) {
                let record = self.adopt(adoption);
                resolved = Some(record);
            }
        }
        for (command, (request, receipt)) in std::mem::take(&mut self.receipts) {
            let durable = matches!(receipt.status, Status::Committed(_));
            match &resolved {
                Some((request_digest, record)) if *request_digest == request && !durable => {
                    let receipt = Receipt { status: Status::Committed(record.clone()), ..receipt };
                    self.receipts.insert(command, (request, receipt));
                }
                _ if durable => {
                    self.receipts.insert(command, (request, receipt));
                }
                _ => {
                    self.lost.insert(command, request);
                }
            }
        }
        self.revision.epoch = epoch;
        self.generation += 1;
    }

    /// Whether `row` can be invoked now, computed from the table and the layer's state with no language code running
    /// (CT-KN5-08). `Ok` does not promise acceptance: arguments and the token are checked at invocation.
    pub fn availability(&self, row: &str) -> Result<(), Error> {
        let Some(row) = self.table.row(row) else {
            return Err(Error::new(code::UNKNOWN_ROW, format!("no row `{row}`")));
        };
        if self.recovery.is_some() && Self::mutates(row) {
            return Err(Error::new(code::READ_ONLY_RECOVERY, "the session is in read-only recovery"));
        }
        match row.id.as_str() {
            "operation.preview" => self.exclusive_free(),
            "operation.execute" => {
                if self.commit_held {
                    return Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight"));
                }
                let Some(p) = self.slot.as_ref().filter(|p| p.state == PreviewState::Presented) else {
                    return Err(Error::new(code::NO_FRESH_PREVIEW, "no presented preview"));
                };
                if p.deps != self.deps(&p.deps.draft) {
                    return Err(Error::new(code::REVISION_CHANGED, "the preview is stale"));
                }
                if self.kernel.protection(&p.effect, &self.protected).verdict == Verdict::Moves {
                    return Err(Error::new(code::ORBIT_WOULD_MOVE, c600_kernel::PROTECTED_MESSAGE));
                }
                Ok(())
            }
            // A preview that is still building waits as a job on the exclusive lane, and `cancel` takes it from there.
            "operation.preview.cancel" => {
                if self.slot.is_some() || self.jobs.iter().any(|job| job.lane == Lane::Exclusive) {
                    Ok(())
                } else {
                    Err(Error::new(code::NO_FRESH_PREVIEW, "no pending preview"))
                }
            }
            "session.undo" | "session.redo" => {
                if self.commit_held {
                    return Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight"));
                }
                let direction = if row.id == "session.undo" { Direction::Undo } else { Direction::Redo };
                match self.undo_hint(direction) {
                    Some(_) => Ok(()),
                    None => Err(Error::new(code::UNDO_SUPERSEDED, "no eligible target")),
                }
            }
            _ if row.overlay.is_some() => match row.lane {
                Lane::Program => self.program_free(),
                _ => Ok(()),
            },
            _ => Err(Error::new(code::ROW_NOT_BUILT, format!("`{}` is not built in the headless layer", row.id))),
        }
    }

    fn mutates(row: &Row) -> bool {
        row.id == "operation.preview" || !matches!(row.kind, Kind::Query | Kind::View)
    }

    fn exclusive_free(&self) -> Result<(), Error> {
        if self.jobs.iter().any(|job| job.lane == Lane::Exclusive) {
            return Err(Error::new(code::LANE_FULL, "the exclusive lane holds a job"));
        }
        Ok(())
    }

    fn program_free(&self) -> Result<(), Error> {
        let queued = self.jobs.iter().filter(|job| job.lane == Lane::Program).count();
        // A running view overlay holds the view lane, not the program lane (KN7.2, LI.22).
        let running = self
            .running
            .values()
            .filter(|program| self.table.row(&program.row).is_some_and(|row| row.lane == Lane::Program))
            .count();
        if queued + running >= self.program_capacity {
            return Err(Error::new(code::LANE_FULL, "the program lane is full"));
        }
        Ok(())
    }

    fn request(&self, invocation: &Invocation) -> String {
        digest(&json::object([
            ("session", Value::String(self.revision.session.clone())),
            ("epoch", Value::Int(invocation.epoch as i64)),
            ("command", Value::String(invocation.command.clone())),
            ("row", Value::String(invocation.row.clone())),
            ("args", invocation.args.clone()),
            ("token", invocation.token.as_ref().map_or(Value::Null, |t| t.binding.to_json())),
        ]))
    }

    /// `invoke(command, row, args) -> Receipt` (KN5.1). A command ID the layer knows is answered by its outcome lookup
    /// before anything else: the same content returns the stored receipt, so a retried delivery commits at most once,
    /// also across a restart; other content is refused (CT-KN5-04, LI.30).
    pub fn invoke(&mut self, invocation: Invocation) -> Receipt {
        let request = self.request(&invocation);
        if let Some(receipt) = self.outcome(&invocation.command, &request) {
            return receipt;
        }
        let (job, status) = match self.dispatch(&invocation, &request) {
            Ok(answer) => answer,
            Err(error) => (None, Status::Refused(error)),
        };
        let receipt = Receipt { command: invocation.command.clone(), request: request.clone(), job, status };
        self.receipts.insert(invocation.command, (request, receipt.clone()));
        receipt
    }

    fn dispatch(&mut self, invocation: &Invocation, request: &str) -> Result<(Option<u64>, Status), Error> {
        if invocation.epoch != self.revision.epoch {
            return refused(code::EPOCH_CHANGED, "the envelope belongs to another engine epoch");
        }
        let Some(row) = self.table.row(&invocation.row).cloned() else {
            return refused(code::UNKNOWN_ROW, format!("no row `{}`", invocation.row));
        };
        if let Caller::Runtime { .. } = invocation.caller {
            if matches!(row.kind, Kind::Commit | Kind::App) {
                return refused(code::COMMIT_ROW_REFUSED, format!("the runtime may not invoke `{}` (KN5.15)", row.id));
            }
            if invocation.token.is_some() {
                return refused(code::TOKEN_INVALID, "the runtime never holds a token (KN5.15)");
            }
        }
        if self.recovery.is_some() && Self::mutates(&row) {
            return refused(code::READ_ONLY_RECOVERY, "the session is in read-only recovery");
        }
        let Some(args) = invocation.args.as_object() else {
            return refused(code::ARGUMENT_FORM, "the arguments are not an object");
        };
        if !args.keys().map(String::as_str).eq(row.plain_arguments()) {
            return refused(code::ARGUMENT_FORM, format!("`{}` takes {:?}", row.id, row.arguments));
        }
        if row.takes_token() != invocation.token.is_some() {
            return match invocation.token {
                None => refused(code::TOKEN_INVALID, format!("`{}` needs a user-action token", row.id)),
                Some(_) => refused(code::ARGUMENT_FORM, format!("`{}` takes no token", row.id)),
            };
        }
        match row.id.as_str() {
            "operation.preview" => self.admit_preview(invocation),
            "operation.execute" => self.execute(invocation, request),
            "operation.preview.cancel" => self.cancel(invocation),
            "session.undo" => self.undo(invocation, request, Direction::Undo),
            "session.redo" => self.undo(invocation, request, Direction::Redo),
            _ if row.overlay.is_some() => self.admit_program(invocation, &row),
            _ => refused(code::ROW_NOT_BUILT, format!("`{}` is not built in the headless layer", row.id)),
        }
    }

    fn next_job(&mut self) -> u64 {
        self.jobs_issued += 1;
        self.jobs_issued
    }

    fn next_event(&mut self) -> String {
        self.events += 1;
        format!("e{}", self.events)
    }

    // ----- operation.preview -----

    fn admit_preview(&mut self, invocation: &Invocation) -> Result<(Option<u64>, Status), Error> {
        let draft = invocation.args.get("draft").expect("the form check saw the draft");
        let Some(recipe) = draft.get("recipe").filter(|_| draft.as_object().is_some_and(|o| o.len() == 1)) else {
            return refused(code::ARGUMENT_FORM, "a draft is `{\"recipe\": [...]}`; ResolvedDraftRef waits for M6");
        };
        let recipe = self.kernel.recipe(recipe)?;
        self.exclusive_free()?;
        let id = self.next_job();
        let kind = JobKind::Preview { recipe, draft: digest(draft), caller: invocation.caller.clone() };
        self.jobs.push_back(Job { id, command: invocation.command.clone(), lane: Lane::Exclusive, kind });
        self.generation += 1;
        Ok((Some(id), Status::Running(id)))
    }

    /// Runs every queued preview job at the current revision, in order. Program jobs wait for their host.
    pub fn run_jobs(&mut self) -> usize {
        let mut ran = 0;
        let mut keep = VecDeque::new();
        while let Some(job) = self.jobs.pop_front() {
            match job.kind {
                JobKind::Program(_) => keep.push_back(job),
                JobKind::Preview { recipe, draft, caller } => {
                    self.build_preview(job.id, &job.command, recipe, &draft, caller);
                    ran += 1;
                }
            }
        }
        self.jobs = keep;
        ran
    }

    fn build_preview(&mut self, id: u64, command: &str, recipe: Recipe, draft: &str, caller: Caller) {
        let base = self.current.clone();
        let status = match self.kernel.derive(&base, &recipe, &self.protected) {
            Err(error) => Status::Failed(kernel_error(error)),
            Ok(derivation) => {
                let (end, effect, protection) = match derivation {
                    Derivation::Derived(d) => (Some(d.state), d.effect, d.protection),
                    Derivation::Refused(r) => (None, r.effect, r.protection),
                };
                let preview = Preview {
                    id,
                    command: command.to_owned(),
                    caller,
                    deps: self.deps(draft),
                    literal: effect.expansion_sha256.clone(),
                    state: PreviewState::Ready,
                    base,
                    end,
                    recipe,
                    effect,
                    protection,
                };
                let result = ResultRef { job: Some(id), digest: digest(&preview.summary()) };
                if let Some(old) = self.slot.replace(preview) {
                    self.history.insert(old.id, PreviewState::Replaced);
                }
                let revision = self.revision.clone();
                Status::Completed(Completion { kind: CompletionKind::PreviewReady, revision, result: Some(result) })
            }
        };
        self.settle(command, status);
        self.generation += 1;
    }

    fn settle(&mut self, command: &str, status: Status) {
        if let Some((_, receipt)) = self.receipts.get_mut(command) {
            receipt.status = status;
        }
    }

    // ----- operation.preview.cancel -----

    fn cancel(&mut self, invocation: &Invocation) -> Result<(Option<u64>, Status), Error> {
        let Some(id) = nat(invocation.args.get("preview")) else {
            return refused(code::ARGUMENT_FORM, "`preview` is a preview ID");
        };
        let done = Completion { kind: CompletionKind::Workspace, revision: self.revision.clone(), result: None };
        if self.slot.as_ref().is_some_and(|p| p.id == id) {
            self.slot.take();
            self.history.insert(id, PreviewState::Cancelled);
            self.generation += 1;
            return Ok((None, Status::Completed(done)));
        }
        if let Some(position) = self.jobs.iter().position(|job| job.id == id && job.lane == Lane::Exclusive) {
            let job = self.jobs.remove(position).expect("the position is in range");
            self.settle(&job.command, Status::Cancelled(CancelReason::ByUser));
            self.history.insert(id, PreviewState::Cancelled);
            self.generation += 1;
            return Ok((None, Status::Completed(done)));
        }
        Err(self.missing_preview(id))
    }

    fn missing_preview(&self, id: u64) -> Error {
        match self.history.get(&id) {
            Some(PreviewState::Replaced) => Error::new(code::PREVIEW_REPLACED, format!("preview {id} was replaced")),
            Some(PreviewState::Discarded) => {
                Error::new(code::REVISION_CHANGED, format!("preview {id} was discarded when the state moved"))
            }
            _ => Error::new(code::NO_FRESH_PREVIEW, format!("preview {id} is not pending")),
        }
    }

    // ----- tokens -----

    /// Step 3: the authenticator, the session and epoch, and the nonce. The nonce is spent once the token
    /// authenticates, so a press refused at a later step stays spent.
    fn authenticate(&mut self, invocation: &Invocation) -> Result<TokenBinding, Error> {
        let Some(token) = &invocation.token else {
            return Err(Error::new(code::TOKEN_INVALID, "no token"));
        };
        let b = &token.binding;
        if !token.verify(&self.secret) || b.session != self.revision.session || b.epoch != self.revision.epoch {
            return Err(Error::new(code::TOKEN_INVALID, "the token does not authenticate for this session and epoch"));
        }
        if !self.used.insert(b.nonce) {
            return Err(Error::new(code::TOKEN_USED, "the token was used"));
        }
        Ok(b.clone())
    }

    /// Step 5's binding: a genuine token's command, row, absolute arguments and identity are this invocation's.
    fn bound(&self, binding: &TokenBinding, invocation: &Invocation) -> Result<(), Error> {
        if binding.command != invocation.command
            || binding.row != invocation.row
            || binding.args != args_digest(&invocation.args)
            || binding.identity != self.revision.identity
        {
            return Err(Error::new(code::TOKEN_MISMATCH, "the token is bound to another invocation"));
        }
        Ok(())
    }

    fn commit_lane(&self) -> Result<(), Error> {
        match self.commit_held {
            true => Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight")),
            false => Ok(()),
        }
    }

    // ----- operation.execute -----

    fn execute(&mut self, invocation: &Invocation, request: &str) -> Result<(Option<u64>, Status), Error> {
        let Some(id) = nat(invocation.args.get("preview")) else {
            return refused(code::ARGUMENT_FORM, "`preview` is a preview ID");
        };
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        let Some(bound) = binding.preview.as_ref().filter(|b| b.id == id) else {
            return refused(code::TOKEN_MISMATCH, "a word commit's token binds the presented preview");
        };
        let Some(p) = self.slot.as_ref().filter(|p| p.id == id) else {
            return Err(self.missing_preview(id));
        };
        if p.state != PreviewState::Presented {
            return refused(code::NO_FRESH_PREVIEW, format!("preview {id} was never presented"));
        }
        if p.literal != bound.literal {
            return refused(code::TOKEN_MISMATCH, "the token's literal digest differs from the preview's");
        }
        let now = self.deps(&p.deps.draft);
        if p.deps != now || bound.deps != now {
            return refused(code::REVISION_CHANGED, "the preview's dependencies changed");
        }
        if self.current.digest() != p.base.digest() {
            return refused(code::STATE_HASH_MISMATCH, "the head hash differs from the preview's start");
        }
        let protection = self.kernel.protection(&p.effect, &self.protected);
        let end = match (&p.end, protection.verdict) {
            (Some(end), Verdict::Clear) => end.clone(),
            _ => return refused(code::ORBIT_WOULD_MOVE, c600_kernel::PROTECTED_MESSAGE),
        };
        let (recipe, literal, pre) = (p.recipe.clone(), p.literal.clone(), p.base.digest().to_owned());
        let event = self.next_event();
        let order = self.order + 1;
        let revision = Revision {
            head: event.clone(),
            state: self.revision.state + 1,
            digest: end.digest().to_owned(),
            ..self.revision.clone()
        };
        let post = end.digest().to_owned();
        let action = Action { order, event: event.clone(), recipe, pre: pre.clone(), post: post.clone() };
        let transaction = Transaction {
            kind: TransactionKind::Commit,
            event,
            order,
            target: None,
            command: invocation.command.clone(),
            row: invocation.row.clone(),
            request: request.to_owned(),
            literal,
            pre,
            post,
            revision,
        };
        self.transact(Adoption { transaction, state: end, stacks: Stacks::Commit(action), preview: Some(id) })
    }

    /// Steps 9 and 10.
    fn transact(&mut self, adoption: Adoption) -> Result<(Option<u64>, Status), Error> {
        match self.store.transact(&adoption.transaction) {
            Fate::Durable => {
                let (_, record) = self.adopt(adoption);
                Ok((None, Status::Committed(record)))
            }
            Fate::RolledBack(error) => Ok((None, Status::Failed(error))),
            Fate::Unknown => {
                self.recovery = Some(Error::new(code::COMMIT_FATE, "the store cannot say whether the commit landed"));
                self.limbo = Some(adoption);
                self.generation += 1;
                Ok((None, Status::UnknownOutcome))
            }
        }
    }

    /// Step 10: adopt, discard or commit the pending preview, cancel the jobs on the replaced revision except file
    /// jobs, and return the request digest and the commit record.
    fn adopt(&mut self, adoption: Adoption) -> (String, CommitRecord) {
        let Adoption { transaction, state, stacks, preview } = adoption;
        let record = CommitRecord {
            event: transaction.event.clone(),
            pre: transaction.pre.clone(),
            post: transaction.post.clone(),
            revision: transaction.revision.clone(),
        };
        self.current = state;
        self.revision = transaction.revision.clone();
        self.order = transaction.order;
        match stacks {
            Stacks::Commit(action) => {
                self.done.push(action);
                self.undone.clear();
            }
            Stacks::Undo => {
                let action = self.done.pop().expect("an undo has a target");
                self.undone.push(action);
            }
            Stacks::Redo(action) => {
                self.undone.pop();
                self.done.push(action);
            }
        }
        if let Some(p) = self.slot.take() {
            let state = if preview == Some(p.id) { PreviewState::Committed } else { PreviewState::Discarded };
            self.history.insert(p.id, state);
        }
        let mut keep = VecDeque::new();
        for job in std::mem::take(&mut self.jobs) {
            if matches!(&job.kind, JobKind::Program(program) if program.kind == Kind::File) {
                keep.push_back(job);
                continue;
            }
            if job.lane == Lane::Exclusive {
                self.history.insert(job.id, PreviewState::Discarded);
            }
            self.settle(&job.command, Status::Cancelled(CancelReason::ReplacedRevision));
        }
        self.jobs = keep;
        let running = std::mem::take(&mut self.running);
        for (id, program) in running {
            if program.kind == Kind::File {
                self.running.insert(id, program);
            } else {
                self.settle(&program.command, Status::Cancelled(CancelReason::ReplacedRevision));
            }
        }
        self.generation += 1;
        (transaction.request, record)
    }

    // ----- session.undo and session.redo -----

    fn inverse(&self, recipe: &Recipe) -> Result<Recipe, Error> {
        let steps = recipe.steps().iter().rev().map(|step| match step {
            Step::Word(letters) => json::object([
                ("kind", Value::String("word".into())),
                ("moves", Value::Array(letters.iter().rev().map(|&m| Value::Int(-(m as i64))).collect())),
            ]),
            Step::Star(star) => json::object([
                ("kind", Value::String("star".into())),
                ("orbit", Value::Int(star.orbit() as i64)),
                ("node", Value::Int(star.node() as i64)),
                ("sign", Value::Int(-star.sign() as i64)),
            ]),
        });
        Ok(self.kernel.recipe(&Value::Array(steps.collect()))?)
    }

    fn undo(
        &mut self,
        invocation: &Invocation,
        request: &str,
        direction: Direction,
    ) -> Result<(Option<u64>, Status), Error> {
        let Some(hint) = invocation.args.get("hint").and_then(ActionHint::from_json) else {
            return refused(code::ARGUMENT_FORM, "`hint` is an ActionHint");
        };
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        if binding.hint != Some(hint) {
            return refused(code::TOKEN_MISMATCH, "an undo or redo token binds its hint");
        }
        if self.undo_hint(direction) != Some(hint) {
            return refused(code::UNDO_SUPERSEDED, "the hint's order or target changed");
        }
        let action = match direction {
            Direction::Undo => self.done.last(),
            Direction::Redo => self.undone.last(),
        }
        .expect("a current hint has a target")
        .clone();
        let (from, to) = match direction {
            Direction::Undo => (&action.post, &action.pre),
            Direction::Redo => (&action.pre, &action.post),
        };
        if self.current.digest() != from {
            return refused(code::STATE_HASH_MISMATCH, "the head hash differs from the action's");
        }
        let recipe = match direction {
            Direction::Undo => self.inverse(&action.recipe)?,
            Direction::Redo => action.recipe.clone(),
        };
        let derived = match self.kernel.derive(&self.current, &recipe, &self.protected).map_err(kernel_error)? {
            Derivation::Derived(derived) => derived,
            Derivation::Refused(refusal) => return Err(refusal.refusal.into()),
        };
        if derived.state.digest() != to {
            return refused(code::STATE_HASH_MISMATCH, "the inverse does not reach the action's recorded state");
        }
        let event = self.next_event();
        let order = self.order + 1;
        let (head, stacks, kind) = match direction {
            Direction::Undo => {
                let below = self.done.len().checked_sub(2).map(|i| self.done[i].event.clone());
                (below.unwrap_or_else(|| "e0".to_owned()), Stacks::Undo, TransactionKind::Undo)
            }
            Direction::Redo => {
                let again = Action { order, event: event.clone(), ..action.clone() };
                (event.clone(), Stacks::Redo(again), TransactionKind::Redo)
            }
        };
        let revision = Revision { head, state: self.revision.state + 1, digest: to.clone(), ..self.revision.clone() };
        let transaction = Transaction {
            kind,
            event,
            order,
            target: Some(action.order),
            command: invocation.command.clone(),
            row: invocation.row.clone(),
            request: request.to_owned(),
            literal: derived.word.clone(),
            pre: from.clone(),
            post: to.clone(),
            revision,
        };
        self.transact(Adoption { transaction, state: derived.state, stacks, preview: None })
    }

    // ----- language-defined rows -----

    fn admit_program(&mut self, invocation: &Invocation, row: &Row) -> Result<(Option<u64>, Status), Error> {
        let overlay = row.overlay.clone().expect("an overlay row");
        if row.lane == Lane::Program {
            self.program_free()?;
        }
        let id = self.next_job();
        if row.lane == Lane::View {
            let superseded: Vec<String> = self
                .jobs
                .iter()
                .filter(|job| job.lane == Lane::View && matches!(&job.kind, JobKind::Program(p) if p.row == row.id))
                .map(|job| job.command.clone())
                .collect();
            self.jobs.retain(|job| !(job.lane == Lane::View && superseded.contains(&job.command)));
            for command in superseded {
                self.settle(&command, Status::Cancelled(CancelReason::Superseded(invocation.command.clone())));
            }
        }
        let program = ProgramJob {
            job: id,
            command: invocation.command.clone(),
            row: row.id.clone(),
            kind: row.kind,
            args: invocation.args.clone(),
            caller: invocation.caller.clone(),
            revision: self.revision.clone(),
            overlay,
        };
        self.jobs.push_back(Job {
            id,
            command: invocation.command.clone(),
            lane: row.lane,
            kind: JobKind::Program(program),
        });
        self.generation += 1;
        Ok((Some(id), Status::Running(id)))
    }

    /// Takes the next queued program job for the host to run.
    pub fn take_program_job(&mut self) -> Option<ProgramJob> {
        let position = self.jobs.iter().position(|job| matches!(job.kind, JobKind::Program(_)))?;
        let job = self.jobs.remove(position)?;
        let JobKind::Program(program) = job.kind else { unreachable!("the position names a program job") };
        self.running.insert(program.job, program.clone());
        Some(program)
    }

    /// Reports a program job's result. A job a commit or undo cancelled meanwhile stays cancelled, and its result is
    /// dropped (`None`).
    pub fn finish_program_job(&mut self, job: u64, result: Result<Value, Error>) -> Option<Receipt> {
        let program = self.running.remove(&job)?;
        let status = match result {
            Err(error) => Status::Failed(error),
            Ok(value) => {
                let kind = match program.kind {
                    Kind::Query => CompletionKind::Query,
                    Kind::Workspace => CompletionKind::Workspace,
                    Kind::File => CompletionKind::File,
                    Kind::View => CompletionKind::ViewPublished,
                    Kind::Commit | Kind::App => unreachable!("an overlay row is never a commit or app row"),
                };
                let result = Some(ResultRef { job: Some(job), digest: digest(&value) });
                Status::Completed(Completion { kind, revision: program.revision.clone(), result })
            }
        };
        self.settle(&program.command, status);
        self.generation += 1;
        self.receipts.get(&program.command).map(|(_, receipt)| receipt.clone())
    }
}
