//! The values a session exchanges with its callers: revision references (KN0.2), rows and envelopes, receipts (KN5.21),
//! action hints (LI.9), job specifications and results (KN7.3) and the status overview (R-13). The Rust shapes and the
//! M6 row names are drafts (`rust/README.md`).

use std::fmt;
use std::sync::Arc;

use c600_kernel::{IdentityBlock, Snapshot};
use c600_run::interp::Loaded;
use c600_run::meter::{Amounts, Summary};
use c600_run::provenance::Json;
use c600_run::value::Value;

use crate::token::Token;

/// The chapter 13 codes this crate raises. Kernel and runtime codes are used from their crates.
pub mod code {
    pub const UNKNOWN_SESSION: &str = "KN4:input.unknown_session";
    pub const DISK_FULL: &str = "KN4:resource_limit.disk_full";
    pub const EPOCH_CHANGED: &str = "KN2:stale.epoch_changed";
    pub const REVISION_GONE: &str = "KN2:stale.revision_gone";
    pub const COMMAND_ID_REUSED: &str = "KN5:input.command_id_reused";
    pub const CONFIRMATION_REQUIRED: &str = "KN5:input.confirmation_required";
    pub const COMMIT_ROW_REFUSED: &str = "KN5:capability.commit_row_refused";
    pub const TOKEN_INVALID: &str = "KN5:auth.token_invalid";
    pub const COMMIT_IN_FLIGHT: &str = "KN5:busy.commit_in_flight";
    pub const NO_FRESH_PREVIEW: &str = "KN5:stale.no_fresh_preview";
    pub const TOKEN_USED: &str = "KN5:stale.token_used";
    pub const PREVIEW_REPLACED: &str = "KN5:stale.preview_replaced";
    pub const TOKEN_MISMATCH: &str = "KN5:stale.token_mismatch";
    pub const REVISION_CHANGED: &str = "KN5:stale.revision_changed";
    pub const UNDO_SUPERSEDED: &str = "KN5:stale.undo_superseded";
    pub const RESULT_GONE: &str = "KN5:stale.result_gone";
    pub const STATE_HASH_MISMATCH: &str = "KN5:consistency.state_hash_mismatch";
    pub const COMMIT_FATE: &str = "KN5:unknown_outcome.commit_fate";
    pub const READ_ONLY_RECOVERY: &str = "KN5:unknown_outcome.read_only_recovery";
    pub const LANE_FULL: &str = "KN7:busy.lane_full";
    pub const JOB_INPUT_DIGEST: &str = "KN7:stale.job_input_digest";
    pub const RUNTIME_LOST: &str = "KN7:resource_limit.runtime_lost";
    pub const DIGEST_MISMATCH: &str = "PS1:input.digest_mismatch";
    pub const PERSISTENCE_FORMAT: &str = "PS1:input.persistence_format";
    pub const NONCANONICAL_VALUE: &str = "PS1:input.noncanonical_value";
    pub const RECORD_TOO_LARGE: &str = "PS1:resource_limit.record_too_large";
}

/// A typed error with its stable code (KN5.24).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    pub detail: String,
}

impl Error {
    pub(crate) fn new(code: &'static str, detail: impl Into<String>) -> Error {
        Error { code, detail: detail.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for Error {}

/// KN0.2's mode. M6 runs 600-cell-Full only; the Jumble rows are not in M6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    JumbleOff,
}

/// A durable action's place in the session's one order of actions. `0` is the session's start; the head of the puzzle
/// journal is the action that made its state (KN5.21: `revision.head` need not be the latest action after an undo).
pub type EventId = u64;

/// KN0.2's revision reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionRef {
    pub session: String,
    pub epoch: u64,
    pub identity: IdentityBlock,
    pub mode: Mode,
    pub head: EventId,
    pub state: u64,
    pub workspace: u64,
    pub protection: u64,
    /// `Digest<labels>` of the head's state.
    pub digest: String,
}

/// LI.9's action hint: the latest action's order and the action an undo or a redo would invert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionHint {
    pub order: u64,
    pub target: u64,
}

/// A job of one engine epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JobId {
    pub epoch: u64,
    pub n: u64,
}

/// Who delivers an invocation. A program reaches the session only through its host (KN5.15, ONE.77).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Shell,
    Program,
}

/// The word a proposal replays: a literal the caller supplies, or a workspace record holding a list of letters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WordSource {
    Literal(Vec<i32>),
    Record(String),
}

/// The value of a workspace record: a JSON payload and its FP.17 provenance block bytes (PS1.16, M3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordValue {
    pub payload: Json,
    pub provenance: Vec<u8>,
}

/// The lane of a program job (KN7.2). The `commit` lane is the commit rows' own, and `none` holds the short rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Exclusive,
    Program,
}

/// What a commit does to a running job: a query is cancelled when it reads the replaced revision; a file job and a
/// digest-pinned research job finish (KN5.18, KN7.5, ONE.49).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobKind {
    Query,
    File,
    Pinned,
}

/// A job's input state: the current revision when the job is admitted, or an opened snapshot.
#[derive(Clone, Debug)]
pub enum Input {
    Current,
    Handle(SnapshotHandle),
}

/// A program job: one exported function of a loaded program, run on opened snapshots under an explicit budget (KN7.7).
#[derive(Clone, Debug)]
pub struct JobSpec {
    pub program: Arc<Loaded>,
    /// The digest that names the program, such as its IR digest. It enters the request digest.
    pub program_digest: String,
    pub module: String,
    pub function: String,
    pub args: Vec<Value>,
    pub inputs: Vec<Input>,
    pub lane: Lane,
    pub kind: JobKind,
    pub budget: Amounts,
}

/// The rows of M6. Names follow the command table where it has them; the others are M6 drafts.
#[derive(Clone, Debug)]
pub enum Row {
    /// `operation.preview` (KN5.9).
    Propose { word: WordSource },
    /// `operation.execute` (KN5.14 to KN5.16).
    Execute { preview: u64 },
    /// `session.undo` (KN5.19, KN5.20, LI.26).
    Undo { hint: ActionHint, confirm: bool },
    /// `session.redo`.
    Redo { hint: ActionHint, confirm: bool },
    /// `session.switch_line` (draft name; ONE.75).
    SwitchLine { line: usize },
    /// `session.checkpoint` (draft name): a named state object; not undoable (LI.9).
    Checkpoint { name: String },
    /// `workspace.set` (draft name): writes one workspace record.
    WorkspaceSet { record: String, payload: Json, provenance: Vec<u8> },
    /// `protection.set` (draft name): a workspace action that sets the protected orbits.
    ProtectionSet { orbits: Vec<i64>, confirm: bool },
    /// `job.submit` (draft name).
    JobSubmit(JobSpec),
    /// `job.stop` (LI.24).
    JobStop { job: JobId },
}

impl Row {
    pub fn name(&self) -> &'static str {
        match self {
            Row::Propose { .. } => "operation.preview",
            Row::Execute { .. } => "operation.execute",
            Row::Undo { .. } => "session.undo",
            Row::Redo { .. } => "session.redo",
            Row::SwitchLine { .. } => "session.switch_line",
            Row::Checkpoint { .. } => "session.checkpoint",
            Row::WorkspaceSet { .. } => "workspace.set",
            Row::ProtectionSet { .. } => "protection.set",
            Row::JobSubmit(_) => "job.submit",
            Row::JobStop { .. } => "job.stop",
        }
    }

    /// Whether the row is a commit row: it needs a user-action token and the commit lane.
    pub fn is_commit(&self) -> bool {
        matches!(self, Row::Execute { .. } | Row::Undo { .. } | Row::Redo { .. } | Row::SwitchLine { .. })
    }
}

/// One invocation as it is delivered (LI.5).
#[derive(Clone, Debug)]
pub struct Envelope {
    pub session: String,
    pub epoch: u64,
    pub command: String,
    pub origin: Origin,
    pub row: Row,
    pub token: Option<Token>,
}

/// KN5.21's receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub command: String,
    pub request: String,
    pub job: Option<JobId>,
    pub status: Status,
}

/// A receipt's status. `Accepted` and `Running` are not terminal; `UnknownOutcome` reports missing knowledge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Accepted,
    Running(JobId),
    Committed(CommitRecord),
    Completed(Completion),
    Refused(Error),
    Failed(Error),
    Cancelled(CancelReason),
    UnknownOutcome,
}

/// KN5.21's commit record. `event` is the durable action; `revision.head` is the resulting head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitRecord {
    pub event: EventId,
    pub pre: String,
    pub post: String,
    pub revision: RevisionRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub kind: CompletionKind,
    pub revision: RevisionRef,
    pub result: Option<ResultRef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    Query,
    PreviewReady,
    Workspace,
    File,
    App,
}

/// What a completion produced: a job result's digest, a preview, or the durable workspace action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultRef {
    pub job: Option<JobId>,
    pub preview: Option<u64>,
    pub action: Option<EventId>,
    pub digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CancelReason {
    ByUser,
    Superseded(String),
    ReplacedRevision,
    Budget,
}

/// KN7.3's job outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobOutcome {
    Completed,
    Cancelled(CancelReason),
    CutOff(&'static str),
    Error(Error),
}

/// KN7.3's job result, bound to its inputs, its parameters and what it covered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobResult {
    pub command: String,
    pub job: JobId,
    pub outcome: JobOutcome,
    /// `Digest<labels>` of the first input (draft: the subject a shown revision is matched against).
    pub input: String,
    pub inputs: Vec<RevisionRef>,
    /// The request digest of the submission.
    pub params: String,
    pub budget: Amounts,
    /// The meter's terminal record: what was used and where a stop happened. `None` when the runtime was lost.
    pub summary: Option<Summary>,
    /// The canonical output bytes; only a completed job has them (KN7.4, KN7.11).
    pub value: Option<Vec<u8>>,
}

/// Where a snapshot is opened.
#[derive(Clone, Debug)]
pub enum At {
    Current,
    Revision(RevisionRef),
    Checkpoint(String),
}

/// An opened read-only snapshot of one revision (ONE.76). It never changes, whatever the session does next.
#[derive(Clone, Debug)]
pub struct SnapshotHandle {
    pub id: u64,
    pub revision: RevisionRef,
    pub(crate) state: Snapshot,
}

impl SnapshotHandle {
    pub fn state(&self) -> &Snapshot {
        &self.state
    }
}

/// The pending preview as the shell sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewInfo {
    pub id: u64,
    /// `Digest<literal word>`.
    pub literal: String,
    pub start: String,
    /// The end state's digest; `None` when a protected orbit would move.
    pub end: Option<String>,
    pub fresh: bool,
}

/// R-13's status, answered from memory: it reads no storage, whatever the history holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overview {
    pub revision: RevisionRef,
    /// The latest durable action's order.
    pub order: u64,
    pub read_only: Option<&'static str>,
    pub pending_preview: Option<u64>,
    pub lines: usize,
    pub active_line: usize,
    pub checkpoints: usize,
    pub undo: Option<ActionHint>,
    pub redo: Option<ActionHint>,
    pub protected: Vec<usize>,
    pub commit_busy: bool,
    pub jobs_running: usize,
}
