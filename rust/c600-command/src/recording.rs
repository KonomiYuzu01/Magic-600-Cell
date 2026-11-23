//! Semantic recording, playback and conversion (LP4-9; chapter 04 EV4.10 to EV4.23; chapter 05 PS1.30 and PS1.44).
//!
//! The command layer captures ([`crate::CommandLayer::invoke`]) and plays ([`crate::CommandLayer::run_jobs`]); this
//! module holds the data and the tables they read, with no state of its own:
//!
//! - [`RecordedStep`] and [`Recording`]: EV4.10's and PS1.30's ordered steps, in the draft form of
//!   `c600-recording-draft` (reserved, FP.4): stable row IDs, resolved canonical arguments with no token, no operation
//!   preview ID and no confirmation (old execution permission, EV4.10), the word a word step took with its provenance
//!   block, the revisions the step read, an overlay step's module digest, the durable action it took, the originating
//!   command and request, and its terminal receipt or none (EV4.11).
//! - [`PlaybackState`]: PS1.44's record with FP.37's payload fields, outside the undo order.
//! - [`eligibility`]: EV4.22's table with EV4.18's user steps, enumerable over any row.
//! - [`historical`] and [`child`]: EV4.19's two tables.
//! - [`play_from`]: EV4.22's `recording.play` state table; [`mapped`]: EV4.18's action map before an undo or redo.
//! - [`convert`]: EV4.23's conversion to a draft and to source, which writes nothing unless every step passes.
//! - [`Recorder`]: the durable home of recordings and playback states, outside the journal's action order
//!   (PS1.44: "outside the undo order"), which the host supplies ([`crate::CommandLayer::set_recorder`]);
//!   [`MemoryRecorder`] keeps the same contract in memory.
//!
//! # Drafts
//!
//! Chapter 05 reserves `c600-recording-draft` and gives FP.37 the playback record's wire form; neither the recorder's
//! store nor the layer's JSON forms here are frozen. Each choice below is a draft of LP4-9.
//!
//! - A `RecordingRef` is the recording's name, a text (H-08 leaves its form open). A name holds one recording; a
//!   second capture under it replaces the first, and its playback state then names other bytes and is not continued
//!   (PS1.44: the recording digest keeps a saved cursor from applying to edited bytes).
//! - A step's `word` is `{"recipe": [...], "source": draft|null, "provenance": block|null}` for a retained word (the
//!   kernel's recipe steps; the draft as the preview read it; the record's provenance block, or none for a word typed
//!   in), and `{"menu": {"cap", "letter"}}` for a pose move with Jumble on, which no draft holds yet.
//! - `depends` holds the revision the layer was at when the step was invoked, then each `RevisionRef` argument.
//! - `outcome` is the terminal receipt in [`Receipt::to_json`]'s form, kept unchanged.
//! - Capture keeps the shell's invocations only: a run's invocation is part of the program job the shell invoked,
//!   which is itself a step.
//! - A playback child's command ID is `{playback}/play/{epoch}/{step}/{n}`; a manual step takes the shell's own ID
//!   (EV4.20).
//! - The integrator's additions to EV4.22's table (both make a user step, never an unattended one): a row whose change
//!   the layer confirms itself (`protection.policy.set`, `protection.orbit.set`, a row with an action guard), since a
//!   recording stores no confirmation and finding LD1's reading of 18 November 2026 keeps a policy change the
//!   solver's own confirmed press; and a `recording.*` row, which capture never stores.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use c600_kernel::json::{self, Value};

use crate::receipt::{code, digest, Error, Status};
use crate::store::Fate;
use crate::table::{Kind, Lane, Row, Table, UndoClass};

/// EV4.22's row that starts capture.
pub const RECORDING_START_ROW: &str = "recording.start";
/// EV4.22's row that ends capture and stores the recording.
pub const RECORDING_STOP_ROW: &str = "recording.stop";
/// EV4.22's row that plays a recording; a run inside its job never replaces a preview (ADR-D01.9).
pub const RECORDING_PLAY_ROW: &str = "recording.play";
/// EV4.23's row that converts a recording to a draft or to source.
pub const RECORDING_CONVERT_ROW: &str = "recording.convert";
/// H-08's row that halts a job; the layer builds it for a playback's job (EV4.22).
pub const JOB_STOP_ROW: &str = "job.stop";
/// FP.4's reserved descriptor of a recording (PS1.12).
pub const RECORDING_FORMAT: &str = "c600-recording-draft";
/// FP.37's identifier of a playback state (PS1.12, PS1.44).
pub const PLAYBACK_FORMAT: &str = "c600-playback-draft";
/// The version of both draft forms.
pub const VERSION: i64 = 1;
/// The record a draft conversion writes, followed by the recording's name (draft).
pub const DRAFT_PREFIX: &str = "draft/recording/";
/// The record a source conversion writes, followed by the recording's name (draft).
pub const SOURCE_PREFIX: &str = "source/recording/";
/// The source edition a conversion to source writes (chapter 02's draft edition).
pub const SOURCE_EDITION: &str = "6cs-draft-1";

/// Whether `row` is one of the four `recording.*` rows, which capture never stores (EV4.11).
pub fn recording_row(row: &str) -> bool {
    matches!(row, RECORDING_START_ROW | RECORDING_STOP_ROW | RECORDING_PLAY_ROW | RECORDING_CONVERT_ROW)
}

fn nat(value: &Value) -> Option<u64> {
    value.as_i64().and_then(|n| u64::try_from(n).ok())
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn optional<T>(value: &Value, read: impl Fn(&Value) -> Option<T>) -> Option<Option<T>> {
    match value {
        Value::Null => Some(None),
        other => read(other).map(Some),
    }
}

// ----- recorded steps and recordings -----

/// EV4.10's `RecordedStep` in PS1.30's form (draft JSON; see the module's drafts).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedStep {
    /// The stable command-table row ID, distinct from the invocation's command ID.
    pub row: String,
    /// The resolved canonical arguments: no token, no operation preview ID and no confirmation.
    pub arguments: Value,
    /// The word a word step took (EV4.18, EV4.23), with its provenance block; `None` for any other step.
    pub word: Option<Value>,
    /// The revisions the step read: the layer's revision at the invocation, then each `RevisionRef` argument.
    pub depends: Vec<Value>,
    /// An overlay step's module digest (the active version when it was invoked); `None` for any other step.
    pub module: Option<String>,
    /// The durable action the invocation took (LI.9); EV4.23 matches undo and redo by it.
    pub action: Option<u64>,
    /// The originating invocation's command ID (KN5.21).
    pub command: String,
    /// The originating invocation's request digest; it cannot authorize a new one.
    pub request: String,
    /// The terminal receipt in [`crate::Receipt::to_json`]'s form, or `None` while no terminal outcome is known.
    pub outcome: Option<Value>,
}

impl RecordedStep {
    pub fn to_json(&self) -> Value {
        let option = |value: &Option<Value>| value.clone().unwrap_or(Value::Null);
        json::object([
            ("row", text(&self.row)),
            ("arguments", self.arguments.clone()),
            ("word", option(&self.word)),
            ("depends", Value::Array(self.depends.clone())),
            ("module", self.module.as_deref().map_or(Value::Null, text)),
            ("action", self.action.map_or(Value::Null, |n| Value::Int(n as i64))),
            ("command", text(&self.command)),
            ("request", text(&self.request)),
            ("outcome", option(&self.outcome)),
        ])
    }

    /// Reads [`RecordedStep::to_json`]'s form strictly: every field and no other, the arguments an object.
    pub fn from_json(value: &Value) -> Option<RecordedStep> {
        (value.as_object()?.len() == 9).then_some(())?;
        let arguments = value.get("arguments")?.clone();
        arguments.as_object()?;
        let outcome = optional(value.get("outcome")?, |v| v.get("status")?.get("name").map(|_| v.clone()))?;
        Some(RecordedStep {
            row: value.get("row")?.as_str()?.to_owned(),
            arguments,
            word: optional(value.get("word")?, |v| v.as_object().map(|_| v.clone()))?,
            depends: value.get("depends")?.as_array()?.to_vec(),
            module: optional(value.get("module")?, |v| v.as_str().map(str::to_owned))?,
            action: optional(value.get("action")?, nat)?,
            command: value.get("command")?.as_str()?.to_owned(),
            request: value.get("request")?.as_str()?.to_owned(),
            outcome,
        })
    }

    /// The name of the recorded outcome's status (KN5.21), or `None` with no terminal record.
    pub fn status(&self) -> Option<&str> {
        self.outcome.as_ref()?.get("status")?.get("name")?.as_str()
    }

    /// The retained letters of a word step, from its word's recipe: each word step's moves in order; a star step has
    /// no literal letters here, so a word with one has no draft form (`None`).
    pub fn letters(&self) -> Option<Vec<i64>> {
        let recipe = self.word.as_ref()?.get("recipe")?.as_array()?;
        let mut letters = Vec::new();
        for step in recipe {
            (step.get("kind")?.as_str()? == "word").then_some(())?;
            letters.extend(step.get("moves")?.as_array()?.iter().map(Value::as_i64).collect::<Option<Vec<_>>>()?);
        }
        Some(letters)
    }

    /// The word's provenance block, when the word came from a workspace record that holds one.
    pub fn block(&self) -> Option<&str> {
        self.word.as_ref()?.get("provenance")?.as_str()
    }

    /// Whether the step is a word step (EV4.23): a committed `operation.execute` with its stored word, or a committed
    /// `twist.apply` or `jumble.twist.menu` with its one letter.
    pub fn is_word_step(&self) -> bool {
        matches!(self.row.as_str(), "operation.execute" | "twist.apply" | "jumble.twist.menu")
            && self.status() == Some("committed")
            && self.word.is_some()
    }

    /// The target an undo or redo step's recorded hint named (LI.9), if the step carries one.
    pub fn hint_target(&self) -> Option<u64> {
        nat(self.arguments.get("hint")?.get("target")?)
    }
}

/// A recording (PS1.30): its name, the session and model identity it was made under, the revision capture started at,
/// and its ordered steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recording {
    pub name: String,
    pub session: String,
    /// KN0.1's identity block in [`crate::identity_json`]'s form; a playback under another refuses (EV4.14).
    pub identity: Value,
    /// The revision capture started at, as recorded context (PS1.30: a process-bound revision is no live grant).
    pub started: Value,
    pub steps: Vec<RecordedStep>,
}

impl Recording {
    pub fn to_json(&self) -> Value {
        json::object([
            ("format", text(RECORDING_FORMAT)),
            ("version", Value::Int(VERSION)),
            ("name", text(&self.name)),
            ("session", text(&self.session)),
            ("identity", self.identity.clone()),
            ("started", self.started.clone()),
            ("steps", Value::Array(self.steps.iter().map(RecordedStep::to_json).collect())),
        ])
    }

    /// Reads [`Recording::to_json`]'s form strictly: the format and version first (PS1.13), then every field and no
    /// other.
    pub fn from_json(value: &Value) -> Option<Recording> {
        (value.get("format")?.as_str()? == RECORDING_FORMAT && value.get("version")?.as_i64()? == VERSION)
            .then_some(())?;
        (value.as_object()?.len() == 7).then_some(())?;
        let steps = value.get("steps")?.as_array()?.iter().map(RecordedStep::from_json).collect::<Option<_>>()?;
        Some(Recording {
            name: value.get("name")?.as_str()?.to_owned(),
            session: value.get("session")?.as_str()?.to_owned(),
            identity: value.get("identity")?.clone(),
            started: value.get("started")?.clone(),
            steps,
        })
    }

    /// `Digest<recording>`: the SHA-256 of the canonical JSON, which a playback state names (PS1.44).
    pub fn digest(&self) -> String {
        digest(&self.to_json())
    }
}

// ----- the playback state -----

/// PS1.44's pending child: the command and request written before its dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub command: String,
    pub request: String,
}

/// One entry of PS1.44's `actions` map: the recorded action and the action the played step took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionEntry {
    pub recorded: u64,
    pub played: u64,
}

/// EV4.19's typed child outcome a playback stops with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildOutcome {
    Refused,
    Failed,
    Cancelled,
    UnknownOutcome,
}

impl ChildOutcome {
    pub fn name(self) -> &'static str {
        match self {
            ChildOutcome::Refused => "refused",
            ChildOutcome::Failed => "failed",
            ChildOutcome::Cancelled => "cancelled",
            ChildOutcome::UnknownOutcome => "unknown_outcome",
        }
    }

    pub fn from_name(name: &str) -> Option<ChildOutcome> {
        [ChildOutcome::Refused, ChildOutcome::Failed, ChildOutcome::Cancelled, ChildOutcome::UnknownOutcome]
            .into_iter()
            .find(|outcome| outcome.name() == name)
    }
}

/// PS1.44's `PlaybackStop`: a stop with a code of its own, or EV4.19's typed child outcome, whose code stays in the
/// unchanged receipt in `last`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    Code(String),
    Outcome(ChildOutcome),
}

/// PS1.44's `status`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayStatus {
    Running,
    Waiting(u64),
    Stopped(Stop),
    Finished,
}

impl PlayStatus {
    /// FP.37's `Status` form.
    pub fn to_json(&self) -> Value {
        match self {
            PlayStatus::Running => json::object([("kind", text("running"))]),
            PlayStatus::Waiting(step) => json::object([("kind", text("waiting")), ("step", Value::Int(*step as i64))]),
            PlayStatus::Stopped(stop) => {
                let stop = match stop {
                    Stop::Code(code) => json::object([("kind", text("code")), ("code", text(code))]),
                    Stop::Outcome(outcome) => {
                        json::object([("kind", text("outcome")), ("outcome", text(outcome.name()))])
                    }
                };
                json::object([("kind", text("stopped")), ("stop", stop)])
            }
            PlayStatus::Finished => json::object([("kind", text("finished"))]),
        }
    }

    pub fn from_json(value: &Value) -> Option<PlayStatus> {
        let fields = value.as_object()?.len();
        match (value.get("kind")?.as_str()?, fields) {
            ("running", 1) => Some(PlayStatus::Running),
            ("finished", 1) => Some(PlayStatus::Finished),
            ("waiting", 2) => Some(PlayStatus::Waiting(nat(value.get("step")?)?)),
            ("stopped", 2) => {
                let stop = value.get("stop")?;
                let fields = stop.as_object()?.len();
                match (stop.get("kind")?.as_str()?, fields) {
                    ("code", 2) => Some(PlayStatus::Stopped(Stop::Code(stop.get("code")?.as_str()?.to_owned()))),
                    ("outcome", 2) => Some(PlayStatus::Stopped(Stop::Outcome(ChildOutcome::from_name(
                        stop.get("outcome")?.as_str()?,
                    )?))),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// PS1.44's `PlaybackState`, with FP.37's payload fields; `last` is the receipt in [`crate::Receipt::to_json`]'s
/// form, unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackState {
    pub playback: String,
    /// `Digest<recording>` of the recording it plays.
    pub recording: String,
    /// The next recorded step.
    pub step: u64,
    pub pending: Option<Pending>,
    /// Sorted by `recorded`, each recorded number once.
    pub actions: Vec<ActionEntry>,
    pub status: PlayStatus,
    pub last: Option<Value>,
}

impl PlaybackState {
    /// A new playback at `step` with an empty map, running.
    pub fn new(playback: &str, recording: &str, step: u64) -> PlaybackState {
        PlaybackState {
            playback: playback.to_owned(),
            recording: recording.to_owned(),
            step,
            pending: None,
            actions: Vec::new(),
            status: PlayStatus::Running,
            last: None,
        }
    }

    /// The action the played step took for recorded action `recorded`, from the map (EV4.18).
    pub fn played(&self, recorded: u64) -> Option<u64> {
        self.actions.iter().find(|entry| entry.recorded == recorded).map(|entry| entry.played)
    }

    /// Enters `recorded -> played` in the map, keeping it sorted with each recorded number once: a later play of the
    /// same recorded step in the same playback replaces its entry.
    pub fn map(&mut self, recorded: u64, played: u64) {
        self.actions.retain(|entry| entry.recorded != recorded);
        let at = self.actions.partition_point(|entry| entry.recorded < recorded);
        self.actions.insert(at, ActionEntry { recorded, played });
    }

    pub fn to_json(&self) -> Value {
        let pending = self
            .pending
            .as_ref()
            .map_or(Value::Null, |p| json::object([("command", text(&p.command)), ("request", text(&p.request))]));
        let actions = self
            .actions
            .iter()
            .map(|e| {
                json::object([("recorded", Value::Int(e.recorded as i64)), ("played", Value::Int(e.played as i64))])
            })
            .collect();
        json::object([
            ("format", text(PLAYBACK_FORMAT)),
            ("version", Value::Int(VERSION)),
            ("playback", text(&self.playback)),
            ("recording", text(&self.recording)),
            ("step", Value::Int(self.step as i64)),
            ("pending", pending),
            ("actions", Value::Array(actions)),
            ("status", self.status.to_json()),
            ("last", self.last.clone().unwrap_or(Value::Null)),
        ])
    }

    /// Reads [`PlaybackState::to_json`]'s form strictly: the format and version first, then every field and no other;
    /// `actions` sorted by `recorded`, each once (FP.37).
    pub fn from_json(value: &Value) -> Option<PlaybackState> {
        (value.get("format")?.as_str()? == PLAYBACK_FORMAT && value.get("version")?.as_i64()? == VERSION)
            .then_some(())?;
        (value.as_object()?.len() == 9).then_some(())?;
        let pending = optional(value.get("pending")?, |p| {
            (p.as_object()?.len() == 2).then_some(())?;
            Some(Pending {
                command: p.get("command")?.as_str()?.to_owned(),
                request: p.get("request")?.as_str()?.to_owned(),
            })
        })?;
        let actions: Vec<ActionEntry> = value
            .get("actions")?
            .as_array()?
            .iter()
            .map(|e| {
                (e.as_object()?.len() == 2).then_some(())?;
                Some(ActionEntry { recorded: nat(e.get("recorded")?)?, played: nat(e.get("played")?)? })
            })
            .collect::<Option<_>>()?;
        actions.windows(2).all(|pair| pair[0].recorded < pair[1].recorded).then_some(())?;
        Some(PlaybackState {
            playback: value.get("playback")?.as_str()?.to_owned(),
            recording: value.get("recording")?.as_str()?.to_owned(),
            step: nat(value.get("step")?)?,
            pending,
            actions,
            status: PlayStatus::from_json(value.get("status")?)?,
            last: optional(value.get("last")?, |v| v.get("status").map(|_| v.clone()))?,
        })
    }
}

// ----- EV4.22's eligibility table -----

/// Why a row plays unattended (EV4.17, EV4.22).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unattended {
    /// Kind `view` or `workspace`, lane `view` or `none`, no `confirm` or `file_check` gate, no job: the owner's rule.
    ViewOrWorkspace,
    /// Kind `query`, lane `none`, no job and no `confirm` or `file_check` gate: the integrator's default.
    QueryDefault,
}

/// Why a row is a user step (EV4.18, EV4.22).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserReason {
    /// The commit lane, `session.checkpoint.save` included.
    CommitLane,
    /// A `confirm` gate.
    Confirm,
    /// A `file_check` gate.
    FileCheck,
    /// A `file` or `app` row.
    FileOrApp,
    /// The exclusive lane.
    ExclusiveLane,
    /// A job row.
    Job,
    /// A row whose change the layer confirms itself, or that has an action guard (the integrator's addition; see
    /// the module's drafts).
    Confirmation,
    /// A `recording.*` row, which capture never stores.
    Recording,
    /// Any other row and lane combination: no unattended permission.
    Other,
}

/// EV4.22's eligibility of a row for unattended playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eligibility {
    Unattended(Unattended),
    User(UserReason),
}

impl Eligibility {
    pub fn unattended(self) -> bool {
        matches!(self, Eligibility::Unattended(_))
    }
}

/// The rows whose change the layer confirms itself (LI.26, finding LD1): a recording stores no confirmation, so they
/// wait for the solver's own press.
fn confirmed_by_layer(row: &Row) -> bool {
    row.id == crate::layer::POLICY_ROW || row.id == "protection.orbit.set" || row.action_guard.is_some()
}

/// EV4.22's table, applied in its order: the user-step rows of EV4.18 first, then the two unattended rows, then any
/// other combination. A `job` row is H-08's job flag: a stored row a built-in tool's entry serves keeps H-08's flag, and
/// a language-defined row is a job row. The table grants nothing: every ordinary gate still applies when the step plays.
pub fn eligibility(row: &Row) -> Eligibility {
    let user = Eligibility::User;
    if row.lane == Lane::Commit || row.kind == Kind::Commit {
        return user(UserReason::CommitLane);
    }
    if row.has_gate("confirm") {
        return user(UserReason::Confirm);
    }
    if row.has_gate("file_check") {
        return user(UserReason::FileCheck);
    }
    if matches!(row.kind, Kind::File | Kind::App) {
        return user(UserReason::FileOrApp);
    }
    if row.lane == Lane::Exclusive {
        return user(UserReason::ExclusiveLane);
    }
    if row.job {
        return user(UserReason::Job);
    }
    if recording_row(&row.id) {
        return user(UserReason::Recording);
    }
    if confirmed_by_layer(row) {
        return user(UserReason::Confirmation);
    }
    match (row.kind, row.lane) {
        (Kind::View | Kind::Workspace, Lane::View | Lane::None) => Eligibility::Unattended(Unattended::ViewOrWorkspace),
        (Kind::Query, Lane::None) => Eligibility::Unattended(Unattended::QueryDefault),
        _ => user(UserReason::Other),
    }
}

// ----- EV4.19's two tables -----

/// EV4.19's first table: what a recorded outcome lets playback do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Historical {
    /// `completed`: plays if eligible under EV4.22; otherwise a user step.
    Completed,
    /// `committed`: a user step.
    Committed,
    /// `refused`, `failed`, `cancelled`: stays visible, never plays, is passed over.
    PassedOver,
    /// `accepted`, `running`, no terminal record, `unknown_outcome`: playback stops at this step with
    /// `EV4:input.recording_step_outcome_unknown`.
    Unknown,
}

pub fn historical(step: &RecordedStep) -> Historical {
    match step.status() {
        Some("completed") => Historical::Completed,
        Some("committed") => Historical::Committed,
        Some("refused" | "failed" | "cancelled") => Historical::PassedOver,
        _ => Historical::Unknown,
    }
}

/// EV4.19's second table: what a new child outcome does to the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    /// `accepted`, `running`: pending, no advance.
    Pending,
    /// The cursor advances.
    Advances,
    /// `committed` from an unattended step: not an unattended step under EV4.18; no automatic advance.
    Held,
    /// Stops with that typed outcome; no automatic retry.
    Stops(ChildOutcome),
}

pub fn child(status: &Status, unattended: bool) -> Advance {
    match status {
        Status::Accepted | Status::Running(_) => Advance::Pending,
        Status::Completed(_) => Advance::Advances,
        Status::Committed(_) if unattended => Advance::Held,
        Status::Committed(_) => Advance::Advances,
        Status::Refused(_) => Advance::Stops(ChildOutcome::Refused),
        Status::Failed(_) => Advance::Stops(ChildOutcome::Failed),
        Status::Cancelled(_) => Advance::Stops(ChildOutcome::Cancelled),
        Status::UnknownOutcome => Advance::Stops(ChildOutcome::UnknownOutcome),
    }
}

// ----- EV4.22's state table and EV4.18's map -----

/// Where `recording.play` starts (EV4.22).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// A new playback at this step, with an empty map; it replaces the recording's state record.
    New(u64),
    /// The same playback, its cursor at this step and its map kept.
    Same(u64),
}

/// EV4.22's `recording.play` table, read after pending reconciliation: `state` is the recording's state record (none
/// when it names other recording bytes), `child_live` whether a child is still `accepted` or `running`, `steps` the
/// recording's length and `from` the explicit start. Every refusal is `EV4:input.recording_state`.
pub fn play_from(
    state: Option<&PlaybackState>,
    child_live: bool,
    steps: u64,
    from: Option<u64>,
) -> Result<Start, Error> {
    let refuse = |detail: &str| Err(Error::new(code::RECORDING_STATE, detail.to_owned()));
    if from.is_some_and(|f| f >= steps) {
        return refuse("`from` must name a step of the recording (0 <= from < steps)");
    }
    let Some(state) = state else {
        return Ok(Start::New(from.unwrap_or(0)));
    };
    if child_live || state.status == PlayStatus::Running {
        return refuse("a child of the playback is still accepted or running, or the playback runs");
    }
    match (&state.status, from) {
        (PlayStatus::Waiting(_) | PlayStatus::Stopped(Stop::Code(_)), _)
        | (
            PlayStatus::Stopped(Stop::Outcome(ChildOutcome::Refused | ChildOutcome::Failed | ChildOutcome::Cancelled)),
            _,
        ) => Ok(match from {
            None => Start::Same(state.step),
            Some(f) if f >= state.step => Start::Same(f),
            Some(f) => Start::New(f),
        }),
        (PlayStatus::Stopped(Stop::Outcome(ChildOutcome::UnknownOutcome)) | PlayStatus::Finished, Some(f)) => {
            Ok(Start::New(f))
        }
        (PlayStatus::Stopped(Stop::Outcome(ChildOutcome::UnknownOutcome)), None) => {
            refuse("the playback stopped on an unknown outcome; only an explicit `from` opens a new one")
        }
        (PlayStatus::Finished, None) => refuse("the playback finished; only an explicit `from` opens a new one"),
        (PlayStatus::Running, _) => unreachable!("a running playback was refused above"),
    }
}

/// EV4.18's map before an undo or redo user step: the hint's target must be the played action that the map gives for
/// the recorded `hint.target`. A missing entry or another target stops with `EV4:stale.recording_step`, and nothing is
/// undone.
pub fn mapped(actions: &[ActionEntry], recorded_target: u64, hint_target: Option<u64>) -> Result<u64, Error> {
    let played = actions.iter().find(|entry| entry.recorded == recorded_target).map(|entry| entry.played);
    match (played, hint_target) {
        (Some(played), Some(hint)) if played == hint => Ok(played),
        (None, _) => Err(Error::new(
            code::STALE_RECORDING_STEP,
            format!("the recorded target {recorded_target} has no played action in this playback"),
        )),
        (Some(played), hint) => Err(Error::new(
            code::STALE_RECORDING_STEP,
            format!(
                "the current hint targets {hint:?}, not {played}, which recorded action {recorded_target} played as"
            ),
        )),
    }
}

// ----- EV4.23's conversion -----

/// The target of `recording.convert`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Draft,
    Source,
}

impl Target {
    pub fn name(self) -> &'static str {
        match self {
            Target::Draft => "draft",
            Target::Source => "source",
        }
    }

    pub fn from_name(name: &str) -> Option<Target> {
        match name {
            "draft" => Some(Target::Draft),
            "source" => Some(Target::Source),
            _ => None,
        }
    }
}

/// A conversion that every step passed (EV4.23): the word's letters in recorded order, the contributing words'
/// provenance blocks (BND.24: the new block lists them as parts), the report of steps that contribute nothing, the
/// calls of a source conversion and its generated text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversion {
    pub target: Target,
    pub letters: Vec<i64>,
    /// The provenance block of each word that remains in the result, in recorded order, each once.
    pub parts: Vec<String>,
    /// `{"step", "row", "contributes"}` for each step the target reads, in recorded order.
    pub report: Vec<Value>,
    /// A source conversion's calls in recorded order: `{"step", "row", "arguments", "guards"}`.
    pub calls: Vec<Value>,
    /// A source conversion's generated module text; `None` for a draft.
    pub source: Option<String>,
}

/// EV4.23: applies EV4.19 first (a step with no terminal outcome stops the conversion with
/// `EV4:input.recording_step_outcome_unknown`; refused, failed and cancelled steps are left out), then checks every
/// remaining step against the target; a step with no form in it refuses the whole conversion with
/// `EV4:input.recording_not_convertible`, naming the step and the target. `invocable` says whether an overlay row may
/// invoke a row (KN5.4). Nothing is written by this function; the layer writes the result only after it returns.
pub fn convert(
    recording: &Recording,
    target: Target,
    table: &Table,
    invocable: impl Fn(&Row) -> bool,
) -> Result<Conversion, Error> {
    let refuse = |index: usize, step: &RecordedStep, why: &str| {
        let detail = format!("step {index} (`{}`) has no form in target `{}`: {why}", step.row, target.name());
        Err(Error::new(code::RECORDING_NOT_CONVERTIBLE, detail))
    };
    // Each word step's letters, keyed by its recorded index, with whether an undo removed it.
    let mut words: Vec<(usize, Vec<i64>, Option<String>, bool)> = Vec::new();
    let mut report = Vec::new();
    let mut calls = Vec::new();
    let word_at = |words: &[(usize, Vec<i64>, Option<String>, bool)], index: usize| {
        words.iter().position(|(at, ..)| *at == index)
    };
    for (index, step) in recording.steps.iter().enumerate() {
        match historical(step) {
            Historical::PassedOver => continue,
            Historical::Unknown => {
                let detail = format!("step {index} (`{}`) has no known terminal outcome", step.row);
                return Err(Error::new(code::RECORDING_STEP_OUTCOME_UNKNOWN, detail));
            }
            Historical::Completed | Historical::Committed => {}
        }
        let entry = |contributes: &str| {
            json::object([
                ("step", Value::Int(index as i64)),
                ("row", text(&step.row)),
                ("contributes", text(contributes)),
            ])
        };
        if recording_row(&step.row) {
            return refuse(index, step, "a recording row is never captured");
        }
        let Some(row) = table.row(&step.row) else {
            return refuse(index, step, "the row is not in the current table");
        };
        // A word step adds its letters at its place.
        if step.is_word_step() {
            let Some(letters) = step.letters() else {
                return refuse(index, step, "its word holds no retained letters (a menu word or a star)");
            };
            report.push(entry("letters"));
            words.push((index, letters, step.block().map(str::to_owned), false));
            continue;
        }
        // An undo or redo of either history (ADR-D01.14 under X1, owner, 23 November 2026, decision 2.7).
        if matches!(row.id.as_str(), "session.undo" | "session.redo" | "layout.undo" | "layout.redo") {
            let undo = row.id.ends_with(".undo");
            let Some(target_action) = step.hint_target() else {
                return refuse(index, step, "its recorded hint names no target");
            };
            let found = recording.steps[..index].iter().position(|s| s.action == Some(target_action));
            match found {
                Some(at) if recording.steps[at].is_word_step() => {
                    let Some(position) = word_at(&words, at) else {
                        return refuse(index, step, "it targets a word step this conversion left out");
                    };
                    words[position].3 = undo;
                    report.push(entry(if undo { "removes" } else { "restores" }));
                }
                Some(_) if target == Target::Draft => report.push(entry("nothing")),
                Some(_) => return refuse(index, step, "it undoes or redoes a step that is no word step"),
                None => return refuse(index, step, "it targets an action outside this recording"),
            }
            continue;
        }
        if row.id == "session.checkpoint.save" {
            if target == Target::Draft {
                report.push(entry("nothing"));
                continue;
            }
            return refuse(index, step, "a checkpoint save is the solver's commit");
        }
        if row.kind == Kind::Commit || row.lane == Lane::Commit {
            return refuse(index, step, "the position changed in a way no letter states");
        }
        match row.kind {
            Kind::File | Kind::App => {
                if target == Target::Source {
                    return refuse(index, step, "a file or app row is never a source call");
                }
                report.push(entry("nothing"));
            }
            Kind::View | Kind::Query | Kind::Workspace => {
                if target == Target::Source {
                    let gated = row.has_gate("confirm") || row.has_gate("file_check");
                    if gated || !invocable(row) {
                        return refuse(
                            index,
                            step,
                            "no overlay row may invoke it, or it has a confirm or file_check gate",
                        );
                    }
                    calls.push(json::object([
                        ("step", Value::Int(index as i64)),
                        ("row", text(&step.row)),
                        ("arguments", step.arguments.clone()),
                        ("guards", Value::Array(step.depends.clone())),
                    ]));
                    report.push(entry("call"));
                } else {
                    report.push(entry("nothing"));
                }
            }
            Kind::Commit => unreachable!("a commit row was refused above"),
        }
    }
    let mut letters = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    for (_, word, block, removed) in &words {
        if *removed {
            continue;
        }
        letters.extend(word.iter().copied());
        if let Some(block) = block {
            if !parts.contains(block) {
                parts.push(block.clone());
            }
        }
    }
    if target == Target::Draft && letters.is_empty() {
        let detail = format!("the draft word of `{}` is empty, so target `draft` has nothing to hold", recording.name);
        return Err(Error::new(code::RECORDING_NOT_CONVERTIBLE, detail));
    }
    let source = (target == Target::Source).then(|| source_text(recording, &letters, &calls));
    Ok(Conversion { target, letters, parts, report, calls, source })
}

/// The module a source conversion writes (EV4.13): its entry proposes the whole word for a fresh preview, and each
/// recorded call is named with its command, absolute arguments and revision guards. The draft edition has no
/// primitive that invokes a stored view or workspace row, so a call is named in a comment and runs nothing; the
/// conversion's `calls` carry them as data.
fn source_text(recording: &Recording, letters: &[i64], calls: &[Value]) -> String {
    let digest = recording.digest();
    let mut text = format!("edition \"{SOURCE_EDITION}\";\nmodule recording_{} requires [];\n\n", &digest[..12]);
    text.push_str(&format!(
        "// Converted from the recording `{}` ({RECORDING_FORMAT} sha256 {digest}) under EV4.13 and EV4.23.\n",
        recording.name.replace('\n', " ")
    ));
    for call in calls {
        let step = call.get("step").and_then(Value::as_i64).unwrap_or(-1);
        let row = call.get("row").and_then(Value::as_str).unwrap_or("");
        let arguments = call.get("arguments").map(Value::canonical).unwrap_or_default();
        let guards: Vec<String> = call
            .get("guards")
            .and_then(Value::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(|guard| guard.get("digest").and_then(Value::as_str).map(str::to_owned))
            .collect();
        text.push_str(&format!("// call {step}: `{row}` {arguments}; revision guards {guards:?}\n"));
    }
    if letters.is_empty() {
        text.push_str("export fn replay() -> Int effects [] {\n    return 0;\n}\n");
        return text;
    }
    let word: Vec<String> = letters.iter().map(i64::to_string).collect();
    text.push_str(&format!(
        "export command replay() -> record {{ base: Text, id: Text }} effects [preview] {{\n    \
         let r = propose(word[{}], target(\"operation.preview\"));\n    \
         return record {{ base = receipt_base(r), id = receipt_id(r) }};\n}}\n",
        word.join(", ")
    ));
    text
}

/// The draft conversion's undo class: one undoable workspace action (EV4.23), whatever the table holds.
pub const CONVERT_UNDO: UndoClass = UndoClass::Workspace;

// ----- the recorder's store -----

/// What a recorder holds: the stored recordings by name and the playback states by recording name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recorded {
    pub recordings: BTreeMap<String, Recording>,
    pub playbacks: BTreeMap<String, PlaybackState>,
}

/// The durable home of recordings and playback states (PS1.30, PS1.44), outside the journal's action order: a write
/// here takes no action number and creates no undo target. Each write is one recoverable write: a playback state's
/// cursor, pending entry, map, status and last receipt land together. LP2's session store implements it on a disk of
/// its own (`c600_session::recorder::DiskRecorder`); [`MemoryRecorder`] keeps it in memory.
pub trait Recorder: Send {
    /// Stores a recording under its name, replacing one of the same name.
    fn save_recording(&mut self, recording: &Recording) -> Fate;
    /// Saves the playback state of the recording named `name`, replacing the one it held.
    fn save_playback(&mut self, name: &str, state: &PlaybackState) -> Fate;
    /// Everything it holds, as the last durable writes left it.
    fn load(&mut self) -> Result<Recorded, Error>;
}

#[derive(Debug, Default)]
struct Memory {
    durable: Recorded,
    inject: VecDeque<Fate>,
    unknown_writes: bool,
    writes: u64,
}

/// The in-memory recorder. Clones share what it holds, so a test keeps a handle after the layer took one, injects the
/// fates of the next writes and hands the same store to a reopened layer.
#[derive(Clone, Debug, Default)]
pub struct MemoryRecorder(Arc<Mutex<Memory>>);

impl MemoryRecorder {
    pub fn new() -> MemoryRecorder {
        MemoryRecorder::default()
    }

    fn memory(&self) -> MutexGuard<'_, Memory> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The fates of the next writes, in order; durable once empty.
    pub fn inject(&self, fate: Fate) {
        self.memory().inject.push_back(fate);
    }

    /// Whether an injected unknown fate still lands, as a write whose acknowledgement was lost.
    pub fn set_unknown_writes(&self, lands: bool) {
        self.memory().unknown_writes = lands;
    }

    /// What it holds durably.
    pub fn held(&self) -> Recorded {
        self.memory().durable.clone()
    }

    /// The number of writes asked of it.
    pub fn writes(&self) -> u64 {
        self.memory().writes
    }

    /// Replaces what it holds, as a store read from another place would (a file the host loads).
    pub fn put(&self, recorded: Recorded) {
        self.memory().durable = recorded;
    }

    fn write(&self, apply: impl FnOnce(&mut Recorded)) -> Fate {
        let mut memory = self.memory();
        memory.writes += 1;
        let fate = memory.inject.pop_front().unwrap_or(Fate::Durable);
        if fate == Fate::Durable || (fate == Fate::Unknown && memory.unknown_writes) {
            apply(&mut memory.durable);
        }
        fate
    }
}

impl Recorder for MemoryRecorder {
    fn save_recording(&mut self, recording: &Recording) -> Fate {
        let recording = recording.clone();
        self.write(|held| {
            held.recordings.insert(recording.name.clone(), recording);
        })
    }

    fn save_playback(&mut self, name: &str, state: &PlaybackState) -> Fate {
        let state = state.clone();
        self.write(|held| {
            held.playbacks.insert(name.to_owned(), state);
        })
    }

    fn load(&mut self) -> Result<Recorded, Error> {
        Ok(self.held())
    }
}

/// The host's reader of user modules' active versions (EV1.7, EV4.10, EV4.14): the hex SHA-256 of the closure a module
/// name activates, or `None` with none active. The layer records an overlay step's module digest with it and checks it
/// before an overlay step plays; the host binds the step's job to the recorded digest, which its own module set checks
/// again (`c600_tools::modules::bind_job`).
pub trait ModuleVersions: Send + Sync {
    fn active(&self, module: &str) -> Option<String>;
}
