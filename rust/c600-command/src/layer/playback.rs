//! The command layer's side of capture and playback (LP4-9; chapter 04 EV4.10 to EV4.23; chapter 05 PS1.30 and
//! PS1.44): the four `recording.*` rows, `job.stop` for a playback's job, the capture hook on every invocation of the
//! shell, the playback job the layer runs itself, the shell's step action and the recovery of playback states. The
//! data and the tables are in [`crate::recording`].
//!
//! - Capture sits in [`CommandLayer::invoke`]: an invocation of the shell's channel whose row resolves becomes one
//!   step, read before its dispatch and completed with its receipt and its action at `recording.stop`. A run's
//!   invocation is not captured: it is part of the program job the solver invoked, which is a step itself. A retry
//!   that the outcome lookup answers is no new step. A `recording.*` row is never captured (EV4.11).
//! - `recording.start`, `recording.stop` and `recording.play` complete as transient answers and take no action number,
//!   and a playback state's writes go to the recorder, outside the journal's action order: none of them is an undo
//!   target (EV4.20, EV4.21). Only `recording.convert` writes the journal, one record write that `session.undo`
//!   reaches.
//! - The playback job holds the program lane while it is live and runs in [`CommandLayer::run_jobs`]. Each step it
//!   plays is a new invocation on the shell's channel with no token and a command ID of its own
//!   (`{playback}/play/{epoch}/{step}/{n}`), written to the state as `pending` before it is dispatched; while it is
//!   dispatched, the layer dispatches no row but the eligible one it was checked for ([`CommandLayer::dispatch`]).
//! - A user step waits for the shell's step action ([`crate::Shell::step`]): the layer checks it against the recorded
//!   step and EV4.18's map and writes `pending` ([`CommandLayer::begin_step`]), then the shell sends the ordinary row
//!   with the solver's token. An invocation outside that action does not move the cursor.
//! - A child's receipt is reconciled after the invocation that settles it, after a program job's report and at each
//!   `recording.play`; at [`CommandLayer::set_recorder`] and at a restart, the outcome lookup (KN5.23) settles every
//!   pending child first, and a state still `running` with no live job stops with `KN5:resource_limit.engine_lost`
//!   (draft). An unknown outcome stops the playback and is never retried.

use std::collections::{BTreeMap, BTreeSet};

use c600_kernel::json::{self, Value};

use super::{refused, steps_json, CommandLayer, Invocation, Key, ProgramJob};
use crate::forms;
use crate::jumble::JUMBLE_TWIST_ROW;
use crate::layer::{Caller, LAYOUT_REDO_ROW, LAYOUT_UNDO_ROW};
use crate::receipt::{
    code, digest, ActionHint, CancelReason, Completion, CompletionKind, Error, Receipt, ResultRef, Status,
};
use crate::recording::{
    self, child, eligibility, historical, mapped, play_from, recording_row, Advance, ChildOutcome, Conversion,
    Historical, ModuleVersions, Pending, PlayStatus, PlaybackState, RecordedStep, Recorder, Recording, Start, Stop,
    Target, DRAFT_PREFIX, SOURCE_PREFIX,
};
use crate::store::{identity_json, Fate, Store};
use crate::table::Row;
use crate::twist::TWIST_ROW;
use crate::worksheet::Worksheet;
use crate::workspace::RecordValue;

/// A capture in progress: its name, the revision it started at, and its steps with the receipt key each step's outcome
/// and action are read from at `recording.stop`.
pub(super) struct Capture {
    name: String,
    started: Value,
    steps: Vec<(RecordedStep, Key)>,
}

/// A live `recording.play` job: the recording it plays and its own receipt's key.
pub(super) struct Play {
    pub(super) name: String,
    key: Key,
}

/// A playback's child this process dispatched or is about to dispatch.
pub(super) struct Child {
    /// Whether the playback job dispatched it (an unattended step) rather than the shell's step action.
    unattended: bool,
    /// The module digest its recorded step names, which its program job binds (EV4.14).
    module: Option<String>,
}

/// The layer's capture and playback state.
#[derive(Default)]
pub(super) struct Rec {
    recorder: Option<Box<dyn Recorder>>,
    versions: Option<Box<dyn ModuleVersions>>,
    capture: Option<Capture>,
    recordings: BTreeMap<String, Recording>,
    playbacks: BTreeMap<String, PlaybackState>,
    pub(super) plays: BTreeMap<u64, Play>,
    children: BTreeMap<Key, Child>,
    /// Pending entries the shell's step action wrote and has not sent yet (EV4.20): no lookup settles them.
    pub(super) awaiting: BTreeSet<Key>,
    /// The action order each durable invocation took (LI.9), filled when the action is adopted, a replay's included.
    pub(super) took: BTreeMap<Key, u64>,
    /// The row an unattended step being dispatched may invoke, while it is dispatched.
    playing: Option<String>,
    /// Set while the layer plays or reconciles, so the invocations it makes do not reconcile again inside it.
    advancing: bool,
    /// The children issued in this layer's life, numbering their command IDs.
    issued: u64,
    /// The last conversion that every step passed, with its report (EV4.23; draft): transient.
    conversion: Option<(String, Conversion)>,
}

fn text<'a>(args: &'a BTreeMap<String, Value>, name: &str) -> &'a str {
    args.get(name).and_then(Value::as_str).unwrap_or_default()
}

fn nat(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_i64).and_then(|n| u64::try_from(n).ok())
}

/// The undo and redo rows of either history (ADR-D01.14), whose user step EV4.18's map checks.
fn hinted(row: &str) -> bool {
    matches!(row, "session.undo" | "session.redo" | LAYOUT_UNDO_ROW | LAYOUT_REDO_ROW)
}

/// Whether argument `name` of `row` names a revision (`RevisionRef`), which the shell fills with the current one at a
/// user step (EV4.18).
fn revision_ref(row: &Row, name: &str) -> bool {
    let ty = row.arguments.get(name).map(String::as_str).unwrap_or_default();
    ty.strip_suffix('?').unwrap_or(ty) == "RevisionRef"
}

/// The arguments a step stores (EV4.10, PS1.30): the invocation's, without a token, an operation preview ID or a
/// confirmation, which is old execution permission.
fn stored_arguments(row: &Row, args: &Value) -> Value {
    let Some(fields) = args.as_object() else { return args.clone() };
    let kept = fields.iter().filter(|(name, _)| {
        let ty = row.arguments.get(*name).map(String::as_str).unwrap_or_default();
        *name != "confirm" && ty != "UserActionToken" && ty != "PreviewId"
    });
    Value::Object(kept.map(|(name, value)| (name.clone(), value.clone())).collect())
}

impl<S: Store> CommandLayer<S> {
    // ----- the host's interface -----

    /// Installs the host's recorder (PS1.30, PS1.44) and recovers what it holds: every pending child is looked up
    /// first ([`CommandLayer::kernel_outcome`], KN5.23) and reconciled at most once; an unknown outcome stops its
    /// playback without a retry; a state still `running` with no live job stops with `KN5:resource_limit.engine_lost`
    /// (draft). Each recovered state is written once. Without a recorder, the four `recording.*` rows and `job.stop`
    /// are refused with `KN5:input.row_not_built`. A recorder that cannot be read is refused with its error and not
    /// installed.
    pub fn set_recorder(&mut self, mut recorder: Box<dyn Recorder>) -> Result<(), Error> {
        let held = recorder.load()?;
        self.rec.recordings = held.recordings;
        self.rec.playbacks = held.playbacks;
        self.rec.recorder = Some(recorder);
        self.rec.plays.clear();
        self.rec.awaiting.clear();
        self.recover_playbacks();
        Ok(())
    }

    /// Installs the host's reader of user modules' active versions (EV1.7, EV4.14). Without one, an overlay step
    /// records no module digest and playback checks none, and the host's binding of the step's job is the only check.
    pub fn set_module_versions(&mut self, versions: Box<dyn ModuleVersions>) {
        self.rec.versions = Some(versions);
    }

    /// The recordings the recorder holds, by name.
    pub fn recordings(&self) -> &BTreeMap<String, Recording> {
        &self.rec.recordings
    }

    /// The recording named `name`.
    pub fn recording(&self, name: &str) -> Option<&Recording> {
        self.rec.recordings.get(name)
    }

    /// The playback state of the recording named `name` (PS1.44), as last written.
    pub fn playback(&self, name: &str) -> Option<&PlaybackState> {
        self.rec.playbacks.get(name)
    }

    /// The name of the capture in progress.
    pub fn capturing(&self) -> Option<&str> {
        self.rec.capture.as_ref().map(|capture| capture.name.as_str())
    }

    /// The steps captured so far, without their outcomes, which `recording.stop` reads.
    pub fn captured(&self) -> Vec<&RecordedStep> {
        self.rec.capture.as_ref().map_or_else(Vec::new, |capture| capture.steps.iter().map(|(step, _)| step).collect())
    }

    /// The last conversion of a recording that every step passed, and the recording's name (EV4.23): its report lists
    /// what each step contributed. It is transient.
    pub fn conversion(&self) -> Option<(&str, &Conversion)> {
        self.rec.conversion.as_ref().map(|(name, conversion)| (name.as_str(), conversion))
    }

    /// The recorded row and arguments of the user step that playback `name` waits at (EV4.20), for the shell's step
    /// action, which fills in the current hint, the presented preview and a confirmation, and mints a token.
    pub fn step_arguments(&self, name: &str) -> Result<(String, Value), Error> {
        let (_, state, step) = self.waiting_step(name)?;
        let _ = state;
        Ok((step.row.clone(), step.arguments.clone()))
    }

    /// The draft for the fresh whole-word preview of the word step that playback `name` waits at (EV4.18): the stored
    /// word's recipe, never the recorded preview. In a solving session the word's provenance block, when it has one,
    /// must pass the host's reader as a record's word does (KN5.26, BND.84), or the draft is refused with its error.
    pub fn step_draft(&self, name: &str) -> Result<Value, Error> {
        let (_, _, step) = self.waiting_step(name)?;
        let recipe = step.word.as_ref().and_then(|word| word.get("recipe")).cloned();
        let Some(recipe) = recipe.filter(|_| step.row == "operation.execute") else {
            let detail = format!("the step at the cursor is `{}`, which takes no word preview", step.row);
            return Err(Error::new(code::RECORDING_STATE, detail));
        };
        if let (Some(block), crate::store::Profile::Solving) = (step.block(), self.profile) {
            let unread = || Error::new(code::NOT_GRANTED, "a recorded word's block needs the host's provenance reader");
            self.provenance.as_ref().ok_or_else(unread)?.admit_kept(block.as_bytes(), &self.blocks)?;
        }
        Ok(json::object([("recipe", recipe)]))
    }

    /// EV4.20's step action, the layer's half: checks `invocation`, which the shell prepared with its own command ID
    /// and the solver's token, against the user step playback `name` waits at, then writes `pending` durably before the
    /// shell sends it. Refused, writing nothing: no recorder (`KN5:input.row_not_built`); no such recording or state
    /// (`KN2:input.named_object_invalid`); a playback that does not wait, a child still pending, another row or other
    /// recorded arguments (`EV4:input.recording_state`); a changed model identity or module
    /// (`EV4:input.recording_identity_mismatch`); a word step whose presented preview is not the stored word's
    /// (`EV4:stale.recording_step`). An undo or redo whose hint does not target the action EV4.18's map gives stops
    /// the playback with `EV4:stale.recording_step`, undoing nothing.
    pub fn begin_step(&mut self, name: &str, invocation: &Invocation) -> Result<(), Error> {
        let (recording, state, step) = self.waiting_step(name)?;
        let (recording, state, step) = (recording.clone(), state.clone(), step.clone());
        self.same_identity(&recording)?;
        if invocation.row != step.row || invocation.epoch != self.revision.epoch {
            let detail = format!("the step at the cursor is `{}` in this epoch", step.row);
            return Err(Error::new(code::RECORDING_STATE, detail));
        }
        // What only the press fixes (EV4.18): the hint, the presented preview, a confirmation and the current revision.
        let row = self.table.row(&step.row).cloned();
        let fresh = |name: &str| {
            matches!(name, "hint" | "preview" | "confirm") || row.as_ref().is_some_and(|row| revision_ref(row, name))
        };
        let given = invocation.args.as_object().cloned().unwrap_or_default();
        let recorded = step.arguments.as_object().cloned().unwrap_or_default();
        let same = given.iter().filter(|(n, _)| !fresh(n)).all(|(n, v)| recorded.get(n) == Some(v))
            && recorded.iter().filter(|(n, _)| !fresh(n)).all(|(n, _)| given.contains_key(n));
        if !same {
            return Err(Error::new(code::RECORDING_STATE, "the step action submits the recorded arguments"));
        }
        if let Some(row) = row {
            if let Err(error) = self.module_identity(&step, &row) {
                self.stop(name, &state, Stop::Code(error.code.to_owned()));
                return Err(error);
            }
        }
        if hinted(&step.row) {
            let hint = invocation.args.get("hint").and_then(ActionHint::from_json).map(|hint| hint.target);
            let Some(target) = step.hint_target() else {
                return Err(Error::new(code::RECORDING_STEP_UNRESOLVED, "the recorded undo or redo names no target"));
            };
            if let Err(error) = mapped(&state.actions, target, hint) {
                self.stop(name, &state, Stop::Code(error.code.to_owned()));
                return Err(error);
            }
        }
        if step.row == "operation.execute" {
            let preview = nat(invocation.args.get("preview"));
            let shown = self.slot.as_ref().filter(|p| Some(p.id) == preview);
            let word = step.word.as_ref().and_then(|word| word.get("recipe"));
            if !shown.is_some_and(|p| Some(&Value::Array(steps_json(&p.recipe, false))) == word) {
                let detail = "the presented preview is not a fresh preview of the stored word";
                return Err(Error::new(code::STALE_RECORDING_STEP, detail));
            }
        }
        let request = self.request(&Caller::Shell, invocation, &[]);
        let mut next = state.clone();
        next.pending = Some(Pending { command: invocation.command.clone(), request });
        match self.save_state(name, next) {
            Fate::Durable => {}
            Fate::RolledBack(error) => return Err(error),
            Fate::Unknown => {
                return Err(Error::new(code::COMMIT_FATE, "the recorder cannot say whether the pending entry landed"))
            }
        }
        let key = (Caller::Shell.scope(), invocation.command.clone());
        self.rec.children.insert(key.clone(), Child { unattended: false, module: step.module.clone() });
        self.rec.awaiting.insert(key);
        Ok(())
    }

    // ----- hooks the layer calls -----

    /// The gates of the recording rows and `job.stop`: the recorder, and for `recording.convert` the provenance reader.
    pub(super) fn recording_gates(&self, row: &Row) -> Result<(), Error> {
        if self.rec.recorder.is_none() {
            return Err(Error::new(code::ROW_NOT_BUILT, format!("`{}` needs the host's recorder", row.id)));
        }
        if row.id == recording::RECORDING_CONVERT_ROW && self.provenance.is_none() {
            return Err(Error::new(code::ROW_NOT_BUILT, format!("`{}` needs the host's provenance reader", row.id)));
        }
        Ok(())
    }

    /// While an unattended step is dispatched, only the eligible row it was checked for, with no token.
    pub(super) fn playing_admits(&self, row: &Row, invocation: &Invocation) -> Result<(), Error> {
        let Some(allowed) = &self.rec.playing else { return Ok(()) };
        if *allowed == row.id && eligibility(row).unattended() && invocation.token.is_none() {
            return Ok(());
        }
        let detail = format!("an unattended playback step never dispatches `{}` (EV4.12, EV4.17, EV4.22)", row.id);
        Err(Error::new(code::NOT_GRANTED, detail))
    }

    /// Whether `program` is an unattended step a playback dispatched: a run inside it is inside a `recording.play` job,
    /// and never replaces a preview (ADR-D01.9).
    pub(super) fn played_child(&self, program: &ProgramJob) -> bool {
        let key = (program.caller.scope(), program.command.clone());
        self.rec.children.get(&key).is_some_and(|child| child.unattended)
    }

    /// The module digest a playback child's job binds, when the invocation is one.
    pub(super) fn child_module(&self, caller: &Caller, command: &str) -> Option<String> {
        self.rec.children.get(&(caller.scope(), command.to_owned())).and_then(|child| child.module.clone())
    }

    /// Reads an invocation of the shell's channel into a step before it is dispatched (EV4.10, EV4.11, PS1.30); `None`
    /// when no capture runs, for a run's invocation, a row that does not resolve and a `recording.*` row.
    pub(super) fn capture_begin(
        &self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Option<RecordedStep> {
        self.rec.capture.as_ref()?;
        if !matches!(caller, Caller::Shell) || recording_row(&invocation.row) {
            return None;
        }
        let row = self.table.row(&invocation.row)?;
        let mut depends = vec![self.revision.to_json()];
        if let Some(fields) = invocation.args.as_object() {
            for (name, value) in fields {
                if revision_ref(row, name) {
                    depends.push(value.clone());
                }
            }
        }
        let word = match row.id.as_str() {
            "operation.execute" => {
                let preview = nat(invocation.args.get("preview"));
                self.slot.as_ref().filter(|p| Some(p.id) == preview).map(|p| {
                    let block = p.resolved.as_ref().and_then(|resolved| {
                        let read = resolved.reads.first()?;
                        self.records.get(&read.record)?.value.as_ref().map(|value| text_value(&value.provenance))
                    });
                    json::object([
                        ("recipe", Value::Array(steps_json(&p.recipe, false))),
                        ("source", p.resolved.as_ref().map_or(Value::Null, |resolved| resolved.source.clone())),
                        ("provenance", block.unwrap_or(Value::Null)),
                    ])
                })
            }
            JUMBLE_TWIST_ROW => {
                let field = |name: &str| invocation.args.get(name).cloned().unwrap_or(Value::Null);
                Some(json::object([("menu", json::object([("cap", field("cap")), ("letter", field("letter"))]))]))
            }
            _ => None,
        };
        let module = row.overlay.as_ref().and_then(|overlay| self.rec.versions.as_ref()?.active(&overlay.module));
        Some(RecordedStep {
            row: row.id.clone(),
            arguments: stored_arguments(row, &invocation.args),
            word,
            depends,
            module,
            action: None,
            command: invocation.command.clone(),
            request: request.to_owned(),
            outcome: None,
        })
    }

    /// Appends a captured step once its invocation was answered: a live twist's word is the head's recipe once it
    /// committed (EV4.23).
    pub(super) fn capture_end(&mut self, step: Option<RecordedStep>, key: Key) {
        let Some(mut step) = step else { return };
        if step.row == TWIST_ROW {
            let committed = self.receipts.get(&key).is_some_and(|r| matches!(r.receipt.status, Status::Committed(_)));
            let recipe = self.nodes.get(&self.head).and_then(|node| node.recipe.as_ref()).filter(|_| committed);
            step.word = recipe.map(|recipe| {
                json::object([
                    ("recipe", Value::Array(steps_json(recipe, false))),
                    ("source", Value::Null),
                    ("provenance", Value::Null),
                ])
            });
        }
        if let Some(capture) = self.rec.capture.as_mut() {
            capture.steps.push((step, key));
        }
    }

    /// Reconciles every pending child whose receipt settled, outside the playback job's own loop.
    pub(super) fn reconcile_children(&mut self) {
        if self.rec.advancing || self.rec.recorder.is_none() {
            return;
        }
        self.rec.advancing = true;
        let names: Vec<String> =
            self.rec.playbacks.iter().filter(|(_, s)| s.pending.is_some()).map(|(name, _)| name.clone()).collect();
        for name in names {
            if let Some(next) = self.reconciled(&name) {
                let current = self.rec.playbacks.get(&name).cloned();
                if let Some(current) = current {
                    let _ = self.write_reconciled(&name, &current, next);
                }
            }
        }
        self.rec.advancing = false;
    }

    /// Runs every live playback job until it waits for a child or ends; the number that moved.
    pub(super) fn run_plays(&mut self) -> usize {
        if self.rec.advancing {
            return 0;
        }
        let jobs: Vec<u64> = self.rec.plays.keys().copied().collect();
        let mut ran = 0;
        for job in jobs {
            self.rec.advancing = true;
            let moved = self.play_on(job);
            self.rec.advancing = false;
            ran += usize::from(moved);
        }
        ran
    }

    /// A restart ends the playback jobs (their jobs end with the epoch) and recovers every state as an open does.
    pub(super) fn end_plays(&mut self) {
        self.rec.plays.clear();
        self.rec.awaiting.clear();
        self.recover_playbacks();
    }

    // ----- the rows -----

    fn recorder_needed(&self, row: &Row) -> Result<(), Error> {
        self.recording_gates(row)
    }

    /// `recording.start {name}` (EV4.22): starts capture under `name`. A start while recording is refused with
    /// `EV4:input.recording_state`. Its answer is a transient `completed(workspace)` with no action.
    pub(super) fn recording_start(
        &mut self,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        forms::check_forms(row, args)?;
        self.recorder_needed(row)?;
        let name = text(args, "name");
        if name.is_empty() {
            return refused(code::ARGUMENT_FORM, "`name` is a nonempty text");
        }
        if let Some(capture) = &self.rec.capture {
            return refused(code::RECORDING_STATE, format!("`{}` is being recorded", capture.name));
        }
        let started = self.revision.to_json();
        self.rec.capture = Some(Capture { name: name.to_owned(), started, steps: Vec::new() });
        Ok((None, self.workspace_completion(None)))
    }

    /// `recording.stop {recording}` (EV4.22): ends the capture named `recording` and stores it, each step with its
    /// terminal receipt or none and the action it took (EV4.11). A stop with no capture of that name is refused with
    /// `EV4:input.recording_state`. Its answer is a transient `completed(workspace)` whose result digest is the
    /// recording's.
    pub(super) fn recording_stop(
        &mut self,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        forms::check_forms(row, args)?;
        self.recorder_needed(row)?;
        let name = text(args, "recording");
        if self.rec.capture.as_ref().is_none_or(|capture| capture.name != name) {
            return refused(code::RECORDING_STATE, format!("no recording named `{name}` is being recorded"));
        }
        let capture = self.rec.capture.take().expect("checked above");
        let steps = capture
            .steps
            .iter()
            .map(|(step, key)| RecordedStep {
                outcome: self.receipts.get(key).map(|record| record.receipt.to_json()),
                action: self.rec.took.get(key).copied(),
                ..step.clone()
            })
            .collect();
        let recording = Recording {
            name: capture.name.clone(),
            session: self.revision.session.clone(),
            identity: identity_json(&self.kernel.identity()),
            started: capture.started.clone(),
            steps,
        };
        let recorder = self.rec.recorder.as_mut().expect("checked above");
        match recorder.save_recording(&recording) {
            Fate::Durable => {
                let result = ResultRef { job: None, digest: recording.digest() };
                self.rec.recordings.insert(recording.name.clone(), recording);
                self.generation += 1;
                Ok((None, self.workspace_completion(Some(result))))
            }
            Fate::RolledBack(error) => {
                self.rec.capture = Some(capture);
                Ok((None, Status::Failed(error)))
            }
            Fate::Unknown => {
                self.rec.capture = Some(capture);
                Ok((None, Status::UnknownOutcome))
            }
        }
    }

    /// `recording.play {recording, from?}` (EV4.22): after the state's pending child is reconciled, EV4.22's table
    /// decides where the playback starts ([`recording::play_from`]), and a new playback replaces the recording's state.
    /// Refused, writing nothing: the forms (`KN5:input.argument_form`), except that a negative integer `from` is
    /// outside EV4.22's table (`EV4:input.recording_state`); no recorder (`KN5:input.row_not_built`); no
    /// recording of that name (`KN2:input.named_object_invalid`); another model identity, or a step from the start
    /// whose module is not the active version (`EV4:input.recording_identity_mismatch`); the table's refusals
    /// (`EV4:input.recording_state`); a full program lane (`KN7:busy.lane_full`). The state is written `running` before
    /// the job is admitted.
    pub(super) fn recording_play(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        // EV4.22: a negative integer `from` is outside the table, as one past the end is (the format packet's
        // `play-from-negative`), so it is refused with `EV4:input.recording_state` once the other names fit.
        let negative = args.get("from").and_then(Value::as_i64).is_some_and(|from| from < 0);
        let mut named = args.clone();
        if negative {
            named.remove("from");
        }
        forms::check_forms(row, &named)?;
        self.recorder_needed(row)?;
        if negative {
            return refused(code::RECORDING_STATE, "`from` must name a step of the recording (0 <= from < steps)");
        }
        let name = text(args, "recording").to_owned();
        let Some(recording) = self.rec.recordings.get(&name).cloned() else {
            return refused(code::NAMED_OBJECT_INVALID, format!("no recording named `{name}`"));
        };
        self.same_identity(&recording)?;
        // EV4.22: the state is read after its pending child is reconciled.
        if let Some(next) = self.reconciled(&name) {
            let current = self.rec.playbacks.get(&name).cloned().expect("a pending state");
            if let Fate::RolledBack(error) = self.write_reconciled(&name, &current, next) {
                return Ok((None, Status::Failed(error)));
            }
        }
        let digest = recording.digest();
        let state = self.rec.playbacks.get(&name).filter(|state| state.recording == digest).cloned();
        let live = state.as_ref().is_some_and(|state| state.pending.is_some())
            || self.rec.plays.values().any(|play| play.name == name);
        let start = play_from(state.as_ref(), live, recording.steps.len() as u64, nat(args.get("from")))?;
        let at = match start {
            Start::New(at) | Start::Same(at) => at as usize,
        };
        for step in &recording.steps[at..] {
            if let Some(row) = self.table.row(&step.row) {
                if let Err(error) = self.module_identity(step, row) {
                    if error.code == code::RECORDING_IDENTITY_MISMATCH {
                        return Err(error);
                    }
                }
            }
        }
        self.program_free()?;
        let mut next = match (start, state) {
            (Start::Same(at), Some(mut same)) => {
                same.step = at;
                same
            }
            (Start::New(at), _) | (Start::Same(at), None) => {
                PlaybackState::new(&format!("{name}#{}", invocation.command), &digest, at)
            }
        };
        next.status = PlayStatus::Running;
        match self.save_state(&name, next) {
            Fate::Durable => {}
            Fate::RolledBack(error) => return Ok((None, Status::Failed(error))),
            Fate::Unknown => return Ok((None, Status::UnknownOutcome)),
        }
        let id = self.next_job();
        self.rec.plays.insert(id, Play { name, key: (caller.scope(), invocation.command.clone()) });
        self.generation += 1;
        Ok((Some(id), Status::Running(id)))
    }

    /// `recording.convert {recording, target}` (EV4.23): writes the conversion as one record write, an undoable
    /// workspace action, only when every step passes ([`recording::convert`]): target `draft` writes
    /// `draft/recording/<name>`, an operation draft whose Macro phase holds the literal word, and target `source`
    /// writes `source/recording/<name>`, the generated module text with the calls. The record's provenance block is
    /// the host reader's ([`crate::Provenance::assemble`]) and lists every contributing word's block in its parts
    /// (BND.24). Refused, writing nothing: the forms; no recorder or reader (`KN5:input.row_not_built`); no such
    /// recording; another model identity; a step with no terminal outcome
    /// (`EV4:input.recording_step_outcome_unknown`); a step with no form in the target, or an empty draft word
    /// (`EV4:input.recording_not_convertible`); the reader's refusal of the block.
    pub(super) fn recording_convert(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        forms::check_forms(row, args)?;
        self.recorder_needed(row)?;
        let name = text(args, "recording").to_owned();
        let Some(recording) = self.rec.recordings.get(&name).cloned() else {
            return refused(code::NAMED_OBJECT_INVALID, format!("no recording named `{name}`"));
        };
        let Some(target) = Target::from_name(text(args, "target")) else {
            return refused(code::ARGUMENT_FORM, "`target` is `draft` or `source`");
        };
        self.same_identity(&recording)?;
        let conversion = recording::convert(&recording, target, &self.table, Self::runtime_may)?;
        let reader = self.provenance.as_ref().expect("the gate checked the reader");
        let block = reader.assemble(target.name(), conversion.source.as_deref(), &conversion.parts)?;
        reader.check(block.as_bytes())?;
        let (record, payload) = match target {
            Target::Draft => {
                let letters = Value::Array(conversion.letters.iter().map(|&letter| Value::Int(letter)).collect());
                let sheet = Worksheet::from_draft(&letters).expect("a list of letters is a draft");
                (format!("{DRAFT_PREFIX}{name}"), sheet.to_payload())
            }
            Target::Source => {
                let source = json::object([
                    ("edition", Value::String(recording::SOURCE_EDITION.to_owned())),
                    ("recording", Value::String(recording.digest())),
                    ("text", Value::String(conversion.source.clone().unwrap_or_default())),
                    ("calls", Value::Array(conversion.calls.clone())),
                ]);
                (format!("{SOURCE_PREFIX}{name}"), json::object([("source", source)]))
            }
        };
        let meta = super::Meta::of(caller, invocation, request);
        let adoption = self.plan_record(meta, record, RecordValue { payload, provenance: block });
        let answer = self.transact(adoption)?;
        if matches!(answer.1, Status::Completed(_) | Status::Committed(_)) {
            self.rec.conversion = Some((name, conversion));
        }
        Ok(answer)
    }

    /// `job.stop {job}` (EV4.22): halts a live playback job. Its state stops with `KN7:cancelled.by_user`, keeping a
    /// pending child for the next play's reconciliation, and the job's receipt is `cancelled(by_user)`; the answer is a
    /// transient `completed(app)`. A job that is not a live playback's is refused with `KN5:input.row_not_built`: the
    /// layer builds `job.stop` for a playback's job only (draft).
    pub(super) fn job_stop(
        &mut self,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        forms::check_forms(row, args)?;
        self.recorder_needed(row)?;
        let Some(job) = nat(args.get("job")) else {
            return refused(code::ARGUMENT_FORM, "`job` is a job ID");
        };
        let Some(play) = self.rec.plays.remove(&job) else {
            return refused(code::ROW_NOT_BUILT, format!("job {job} is not a live playback's; only those stop here"));
        };
        if let Some(state) = self.rec.playbacks.get(&play.name).cloned() {
            let mut next = state;
            next.status = PlayStatus::Stopped(Stop::Code(code::BY_USER.to_owned()));
            self.save_state(&play.name, next);
        }
        self.settle(&play.key, Status::Cancelled(CancelReason::ByUser));
        self.generation += 1;
        let completion = Completion { kind: CompletionKind::App, revision: self.revision.clone(), result: None };
        Ok((None, Status::Completed(completion)))
    }

    // ----- playing -----

    /// Plays live job `job` from its cursor until a user step, a pending child, a stop or the end (EV4.22). Whether it
    /// moved.
    fn play_on(&mut self, job: u64) -> bool {
        let mut moved = false;
        loop {
            let Some(name) = self.rec.plays.get(&job).map(|play| play.name.clone()) else { return moved };
            let (Some(state), Some(recording)) =
                (self.rec.playbacks.get(&name).cloned(), self.rec.recordings.get(&name).cloned())
            else {
                self.end_play(job, Status::Failed(Error::new(code::NAMED_OBJECT_INVALID, "the recording is gone")));
                return true;
            };
            if state.pending.is_some() {
                // The child is still accepted or running; its report reconciles it.
                if let Some(next) = self.reconciled(&name) {
                    if let Fate::RolledBack(error) = self.write_reconciled(&name, &state, next) {
                        self.end_play(job, Status::Failed(error));
                    }
                    moved = true;
                    continue;
                }
                return moved;
            }
            if state.status != PlayStatus::Running {
                let status = self.play_answer(&state);
                self.end_play(job, status);
                return true;
            }
            moved = true;
            let at = state.step as usize;
            let mut next = state.clone();
            let leaves = match recording.steps.get(at) {
                None => Some(PlayStatus::Finished),
                Some(step) => match historical(step) {
                    Historical::PassedOver => {
                        next.step += 1;
                        Some(PlayStatus::Running)
                    }
                    Historical::Unknown => {
                        Some(PlayStatus::Stopped(Stop::Code(code::RECORDING_STEP_OUTCOME_UNKNOWN.to_owned())))
                    }
                    Historical::Committed => Some(PlayStatus::Waiting(state.step)),
                    Historical::Completed => match self.table.row(&step.row) {
                        None => Some(PlayStatus::Stopped(Stop::Code(code::RECORDING_STEP_UNRESOLVED.to_owned()))),
                        Some(row) if !eligibility(row).unattended() => Some(PlayStatus::Waiting(state.step)),
                        Some(row) => {
                            match self.module_identity(step, row).and_then(|()| self.step_fresh(row, &step.arguments)) {
                                Err(error) => Some(PlayStatus::Stopped(Stop::Code(error.code.to_owned()))),
                                Ok(()) => None,
                            }
                        }
                    },
                },
            };
            if let Some(status) = leaves {
                next.status = status;
                if let Fate::RolledBack(error) = self.save_state(&name, next) {
                    self.end_play(job, Status::Failed(error));
                    return true;
                }
                continue;
            }
            let step = &recording.steps[at];
            let row = self.table.row(&step.row).cloned().expect("checked above");
            // The child: a new invocation with its own command ID and no token (EV4.13, EV4.22), written as pending
            // before it is dispatched (EV4.20).
            self.rec.issued += 1;
            let command = format!("{}/play/{}/{at}/{}", state.playback, self.revision.epoch, self.rec.issued);
            let mut args = step.arguments.clone();
            if let Value::Object(fields) = &mut args {
                fields.remove("confirm");
            }
            let budget = row.overlay.is_some().then(|| self.budget_ceiling());
            let invocation = Invocation {
                command: command.clone(),
                row: row.id.clone(),
                args,
                token: None,
                epoch: self.revision.epoch,
                budget,
            };
            let request = self.request(&Caller::Shell, &invocation, &[]);
            let mut next = state.clone();
            next.pending = Some(Pending { command: command.clone(), request });
            match self.save_state(&name, next) {
                Fate::Durable => {}
                Fate::RolledBack(error) => {
                    self.end_play(job, Status::Failed(error));
                    return true;
                }
                // The entry may have landed with no dispatch: the next reconciliation looks it up and stops on an
                // unknown outcome, never dispatching it (EV4.20).
                Fate::Unknown => {
                    self.end_play(job, Status::UnknownOutcome);
                    return true;
                }
            }
            let key = (Caller::Shell.scope(), command);
            self.rec.children.insert(key, Child { unattended: true, module: step.module.clone() });
            self.rec.playing = Some(row.id.clone());
            self.invoke_with_inputs(&Caller::Shell, invocation, Vec::new());
            self.rec.playing = None;
        }
    }

    /// The play job's receipt when its playback leaves `running` (draft): `completed(workspace)` with the state's
    /// digest when it waits or finished; `failed` with the stop's own code; `failed` with the child's refusal or
    /// failure forwarded, its code and detail unchanged (ERR.7); `cancelled` with the child's reason; `unknown_outcome`
    /// for a child whose outcome is unknown.
    fn play_answer(&self, state: &PlaybackState) -> Status {
        let result = ResultRef { job: None, digest: digest(&state.to_json()) };
        match &state.status {
            PlayStatus::Running | PlayStatus::Waiting(_) | PlayStatus::Finished => {
                self.workspace_completion(Some(result))
            }
            PlayStatus::Stopped(Stop::Code(stopped)) => {
                let known = [
                    code::STALE_RECORDING_STEP,
                    code::RECORDING_IDENTITY_MISMATCH,
                    code::RECORDING_STEP_UNRESOLVED,
                    code::RECORDING_STEP_OUTCOME_UNKNOWN,
                    code::BY_USER,
                    code::ENGINE_LOST,
                    code::COMMIT_FATE,
                ];
                let code = known.into_iter().find(|known| known == stopped).unwrap_or(code::RECORDING_STATE);
                Status::Failed(Error::new(code, format!("the playback stopped at step {}", state.step)))
            }
            PlayStatus::Stopped(Stop::Outcome(outcome)) => {
                let command = state.last.as_ref().and_then(|last| last.get("command")).and_then(Value::as_str);
                let child = command.and_then(|command| self.receipts.get(&(Caller::Shell.scope(), command.to_owned())));
                match (outcome, child.map(|record| &record.receipt.status)) {
                    (
                        ChildOutcome::Refused | ChildOutcome::Failed,
                        Some(Status::Refused(error) | Status::Failed(error)),
                    ) => Status::Failed(error.clone()),
                    (ChildOutcome::Cancelled, Some(Status::Cancelled(reason))) => Status::Cancelled(reason.clone()),
                    (ChildOutcome::Cancelled, _) => Status::Cancelled(CancelReason::ByUser),
                    _ => Status::UnknownOutcome,
                }
            }
        }
    }

    fn end_play(&mut self, job: u64, status: Status) {
        if let Some(play) = self.rec.plays.remove(&job) {
            self.settle(&play.key, status);
            self.generation += 1;
        }
    }

    fn stop(&mut self, name: &str, state: &PlaybackState, stop: Stop) {
        let mut next = state.clone();
        next.status = PlayStatus::Stopped(stop);
        self.save_state(name, next);
    }

    /// The user step playback `name` waits at: its recording, state and step.
    fn waiting_step(&self, name: &str) -> Result<(&Recording, &PlaybackState, &RecordedStep), Error> {
        if self.rec.recorder.is_none() {
            return Err(Error::new(code::ROW_NOT_BUILT, "the step action needs the host's recorder"));
        }
        let recording = self.rec.recordings.get(name);
        let state = self.rec.playbacks.get(name).filter(|s| recording.is_some_and(|r| r.digest() == s.recording));
        let (Some(recording), Some(state)) = (recording, state) else {
            return Err(Error::new(code::NAMED_OBJECT_INVALID, format!("no playback of a recording named `{name}`")));
        };
        if state.status != PlayStatus::Waiting(state.step) || state.pending.is_some() {
            return Err(Error::new(code::RECORDING_STATE, "the playback does not wait for a step"));
        }
        let step = recording
            .steps
            .get(state.step as usize)
            .filter(|step| matches!(historical(step), Historical::Completed | Historical::Committed));
        let Some(step) = step else {
            return Err(Error::new(code::RECORDING_STATE, "the cursor names no step to play"));
        };
        Ok((recording, state, step))
    }

    /// EV4.14: the model identity the recording was made under.
    fn same_identity(&self, recording: &Recording) -> Result<(), Error> {
        if recording.identity != identity_json(&self.kernel.identity()) {
            let detail = format!("`{}` was recorded under another model identity", recording.name);
            return Err(Error::new(code::RECORDING_IDENTITY_MISMATCH, detail));
        }
        Ok(())
    }

    /// EV4.14 for a step that recorded a module digest (PS1.30's `module`: a user module's active version when it was
    /// recorded; a built-in tool's entry records none, and the host binds its entry as before): the row must still run
    /// that module, and the digest must name its active version, or the step is unresolved (no row, no active version)
    /// or mismatched (another version). With no reader of the host's versions the layer checks nothing here, and the
    /// host's binding of the step's job to the recorded digest is the check ([`ProgramJob::module`]).
    fn module_identity(&self, step: &RecordedStep, row: &Row) -> Result<(), Error> {
        let Some(recorded) = &step.module else { return Ok(()) };
        let Some(overlay) = &row.overlay else {
            let detail = format!("`{}` no longer runs the module its step recorded", step.row);
            return Err(Error::new(code::RECORDING_STEP_UNRESOLVED, detail));
        };
        let Some(versions) = &self.rec.versions else { return Ok(()) };
        match versions.active(&overlay.module) {
            Some(active) if active == *recorded => Ok(()),
            Some(_) => {
                let detail = format!("`{}` is no longer the version `{}` recorded", overlay.module, step.row);
                Err(Error::new(code::RECORDING_IDENTITY_MISMATCH, detail))
            }
            None => {
                let detail = format!("no version of `{}` is active", overlay.module);
                Err(Error::new(code::RECORDING_STEP_UNRESOLVED, detail))
            }
        }
    }

    /// EV4.14 for an unattended step: a `RevisionRef` argument must name the current revision and a `FilterPreviewRef`
    /// a fresh filter preview, or the step stops as stale. Nothing is retargeted.
    fn step_fresh(&self, row: &Row, arguments: &Value) -> Result<(), Error> {
        let stale = |error: Error| Error::new(code::STALE_RECORDING_STEP, format!("{}: {}", error.code, error.detail));
        let fields = arguments.as_object().cloned().unwrap_or_default();
        forms::check_revisions(row, &fields, &self.revision).map_err(stale)?;
        self.fresh_filter_gate(row, arguments).map_err(stale)
    }

    // ----- reconciliation and recovery -----

    /// The state of playback `name` once its pending child's receipt is reconciled (EV4.19's second table, EV4.20), or
    /// `None` when nothing changes: no pending child, a pending entry the shell has not sent, or a child still accepted
    /// or running. The outcome lookup is KN5.23's: a command the layer never saw, or whose transient receipt a restart
    /// lost, is `unknown_outcome`, which stops the playback and is never retried.
    fn reconciled(&self, name: &str) -> Option<PlaybackState> {
        let state = self.rec.playbacks.get(name)?;
        let pending = state.pending.as_ref()?;
        let key = (Caller::Shell.scope(), pending.command.clone());
        if self.rec.awaiting.contains(&key) {
            return None;
        }
        let receipt: Receipt = self.kernel_outcome(&Caller::Shell, &pending.command, &pending.request);
        let recording = self.rec.recordings.get(name).filter(|r| r.digest() == state.recording);
        let step = recording.and_then(|r| r.steps.get(state.step as usize));
        let unattended =
            step.and_then(|step| self.table.row(&step.row)).is_some_and(|row| eligibility(row).unattended());
        let mut next = state.clone();
        match child(&receipt.status, unattended) {
            Advance::Pending => return None,
            Advance::Advances => {
                next.pending = None;
                next.last = Some(receipt.to_json());
                if let (Some(recorded), Some(&played)) = (step.and_then(|step| step.action), self.rec.took.get(&key)) {
                    next.map(recorded, played);
                }
                next.step += 1;
                if let PlayStatus::Waiting(_) = next.status {
                    let steps = recording.map_or(&[][..], |r| &r.steps[..]);
                    while steps.get(next.step as usize).is_some_and(|s| historical(s) == Historical::PassedOver) {
                        next.step += 1;
                    }
                    next.status = match (next.step as usize) < steps.len() {
                        true => PlayStatus::Waiting(next.step),
                        false => PlayStatus::Finished,
                    };
                }
            }
            Advance::Held => {
                next.pending = None;
                next.last = Some(receipt.to_json());
                next.status = PlayStatus::Waiting(next.step);
            }
            Advance::Stops(outcome) => {
                next.pending = None;
                next.last = Some(receipt.to_json());
                next.status = PlayStatus::Stopped(Stop::Outcome(outcome));
            }
        }
        Some(next)
    }

    /// Writes a reconciled state: the cursor, the cleared pending entry, a new map entry, the status and the last
    /// receipt together, in one write (EV4.20). The child leaves the live set.
    fn write_reconciled(&mut self, name: &str, current: &PlaybackState, next: PlaybackState) -> Fate {
        let fate = self.save_state(name, next);
        if let (Some(pending), false) = (&current.pending, matches!(fate, Fate::RolledBack(_))) {
            self.rec.children.remove(&(Caller::Shell.scope(), pending.command.clone()));
        }
        fate
    }

    /// Recovers every playback state: its pending child is looked up and reconciled, and a state still `running` with
    /// no live job stops with `KN5:resource_limit.engine_lost` (draft), in one write per state.
    fn recover_playbacks(&mut self) {
        let names: Vec<String> = self.rec.playbacks.keys().cloned().collect();
        for name in names {
            let live = self.rec.plays.values().any(|play| play.name == name);
            let current = self.rec.playbacks.get(&name).cloned().expect("a listed state");
            let mut next = self.reconciled(&name).unwrap_or_else(|| current.clone());
            if next.status == PlayStatus::Running && !live && next.pending.is_none() {
                next.status = PlayStatus::Stopped(Stop::Code(code::ENGINE_LOST.to_owned()));
            }
            if next != current {
                let _ = self.write_reconciled(&name, &current, next);
            }
        }
    }

    /// Writes playback `name`'s state to the recorder and keeps it. A rolled-back write keeps the state before it; an
    /// unknown write keeps the new state, which every later write replaces whole.
    fn save_state(&mut self, name: &str, state: PlaybackState) -> Fate {
        let Some(recorder) = self.rec.recorder.as_mut() else {
            return Fate::RolledBack(Error::new(code::ROW_NOT_BUILT, "no recorder"));
        };
        let fate = recorder.save_playback(name, &state);
        if !matches!(fate, Fate::RolledBack(_)) {
            self.rec.playbacks.insert(name.to_owned(), state);
        }
        self.generation += 1;
        fate
    }

    fn workspace_completion(&self, result: Option<ResultRef>) -> Status {
        Status::Completed(Completion { kind: CompletionKind::Workspace, revision: self.revision.clone(), result })
    }
}

fn text_value(text: &str) -> Value {
    Value::String(text.to_owned())
}
