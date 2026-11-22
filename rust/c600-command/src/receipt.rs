//! Receipts (KN5.21), revision references (KN0.2), preview dependencies (KN0.8), action hints (LI.9) and the codes
//! the command layer raises.

use c600_kernel::json::{self, Value};

/// The codes the command layer raises. All but the last group are registered in chapter 13.
pub mod code {
    pub const NO_FRESH_PREVIEW: &str = "KN5:stale.no_fresh_preview";
    pub const TOKEN_INVALID: &str = "KN5:auth.token_invalid";
    pub const TOKEN_USED: &str = "KN5:stale.token_used";
    pub const TOKEN_MISMATCH: &str = "KN5:stale.token_mismatch";
    pub const COMMIT_IN_FLIGHT: &str = "KN5:busy.commit_in_flight";
    pub const PREVIEW_REPLACED: &str = "KN5:stale.preview_replaced";
    pub const REVISION_CHANGED: &str = "KN5:stale.revision_changed";
    pub const STALE_FILTER_PREVIEW: &str = "KN5:stale.filter_preview";
    pub const STATE_HASH_MISMATCH: &str = "KN5:consistency.state_hash_mismatch";
    pub const ORBIT_WOULD_MOVE: &str = c600_kernel::codes::ORBIT_WOULD_MOVE;
    pub const COMMIT_FATE: &str = "KN5:unknown_outcome.commit_fate";
    pub const READ_ONLY_RECOVERY: &str = "KN5:unknown_outcome.read_only_recovery";
    pub const UNDO_SUPERSEDED: &str = "KN5:stale.undo_superseded";
    pub const CONFIRMATION_REQUIRED: &str = "KN5:input.confirmation_required";
    pub const COMMIT_ROW_REFUSED: &str = "KN5:capability.commit_row_refused";
    pub const OVERLAY_ROW_KIND: &str = "KN5:input.overlay_row_kind";
    pub const COMMAND_ID_REUSED: &str = "KN5:input.command_id_reused";
    pub const RESULT_GONE: &str = "KN5:stale.result_gone";
    pub const EPOCH_CHANGED: &str = "KN2:stale.epoch_changed";
    /// A journal event the store does not hold (KN4.3, ONE.17).
    pub const REVISION_GONE: &str = "KN2:stale.revision_gone";
    pub const LANE_FULL: &str = "KN7:busy.lane_full";
    pub const BUDGET_NOT_DECLARED: &str = "KN7:input.budget_not_declared";
    pub const BUDGET_CEILING: &str = "KN7:resource_limit.budget_ceiling";
    pub const REVISION_REPLACED: &str = "KN7:cancelled.revision_replaced";
    pub const BY_USER: &str = "KN7:cancelled.by_user";
    pub const NOT_GRANTED: &str = "LG3:capability.not_granted";
    pub const UNKNOWN_SESSION: &str = "KN4:input.unknown_session";
    pub const PERSISTENCE_FORMAT: &str = "PS1:input.persistence_format";
    /// A provenance block the host's reader refuses (PS1.16); the layer passes it on from [`crate::Provenance`].
    pub const PROVENANCE_INCOMPATIBLE: &str = "PS1:capability.provenance_incompatible";
    /// A draft that names a record that does not exist or does not hold a list of letters (M6's code).
    pub const NAMED_OBJECT_INVALID: &str = c600_kernel::codes::NAMED_OBJECT_INVALID;
    /// Jumble on without a valid retained witness: labels alone (A5, ONE.54, JI.25).
    pub const JUMBLE_ON_LABELS_ONLY: &str = "KN5:input.jumble_on_labels_only";
    /// A row of the retained workflow, or `jumble.on`, with Jumble on (LG2.8; draft for `jumble.on`).
    pub const RETAINED_TOOL_NEEDS_JUMBLE_OFF: &str = "KN5:input.retained_tool_needs_jumble_off";
    /// Jumble off with a piece off the lattice: N > 0 (A5, JI.27).
    pub const JUMBLE_OFF_PIECES_OFF_LATTICE: &str = "KN5:input.jumble_off_pieces_off_lattice";
    /// Jumble off on a lattice configuration whose ν is not zero (A5, JI.27).
    pub const JUMBLE_OFF_NU_NONZERO: &str = "KN5:input.jumble_off_nu_nonzero";
    /// Jumble off where the poses are not shown reachable: no witnessed chain, or a displaced centre (A5, JI.27).
    pub const JUMBLE_OFF_NOT_SHOWN_REACHABLE: &str = "KN5:input.jumble_off_not_shown_reachable";
    /// A language-defined row whose required kernel row the table does not hold (LI.20).
    pub const OVERLAY_DEPENDENCY_MISSING: &str = "KN5:input.overlay_dependency_missing";

    /// Drafts that chapter 13 does not register yet.
    pub const UNKNOWN_ROW: &str = "KN5:input.unknown_row";
    pub const ARGUMENT_FORM: &str = "KN5:input.argument_form";
    pub const ROW_NOT_BUILT: &str = "KN5:input.row_not_built";
    pub const OVERLAY_ROW_CONFLICT: &str = "KN5:input.overlay_row_conflict";
    pub const SESSION_EXISTS: &str = "KN4:input.session_exists";
    /// A journal action that no solver action could have made, met at a reopen's replay: a commit whose caller is not
    /// the shell, a commit that moves a protected orbit under the protection and policy then in force, or a release of
    /// a protected orbit without its confirmation (finding B7 of the KN5 boundary attack; the owner's reading of
    /// 18 November 2026). The session opens in read-only recovery before it (KN5.23).
    pub const UNADMITTED_JOURNAL_ACTION: &str = "KN5:consistency.unadmitted_journal_action";
    /// A run's `operation.preview.cancel` of the preview the solver is shown: only the solver's action ends a shown
    /// preview (findings B1 and L17 of the KN5 boundary attack, KN5.12; the owner's reading of 18 November 2026). A
    /// run's preview may replace it since ADR-D01.9 ([`PREVIEW_SETTLING`]).
    pub const PREVIEW_SHOWN: &str = "KN5:capability.preview_shown";
    /// A press of the commit key on a preview that replaced one the solver was shown, inside ADR-D01.9's settle interval
    /// (owner, 22 November 2026, item 4; draft until chapter 13 registers it): the press is ignored before its token is
    /// spent, and a press after the interval commits.
    pub const PREVIEW_SETTLING: &str = "KN5:busy.preview_settling";
    /// A run's `operation.preview` outside a command the solver invoked: a preview is prepared only inside one (finding
    /// B2 of the KN5 boundary attack, ONE.34; the owner's reading of 18 November 2026). The check is on, so a host must
    /// start its runs inside a solver command.
    pub const OUTSIDE_SOLVER_COMMAND: &str = "KN5:capability.outside_solver_command";
    /// A `set.save` under a name that holds a saved set, without `replace: true` (U1-057: named-set replacement stays
    /// explicit; [`crate::sets`]).
    pub const SAVED_SET_EXISTS: &str = "KN5:input.saved_set_exists";
    /// A commit row while ADR-D01.39's explore state is on (owner, 22 November 2026, item 4; draft until chapter 13
    /// registers it): the explore state pauses commits and never makes one. The refusal comes before the token is
    /// spent.
    pub const COMMITS_PAUSED: &str = "KN5:capability.commits_paused";
}

/// An error with its chapter 13 code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    pub detail: String,
}

impl Error {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Error {
        Error { code, detail: detail.into() }
    }

    pub fn to_json(&self) -> Value {
        json::object([("code", Value::String(self.code.into())), ("detail", Value::String(self.detail.clone()))])
    }
}

impl From<c600_kernel::Refusal> for Error {
    fn from(refusal: c600_kernel::Refusal) -> Error {
        Error { code: refusal.code, detail: refusal.message }
    }
}

/// KN0.2's revision reference. `identity` is the model ID of the Full identity block; `mode` is `jumble_off` here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub session: String,
    pub epoch: u64,
    pub identity: String,
    pub mode: &'static str,
    pub head: String,
    pub state: u64,
    pub workspace: u64,
    pub protection: u64,
    /// The digest of the state at `head`: the labels' digest with Jumble off, the pose digest with Jumble on.
    pub digest: String,
}

impl Revision {
    pub fn to_json(&self) -> Value {
        let nat = |n: u64| Value::Int(n as i64);
        json::object([
            ("session", Value::String(self.session.clone())),
            ("epoch", nat(self.epoch)),
            ("identity", Value::String(self.identity.clone())),
            ("mode", Value::String(self.mode.into())),
            ("head", Value::String(self.head.clone())),
            ("state", nat(self.state)),
            ("workspace", nat(self.workspace)),
            ("protection", nat(self.protection)),
            ("digest", Value::String(self.digest.clone())),
        ])
    }

    /// Reads [`Revision::to_json`]'s form strictly; the mode must be one of this layer's, Jumble off
    /// ([`crate::layer::MODE`]) or Jumble on ([`crate::layer::JUMBLE_ON_MODE`]).
    pub fn from_json(value: &Value) -> Option<Revision> {
        (value.as_object()?.len() == 9).then_some(())?;
        let text = |key: &str| value.get(key)?.as_str().map(str::to_owned);
        let nat = |key: &str| value.get(key)?.as_i64().and_then(|n| u64::try_from(n).ok());
        let mode = [crate::layer::MODE, crate::layer::JUMBLE_ON_MODE];
        let mode = mode.into_iter().find(|mode| Some(*mode) == value.get("mode").and_then(Value::as_str))?;
        Some(Revision {
            session: text("session")?,
            epoch: nat("epoch")?,
            identity: text("identity")?,
            mode,
            head: text("head")?,
            state: nat("state")?,
            workspace: nat("workspace")?,
            protection: nat("protection")?,
            digest: text("digest")?,
        })
    }
}

/// KN0.8's preview dependencies. A preview is fresh when its dependencies equal the current ones exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewDeps {
    pub epoch: u64,
    pub identity: String,
    pub mode: &'static str,
    pub state: u64,
    pub protection: u64,
    /// The digest of the draft argument the preview was built from.
    pub draft: String,
}

impl PreviewDeps {
    pub fn to_json(&self) -> Value {
        json::object([
            ("epoch", Value::Int(self.epoch as i64)),
            ("identity", Value::String(self.identity.clone())),
            ("mode", Value::String(self.mode.into())),
            ("state", Value::Int(self.state as i64)),
            ("protection", Value::Int(self.protection as i64)),
            ("draft", Value::String(self.draft.clone())),
        ])
    }
}

/// The direction of `kernel.undo_hint`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Undo,
    Redo,
}

/// LI.9's action hint: the current durable action-order number and the action to invert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionHint {
    pub order: u64,
    pub target: u64,
}

impl ActionHint {
    pub fn to_json(&self) -> Value {
        json::object([("order", Value::Int(self.order as i64)), ("target", Value::Int(self.target as i64))])
    }

    pub fn from_json(value: &Value) -> Option<ActionHint> {
        let field = |name| value.get(name).and_then(Value::as_i64).and_then(|n| u64::try_from(n).ok());
        let object = value.as_object()?;
        (object.len() == 2).then_some(())?;
        Some(ActionHint { order: field("order")?, target: field("target")? })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitRecord {
    pub event: String,
    pub pre: String,
    pub post: String,
    pub revision: Revision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    Query,
    PreviewReady,
    Workspace,
    ViewPublished,
    File,
    App,
}

impl CompletionKind {
    pub fn name(self) -> &'static str {
        match self {
            CompletionKind::Query => "query",
            CompletionKind::PreviewReady => "preview_ready",
            CompletionKind::Workspace => "workspace",
            CompletionKind::ViewPublished => "view_published",
            CompletionKind::File => "file",
            CompletionKind::App => "app",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultRef {
    pub job: Option<u64>,
    pub digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub kind: CompletionKind,
    pub revision: Revision,
    pub result: Option<ResultRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CancelReason {
    ByUser,
    Superseded(String),
    ReplacedRevision,
    Budget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Accepted,
    Running(u64),
    Committed(CommitRecord),
    Completed(Completion),
    Refused(Error),
    Failed(Error),
    Cancelled(CancelReason),
    UnknownOutcome,
}

impl Status {
    /// The status's name in KN5.21.
    pub fn name(&self) -> &'static str {
        match self {
            Status::Accepted => "accepted",
            Status::Running(_) => "running",
            Status::Committed(_) => "committed",
            Status::Completed(_) => "completed",
            Status::Refused(_) => "refused",
            Status::Failed(_) => "failed",
            Status::Cancelled(_) => "cancelled",
            Status::UnknownOutcome => "unknown_outcome",
        }
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self, Status::Accepted | Status::Running(_))
    }

    /// The code of a refusal or failure.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Status::Refused(error) | Status::Failed(error) => Some(error.code),
            _ => None,
        }
    }
}

/// KN5.21's receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub command: String,
    pub request: String,
    pub job: Option<u64>,
    pub status: Status,
}

impl Receipt {
    pub fn to_json(&self) -> Value {
        let status = match &self.status {
            Status::Accepted | Status::UnknownOutcome => {
                json::object([("name", Value::String(self.status.name().into()))])
            }
            Status::Running(job) => {
                json::object([("name", Value::String("running".into())), ("job", Value::Int(*job as i64))])
            }
            Status::Committed(record) => json::object([
                ("name", Value::String("committed".into())),
                ("event", Value::String(record.event.clone())),
                ("pre", Value::String(record.pre.clone())),
                ("post", Value::String(record.post.clone())),
                ("revision", record.revision.to_json()),
            ]),
            Status::Completed(completion) => json::object([
                ("name", Value::String("completed".into())),
                ("kind", Value::String(completion.kind.name().into())),
                ("revision", completion.revision.to_json()),
                (
                    "result",
                    completion.result.as_ref().map_or(Value::Null, |result| {
                        json::object([
                            ("job", result.job.map_or(Value::Null, |job| Value::Int(job as i64))),
                            ("digest", Value::String(result.digest.clone())),
                        ])
                    }),
                ),
            ]),
            Status::Refused(error) | Status::Failed(error) => {
                json::object([("name", Value::String(self.status.name().into())), ("error", error.to_json())])
            }
            Status::Cancelled(reason) => {
                let reason = match reason {
                    CancelReason::ByUser => Value::String("by_user".into()),
                    CancelReason::Superseded(by) => json::object([("superseded", Value::String(by.clone()))]),
                    CancelReason::ReplacedRevision => Value::String("replaced_revision".into()),
                    CancelReason::Budget => Value::String("budget".into()),
                };
                json::object([("name", Value::String("cancelled".into())), ("reason", reason)])
            }
        };
        json::object([
            ("command", Value::String(self.command.clone())),
            ("request", Value::String(self.request.clone())),
            ("job", self.job.map_or(Value::Null, |job| Value::Int(job as i64))),
            ("status", status),
        ])
    }
}

/// The SHA-256 of a value's canonical JSON, in hex.
pub fn digest(value: &Value) -> String {
    c600_digest::sha256_hex(value.canonical().as_bytes())
}
