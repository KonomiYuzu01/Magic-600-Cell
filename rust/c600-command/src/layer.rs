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
//! (KN5.20). They follow M6's move tree: every commit adds a node (keyed by its action order) under the head, every
//! undoable action (a commit, a `protection.orbit.set`, a `protection.policy.set` or a record write) pushes its order
//! on the undo stack and drops the redo stack, and an undo or redo inverts or repeats the change on top of its stack. A
//! head change derives the inverse of the node's recipe, or the recipe again, from the current state through the kernel
//! under the protection set and policy in force (LI.26's normal permission path) and adopts the result only when it
//! reaches the recorded node; a protection change that releases an orbit needs `confirm`. The journal keeps recipes,
//! digests, protected orbits, policies and record values, not states.
//!
//! ADR-D01.14's layout history (owner, 22 November 2026, item 4; draft) sits beside that move history:
//! [`LAYOUT_UNDO_ROW`] and [`LAYOUT_REDO_ROW`] take the same hint and token and step only the completions of workspace
//! rows whose undo class is `layout` (`UndoClass::Layout`), which `session.undo` and `session.redo` never reach. Each
//! history keeps its own undo and redo stacks over the shared change map, and a step of one cuts the other's actions
//! that wrote one of its records, so no record is held by both. A layout step journals as a workspace transaction and
//! its undo or redo as an undo or redo whose row names the history, so a replay rebuilds both. While the explore state
//! ([`EXPLORE_ROW`]; ADR-D01.39) is on, every commit row but the two layout rows is refused with
//! `KN5:capability.commits_paused` before its token is spent; the state is transient and a restart clears it.
//!
//! The protection policy ([`CommandLayer::policy`]) is Net from a session's start; `protection.policy.set`
//! ([`POLICY_ROW`]) switches it. Net judges the end of the complete operation; Strict judges every executed literal
//! prefix, so a word with an identity net map can pass Net and fail Strict (KN2.7, LB2.3). The policy in force decides
//! how a preview is built, how step 8 judges it again, and how an undo or redo derives its path.
//!
//! Only an action that moves the puzzle advances the state revision, ends the pending preview and cancels the jobs that
//! read the revision it replaces, except file jobs (KN5.18). A protection or policy change advances the workspace and
//! protection revisions, and a record write or a workspace completion the workspace revision; they keep the preview
//! slot and the jobs, and every durable action takes the next action order (LI.9). The inverse of a protection or
//! policy change or of a record write is a workspace inverse (`UndoClass::Workspace`): it answers `committed` with
//! equal puzzle hashes, the head and the state revision unchanged, and advances the workspace revision and the versions
//! it touches (KN5.20, LI.26).
//!
//! KN5's stage 3, the workspace store ([`crate::workspace`]): `workspace.set` ([`RECORD_ROW`]) writes one record, and
//! a draft `{"record": name}` previews the letters the record holds, with the record's content digest and version in
//! its draft digest (KN0.8), after the host's provenance reader admits the record's block (KN5.26) with the part
//! blocks the session keeps (CT-BND-27; [`crate::workspace`]). LB7.2's work
//! sheets ([`crate::worksheet`]) are records too: `worksheet.save` writes one from a draft record, and `worksheet.load`
//! places its piece roles again and writes it again, so a preview of the sheet goes stale (CT-LB2-03). KN5's stage 5
//! ([`crate::frame`]): [`CommandLayer::present`] reads the adapter's frame reports, only a frame that shows the pending
//! preview through the built-in chain presents it, and a word commit's token binds that frame's stamp (ONE.41).
//!
//! `twist.apply` ([`TWIST_ROW`]) is a live twist (ONE.46, ONE.78): a commit with no preview whose token binds the state
//! and protection revisions (LI.6). Its `cap` and `rotation` carry KN3.7's exact request binding ([`crate::twist`]),
//! which the layer resolves against the twist menu the host pinned ([`CommandLayer::pin_menu`]); on 600-cell-Full the
//! resolved item becomes its retained word, which the kernel derives under the protection policy in force at the
//! current revision. Like a word commit it ends the pending preview and cancels the jobs that read the replaced
//! revision, except file jobs (LI.32).
//!
//! `jumble.on` ([`JUMBLE_ON_ROW`]) is A5's entry (ONE.54, ONE.55, ONE.80; draft): a crossing commit from Jumble off
//! into Jumble on under the pinned menu ([`crate::jumble`]). Its witness (JI.25, JI.26) is the active exit's root when
//! the path from the head climbs through retained commits only to an exit: the retained suffix from the exit to the
//! head, replayed in the jumbling engine from the exit's own poses, which must have been made under the pinned menu
//! (`KN1:input.identity_mismatch`). With no active exit it is the solved witness, the chronological literal retained
//! path from the session's start, which must be solved, to the head, replayed from solved. Labels alone are refused
//! (`KN5:input.jumble_on_labels_only`). The crossing is a node of the move tree that holds no retained recipe; the
//! revision's mode becomes [`JUMBLE_ON_MODE`] and its digest the pose digest. With Jumble on, the retained workflow's
//! rows (`operation.preview`, `operation.execute`, `twist.apply`) and `jumble.on` itself are refused with
//! `KN5:input.retained_tool_needs_jumble_off` (LG2.8); `jumble.twist.menu` and the protection, policy and record rows
//! stay. An undo of the entry returns to the exact Jumble off state before it, and a redo makes the same entry again,
//! which must reach the same poses and record (ONE.81, JI.27, JI.29, FP.31).
//!
//! `jumble.twist.menu` ([`JUMBLE_TWIST_ROW`]) is JI.35's menu twist (FP.31, JI.27; draft), live like `twist.apply`: its
//! `letter` carries KN3.7's binding without its cap, resolved against the pinned menu. With Jumble off a retained item
//! commits its retained word as `twist.apply` does, and a jumble item is refused
//! (`KN5:input.menu_word_needs_jumble_on`). With Jumble on every item is a pose move, journaled as `jumble_move`
//! ([`TransactionKind::JumbleMove`]): a node of the move tree that holds the item's exact twist, derived in the
//! jumbling engine from the head's value under the policy in force, with the engine's pieces that carry the
//! protected orbits as pose-protected pieces (finding B5, the owner's reading of 18 November 2026): a pose move that
//! would move one is the engine's `KN2:protection.piece_would_move`. The engine nests each derivation's
//! evidence in the next, so after 64 chained moves the layer replays the jumbling journal flat from solved instead. An
//! undo applies the exact inverse twist when the menu holds it and replays the journal otherwise, a redo applies the
//! twist again, and each must reach the move's recorded poses.
//!
//! `jumble.off` ([`JUMBLE_OFF_ROW`]) is A5's exit (ONE.54, JI.27; draft): a crossing commit from Jumble on back to
//! Jumble off, allowed exactly at A5's label checkpoint ([`jumble::exit`]): every piece on the lattice, every centre in
//! its own chamber, ν = 0, and the head's poses witnessed from solved by the jumbling journal. A refusal names the fact
//! only (`KN5:input.jumble_off_pieces_off_lattice`, `KN5:input.jumble_off_nu_nonzero`,
//! `KN5:input.jumble_off_not_shown_reachable`). The exit is a node of the move tree that holds no retained recipe and
//! has the exit's label digest: the poses' labels become the retained state, the revision's mode becomes [`MODE`],
//! and the journal keeps the exit's record, not the poses. The retained rows work from the exit's state again, and the
//! jumbling journal runs on through the exit, which moves no piece. An undo of the exit returns to the exact poses
//! before it with the retained state the entry left, and a redo checks the checkpoint again. While only retained
//! commits follow it on the path to the head, the exit stays active, and the next `jumble.on` enters from its root
//! (JI.26). The exit checks no protected orbit; the protected set and the policy stay.
//!
//! [`CommandLayer::open`] reopens a session from a [`Reopen`] store: it replays every durable transaction through the
//! kernel, restores the revision, the protected set, the move tree and the durable receipts, and draws a fresh random
//! epoch (KN5.23, L10). A session that crossed into Jumble on reopens with [`CommandLayer::open_with_menu`], which pins
//! the menu before the replay.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use c600_digest::sha256_hex;
use c600_jumble::codes::MENU_WORD_NEEDS_JUMBLE_ON;
use c600_jumble::levels::{self, Base, Derived};
use c600_jumble::observe::cuts;
use c600_jumble::q5::{identity4, transpose};
use c600_jumble::rotation;
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::{
    CellComparison, Derivation, Effect, IdentityBlock, Kernel, LayerCounts, LayerMembers, PiecePosition, ProtectedSet,
    Protection, ProtectionMode, Recipe, Snapshot, Step, SupportRow, Verdict,
};

use crate::availability::Availability;
use crate::budget::Budget;
use crate::filter::{self, FilterPreview, FilterPreviewDeps, Predicate, ResolvedFilter, FRESH_FILTER_PREVIEW};
use crate::forms;
use crate::frame::{FrameChain, FrameStamp, PresentedFrame, ShownPreview};
use crate::grip::{slot_map_digest, SlotMap, GRIP_PREFIX, GRIP_REBIND_ROW, SLOT_MAP_PROVENANCE, SLOT_MAP_RECORD};
use crate::journal::{self, EntryKind, JournalEntry, JournalPage, JOURNAL_PAGE_LIMIT};
use crate::jumble::{self, JUMBLE_OFF_ROW, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW};
use crate::macros::{macro_record, menu_codes, Alphabet, Macro, MACRO_PREFIX, MACRO_PROVENANCE, MACRO_SAVE_ROW};
use crate::palette::FIND_ROW;
use crate::receipt::{
    code, digest, ActionHint, CancelReason, Completion, CompletionKind, Direction, Error, PreviewDeps, Receipt,
    ResultRef, Revision, Status,
};
use crate::recording::{
    JOB_STOP_ROW, RECORDING_CONVERT_ROW, RECORDING_PLAY_ROW, RECORDING_START_ROW, RECORDING_STOP_ROW,
};
use crate::repair::{self, REPAIR_LIST_ROW};
use crate::sets::{saved_set_record, SavedSet, REPLACE, SAVED_SET_PREFIX, SET_SAVE_RECORD, SET_SAVE_ROW};
use crate::store::{
    identity_json, policy_from_name, policy_name, Fate, Genesis, MemoryStore, Profile, Reopen, Store, Transaction,
    TransactionKind,
};
use crate::table::{Binding, Kind, Lane, Metadata, Overlay, Registration, Row, Table, UndoClass};
use crate::token::{args_digest, fresh_epoch, Secret, Token, TokenBinding};
use crate::twist::{self, Twists, TWIST_ROW};
use crate::view::{ViewSourceRef, Views, VIEW_CLOSE_ROW, VIEW_OPEN_ROW};
use crate::worksheet::{
    compare_inputs, MacroVersion, Occupancy, Placement, Resolution, Worksheet, WorksheetRef, WORKSHEET_COMPARE_ROW,
    WORKSHEET_LOAD_ROW, WORKSHEET_SAVE_ROW,
};
use crate::workspace::{Provenance, RecordRead, RecordValue, RecordWrite, ResolvedDraft, WorkspaceRecord, RECORD_ROW};

mod playback;

/// The mode of a revision with Jumble off: a session starts with Jumble off.
pub const MODE: &str = "jumble_off";
/// The mode of a revision with Jumble on, after [`JUMBLE_ON_ROW`]'s crossing (KN0.2).
pub const JUMBLE_ON_MODE: &str = "jumble_on";
/// H-08's row that sets the protection policy, Net or Strict.
pub const POLICY_ROW: &str = "protection.policy.set";
/// ADR-D01.14's layout-history undo row (owner, 22 November 2026, item 4; draft): it inverts the layout history's top
/// step, as `session.undo` inverts the move history's.
pub const LAYOUT_UNDO_ROW: &str = "layout.undo";
/// ADR-D01.14's layout-history redo row (draft): it repeats the layout history's top undone step.
pub const LAYOUT_REDO_ROW: &str = "layout.redo";
/// ADR-D01.39's explore state (draft): while it is on, the commit rows of the move history are paused.
pub const EXPLORE_ROW: &str = "explore.set";
/// ADR-D01.15's partial undo (owner, 22 November 2026, item 4; draft): a commit that steps back inside the head's
/// journal entry to one of its moves, recorded as its own action naming the entry and the move, so nothing is
/// rewritten.
pub const PARTIAL_UNDO_ROW: &str = "session.undo.partial";
/// ADR-D01.15's multi-step rollback (owner, 22 November 2026, item 4; draft; the integrator's ruling of 22 November
/// 2026): one solver action that undoes or redoes the move history to the state just after a target action, as k
/// ordinary undo or redo steps under one gesture, each its own transaction with its own admission and receipt.
pub const ROLLBACK_ROW: &str = "session.rollback";
/// ADR-D01.9's settle interval until Look Lab tunes it and stores it as token data (owner, 22 November 2026, item 4;
/// draft): a preview that replaced one the solver was shown commits only once this long has passed since the first
/// frame that shows it.
pub const SETTLE_DRAFT: Duration = Duration::from_millis(250);

/// The monotonic clock the layer measures ADR-D01.9's settle interval with: the time since a fixed start. The layer
/// starts with the process's clock ([`MonotonicClock`]); a host may give it another ([`CommandLayer::set_clock`]).
pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;
}

/// The process's monotonic clock, the layer's default.
#[derive(Debug)]
pub struct MonotonicClock(Instant);

impl MonotonicClock {
    pub fn new() -> MonotonicClock {
        MonotonicClock(Instant::now())
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        MonotonicClock::new()
    }
}

impl Clock for MonotonicClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}
/// 0.4's word limit: a partial undo's inverse enters the kernel in word steps of at most this many letters, since an
/// entry's expansion may be longer than one word step allows.
const WORD_CHUNK: usize = 100_000;

/// ADR-D01.12's one-key preview-and-commit of the solver's own macro (owner, 22 November 2026, item 4; draft): one
/// press prepares the macro's preview, checks it as `operation.execute` checks a presented one, and commits it.
pub const ONE_KEY_ROW: &str = "operation.preview.commit";
/// Whether a run's preview is admitted only inside a command the solver invoked (finding B2 of the KN5 boundary attack,
/// ONE.34; the owner's reading of 18 November 2026). The check is on, so a host must start its runs through a tool row
/// the shell invokes, as LP3's host does on both its hosts.
const SOLVER_COMMAND_CHECK: bool = true;
/// A star certificate failure has no registered code yet (draft).
pub const CERTIFICATE: &str = "KN2:consistency.star_certificate";

/// The number of pose moves derived in a chain before the layer replays the jumbling journal flat from solved (draft).
/// The jumbling engine nests each derivation's reachability evidence in the next, and a chain of some hundreds of
/// derivations overflows a thread's stack when its value is cloned or dropped.
const POSE_CHAIN: usize = 64;

/// Who invokes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Caller {
    /// The shell, for a solver's press or click.
    Shell,
    /// The language runtime, for one run. It never holds a token.
    Runtime { run: String },
}

impl Caller {
    /// The caller's command-ID space (ruling 4): a record is keyed by it and the command ID, so the runtime's
    /// IDs never reach the shell's records. Each run has a space of its own, `runtime/<run>`, so a command ID one run
    /// used is not used up for another run (finding G1 of the KN5 boundary attack; the owner's reading of 18 November
    /// 2026).
    pub fn scope(&self) -> String {
        match self {
            Caller::Shell => "shell".to_owned(),
            Caller::Runtime { run } => format!("runtime/{run}"),
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            Caller::Shell => Value::String("shell".into()),
            Caller::Runtime { run } => json::object([("runtime", Value::String(run.clone()))]),
        }
    }

    pub fn from_json(value: &Value) -> Option<Caller> {
        match value {
            Value::String(name) if name == "shell" => Some(Caller::Shell),
            Value::Object(fields) if fields.len() == 1 => {
                Some(Caller::Runtime { run: fields.get("runtime")?.as_str()?.to_owned() })
            }
            _ => None,
        }
    }
}

/// LI.5's envelope. `args` holds the absolute arguments, without the token. The envelope names no caller: the host
/// stamps the caller from the channel the envelope came on ([`CommandLayer::invoke`]; PHS.52 ruling 4, owner decision
/// of 16 November 2026).
#[derive(Clone, Debug)]
pub struct Invocation {
    pub command: String,
    pub row: String,
    pub args: Value,
    pub token: Option<Token>,
    /// The originating engine epoch. The request digest covers it, and a transport retry keeps the whole envelope, so
    /// a retry finds its receipt after a restart.
    pub epoch: u64,
    /// The budget the invocation declares for a program row's job ([`crate::budget`]; KN7.7; draft): explicit, beside
    /// the epoch and the token, never in the arguments. A row an entry serves refuses an invocation without one
    /// (`KN7:input.budget_not_declared`) or over the ceiling (`KN7:resource_limit.budget_ceiling`); another row runs
    /// no program and reads none. The request digest covers a declared budget.
    pub budget: Option<Budget>,
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
    /// For a draft that names a workspace record, what its draft digest covers: the draft, the letters the record held
    /// and the records read, each with its content digest and version at admission (KN0.8). `None` for an inline
    /// recipe, whose draft digest is the digest of the draft.
    pub resolved: Option<ResolvedDraft>,
    /// `Digest<literal word>`.
    pub literal: String,
    pub state: PreviewState,
    pub base: Snapshot,
    /// `None` when a protected orbit would move: the preview is still built, and execute refuses it (KN5.9).
    pub end: Option<Snapshot>,
    pub recipe: Recipe,
    pub effect: Effect,
    pub protection: Protection,
    /// The preview the solver was shown that this one replaced (ADR-D01.9; draft): the preview view marks the change,
    /// and the commit waits out the settle interval once a frame shows this one. A preview that replaces an unseen
    /// replacement keeps its mark, since the solver has seen neither. `None` when it replaced nothing the solver saw.
    pub replaced: Option<u64>,
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
    /// The preview's typed result: what the shell shows, and what its result digest covers. A Strict preview also names
    /// its mode and its first refused prefix (`null` when clear), whose net map its conflicts are; a Net preview's
    /// summary has neither, so its digest is what it was before the Strict mode (draft).
    pub fn summary(&self) -> Value {
        let verdict = match self.protection.verdict {
            Verdict::Clear => "clear",
            Verdict::Moves => "moves",
        };
        let mut summary = json::object([
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
        ]);
        if let (Value::Object(fields), ProtectionMode::Strict) = (&mut summary, self.protection.mode) {
            fields.insert("mode".to_owned(), Value::String(policy_name(ProtectionMode::Strict).to_owned()));
            let prefix = self.protection.first_refused_prefix.map_or(Value::Null, |n| Value::Int(n as i64));
            fields.insert("first_refused_prefix".to_owned(), prefix);
        }
        // ADR-D01.9's mark: only a replacement names the preview it replaced, so every other summary is as before.
        if let (Value::Object(fields), Some(replaced)) = (&mut summary, self.replaced) {
            fields.insert("replaced".to_owned(), Value::Int(replaced as i64));
        }
        summary
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
    /// The provenance blocks of the job's arguments (BND.22; LP2 draft), as the caller's host gave them with
    /// [`CommandLayer::invoke_with_inputs`], in their order; empty after [`CommandLayer::invoke`]. The layer checked
    /// each block with the host's reader and holds it as given. The host runs the entry with them as its run's inputs.
    pub inputs: Vec<Vec<u8>>,
    /// The budget the invocation declared, admitted under the ceiling ([`CommandLayer::budget_ceiling`]): the host runs
    /// the entry under it (KN7.7; draft).
    pub budget: Budget,
    /// The module digest a recorded step names, for a job a playback's step started (EV4.10, EV4.14; LP4-9 draft): the
    /// hex SHA-256 of the closure of the module's version that was active when the step was recorded. The host binds
    /// the job to that version and no other (`Modules::bind_pinned`), so a module of the same name never stands in for
    /// it. `None` for every other job, which binds the active version.
    pub module: Option<String>,
}

impl ProgramJob {
    /// The name of the job's run, `job-<job>`: the host runs the job's entry as this run, on the channel
    /// [`Caller::Runtime`] with this name. While the host holds the job, between [`CommandLayer::take_program_job`] and
    /// its report, the run is inside the command that invoked the job, and when the shell invoked it the run may prepare
    /// a preview (ONE.34; finding B2 of the KN5 boundary attack, the owner's reading of 18 November 2026).
    pub fn run(&self) -> String {
        format!("job-{}", self.job)
    }

    /// The job whose run `run` is, if `run` is a job's run name.
    fn job_of(run: &str) -> Option<u64> {
        let job = run.strip_prefix("job-")?.parse().ok()?;
        (format!("job-{job}") == run).then_some(job)
    }
}

/// A program job's output as the runtime's sinks gave it (LP3's C6, its second common need; draft): the value its
/// entry returned, the views it published, in order, and the workspace records it wrote, by name. The runtime's
/// boundary keeps a record's last write, so a name appears once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobOutput {
    pub value: Value,
    pub views: Vec<Value>,
    pub records: BTreeMap<String, RecordValue>,
}

impl JobOutput {
    /// An output that publishes no view and writes no record: what [`CommandLayer::finish_program_job`] reports.
    pub fn of(value: Value) -> JobOutput {
        JobOutput { value, views: Vec::new(), records: BTreeMap::new() }
    }

    /// The digest a completion's result reference carries (draft): the value's own digest when the job published no
    /// view and wrote no record, so [`CommandLayer::finish_program_job`]'s receipts keep their digest; otherwise the
    /// SHA-256 of the canonical `{"records", "value", "views"}`, each record in [`RecordWrite::to_json`]'s form.
    pub fn digest(&self) -> String {
        if self.views.is_empty() && self.records.is_empty() {
            return digest(&self.value);
        }
        let records = self
            .records
            .iter()
            .map(|(name, value)| RecordWrite { name: name.clone(), value: Some(value.clone()) }.to_json())
            .collect();
        digest(&json::object([
            ("records", Value::Array(records)),
            ("value", self.value.clone()),
            ("views", Value::Array(self.views.clone())),
        ]))
    }
}

enum JobKind {
    Preview {
        recipe: Recipe,
        draft: String,
        resolved: Option<ResolvedDraft>,
        caller: Caller,
    },
    Program(ProgramJob),
    /// `worksheet.compare`'s job: the saved sheet and the draft record it is compared with.
    Compare {
        sheet: WorksheetRef,
        draft: String,
    },
    /// `jumble.repair.list`'s job: the cap the invocation named.
    Repair {
        cap: i64,
    },
}

struct Job {
    id: u64,
    command: String,
    /// The invoking caller's command-ID space.
    scope: String,
    lane: Lane,
    kind: JobKind,
}

impl Job {
    /// Whether the job builds a preview: the exclusive lane also holds `worksheet.compare`'s job, which is no preview
    /// to show, cancel or discard.
    fn previews(&self) -> bool {
        matches!(self.kind, JobKind::Preview { .. })
    }
}

/// A node of the move tree: the start (node 0), a commit, a crossing, a pose move or an exit, keyed by its action
/// order. The journal keeps the recipe or the twist and the digest, not the state.
#[derive(Clone, Debug)]
struct Node {
    parent: u64,
    /// A commit's retained recipe; `None` for the start, a crossing, a pose move and an exit.
    recipe: Option<Recipe>,
    /// A pose move's exact twist (draft); `None` for every other node.
    twist: Option<Twist>,
    /// An exit's mark: the SHA-256 of its exit record's canonical JSON, the value an `exit_root` entry names
    /// ([`jumble::exit_root_record`]); `None` for every other node. A crossing is the node with none of the three,
    /// which is not the start. The journal query reads the node's kind from these alone ([`CommandLayer::journal`]),
    /// since an exit's change leaves `changes` with the redo stack.
    exit: Option<String>,
    /// A partial undo's place (ADR-D01.15; draft): the action order of the journal entry it steps back inside and how
    /// many of that entry's literal moves stay applied; `None` for every other node.
    partial: Option<(u64, usize)>,
    digest: String,
    depth: u64,
}

/// An undoable action's change, kept by its action order while an undo or redo can reach it.
#[derive(Clone, Debug)]
enum Change {
    Head {
        before: u64,
        after: u64,
    },
    Protection {
        before: Vec<usize>,
        after: Vec<usize>,
    },
    /// A record write: the record's value before and after it.
    Record {
        name: String,
        before: Option<RecordValue>,
        after: Option<RecordValue>,
    },
    /// A workspace completion of a row whose undo class is `workspace`, a language-defined row or a stored one (draft;
    /// LP3's C6, LP2's reading 18): its records' values before and after it, in the order of their names.
    Records {
        before: Vec<RecordWrite>,
        after: Vec<RecordWrite>,
    },
    /// A protection-policy change: the policy before and after it.
    Policy {
        before: ProtectionMode,
        after: ProtectionMode,
    },
    /// A crossing into Jumble on: the head before it, its own node, the jumbling identity block it was made under,
    /// which a redo needs pinned again, and its crossing record, which a redo must make again (FP.31).
    Crossing {
        before: u64,
        after: u64,
        identity: IdentityBlock,
        record: Value,
    },
    /// A pose move with Jumble on (`jumble.twist.menu`; draft): the head before it and its own node, which holds the
    /// move's twist.
    Pose {
        before: u64,
        after: u64,
    },
    /// An exit to Jumble off (`jumble.off`; draft): the head before it, its own node, and what an undo restores and a
    /// redo checks ([`ExitPoint`]).
    Exit {
        before: u64,
        after: u64,
        point: Box<ExitPoint>,
    },
}

/// What an adoption does to the move tree and the stacks.
#[derive(Clone, Debug)]
enum Tree {
    /// A new undoable action, which drops the redo stack.
    Push {
        node: Option<(u64, Node)>,
        change: Change,
    },
    Undo,
    Redo,
    /// A new step of ADR-D01.14's layout history ([`History::Layout`]): a workspace completion of a stored row whose
    /// undo class is `layout`, which drops the layout history's redo stack and leaves the move history's stacks.
    LayoutPush(Change),
    LayoutUndo,
    LayoutRedo,
    /// A workspace completion that no undo reaches: a stored row's whose undo class is `none`, or one that writes no
    /// record.
    None,
}

/// ADR-D01.14's two histories (owner, 22 November 2026, item 4; draft). The move history holds the commits and the
/// puzzle-related workspace actions, and `session.undo` and `session.redo` step it; the layout history holds the
/// completions of the rows whose undo class is `layout` (design-profile and layout changes), and `layout.undo` and
/// `layout.redo` step it. Neither holds the other's steps. Both share the session's action order, so a step of either
/// takes the next order, and both survive a restart and a reopen, since the journal keeps every step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum History {
    Moves,
    Layout,
}

impl History {
    /// The history and direction an undo or redo row steps: `session.undo` and `session.redo` the move history,
    /// `layout.undo` and `layout.redo` the layout history; no other row steps one.
    fn of(row: &str) -> Option<(History, Direction)> {
        match row {
            "session.undo" => Some((History::Moves, Direction::Undo)),
            "session.redo" => Some((History::Moves, Direction::Redo)),
            LAYOUT_UNDO_ROW => Some((History::Layout, Direction::Undo)),
            LAYOUT_REDO_ROW => Some((History::Layout, Direction::Redo)),
            _ => None,
        }
    }
}

/// The outcome of the last multi-step rollback since the layer opened or restarted (ADR-D01.15; draft): its gesture
/// (the press's command ID), the action whose state it rolls to (0: the session's start), the direction, the k steps
/// it planned and the j that landed, and the status of the step that stopped it, if one did.
#[derive(Clone, Debug, PartialEq)]
pub struct Rollback {
    pub gesture: String,
    pub target: u64,
    pub direction: Direction,
    pub of: u64,
    pub landed: u64,
    pub stop: Option<Status>,
}

/// The gesture, step i and step count k of a multi-step rollback's step from its command ID, `{gesture}#{i}/{k}` with
/// 1 <= i <= k, both in plain decimal (ADR-D01.15; draft; R2-5, owner, 23 November 2026, decision 4.2). The history
/// dock groups the steps by the gesture, and k makes "j of k" durable: after a reopen the step ID alone tells a gesture
/// cut at j of k from a whole one. The older form `{gesture}#{i}` names no step.
pub fn rollback_step(command: &str) -> Option<(&str, u64, u64)> {
    let (gesture, tail) = command.rsplit_once('#')?;
    let (step, of) = tail.split_once('/')?;
    let decimal = |text: &str| text.parse::<u64>().ok().filter(|n| n.to_string() == text);
    let (step, of) = (decimal(step)?, decimal(of)?);
    (!gesture.is_empty() && 1 <= step && step <= of).then_some((gesture, step, of))
}

/// The last action's place in a multi-step rollback when it was one of its steps (ADR-D01.15; draft), so a replay
/// admits a step after the first only when it follows its gesture's step before it.
#[derive(Clone, Debug)]
struct StepAt {
    gesture: String,
    step: u64,
    of: u64,
    kind: TransactionKind,
    epoch: u64,
}

/// Who turned ADR-D01.39's explore state on (draft): the solver, by `explore.set` from the shell, or a run inside a
/// command the solver invoked. The state may only pause commits, never enable them, so no run ends a pause the solver
/// set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Explorer {
    Solver,
    Run,
}

/// The jumbling engine's value at the head while Jumble is on, with its pose digest.
#[derive(Clone, Debug)]
struct Jumbled {
    derived: Derived,
    digest: String,
    /// How many pose moves were derived in a chain since the value was last replayed from solved ([`POSE_CHAIN`]).
    depth: usize,
}

/// What an exit's undo restores and its redo checks: the head's poses before the exit, the retained state the entry
/// left, and the exit's record.
#[derive(Clone, Debug)]
struct ExitPoint {
    poses: Jumbled,
    retained: Snapshot,
    record: Value,
}

/// What an adoption does to the mode.
#[derive(Clone, Debug)]
enum Side {
    /// The mode stays.
    Stay,
    /// Jumble is on at this value: a crossing turns it on, and a pose move, or its undo or redo, moves the poses.
    On(Jumbled),
    /// Jumble turns off: an undo of an entry keeps the retained state the entry left, and an exit adopts its labels as
    /// the retained state.
    Off,
}

/// Everything step 10 applies, computed before step 9.
struct Adoption {
    transaction: Transaction,
    /// The new state of an action that moves the puzzle.
    state: Option<Snapshot>,
    /// The new protected set of a protection change.
    protected: Option<ProtectedSet>,
    head: u64,
    tree: Tree,
    /// The preview a word commit commits.
    preview: Option<u64>,
    /// What the action does to the mode: a crossing turns Jumble on and an exit turns it off, and their undos do the
    /// reverse; a pose move moves the poses.
    side: Side,
}

/// Who asked for an action, as its transaction records it.
struct Meta {
    caller: Caller,
    command: String,
    row: String,
    request: String,
    /// Whether the solver confirmed a release (LI.26).
    confirm: bool,
}

impl Meta {
    fn of(caller: &Caller, invocation: &Invocation, request: &str) -> Meta {
        Meta {
            caller: caller.clone(),
            command: invocation.command.clone(),
            row: invocation.row.clone(),
            request: request.to_owned(),
            confirm: invocation.args.get("confirm") == Some(&Value::Bool(true)),
        }
    }

    /// A stored transaction's, for its replay: a replayed release counts only the confirmation stored with it, so a
    /// release stored without one does not replay (finding B7; the owner's reading of 18 November 2026).
    fn replay(transaction: &Transaction) -> Meta {
        Meta {
            caller: transaction.caller.clone(),
            command: transaction.command.clone(),
            row: transaction.row.clone(),
            request: transaction.request.clone(),
            confirm: transaction.confirm,
        }
    }
}

/// A receipt under its caller's scope and command ID.
#[derive(Clone, Debug)]
struct Record {
    request: String,
    receipt: Receipt,
    /// A durable receipt was written with its action's effect and survives a restart and a reopen (KN5.23).
    durable: bool,
}

/// A caller's command-ID space and a command ID (ruling 4).
type Key = (String, String);

/// A recipe's steps in the kernel's JSON form, or the steps of its inverse.
fn steps_json(recipe: &Recipe, inverse: bool) -> Vec<Value> {
    let step = |step: &Step| match step {
        Step::Word(letters) => {
            let moves: Vec<Value> = match inverse {
                false => letters.iter().map(|&m| Value::Int(m as i64)).collect(),
                true => letters.iter().rev().map(|&m| Value::Int(-(m as i64))).collect(),
            };
            json::object([("kind", Value::String("word".into())), ("moves", Value::Array(moves))])
        }
        Step::Star(star) => {
            let sign = if inverse { -star.sign() } else { star.sign() };
            json::object([
                ("kind", Value::String("star".into())),
                ("orbit", Value::Int(star.orbit() as i64)),
                ("node", Value::Int(star.node() as i64)),
                ("sign", Value::Int(sign as i64)),
            ])
        }
    };
    match inverse {
        false => recipe.steps().iter().map(step).collect(),
        true => recipe.steps().iter().rev().map(step).collect(),
    }
}

/// Whether a stored transaction agrees with its replay: every field the replay recomputes, the epoch aside.
fn agrees(planned: &Transaction, stored: &Transaction) -> bool {
    let epochless = |revision: &Revision| Revision { epoch: 0, ..revision.clone() };
    planned.kind == stored.kind
        && planned.event == stored.event
        && planned.order == stored.order
        && planned.target == stored.target
        && planned.literal == stored.literal
        && planned.pre == stored.pre
        && planned.post == stored.post
        && planned.orbits == stored.orbits
        && planned.job == stored.job
        && planned.result == stored.result
        && planned.record == stored.record
        && planned.policy == stored.policy
        && planned.records == stored.records
        && planned.crossing == stored.crossing
        && planned.budget == stored.budget
        && epochless(&planned.revision) == epochless(&stored.revision)
}

fn refused(code: &'static str, detail: impl Into<String>) -> Result<(Option<u64>, Status), Error> {
    Err(Error::new(code, detail))
}

/// The argument checks an invocation makes before its row's own: the arguments are an object, a confirmable row's
/// `confirm` and `set.save`'s `replace` are booleans, and the names fit the row's arguments. Returns the arguments.
fn argument_names<'a>(row: &Row, args: &'a Value) -> Result<&'a BTreeMap<String, Value>, Error> {
    let Some(args) = args.as_object() else {
        return Err(Error::new(code::ARGUMENT_FORM, "the arguments are not an object"));
    };
    // A row that confirms a release takes an optional boolean `confirm` beside its table arguments (draft; LI.26).
    let confirmable = row.has_gate("confirm") || row.action_guard.is_some() || row.id == POLICY_ROW;
    if confirmable && args.get("confirm").is_some_and(|confirm| !matches!(confirm, Value::Bool(_))) {
        return Err(Error::new(code::ARGUMENT_FORM, "`confirm` is a boolean"));
    }
    // `set.save` takes the optional boolean `replace` (`Bool?` since the freeze, CM.17; U1-057, [`crate::sets`]).
    let replaceable = row.id == SET_SAVE_ROW;
    if replaceable && args.get(REPLACE).is_some_and(|replace| !matches!(replace, Value::Bool(_))) {
        return Err(Error::new(code::ARGUMENT_FORM, "`replace` is a boolean"));
    }
    let names = args
        .keys()
        .map(String::as_str)
        .filter(|name| !(confirmable && *name == "confirm") && !(replaceable && *name == REPLACE));
    if !forms::names_fit(row, names) {
        return Err(Error::new(code::ARGUMENT_FORM, format!("`{}` takes {:?}", row.id, row.arguments)));
    }
    Ok(args)
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

/// The names of the records an action wrote: one for a record write, each of a workspace completion's, none otherwise.
fn written(change: &Change) -> Box<dyn Iterator<Item = &str> + '_> {
    match change {
        Change::Record { name, .. } => Box::new(std::iter::once(name.as_str())),
        Change::Records { after, .. } => Box::new(after.iter().map(|write| write.name.as_str())),
        _ => Box::new(std::iter::empty()),
    }
}

/// The actions of an undo or redo `stack` that a write no step reaches cuts off (draft): walked from the top, in the
/// order the steps would take them, an action that wrote a record of `cut` leaves the stack, since its step would write
/// over that write. The records it wrote then join the cut for the actions beneath it. On the undo stack its writes
/// stand beyond every step, so an undo beneath it would write over them; on the redo stack its writes are undone and
/// stay so, so a redo beneath it would start from values its own undo does not expect. An action above one that leaves
/// keeps its place: its step finds what it wrote.
fn cut_stack(changes: &BTreeMap<u64, Change>, stack: &[u64], cut: &BTreeSet<String>) -> BTreeSet<u64> {
    let mut cut = cut.clone();
    let mut dropped = BTreeSet::new();
    for &order in stack.iter().rev() {
        let Some(change) = changes.get(&order) else { continue };
        if written(change).any(|name| cut.contains(name)) {
            cut.extend(written(change).map(str::to_owned));
            dropped.insert(order);
        }
    }
    dropped
}

/// The command layer for one session.
pub struct CommandLayer<S: Store = MemoryStore> {
    kernel: Arc<Kernel>,
    table: Table,
    secret: Secret,
    store: S,
    /// The session's profile (BND.15, BND.17; draft): the host sets it when it creates the session, the journal keeps
    /// it, and no program or command changes it.
    profile: Profile,
    protected: ProtectedSet,
    /// The protection policy in force: Net from a session's start, then what the last policy action left.
    policy: ProtectionMode,
    revision: Revision,
    current: Snapshot,
    /// The digest of the identity block's canonical JSON, which a token binds (L9).
    identity: String,
    /// The current durable action-order number (LI.9).
    order: u64,
    /// The move tree: node 0 is the start, every other node a commit under its action order.
    nodes: BTreeMap<u64, Node>,
    head: u64,
    changes: BTreeMap<u64, Change>,
    undo: Vec<u64>,
    redo: Vec<u64>,
    /// ADR-D01.14's layout history: the undo and redo stacks of the completions whose row's undo class is `layout`
    /// ([`History::Layout`]). Their changes sit in `changes` beside the move history's, under their own action orders.
    layout_undo: Vec<u64>,
    layout_redo: Vec<u64>,
    /// ADR-D01.39's explore state (draft): while it is on, every commit row but `layout.undo` and `layout.redo` is
    /// refused before its token is spent ([`code::COMMITS_PAUSED`]). It only pauses commits and never makes one. It is
    /// transient: a restart and a reopen clear it, since the runs that provided it end with them. It holds who turned it
    /// on ([`Explorer`]).
    explore: Option<Explorer>,
    /// Every epoch of the session, so no open or restart draws one again (L10).
    epochs: BTreeSet<u64>,
    jobs_issued: u64,
    jobs: VecDeque<Job>,
    running: BTreeMap<u64, ProgramJob>,
    /// View runners a newer request of their row fenced (LI.23): settled as superseded, they hold their row's runner
    /// slot until the host reports them, and their result is dropped.
    fenced: BTreeMap<u64, ProgramJob>,
    program_capacity: usize,
    /// The host's budget ceiling, if it set one ([`CommandLayer::set_budget_ceiling`]).
    host_ceiling: Option<Budget>,
    slot: Option<Preview>,
    /// The newest frame the adapter reported (RM.26).
    frame: Option<PresentedFrame>,
    /// The pending preview's presentation run: its ID, the stamp of the frame that started showing it (ONE.41) and the
    /// clock's time when the layer read that frame (ADR-D01.9's settle interval).
    run: Option<(u64, FrameStamp, Duration)>,
    /// The clock and the settle interval of ADR-D01.9 ([`CommandLayer::set_clock`], [`CommandLayer::set_settle`]).
    clock: Box<dyn Clock>,
    settle_interval: Duration,
    /// The preview the solver last saw, while no action of theirs has ended it (ADR-D01.9's mark; draft): a frame the
    /// layer read presented it, also when that presentation has lapsed since. A preview built now carries it as the
    /// mark of the change, also into a slot that a run's cancel emptied. The solver's cancel, a state move and a restart
    /// clear it.
    last_seen: Option<u64>,
    /// The workspace records by name (KN5's stage 3).
    records: BTreeMap<String, WorkspaceRecord>,
    /// The values the solver kept with `workspace.set`, as record name and value digest (ADR-D01.12; draft): what the
    /// one-key commit takes as the solver's own macro. A replay rebuilds them.
    kept: BTreeSet<(String, String)>,
    /// The last multi-step rollback's outcome (ADR-D01.15; draft); a restart clears it.
    rollback: Option<Rollback>,
    /// The gesture, step and kind of the last action when it was a rollback's step (ADR-D01.15; draft), so a replay
    /// admits a step after the first only when it follows the gesture's step before it.
    last_step: Option<StepAt>,
    /// The command IDs of the rollback steps the layer journaled or tried, by caller scope, for LI.30's lookup of a
    /// gesture's steps (ADR-D01.15): an ordinary command whose ID looks like a step is no step.
    step_ids: BTreeSet<Key>,
    /// The step count k of each rollback gesture the layer journaled or tried, by caller scope and gesture ID
    /// (ADR-D01.15; R2-5): LI.30's lookup builds the step IDs `{gesture}#{i}/{k}` from it, and one gesture ID names
    /// one gesture whatever its k.
    gestures: BTreeMap<Key, u64>,
    /// The part blocks the session keeps, each block's text by the SHA-256 hex of its bytes (BND.24, BND.85,
    /// CT-BND-27; draft). Record writes and workspace completions journal them; a replay keeps them again.
    blocks: BTreeMap<String, String>,
    /// The completed jobs of the rows that prepare a predicate, by job, while their writes stand: what a `FilterRef`
    /// names ([`crate::filter`]; draft). A replay rebuilds them.
    predicates: BTreeMap<u64, Predicate>,
    /// The filter previews of completed `filter.preview` jobs, by job, while they are fresh (KN0.9; draft). They are
    /// transient: a reopen keeps none.
    filters: BTreeMap<u64, FilterPreview>,
    /// The newest completed `worksheet.compare` job and its comparison (draft). It is transient: a reopen keeps none.
    comparison: Option<(u64, Value)>,
    /// The newest completed `jumble.repair.list` job and its list (draft). It is transient: a reopen keeps none.
    repairs: Option<(u64, Value)>,
    /// The host's provenance reader (PS1.16, KN5.26). Without one, record writes are refused, and so are record
    /// previews and comparisons in a solving session.
    provenance: Option<Box<dyn Provenance>>,
    /// The host's view model (EV3.2, RPI.83). Without one, `view.open` and `view.close` are refused.
    views: Option<Box<dyn Views>>,
    /// The twist menu the host pinned (KN3.8). Without one, `twist.apply` and `jumble.on` are refused.
    twists: Option<Twists>,
    /// The jumbling engine's value at the head while Jumble is on (draft; [`crate::jumble`]). `None` with Jumble off.
    jumbled: Option<Jumbled>,
    history: BTreeMap<u64, PreviewState>,
    used: BTreeSet<u64>,
    receipts: BTreeMap<Key, Record>,
    /// Records whose transient receipts a restart lost, with their request digests.
    lost: BTreeMap<Key, String>,
    /// Refusals of envelopes that do not authenticate, kept for the epoch under the envelope's full digest and never
    /// attached to the command ID (ruling 4).
    retained: BTreeMap<Key, Receipt>,
    commit_held: bool,
    recovery: Option<Error>,
    limbo: Option<Adoption>,
    generation: u64,
    /// Capture and playback (LP4-9; [`crate::recording`]): the host's recorder, the capture in progress, the stored
    /// recordings and playback states, and the live playback jobs and children.
    rec: playback::Rec,
}

impl<S: Store> CommandLayer<S> {
    fn empty(
        kernel: Arc<Kernel>,
        table: Table,
        secret: Secret,
        store: S,
        revision: Revision,
        initial: Snapshot,
        protected: ProtectedSet,
    ) -> CommandLayer<S> {
        let identity = digest(&identity_json(&kernel.identity()));
        let start = Node {
            parent: 0,
            recipe: None,
            twist: None,
            exit: None,
            partial: None,
            digest: initial.digest().to_owned(),
            depth: 0,
        };
        CommandLayer {
            kernel,
            table: table.with_drafts(),
            secret,
            store,
            profile: Profile::Solving,
            protected,
            policy: ProtectionMode::Net,
            epochs: BTreeSet::from([revision.epoch]),
            revision,
            current: initial,
            identity,
            order: 0,
            nodes: BTreeMap::from([(0, start)]),
            head: 0,
            changes: BTreeMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            layout_undo: Vec::new(),
            layout_redo: Vec::new(),
            explore: None,
            jobs_issued: 0,
            jobs: VecDeque::new(),
            running: BTreeMap::new(),
            fenced: BTreeMap::new(),
            program_capacity: 1,
            host_ceiling: None,
            slot: None,
            frame: None,
            run: None,
            clock: Box::new(MonotonicClock::new()),
            settle_interval: SETTLE_DRAFT,
            last_seen: None,
            records: BTreeMap::new(),
            kept: BTreeSet::new(),
            rollback: None,
            last_step: None,
            step_ids: BTreeSet::new(),
            gestures: BTreeMap::new(),
            predicates: BTreeMap::new(),
            filters: BTreeMap::new(),
            comparison: None,
            repairs: None,
            blocks: BTreeMap::new(),
            provenance: None,
            views: None,
            twists: None,
            jumbled: None,
            history: BTreeMap::new(),
            used: BTreeSet::new(),
            receipts: BTreeMap::new(),
            lost: BTreeMap::new(),
            retained: BTreeMap::new(),
            commit_held: false,
            recovery: None,
            limbo: None,
            generation: 0,
            rec: playback::Rec::default(),
        }
    }

    /// A new session `session` at `initial` with the protected set `protected`, in epoch 1, in the solving profile
    /// ([`CommandLayer::new_with_profile`]). The store records the session's start ([`Store::begin`]); a store that
    /// does not answer durably leaves the layer in read-only recovery.
    pub fn new(
        kernel: Arc<Kernel>,
        table: Table,
        secret: Secret,
        store: S,
        session: &str,
        initial: Snapshot,
        protected: ProtectedSet,
    ) -> CommandLayer<S> {
        Self::new_with_profile(kernel, table, secret, store, session, initial, protected, Profile::Solving)
    }

    /// [`CommandLayer::new`] in the profile `profile` (BND.17; draft). The host sets it from the place the session
    /// starts: an ordinary session, the practice copy included until LQ2 answers BND.15, is `solving`. The session's
    /// start records it ([`Genesis::profile`]), a reopen reads it back ([`CommandLayer::open`]), and no row changes it.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_profile(
        kernel: Arc<Kernel>,
        table: Table,
        secret: Secret,
        store: S,
        session: &str,
        initial: Snapshot,
        protected: ProtectedSet,
        profile: Profile,
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
        let genesis = Genesis {
            session: session.to_owned(),
            identity: kernel.identity(),
            epoch: 1,
            labels: initial.labels().to_vec(),
            digest: initial.digest().to_owned(),
            orbits: protected.orbits().to_vec(),
            profile,
        };
        let mut layer = Self::empty(kernel, table, secret, store, revision, initial, protected);
        layer.profile = profile;
        let fate = layer.store.begin(&genesis);
        layer.settle_fate(fate, "the session's start");
        layer
    }

    /// Reopens the session a [`Reopen`] store holds (KN5.23). Every durable transaction is replayed through the kernel
    /// from the session's start and must reach its recorded digests and revision; the revision, the protected set, the
    /// move tree, the stacks and the durable receipts are restored, and the layer starts a fresh random epoch that no
    /// earlier open of the session drew (L10). A transaction that does not replay, or a record the store could not
    /// read, leaves the layer in read-only recovery after the last transaction that replayed. A command ID the store
    /// does not hold answers `unknown_outcome` ([`CommandLayer::kernel_outcome`], LI.30). A crossing into Jumble on
    /// replays only under its twist menu ([`CommandLayer::open_with_menu`]); here it leaves the layer in read-only
    /// recovery before it.
    ///
    /// The replay checks again what the live path admitted for two kinds of action (finding B7; the owner's reading of
    /// 18 November 2026): a commit replays only from the shell's caller and derives under the protected set and the
    /// policy in force at its place in the journal, and a protection change, or the undo or redo of one, that releases
    /// an orbit replays only with the confirmation stored with it. A journal action that fails either check leaves the
    /// layer in read-only recovery before it with `KN5:consistency.unadmitted_journal_action` (draft), so the session
    /// does not open as an intact writable one. A journal carries no authentication: an edit that rewrites a
    /// program-made action as the solver's own (its caller, its command ID) and that the live path would have admitted
    /// from the shell is not detected.
    pub fn open(kernel: Arc<Kernel>, table: Table, secret: Secret, store: S) -> Result<CommandLayer<S>, Error>
    where
        S: Reopen,
    {
        Self::open_inner(kernel, table, secret, store, None)
    }

    /// [`CommandLayer::open`] with the twist menu `menu`, read in the exact model `context`, pinned before the replay
    /// ([`CommandLayer::pin_menu`]; draft), so a crossing into Jumble on replays its witness under it. A crossing made
    /// under another menu does not replay, and the layer stays in read-only recovery before it. A `context` of another
    /// model is refused as `pin_menu` refuses it (`KN1:input.identity_mismatch`), and the layer does not open.
    pub fn open_with_menu(
        kernel: Arc<Kernel>,
        table: Table,
        secret: Secret,
        store: S,
        context: Arc<Context>,
        menu: Menu,
    ) -> Result<CommandLayer<S>, Error>
    where
        S: Reopen,
    {
        Self::open_inner(kernel, table, secret, store, Some((context, menu)))
    }

    fn open_inner(
        kernel: Arc<Kernel>,
        table: Table,
        secret: Secret,
        mut store: S,
        pin: Option<(Arc<Context>, Menu)>,
    ) -> Result<CommandLayer<S>, Error>
    where
        S: Reopen,
    {
        let history = store.history()?;
        let genesis = &history.genesis;
        kernel.check_identity(&genesis.identity)?;
        let initial = kernel.state_from_labels(genesis.labels.clone()).filter(|s| s.digest() == genesis.digest);
        let Some(initial) = initial else {
            return Err(Error::new(code::PERSISTENCE_FORMAT, "the session's start state does not match its digest"));
        };
        let orbits: Vec<i64> = genesis.orbits.iter().map(|&orbit| orbit as i64).collect();
        let protected = kernel.protected_set(&orbits)?;
        if protected.orbits() != genesis.orbits.as_slice() {
            return Err(Error::new(code::PERSISTENCE_FORMAT, "the session's protected orbits are not in order"));
        }
        let revision = Revision {
            session: genesis.session.clone(),
            epoch: genesis.epoch,
            identity: kernel.identity().model_id,
            mode: MODE,
            head: "e0".to_owned(),
            state: 0,
            workspace: 0,
            protection: 0,
            digest: initial.digest().to_owned(),
        };
        let profile = genesis.profile;
        let mut layer = Self::empty(kernel, table, secret, store, revision, initial, protected);
        layer.profile = profile;
        if let Some((context, menu)) = pin {
            layer.pin_menu(context, menu)?;
        }
        layer.epochs.extend(history.epochs.iter().copied());
        for transaction in &history.transactions {
            if let Err(error) = layer.replay(transaction) {
                layer.recovery = Some(error);
                break;
            }
        }
        if layer.recovery.is_none() {
            layer.recovery = history.damaged.clone();
        }
        layer.next_epoch();
        Ok(layer)
    }

    /// Replays one durable transaction at a reopen and adopts it as stored.
    fn replay(&mut self, stored: &Transaction) -> Result<(), Error> {
        let broken = |detail: &str| Error::new(code::READ_ONLY_RECOVERY, format!("{}: {detail}", stored.event));
        let unadmitted =
            |detail: &str| Error::new(code::UNADMITTED_JOURNAL_ACTION, format!("{}: {detail}", stored.event));
        // A release replays only with its stored confirmation (B7).
        let release = |error: Error| match error.code {
            code::CONFIRMATION_REQUIRED => unadmitted("a release of a protected orbit without its confirmation"),
            _ => error,
        };
        if stored.order != self.order + 1 {
            return Err(broken("the action order is broken"));
        }
        // The rollback row journals only its undo and redo steps (ADR-D01.15).
        if stored.row == ROLLBACK_ROW && !matches!(stored.kind, TransactionKind::Undo | TransactionKind::Redo) {
            return Err(unadmitted("a rollback row whose action is not an undo or redo step"));
        }
        let meta = Meta::replay(stored);
        let planned = match stored.kind {
            // A twist's rotation record is adopted as stored: the replay derives the recorded recipe and runs neither
            // the producer nor the chooser again (KN3.9).
            TransactionKind::Commit => {
                if self.jumbled.is_some() {
                    return Err(broken("a retained commit while Jumble is on"));
                }
                // Only the solver's own action commits (KN5.11, KN5.15), and the live path derives a commit under the
                // protected set and the policy in force (B7).
                if !matches!(stored.caller, Caller::Shell) {
                    return Err(unadmitted("a commit whose caller is not the shell"));
                }
                let recipe = self.kernel.recipe(&stored.recipe)?;
                // A partial undo steps back inside the head's entry, as the live path builds it (ADR-D01.15).
                if stored.row == PARTIAL_UNDO_ROW && !self.steps_back_inside(&recipe) {
                    return Err(unadmitted("a partial undo that does not step back inside the head's entry"));
                }
                // A one-key commit commits the word of a value the solver kept, as the live press does (ADR-D01.12).
                if stored.row == ONE_KEY_ROW && !self.kept_word(&recipe) {
                    return Err(unadmitted("a one-key commit of a word that no record the solver kept holds"));
                }
                match self.derive(&self.current, &recipe, &self.protected).map_err(kernel_error)? {
                    Derivation::Derived(d) => self.commit_adoption(meta, recipe, d.state, d.word, None),
                    Derivation::Refused(r) => {
                        let error = Error::from(r.refusal);
                        return Err(match error.code {
                            code::ORBIT_WOULD_MOVE => unadmitted("a commit that moves a protected orbit"),
                            _ => error,
                        });
                    }
                }
            }
            TransactionKind::Undo | TransactionKind::Redo => {
                let direction = match stored.kind {
                    TransactionKind::Undo => Direction::Undo,
                    _ => Direction::Redo,
                };
                // Only the solver's own action steps a history: the four undo and redo rows are commit rows, which no
                // run reaches (KN5.15, B7).
                if !matches!(stored.caller, Caller::Shell) {
                    return Err(unadmitted("an undo or redo whose caller is not the shell"));
                }
                // The row names the history and the direction: `layout.undo` and `layout.redo` step the layout history
                // (ADR-D01.14), `session.undo` and `session.redo` the move history. A multi-step rollback's step is a
                // move-history step under command `{gesture}#{i}/{k}`, and a step after the first follows its
                // gesture's step before it with the same k in the same direction, as the live gesture writes them
                // (ADR-D01.15; R2-5).
                let history = match (History::of(&stored.row), stored.row == ROLLBACK_ROW) {
                    (Some((history, named)), false) if named == direction => history,
                    (None, true) => {
                        let Some((gesture, step, of)) = rollback_step(&stored.command) else {
                            return Err(unadmitted("a rollback step whose command names no gesture, step and count"));
                        };
                        let follows = self.last_step.as_ref().is_some_and(|last| {
                            last.gesture == gesture
                                && step.checked_sub(1) == Some(last.step)
                                && last.of == of
                                && last.kind == stored.kind
                                && last.epoch == stored.revision.epoch
                        });
                        if step > 1 && !follows {
                            return Err(unadmitted(
                                "a rollback step that does not follow its gesture's step before it in its epoch",
                            ));
                        }
                        let gesture_key = (stored.caller.scope(), gesture.to_owned());
                        if step == 1
                            && (self.receipts.contains_key(&gesture_key) || self.gestures.contains_key(&gesture_key))
                        {
                            return Err(unadmitted("a rollback whose gesture ID an earlier action used"));
                        }
                        if self.receipts.contains_key(&(stored.caller.scope(), stored.command.clone())) {
                            return Err(unadmitted("a rollback step whose command ID an earlier action used"));
                        }
                        History::Moves
                    }
                    _ => {
                        return Err(broken("an undo or redo action whose row is not its direction's undo or redo row"))
                    }
                };
                if stored.target != self.stack(history, direction).last().copied() {
                    return Err(broken("the action does not invert or repeat the top of its stack"));
                }
                self.plan_step(meta, history, direction, &ProtectedSet::none()).map_err(release)?
            }
            TransactionKind::Protection => {
                let orbits: Vec<i64> = stored.orbits.iter().map(|&orbit| orbit as i64).collect();
                let set = self.kernel.protected_set(&orbits)?;
                self.plan_orbits(meta, set).map_err(release)?
            }
            TransactionKind::Workspace => {
                let (Some(job), Some(result)) = (stored.job, stored.result.clone()) else {
                    return Err(broken("a workspace completion names no job and result"));
                };
                // No run reaches a stored workspace row's job (ONE.52, findings B3 and B4: [`Self::runtime_may`]), a
                // layout-history step included (ADR-D01.14), so a completion the journal gives to a run was never
                // admitted (B7).
                let stored_workspace = self.table.row(&stored.row).is_some_and(|row| row.kind == Kind::Workspace);
                if stored_workspace && !matches!(stored.caller, Caller::Shell) {
                    return Err(unadmitted("a workspace row's completion whose caller is not the shell"));
                }
                // No job's completion writes the applied grip map or the macro base, and none comes under their rows,
                // which run no job and write only their own record (EV4.8, EV4.9).
                if stored.row == GRIP_REBIND_ROW || stored.row == MACRO_SAVE_ROW {
                    return Err(unadmitted("a job's completion under the grip map's or the macro base's row"));
                }
                if stored.records.iter().any(|write| reserved_by_row(&write.name)) {
                    return Err(unadmitted("a job's completion that writes the grip map or the macro base"));
                }
                self.jobs_issued = self.jobs_issued.max(job);
                // The records' blocks were read when the job finished (PS1.16), so the replay needs no reader.
                self.workspace_adoption(meta, job, result, stored.records.clone(), stored.budget)
            }
            // The record's block was read when the record was written (PS1.16), so the replay needs no reader. The
            // applied grip map and the macro base are written only by their own rows from the shell, as the live path
            // writes them (EV4.8, EV4.9; B7).
            TransactionKind::Record => match stored.record.clone() {
                Some(RecordWrite { name, value: Some(value) }) => {
                    reserved_write(stored, &name, &value, &self.records).map_err(|detail| unadmitted(&detail))?;
                    self.plan_record(meta, name, value)
                }
                _ => return Err(broken("a record write names no record and value")),
            },
            TransactionKind::Policy => match stored.policy {
                Some(policy) => self.plan_policy(meta, policy),
                None => return Err(broken("a policy change names no policy")),
            },
            // A crossing replays its witness under the menu pinned before the replay, and its record must be the one
            // the replay makes ([`agrees`]).
            TransactionKind::Crossing => {
                if stored.row != JUMBLE_ON_ROW || self.jumbled.is_some() {
                    return Err(broken("a crossing is not an entry from Jumble off"));
                }
                if self.twists.is_none() {
                    return Err(broken("a crossing replays only under its twist menu (open_with_menu)"));
                }
                self.plan_entry(meta)?
            }
            // A pose move replays its recorded exact twist, which must be an item of the pinned menu, from the head's
            // poses; neither the producer nor the chooser runs again (KN3.9).
            TransactionKind::JumbleMove => {
                if stored.row != JUMBLE_TWIST_ROW || self.jumbled.is_none() {
                    return Err(broken("a jumble move is not a pose move of `jumble.twist.menu` with Jumble on"));
                }
                let (Some(twists), Some(record)) = (&self.twists, &stored.rotation) else {
                    return Err(broken("a jumble move replays only under its twist menu, with its rotation"));
                };
                let twist = rotation::replay(&twists.context, record)
                    .map_err(|error| broken(&format!("the rotation record does not replay: {error}")))?;
                if !twists.menu.contains(&twists.context, &twist) {
                    return Err(broken("the recorded rotation is not an item of the pinned menu"));
                }
                self.plan_pose(meta, twist, record.clone())?
            }
            // An exit checks A5's label checkpoint again at the replayed poses, and its record must be the one the
            // check makes ([`agrees`]).
            TransactionKind::Exit => {
                if stored.row != JUMBLE_OFF_ROW || self.jumbled.is_none() {
                    return Err(broken("an exit is not `jumble.off` with Jumble on"));
                }
                self.plan_exit(meta)?
            }
        };
        if !agrees(&planned.transaction, stored) {
            return Err(broken("the replay does not reach the recorded action"));
        }
        // An action journals only the part blocks the session did not keep yet ([`Self::parts_to_keep`]).
        if stored.blocks.iter().any(|block| self.blocks.contains_key(&sha256_hex(block.as_bytes()))) {
            return Err(broken("the action keeps a part block the session keeps already"));
        }
        self.adopt(Adoption { transaction: stored.clone(), ..planned });
        Ok(())
    }

    /// Draws a fresh random epoch that the session never had and records it (L10).
    fn next_epoch(&mut self) {
        let epoch = fresh_epoch(&self.epochs);
        self.epochs.insert(epoch);
        self.revision.epoch = epoch;
        let fate = self.store.begin_epoch(epoch);
        self.settle_fate(fate, "the new epoch");
    }

    /// Puts the layer into read-only recovery unless a write of `what` was durable.
    fn settle_fate(&mut self, fate: Fate, what: &str) {
        match fate {
            Fate::Durable => {}
            Fate::RolledBack(error) => self.recovery = Some(error),
            Fate::Unknown => {
                self.recovery =
                    Some(Error::new(code::COMMIT_FATE, format!("the store cannot say whether {what} landed")))
            }
        }
    }

    /// The digest of the identity block's canonical JSON, which a token binds (L9).
    pub fn identity_binding(&self) -> String {
        self.identity.clone()
    }

    pub fn kernel(&self) -> &Arc<Kernel> {
        &self.kernel
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    /// Registers a language-defined row (KN5.3) with no label and no grant: [`CommandLayer::register_overlay_with`]
    /// with the default [`Registration`], so a `file` row is refused.
    pub fn register_overlay(
        &mut self,
        id: &str,
        kind: Kind,
        arguments: &[(&str, &str)],
        overlay: Overlay,
    ) -> Result<(), Error> {
        self.register_overlay_with(id, kind, arguments, overlay, Registration::default())
    }

    /// Registers a language-defined row with its label and the capabilities the solver granted its module (KN5.3,
    /// EV4.1). [`Table::register_overlay_with`] says what is refused. The generation advances, since the overlay rows
    /// are an input of availability (LI.21); a refusal changes nothing.
    pub fn register_overlay_with(
        &mut self,
        id: &str,
        kind: Kind,
        arguments: &[(&str, &str)],
        overlay: Overlay,
        registration: Registration,
    ) -> Result<(), Error> {
        self.table.register_overlay_with(id, kind, arguments, overlay, registration)?;
        self.generation += 1;
        Ok(())
    }

    /// Serves stored row `id` with a built-in tool's entry (draft; LP3's C6). An invocation of the row admits a program
    /// job for the entry, as for an overlay row, and the row keeps H-08's shape. [`Table::serve_row`] says which rows
    /// can be served and what is refused. The generation advances as for a registration.
    pub fn serve_row(&mut self, id: &str, overlay: Overlay) -> Result<(), Error> {
        self.table.serve_row(id, overlay)?;
        self.generation += 1;
        Ok(())
    }

    /// Stores language-defined row `id`'s metadata (draft; KN5.5, KN5.7, LI.20), which its availability reads with no
    /// language code running ([`CommandLayer::availability`]). [`Table::declare`] says what is refused. The generation
    /// advances, since the static verdicts are an input of availability (LI.21); a refusal changes nothing.
    pub fn declare_overlay(&mut self, id: &str, metadata: Metadata) -> Result<(), Error> {
        self.table.declare(id, metadata)?;
        self.generation += 1;
        Ok(())
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    /// The store, to inject its faults. A test hook: a write through it bypasses every gate, so it exists only in the
    /// builds of [`CommandLayer::hold_commit_lane`] (draft; L15).
    #[cfg(any(debug_assertions, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    /// The retained state at the head. With Jumble on, the retained state the crossing entered from;
    /// [`CommandLayer::jumbled`] is the head's value.
    pub fn snapshot(&self) -> &Snapshot {
        &self.current
    }

    /// The jumbling engine's value at the head with Jumble on, under the pinned menu; `None` with Jumble off (draft).
    pub fn jumbled(&self) -> Option<&Derived> {
        self.jumbled.as_ref().map(|jumbled| &jumbled.derived)
    }

    /// KN2.4's `positions` row at the head (draft; CT-LB2-04): where each named piece's stickers are, as
    /// [`Kernel::positions`] reads them, with its refusal of a piece outside 0 to 177,119. It is a row of the retained
    /// workflow, so Jumble on refuses it (LG2.8): the retained state is then the one the crossing entered from. A read
    /// changes nothing.
    pub fn positions(&self, pieces: &[i64]) -> Result<Vec<PiecePosition>, Error> {
        self.retained_mode()?;
        Ok(self.kernel.positions(&self.current, pieces)?)
    }

    /// KN2.4's `labels` row at the head, read for whole positions (draft; a work sheet's position roles, U1-057): the
    /// piece whose stickers fill each named position, or `None` while no one piece fills it. A position's occupant in
    /// a state is where the position's own piece is in the inverse state, so this is [`Kernel::positions`] on the
    /// inverse of the head's labels, with its refusal of a position outside 0 to 177,119. Jumble on refuses it, as it
    /// refuses [`CommandLayer::positions`], even when no position is named. A read changes nothing.
    fn occupants(&self, positions: &[i64]) -> Result<Vec<Option<i64>>, Error> {
        self.retained_mode()?;
        if positions.is_empty() {
            return Ok(Vec::new());
        }
        let labels = self.current.labels();
        let mut inverse = vec![0i32; labels.len()];
        for (slot, &label) in labels.iter().enumerate() {
            inverse[label as usize] = slot as i32;
        }
        let inverse = self.kernel.state_from_labels(inverse).expect("the inverse of a permutation of the slots");
        let found = self.kernel.positions(&inverse, positions)?;
        Ok(found.into_iter().map(|row| row.position.map(|piece| piece as i64)).collect())
    }

    /// The structure row that `structure.compare` reads (draft; LP3's C6, step 1 item 3): two cells compared, as
    /// [`Kernel::compare_cells`] counts their face steps, with its refusal of a cell outside 0 to 599. Jumble on
    /// refuses it, as it refuses [`CommandLayer::positions`] (LG2.8). A read changes nothing.
    pub fn compare_cells(&self, first: i64, second: i64) -> Result<CellComparison, Error> {
        self.retained_mode()?;
        Ok(self.kernel.compare_cells(first, second)?)
    }

    /// The counts that `filter.preview` reads for a layer predicate (draft; LP3's C6, step 1 item 3): the positions
    /// and slots of each layer around `cell`, as [`Kernel::cell_layers`] counts them. Jumble on refuses it, as it
    /// refuses [`CommandLayer::positions`] (LG2.8), since the positions are the retained model's. A read changes
    /// nothing.
    pub fn cell_layers(&self, cell: i64) -> Result<Vec<LayerCounts>, Error> {
        self.retained_mode()?;
        Ok(self.kernel.cell_layers(cell)?)
    }

    /// One layer of [`CommandLayer::cell_layers`], as [`Kernel::layer_counts`] counts it, with its refusal of a layer
    /// past the cell's last.
    pub fn layer_counts(&self, cell: i64, layer: i64) -> Result<LayerCounts, Error> {
        self.retained_mode()?;
        Ok(self.kernel.layer_counts(cell, layer)?)
    }

    /// The members that `structure.layer` reads for a layer predicate (draft; LP3's C6): the positions of one layer
    /// around `cell` and their home slots, as [`Kernel::layer_members`] gives them, with the refusals of
    /// [`CommandLayer::layer_counts`]. A read changes nothing.
    pub fn layer_members(&self, cell: i64, layer: i64) -> Result<LayerMembers, Error> {
        self.retained_mode()?;
        Ok(self.kernel.layer_members(cell, layer)?)
    }

    /// KN4.3's journal query (draft; LP3's C8): at most `limit` events of the move tree, newest first, from the event
    /// `from` (the head for `None`) along its parents toward the start ([`journal`]). `cursor` continues an earlier
    /// page of the same walk at its `next`. The move tree is durable, so the walk answers for the events of every
    /// epoch, with Jumble on or off; its cost is linear in the page and flat in the history, and a read changes
    /// nothing. Refusals, in this order: a `limit` of 0 or above [`JOURNAL_PAGE_LIMIT`], `KN5:input.argument_form`; a
    /// `from` the move tree does not hold, `KN2:stale.revision_gone`; a cursor of another walk, session or launch,
    /// `KN5:input.argument_form`.
    pub fn journal(&self, from: Option<u64>, limit: u64, cursor: Option<&str>) -> Result<JournalPage, Error> {
        if limit == 0 || limit > JOURNAL_PAGE_LIMIT {
            let detail = format!("a journal page holds 1 to {JOURNAL_PAGE_LIMIT} events");
            return Err(Error::new(code::ARGUMENT_FORM, detail));
        }
        let first = from.unwrap_or(self.head);
        if !self.nodes.contains_key(&first) {
            let detail = format!("the move tree holds no event {}", journal::event_name(first));
            return Err(Error::new(code::REVISION_GONE, detail));
        }
        let session = &self.revision.session;
        let mut node = match cursor {
            None => first,
            Some(cursor) => journal::cursor_next(cursor, &self.secret, session, from)
                .filter(|next| self.nodes.contains_key(next))
                .ok_or_else(|| Error::new(code::ARGUMENT_FORM, "the cursor continues another walk"))?,
        };
        let mut entries = Vec::new();
        loop {
            entries.push(self.journal_entry(node));
            if node == 0 {
                return Ok(JournalPage { entries, next: None });
            }
            node = self.nodes[&node].parent;
            if entries.len() as u64 == limit {
                return Ok(JournalPage { entries, next: Some(journal::cursor(&self.secret, session, from, node)) });
            }
        }
    }

    /// The journal entry of the move tree's node `order`, which the tree holds. Its kind follows from the node alone:
    /// the start is node 0, a commit keeps its recipe, a pose move its twist and an exit its mark, and a crossing is
    /// the node with none of them.
    fn journal_entry(&self, order: u64) -> JournalEntry {
        let node = &self.nodes[&order];
        let kind = if order == 0 {
            EntryKind::Start
        } else if node.recipe.is_some() {
            EntryKind::Commit
        } else if node.twist.is_some() {
            EntryKind::Pose
        } else if node.exit.is_some() {
            EntryKind::Exit
        } else {
            EntryKind::Crossing
        };
        let letters = node.recipe.as_ref().map_or(0, |recipe| recipe.primitive_count() as u64);
        let solved = order == 0 && node.digest == self.kernel.solved().digest();
        JournalEntry { order, kind, letters, solved, exit: node.exit.clone() }
    }

    /// The digest of the state at the head: the labels' digest with Jumble off, the pose digest with Jumble on. The
    /// revision's digest and an action's `pre` and `post` are this digest.
    fn state_digest(&self) -> &str {
        match &self.jumbled {
            Some(jumbled) => &jumbled.digest,
            None => self.current.digest(),
        }
    }

    /// LG2.8: the retained workflow's rows need Jumble off (`KN5:input.retained_tool_needs_jumble_off`).
    fn retained_mode(&self) -> Result<(), Error> {
        match self.jumbled {
            Some(_) => Err(Error::new(code::RETAINED_TOOL_NEEDS_JUMBLE_OFF, "the retained workflow needs Jumble off")),
            None => Ok(()),
        }
    }

    /// Whether `row`, a query of the jumbling engine's poses, can read them: the host pinned a twist menu
    /// (`KN5:input.row_not_built` otherwise) and Jumble is on (`KN5:input.menu_word_needs_jumble_on` otherwise; draft:
    /// the registry has no code of its own for a jumbling query with Jumble off).
    fn jumble_mode(&self, row: &str) -> Result<(), Error> {
        match (&self.twists, &self.jumbled) {
            (None, _) => Err(Error::new(code::ROW_NOT_BUILT, format!("`{row}` needs the host's pinned twist menu"))),
            (Some(_), None) => {
                Err(Error::new(MENU_WORD_NEEDS_JUMBLE_ON, format!("`{row}` reads the poses: Jumble is off")))
            }
            (Some(_), Some(_)) => Ok(()),
        }
    }

    pub fn protected(&self) -> &ProtectedSet {
        &self.protected
    }

    /// The protection policy in force, Net or Strict: Net from a session's start, then what the last
    /// `protection.policy.set`, or the undo or redo of one, left.
    pub fn policy(&self) -> ProtectionMode {
        self.policy
    }

    /// The session's profile (BND.15, BND.17; draft), set when the host created the session and read back by a reopen.
    /// A host sets a run's profile from it (BND.17).
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Derives `recipe` from `base` under `protected` in the policy in force: Net judges the end of the complete
    /// operation, Strict every executed literal prefix (KN2.7, LB2.3).
    fn derive(
        &self,
        base: &Snapshot,
        recipe: &Recipe,
        protected: &ProtectedSet,
    ) -> Result<Derivation, c600_kernel::Error> {
        match self.policy {
            ProtectionMode::Net => self.kernel.derive(base, recipe, protected),
            ProtectionMode::Strict => self.kernel.derive_strict(base, recipe, protected),
        }
    }

    /// The pending preview.
    pub fn preview(&self) -> Option<&Preview> {
        self.slot.as_ref()
    }

    /// The preview the solver is shown: the pending preview while a frame presents it.
    fn shown(&self) -> Option<&Preview> {
        self.slot.as_ref().filter(|p| p.state == PreviewState::Presented)
    }

    /// Whether run `run` may prepare a preview now. Only inside a command the solver invoked (finding B2 of the KN5
    /// boundary attack, ONE.34; the owner's reading of 18 November 2026, `docs/wiki/decisions/owner-decisions-2026-11-18.md`):
    /// the one check is here, not in each host, and it is on ([`SOLVER_COMMAND_CHECK`]). Such a run may replace the
    /// preview the solver is shown (ADR-D01.9, which answers KN5.12; owner, 22 November 2026, and supersedes the
    /// refusal of findings B1 and L17's reading): the replacement takes a new ID when it builds, carries the mark of the
    /// change ([`Preview::replaced`]), and its commit waits out the settle interval ([`Self::settled`]). A recording
    /// never replaces a preview: a run inside a `recording.play` job is refused while a preview is pending
    /// (`LG3:capability.not_granted`).
    fn run_may_preview(&self, run: &str) -> Result<(), Error> {
        if SOLVER_COMMAND_CHECK && !self.inside_solver_command(run) {
            let detail = format!("run `{run}` is not inside a command the solver invoked (ONE.34)");
            return Err(Error::new(code::OUTSIDE_SOLVER_COMMAND, detail));
        }
        if let Some(pending) = self.slot.as_ref().filter(|_| self.in_recording(run)) {
            let detail = format!("a recording never replaces a preview (ADR-D01.9): preview {} is pending", pending.id);
            return Err(Error::new(code::NOT_GRANTED, detail));
        }
        Ok(())
    }

    /// Whether run `run` runs inside a `recording.play` job: its job, or a job of a run that invoked it, is an unattended
    /// step a playback started ([`CommandLayer::played_child`]). A chain that does not get older is not followed.
    fn in_recording(&self, run: &str) -> bool {
        let mut run = run.to_owned();
        let mut below = u64::MAX;
        loop {
            let Some(job) = ProgramJob::job_of(&run).filter(|&job| job < below) else {
                return false;
            };
            let Some(program) = self.running.get(&job) else {
                return false;
            };
            if self.played_child(program) {
                return true;
            }
            match &program.caller {
                Caller::Shell => return false,
                Caller::Runtime { run: parent } => (run, below) = (parent.clone(), job),
            }
        }
    }

    /// The mark a preview built now carries (ADR-D01.9): the preview the solver last saw, while no action of theirs has
    /// ended it (`last_seen`). A replacement of an unseen replacement, or a preview built into a slot that a
    /// run's cancel emptied, keeps the mark of the preview the solver did see. The host reports each frame before the
    /// input that follows it (RM.26), so a preview on screen is one the layer has read.
    fn replacing(&self) -> Option<u64> {
        self.last_seen
    }

    /// Whether a press of the commit key on `p` waits out the settle interval: `p` replaced a preview the solver saw
    /// (ADR-D01.9), or a run prepared it (draft reading: a key-down never commits on its first frame a word a program
    /// prepared, also when it replaced a preview the solver had not yet seen or filled an empty slot).
    fn settles(p: &Preview) -> bool {
        p.replaced.is_some() || matches!(p.caller, Caller::Runtime { .. })
    }

    /// ADR-D01.9's settle interval: a preview that waits ([`Self::settles`]) commits only once the interval has passed
    /// since the layer read the first frame of its current presentation, so a key-down never commits a preview the
    /// solver has not seen. A press inside the interval is refused before its token is spent
    /// (`KN5:busy.preview_settling`; draft), and the next press after it commits. A presentation that ends and starts
    /// again waits again. [`CommandLayer::settle_remaining`] says how long the pending preview still waits.
    fn settled(&self, p: &Preview) -> Result<(), Error> {
        let since = match &self.run {
            Some((run, _, since)) if *run == p.id => *since,
            _ => return Err(Error::new(code::NO_FRESH_PREVIEW, format!("preview {} is not presented", p.id))),
        };
        let shown = self.clock.now().saturating_sub(since);
        if shown < self.settle_interval {
            let (shown, interval) = (shown.as_millis(), self.settle_interval.as_millis());
            let why = if p.replaced.is_some() { "replaced a shown preview" } else { "was prepared by a run" };
            let detail = format!("preview {} {why} and has been shown {shown} of {interval} ms", p.id);
            return Err(Error::new(code::PREVIEW_SETTLING, format!("{detail} (ADR-D01.9)")));
        }
        Ok(())
    }

    /// Whether run `run` is inside a command the solver invoked: the run is a program job's run
    /// ([`ProgramJob::run`]), the host has taken that job and not yet reported it, and the job was invoked by the shell,
    /// or by a run that is itself inside a command the solver invoked. A job is invoked by a run only while that run's
    /// job runs, so its job is older; a chain that does not get older is not followed.
    fn inside_solver_command(&self, run: &str) -> bool {
        let mut run = run.to_owned();
        let mut below = u64::MAX;
        loop {
            let Some(job) = ProgramJob::job_of(&run).filter(|&job| job < below) else {
                return false;
            };
            let Some(program) = self.running.get(&job) else {
                return false;
            };
            match &program.caller {
                Caller::Shell => return true,
                Caller::Runtime { run: parent } => (run, below) = (parent.clone(), job),
            }
        }
    }

    /// The state of any preview this epoch built.
    pub fn preview_state(&self, id: u64) -> Option<PreviewState> {
        match &self.slot {
            Some(p) if p.id == id => Some(p.state),
            _ if self.jobs.iter().any(|job| job.id == id && job.previews()) => Some(PreviewState::Building),
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

    /// Sets the host's budget ceiling (BM3; draft). A program row's budget must fit the lower of this and the session
    /// profile's ceiling ([`Profile::budget_ceiling`]) in each dimension, so a host ceiling above the profile's
    /// raises nothing.
    pub fn set_budget_ceiling(&mut self, ceiling: Budget) {
        self.host_ceiling = Some(ceiling);
    }

    /// The ceiling a program row's budget must fit: the session profile's, lowered by the host's in each dimension.
    pub fn budget_ceiling(&self) -> Budget {
        let profile = self.profile.budget_ceiling();
        self.host_ceiling.map_or(profile, |host| profile.min(host))
    }

    /// The current preview dependencies for a draft digest (KN0.8).
    pub fn deps(&self, draft: &str) -> PreviewDeps {
        PreviewDeps {
            epoch: self.revision.epoch,
            identity: self.revision.identity.clone(),
            mode: self.revision.mode,
            state: self.revision.state,
            protection: self.revision.protection,
            draft: draft.to_owned(),
        }
    }

    /// The filter preview that completed `filter.preview` job `job` made, while the layer keeps it (KN0.9; draft): a
    /// `FilterPreviewRef` names it by `job` and the epoch it was made in ([`FilterPreview::reference`]). The layer
    /// keeps a filter preview while it is fresh and drops stale ones as it keeps a new one, so a missing preview and a
    /// stale one are refused alike.
    pub fn filter_preview(&self, job: u64) -> Option<&FilterPreview> {
        self.filters.get(&job)
    }

    /// The current filter-preview dependencies for a resolved filter's digest (KN0.9).
    pub fn filter_deps(&self, filter: &str) -> FilterPreviewDeps {
        FilterPreviewDeps {
            epoch: self.revision.epoch,
            identity: self.revision.identity.clone(),
            mode: self.revision.mode,
            state: self.revision.state,
            filter: filter.to_owned(),
        }
    }

    /// KN0.9's `filter_fresh(f)`: `f`'s dependencies equal the same fields read now, its filter's records read again.
    pub fn filter_fresh(&self, f: &FilterPreview) -> bool {
        let reads = f.resolved.reads.iter().map(|read| RecordRead::of(&read.record, self.records.get(&read.record)));
        let now = ResolvedFilter { source: f.resolved.source.clone(), reads: reads.collect() };
        self.filter_deps(&now.digest()) == f.deps
    }

    /// Resolves a `FilterRef` ([`crate::filter`]): it must name a completed job of a predicate row with that result
    /// digest whose writes still stand, or it is refused with `KN2:input.named_object_invalid`. The read set holds each
    /// record the job wrote, with its content digest and version.
    fn resolve_filter(&self, source: &Value) -> Result<ResolvedFilter, Error> {
        let named = filter::filter_ref(source)
            .and_then(|(job, digest)| self.predicates.get(&job).filter(|predicate| predicate.digest == digest));
        let Some(predicate) = named else {
            let detail = "the predicate names no completed job that prepared one with that result";
            return Err(Error::new(code::NAMED_OBJECT_INVALID, detail));
        };
        let reads: Vec<RecordRead> =
            predicate.writes.iter().map(|(name, _)| RecordRead::of(name, self.records.get(name))).collect();
        if predicate.writes.iter().zip(&reads).any(|((_, version), read)| read.version != *version) {
            let detail = "a record the predicate's job wrote has been written again since";
            return Err(Error::new(code::NAMED_OBJECT_INVALID, detail));
        }
        Ok(ResolvedFilter { source: source.clone(), reads })
    }

    /// Every `FilterRef` argument of `args` must resolve ([`CommandLayer::resolve_filter`]) when a job is admitted.
    fn check_filter_refs(&self, row: &Row, args: &BTreeMap<String, Value>) -> Result<(), Error> {
        for (name, value) in args {
            if row.arguments.get(name).is_some_and(|ty| ty.strip_suffix('?').unwrap_or(ty) == "FilterRef") {
                self.resolve_filter(value)?;
            }
        }
        Ok(())
    }

    /// The gate `fresh_filter_preview` (LI.41), for a row that takes it: the filter preview its `FilterPreviewRef`
    /// argument names must be one the layer keeps, made in the reference's epoch, and fresh
    /// ([`CommandLayer::filter_fresh`]), or the row is refused with `KN5:stale.filter_preview`. Job IDs start again
    /// after a reopen, so the epoch keeps a reference from before it off a later preview with the same job ID. The gate
    /// reads neither the word-preview slot nor a presentation. The layer checks it when it admits the job and again
    /// when the job's output is published (KN0.9).
    fn fresh_filter_gate(&self, row: &Row, args: &Value) -> Result<(), Error> {
        if !row.has_gate(FRESH_FILTER_PREVIEW) {
            return Ok(());
        }
        let named = row.arguments.iter().find(|(_, ty)| ty.as_str() == "FilterPreviewRef").map(|(name, _)| name);
        let reference = named.and_then(|name| filter::filter_preview_ref(args.get(name)?));
        let kept = reference.and_then(|(job, epoch)| self.filters.get(&job).filter(|f| f.deps.epoch == epoch));
        let Some(f) = kept else {
            let detail = "the filter preview the row names is not one the layer keeps";
            return Err(Error::new(code::STALE_FILTER_PREVIEW, detail));
        };
        if !self.filter_fresh(f) {
            let detail = "the filter preview's dependencies differ from the current ones";
            return Err(Error::new(code::STALE_FILTER_PREVIEW, detail));
        }
        Ok(())
    }

    /// Keeps the filter preview a completed `filter.preview` job made: bound to the revision the job read and to its
    /// predicate, resolved now, and dropping the filter previews that are stale. A predicate that no longer resolves
    /// keeps nothing, so a `FilterPreviewRef` to the job is refused as stale.
    fn keep_filter_preview(&mut self, program: &ProgramJob, result: ResultRef) {
        let typed = |ty: &str| ty.strip_suffix('?').unwrap_or(ty) == "FilterRef";
        let Some(row) = self.table.row(&program.row) else { return };
        let predicate = row.arguments.iter().find(|(_, ty)| typed(ty)).and_then(|(name, _)| program.args.get(name));
        let Some(Ok(resolved)) = predicate.map(|source| self.resolve_filter(source)) else { return };
        let deps = FilterPreviewDeps {
            epoch: program.revision.epoch,
            identity: program.revision.identity.clone(),
            mode: program.revision.mode,
            state: program.revision.state,
            filter: resolved.digest(),
        };
        let stale: Vec<u64> = self.filters.iter().filter(|(_, f)| !self.filter_fresh(f)).map(|(&job, _)| job).collect();
        for job in stale {
            self.filters.remove(&job);
        }
        self.filters.insert(program.job, FilterPreview { job: program.job, result, deps, resolved });
    }

    /// `kernel.undo_hint`: the current order and the eligible target for `direction`, read together (LI.9).
    pub fn undo_hint(&self, direction: Direction) -> Option<ActionHint> {
        self.hint(History::Moves, direction)
    }

    /// The layout history's hint (ADR-D01.14; draft): the current order and the eligible target of `layout.undo` or
    /// `layout.redo`, read together, as [`CommandLayer::undo_hint`] reads the move history's.
    pub fn layout_hint(&self, direction: Direction) -> Option<ActionHint> {
        self.hint(History::Layout, direction)
    }

    /// The multi-step rollback's hint (ADR-D01.15; draft): the current order and `target`, read together, when
    /// `session.rollback` reaches the state just after action `target` (0: the session's start) by undoing or redoing
    /// the move history; `None` when `target` is the head's own action or on neither of its stacks.
    pub fn rollback_hint(&self, target: u64) -> Option<ActionHint> {
        self.span(target).map(|_| ActionHint { order: self.order, target })
    }

    /// The outcome of the last multi-step rollback since the layer opened or restarted (ADR-D01.15; draft).
    pub fn rollback(&self) -> Option<&Rollback> {
        self.rollback.as_ref()
    }

    /// LI.30's lookup of a gesture's steps (ADR-D01.15; draft): the shell's receipts of `{gesture}#1/{k}`,
    /// `{gesture}#2/{k}` and on, in order, up to the first step with none. After a crash a retry finds the complete
    /// prefix that landed; a step whose transient receipt a restart lost answers `unknown_outcome`. Each receipt's
    /// command carries k, so after a reopen j landed steps of k tell a cut gesture from a whole one (R2-5).
    pub fn rollback_steps(&self, gesture: &str) -> Vec<Receipt> {
        let scope = Caller::Shell.scope();
        let Some(&of) = self.gestures.get(&(scope.clone(), gesture.to_owned())) else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        for step in 1..=of {
            let key = (scope.clone(), format!("{gesture}#{step}/{of}"));
            if !self.step_ids.contains(&key) {
                break;
            }
            match (self.receipts.get(&key), self.lost.get(&key)) {
                (Some(record), _) => steps.push(record.receipt.clone()),
                (None, Some(request)) => {
                    let (command, request) = (key.1, request.clone());
                    steps.push(Receipt { command, request, job: None, status: Status::UnknownOutcome });
                }
                (None, None) => break,
            }
        }
        steps
    }

    /// The direction and number of move-history steps that reach the state just after action `target` (0: the
    /// session's start): an undo of every action above `target` on the undo stack (all of them for 0), or a redo of
    /// the redo stack's actions down to and including `target`.
    fn span(&self, target: u64) -> Option<(Direction, u64)> {
        if target == 0 {
            return (!self.undo.is_empty()).then_some((Direction::Undo, self.undo.len() as u64));
        }
        if let Some(at) = self.undo.iter().position(|&order| order == target) {
            let steps = (self.undo.len() - 1 - at) as u64;
            return (steps > 0).then_some((Direction::Undo, steps));
        }
        let at = self.redo.iter().position(|&order| order == target)?;
        Some((Direction::Redo, (self.redo.len() - at) as u64))
    }

    fn hint(&self, history: History, direction: Direction) -> Option<ActionHint> {
        self.stack(history, direction).last().map(|&target| ActionHint { order: self.order, target })
    }

    fn stack(&self, history: History, direction: Direction) -> &Vec<u64> {
        match (history, direction) {
            (History::Moves, Direction::Undo) => &self.undo,
            (History::Moves, Direction::Redo) => &self.redo,
            (History::Layout, Direction::Undo) => &self.layout_undo,
            (History::Layout, Direction::Redo) => &self.layout_redo,
        }
    }

    /// Whether ADR-D01.39's explore state is on (draft): the host signals it on every view frame.
    pub fn explore(&self) -> bool {
        self.explore.is_some()
    }

    /// The journal entry that the head is inside, with how many of its literal moves stay applied (ADR-D01.15; draft):
    /// the entry and the bound of the move that `session.undo.partial` names. `None` when the head is the start, a
    /// crossing, a pose move or an exit.
    pub fn head_entry(&self) -> Option<(u64, usize)> {
        self.inside(self.head)
    }

    /// The journal entry that node `node` leaves the head inside, with how many of its literal moves stay applied: a
    /// partial undo's own place, or a commit's whole recipe; `None` for the start, a crossing, a pose move and an exit.
    fn inside(&self, node: u64) -> Option<(u64, usize)> {
        let found = &self.nodes[&node];
        match (found.partial, &found.recipe) {
            (Some(place), _) => Some(place),
            (None, Some(recipe)) => Some((node, recipe.primitive_count())),
            (None, None) => None,
        }
    }

    /// The recipe that steps back inside `entry`, whose first `applied` literal moves are applied, to its first `keep`:
    /// the inverses of the moves from `keep` to `applied`, last first, in word steps of at most [`WORD_CHUNK`] letters.
    fn partial_recipe(&self, entry: u64, applied: usize, keep: usize) -> Result<Recipe, Error> {
        let detail = "the entry holds no retained recipe";
        let recipe = self.nodes[&entry].recipe.as_ref().ok_or_else(|| Error::new(code::UNDO_SUPERSEDED, detail))?;
        let letters = self.kernel.letters(recipe)?;
        if keep >= applied || applied > letters.len() {
            return Err(Error::new(code::UNDO_SUPERSEDED, "the move is not an applied move of the entry"));
        }
        let inverse: Vec<Value> = letters[keep..applied].iter().rev().map(|&m| Value::Int(-(m as i64))).collect();
        let word = |moves: &[Value]| {
            json::object([("kind", Value::String("word".into())), ("moves", Value::Array(moves.to_vec()))])
        };
        let steps: Vec<Value> = inverse.chunks(WORD_CHUNK).map(word).collect();
        Ok(self.kernel.recipe(&Value::Array(steps))?)
    }

    /// Whether a stored partial undo's `recipe` steps back inside the head's entry: the one word the live path builds
    /// for the moves it undoes.
    fn steps_back_inside(&self, recipe: &Recipe) -> bool {
        let Some((entry, applied)) = self.inside(self.head) else {
            return false;
        };
        let Some(keep) = applied.checked_sub(recipe.primitive_count()) else {
            return false;
        };
        self.partial_recipe(entry, applied, keep).is_ok_and(|expected| expected == *recipe)
    }

    /// Whether the explore state pauses `row`: every commit row but the layout history's undo and redo, which commit
    /// no puzzle state (ADR-D01.39).
    fn paused(&self, row: &Row) -> bool {
        self.explore.is_some()
            && row.kind == Kind::Commit
            && !matches!(row.id.as_str(), LAYOUT_UNDO_ROW | LAYOUT_REDO_ROW)
    }

    /// The shell's outcome lookup: [`CommandLayer::kernel_outcome`] for a shell command ID, or `None` when the shell's
    /// command-ID space never saw the ID in this layer's life.
    pub fn outcome(&self, command: &str, request: &str) -> Option<Receipt> {
        let scope = Caller::Shell.scope();
        let key = (scope.clone(), command.to_owned());
        let seen = self.receipts.contains_key(&key)
            || self.lost.contains_key(&key)
            || self.retained.iter().any(|((s, _), receipt)| *s == scope && receipt.command == command);
        seen.then(|| self.kernel_outcome(&Caller::Shell, command, request))
    }

    /// `kernel.outcome(command, request)` (KN5.23, LI.30): the stored answer for the caller's command ID, with no side
    /// effect and no lane. A refusal retained for this epoch answers its envelope's full digest (ruling 4); a
    /// recorded command with another request digest answers `refused(KN5:input.command_id_reused)` and leaves the
    /// record unchanged; a transient receipt a restart lost, and any command ID never recorded, answer
    /// `unknown_outcome` (CT-ONE-21, LI.30).
    pub fn kernel_outcome(&self, caller: &Caller, command: &str, request: &str) -> Receipt {
        let scope = caller.scope();
        if let Some(receipt) = self.retained.get(&(scope.clone(), request.to_owned())).filter(|r| r.command == command)
        {
            return receipt.clone();
        }
        let key = (scope, command.to_owned());
        let answer = |request: &str, status: Status| Receipt {
            command: command.to_owned(),
            request: request.to_owned(),
            job: None,
            status,
        };
        let reused = || {
            let error = Error::new(code::COMMAND_ID_REUSED, "this command ID was used for other content");
            answer(request, Status::Refused(error))
        };
        if let Some(record) = self.receipts.get(&key) {
            return if record.request == request { record.receipt.clone() } else { reused() };
        }
        if let Some(stored) = self.lost.get(&key) {
            return if stored == request { answer(stored, Status::UnknownOutcome) } else { reused() };
        }
        answer(request, Status::UnknownOutcome)
    }

    /// RM.26's report of a frame whose presentation the adapter confirmed: its stamp, the preview it shows and the chain
    /// that drew it (ONE.41, RPI.76). Reports are read in frame order. A report older than the newest one is refused and
    /// changes nothing, and the newest report repeated changes nothing. A newer report takes effect whatever `id` says:
    /// a frame that shows the pending preview through the built-in chain presents it, and the preview's presentation
    /// run starts at the first such frame on its device epoch, surface and surface generation; any other frame (one that
    /// shows no preview, another preview, or the preview drawn through a user stage) ends the presentation, and the
    /// pending preview is ready again. The answer is the pending preview `id` when the frame presents it, and
    /// `KN5:stale.no_fresh_preview` otherwise.
    pub fn present(&mut self, id: u64, frame: &PresentedFrame) -> Result<&Preview, Error> {
        if self.frame.as_ref() != Some(frame) {
            if self.frame.as_ref().is_some_and(|newest| frame.stamp.order() <= newest.stamp.order()) {
                return Err(Error::new(
                    code::NO_FRESH_PREVIEW,
                    "the frame is not newer than the newest frame reported",
                ));
            }
            self.frame = Some(frame.clone());
            self.observe(frame);
        }
        let shown = |p: &Preview| frame.presents(&ShownPreview::of(p));
        if self.slot.as_ref().is_some_and(|p| p.id == id && p.state == PreviewState::Presented && shown(p)) {
            return Ok(self.slot.as_ref().expect("the slot holds the presented preview"));
        }
        let detail = match (&frame.preview, frame.chain) {
            (_, FrameChain::UserStage) => "the frame drew a preview through a user stage (RPI.76)".to_owned(),
            (None, FrameChain::BuiltIn) => "the frame shows no preview".to_owned(),
            (Some(_), FrameChain::BuiltIn) if self.slot.as_ref().is_some_and(shown) => {
                format!("preview {id} is not the pending preview")
            }
            (Some(_), FrameChain::BuiltIn) => "the frame shows another preview than the pending one".to_owned(),
        };
        Err(Error::new(code::NO_FRESH_PREVIEW, detail))
    }

    /// RM.26's report that the adapter cannot confirm presentation (`read_presented` answers `unavailable(Error)`, for
    /// example after a device loss): the host clears the presented-preview latch and never guesses the newest preview
    /// it received. A presented pending preview is ready again, its presentation run ends and the generation advances
    /// (LI.21), so `operation.execute` answers `KN5:stale.no_fresh_preview` until a newer frame presents the preview
    /// again. The newest frame reported stays the newest, so neither a delayed older report nor the newest one repeated
    /// restores the presentation. The adapter's error is the host's to show; the layer keeps nothing of it.
    pub fn presentation_unavailable(&mut self) {
        self.run = None;
        if let Some(p) = self.slot.as_mut().filter(|p| p.state == PreviewState::Presented) {
            p.state = PreviewState::Ready;
            self.generation += 1;
        }
    }

    /// Applies a newer frame report to the pending preview and its presentation run.
    fn observe(&mut self, frame: &PresentedFrame) {
        let Some(p) = self.slot.as_mut() else {
            self.run = None;
            return;
        };
        if frame.presents(&ShownPreview::of(p)) {
            self.last_seen = Some(p.id);
            if p.state == PreviewState::Ready {
                p.state = PreviewState::Presented;
                self.generation += 1;
            }
            let continued =
                self.run.as_ref().is_some_and(|(run, first, _)| *run == p.id && first.same_surface(&frame.stamp));
            if !continued {
                self.run = Some((p.id, frame.stamp.clone(), self.clock.now()));
            }
        } else {
            if p.state == PreviewState::Presented {
                p.state = PreviewState::Ready;
                self.generation += 1;
            }
            self.run = None;
        }
    }

    /// The newest frame the adapter reported (RM.26).
    pub fn frame(&self) -> Option<&PresentedFrame> {
        self.frame.as_ref()
    }

    /// Whether the frame `stamp` belongs to the presentation run of the pending preview `id`: on the run's device epoch,
    /// surface and surface generation, no older than the frame that started the run and no newer than the newest frame
    /// reported (ONE.41).
    fn in_run(&self, id: u64, stamp: &FrameStamp) -> bool {
        let (Some((run, first, _)), Some(newest)) = (&self.run, &self.frame) else {
            return false;
        };
        *run == id
            && first.same_surface(stamp)
            && newest.stamp.same_surface(stamp)
            && first.sequence <= stamp.sequence
            && stamp.sequence <= newest.stamp.sequence
    }

    /// Gives the layer the clock it measures ADR-D01.9's settle interval with; until then it is the process's
    /// [`MonotonicClock`]. A presentation already running waits the whole interval again on the new clock.
    pub fn set_clock(&mut self, clock: Box<dyn Clock>) {
        self.clock = clock;
        if let Some((_, _, since)) = &mut self.run {
            *since = self.clock.now();
        }
        self.generation += 1;
    }

    /// Sets ADR-D01.9's settle interval, which the host reads from the tuned token data; until then it is
    /// [`SETTLE_DRAFT`]. A zero interval waits for nothing beyond the presentation.
    pub fn set_settle(&mut self, interval: Duration) {
        self.settle_interval = interval;
        self.generation += 1;
    }

    /// ADR-D01.9's settle interval in force.
    pub fn settle_interval(&self) -> Duration {
        self.settle_interval
    }

    /// How long a press of the commit key on the pending preview is still ignored (ADR-D01.9): `None` when there is no
    /// pending preview or it does not wait (it replaced nothing the solver was shown and a run did not prepare it); the
    /// whole interval while no frame shows it; zero once it has settled. Availability does not change when it reaches
    /// zero, so a host that greys the key reads this.
    pub fn settle_remaining(&self) -> Option<Duration> {
        let p = self.slot.as_ref().filter(|p| Self::settles(p))?;
        Some(match &self.run {
            Some((run, _, since)) if *run == p.id => {
                self.settle_interval.saturating_sub(self.clock.now().saturating_sub(*since))
            }
            _ => self.settle_interval,
        })
    }

    /// Gives the layer the host's provenance reader (PS1.16, KN5.26). Without one, `workspace.set` and a draft that
    /// names a record are refused.
    pub fn set_provenance(&mut self, reader: Box<dyn Provenance>) {
        self.provenance = Some(reader);
        self.generation += 1;
    }

    /// Gives the layer the host's view model (EV3.2, RPI.83; [`crate::view`]). Without one, `view.open` and
    /// `view.close` are refused with `KN5:input.row_not_built`.
    pub fn set_views(&mut self, views: Box<dyn Views>) {
        self.views = Some(views);
        self.generation += 1;
    }

    /// Pins the twist menu that `twist.apply` resolves its exact requests against and that `jumble.on` enters under
    /// (KN3.8; draft), with the exact model `context` it is read in, which must be the session's model
    /// (`KN1:input.identity_mismatch`). A pin replaces the one before; with Jumble on, only by a menu with the
    /// identity block the session entered under (`KN1:input.identity_mismatch` otherwise). Without a pin, `twist.apply`
    /// and `jumble.on` are refused with `KN5:input.row_not_built`. The pin is a host setting: the journal does not keep
    /// it, and a reopened layer needs it again ([`CommandLayer::open_with_menu`]).
    pub fn pin_menu(&mut self, context: Arc<Context>, menu: Menu) -> Result<(), Error> {
        if context.model.model_id != self.revision.identity {
            return Err(Error::new(
                c600_kernel::codes::IDENTITY_MISMATCH,
                "the twist menu's model is not the session's model",
            ));
        }
        let menu = Arc::new(menu);
        if self.jumbled.as_ref().is_some_and(|jumbled| context.identity(Some(&menu)) != *jumbled.derived.identity()) {
            return Err(Error::new(c600_kernel::codes::IDENTITY_MISMATCH, "Jumble is on under another twist menu"));
        }
        self.twists = Some(Twists { context, menu });
        self.generation += 1;
        Ok(())
    }

    /// The workspace records by name, each with its version and value (KN5's stage 3).
    pub fn records(&self) -> &BTreeMap<String, WorkspaceRecord> {
        &self.records
    }

    /// The part blocks the session keeps, each block's text by the SHA-256 hex of its bytes (BND.24, BND.85,
    /// CT-BND-27; draft): what the host's reader held for the blocks of the records written, kept with those writes.
    pub fn blocks(&self) -> &BTreeMap<String, String> {
        &self.blocks
    }

    /// The value record `name` holds, if any.
    pub fn record(&self, name: &str) -> Option<&RecordValue> {
        self.records.get(name).and_then(|record| record.value.as_ref())
    }

    /// Holds or releases the commit lane, as an unsettled commit of another invocation would. A test hook: it exists
    /// only in builds with debug assertions or the `test-hooks` feature (draft; L15).
    #[cfg(any(debug_assertions, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn hold_commit_lane(&mut self, held: bool) {
        if self.commit_held != held {
            self.commit_held = held;
            self.generation += 1;
        }
    }

    /// Replaces the head state without any revision change, as a corrupted head would (fault rehearsal for step 7). A
    /// test hook, in the same builds as [`CommandLayer::hold_commit_lane`].
    #[cfg(any(debug_assertions, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn fault_replace_head(&mut self, state: Snapshot) {
        self.current = state;
    }

    /// Ends the epoch and starts a fresh random one with a new per-launch secret (CT-ONE-21, L10). Previews, jobs,
    /// handles and the frame reports end; durable receipts stay; other receipts become `unknown_outcome`, and the
    /// refusals retained for the epoch are dropped; no token of the old epoch is valid. A transaction whose fate was unknown is adopted if the
    /// store answers that it holds it durably and dropped if the store answers that it does not; either answer ends its
    /// recovery. While the store cannot tell, the transaction stays unknown and the layer stays in read-only recovery,
    /// so no later action takes its place in the order; a later restart settles it. An action whose write a panic
    /// interrupted is settled the same way.
    pub fn restart(&mut self, secret: Secret) {
        self.secret = secret;
        self.used.clear();
        self.retained.clear();
        if let Some(p) = self.slot.take() {
            self.history.insert(p.id, PreviewState::Discarded);
        }
        self.frame = None;
        self.run = None;
        self.last_seen = None;
        self.rollback = None;
        self.jobs.clear();
        self.running.clear();
        self.fenced.clear();
        self.comparison = None;
        self.repairs = None;
        self.commit_held = false;
        self.explore = None;
        if let Some(adoption) = self.limbo.take() {
            match self.store.durable(&adoption.transaction.event) {
                Some(true) => {
                    self.recovery = None;
                    self.adopt(adoption);
                }
                Some(false) => self.recovery = None,
                None => self.limbo = Some(adoption),
            }
        }
        for (key, record) in std::mem::take(&mut self.receipts) {
            if record.durable {
                self.receipts.insert(key, record);
            } else {
                self.lost.insert(key, record.request);
            }
        }
        self.next_epoch();
        self.end_plays();
        self.generation += 1;
    }

    /// Whether `row` can be invoked now, computed from the table and the layer's state with no language code running:
    /// the row's argument-independent gates (KN5.6, KN5.25). A row an entry runs answers from its stored metadata, as
    /// LI.20 orders it (KN5.7; CT-KN5-08): see [`CommandLayer::declare_overlay`]. `Ok` does not promise acceptance:
    /// arguments and the token are checked at invocation.
    pub fn availability(&self, row: &str) -> Result<(), Error> {
        self.gates(self.known(row)?)
    }

    /// Whether `row` can be invoked now with `args` (draft; LI.18's argument-aware hint without its revision, for one
    /// binding that LI.19 keys by the arguments' digest): [`CommandLayer::availability`]'s answer, then the argument
    /// checks an invocation makes before admission. The arguments are an object, a confirmable row's `confirm` is a
    /// boolean, the names fit the row's arguments, and each argument of a type whose form the layer knows has that
    /// form and, for a `RevisionRef`, names the current revision; a `FilterRef` resolves, and a row that takes the gate
    /// `fresh_filter_preview` names a fresh filter preview. Like every hint it grants nothing: an invocation checks its
    /// actual arguments and the current inputs again.
    pub fn availability_for(&self, row: &str, args: &Value) -> Result<(), Error> {
        let found = self.known(row)?;
        self.binding(found, args)?;
        if found.id == ROLLBACK_ROW && ["gesture", "step", "of"].iter().any(|name| args.get(name).is_some()) {
            return Err(Error::new(code::ARGUMENT_FORM, "only the layer names a rollback's gesture and steps"));
        }
        Ok(())
    }

    /// KN5.6's availability table for the shell's `bindings` (draft; L13 (d) of the KN5 attack leads): a static
    /// binding answers as [`CommandLayer::availability_for`] does for its arguments, a computed one as
    /// [`CommandLayer::availability`] does, and an unknown row with `KN5:input.unknown_row`. The answers, the revision,
    /// the generation (KN5.25) and the context digest are read at one moment, as LI.19 stamps them. Bindings with the
    /// same key share one entry. No language code runs, no program starts and no lane is reserved, and the table grants
    /// nothing: an invocation checks its actual arguments and the current inputs again.
    pub fn availability_table<'a>(&self, bindings: impl IntoIterator<Item = (&'a str, &'a Binding)>) -> Availability {
        let rows = bindings
            .into_iter()
            .map(|(row, binding)| {
                let answer = match binding {
                    Binding::Static(args) => self.availability_for(row, args),
                    Binding::Computed => self.availability(row),
                };
                (Availability::key(row, binding), answer.map_err(|error| error.code))
            })
            .collect();
        // The shell's grants follow from the caller and the session's profile, so their names stand for the context. A
        // new input of the grants joins this digest.
        let profile = Value::String(self.profile.name().to_owned());
        let context = json::object([("caller", Value::String("shell".to_owned())), ("profile", profile)]);
        Availability { revision: self.revision.clone(), generation: self.generation, context: digest(&context), rows }
    }

    fn known(&self, id: &str) -> Result<&Row, Error> {
        self.table.row(id).ok_or_else(|| Error::new(code::UNKNOWN_ROW, format!("no row `{id}`")))
    }

    fn binding(&self, row: &Row, args: &Value) -> Result<(), Error> {
        self.gates(row)?;
        let named = argument_names(row, args)?;
        forms::check(row, named, &self.revision)?;
        self.check_filter_refs(row, named)?;
        self.fresh_filter_gate(row, args)
    }

    /// The availability of a row that an entry runs: a data-only conjunction over its stored metadata (KN5.7, LI.20),
    /// whose first failure is reported in this order:
    /// 1. the static check's first failed diagnostic;
    /// 2. a required row the runtime may not invoke (a commit or app row, one that takes a token, or one that
    ///    [`CommandLayer::invoke`] grants no program, a served workspace or file row among them),
    ///    `LG3:capability.not_granted`;
    /// 3. the row's own gates: read-only recovery for a row that changes the session, then a full program lane;
    /// 4. the first failing required row in sorted row ID order, ties in stored order. A row that is not a kernel row
    ///    (absent, or language-defined) is `KN5:input.overlay_dependency_missing`; a static binding answers as
    ///    [`CommandLayer::availability_for`] does for its arguments, and a computed one as
    ///    [`CommandLayer::availability`] does.
    ///
    /// Optional rows take no part. No language code runs, no program starts and no lane is reserved. A stored row that
    /// an entry serves has no metadata, so it answers by its own gates.
    fn entry_availability(&self, row: &Row) -> Result<(), Error> {
        let none = Metadata::default();
        let metadata = self.table.metadata(&row.id).unwrap_or(&none);
        if let Some(diagnostic) = &metadata.diagnostic {
            return Err(diagnostic.clone());
        }
        let mut required: Vec<_> = metadata.requires.iter().filter(|dependency| !dependency.optional).collect();
        required.sort_by(|a, b| a.row.cmp(&b.row));
        let kernel_row = |id: &str| self.table.row(id).filter(|_| !self.table.language_defined(id));
        let ungranted = |row: &Row| !Self::runtime_may(row) || !self.profile_grants(row) || row.takes_token();
        if let Some(denied) =
            required.iter().find_map(|dependency| kernel_row(&dependency.row).filter(|r| ungranted(r)))
        {
            return Err(Error::new(code::NOT_GRANTED, format!("the runtime may not invoke `{}`", denied.id)));
        }
        if self.recovery.is_some() && Self::mutates(row) {
            return Err(Error::new(code::READ_ONLY_RECOVERY, "the session is in read-only recovery"));
        }
        if row.lane == Lane::Program {
            self.program_free()?;
        }
        for dependency in required {
            let Some(kernel) = kernel_row(&dependency.row) else {
                let detail = format!("`{}` requires `{}`, which is not a kernel row", row.id, dependency.row);
                return Err(Error::new(code::OVERLAY_DEPENDENCY_MISSING, detail));
            };
            match &dependency.binding {
                Binding::Static(args) => self.binding(kernel, args)?,
                Binding::Computed => self.gates(kernel)?,
            }
        }
        Ok(())
    }

    /// `row`'s argument-independent gates.
    fn gates(&self, row: &Row) -> Result<(), Error> {
        if row.overlay.is_some() {
            return self.entry_availability(row);
        }
        if self.recovery.is_some() && Self::mutates(row) {
            return Err(Error::new(code::READ_ONLY_RECOVERY, "the session is in read-only recovery"));
        }
        if self.paused(row) {
            return Err(Error::new(code::COMMITS_PAUSED, "the explore state pauses commits (ADR-D01.39)"));
        }
        match row.id.as_str() {
            "operation.preview" => {
                self.retained_mode()?;
                self.exclusive_free()
            }
            "operation.execute" => {
                if self.commit_held {
                    return Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight"));
                }
                self.retained_mode()?;
                let Some(p) = self.slot.as_ref().filter(|p| p.state == PreviewState::Presented) else {
                    return Err(Error::new(code::NO_FRESH_PREVIEW, "no presented preview"));
                };
                if p.deps != self.deps(&self.draft_now(p)) {
                    return Err(Error::new(code::REVISION_CHANGED, "the preview is stale"));
                }
                let moves = match self.policy {
                    ProtectionMode::Net => self.kernel.protection(&p.effect, &self.protected).verdict,
                    // The build judged every prefix under the set and the policy in force: a change of either advances
                    // the protection revision, and the preview's dependencies are current.
                    ProtectionMode::Strict => p.protection.verdict,
                };
                if moves == Verdict::Moves {
                    return Err(Error::new(code::ORBIT_WOULD_MOVE, c600_kernel::PROTECTED_MESSAGE));
                }
                Ok(())
            }
            // A preview that is still building waits as a job on the exclusive lane, and `cancel` takes it from there.
            "operation.preview.cancel" => {
                if self.slot.is_some() || self.jobs.iter().any(Job::previews) {
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
            LAYOUT_UNDO_ROW | LAYOUT_REDO_ROW => {
                if self.commit_held {
                    return Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight"));
                }
                let direction = if row.id == LAYOUT_UNDO_ROW { Direction::Undo } else { Direction::Redo };
                match self.layout_hint(direction) {
                    Some(_) => Ok(()),
                    None => Err(Error::new(code::UNDO_SUPERSEDED, "no eligible target")),
                }
            }
            EXPLORE_ROW | FIND_ROW => Ok(()),
            ONE_KEY_ROW => {
                if self.commit_held {
                    return Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight"));
                }
                self.retained_mode()
            }
            PARTIAL_UNDO_ROW => {
                if self.commit_held {
                    return Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight"));
                }
                self.retained_mode()?;
                match self.head_entry() {
                    Some((_, applied)) if applied > 0 => Ok(()),
                    _ => Err(Error::new(code::UNDO_SUPERSEDED, "no eligible target")),
                }
            }
            ROLLBACK_ROW => {
                if self.commit_held {
                    return Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight"));
                }
                match self.undo.is_empty() && self.redo.is_empty() {
                    true => Err(Error::new(code::UNDO_SUPERSEDED, "no eligible target")),
                    false => Ok(()),
                }
            }
            TWIST_ROW => match (&self.twists, self.commit_held) {
                (None, _) => {
                    Err(Error::new(code::ROW_NOT_BUILT, format!("`{TWIST_ROW}` needs the host's pinned twist menu")))
                }
                (Some(_), true) => Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight")),
                (Some(_), false) => self.retained_mode(),
            },
            JUMBLE_ON_ROW => match (&self.twists, self.commit_held, &self.jumbled) {
                (None, _, _) => Err(Error::new(
                    code::ROW_NOT_BUILT,
                    format!("`{JUMBLE_ON_ROW}` needs the host's pinned twist menu"),
                )),
                (Some(_), true, _) => Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight")),
                (Some(_), false, Some(_)) => {
                    Err(Error::new(code::RETAINED_TOOL_NEEDS_JUMBLE_OFF, "Jumble is on already"))
                }
                (Some(_), false, None) => Ok(()),
            },
            JUMBLE_OFF_ROW => match (&self.twists, self.commit_held, &self.jumbled) {
                (None, _, _) => Err(Error::new(
                    code::ROW_NOT_BUILT,
                    format!("`{JUMBLE_OFF_ROW}` needs the host's pinned twist menu"),
                )),
                (Some(_), true, _) => Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight")),
                (Some(_), false, None) => Err(Error::new(MENU_WORD_NEEDS_JUMBLE_ON, "Jumble is off already")),
                (Some(_), false, Some(_)) => Ok(()),
            },
            JUMBLE_TWIST_ROW => match (&self.twists, self.commit_held) {
                (None, _) => Err(Error::new(
                    code::ROW_NOT_BUILT,
                    format!("`{JUMBLE_TWIST_ROW}` needs the host's pinned twist menu"),
                )),
                (Some(_), true) => Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight")),
                (Some(_), false) => Ok(()),
            },
            REPAIR_LIST_ROW => {
                self.jumble_mode(REPAIR_LIST_ROW)?;
                self.exclusive_free()
            }
            "protection.orbit.set" | POLICY_ROW | GRIP_REBIND_ROW | MACRO_SAVE_ROW => Ok(()),
            RECORDING_START_ROW | RECORDING_STOP_ROW | RECORDING_PLAY_ROW | RECORDING_CONVERT_ROW | JOB_STOP_ROW => {
                self.recording_gates(row)
            }
            RECORD_ROW => match self.provenance {
                Some(_) => Ok(()),
                None => {
                    Err(Error::new(code::ROW_NOT_BUILT, format!("`{RECORD_ROW}` needs the host's provenance reader")))
                }
            },
            WORKSHEET_SAVE_ROW | WORKSHEET_LOAD_ROW | WORKSHEET_COMPARE_ROW => match self.provenance {
                Some(_) => self.retained_mode(),
                None => {
                    Err(Error::new(code::ROW_NOT_BUILT, format!("`{}` needs the host's provenance reader", row.id)))
                }
            },
            VIEW_OPEN_ROW | VIEW_CLOSE_ROW => match self.views {
                Some(_) => Ok(()),
                None => Err(Error::new(code::ROW_NOT_BUILT, format!("`{}` needs the host's view model", row.id))),
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
        // A live playback holds the program lane too (H-08: `recording.play` is a program-lane job row).
        if queued + running + self.rec.plays.len() >= self.program_capacity {
            return Err(Error::new(code::LANE_FULL, "the program lane is full"));
        }
        Ok(())
    }

    /// The request digest: the envelope without the token's authenticator (LI.5) and the caller the host stamped
    /// (ruling 4).
    fn request(&self, caller: &Caller, invocation: &Invocation, inputs: &[Vec<u8>]) -> String {
        let mut request = json::object([
            ("session", Value::String(self.revision.session.clone())),
            ("epoch", Value::Int(invocation.epoch as i64)),
            ("caller", caller.to_json()),
            ("command", Value::String(invocation.command.clone())),
            ("row", Value::String(invocation.row.clone())),
            ("args", invocation.args.clone()),
            ("token", invocation.token.as_ref().map_or(Value::Null, |t| t.binding.to_json())),
        ]);
        // The digest of each argument block, only when there are any: an invocation without blocks keeps the request
        // digest it had before blocks existed, in stored receipts and journals too.
        if let (false, Value::Object(fields)) = (inputs.is_empty(), &mut request) {
            let blocks = inputs.iter().map(|block| Value::String(c600_digest::sha256_hex(block))).collect();
            fields.insert("inputs".to_owned(), Value::Array(blocks));
        }
        // The declared budget, only when there is one: an invocation without one keeps the digest it had before
        // budgets were declared.
        if let (Some(budget), Value::Object(fields)) = (&invocation.budget, &mut request) {
            fields.insert("budget".to_owned(), budget.to_json());
        }
        digest(&request)
    }

    /// The envelope's full digest: the request digest and the token's authenticator (ruling 4).
    fn envelope(invocation: &Invocation, request: &str) -> String {
        let authenticator = invocation.token.as_ref().map_or(Value::Null, |t| Value::String(t.authenticator()));
        digest(&json::object([("request", Value::String(request.to_owned())), ("authenticator", authenticator)]))
    }

    /// Whether the runtime may invoke a row: never a commit or app row (KN5.15), not `protection.orbit.set`, whose
    /// release confirmation is the solver's, not `protection.policy.set`, since the protection a commit is judged under
    /// is the solver's too, not `workspace.set`, which M6 does not grant a program (draft), and not `worksheet.save` or
    /// `worksheet.load`, since a work sheet changes only in a command the solver invokes (LB7.2), and not
    /// `worksheet.compare`, the solver's comparison before a reuse (U6-040; draft), whose job would hold the exclusive
    /// lane the solver's previews take, and not a `recording.*` row: capture and playback are the solver's (EV4.22: RPI.14
    /// stays true, playback is solver-invoked). Of the rows an entry
    /// runs, an overlay row or a served row, only a query or view row: a run alone causes no durable action, so it
    /// reaches no workspace row, whose completion is a durable write outside the undo order, and no file row (findings
    /// B3 and B4 of the KN5 boundary attack). A run that a solver command started reaches none either: a program
    /// prepares previews and views, and the solver saves (ONE.52; the owner's reading of 18 November 2026). The records
    /// a workspace row's job returns are written in that row's completion, which only the solver invokes.
    fn runtime_may(row: &Row) -> bool {
        !matches!(row.kind, Kind::Commit | Kind::App)
            && row.id != "protection.orbit.set"
            && row.id != POLICY_ROW
            && row.id != RECORD_ROW
            && row.id != GRIP_REBIND_ROW
            && row.id != MACRO_SAVE_ROW
            && row.id != WORKSHEET_SAVE_ROW
            && row.id != WORKSHEET_LOAD_ROW
            && row.id != WORKSHEET_COMPARE_ROW
            && !crate::recording::recording_row(&row.id)
            && (row.overlay.is_none() || matches!(row.kind, Kind::Query | Kind::View))
    }

    /// Whether the envelope authenticates: its row exists, its caller may invoke the row and hold what it holds, and
    /// its token, where the row takes one, verifies under this launch's secret (ruling 4).
    fn authentic(&self, caller: &Caller, invocation: &Invocation) -> bool {
        let Some(row) = self.table.row(&invocation.row) else {
            return false;
        };
        match (caller, &invocation.token) {
            (Caller::Runtime { .. }, Some(_)) => false,
            (Caller::Runtime { .. }, None) => Self::runtime_may(row) && self.profile_grants(row) && !row.takes_token(),
            (Caller::Shell, None) => !row.takes_token(),
            (Caller::Shell, Some(token)) => row.takes_token() && token.verify(&self.secret),
        }
    }

    /// Whether this session's profile lets a run invoke `row`, which [`CommandLayer::runtime_may`] allows: BND.16's
    /// table of rows by profile, from chapter 07 section 8 (draft). The kernel knows no profile, so the table sits here
    /// (KN2.6, BND.17).
    /// - `solving`: every row `runtime_may` allows. At `operation.preview` the host's reader admits a record's word
    ///   (KN5.26), and a run's preview is admitted as the run's command allows.
    /// - `tool_authoring` and `research`: the same rows except `operation.preview`, refused with
    ///   `LG3:capability.not_granted`. A run's `propose` there goes to a capture in the runtime, and no engine preview
    ///   exists, so no token and no commit can follow (BND.60). An entry that requires the row is unavailable.
    ///
    /// The cells of section 8 that grant a run more outside a solving session are not built: a run writes no record
    /// of the authoring workspace or the research records in any profile, since the label such a record carries
    /// (BND.61) is the host's to set. The shell's rows are the same in every profile: a profile belongs to a run
    /// (BND.17), and the solver's own commands are no run's.
    fn profile_grants(&self, row: &Row) -> bool {
        self.profile == Profile::Solving || row.id != "operation.preview"
    }

    /// `invoke(command, row, args) -> Receipt` (KN5.1) for an envelope that came on `caller`'s channel. The host stamps
    /// the caller from the channel and never takes it from the envelope, which names none (ruling 4, PHS.52, owner
    /// decision of 16 November 2026): the shell's channel is [`Caller::Shell`], and a run's channel is that run's
    /// [`Caller::Runtime`]. The retry lookup comes first, before lane admission or a token is consumed (chapter 08
    /// section 7, KN5.21): an envelope refused earlier in the epoch without authenticating gets its refusal again, and
    /// a command ID recorded for the caller is answered by [`CommandLayer::kernel_outcome`], so the same content
    /// returns the stored receipt and a retried delivery commits at most once, also across a restart or a reopen, while
    /// other content is refused (CT-KN5-04, LI.30). A refusal of an envelope that does not authenticate is retained for
    /// the epoch under its full digest and never attaches to the command ID, so a later authentic envelope with the ID
    /// is treated as the first (ruling 4); every other answer attaches to the ID (KN5.21).
    pub fn invoke(&mut self, caller: &Caller, invocation: Invocation) -> Receipt {
        self.invoke_with_inputs(caller, invocation, Vec::new())
    }

    /// [`CommandLayer::invoke`] with the provenance blocks of the invocation's arguments (BND.22; LP2 draft). A program
    /// sets no field of a block: the host builds them, for a running program's invocation from the run's context, and
    /// passes them here beside the envelope, never inside it. Blocks change none of the checks before the row is
    /// dispatched (epoch, row, caller, token, recovery, argument names), whose refusals come first. Only a row an entry
    /// serves takes blocks; another row that passed those checks refuses them with `KN5:input.argument_form`. For a
    /// served row, the layer checks each block with the host's reader after the arguments' forms, the `FilterRef`s and
    /// the gate, as for a record's block (PS1.16): a block the reader refuses is refused with its error, and with no
    /// reader the invocation is refused with `KN5:input.row_not_built`. It holds the blocks as given, on the job
    /// ([`ProgramJob::inputs`]), with no limit of its own on their number or size: the host's channel carries them. The
    /// request digest covers each block's SHA-256 when there are any, so a retry with other blocks is other content
    /// (KN5.21), and an invocation without blocks keeps its digest.
    pub fn invoke_with_inputs(&mut self, caller: &Caller, invocation: Invocation, inputs: Vec<Vec<u8>>) -> Receipt {
        let scope = caller.scope();
        let request = self.request(caller, &invocation, &inputs);
        let full = Self::envelope(&invocation, &request);
        let retained = self.retained.get(&(scope.clone(), full.clone())).filter(|r| r.command == invocation.command);
        if let Some(receipt) = retained {
            return receipt.clone();
        }
        let key = (scope.clone(), invocation.command.clone());
        if self.receipts.contains_key(&key) || self.lost.contains_key(&key) {
            return self.kernel_outcome(caller, &invocation.command, &request);
        }
        // A manual step's pending entry is dispatched now (EV4.20); capture reads the invocation before it runs.
        self.rec.awaiting.remove(&key);
        let captured = self.capture_begin(caller, &invocation, &request);
        let (job, status) = match self.dispatch(caller, &invocation, &request, &inputs) {
            Ok(answer) => answer,
            Err(error) => (None, Status::Refused(error)),
        };
        if matches!(status, Status::Refused(_)) && !self.authentic(caller, &invocation) {
            let receipt = Receipt { command: invocation.command.clone(), request: full.clone(), job, status };
            self.retained.insert((scope, full), receipt.clone());
            return receipt;
        }
        let receipt = Receipt { command: invocation.command.clone(), request: request.clone(), job, status };
        let record = self.receipts.entry(key.clone()).or_insert(Record { request, receipt, durable: false });
        let receipt = record.receipt.clone();
        self.capture_end(captured, key);
        self.reconcile_children();
        receipt
    }

    fn dispatch(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
        inputs: &[Vec<u8>],
    ) -> Result<(Option<u64>, Status), Error> {
        if invocation.epoch != self.revision.epoch {
            return refused(code::EPOCH_CHANGED, "the envelope belongs to another engine epoch");
        }
        let Some(row) = self.table.row(&invocation.row).cloned() else {
            return refused(code::UNKNOWN_ROW, format!("no row `{}`", invocation.row));
        };
        // An unattended playback step plays only the eligible row it was checked for, with no token (EV4.12, EV4.17,
        // EV4.22): whatever reached this point, nothing else is dispatched while it plays.
        self.playing_admits(&row, invocation)?;
        if let Caller::Runtime { .. } = caller {
            if matches!(row.kind, Kind::Commit | Kind::App) {
                return refused(code::COMMIT_ROW_REFUSED, format!("the runtime may not invoke `{}` (KN5.15)", row.id));
            }
            if !Self::runtime_may(&row) {
                return refused(code::NOT_GRANTED, format!("the runtime may not invoke `{}`", row.id));
            }
            if !self.profile_grants(&row) {
                let detail = format!("a run in a {} session prepares no engine preview (BND.60)", self.profile.name());
                return refused(code::NOT_GRANTED, detail);
            }
            if invocation.token.is_some() {
                return refused(code::TOKEN_INVALID, "the runtime never holds a token (KN5.15)");
            }
        }
        if self.recovery.is_some() && Self::mutates(&row) {
            return refused(code::READ_ONLY_RECOVERY, "the session is in read-only recovery");
        }
        let args = argument_names(&row, &invocation.args)?;
        if row.takes_token() != invocation.token.is_some() {
            return match invocation.token {
                None => refused(code::TOKEN_INVALID, format!("`{}` needs a user-action token", row.id)),
                Some(_) => refused(code::ARGUMENT_FORM, format!("`{}` takes no token", row.id)),
            };
        }
        if !inputs.is_empty() && row.overlay.is_none() {
            return refused(code::ARGUMENT_FORM, format!("`{}` runs no entry, so it takes no argument blocks", row.id));
        }
        // Before the token is spent: a paused commit leaves the press's token unused (ADR-D01.39).
        if self.paused(&row) {
            return refused(code::COMMITS_PAUSED, "the explore state pauses commits (ADR-D01.39)");
        }
        match row.id.as_str() {
            "operation.preview" => self.admit_preview(caller, invocation),
            "operation.execute" => self.execute(caller, invocation, request),
            "operation.preview.cancel" => self.cancel(caller, invocation),
            "session.undo" => self.undo(caller, invocation, request, History::Moves, Direction::Undo),
            "session.redo" => self.undo(caller, invocation, request, History::Moves, Direction::Redo),
            LAYOUT_UNDO_ROW => self.undo(caller, invocation, request, History::Layout, Direction::Undo),
            LAYOUT_REDO_ROW => self.undo(caller, invocation, request, History::Layout, Direction::Redo),
            EXPLORE_ROW => self.set_explore(caller, &row, args),
            PARTIAL_UNDO_ROW => self.partial_undo(caller, invocation, request),
            ONE_KEY_ROW => self.one_key(caller, invocation, request),
            ROLLBACK_ROW => self.rollback_press(caller, invocation),
            "protection.orbit.set" => self.set_orbits(caller, invocation, request),
            POLICY_ROW => self.set_policy(caller, invocation, request),
            RECORD_ROW => self.write_record(caller, invocation, request),
            GRIP_REBIND_ROW => self.rebind_grip(caller, invocation, request),
            MACRO_SAVE_ROW => self.save_macro(caller, invocation, request),
            WORKSHEET_SAVE_ROW => self.save_worksheet(caller, invocation, request),
            WORKSHEET_LOAD_ROW => self.load_worksheet(caller, invocation, request),
            WORKSHEET_COMPARE_ROW => self.admit_compare(caller, invocation),
            TWIST_ROW => self.twist(caller, invocation, request),
            JUMBLE_ON_ROW => self.jumble_on(caller, invocation, request, &row),
            JUMBLE_OFF_ROW => self.jumble_off(caller, invocation, request, &row),
            JUMBLE_TWIST_ROW => self.twist_menu(caller, invocation, request, &row),
            REPAIR_LIST_ROW => self.admit_repair(caller, invocation),
            VIEW_OPEN_ROW | VIEW_CLOSE_ROW => self.view_row(caller, invocation, &row, args),
            FIND_ROW => self.find_row(caller, &row, args),
            RECORDING_START_ROW => self.recording_start(&row, args),
            RECORDING_STOP_ROW => self.recording_stop(&row, args),
            RECORDING_PLAY_ROW => self.recording_play(caller, invocation, &row, args),
            RECORDING_CONVERT_ROW => self.recording_convert(caller, invocation, request, &row, args),
            JOB_STOP_ROW => self.job_stop(&row, args),
            _ if row.overlay.is_some() => {
                // A language-defined row whose module failed its static check runs nothing: the palette lists it as
                // unavailable with this diagnostic, and a press by key or palette is refused with it (KN5.4, EV4.2).
                if let Some(diagnostic) = self.table.metadata(&row.id).and_then(|stored| stored.diagnostic.clone()) {
                    return Err(diagnostic);
                }
                forms::check(&row, args, &self.revision)?;
                self.check_filter_refs(&row, args)?;
                self.fresh_filter_gate(&row, &invocation.args)?;
                self.check_inputs(&row, inputs)?;
                if row.id == SET_SAVE_ROW {
                    let name = args.get("name").and_then(Value::as_str).unwrap_or_default();
                    self.check_set_name(name, args.get(REPLACE) == Some(&Value::Bool(true)))?;
                }
                self.admit_program(caller, invocation, &row, inputs)
            }
            _ => refused(code::ROW_NOT_BUILT, format!("`{}` is not built in the headless layer", row.id)),
        }
    }

    fn next_job(&mut self) -> u64 {
        self.jobs_issued += 1;
        self.jobs_issued
    }

    // ----- view.open, view.close -----

    /// `view.open {view, source}` and `view.close {view}` (EV3.2, RPI.83; [`crate::view`]). Refused, calling no host,
    /// in this order: no view model (`KN5:input.row_not_built`); the argument forms, and an overlay source that does
    /// not name an overlay row of kind `view` or whose arguments do not fit it (`KN5:input.argument_form`), name a
    /// revision that is not current or a `FilterRef` that does not resolve (that row's own refusals); a run not inside
    /// a command the solver invoked (`LG3:capability.not_granted`). Then the host's view model answers; an admitted
    /// open or close is `accepted` with no job, the layer's final answer for either row.
    fn view_row(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        if self.views.is_none() {
            return refused(code::ROW_NOT_BUILT, format!("`{}` needs the host's view model", row.id));
        }
        forms::check_forms(row, args)?;
        let view = args.get("view").and_then(Value::as_str).unwrap_or_default();
        let source = match args.get("source").and_then(ViewSourceRef::from_json) {
            Some(source) if row.id == VIEW_OPEN_ROW => Some(source),
            _ => None,
        };
        if let Some(ViewSourceRef::Overlay { row: name, arguments, .. }) = &source {
            let Some(overlay) = self.table.row(name).filter(|r| r.overlay.is_some() && r.kind == Kind::View) else {
                return refused(code::ARGUMENT_FORM, format!("`{name}` is not an overlay row of kind `view`"));
            };
            if !forms::names_fit(overlay, arguments.keys().map(String::as_str)) {
                return refused(code::ARGUMENT_FORM, format!("`{name}` takes {:?}", overlay.arguments));
            }
            forms::check(overlay, arguments, &self.revision)?;
            self.check_filter_refs(overlay, arguments)?;
        }
        if let Caller::Runtime { run } = caller {
            if !self.inside_solver_command(run) {
                let detail = format!("run `{run}` is not inside a command the solver invoked (RPI.83)");
                return refused(code::NOT_GRANTED, detail);
            }
        }
        let views = self.views.as_mut().expect("checked above");
        let command = invocation.command.as_str();
        match &source {
            Some(source) => views.open(command, view, source)?,
            None => views.close(command, view)?,
        }
        Ok((None, Status::Accepted))
    }

    // ----- command.find -----

    /// `command.find {query, object?}` (EV4.2; DT-M1 C1): opens the palette over `query`. The palette's data is the
    /// shell's read of the table and KN5.6's availability table ([`crate::palette`]), searched with no engine call per
    /// keystroke, so the layer only admits the row. Refused, in this order: a malformed `query`
    /// (`KN5:input.argument_form`); a run not inside a command the solver invoked (`LG3:capability.not_granted`, as for
    /// `view.open`, RPI.83). An admitted find is `accepted` with no job and changes nothing, the layer's final answer.
    /// The selection `object` passes unchecked, as its type's form is not written down, and filters nothing yet.
    fn find_row(
        &self,
        caller: &Caller,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        forms::check_forms(row, args)?;
        if let Caller::Runtime { run } = caller {
            if !self.inside_solver_command(run) {
                let detail = format!("run `{run}` is not inside a command the solver invoked (RPI.83)");
                return refused(code::NOT_GRANTED, detail);
            }
        }
        Ok((None, Status::Accepted))
    }

    // ----- operation.preview -----

    /// Admits a draft: `{"recipe": [...]}`, an inline recipe whose draft digest is the draft's digest, or
    /// `{"record": name}`, a `ResolvedDraftRef` to a workspace record that holds a word's letters, resolved now
    /// ([`CommandLayer::resolve_record`]). A refused draft takes no job and leaves the slot as it was (KN5.11).
    fn admit_preview(&mut self, caller: &Caller, invocation: &Invocation) -> Result<(Option<u64>, Status), Error> {
        if let Caller::Runtime { run } = caller {
            self.run_may_preview(run)?;
        }
        self.retained_mode()?;
        let draft = invocation.args.get("draft").expect("the form check saw the draft");
        let only = |key: &str| draft.get(key).filter(|_| draft.as_object().is_some_and(|o| o.len() == 1));
        let (recipe, digested, resolved) = if let Some(recipe) = only("recipe") {
            (self.kernel.recipe(recipe)?, digest(draft), None)
        } else if let Some(name) = only("record").and_then(Value::as_str) {
            let resolved = self.resolve_record(draft, name)?;
            (self.kernel.word(&resolved.letters)?, resolved.digest(), Some(resolved))
        } else {
            return refused(code::ARGUMENT_FORM, "a draft is `{\"recipe\": [...]}` or `{\"record\": name}`");
        };
        self.exclusive_free()?;
        let id = self.next_job();
        let kind = JobKind::Preview { recipe, draft: digested, resolved, caller: caller.clone() };
        let (command, scope) = (invocation.command.clone(), caller.scope());
        self.jobs.push_back(Job { id, command, scope, lane: Lane::Exclusive, kind });
        self.generation += 1;
        Ok((Some(id), Status::Running(id)))
    }

    /// Whether record `name` holds a value the solver kept with `workspace.set` (ADR-D01.12; the layer's `kept`).
    fn kept_now(&self, name: &str) -> bool {
        let value = self.records.get(name).and_then(|record| record.value.as_ref());
        value.is_some_and(|value| self.kept.contains(&(name.to_owned(), digest(&value.to_json()))))
    }

    /// Whether a stored one-key commit's `recipe` is the word of a record that holds a value the solver kept: the only
    /// commit the live press makes (ADR-D01.12).
    fn kept_word(&self, recipe: &Recipe) -> bool {
        self.records.iter().any(|(name, record)| {
            let letters = record.value.as_ref().and_then(RecordValue::operation);
            let word = letters.and_then(|letters| self.kernel.word(&letters).ok());
            self.kept_now(name) && word.is_some_and(|word| word == *recipe)
        })
    }

    /// Resolves a draft `{"record": name}` (KN0.8, KN5.26): the record must hold a list of letters, as in M6, or a work
    /// sheet, whose complete operation it previews ([`RecordValue::operation`]; `KN2:input.named_object_invalid`
    /// otherwise). In a solving session the host's provenance reader must admit the record's block, or the draft is
    /// refused with the reader's error (`LG3:capability.not_granted` without a reader). The record is checked before
    /// the reader, so a session without a reader answers `named_object_invalid` for a record that holds no letters. In
    /// a `tool_authoring` or `research` session KN5.26 does not apply and no reader is asked, so the solver's own
    /// preview there, and a commit of it, needs no admission; a record previewed later in a solving session meets its
    /// check there (BND.61; draft). The read set holds the record's content digest and version.
    fn resolve_record(&self, draft: &Value, name: &str) -> Result<ResolvedDraft, Error> {
        let record = self.records.get(name);
        let Some(value) = record.and_then(|record| record.value.as_ref()) else {
            return Err(Error::new(code::NAMED_OBJECT_INVALID, "no such record"));
        };
        let Some(letters) = value.operation() else {
            let detail = "the record holds neither a list of letters nor a work sheet";
            return Err(Error::new(code::NAMED_OBJECT_INVALID, detail));
        };
        if self.profile == Profile::Solving {
            let unread =
                || Error::new(code::NOT_GRANTED, "a record's word needs the host's provenance reader (KN5.26)");
            self.provenance.as_ref().ok_or_else(unread)?.admit_kept(value.provenance.as_bytes(), &self.blocks)?;
        }
        Ok(ResolvedDraft { source: draft.clone(), letters, reads: vec![RecordRead::of(name, record)] })
    }

    /// The draft digest of `p`'s draft as its records stand now (KN0.8). A record draft reads its records again, so a
    /// write of one, its undo included, makes the preview stale; an inline recipe's digest never changes.
    fn draft_now(&self, p: &Preview) -> String {
        match &p.resolved {
            None => p.deps.draft.clone(),
            Some(resolved) => {
                let reads =
                    resolved.reads.iter().map(|read| RecordRead::of(&read.record, self.records.get(&read.record)));
                ResolvedDraft { reads: reads.collect(), ..resolved.clone() }.digest()
            }
        }
    }

    /// Runs every queued preview job at the current revision, in order. Program jobs wait for their host.
    pub fn run_jobs(&mut self) -> usize {
        let mut ran = 0;
        let mut keep = VecDeque::new();
        while let Some(job) = self.jobs.pop_front() {
            match job.kind {
                JobKind::Program(_) => keep.push_back(job),
                JobKind::Preview { recipe, draft, resolved, caller } => {
                    self.build_preview(job.id, &job.command, recipe, &draft, resolved, caller);
                    ran += 1;
                }
                JobKind::Compare { sheet, draft } => {
                    self.build_comparison(job.id, &(job.scope, job.command), &sheet, &draft);
                    ran += 1;
                }
                JobKind::Repair { cap } => {
                    self.build_repair(job.id, &(job.scope, job.command), cap);
                    ran += 1;
                }
            }
        }
        self.jobs = keep;
        // A playback's user step may be a preview job that just settled (EV4.20).
        self.reconcile_children();
        ran + self.run_plays()
    }

    fn build_preview(
        &mut self,
        id: u64,
        command: &str,
        recipe: Recipe,
        draft: &str,
        resolved: Option<ResolvedDraft>,
        caller: Caller,
    ) {
        let key = (caller.scope(), command.to_owned());
        // A run's preview is checked again when it builds: the run may have ended, or a recording's run may find a
        // preview pending that was not when its job was admitted.
        if let Caller::Runtime { run } = &caller {
            if let Err(error) = self.run_may_preview(run) {
                self.settle(&key, Status::Failed(error));
                self.generation += 1;
                return;
            }
        }
        let base = self.current.clone();
        let status = match self.derive(&base, &recipe, &self.protected) {
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
                    resolved,
                    literal: effect.expansion_sha256.clone(),
                    state: PreviewState::Ready,
                    base,
                    end,
                    recipe,
                    effect,
                    protection,
                    replaced: self.replacing(),
                };
                let result = ResultRef { job: Some(id), digest: digest(&preview.summary()) };
                if let Some(old) = self.slot.replace(preview) {
                    self.history.insert(old.id, PreviewState::Replaced);
                }
                let revision = self.revision.clone();
                Status::Completed(Completion { kind: CompletionKind::PreviewReady, revision, result: Some(result) })
            }
        };
        self.settle(&key, status);
        self.generation += 1;
    }

    /// Settles a transient receipt. A durable record is never overwritten.
    fn settle(&mut self, key: &Key, status: Status) {
        if let Some(record) = self.receipts.get_mut(key).filter(|record| !record.durable) {
            record.receipt.status = status;
        }
    }

    // ----- operation.preview.cancel -----

    fn cancel(&mut self, caller: &Caller, invocation: &Invocation) -> Result<(Option<u64>, Status), Error> {
        let Some(id) = nat(invocation.args.get("preview")) else {
            return refused(code::ARGUMENT_FORM, "`preview` is a preview ID");
        };
        if matches!(caller, Caller::Runtime { .. }) && self.shown().is_some_and(|p| p.id == id) {
            return refused(
                code::PREVIEW_SHOWN,
                format!("preview {id} is shown to the solver; only the solver ends it"),
            );
        }
        // A recording never replaces a preview (ADR-D01.9), so it does not end one first either: a run inside a
        // `recording.play` job cancels only a preview its own run asked for.
        if let Caller::Runtime { run } = caller {
            let pending = self.slot.as_ref().filter(|p| p.id == id).map(|p| p.caller.scope());
            let building = || self.jobs.iter().find(|job| job.id == id && job.previews()).map(|job| job.scope.clone());
            let owner = pending.or_else(building);
            if self.in_recording(run) && owner.is_some_and(|scope| scope != caller.scope()) {
                let detail = format!("a recording never replaces a preview (ADR-D01.9): preview {id} is not its own");
                return refused(code::NOT_GRANTED, detail);
            }
        }
        let done = Completion { kind: CompletionKind::Workspace, revision: self.revision.clone(), result: None };
        // The solver's cancel ends the line of previews the mark follows; a run's cancel of its own preview does not,
        // so the next preview still carries the mark of the one the solver saw (ADR-D01.9).
        let solver = matches!(caller, Caller::Shell);
        if self.slot.as_ref().is_some_and(|p| p.id == id) {
            self.slot.take();
            self.history.insert(id, PreviewState::Cancelled);
            if solver {
                self.last_seen = None;
            }
            self.generation += 1;
            return Ok((None, Status::Completed(done)));
        }
        if let Some(position) = self.jobs.iter().position(|job| job.id == id && job.previews()) {
            let job = self.jobs.remove(position).expect("the position is in range");
            self.settle(&(job.scope, job.command), Status::Cancelled(CancelReason::ByUser));
            self.history.insert(id, PreviewState::Cancelled);
            if solver {
                self.last_seen = None;
            }
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

    /// Step 3: the authenticator (`KN5:auth.token_invalid`), then the session and epoch (`KN5:stale.token_mismatch`,
    /// draft ruling 5), then the nonce. The nonce is spent once the token passes all three, so a press refused at a
    /// later step stays spent.
    fn authenticate(&mut self, invocation: &Invocation) -> Result<TokenBinding, Error> {
        let Some(token) = &invocation.token else {
            return Err(Error::new(code::TOKEN_INVALID, "no token"));
        };
        if !token.verify(&self.secret) {
            return Err(Error::new(code::TOKEN_INVALID, "the token's authenticator does not verify"));
        }
        let b = &token.binding;
        if b.session != self.revision.session || b.epoch != self.revision.epoch {
            return Err(Error::new(code::TOKEN_MISMATCH, "the token belongs to another session or epoch"));
        }
        if !self.used.insert(b.nonce) {
            return Err(Error::new(code::TOKEN_USED, "the token was used"));
        }
        Ok(b.clone())
    }

    /// Step 5's binding: a genuine token's command, row, absolute arguments and identity block are this invocation's.
    fn bound(&self, binding: &TokenBinding, invocation: &Invocation) -> Result<(), Error> {
        if binding.command != invocation.command
            || binding.row != invocation.row
            || binding.args != args_digest(&invocation.args)
            || binding.identity != self.identity
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

    fn execute(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let Some(id) = nat(invocation.args.get("preview")) else {
            return refused(code::ARGUMENT_FORM, "`preview` is a preview ID");
        };
        // ADR-D01.9's settle interval ignores the press before its token is spent. It is not a gate: it ends with the
        // clock, not with an action that advances the generation (LI.21), so availability keeps the key on and a press
        // inside the interval is ignored.
        if let Some(p) = self.slot.as_ref().filter(|p| p.id == id && Self::settles(p)) {
            self.settled(p)?;
        }
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        self.retained_mode()?;
        let Some(bound) = binding.preview.as_ref() else {
            return refused(code::TOKEN_MISMATCH, "a word commit's token binds the presented preview");
        };
        // The token's preview ended: an empty slot is `no_fresh_preview`, a slot holding another preview is
        // `preview_replaced` (ruling 1).
        match &self.slot {
            None => return refused(code::NO_FRESH_PREVIEW, format!("preview {} is not pending", bound.id)),
            Some(p) if p.id != bound.id => {
                return refused(code::PREVIEW_REPLACED, format!("preview {} was replaced by {}", bound.id, p.id))
            }
            Some(_) => {}
        }
        if id != bound.id {
            return refused(code::TOKEN_MISMATCH, "the token binds another preview");
        }
        let p = self.slot.as_ref().expect("the slot holds the token's preview");
        if p.state != PreviewState::Presented {
            return refused(code::NO_FRESH_PREVIEW, format!("preview {id} is not presented"));
        }
        // The token binds the frame that presented the preview: a frame before its current presentation, on another
        // device or surface, or never reported does not show it now (ONE.41, RM.26).
        if !self.in_run(id, &bound.frame) {
            return refused(code::NO_FRESH_PREVIEW, format!("the token's frame does not present preview {id} now"));
        }
        if p.literal != bound.literal {
            return refused(code::TOKEN_MISMATCH, "the token's literal digest differs from the preview's");
        }
        let end = self.recheck(p, Some(&bound.deps))?;
        let (recipe, literal) = (p.recipe.clone(), p.literal.clone());
        let adoption = self.commit_adoption(Meta::of(caller, invocation, request), recipe, end, literal, Some(id));
        self.transact(adoption)
    }

    /// Steps 6 to 8 of a preview's commit, which `operation.execute` and the one-key commit share: the preview's
    /// dependencies as they stand now, and as the token bound them where it binds them (`KN5:stale.revision_changed`),
    /// the head hash, which must be the head's recorded state and the preview's start (`KN5:stale.state_hash_mismatch`),
    /// and the operation judged again under the policy in force, Net its effect and Strict every executed prefix of its
    /// recipe's literal expansion (`KN2:protection.orbit_would_move`). The answer is the state the commit adopts.
    fn recheck(&self, p: &Preview, bound: Option<&PreviewDeps>) -> Result<Snapshot, Error> {
        let now = self.deps(&self.draft_now(p));
        if p.deps != now || bound.is_some_and(|deps| *deps != now) {
            return Err(Error::new(code::REVISION_CHANGED, "the preview's dependencies changed"));
        }
        if self.current.digest() != self.nodes[&self.head].digest {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state"));
        }
        if self.current.digest() != p.base.digest() {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the head hash differs from the preview's start"));
        }
        // Step 8 judges the operation again under the policy in force: Net its effect, Strict every executed prefix of
        // its recipe's literal expansion.
        let protection = match self.policy {
            ProtectionMode::Net => self.kernel.protection(&p.effect, &self.protected),
            ProtectionMode::Strict => self.kernel.strict_protection(&p.recipe, &self.protected)?,
        };
        match (&p.end, protection.verdict) {
            (Some(end), Verdict::Clear) => Ok(end.clone()),
            _ => Err(Error::new(code::ORBIT_WOULD_MOVE, c600_kernel::PROTECTED_MESSAGE)),
        }
    }

    // ----- operation.preview.commit -----

    /// `operation.preview.commit {draft, token}` (ADR-D01.12; draft): one press prepares, rechecks and commits the
    /// solver's own macro. The draft is `{"record": name}` only (`KN5:input.argument_form` otherwise, before the token
    /// is spent), a record resolved now as `operation.preview` resolves one, so in a solving session the host's
    /// provenance reader admits it (KN5.26): no inline recipe, so no keymap a script supplies commits a word the solver
    /// never kept. After the token, the commit lane, the binding, Jumble off and the token's state and protection
    /// revisions (LI.6), so that a stale press leaves the pending preview in place, the record must hold a value the
    /// solver kept with `workspace.set` (`KN2:input.named_object_invalid` otherwise; a value a job's completion, a
    /// recording or a work sheet load wrote is not their own macro until they keep it), and the record is resolved and its
    /// preview built at the head, replacing the pending preview (ADR-D01.9); a draft that does not resolve, or a word
    /// the kernel refuses, builds none. Then the preview is checked as `operation.execute` checks a presented one
    /// ([`Self::recheck`]) and committed with its preview's ID. A failed recheck leaves the preview pending and
    /// uncommitted, for the solver to see. No run reaches it, since
    /// it is a commit row, and the explore state pauses it. A twist's one-key commit is `twist.apply`.
    fn one_key(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let draft = invocation.args.get("draft").cloned().unwrap_or(Value::Null);
        let only = draft.as_object().filter(|fields| fields.len() == 1);
        let Some(name) = only.and_then(|fields| fields.get("record")).and_then(Value::as_str).map(str::to_owned) else {
            return refused(code::ARGUMENT_FORM, "a one-key draft is `{\"record\": name}`, the solver's own macro");
        };
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        self.retained_mode()?;
        // Before the build, so a stale press leaves the solver's pending preview in place.
        if binding.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        if !self.kept_now(&name) {
            let detail =
                format!("record `{name}` does not hold a value the solver kept with `{RECORD_ROW}` (ADR-D01.12)");
            return refused(code::NAMED_OBJECT_INVALID, detail);
        }
        let resolved = self.resolve_record(&draft, &name)?;
        let recipe = self.kernel.word(&resolved.letters)?;
        let base = self.current.clone();
        let (end, effect, protection) = match self.derive(&base, &recipe, &self.protected).map_err(kernel_error)? {
            Derivation::Derived(d) => (Some(d.state), d.effect, d.protection),
            Derivation::Refused(r) => (None, r.effect, r.protection),
        };
        let id = self.next_job();
        let preview = Preview {
            id,
            command: invocation.command.clone(),
            caller: caller.clone(),
            deps: self.deps(&resolved.digest()),
            resolved: Some(resolved),
            literal: effect.expansion_sha256.clone(),
            state: PreviewState::Ready,
            base,
            end,
            recipe,
            effect,
            protection,
            replaced: self.replacing(),
        };
        if let Some(old) = self.slot.replace(preview) {
            self.history.insert(old.id, PreviewState::Replaced);
        }
        self.generation += 1;
        let p = self.slot.as_ref().expect("the slot holds the preview just built");
        let end = self.recheck(p, None)?;
        let (recipe, literal) = (p.recipe.clone(), p.literal.clone());
        let adoption = self.commit_adoption(Meta::of(caller, invocation, request), recipe, end, literal, Some(id));
        self.transact(adoption)
    }

    /// A commit of `recipe`, which takes the head from the current state to `end`: a new node under the head.
    fn commit_adoption(
        &self,
        meta: Meta,
        recipe: Recipe,
        end: Snapshot,
        literal: String,
        preview: Option<u64>,
    ) -> Adoption {
        let order = self.order + 1;
        let event = format!("e{order}");
        let post = end.digest().to_owned();
        let revision = Revision {
            head: event.clone(),
            state: self.revision.state + 1,
            digest: post.clone(),
            ..self.revision.clone()
        };
        let depth = self.nodes[&self.head].depth + 1;
        // A partial undo stays inside the head's entry, with fewer of its moves applied (ADR-D01.15).
        let partial = match meta.row.as_str() {
            PARTIAL_UNDO_ROW => {
                self.inside(self.head).map(|(entry, applied)| (entry, applied.saturating_sub(recipe.primitive_count())))
            }
            _ => None,
        };
        let kept = Some(recipe.clone());
        let node =
            Node { parent: self.head, recipe: kept, twist: None, exit: None, partial, digest: post.clone(), depth };
        let transaction = Transaction {
            kind: TransactionKind::Commit,
            event,
            order,
            target: None,
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal,
            pre: self.current.digest().to_owned(),
            post,
            revision,
            recipe: Value::Array(steps_json(&recipe, false)),
            orbits: self.protected.orbits().to_vec(),
            confirm: meta.confirm,
            job: None,
            result: None,
            record: None,
            policy: None,
            records: Vec::new(),
            rotation: None,
            crossing: None,
            blocks: Vec::new(),
            budget: None,
        };
        let change = Change::Head { before: self.head, after: order };
        Adoption {
            transaction,
            state: Some(end),
            protected: None,
            head: order,
            tree: Tree::Push { node: Some((order, node)), change },
            preview,
            side: Side::Stay,
        }
    }

    /// Steps 9 and 10: writes the action and adopts it only when the store answers that it is durable. From the write
    /// until the adoption ends, the layer holds the action as one whose fate is unknown, in read-only recovery
    /// (CT-KN5-05, L12 of the KN5 attack leads): a panic that unwinds out of the store's write leaves the action
    /// pending for [`CommandLayer::restart`], which settles it from the store's answer as after an unknown fate, and
    /// a panic in the adoption leaves the layer read-only until the session is reopened from its store.
    fn transact(&mut self, mut adoption: Adoption) -> Result<(Option<u64>, Status), Error> {
        adoption.transaction.blocks = self.parts_to_keep(&adoption.transaction);
        let unfinished = Error::new(code::COMMIT_FATE, "an action's write or adoption did not finish");
        let before = self.recovery.replace(unfinished);
        let pending = self.limbo.insert(adoption);
        match self.store.transact(&pending.transaction) {
            Fate::Durable => {
                let adoption = self.limbo.take().expect("the action being written");
                let receipt = self.adopt(adoption);
                self.recovery = before;
                Ok((receipt.job, receipt.status))
            }
            Fate::RolledBack(error) => {
                self.limbo = None;
                self.recovery = before;
                Ok((None, Status::Failed(error)))
            }
            Fate::Unknown => {
                self.recovery = Some(Error::new(code::COMMIT_FATE, "the store cannot say whether the action landed"));
                self.generation += 1;
                Ok((None, Status::UnknownOutcome))
            }
        }
    }

    /// The part blocks a record write or a workspace row's completion keeps (CT-BND-27; draft): those the host's reader
    /// holds for the blocks the action writes ([`Provenance::parts`]) that the session does not keep yet, in the order
    /// of their digests. An undo or redo writes values whose parts were kept when they were first written.
    fn parts_to_keep(&self, transaction: &Transaction) -> Vec<String> {
        let (Some(reader), TransactionKind::Record | TransactionKind::Workspace) = (&self.provenance, transaction.kind)
        else {
            return Vec::new();
        };
        let mut new = BTreeMap::new();
        for value in transaction.record.iter().chain(&transaction.records).filter_map(|write| write.value.as_ref()) {
            for part in reader.parts(value.provenance.as_bytes()) {
                let digest = sha256_hex(part.as_bytes());
                if !self.blocks.contains_key(&digest) {
                    new.insert(digest, part);
                }
            }
        }
        new.into_values().collect()
    }

    /// Step 10: applies a durable action, stores its durable receipt and returns it. Only an action that moves the
    /// puzzle ends the pending preview (committed when it is the one committed, discarded otherwise) and cancels the
    /// jobs on the replaced revision, except file jobs (KN5.18).
    fn adopt(&mut self, adoption: Adoption) -> Receipt {
        let Adoption { transaction, state, protected, head, tree, preview, side } = adoption;
        let moved = state.is_some() || !matches!(side, Side::Stay);
        match side {
            Side::Stay => {}
            Side::On(jumbled) => self.jumbled = Some(jumbled),
            Side::Off => self.jumbled = None,
        }
        if let Some(state) = state {
            self.current = state;
        }
        if let Some(protected) = protected {
            self.protected = protected;
        }
        self.revision = transaction.revision.clone();
        self.order = transaction.order;
        self.head = head;
        // The action each durable invocation took, for a recording's `action` and a playback's map (LI.9, EV4.18).
        self.rec.took.insert((transaction.caller.scope(), transaction.command.clone()), transaction.order);
        // A record write, or its undo or redo, or a workspace row's writes: each record's version advances and never
        // goes back (KN0.8).
        for RecordWrite { name, value } in transaction.record.iter().chain(&transaction.records) {
            let record = self.records.entry(name.clone()).or_default();
            record.version += 1;
            record.value = value.clone();
        }
        // The solver's own `workspace.set` keeps its value for the one-key commit (ADR-D01.12): a value a job's
        // completion, a recording or a work sheet load wrote is not the solver's own macro until they keep it.
        if let (TransactionKind::Record, Caller::Shell, Some(RecordWrite { name, value: Some(value) })) =
            (transaction.kind, &transaction.caller, &transaction.record)
        {
            if transaction.row == RECORD_ROW {
                self.kept.insert((name.clone(), digest(&value.to_json())));
            }
        }
        // The part blocks it keeps stay for the session, an undo's included (CT-BND-27).
        for block in &transaction.blocks {
            self.blocks.entry(sha256_hex(block.as_bytes())).or_insert_with(|| block.clone());
        }
        // No undo reaches a workspace completion of a row whose undo class is not `workspace`, so an undo or redo of an
        // earlier write of one of its records would write over it. The actions that wrote one leave both stacks
        // (draft), and the cut runs on beneath them ([`cut_stack`]). An undoable completion, and an undo or redo of
        // one, keep them: the stack inverts them in order.
        //
        // Neither history holds the other's steps (ADR-D01.14), so a new step of one history cuts the other's actions
        // that wrote one of its records the same way: an undo there would write over the new step's write.
        let writes = transaction.record.iter().chain(&transaction.records);
        let cut: BTreeSet<String> = writes.map(|write| write.name.clone()).collect();
        let others: &[History] = match &tree {
            Tree::None => &[History::Moves, History::Layout],
            Tree::Push { .. } => &[History::Layout],
            Tree::LayoutPush(_) => &[History::Moves],
            Tree::Undo | Tree::Redo | Tree::LayoutUndo | Tree::LayoutRedo => &[],
        };
        if !cut.is_empty() {
            for &history in others {
                self.cut_history(history, &cut);
            }
        }
        // A predicate row's completion is what a `FilterRef` names while its writes stand ([`crate::filter`]). Those
        // whose writes another action wrote over are dropped.
        let predicate = transaction.kind == TransactionKind::Workspace
            && filter::PREDICATE_ROWS.contains(&transaction.row.as_str());
        if let (true, Some(job), Some(result)) = (predicate, transaction.job, &transaction.result) {
            let records = &self.records;
            let version = |name: &str| records.get(name).map_or(0, |record| record.version);
            self.predicates.retain(|_, predicate| predicate.writes.iter().all(|(name, at)| version(name) == *at));
            let writes = transaction.records.iter().map(|write| (write.name.clone(), version(&write.name))).collect();
            self.predicates.insert(job, Predicate { digest: result.clone(), writes });
        }
        // A policy change, or its undo or redo.
        if let Some(policy) = transaction.policy {
            self.policy = policy;
        }
        match tree {
            Tree::Push { node, change } => {
                for order in std::mem::take(&mut self.redo) {
                    self.changes.remove(&order);
                }
                if let Some((key, node)) = node {
                    self.nodes.insert(key, node);
                }
                self.changes.insert(transaction.order, change);
                self.undo.push(transaction.order);
            }
            Tree::Undo => {
                if let Some(order) = self.undo.pop() {
                    self.redo.push(order);
                }
            }
            Tree::Redo => {
                if let Some(order) = self.redo.pop() {
                    self.undo.push(order);
                }
            }
            Tree::LayoutPush(change) => {
                for order in std::mem::take(&mut self.layout_redo) {
                    self.changes.remove(&order);
                }
                self.changes.insert(transaction.order, change);
                self.layout_undo.push(transaction.order);
            }
            Tree::LayoutUndo => {
                if let Some(order) = self.layout_undo.pop() {
                    self.layout_redo.push(order);
                }
            }
            Tree::LayoutRedo => {
                if let Some(order) = self.layout_redo.pop() {
                    self.layout_undo.push(order);
                }
            }
            Tree::None => {}
        }
        // A rollback's step, for the replay's check of the next one and LI.30's lookup (ADR-D01.15; R2-5).
        self.last_step = None;
        if transaction.row == ROLLBACK_ROW {
            let scope = transaction.caller.scope();
            if let Some((gesture, step, of)) = rollback_step(&transaction.command) {
                self.gestures.entry((scope.clone(), gesture.to_owned())).or_insert(of);
                let (gesture, kind, epoch) = (gesture.to_owned(), transaction.kind, transaction.revision.epoch);
                self.last_step = Some(StepAt { gesture, step, of, kind, epoch });
            }
            self.step_ids.insert((scope, transaction.command.clone()));
        }
        let receipt = transaction.receipt();
        let key = (transaction.caller.scope(), transaction.command.clone());
        self.lost.remove(&key);
        let record = Record { request: transaction.request.clone(), receipt: receipt.clone(), durable: true };
        self.receipts.insert(key, record);
        if moved {
            self.end_revision(preview);
        }
        self.generation += 1;
        receipt
    }

    /// Drops the actions of `history`'s stacks that a write of `cut` reaches ([`cut_stack`]), with their changes.
    fn cut_history(&mut self, history: History, cut: &BTreeSet<String>) {
        let (undo, redo) = match history {
            History::Moves => (&mut self.undo, &mut self.redo),
            History::Layout => (&mut self.layout_undo, &mut self.layout_redo),
        };
        let mut dropped = cut_stack(&self.changes, undo, cut);
        dropped.extend(cut_stack(&self.changes, redo, cut));
        undo.retain(|order| !dropped.contains(order));
        redo.retain(|order| !dropped.contains(order));
        for order in dropped {
            self.changes.remove(&order);
        }
    }

    /// Ends what read the state an action replaced: the pending preview, and every job but file jobs (KN5.18).
    fn end_revision(&mut self, committed: Option<u64>) {
        self.last_seen = None;
        if let Some(p) = self.slot.take() {
            let state = if committed == Some(p.id) { PreviewState::Committed } else { PreviewState::Discarded };
            self.history.insert(p.id, state);
        }
        let mut keep = VecDeque::new();
        for job in std::mem::take(&mut self.jobs) {
            if matches!(&job.kind, JobKind::Program(program) if program.kind == Kind::File) {
                keep.push_back(job);
                continue;
            }
            if job.previews() {
                self.history.insert(job.id, PreviewState::Discarded);
            }
            self.settle(&(job.scope, job.command), Status::Cancelled(CancelReason::ReplacedRevision));
        }
        self.jobs = keep;
        // A comparison analysed the state this action replaced (LB7.2: a stored result never stands in for a fresh
        // one), and a repair list speaks only of the input it read (JI.12).
        self.comparison = None;
        self.repairs = None;
        let running = std::mem::take(&mut self.running);
        for (id, program) in running {
            if program.kind == Kind::File {
                self.running.insert(id, program);
            } else {
                let key = (program.caller.scope(), program.command);
                self.settle(&key, Status::Cancelled(CancelReason::ReplacedRevision));
            }
        }
    }

    // ----- twist.apply -----

    /// `twist.apply` (A6; ONE.46, ONE.78): a live twist, committed at once with no preview, under the protection in
    /// force at the current revision. Before the token is spent, the arguments must form KN3.7's binding
    /// (`KN2:input.rotation_request_malformed`), which must resolve against the pinned menu (`KN2:input.*`, KN3.8: a free
    /// binding is `menu_binding_mismatch`, as is a menu pin other than the pinned menu's; a direct rotation off the menu
    /// is `twist_off_menu`), and the item must be a retained one (`KN5:input.menu_word_needs_jumble_on` for a jumble
    /// item, JI.35) that moves a piece (`KN2:input.recipe_shape` for the identity). Then the commit steps run: the token
    /// (step 3), the commit lane (step 4), its binding to this invocation and to the state and protection revisions
    /// (step 5 and LI.6: `KN5:stale.revision_changed`), the head hash (step 7), the kernel's derivation under the
    /// policy in force (step 8: `KN2:protection.orbit_would_move`), the durable transaction with its rotation record
    /// (step 9) and the adoption (step 10, LI.32). A refusal changes nothing.
    fn twist(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let Some(twists) = &self.twists else {
            return refused(code::ROW_NOT_BUILT, format!("`{TWIST_ROW}` needs the host's pinned twist menu"));
        };
        let binding = twist::binding(&invocation.args)?;
        let resolution = rotation::resolve(&twists.context, Some(&*twists.menu), &binding)?;
        // With Jumble on, the mode check after the token refuses the row (LG2.8), so the item's level is judged only
        // with Jumble off.
        let word = match self.jumbled {
            Some(_) => None,
            None => Some(rotation::retained_word(&twists.context, &resolution)?),
        };
        if word.as_ref().is_some_and(Vec::is_empty) {
            return refused(c600_kernel::codes::RECIPE_SHAPE, "the menu's identity item moves no piece");
        }
        let recipe = word.map(|word| self.kernel.word(&word)).transpose()?;
        let token = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&token, invocation)?;
        self.retained_mode()?;
        let recipe = recipe.expect("with Jumble off, the item's retained word is a recipe");
        if token.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        if self.current.digest() != self.nodes[&self.head].digest {
            return refused(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state");
        }
        let derived = match self.derive(&self.current, &recipe, &self.protected).map_err(kernel_error)? {
            Derivation::Derived(derived) => derived,
            Derivation::Refused(refusal) => return Err(refusal.refusal.into()),
        };
        let meta = Meta::of(caller, invocation, request);
        let mut adoption = self.commit_adoption(meta, recipe, derived.state, derived.word, None);
        adoption.transaction.rotation = Some(resolution.to_json());
        self.transact(adoption)
    }

    // ----- session.undo.partial -----

    /// `session.undo.partial {entry, move, token}` (ADR-D01.15; draft): steps back inside the head's journal entry
    /// `entry`, a commit or a partial undo of one, so that its first `move` literal moves stay applied. The move must be
    /// one of the entry's applied moves (`KN5:stale.undo_superseded` otherwise, as for an undo with no eligible target).
    /// It is a commit with no preview whose token binds the state and protection revisions (LI.6), and its recipe is
    /// one word, the inverses of the undone moves, last first. The commit steps run as `twist.apply`'s do: the token,
    /// the commit lane, the binding, Jumble off, the revisions, the head hash, the kernel's derivation under the
    /// protected set and the policy in force (`KN2:protection.orbit_would_move`), the durable transaction and the
    /// adoption. It journals as a retained move (FP.31) whose row and request name the entry and the move, and
    /// `session.undo` undoes it as any commit; nothing earlier is rewritten. A partial undo of every move reaches the
    /// entry's recorded start.
    fn partial_undo(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let (Some(entry), Some(keep)) = (nat(invocation.args.get("entry")), nat(invocation.args.get("move"))) else {
            return refused(code::ARGUMENT_FORM, "`entry` is an action number and `move` a natural number");
        };
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        self.retained_mode()?;
        // The target before the bound revisions, so a raced press is `undo_superseded`, as an undo's hint is.
        let Some((_, applied)) = self.head_entry().filter(|&(inside, _)| inside == entry) else {
            return refused(code::UNDO_SUPERSEDED, format!("the head is not inside entry {entry}"));
        };
        let keep = usize::try_from(keep).unwrap_or(usize::MAX);
        if keep >= applied {
            return refused(code::UNDO_SUPERSEDED, "the move is not an applied move of the entry");
        }
        if binding.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        if self.current.digest() != self.nodes[&self.head].digest {
            return refused(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state");
        }
        let recipe = self.partial_recipe(entry, applied, keep)?;
        let derived = match self.derive(&self.current, &recipe, &self.protected).map_err(kernel_error)? {
            Derivation::Derived(derived) => derived,
            Derivation::Refused(refusal) => return Err(refusal.refusal.into()),
        };
        if keep == 0 && derived.state.digest() != self.nodes[&self.nodes[&entry].parent].digest {
            return refused(code::STATE_HASH_MISMATCH, "the partial undo does not reach the entry's recorded start");
        }
        let meta = Meta::of(caller, invocation, request);
        let adoption = self.commit_adoption(meta, recipe, derived.state, derived.word, None);
        self.transact(adoption)
    }

    // ----- session.undo and session.redo -----

    /// The recipe from the head to node `to`: the inverses of the nodes up to their common ancestor, then the nodes
    /// down to `to` (M6's move tree).
    fn path(&self, to: u64) -> Result<Recipe, Error> {
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
        // Every node but the start is a commit, a crossing or a pose move, and only a commit holds a retained recipe.
        let recipe = |node: &u64| {
            let detail = "the path crosses a node that holds no retained recipe";
            self.nodes[node].recipe.as_ref().ok_or_else(|| Error::new(code::STATE_HASH_MISMATCH, detail))
        };
        let mut steps = Vec::new();
        for node in &up {
            steps.extend(steps_json(recipe(node)?, true));
        }
        for node in down.iter().rev() {
            steps.extend(steps_json(recipe(node)?, false));
        }
        Ok(self.kernel.recipe(&Value::Array(steps))?)
    }

    /// The state at node `to`, derived from the current state along the path under `protected` in the policy in force
    /// (LI.26's normal permission path); it must reach the node's recorded digest. Returns the state and the path's
    /// literal digest.
    fn walk(&self, to: u64, protected: &ProtectedSet) -> Result<(Snapshot, String), Error> {
        if self.state_digest() != self.nodes[&self.head].digest {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state"));
        }
        if to == self.head {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the head is the action's target already"));
        }
        let recipe = self.path(to)?;
        let derived = match self.derive(&self.current, &recipe, protected).map_err(kernel_error)? {
            Derivation::Derived(derived) => derived,
            Derivation::Refused(refusal) => return Err(refusal.refusal.into()),
        };
        if derived.state.digest() != self.nodes[&to].digest {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the path does not reach the action's recorded state"));
        }
        Ok((derived.state, derived.word))
    }

    /// Whether going from the protected set in force to `to` releases an orbit.
    fn releases(&self, to: &[usize]) -> bool {
        self.protected.orbits().iter().any(|orbit| !to.contains(orbit))
    }

    /// An undo or redo of the action on top of its stack, through the original permission path: a head change derives
    /// its path under `protected`, and a protection change that releases an orbit needs `confirm` (LI.26). A pose
    /// move's undo applies the exact inverse of its twist and its redo the twist again; an exit's undo returns to the
    /// poses before it and its redo checks A5's label checkpoint again (JI.27; draft).
    ///
    /// A step of the layout history ([`History::Layout`]) inverts or repeats a workspace completion's records, as a
    /// workspace inverse; it is journaled as an undo or redo whose row is `layout.undo` or `layout.redo` (ADR-D01.14).
    fn plan_step(
        &self,
        meta: Meta,
        history: History,
        direction: Direction,
        protected: &ProtectedSet,
    ) -> Result<Adoption, Error> {
        let (kind, tree) = match (history, direction) {
            (History::Moves, Direction::Undo) => (TransactionKind::Undo, Tree::Undo),
            (History::Moves, Direction::Redo) => (TransactionKind::Redo, Tree::Redo),
            (History::Layout, Direction::Undo) => (TransactionKind::Undo, Tree::LayoutUndo),
            (History::Layout, Direction::Redo) => (TransactionKind::Redo, Tree::LayoutRedo),
        };
        let Some(&target) = self.stack(history, direction).last() else {
            return Err(Error::new(code::UNDO_SUPERSEDED, "no eligible target"));
        };
        let change = self.changes.get(&target).expect("a stacked action keeps its change").clone();
        let order = self.order + 1;
        let digest = self.state_digest().to_owned();
        let mut transaction = Transaction {
            kind,
            event: format!("e{order}"),
            order,
            target: Some(target),
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal: String::new(),
            pre: digest.clone(),
            post: digest,
            revision: self.revision.clone(),
            recipe: Value::Null,
            orbits: self.protected.orbits().to_vec(),
            confirm: meta.confirm,
            job: None,
            result: None,
            record: None,
            policy: None,
            records: Vec::new(),
            rotation: None,
            crossing: None,
            blocks: Vec::new(),
            budget: None,
        };
        let side = Side::Stay;
        match change {
            Change::Head { before, after } => {
                let to = if direction == Direction::Undo { before } else { after };
                let (state, literal) = self.walk(to, protected)?;
                transaction.literal = literal;
                transaction.post = state.digest().to_owned();
                transaction.revision.head = format!("e{to}");
                transaction.revision.state += 1;
                transaction.revision.digest = state.digest().to_owned();
                Ok(Adoption { transaction, state: Some(state), protected: None, head: to, tree, preview: None, side })
            }
            Change::Protection { before, after } => {
                let to = if direction == Direction::Undo { before } else { after };
                if !meta.confirm && self.releases(&to) {
                    return Err(Error::new(
                        code::CONFIRMATION_REQUIRED,
                        "releasing a protected orbit needs confirmation",
                    ));
                }
                let orbits: Vec<i64> = to.iter().map(|&orbit| orbit as i64).collect();
                let set = self.kernel.protected_set(&orbits)?;
                transaction.orbits = to;
                transaction.revision.workspace += 1;
                transaction.revision.protection += 1;
                let protected = Some(set);
                Ok(Adoption { transaction, state: None, protected, head: self.head, tree, preview: None, side })
            }
            // A workspace inverse: the record takes its value before or after the write again, and its version and the
            // workspace revision advance (KN5.20, LI.26).
            Change::Record { name, before, after } => {
                let to = if direction == Direction::Undo { before } else { after };
                transaction.record = Some(RecordWrite { name, value: to });
                transaction.revision.workspace += 1;
                Ok(Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None, side })
            }
            // A workspace completion's inverse: each of its records takes its value before or after the completion
            // again, as one action, and their versions and the workspace revision advance (KN5.20, LI.26; draft).
            Change::Records { before, after } => {
                transaction.records = if direction == Direction::Undo { before } else { after };
                transaction.revision.workspace += 1;
                Ok(Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None, side })
            }
            // A workspace inverse: the policy before or after the change is in force again, and the workspace and
            // protection revisions advance, as for a protection change. Finding LD1's reading asks the change itself
            // for a confirmation, not its undo or redo.
            Change::Policy { before, after } => {
                let to = if direction == Direction::Undo { before } else { after };
                transaction.policy = Some(to);
                transaction.revision.workspace += 1;
                transaction.revision.protection += 1;
                Ok(Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None, side })
            }
            // An undo of a crossing returns to the exact Jumble off state before the entry (JI.29); a redo makes the
            // entry again from the same witness, under a pinned menu with the identity block the entry was made under,
            // and must reach the crossing's recorded poses and record (ONE.81, JI.27, FP.31).
            Change::Crossing { before, after, identity, record } => {
                let (from, to) = if direction == Direction::Undo { (after, before) } else { (before, after) };
                if self.head != from || self.state_digest() != self.nodes[&from].digest {
                    let detail = "the head hash differs from the head's recorded state";
                    return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
                }
                let (mode, digest, side) = match direction {
                    Direction::Undo => {
                        // The retained state does not move while Jumble is on.
                        if self.current.digest() != self.nodes[&to].digest {
                            let detail = "the retained state is not the one the crossing entered from";
                            return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
                        }
                        (MODE, self.current.digest().to_owned(), Side::Off)
                    }
                    Direction::Redo => {
                        let Some(twists) = &self.twists else {
                            let detail = format!("`{JUMBLE_ON_ROW}` needs the host's pinned twist menu");
                            return Err(Error::new(code::ROW_NOT_BUILT, detail));
                        };
                        if twists.context.identity(Some(&*twists.menu)) != identity {
                            let detail = "the pinned twist menu is not the one the crossing was made under";
                            return Err(Error::new(c600_kernel::codes::IDENTITY_MISMATCH, detail));
                        }
                        let (entry, depth) = self.entry_at_head(twists)?;
                        if entry.digest != self.nodes[&to].digest || entry.record != record {
                            let detail = "the witness does not reach the crossing's recorded poses and record";
                            return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
                        }
                        let digest = entry.digest.clone();
                        let jumbled = Jumbled { derived: entry.derived, digest: entry.digest, depth };
                        (JUMBLE_ON_MODE, digest, Side::On(jumbled))
                    }
                };
                transaction.post = digest.clone();
                transaction.revision.mode = mode;
                transaction.revision.head = format!("e{to}");
                transaction.revision.state += 1;
                transaction.revision.digest = digest;
                Ok(Adoption { transaction, state: None, protected: None, head: to, tree, preview: None, side })
            }
            // A pose move (JI.27; draft): an undo applies the exact inverse of the move's twist, a redo the twist
            // again, from the head's value ([`CommandLayer::pose_value`]); each must reach its target's recorded poses.
            // The literal is the step's.
            Change::Pose { before, after } => {
                let (from, to) = if direction == Direction::Undo { (after, before) } else { (before, after) };
                if self.jumbled.is_none() || self.head != from || self.state_digest() != self.nodes[&from].digest {
                    let detail = "the head hash differs from the head's recorded state";
                    return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
                }
                let Some(twists) = &self.twists else {
                    let detail = format!("`{JUMBLE_TWIST_ROW}` needs the host's pinned twist menu");
                    return Err(Error::new(code::ROW_NOT_BUILT, detail));
                };
                let twist = self.nodes[&after].twist.clone().expect("a pose move's node holds its twist");
                let step = match direction {
                    Direction::Undo => {
                        let inverse = Twist::new(&twists.context, twist.grip, transpose(&twist.matrix));
                        let detail = |error| format!("the move's exact inverse: {error}");
                        inverse.map_err(|error| Error::new(code::STATE_HASH_MISMATCH, detail(error)))?
                    }
                    Direction::Redo => twist,
                };
                let held = twists.menu.contains(&twists.context, &step).then_some(&step);
                let jumbled = self.pose_value(held, || self.jumble_journal(to))?;
                if jumbled.digest != self.nodes[&to].digest {
                    let detail = "the step does not reach the move's recorded poses";
                    return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
                }
                transaction.literal = levels::word_digest(std::slice::from_ref(&step));
                transaction.post = jumbled.digest.clone();
                transaction.revision.head = format!("e{to}");
                transaction.revision.state += 1;
                transaction.revision.digest = jumbled.digest.clone();
                let side = Side::On(jumbled);
                Ok(Adoption { transaction, state: None, protected: None, head: to, tree, preview: None, side })
            }
            // An undo of an exit returns to the exact poses before it, Jumble on, with the retained state the entry
            // left. A redo checks A5's label checkpoint again from the head's value, which must reach the exit's
            // recorded labels and record (JI.27; draft).
            Change::Exit { before, after, point } => {
                let (from, to) = if direction == Direction::Undo { (after, before) } else { (before, after) };
                if self.head != from || self.state_digest() != self.nodes[&from].digest {
                    let detail = "the head hash differs from the head's recorded state";
                    return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
                }
                let ExitPoint { poses, retained, record } = *point;
                let (mode, state, side) = match direction {
                    Direction::Undo => {
                        if poses.digest != self.nodes[&to].digest {
                            let detail = "the exit's poses are not the recorded poses before it";
                            return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
                        }
                        (JUMBLE_ON_MODE, retained, Side::On(poses))
                    }
                    Direction::Redo => {
                        let exit = self.exit_at_head()?;
                        if exit.state.digest() != self.nodes[&to].digest || exit.record != record {
                            let detail = "the checkpoint does not reach the exit's recorded labels and record";
                            return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
                        }
                        (MODE, exit.state, Side::Off)
                    }
                };
                let digest = self.nodes[&to].digest.clone();
                transaction.post = digest.clone();
                transaction.revision.mode = mode;
                transaction.revision.head = format!("e{to}");
                transaction.revision.state += 1;
                transaction.revision.digest = digest;
                let state = Some(state);
                Ok(Adoption { transaction, state, protected: None, head: to, tree, preview: None, side })
            }
        }
    }

    fn undo(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
        history: History,
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
        // The hint before the bound revisions, so a raced undo is `undo_superseded` (draft ruling 2).
        if self.hint(history, direction) != Some(hint) {
            return refused(code::UNDO_SUPERSEDED, "the hint's order or target changed");
        }
        if binding.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        let protected = self.protected.clone();
        let adoption = self.plan_step(Meta::of(caller, invocation, request), history, direction, &protected)?;
        self.transact(adoption)
    }

    // ----- session.rollback -----

    /// `session.rollback {hint, token}` (ADR-D01.15; draft; the integrator's ruling of 22 November 2026): one solver
    /// action that rolls the move history back or forward to the state just after action `hint.target` (0: the
    /// session's start; [`CommandLayer::rollback_hint`]). It is k ordinary undo or redo steps of the move history, each
    /// its own transaction with its own admission and receipt under command `{gesture}#{i}/{k}`, where the gesture is
    /// the press's command ID, and with the gesture and step i of k in its arguments; so no journal kind or format
    /// changes, and every span has one encoding with no branch transition. The layer holds the writer for the whole
    /// gesture and stops at the first step that does not commit, reporting j of k ([`CommandLayer::rollback`]). A crash
    /// leaves a complete prefix of steps, which a retry finds by LI.30's lookup of the step command IDs
    /// ([`CommandLayer::rollback_steps`]); there is no all-or-nothing across a crash. Since each step ID carries k,
    /// "j of k" is durable: a reopen tells a cut gesture from a whole one by the step IDs alone (R2-5, owner, 23
    /// November 2026, decision 4.2). A gesture ID names one gesture: a press under an ID whose gesture has steps is
    /// refused with `command_id_reused`, whatever its k.
    ///
    /// The token binds the hint and the state and protection revisions as `session.undo`'s does; a raced hint is
    /// `undo_superseded` before the revisions are checked. Each step takes `session.undo`'s permission path under the
    /// protected set in force when it runs, and the press's `confirm` confirms every release in the span (LI.26). The
    /// press answers with the last landed step's receipt, a refusal when no step landed, or `unknown_outcome` when a
    /// step's fate is unknown.
    fn rollback_press(&mut self, caller: &Caller, invocation: &Invocation) -> Result<(Option<u64>, Status), Error> {
        // A press ends the report of the gesture before it, refused or not.
        if self.rollback.take().is_some() {
            self.generation += 1;
        }
        let Some(hint) = invocation.args.get("hint").and_then(ActionHint::from_json) else {
            return refused(code::ARGUMENT_FORM, "`hint` is an ActionHint");
        };
        if ["gesture", "step", "of"].iter().any(|name| invocation.args.get(name).is_some()) {
            return refused(code::ARGUMENT_FORM, "only the layer names a rollback's gesture and steps");
        }
        let gesture = invocation.command.as_str();
        if rollback_step(&format!("{gesture}#1/1")) != Some((gesture, 1, 1)) {
            return refused(code::ARGUMENT_FORM, "a rollback's command ID must name its gesture's steps");
        }
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        if binding.hint != Some(hint) {
            return refused(code::TOKEN_MISMATCH, "a rollback token binds its hint");
        }
        let Some((direction, of)) = self.span(hint.target).filter(|_| hint.order == self.order) else {
            return refused(code::UNDO_SUPERSEDED, "the hint's order or target changed");
        };
        if binding.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        let scope = caller.scope();
        let steps: Vec<Invocation> = (1..=of).map(|step| Self::rollback_invocation(invocation, step, of)).collect();
        let taken = |key: &Key| self.receipts.contains_key(key) || self.lost.contains_key(key);
        if steps.iter().any(|step| taken(&(scope.clone(), step.command.clone()))) {
            return refused(code::COMMAND_ID_REUSED, "a step of the gesture would reuse a command ID");
        }
        if self.gestures.contains_key(&(scope.clone(), invocation.command.clone())) {
            return refused(code::COMMAND_ID_REUSED, "the gesture ID names an earlier gesture's steps");
        }
        self.gestures.insert((scope.clone(), invocation.command.clone()), of);
        let (mut landed, mut last, mut stop) = (0, None, None);
        for step in &steps {
            let request = self.request(caller, step, &[]);
            let protected = self.protected.clone();
            let meta = Meta::of(caller, step, &request);
            let outcome = self.plan_step(meta, History::Moves, direction, &protected).and_then(|a| self.transact(a));
            let (job, status) = outcome.unwrap_or_else(|error| (None, Status::Refused(error)));
            let receipt =
                Receipt { command: step.command.clone(), request: request.clone(), job, status: status.clone() };
            let record = Record { request, receipt, durable: false };
            self.step_ids.insert((scope.clone(), step.command.clone()));
            self.receipts.entry((scope.clone(), step.command.clone())).or_insert(record);
            if !matches!(status, Status::Committed(_)) {
                stop = Some(status);
                break;
            }
            landed += 1;
            last = Some((job, status));
        }
        let gesture = invocation.command.clone();
        self.rollback = Some(Rollback { gesture, target: hint.target, direction, of, landed, stop: stop.clone() });
        self.generation += 1;
        match (stop, last) {
            (Some(Status::UnknownOutcome), _) => Ok((None, Status::UnknownOutcome)),
            (_, Some(answer)) => Ok(answer),
            (Some(Status::Refused(error)), None) => Err(error),
            (Some(status), None) => Ok((None, status)),
            (None, None) => refused(code::UNDO_SUPERSEDED, "no eligible target"),
        }
    }

    /// Step `step` of `of` of the rollback `invocation` presses: command `{gesture}#{step}/{of}` on the same row, with
    /// the press's arguments, the gesture, the step and the count in its arguments, and the press's token and epoch.
    fn rollback_invocation(invocation: &Invocation, step: u64, of: u64) -> Invocation {
        let mut args = invocation.args.as_object().cloned().unwrap_or_default();
        args.insert("gesture".to_owned(), Value::String(invocation.command.clone()));
        args.insert("step".to_owned(), Value::Int(step as i64));
        args.insert("of".to_owned(), Value::Int(of as i64));
        Invocation {
            command: format!("{}#{step}/{of}", invocation.command),
            row: ROLLBACK_ROW.to_owned(),
            args: Value::Object(args),
            token: invocation.token.clone(),
            epoch: invocation.epoch,
            budget: None,
        }
    }

    // ----- explore.set -----

    /// `explore.set {on}` (ADR-D01.39; draft): turns the explore state on or off and answers `accepted` with no job.
    /// It writes no journal action and takes no undo (ADR-D01.14). A run turns it only inside a command the solver
    /// invoked (ADR-D01.2's first route; `LG3:capability.not_granted` otherwise). While it is on, the commit rows are
    /// refused with [`code::COMMITS_PAUSED`] ([`CommandLayer::paused`]); the solver's own `explore.set` turns it off.
    /// The state may only pause commits, never enable them, so a run that turns it off while the solver's pause holds
    /// is refused with `LG3:capability.not_granted`; a run ends only a pause a run provided, and the solver's
    /// `explore.set {on: true}` makes a run's pause the solver's.
    fn set_explore(
        &mut self,
        caller: &Caller,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        forms::check_forms(row, args)?;
        let Some(&Value::Bool(on)) = args.get("on") else {
            return refused(code::ARGUMENT_FORM, "`on` is a Bool");
        };
        let by = match caller {
            Caller::Shell => Explorer::Solver,
            Caller::Runtime { run } => {
                if !self.inside_solver_command(run) {
                    let detail = format!("run `{run}` is not inside a command the solver invoked (ADR-D01.2)");
                    return refused(code::NOT_GRANTED, detail);
                }
                Explorer::Run
            }
        };
        let next = match (on, by, self.explore) {
            (true, Explorer::Solver, _) => Some(Explorer::Solver),
            (true, Explorer::Run, held) => held.or(Some(Explorer::Run)),
            (false, Explorer::Run, Some(Explorer::Solver)) => {
                let detail = "a run may not end the pause the solver set: the explore state never enables commits";
                return refused(code::NOT_GRANTED, format!("{detail} (ADR-D01.39)"));
            }
            (false, _, _) => None,
        };
        if self.explore != next {
            self.explore = next;
            self.generation += 1;
        }
        Ok((None, Status::Accepted))
    }

    // ----- protection.orbit.set -----

    /// A change of the protected set: the workspace and protection revisions advance, the state and the preview slot
    /// stay. A release needs `confirm` (LI.26).
    fn plan_orbits(&self, meta: Meta, set: ProtectedSet) -> Result<Adoption, Error> {
        let to = set.orbits().to_vec();
        if !meta.confirm && self.releases(&to) {
            return Err(Error::new(code::CONFIRMATION_REQUIRED, "releasing a protected orbit needs confirmation"));
        }
        let order = self.order + 1;
        let digest = self.state_digest().to_owned();
        let mut revision = self.revision.clone();
        revision.workspace += 1;
        revision.protection += 1;
        let transaction = Transaction {
            kind: TransactionKind::Protection,
            event: format!("e{order}"),
            order,
            target: None,
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal: String::new(),
            pre: digest.clone(),
            post: digest,
            revision,
            recipe: Value::Null,
            orbits: to.clone(),
            confirm: meta.confirm,
            job: None,
            result: None,
            record: None,
            policy: None,
            records: Vec::new(),
            rotation: None,
            crossing: None,
            blocks: Vec::new(),
            budget: None,
        };
        let change = Change::Protection { before: self.protected.orbits().to_vec(), after: to };
        let tree = Tree::Push { node: None, change };
        let (protected, side) = (Some(set), Side::Stay);
        Ok(Adoption { transaction, state: None, protected, head: self.head, tree, preview: None, side })
    }

    fn set_orbits(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let orbits = invocation.args.get("orbits").and_then(Value::as_array);
        let Some(orbits) = orbits.and_then(|orbits| orbits.iter().map(Value::as_i64).collect::<Option<Vec<_>>>())
        else {
            return refused(code::ARGUMENT_FORM, "`orbits` is a list of orbit numbers");
        };
        let set = self.kernel.protected_set(&orbits)?;
        let adoption = self.plan_orbits(Meta::of(caller, invocation, request), set)?;
        self.transact(adoption)
    }

    // ----- protection.policy.set -----

    /// A change of the protection policy (draft): as for a protection change, the workspace and protection revisions
    /// advance, so a pending preview built under the old policy is stale (H-08: "invalidates any open preview"), and
    /// the state and the preview slot stay. It is undoable as a workspace inverse. A change in either direction needs
    /// the solver's confirmed press (finding LD1, the owner's reading of 18 November 2026; [`Self::set_policy`] checks
    /// it, as H-08 gives the row no `confirm` gate), and the transaction stores the confirmation. A reopen replays a
    /// stored change without checking it: a journal from before the reading stores none.
    fn plan_policy(&self, meta: Meta, policy: ProtectionMode) -> Adoption {
        let order = self.order + 1;
        let digest = self.state_digest().to_owned();
        let mut revision = self.revision.clone();
        revision.workspace += 1;
        revision.protection += 1;
        let transaction = Transaction {
            kind: TransactionKind::Policy,
            event: format!("e{order}"),
            order,
            target: None,
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal: String::new(),
            pre: digest.clone(),
            post: digest,
            revision,
            recipe: Value::Null,
            orbits: self.protected.orbits().to_vec(),
            confirm: meta.confirm,
            job: None,
            result: None,
            record: None,
            policy: Some(policy),
            records: Vec::new(),
            rotation: None,
            crossing: None,
            blocks: Vec::new(),
            budget: None,
        };
        let tree = Tree::Push { node: None, change: Change::Policy { before: self.policy, after: policy } };
        Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None, side: Side::Stay }
    }

    fn set_policy(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let policy = invocation.args.get("policy").and_then(Value::as_str).and_then(policy_from_name);
        let Some(policy) = policy else {
            return refused(code::ARGUMENT_FORM, "`policy` is `net` or `strict`");
        };
        let meta = Meta::of(caller, invocation, request);
        if !meta.confirm {
            return refused(
                code::CONFIRMATION_REQUIRED,
                "a change of the protection policy needs the solver's confirmation",
            );
        }
        let adoption = self.plan_policy(meta, policy);
        self.transact(adoption)
    }

    /// A workspace row's job completion: one durable action that advances the workspace revision, writes the job's
    /// records and keeps the state, the preview slot and the jobs (KN5.23). A row whose undo class is `workspace`, a
    /// language-defined row or a stored one ([`Self::workspace_undoable`]), makes an undoable action, which
    /// `session.undo` and `session.redo` invert and repeat as a workspace inverse ([`Change::Records`]), when it writes
    /// a record: one that writes none changes nothing an undo could restore, so it takes no undo entry. No undo reaches
    /// the completion of a stored row whose undo class is `none` (draft).
    fn workspace_adoption(
        &self,
        meta: Meta,
        job: u64,
        result: String,
        records: Vec<RecordWrite>,
        budget: Option<Budget>,
    ) -> Adoption {
        let tree = match self.workspace_undoable(&meta.row).filter(|_| !records.is_empty()) {
            Some(history) => {
                let value = |name: &str| self.records.get(name).and_then(|record| record.value.clone());
                let before =
                    records.iter().map(|write| RecordWrite { name: write.name.clone(), value: value(&write.name) });
                let change = Change::Records { before: before.collect(), after: records.clone() };
                match history {
                    History::Moves => Tree::Push { node: None, change },
                    History::Layout => Tree::LayoutPush(change),
                }
            }
            None => Tree::None,
        };
        let order = self.order + 1;
        let digest = self.state_digest().to_owned();
        let mut revision = self.revision.clone();
        revision.workspace += 1;
        let transaction = Transaction {
            kind: TransactionKind::Workspace,
            event: format!("e{order}"),
            order,
            target: None,
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal: String::new(),
            pre: digest.clone(),
            post: digest,
            revision,
            recipe: Value::Null,
            orbits: self.protected.orbits().to_vec(),
            confirm: false,
            job: Some(job),
            result: Some(result),
            record: None,
            policy: None,
            records,
            rotation: None,
            crossing: None,
            blocks: Vec::new(),
            budget,
        };
        Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None, side: Side::Stay }
    }

    /// The history a completion of workspace row `row` is undone in: the move history when its row's undo class is
    /// `workspace`, the layout history when it is `layout` (ADR-D01.14), none otherwise. Every language-defined
    /// workspace row's class is `workspace` ([`Table::register_overlay`]; LG3.5, LB7.2, LP2's reading 18), and a stored
    /// row's is H-08's. A replay runs before the host registers its language-defined rows again, so a row the table
    /// does not hold is one of them, and the replay reads the same answer as the completion did.
    fn workspace_undoable(&self, row: &str) -> Option<History> {
        match self.table.row(row) {
            None => Some(History::Moves),
            Some(row) if row.kind == Kind::Workspace && row.undo == UndoClass::Workspace => Some(History::Moves),
            Some(row) if row.kind == Kind::Workspace && row.undo == UndoClass::Layout => Some(History::Layout),
            _ => None,
        }
    }

    // ----- workspace.set -----

    /// `workspace.set` (M6's record write): writes `payload` with its provenance block under `record` as one durable
    /// action whose receipt is `completed(workspace)`. The host's provenance reader checks the block first (PS1.16); a
    /// block it refuses, a missing reader or a bad argument writes nothing. A name under `set/`, `grip/` or `macro/`
    /// is a saved set's, the applied grip map's or the macro base's ([`crate::sets`], [`crate::grip`],
    /// [`crate::macros`]) and is refused with `KN5:input.argument_form`.
    fn write_record(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let args = &invocation.args;
        let text = |key: &str| args.get(key).and_then(Value::as_str);
        let (Some(name), Some(payload), Some(provenance)) = (text("record"), args.get("payload"), text("provenance"))
        else {
            return refused(code::ARGUMENT_FORM, "`record` and `provenance` are text");
        };
        reserved_name(name)?;
        let Some(reader) = &self.provenance else {
            return refused(code::ROW_NOT_BUILT, format!("`{RECORD_ROW}` needs the host's provenance reader"));
        };
        reader.check(provenance.as_bytes())?;
        let value = RecordValue { payload: payload.clone(), provenance: provenance.to_owned() };
        let adoption = self.plan_record(Meta::of(caller, invocation, request), name.to_owned(), value);
        self.transact(adoption)
    }

    /// A record write: the record's version and the workspace revision advance, and the state, the head, the preview
    /// slot and the jobs stay (KN5.18). It is undoable as a workspace inverse.
    fn plan_record(&self, meta: Meta, name: String, value: RecordValue) -> Adoption {
        let order = self.order + 1;
        let digest = self.state_digest().to_owned();
        let mut revision = self.revision.clone();
        revision.workspace += 1;
        let before = self.records.get(&name).and_then(|record| record.value.clone());
        let transaction = Transaction {
            kind: TransactionKind::Record,
            event: format!("e{order}"),
            order,
            target: None,
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal: String::new(),
            pre: digest.clone(),
            post: digest,
            revision,
            recipe: Value::Null,
            orbits: self.protected.orbits().to_vec(),
            confirm: meta.confirm,
            job: None,
            result: None,
            record: Some(RecordWrite { name: name.clone(), value: Some(value.clone()) }),
            policy: None,
            records: Vec::new(),
            rotation: None,
            crossing: None,
            blocks: Vec::new(),
            budget: None,
        };
        let tree = Tree::Push { node: None, change: Change::Record { name, before, after: Some(value) } };
        Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None, side: Side::Stay }
    }

    // ----- grip.rebind -----

    /// `grip.rebind` (EV4.8; [`crate::grip`]): replaces the applied grip-slot map with the transported map `map` from
    /// `anchor`, as one record write of `grip/map` whose receipt is `completed(workspace)`, undoable as a workspace
    /// inverse. Refused, writing nothing: a map or anchor of another form (`KN5:input.argument_form`), and an
    /// `expected_map` that is not the digest of the applied map ([`CommandLayer::slot_map_digest`];
    /// `KN5:stale.revision_changed`).
    fn rebind_grip(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let args = &invocation.args;
        let model = self.kernel.identity().model_id;
        let map = SlotMap::from_args(args.get("anchor"), args.get("map"), &model)?;
        if args.get("expected_map").and_then(Value::as_str) != Some(self.slot_map_digest().as_str()) {
            return refused(code::REVISION_CHANGED, "`expected_map` is not the digest of the applied grip map");
        }
        let value = RecordValue { payload: map.to_payload(), provenance: SLOT_MAP_PROVENANCE.to_owned() };
        let meta = Meta::of(caller, invocation, request);
        let adoption = self.plan_record(meta, SLOT_MAP_RECORD.to_owned(), value);
        self.transact(adoption)
    }

    /// The applied grip-slot map the solver's last rebind wrote, or `None` while the host's default map applies
    /// (EV4.8).
    pub fn slot_map(&self) -> Option<SlotMap> {
        self.record(SLOT_MAP_RECORD).and_then(|value| SlotMap::from_payload(&value.payload))
    }

    /// The digest `grip.rebind`'s `expected_map` must name: the applied map's ([`crate::grip::slot_map_digest`]).
    pub fn slot_map_digest(&self) -> String {
        slot_map_digest(self.record(SLOT_MAP_RECORD).map(|value| &value.payload))
    }

    // ----- macro.save -----

    /// `macro.save` (EV4.9; [`crate::macros`]): saves `word` under `name` in the macro base as one record write of
    /// `macro/<name>` whose receipt is `completed(workspace)`, undoable as a workspace inverse. A save under a name
    /// that holds a macro writes its next version. It neither selects, inserts nor executes the word, and it is
    /// allowed in either Jumble mode, since one base serves both puzzles. Refused, writing nothing, in this order:
    /// without `confirm: true` (H-08's `confirm` gate; `KN5:input.confirmation_required`), a name that is not text or
    /// is empty and a word of another form (`KN5:input.argument_form`), a recipe the kernel refuses (its error), and a
    /// menu word without a pinned twist menu (`KN5:input.row_not_built`).
    ///
    /// A stored word, `word: {"record": r}` (BND.32; draft): the record's letters, or a work sheet's complete
    /// operation ([`RecordValue::operation`]), saved as a Full word whose record keeps the stored word's provenance
    /// block, which the host's reader checks as `worksheet.save` checks a sheet's (PS1.16), and whose source pins the
    /// record's content digest ([`crate::macros::stored_source`]). Refused, writing nothing, also for: no reader
    /// (`KN5:input.row_not_built`), a record that is missing or holds no word (`KN2:input.named_object_invalid`), a
    /// block the reader refuses (its error) and letters the kernel refuses (its error).
    fn save_macro(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let meta = Meta::of(caller, invocation, request);
        if !meta.confirm {
            return refused(code::CONFIRMATION_REQUIRED, "`macro.save` saves only with `confirm: true`");
        }
        let args = &invocation.args;
        let Some(name) = args.get("name").and_then(Value::as_str).filter(|name| !name.is_empty()) else {
            return refused(code::ARGUMENT_FORM, "a macro's name is a nonempty text");
        };
        let word = args.get("word").expect("the names check saw the word");
        let only = |key: &str| word.get(key).filter(|_| word.as_object().is_some_and(|o| o.len() == 1));
        let mut provenance = MACRO_PROVENANCE.to_owned();
        let mut source = word.clone();
        let (alphabet, identity, letters) = if let Some(recipe) = only("recipe") {
            let recipe = self.kernel.recipe(recipe)?;
            let letters = self.kernel.letters(&recipe)?.into_iter().map(i64::from).collect();
            (Alphabet::Full, self.kernel.identity(), letters)
        } else if let Some(letters) = only("menu") {
            let Some(twists) = &self.twists else {
                return refused(code::ROW_NOT_BUILT, "a menu word needs the host's pinned twist menu");
            };
            let codes = menu_codes(letters, twists.menu.items.len())?;
            (Alphabet::Menu, twists.context.identity(Some(&*twists.menu)), codes)
        } else if let Some(stored) = only("record").and_then(Value::as_str) {
            // BND.32: a macro of a stored word keeps the word's block.
            let Some(reader) = &self.provenance else {
                return refused(code::ROW_NOT_BUILT, "a macro of a stored word needs the host's provenance reader");
            };
            let Some(value) = self.records.get(stored).and_then(|record| record.value.as_ref()) else {
                return refused(code::NAMED_OBJECT_INVALID, "no such record");
            };
            let Some(letters) = value.operation() else {
                let detail = "the record holds neither a list of letters nor a work sheet";
                return refused(code::NAMED_OBJECT_INVALID, detail);
            };
            reader.check(value.provenance.as_bytes())?;
            self.kernel.word(&letters)?;
            provenance = value.provenance.clone();
            source = crate::macros::stored_source(stored, &value.digest());
            (Alphabet::Full, self.kernel.identity(), letters)
        } else {
            let detail = "a macro's word is `{\"recipe\": [...]}`, `{\"menu\": [...]}` or `{\"record\": name}`";
            return refused(code::ARGUMENT_FORM, detail);
        };
        let saved = Macro {
            name: name.to_owned(),
            alphabet,
            identity,
            length: letters.len() as u64,
            literal: crate::macros::literal_digest(alphabet, &letters),
            letters,
            start: self.state_digest().to_owned(),
            source,
            keys: alphabet.contracts(),
        };
        let value = RecordValue { payload: saved.to_payload(), provenance };
        let adoption = self.plan_record(meta, macro_record(name), value);
        self.transact(adoption)
    }

    /// The macro base (EV4.9): every macro the records hold, in name order, for both puzzles.
    pub fn macro_base(&self) -> Vec<Macro> {
        let records = self.records.iter().filter(|(name, _)| name.starts_with(MACRO_PREFIX));
        records.filter_map(|(_, record)| record.value.as_ref().and_then(|v| Macro::from_payload(&v.payload))).collect()
    }

    // ----- worksheet.save and worksheet.load -----

    /// `worksheet.save` (LB7.2; draft): saves the operation that the draft record `{"record": r}` holds as a work sheet
    /// under `name`. A record of letters becomes the Macro phase alone; a work sheet keeps its phases, roles and
    /// reference. The sheet keeps the draft record's provenance block, which the host's reader checks again (PS1.16).
    /// Its piece roles are placed at the head ([`CommandLayer::place`]), and it is written as one record write,
    /// undoable as a workspace inverse (LB7.2). Refused, writing nothing, in this order: no reader
    /// (`KN5:input.row_not_built`), Jumble on (`KN5:input.retained_tool_needs_jumble_off`), a draft of another form or
    /// a name that is not text, is empty or is under `set/` (`KN5:input.argument_form`; [`crate::sets`]), a draft
    /// record that is missing or holds neither letters nor a work sheet (`KN2:input.named_object_invalid`), and a block
    /// the reader refuses (its error).
    fn save_worksheet(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let Some(reader) = &self.provenance else {
            return refused(code::ROW_NOT_BUILT, format!("`{WORKSHEET_SAVE_ROW}` needs the host's provenance reader"));
        };
        self.retained_mode()?;
        let args = &invocation.args;
        let draft = args.get("draft").and_then(Value::as_object).filter(|draft| draft.len() == 1);
        let source = draft.and_then(|draft| draft.get("record")).and_then(Value::as_str);
        let name = args.get("name").and_then(Value::as_str).filter(|name| !name.is_empty());
        let (Some(source), Some(name)) = (source, name) else {
            return refused(code::ARGUMENT_FORM, "a work sheet is saved from `{\"record\": name}` under a name");
        };
        reserved_name(name)?;
        let (sheet, value) = self.drafted(source)?;
        reader.check(value.provenance.as_bytes())?;
        let provenance = value.provenance.clone();
        let sheet = self.place(sheet, None)?;
        let value = RecordValue { payload: sheet.to_payload(), provenance };
        let adoption = self.plan_record(Meta::of(caller, invocation, request), name.to_owned(), value);
        self.transact(adoption)
    }

    /// `worksheet.load` (LB7.2, solving workflow section 4; draft): reuses the saved work sheet that `worksheet` names,
    /// `{"record": name, "version": n}` ([`WorksheetRef`], H-08's `WorksheetRevisionRef` in its draft form). Its piece
    /// roles are placed again and its position roles' occupants read again at the head, each keeping what it was at
    /// the save, so a piece that moved ([`Placement::moved`]) and a position whose occupant changed
    /// ([`Occupancy::changed`]) are shown, and the phases and roles are never retargeted (section 4.1; U1-057,
    /// U4-037). The sheet is written again as one
    /// record write, undoable as a workspace inverse. The write advances the record's version, so a preview that read
    /// the sheet goes stale (KN0.8; CT-LB2-03) and the reuse needs a fresh engine preview, with its fresh exact
    /// analysis (LB7.2). Refused, writing nothing, in this order: no reader (`KN5:input.row_not_built`), Jumble on
    /// (`KN5:input.retained_tool_needs_jumble_off`), a reference of another form (`KN5:input.argument_form`), a record
    /// that is missing or holds no value (`KN2:input.named_object_invalid`), a record at another version
    /// (`KN5:stale.revision_changed`), a record that holds no saved work sheet (`KN2:input.named_object_invalid`), and
    /// a block the reader refuses (its error).
    fn load_worksheet(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let Some(reader) = &self.provenance else {
            return refused(code::ROW_NOT_BUILT, format!("`{WORKSHEET_LOAD_ROW}` needs the host's provenance reader"));
        };
        self.retained_mode()?;
        let Some(named) = invocation.args.get("worksheet").and_then(WorksheetRef::from_json) else {
            return refused(code::ARGUMENT_FORM, "a work sheet is named `{\"record\": name, \"version\": n}`");
        };
        let (sheet, saved, value) = self.saved_sheet(&named)?;
        reader.check(value.provenance.as_bytes())?;
        let provenance = value.provenance.clone();
        let sheet = self.place(sheet, Some(&saved))?;
        let value = RecordValue { payload: sheet.to_payload(), provenance };
        let adoption = self.plan_record(Meta::of(caller, invocation, request), named.record, value);
        self.transact(adoption)
    }

    /// `sheet` with its piece roles placed at the head, where KN2.4's `positions` row finds each piece now, and its
    /// position roles' occupants read there ([`CommandLayer::occupants`]). A save (`saved` is `None`) places and reads
    /// them for the first time; a reuse keeps where each piece was, what each position held and the digest of the
    /// state they were saved at. Position roles stay on their positions.
    fn place(&self, sheet: Worksheet, saved: Option<&Resolution>) -> Result<Worksheet, Error> {
        let roles = sheet.pieces();
        let pieces: Vec<i64> = roles.iter().map(|(_, piece)| *piece).collect();
        let found = self.positions(&pieces)?;
        let digest = self.state_digest().to_owned();
        let placements = roles.into_iter().zip(found).enumerate().map(|(i, ((name, piece), found))| {
            let now = found.position.map(|position| position as i64);
            let saved = saved.map_or(now, |resolution| resolution.pieces[i].saved);
            Placement { name, piece, saved, now }
        });
        let held = sheet.positions();
        let at: Vec<i64> = held.iter().map(|(_, position)| *position).collect();
        let occupants = self.occupants(&at)?;
        let occupancies = held.into_iter().zip(occupants).enumerate().map(|(i, ((name, position), now))| {
            let saved = saved.map_or(now, |resolution| resolution.positions[i].saved);
            Occupancy { name, position, saved, now }
        });
        let resolution = Resolution {
            saved: saved.map_or_else(|| digest.clone(), |resolution| resolution.saved.clone()),
            now: digest,
            pieces: placements.collect(),
            positions: occupancies.collect(),
        };
        Ok(Worksheet { resolution: Some(resolution), ..sheet })
    }

    /// The sheet draft record `name` saves or compares as ([`Worksheet::from_draft`]), and the record's value. A record
    /// of letters names itself, at its version and content digest, as the sheet's macro version (solving workflow
    /// section 4.1; draft). Refused with `KN2:input.named_object_invalid` when the record is missing or holds neither
    /// letters nor a work sheet.
    fn drafted(&self, name: &str) -> Result<(Worksheet, &RecordValue), Error> {
        let record = self.records.get(name);
        let Some((version, value)) = record.and_then(|record| Some((record.version, record.value.as_ref()?))) else {
            return Err(Error::new(code::NAMED_OBJECT_INVALID, "no such record"));
        };
        let Some(mut sheet) = Worksheet::from_draft(&value.payload) else {
            let detail = "the record holds neither a list of letters nor a work sheet";
            return Err(Error::new(code::NAMED_OBJECT_INVALID, detail));
        };
        if value.letters().is_some() {
            sheet.macro_version = Some(MacroVersion { record: name.to_owned(), version, digest: value.digest() });
        }
        Ok((sheet, value))
    }

    /// The saved work sheet that `named` names, its placements and its record's value. Refused in this order: a record
    /// that is missing or holds no value (`KN2:input.named_object_invalid`), a record at another version
    /// (`KN5:stale.revision_changed`) and a record that holds no saved work sheet (`KN2:input.named_object_invalid`).
    fn saved_sheet(&self, named: &WorksheetRef) -> Result<(Worksheet, Resolution, &RecordValue), Error> {
        let record = self.records.get(&named.record);
        let Some((version, value)) = record.and_then(|record| Some((record.version, record.value.as_ref()?))) else {
            return Err(Error::new(code::NAMED_OBJECT_INVALID, "no such record"));
        };
        if version != named.version {
            return Err(Error::new(code::REVISION_CHANGED, "the work sheet changed since it was named"));
        }
        let saved = Worksheet::from_payload(&value.payload).and_then(|sheet| Some((sheet.resolution.clone()?, sheet)));
        let Some((saved, sheet)) = saved else {
            return Err(Error::new(code::NAMED_OBJECT_INVALID, "the record holds no saved work sheet"));
        };
        Ok((sheet, saved, value))
    }

    // ----- worksheet.compare -----

    /// The comparison that completed `worksheet.compare` job `job` made, while it is the newest one and no action has
    /// moved the state since (draft): the value its completion's result reference digests. A reopen keeps none.
    pub fn comparison(&self, job: u64) -> Option<&Value> {
        self.comparison.as_ref().filter(|(id, _)| *id == job).map(|(_, value)| value)
    }

    /// `worksheet.compare {worksheet, draft}` (LB7.2, LB2's work sheets; U2-065, U5-055, U6-040; draft): compares the
    /// saved work sheet that `worksheet` names, `{"record": name, "version": n}`, with the current work, a draft
    /// `{"record": name}` that holds letters or a sheet, and analyses the sheet's complete operation afresh. Admission
    /// queues a job on the exclusive lane; [`CommandLayer::run_jobs`] runs it at the head
    /// ([`CommandLayer::compare_now`]). Refused, queuing nothing, in this order: no reader (`KN5:input.row_not_built`),
    /// Jumble on (`KN5:input.retained_tool_needs_jumble_off`), arguments of another form (`KN5:input.argument_form`),
    /// the sheet's refusals ([`CommandLayer::saved_sheet`]), the draft's (`KN2:input.named_object_invalid`), the
    /// reader's refusal of the sheet's block in this session (KN5.26, in a solving session only, as at
    /// `operation.preview`: the analysis shows what a preview of its word would) and a held exclusive lane
    /// (`KN7:busy.lane_full`). The runtime may not invoke it.
    fn admit_compare(&mut self, caller: &Caller, invocation: &Invocation) -> Result<(Option<u64>, Status), Error> {
        let Some(reader) = &self.provenance else {
            return refused(
                code::ROW_NOT_BUILT,
                format!("`{WORKSHEET_COMPARE_ROW}` needs the host's provenance reader"),
            );
        };
        self.retained_mode()?;
        let args = &invocation.args;
        let named = args.get("worksheet").and_then(WorksheetRef::from_json);
        let draft = args.get("draft").and_then(Value::as_object).filter(|draft| draft.len() == 1);
        let draft = draft.and_then(|draft| draft.get("record")).and_then(Value::as_str);
        let (Some(named), Some(draft)) = (named, draft) else {
            let detail =
                "a work sheet `{\"record\": name, \"version\": n}` is compared with a draft `{\"record\": name}`";
            return refused(code::ARGUMENT_FORM, detail);
        };
        let (_, _, value) = self.saved_sheet(&named)?;
        self.drafted(draft)?;
        if self.profile == Profile::Solving {
            reader.admit_kept(value.provenance.as_bytes(), &self.blocks)?;
        }
        self.exclusive_free()?;
        let id = self.next_job();
        let kind = JobKind::Compare { sheet: named, draft: draft.to_owned() };
        let (command, scope) = (invocation.command.clone(), caller.scope());
        self.jobs.push_back(Job { id, command, scope, lane: Lane::Exclusive, kind });
        self.generation += 1;
        Ok((Some(id), Status::Running(id)))
    }

    /// Runs `worksheet.compare` job `id` and settles its receipt: `completed(query)` with the comparison's digest, kept
    /// as the newest comparison ([`CommandLayer::comparison`]), or failed with [`CommandLayer::compare_now`]'s error.
    fn build_comparison(&mut self, id: u64, key: &Key, sheet: &WorksheetRef, draft: &str) {
        let status = match self.compare_now(sheet, draft) {
            Ok(value) => {
                let result = ResultRef { job: Some(id), digest: digest(&value) };
                self.comparison = Some((id, value));
                let revision = self.revision.clone();
                Status::Completed(Completion { kind: CompletionKind::Query, revision, result: Some(result) })
            }
            Err(error) => Status::Failed(error),
        };
        self.settle(key, status);
        self.generation += 1;
    }

    /// The comparison at the head (draft). The sheet and the draft are read again, so a write of the sheet since
    /// admission fails the job with `KN5:stale.revision_changed`, and Jumble on with its refusal. The value is
    /// `{"worksheet", "draft", "inputs", "resolution", "moved", "occupant_changed", "macro_version", "analysis"}`:
    /// - `worksheet` names the sheet as the job read it, `draft` the draft record with its version and content digest;
    /// - `inputs` compares the sheet's phases, roles, reference and macro version with the draft's ([`compare_inputs`]);
    /// - `resolution` places the sheet's piece roles and reads its position roles' occupants again at the head, as
    ///   `worksheet.load` would; `moved` names the piece roles whose piece is not where it was saved, and
    ///   `occupant_changed` the position roles whose position holds another occupant than at the save: shown, never
    ///   retargeted (section 4.1);
    /// - `macro_version` is the sheet's macro version (`saved`, `null` when it names none) and its `status`: `none`,
    ///   `current` (the record is at that version and holds that content), `changed` (otherwise) or `missing` (it
    ///   holds no value);
    /// - `analysis` is the fresh exact analysis of the sheet's complete operation from the head under the protection
    ///   and policy in force ([`CommandLayer::analyse`]). The sheet stores no result that could stand in for it
    ///   (LB7.2), and a comparison grants nothing: a reuse still needs `worksheet.load` and a fresh engine preview.
    fn compare_now(&self, named: &WorksheetRef, draft: &str) -> Result<Value, Error> {
        self.retained_mode()?;
        let (sheet, saved, _) = self.saved_sheet(named)?;
        let (work, value) = self.drafted(draft)?;
        let version = self.records.get(draft).map_or(0, |record| record.version);
        let current = json::object([
            ("record", Value::String(draft.to_owned())),
            ("version", Value::Int(version as i64)),
            ("digest", Value::String(value.digest())),
        ]);
        let inputs = compare_inputs(&sheet, &work);
        let placed = self.place(sheet.clone(), Some(&saved))?.resolution.expect("place resolves the sheet");
        let moved = placed.pieces.iter().filter(|p| p.moved()).map(|p| Value::String(p.name.clone())).collect();
        let changed = placed.positions.iter().filter(|o| o.changed()).map(|o| Value::String(o.name.clone())).collect();
        let status = match &sheet.macro_version {
            None => "none",
            Some(saved) => match self.records.get(&saved.record) {
                Some(WorkspaceRecord { version, value: Some(value) }) => {
                    if *version == saved.version && value.digest() == saved.digest {
                        "current"
                    } else {
                        "changed"
                    }
                }
                _ => "missing",
            },
        };
        let macro_version = json::object([
            ("saved", sheet.macro_version.as_ref().map_or(Value::Null, MacroVersion::to_json)),
            ("status", Value::String(status.to_owned())),
        ]);
        Ok(json::object([
            ("worksheet", named.to_json()),
            ("draft", current),
            ("inputs", inputs),
            ("resolution", placed.to_json()),
            ("moved", Value::Array(moved)),
            ("occupant_changed", Value::Array(changed)),
            ("macro_version", macro_version),
            ("analysis", self.analyse(&sheet.letters())?),
        ]))
    }

    // ----- jumble.repair.list -----

    /// The list that completed `jumble.repair.list` job `job` made, while it is the newest one and no action has moved
    /// the state since (draft): the value its completion's result reference digests. A reopen keeps none.
    pub fn repair_list(&self, job: u64) -> Option<&Value> {
        self.repairs.as_ref().filter(|(id, _)| *id == job).map(|(_, value)| value)
    }

    /// `jumble.repair.list {snapshot, cap, max_chunk}` (BND.2, the 17 October exception; JI.12, JI.33; draft): the
    /// repair list at the cap the invocation names, on the head's poses. Admission queues a job on the exclusive lane;
    /// [`CommandLayer::run_jobs`] runs it ([`CommandLayer::build_repair`]). The row names its cap: an invocation
    /// without one is refused with `KN5:input.argument_form` before this admission, so no job starts for an unnamed
    /// cap (BND.64), and nothing here picks a cap. Refused, queuing nothing, in this order:
    /// - no pinned twist menu (`KN5:input.row_not_built`) or Jumble off (`KN5:input.menu_word_needs_jumble_on`);
    /// - `cap` not a `CapId` or `max_chunk` not a `PositiveNat` (`KN5:input.argument_form`; [`repair::named_cap`]);
    /// - `snapshot` not KN0.2's `RevisionRef` (`KN5:input.argument_form`; a derived state reference is not built in the
    ///   headless layer) or not the current revision (`KN5:stale.revision_changed`);
    /// - in a solving session, a run that is not inside a command the solver invoked (`LG3:capability.not_granted`):
    ///   the cap then comes from the solver's command, which the checker's repair-list exception holds to the cap the
    ///   solver named (BND.2, BND.64; c600-check's `Row::RepairList`). Outside a solving session BND.64 does not apply;
    /// - a held exclusive lane (`KN7:busy.lane_full`).
    fn admit_repair(&mut self, caller: &Caller, invocation: &Invocation) -> Result<(Option<u64>, Status), Error> {
        self.jumble_mode(REPAIR_LIST_ROW)?;
        let args = invocation.args.as_object().expect("the dispatch checked that the arguments are an object");
        let cap = repair::named_cap(args)?;
        let snapshot = args.get("snapshot").expect("the dispatch checked that the row's arguments are named");
        let row = self.known(REPAIR_LIST_ROW)?;
        let as_revision = Row { arguments: [("snapshot".to_owned(), "RevisionRef".to_owned())].into(), ..row.clone() };
        if let Err(error) = forms::check_forms(&as_revision, args) {
            let detail = "`snapshot` names a revision, KN0.2's RevisionRef; a derived state is not built here";
            return Err(Error::new(error.code, detail));
        }
        if *snapshot != self.revision.to_json() {
            return refused(code::REVISION_CHANGED, "`snapshot` names a revision that is not current");
        }
        if let Caller::Runtime { run } = caller {
            if self.profile == Profile::Solving && !self.inside_solver_command(run) {
                let detail = format!("run `{run}` is not inside a command the solver invoked (BND.64)");
                return refused(code::NOT_GRANTED, detail);
            }
        }
        self.exclusive_free()?;
        let id = self.next_job();
        let (command, scope) = (invocation.command.clone(), caller.scope());
        self.jobs.push_back(Job { id, command, scope, lane: Lane::Exclusive, kind: JobKind::Repair { cap } });
        self.generation += 1;
        Ok((Some(id), Status::Running(id)))
    }

    /// Runs `jumble.repair.list` job `id` and settles its receipt: `completed(query)` with the list's digest, kept as
    /// the newest list ([`CommandLayer::repair_list`]), or failed with [`CommandLayer::repair_now`]'s error. Unlike a
    /// run's preview, a run's list is not checked again here: admission held it to a command the solver invoked, and
    /// the list is a query of the revision it was admitted at that changes nothing and picks no pair.
    fn build_repair(&mut self, id: u64, key: &Key, cap: i64) {
        let status = match self.repair_now(cap) {
            Ok(value) => {
                let result = ResultRef { job: Some(id), digest: digest(&value) };
                self.repairs = Some((id, value));
                let revision = self.revision.clone();
                Status::Completed(Completion { kind: CompletionKind::Query, revision, result: Some(result) })
            }
            Err(error) => Status::Failed(error),
        };
        self.settle(key, status);
        self.generation += 1;
    }

    /// The repair list at `cap` on the head's poses (draft): the kernel's answer whole
    /// (`c600_jumble::observe::cuts::repair_list`, with no pair budget and no cancellation), worded by
    /// [`repair::worded`]. Its `status` is the kernel's: `completed` with its rows, or `error` with no outcomes for a
    /// blocked or closed cap, a fact about the configuration (BND.4). A state change cancels a queued job
    /// ([`CommandLayer::end_revision`]), so the job reads the input it was admitted at. An engine fault is
    /// `KN5:consistency.state_hash_mismatch`, as for the other jumbling rows.
    fn repair_now(&self, cap: i64) -> Result<Value, Error> {
        let (Some(twists), Some(jumbled)) = (&self.twists, &self.jumbled) else {
            let detail = format!("`{REPAIR_LIST_ROW}` reads the poses: Jumble is off");
            return Err(Error::new(MENU_WORD_NEEDS_JUMBLE_ON, detail));
        };
        let list = cuts::repair_list(&twists.context, jumbled.derived.state(), cap, None, None, None);
        let engine = |error: String| Error::new(code::STATE_HASH_MISMATCH, format!("the jumbling engine: {error}"));
        Ok(repair::worded(list.map_err(engine)?, cap))
    }

    /// The exact analysis of `letters` as one complete operation from the head under the protection and policy in
    /// force (LB2.2, LB2.3; draft): what a preview's summary reports (its literal word, net map, support, conflicts,
    /// verdict, mode and first refused prefix, and the end state's digest, `null` when protection refuses it) and LB2's
    /// gains and losses ([`CommandLayer::gains_and_losses`]), which are reported also when protection refuses it.
    fn analyse(&self, letters: &[i64]) -> Result<Value, Error> {
        let recipe = self.kernel.word(letters)?;
        let base = &self.current;
        let (end, effect, protection) = match self.derive(base, &recipe, &self.protected).map_err(kernel_error)? {
            Derivation::Derived(d) => (Some(d.state), d.effect, d.protection),
            Derivation::Refused(r) => (None, r.effect, r.protection),
        };
        let (gains, losses) = self.gains_and_losses(base, &effect)?;
        let verdict = match protection.verdict {
            Verdict::Clear => "clear",
            Verdict::Moves => "moves",
        };
        let positions = |list: Vec<i64>| Value::Array(list.into_iter().map(Value::Int).collect());
        Ok(json::object([
            ("literal", Value::String(effect.expansion_sha256.clone())),
            ("base", Value::String(base.digest().to_owned())),
            ("end", end.as_ref().map_or(Value::Null, |s| Value::String(s.digest().to_owned()))),
            ("net_sha256", Value::String(effect.net_sha256.clone())),
            ("net_moved_stickers", Value::Int(effect.moved_stickers() as i64)),
            ("net_moved_pieces", Value::Int(effect.moved_pieces as i64)),
            ("primitive_count", Value::Int(effect.primitive_count as i64)),
            ("support", rows_json(&effect.support)),
            ("conflicts", rows_json(&protection.conflicts)),
            ("verdict", Value::String(verdict.into())),
            ("mode", Value::String(policy_name(protection.mode).to_owned())),
            ("first_refused_prefix", protection.first_refused_prefix.map_or(Value::Null, |n| Value::Int(n as i64))),
            ("gains", positions(gains)),
            ("losses", positions(losses)),
        ]))
    }

    /// LB2's gains and losses of `effect` applied to `base` (0.4's exact gains and losses; draft): the positions,
    /// each named by its home piece as KN2.4's `positions` row names it, in increasing order, whose home piece is not
    /// solved in `base` and is solved after the effect (gains), and the reverse (losses). A position is finished when
    /// every sticker of its home piece is in its home slot ([`PiecePosition::solved`]), on all labels, whatever a
    /// filter shows (LB2.2).
    fn gains_and_losses(&self, base: &Snapshot, effect: &Effect) -> Result<(Vec<i64>, Vec<i64>), Error> {
        let labels = base.labels();
        let mut end = labels.to_vec();
        for (&source, &destination) in effect.source().iter().zip(effect.destination()) {
            end[destination as usize] = labels[source as usize];
        }
        let all: Vec<i64> = (0..c600_kernel::PIECES as i64).collect();
        let (mut gains, mut losses) = (Vec::new(), Vec::new());
        for piece in self.kernel.positions(base, &all)? {
            let after = piece.home.iter().all(|&slot| end[slot as usize] == slot);
            match (piece.solved(), after) {
                (false, true) => gains.push(piece.piece as i64),
                (true, false) => losses.push(piece.piece as i64),
                _ => {}
            }
        }
        Ok((gains, losses))
    }

    // ----- jumble.on -----

    /// `jumble.on` (A5; ONE.54, ONE.55, ONE.80; draft): a crossing commit from Jumble off into Jumble on. Before the
    /// token is spent: the pinned menu (`KN5:input.row_not_built`) and the `revision` argument's form
    /// (`KN5:input.argument_form`). Then ONE.80's order: the token (step 3), the commit lane (step 4), its binding to
    /// this invocation and the session's identity block (step 5); the mode (`KN5:input.retained_tool_needs_jumble_off`:
    /// an already-on position needs no new entry); the named revision and the token's state and protection revisions
    /// (`KN5:stale.revision_changed`); the head hash (`KN5:consistency.state_hash_mismatch`); A5's witness
    /// ([`CommandLayer::plan_entry`]); the durable transaction with its crossing record; the adoption. A refusal
    /// changes nothing.
    fn jumble_on(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
        row: &Row,
    ) -> Result<(Option<u64>, Status), Error> {
        if self.twists.is_none() {
            return refused(code::ROW_NOT_BUILT, format!("`{JUMBLE_ON_ROW}` needs the host's pinned twist menu"));
        }
        let args = invocation.args.as_object().expect("the dispatch checked that the arguments are an object");
        forms::check_forms(row, args)?;
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        if self.jumbled.is_some() {
            return refused(
                code::RETAINED_TOOL_NEEDS_JUMBLE_OFF,
                "Jumble is on already: the position needs no new entry",
            );
        }
        forms::check_revisions(row, args, &self.revision)?;
        if binding.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        let adoption = self.plan_entry(Meta::of(caller, invocation, request))?;
        self.transact(adoption)
    }

    /// A crossing into Jumble on (A5, JI.25 to JI.27; draft): the head hash, then the witness at the head
    /// ([`CommandLayer::entry_at_head`]): the active exit's root, or with no active exit the solved witness, replayed
    /// in the jumbling engine under the pinned menu, which must reach the current labels. The crossing is a new node
    /// under the head that holds no retained recipe and has the pose digest; the revision's mode becomes
    /// [`JUMBLE_ON_MODE`], its state revision advances and its digest is the pose digest. The retained state, the
    /// protected set and the policy stay.
    fn plan_entry(&self, meta: Meta) -> Result<Adoption, Error> {
        let Some(twists) = &self.twists else {
            return Err(Error::new(
                code::ROW_NOT_BUILT,
                format!("`{JUMBLE_ON_ROW}` needs the host's pinned twist menu"),
            ));
        };
        if self.current.digest() != self.nodes[&self.head].digest {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state"));
        }
        let (entry, chain) = self.entry_at_head(twists)?;
        let order = self.order + 1;
        let event = format!("e{order}");
        let revision = Revision {
            mode: JUMBLE_ON_MODE,
            head: event.clone(),
            state: self.revision.state + 1,
            digest: entry.digest.clone(),
            ..self.revision.clone()
        };
        let depth = self.nodes[&self.head].depth + 1;
        let node = Node {
            parent: self.head,
            recipe: None,
            twist: None,
            exit: None,
            partial: None,
            digest: entry.digest.clone(),
            depth,
        };
        let transaction = Transaction {
            kind: TransactionKind::Crossing,
            event,
            order,
            target: None,
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal: String::new(),
            pre: self.current.digest().to_owned(),
            post: entry.digest.clone(),
            revision,
            recipe: Value::Null,
            orbits: self.protected.orbits().to_vec(),
            confirm: false,
            job: None,
            result: None,
            record: None,
            policy: None,
            records: Vec::new(),
            rotation: None,
            crossing: Some(entry.record.clone()),
            blocks: Vec::new(),
            budget: None,
        };
        let (identity, record) = (entry.identity, entry.record);
        let change = Change::Crossing { before: self.head, after: order, identity, record };
        let tree = Tree::Push { node: Some((order, node)), change };
        let side = Side::On(Jumbled { derived: entry.derived, digest: entry.digest, depth: chain });
        Ok(Adoption { transaction, state: None, protected: None, head: order, tree, preview: None, side })
    }

    /// The entry at the head with Jumble off (JI.25 to JI.27; draft), with the chain depth of its value: from the
    /// active exit's root ([`CommandLayer::active_exit`], [`jumble::replay_from_exit`]) when there is one, and
    /// otherwise from the solved witness ([`jumble::replay_from_solved`]), whose value is replayed from solved. A
    /// non-empty suffix nests one more derivation on the exit's poses, so after [`POSE_CHAIN`] chained derivations the
    /// value is the jumbling journal to the head replayed flat from solved ([`jumble::replay_poses`]), which must reach
    /// the same poses.
    fn entry_at_head(&self, twists: &Twists) -> Result<(jumble::Entry, usize), Error> {
        let labels = self.current.digest();
        let Some((exit, point, suffix)) = self.active_exit()? else {
            let letters = self.solved_witness()?;
            return Ok((jumble::replay_from_solved(twists, &letters, labels)?, 0));
        };
        let (root, crossing) = (&point.poses.derived, format!("e{exit}"));
        let mut entry = jumble::replay_from_exit(twists, root, &point.record, &crossing, &suffix, labels)?;
        let chain = point.poses.depth + usize::from(!suffix.is_empty());
        if chain < POSE_CHAIN {
            return Ok((entry, chain));
        }
        let (derived, digest) = jumble::replay_poses(twists, &self.jumble_journal(self.head)?)?;
        if digest != entry.digest {
            let detail = "the jumbling journal's replay from solved does not reach the exit root's replay";
            return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
        }
        entry.derived = derived;
        Ok((entry, 0))
    }

    /// The active exit at the head with Jumble off (JI.26; draft): the exit that the path from the head reaches through
    /// retained commits only, with its order, what its undo restores ([`ExitPoint`], whose poses are the exit's own)
    /// and the retained suffix from the exit to the head as 600-cell-Full's signed primitive letters, in chronological
    /// order. `None` when the path reaches the start first, where the solved witness applies. A pose move cannot
    /// supply a suffix: with Jumble on the path climbs to an entry. No void is built, so none names an exit.
    fn active_exit(&self) -> Result<Option<(u64, &ExitPoint, Vec<i32>)>, Error> {
        let mut suffix = Vec::new();
        let mut node = self.head;
        while let Some(recipe) = &self.nodes[&node].recipe {
            suffix.push(recipe);
            node = self.nodes[&node].parent;
        }
        let Some(Change::Exit { point, .. }) = self.changes.get(&node) else {
            return Ok(None);
        };
        let mut letters = Vec::new();
        for recipe in suffix.iter().rev() {
            letters.extend(self.kernel.letters(recipe)?);
        }
        Ok(Some((node, point.as_ref(), letters)))
    }

    /// JI.25's solved witness, when no exit is active (JI.26): the chronological literal retained path from the
    /// session's start to the head, as 600-cell-Full's signed primitive letters. The start must be solved and every node
    /// on the path a retained commit; otherwise there is no witness, and labels alone are refused
    /// (`KN5:input.jumble_on_labels_only`).
    fn solved_witness(&self) -> Result<Vec<i32>, Error> {
        let labels_only = |detail: &str| Error::new(code::JUMBLE_ON_LABELS_ONLY, detail);
        if self.nodes[&0].digest != self.kernel.solved().digest() {
            return Err(labels_only("the session did not start at solved, so it holds no retained word from solved"));
        }
        let mut path = Vec::new();
        let mut node = self.head;
        while node != 0 {
            path.push(node);
            node = self.nodes[&node].parent;
        }
        let mut letters = Vec::new();
        for node in path.iter().rev() {
            let Some(recipe) = &self.nodes[node].recipe else {
                return Err(labels_only("the path from the start crosses a node that holds no retained recipe"));
            };
            letters.extend(self.kernel.letters(recipe)?);
        }
        Ok(letters)
    }

    // ----- jumble.off -----

    /// `jumble.off` (A5; ONE.54, ONE.80, JI.27; draft): a crossing commit from Jumble on back to Jumble off. Before
    /// the token is spent: the pinned menu (`KN5:input.row_not_built`) and the `revision` argument's form
    /// (`KN5:input.argument_form`). Then ONE.80's order, as for `jumble.on`: the token (step 3), the commit lane (step
    /// 4), its binding to this invocation and the session's identity block (step 5); the mode
    /// (`KN5:input.menu_word_needs_jumble_on`: with Jumble off there is no episode to exit); the named revision and
    /// the token's state and protection revisions (`KN5:stale.revision_changed`); the head hash
    /// (`KN5:consistency.state_hash_mismatch`); A5's label checkpoint ([`CommandLayer::plan_exit`]); the durable
    /// transaction with its exit record; the adoption. A refusal changes nothing.
    fn jumble_off(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
        row: &Row,
    ) -> Result<(Option<u64>, Status), Error> {
        if self.twists.is_none() {
            return refused(code::ROW_NOT_BUILT, format!("`{JUMBLE_OFF_ROW}` needs the host's pinned twist menu"));
        }
        let args = invocation.args.as_object().expect("the dispatch checked that the arguments are an object");
        forms::check_forms(row, args)?;
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        if self.jumbled.is_none() {
            return refused(MENU_WORD_NEEDS_JUMBLE_ON, "Jumble is off already: there is no episode to exit");
        }
        forms::check_revisions(row, args, &self.revision)?;
        if binding.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        let adoption = self.plan_exit(Meta::of(caller, invocation, request))?;
        self.transact(adoption)
    }

    /// An exit to Jumble off (A5, JI.27; draft): the head hash, then A5's label checkpoint at the head's poses
    /// ([`CommandLayer::exit_at_head`]). The exit is a new node under the head that holds no retained recipe and has
    /// the exit's label digest; the poses' labels become the retained state, the revision's mode becomes [`MODE`], its
    /// state revision advances and its digest is the label digest. The protected set and the policy stay: the exit
    /// checks no protected orbit (draft).
    fn plan_exit(&self, meta: Meta) -> Result<Adoption, Error> {
        if self.state_digest() != self.nodes[&self.head].digest {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state"));
        }
        let Some(head) = &self.jumbled else {
            return Err(Error::new(MENU_WORD_NEEDS_JUMBLE_ON, "Jumble is off already: there is no episode to exit"));
        };
        let exit = self.exit_at_head()?;
        let post = exit.state.digest().to_owned();
        let order = self.order + 1;
        let event = format!("e{order}");
        let revision = Revision {
            mode: MODE,
            head: event.clone(),
            state: self.revision.state + 1,
            digest: post.clone(),
            ..self.revision.clone()
        };
        let depth = self.nodes[&self.head].depth + 1;
        let mark = Some(digest(&exit.record));
        let node = Node {
            parent: self.head,
            recipe: None,
            twist: None,
            exit: mark,
            partial: None,
            digest: post.clone(),
            depth,
        };
        let transaction = Transaction {
            kind: TransactionKind::Exit,
            event,
            order,
            target: None,
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal: String::new(),
            pre: self.state_digest().to_owned(),
            post,
            revision,
            recipe: Value::Null,
            orbits: self.protected.orbits().to_vec(),
            confirm: false,
            job: None,
            result: None,
            record: None,
            policy: None,
            records: Vec::new(),
            rotation: None,
            crossing: Some(exit.record.clone()),
            blocks: Vec::new(),
            budget: None,
        };
        let point = ExitPoint { poses: head.clone(), retained: self.current.clone(), record: exit.record };
        let change = Change::Exit { before: self.head, after: order, point: Box::new(point) };
        let tree = Tree::Push { node: Some((order, node)), change };
        let state = Some(exit.state);
        Ok(Adoption { transaction, state, protected: None, head: order, tree, preview: None, side: Side::Off })
    }

    /// A5's label checkpoint at the head's poses with Jumble on ([`jumble::exit`]): in the episode that the head's
    /// entry began, named by the entry's event, along the jumbling journal from solved to the head.
    fn exit_at_head(&self) -> Result<jumble::Exit, Error> {
        let mismatch = |detail: &str| Error::new(code::STATE_HASH_MISMATCH, detail);
        let (Some(twists), Some(head)) = (&self.twists, &self.jumbled) else {
            return Err(mismatch("an exit needs Jumble on under the pinned twist menu"));
        };
        // With Jumble on the head's path climbs through pose moves to the entry.
        let mut entry = self.head;
        while self.nodes[&entry].twist.is_some() {
            entry = self.nodes[&entry].parent;
        }
        if entry == 0 || self.nodes[&entry].recipe.is_some() {
            return Err(mismatch("the head's poses do not descend from an entry"));
        }
        let journal = self.jumble_journal(self.head)?;
        let exit = jumble::exit(twists, &self.kernel, &head.derived, &format!("e{entry}"), &journal)?;
        if exit.record.get("poses").and_then(Value::as_str) != Some(head.digest.as_str()) {
            return Err(mismatch("the checkpoint's poses are not the head's"));
        }
        Ok(exit)
    }

    // ----- jumble.twist.menu -----

    /// `jumble.twist.menu` (JI.35, FP.31, JI.27; draft): a live menu twist, committed at once with no preview (ONE.78).
    /// Before the token is spent: the pinned menu (`KN5:input.row_not_built`); the `revision` argument's form
    /// (`KN5:input.argument_form`); `letter`, KN3.7's binding without the cap the row names
    /// (`KN2:input.rotation_request_malformed`), which must resolve against the pinned menu (`KN2:input.*`, as for
    /// `twist.apply`); the item, which must not be the menu's identity (`KN2:input.recipe_shape`) and with Jumble off
    /// must be a retained one (`KN5:input.menu_word_needs_jumble_on`, JI.35). Then the commit steps: the token (step
    /// 3), the commit lane (step 4), its binding to this invocation and the session's identity block (step 5), the
    /// named revision and the token's state and protection revisions (`KN5:stale.revision_changed`), the head hash
    /// (step 7) and the move. With Jumble off the item's retained word is derived by the kernel under the protection
    /// set and policy in force (step 8: `KN2:protection.orbit_would_move`) and committed as a retained commit; with
    /// Jumble on the item's exact twist is a pose move ([`CommandLayer::plan_pose`]). The durable transaction carries
    /// the rotation record (step 9), and the adoption ends the preview and the jobs that read the replaced revision,
    /// except file jobs (step 10, LI.32). A refusal changes nothing.
    fn twist_menu(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
        row: &Row,
    ) -> Result<(Option<u64>, Status), Error> {
        let Some(twists) = &self.twists else {
            return refused(code::ROW_NOT_BUILT, format!("`{JUMBLE_TWIST_ROW}` needs the host's pinned twist menu"));
        };
        let args = invocation.args.as_object().expect("the dispatch checked that the arguments are an object");
        forms::check_forms(row, args)?;
        let binding = twist::binding_at(&invocation.args, "letter")?;
        let resolution = rotation::resolve(&twists.context, Some(&*twists.menu), &binding)?;
        if *resolution.twist.matrix == identity4() {
            return refused(c600_kernel::codes::RECIPE_SHAPE, "the menu's identity item moves no piece");
        }
        // With Jumble off a retained item dispatches to its retained word, and a jumble item needs Jumble on; with
        // Jumble on every item is a pose move (JI.35).
        let recipe = match self.jumbled {
            Some(_) => None,
            None => Some(self.kernel.word(&rotation::retained_word(&twists.context, &resolution)?)?),
        };
        let token = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&token, invocation)?;
        forms::check_revisions(row, args, &self.revision)?;
        if token.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        if self.state_digest() != self.nodes[&self.head].digest {
            return refused(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state");
        }
        let meta = Meta::of(caller, invocation, request);
        let adoption = match recipe {
            Some(recipe) => {
                let derived = match self.derive(&self.current, &recipe, &self.protected).map_err(kernel_error)? {
                    Derivation::Derived(derived) => derived,
                    Derivation::Refused(refusal) => return Err(refusal.refusal.into()),
                };
                let mut adoption = self.commit_adoption(meta, recipe, derived.state, derived.word, None);
                adoption.transaction.rotation = Some(resolution.to_json());
                adoption
            }
            None => self.plan_pose(meta, resolution.twist.clone(), resolution.to_json())?,
        };
        self.transact(adoption)
    }

    /// A pose move with Jumble on (FP.31's `jumble_move`, JI.27; draft): the head hash, then `twist`, an item of the
    /// pinned menu, derived from the head's value ([`CommandLayer::pose_value`]). The move is a new node under the head
    /// that holds the twist and has the new pose digest; the state revision advances and the revision's digest is the
    /// new pose digest, while the mode, the retained state, the protected set and the policy stay. `rotation` is the
    /// move's rotation record, which the journal keeps.
    fn plan_pose(&self, meta: Meta, twist: Twist, rotation: Value) -> Result<Adoption, Error> {
        if self.state_digest() != self.nodes[&self.head].digest {
            return Err(Error::new(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state"));
        }
        let jumbled = self.pose_value(Some(&twist), || {
            let mut word = self.jumble_journal(self.head)?;
            word.push(twist.clone());
            Ok(word)
        })?;
        let order = self.order + 1;
        let event = format!("e{order}");
        let revision = Revision {
            head: event.clone(),
            state: self.revision.state + 1,
            digest: jumbled.digest.clone(),
            ..self.revision.clone()
        };
        let literal = levels::word_digest(std::slice::from_ref(&twist));
        let depth = self.nodes[&self.head].depth + 1;
        let digest = jumbled.digest.clone();
        let node =
            Node { parent: self.head, recipe: None, twist: Some(twist), exit: None, partial: None, digest, depth };
        let transaction = Transaction {
            kind: TransactionKind::JumbleMove,
            event,
            order,
            target: None,
            caller: meta.caller,
            command: meta.command,
            row: meta.row,
            request: meta.request,
            literal,
            pre: self.state_digest().to_owned(),
            post: jumbled.digest.clone(),
            revision,
            recipe: Value::Null,
            orbits: self.protected.orbits().to_vec(),
            confirm: false,
            job: None,
            result: None,
            record: None,
            policy: None,
            records: Vec::new(),
            rotation: Some(rotation),
            crossing: None,
            blocks: Vec::new(),
            budget: None,
        };
        let change = Change::Pose { before: self.head, after: order };
        let tree = Tree::Push { node: Some((order, node)), change };
        let side = Side::On(jumbled);
        Ok(Adoption { transaction, state: None, protected: None, head: order, tree, preview: None, side })
    }

    /// The jumbling engine's value after one step from the head's value with Jumble on (draft). `step`, an item of the
    /// pinned menu, is derived from the head's value under the policy in force, with the pieces that carry the
    /// protected orbits as pose-protected pieces ([`jumble::protected_pieces`]; finding B5, the owner's reading of 18
    /// November 2026). The verdict is the engine's own at the pose level, so no label verdict crosses into Jumble
    /// (chapter 03, KN1.11). A step that would move such a piece is refused with `KN2:protection.piece_would_move`,
    /// and a blocked step with the engine's code (`KN2:input.word_blocked_prefix`). The engine nests each derivation's
    /// evidence in the next, so the chain is bounded: after [`POSE_CHAIN`] chained steps, or with no step (an undo
    /// whose exact inverse the menu does not hold), the value is `journal`, the whole jumbling journal to the step's
    /// end, replayed flat from solved ([`jumble::replay_poses`]), which must reach the step's poses. Such an undo has
    /// no verdict and needs none: every protection change made after the move it takes back is undone before it, so the
    /// protection in force is the one the move was judged under, and the inverse moves only the pieces the move moved.
    /// A reopen replays a stored pose move through this step too, so one that would move a protected piece leaves the
    /// layer in read-only recovery before it with the engine's code.
    fn pose_value(
        &self,
        step: Option<&Twist>,
        journal: impl FnOnce() -> Result<Vec<Twist>, Error>,
    ) -> Result<Jumbled, Error> {
        let (Some(twists), Some(head)) = (&self.twists, &self.jumbled) else {
            let detail = "a pose move needs Jumble on under the pinned twist menu";
            return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
        };
        let ctx = &*twists.context;
        let chained = match step {
            None => None,
            Some(twist) => {
                let (base, identity) = (Base::Derived(&head.derived), head.derived.identity());
                let protect = jumble::protected_pieces(twists, self.protected.orbits());
                let derivation =
                    levels::derive(ctx, base, identity, std::slice::from_ref(twist), self.policy, &protect);
                let engine = |error| Error::new(code::STATE_HASH_MISMATCH, format!("the jumbling engine: {error}"));
                match derivation.map_err(engine)? {
                    Err(refusal) => return Err(refusal.into()),
                    Ok(levels::Derivation::Refused(refused)) => return Err(refused.refusal.into()),
                    Ok(levels::Derivation::Derived(derived)) => {
                        let digest = derived.digest(ctx);
                        if head.depth + 1 < POSE_CHAIN {
                            return Ok(Jumbled { derived, digest, depth: head.depth + 1 });
                        }
                        Some(digest)
                    }
                }
            }
        };
        let (derived, digest) = jumble::replay_poses(twists, &journal()?)?;
        if chained.is_some_and(|chained| chained != digest) {
            let detail = "the jumbling journal's replay from solved does not reach the chained poses";
            return Err(Error::new(code::STATE_HASH_MISMATCH, detail));
        }
        Ok(Jumbled { derived, digest, depth: 0 })
    }

    /// The jumbling journal from solved to node `to` (draft): along the move tree's path from the start, the primitive
    /// letters of every retained commit and the twist of every pose move; an entry and an exit move no piece. The
    /// start must be solved, as a crossing's witness needs (JI.25). The journal runs on through an exit: the exit's
    /// labels are its poses' labels, and a retained commit after it moves those poses as it moves the labels, so the
    /// journal replays from solved to poses whose labels are the retained state's.
    fn jumble_journal(&self, to: u64) -> Result<Vec<Twist>, Error> {
        let mismatch = |detail: String| Error::new(code::STATE_HASH_MISMATCH, detail);
        let Some(twists) = &self.twists else {
            return Err(mismatch(format!("`{JUMBLE_TWIST_ROW}` needs the host's pinned twist menu")));
        };
        if self.nodes[&0].digest != self.kernel.solved().digest() {
            return Err(mismatch(
                "the session did not start at solved, so its journal does not replay from solved".into(),
            ));
        }
        let mut path = Vec::new();
        let mut node = to;
        while node != 0 {
            path.push(node);
            node = self.nodes[&node].parent;
        }
        let mut word = Vec::new();
        for node in path.iter().rev().map(|node| &self.nodes[node]) {
            if let Some(recipe) = &node.recipe {
                for letter in self.kernel.letters(recipe)? {
                    let primitive = levels::primitive(&twists.context, i64::from(letter));
                    word.push(primitive.map_err(|error| mismatch(format!("the jumbling engine: {error}")))?);
                }
            }
            word.extend(node.twist.clone());
        }
        Ok(word)
    }

    // ----- language-defined rows -----

    /// The argument blocks of an invocation that an entry serves (BND.22): the host's reader checks each block's form,
    /// as for a record's block (PS1.16). The layer reads nothing else in them.
    fn check_inputs(&self, row: &Row, inputs: &[Vec<u8>]) -> Result<(), Error> {
        if inputs.is_empty() {
            return Ok(());
        }
        let Some(reader) = &self.provenance else {
            let detail = format!("`{}`'s argument blocks need the host's provenance reader", row.id);
            return Err(Error::new(code::ROW_NOT_BUILT, detail));
        };
        inputs.iter().try_for_each(|block| reader.check(block))
    }

    fn admit_program(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        row: &Row,
        inputs: &[Vec<u8>],
    ) -> Result<(Option<u64>, Status), Error> {
        let overlay = row.overlay.clone().expect("an overlay row");
        // The budget is admitted before a job ID exists, so a refusal uses nothing (KN7.7, BM3).
        let budget = Budget::admit(invocation.budget, self.budget_ceiling())?;
        if row.lane == Lane::Program {
            self.program_free()?;
        }
        let id = self.next_job();
        if row.lane == Lane::View {
            let superseded: Vec<u64> = self
                .jobs
                .iter()
                .filter(|job| job.lane == Lane::View && matches!(&job.kind, JobKind::Program(p) if p.row == row.id))
                .map(|job| job.id)
                .collect();
            let (ended, kept) =
                std::mem::take(&mut self.jobs).into_iter().partition(|job| superseded.contains(&job.id));
            self.jobs = kept;
            for job in ended {
                let reason = CancelReason::Superseded(invocation.command.clone());
                self.settle(&(job.scope, job.command), Status::Cancelled(reason));
            }
            // The row's runner is fenced too (LI.23): its result cannot publish after this request.
            let runners: Vec<u64> =
                self.running.iter().filter(|(_, program)| program.row == row.id).map(|(&job, _)| job).collect();
            for job in runners {
                let program = self.running.remove(&job).expect("a listed runner");
                let reason = CancelReason::Superseded(invocation.command.clone());
                self.settle(&(program.caller.scope(), program.command.clone()), Status::Cancelled(reason));
                self.fenced.insert(job, program);
            }
        }
        let program = ProgramJob {
            job: id,
            command: invocation.command.clone(),
            row: row.id.clone(),
            kind: row.kind,
            args: invocation.args.clone(),
            caller: caller.clone(),
            revision: self.revision.clone(),
            overlay,
            inputs: inputs.to_vec(),
            budget,
            module: self.child_module(caller, &invocation.command),
        };
        self.jobs.push_back(Job {
            id,
            command: invocation.command.clone(),
            scope: caller.scope(),
            lane: row.lane,
            kind: JobKind::Program(program),
        });
        self.generation += 1;
        Ok((Some(id), Status::Running(id)))
    }

    /// Takes the next queued program job for the host to run. A view row's replacement starts only after the host
    /// reported the runner it fenced (LI.23), so until then it stays queued.
    pub fn take_program_job(&mut self) -> Option<ProgramJob> {
        let position = self.jobs.iter().position(|job| match &job.kind {
            JobKind::Program(program) => !self.fenced.values().any(|runner| runner.row == program.row),
            _ => false,
        })?;
        let job = self.jobs.remove(position)?;
        let JobKind::Program(program) = job.kind else { unreachable!("the position names a program job") };
        self.running.insert(program.job, program.clone());
        Some(program)
    }

    /// Reports a program job's result: the host's interface. A job a commit or undo cancelled meanwhile, or a view
    /// runner a newer request of its row fenced, stays cancelled, and its result is dropped (`None`). A workspace row's completion is one durable action, written with
    /// its receipt (KN5.23, L8); in read-only recovery it fails. Other completions stay transient. The same as
    /// [`CommandLayer::finish_program_job_with`] with an output that publishes no view and writes no record.
    pub fn finish_program_job(&mut self, job: u64, result: Result<Value, Error>) -> Option<Receipt> {
        self.finish_program_job_with(job, result.map(JobOutput::of))
    }

    /// Reports a program job's output as the runtime's sinks gave it: the value its entry returned, the views it
    /// published and the workspace records it wrote (LP3's C6, its second common need; draft). The output becomes the
    /// receipt: the completion's result reference carries [`JobOutput::digest`].
    ///
    /// Before the layer takes the output, it checks it: only a workspace row's job writes records
    /// (`LG3:capability.not_granted` otherwise), and every record's provenance block passes the host's reader, as for
    /// `workspace.set` (PS1.16), and a row that takes the gate `fresh_filter_preview` must pass it again, or the job
    /// fails with `KN5:stale.filter_preview` (KN0.9). A job whose output fails a check fails with that error and leaves
    /// nothing written. A workspace row's records are written in its completion's one durable action, which
    /// `session.undo` reaches for a row whose undo class is `workspace`, every language-defined workspace row's
    /// included, and never for a stored row whose undo class is `none`. A completed `filter.preview` job's
    /// result is kept as a filter preview ([`CommandLayer::filter_preview`]).
    pub fn finish_program_job_with(&mut self, job: u64, result: Result<JobOutput, Error>) -> Option<Receipt> {
        // A fenced runner's report releases its row's runner slot; its result never publishes (LI.23).
        if self.fenced.remove(&job).is_some() {
            self.generation += 1;
            return None;
        }
        let program = self.running.remove(&job)?;
        let key = (program.caller.scope(), program.command.clone());
        // The gate `fresh_filter_preview` must hold again when the output is published (KN0.9).
        let gate = |output: JobOutput| match self.table.row(&program.row) {
            Some(row) => self.fresh_filter_gate(row, &program.args).map(|()| output),
            None => Ok(output),
        };
        let status = match result.and_then(|output| self.admit_output(&program, output)).and_then(gate) {
            Err(error) => Status::Failed(error),
            Ok(_) if program.kind == Kind::Workspace && self.recovery.is_some() => {
                Status::Failed(Error::new(code::READ_ONLY_RECOVERY, "the session is in read-only recovery"))
            }
            Ok(output) if program.kind == Kind::Workspace => match self.workspace_writes(&program, output) {
                Err(error) => Status::Failed(error),
                Ok((result, records)) => {
                    let request = self.receipts.get(&key).map(|record| record.request.clone()).unwrap_or_default();
                    let meta = Meta {
                        caller: program.caller.clone(),
                        command: program.command.clone(),
                        row: program.row.clone(),
                        request,
                        confirm: false,
                    };
                    let adoption = self.workspace_adoption(meta, job, result, records, Some(program.budget));
                    match self.transact(adoption) {
                        Ok((_, status)) => status,
                        Err(error) => Status::Failed(error),
                    }
                }
            },
            Ok(output) => {
                let kind = match program.kind {
                    Kind::Query => CompletionKind::Query,
                    Kind::File => CompletionKind::File,
                    Kind::View => CompletionKind::ViewPublished,
                    Kind::Workspace | Kind::Commit | Kind::App => {
                        unreachable!(
                            "a workspace completion is durable, and an overlay row is never a commit or app row"
                        )
                    }
                };
                let result = ResultRef { job: Some(job), digest: output.digest() };
                if program.row == filter::FILTER_PREVIEW_ROW && kind == CompletionKind::Query {
                    self.keep_filter_preview(&program, result.clone());
                }
                Status::Completed(Completion { kind, revision: program.revision.clone(), result: Some(result) })
            }
        };
        self.settle(&key, status);
        self.generation += 1;
        let receipt = self.receipts.get(&key).map(|record| record.receipt.clone());
        self.reconcile_children();
        receipt
    }

    /// The checks a job's output passes before the layer takes it (draft): only a workspace row's job writes records,
    /// and the host's reader admits every record's provenance block (PS1.16). Views pass from every row.
    fn admit_output(&self, program: &ProgramJob, output: JobOutput) -> Result<JobOutput, Error> {
        if output.records.is_empty() {
            return Ok(output);
        }
        if program.kind != Kind::Workspace {
            let detail = format!("`{}` is not a workspace row, so its job writes no record", program.row);
            return Err(Error::new(code::NOT_GRANTED, detail));
        }
        let Some(reader) = &self.provenance else {
            let detail = format!("`{}`'s records need the host's provenance reader", program.row);
            return Err(Error::new(code::ROW_NOT_BUILT, detail));
        };
        for value in output.records.values() {
            reader.check(value.provenance.as_bytes())?;
        }
        Ok(output)
    }

    /// A workspace row's completion writes and the digest its result reference carries: the output's records and
    /// [`JobOutput::digest`], except for `set.save` ([`crate::sets`]); another row's record under `set/` fails the job
    /// with `KN2:input.named_object_invalid`. `set.save`'s entry writes the one record `set.save`,
    /// which must name the set the invocation names: the same kind, name and members, these in canonical order.
    /// Otherwise the job fails with `KN2:input.named_object_invalid`. The completion writes the saved set's own record
    /// `set/<name>`, keeping the entry's provenance block, and its result reference carries the exact set's digest
    /// under the model the job read. A name that holds a saved set fails it with `KN5:input.saved_set_exists` unless
    /// the invocation carried `replace: true`. A failed job writes nothing.
    fn workspace_writes(&self, program: &ProgramJob, output: JobOutput) -> Result<(String, Vec<RecordWrite>), Error> {
        // The applied grip map and the macro base are written by their own rows alone (EV4.8, EV4.9).
        if let Some(name) = output.records.keys().find(|name| reserved_by_row(name)) {
            let detail = format!("the output's record `{name}` is the grip map's or the macro base's");
            return Err(Error::new(code::NAMED_OBJECT_INVALID, detail));
        }
        if program.row != SET_SAVE_ROW {
            if let Some(name) = output.records.keys().find(|name| name.starts_with(SAVED_SET_PREFIX)) {
                let detail = format!("the output's record `{name}` is a saved set's, which only `set.save` writes");
                return Err(Error::new(code::NAMED_OBJECT_INVALID, detail));
            }
            let result = output.digest();
            let records =
                output.records.into_iter().map(|(name, value)| RecordWrite { name, value: Some(value) }).collect();
            return Ok((result, records));
        }
        let mut records = output.records;
        let entry = records.remove(SET_SAVE_RECORD).filter(|_| records.is_empty()).ok_or_else(|| {
            let detail = format!("`{SET_SAVE_ROW}`'s entry writes the one record `{SET_SAVE_RECORD}`");
            Error::new(code::NAMED_OBJECT_INVALID, detail)
        })?;
        let set = SavedSet::from_entry(&entry.payload)?;
        let args = &program.args;
        let mut asked: Vec<u64> = match args.get("members").and_then(Value::as_array) {
            Some(members) => members.iter().filter_map(|m| m.as_i64().and_then(|n| u64::try_from(n).ok())).collect(),
            None => Vec::new(),
        };
        asked.sort_unstable();
        asked.dedup();
        let kind = args.get("kind").and_then(Value::as_str);
        let name = args.get("name").and_then(Value::as_str);
        if (kind, name, &asked) != (Some(set.kind), Some(set.name.as_str()), &set.members) {
            let detail = format!("the entry's record `{SET_SAVE_RECORD}` is not the set the invocation names");
            return Err(Error::new(code::NAMED_OBJECT_INVALID, detail));
        }
        self.check_set_name(&set.name, args.get(REPLACE) == Some(&Value::Bool(true)))?;
        let model = &program.revision.identity;
        let value = RecordValue { payload: set.payload(model, &program.revision), provenance: entry.provenance };
        Ok((set.digest(model), vec![RecordWrite { name: saved_set_record(&set.name), value: Some(value) }]))
    }

    /// A save under `name` when the name holds a saved set is refused with `KN5:input.saved_set_exists`, unless it
    /// carries `replace: true` (U1-057; draft).
    fn check_set_name(&self, name: &str, replace: bool) -> Result<(), Error> {
        if replace || self.record(&saved_set_record(name)).is_none() {
            return Ok(());
        }
        let detail = format!("a saved set is named `{name}`; a save with `replace: true` replaces it");
        Err(Error::new(code::SAVED_SET_EXISTS, detail))
    }
}

/// A record name `workspace.set` and `worksheet.save` refuse: a name under `set/`, which only a `set.save` completion
/// writes ([`crate::sets`]; draft), under `grip/`, which only `grip.rebind` writes ([`crate::grip`]), or under
/// `macro/`, which only `macro.save` writes ([`crate::macros`]).
fn reserved_name(name: &str) -> Result<(), Error> {
    let owner = [(SAVED_SET_PREFIX, SET_SAVE_ROW), (GRIP_PREFIX, GRIP_REBIND_ROW), (MACRO_PREFIX, MACRO_SAVE_ROW)]
        .into_iter()
        .find(|(prefix, _)| name.starts_with(prefix));
    match owner {
        Some((prefix, row)) => {
            Err(Error::new(code::ARGUMENT_FORM, format!("a name under `{prefix}` is reserved; only `{row}` writes it")))
        }
        None => Ok(()),
    }
}

/// Whether record `name` is the applied grip map's or the macro base's, which only `grip.rebind` and `macro.save` write
/// (EV4.8, EV4.9).
fn reserved_by_row(name: &str) -> bool {
    name.starts_with(GRIP_PREFIX) || name.starts_with(MACRO_PREFIX)
}

/// Whether a stored record write of `name` with `value` is one the live path makes (EV4.8, EV4.9; B7): a write of
/// `grip/` or `macro/`, and a write under `grip.rebind` or `macro.save`, come only from that row, from the shell, to
/// its own record with a value that reads; a `macro.save` write also carries its stored confirmation. A macro's block
/// is the row's own, or, for a stored word, the block of the record its source names, which `records` (the replay's,
/// at this point of the journal) holds at the content digest the source pins (BND.32). The reason, otherwise.
fn reserved_write(
    stored: &Transaction,
    name: &str,
    value: &RecordValue,
    records: &BTreeMap<String, WorkspaceRecord>,
) -> Result<(), String> {
    let grip = name.starts_with(GRIP_PREFIX) || stored.row == GRIP_REBIND_ROW;
    let saved = name.starts_with(MACRO_PREFIX) || stored.row == MACRO_SAVE_ROW;
    if !grip && !saved {
        return Ok(());
    }
    let shell = matches!(stored.caller, Caller::Shell);
    let admitted = if grip {
        shell
            && stored.row == GRIP_REBIND_ROW
            && name == SLOT_MAP_RECORD
            && value.provenance == SLOT_MAP_PROVENANCE
            && SlotMap::from_payload(&value.payload).is_some()
    } else {
        let read = Macro::from_payload(&value.payload);
        let block = |read: &Macro| match read.stored() {
            None => value.provenance == MACRO_PROVENANCE,
            Some((record, digest)) => {
                let held = records.get(record).and_then(|record| record.value.as_ref());
                held.is_some_and(|held| held.digest() == digest && held.provenance == value.provenance)
            }
        };
        shell
            && stored.row == MACRO_SAVE_ROW
            && stored.confirm
            && read.is_some_and(|read| macro_record(&read.name) == name && block(&read))
    };
    match admitted {
        true => Ok(()),
        false => Err(format!("a write of `{name}` under `{}` that no rebind or macro save makes", stored.row)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::{MemoryStore, Shell};

    const SECRET: [u8; 32] = [7; 32];
    const TOOL: &str = "tool.propose";

    fn layer() -> (CommandLayer, Shell) {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
        let k = Arc::new(Kernel::load(&assets).expect("the model in assets/ loads"));
        let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
        let table = Table::h08().with_drafts();
        let mut layer =
            CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
        let overlay = Overlay { module: "tools/propose".into(), entry: "run".into() };
        layer.register_overlay(TOOL, Kind::Query, &[], overlay).unwrap();
        layer.set_program_capacity(3);
        (layer, Shell::new(Secret::fixed(SECRET), "shell"))
    }

    /// The tool's row, invoked on `caller`'s channel; the host takes the job.
    fn tool(layer: &mut CommandLayer, caller: &Caller, command: &str) -> ProgramJob {
        let invocation = Invocation {
            command: command.to_owned(),
            row: TOOL.to_owned(),
            args: json::object([]),
            token: None,
            epoch: layer.revision().epoch,
            budget: Some(Budget::CEILING),
        };
        let receipt = layer.invoke(caller, invocation);
        assert!(matches!(receipt.status, Status::Running(_)), "{caller:?}: {:?}", receipt.status);
        layer.take_program_job().expect("the tool's job")
    }

    #[test]
    fn a_run_name_names_a_job_only_in_its_one_form() {
        assert_eq!(ProgramJob::job_of("job-0"), Some(0));
        assert_eq!(ProgramJob::job_of("job-17"), Some(17));
        for name in
            ["job-", "job-+1", "job-01", "job--1", "job-1 ", " job-1", "Job-1", "job-1x", "job-18446744073709551616"]
        {
            assert_eq!(ProgramJob::job_of(name), None, "{name:?}");
        }
        for name in ["shell", "runtime/job-1", "tool-run", ""] {
            assert_eq!(ProgramJob::job_of(name), None, "{name:?}");
        }
    }

    #[test]
    fn a_run_is_inside_a_solver_command_only_while_its_chain_reaches_the_shell() {
        let (mut layer, mut shell) = layer();
        let receipt = shell.invoke(&mut layer, TOOL, json::object([])).unwrap();
        assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
        let outer = layer.take_program_job().expect("the solver's tool job");
        assert!(layer.inside_solver_command(&outer.run()));

        // A job the solver's run invoked is inside too; a job a stray run invoked is not.
        let inner = tool(&mut layer, &Caller::Runtime { run: outer.run() }, "inner/1");
        assert!(layer.inside_solver_command(&inner.run()));
        let stray = tool(&mut layer, &Caller::Runtime { run: "stray".into() }, "stray/1");
        assert!(!layer.inside_solver_command(&stray.run()));
        assert!(!layer.inside_solver_command("stray"));
        assert!(!layer.inside_solver_command("job-999"), "a job the layer never admitted");

        // A chain that does not get older is not followed.
        let older = inner.job.min(outer.job);
        layer.running.get_mut(&older).expect("running").caller = Caller::Runtime { run: inner.run() };
        assert!(!layer.inside_solver_command(&inner.run()), "a cycle is not inside");
        layer.running.get_mut(&older).expect("running").caller = Caller::Shell;

        // When the host reports the solver's job, its run and the run it invoked are no longer inside.
        layer.finish_program_job(outer.job, Ok(Value::Null)).expect("reported");
        assert!(!layer.inside_solver_command(&outer.run()));
        assert!(!layer.inside_solver_command(&inner.run()));
    }
}
