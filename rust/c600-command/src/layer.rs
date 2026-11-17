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
//! its draft digest (KN0.8), after the host's provenance reader admits the record's block (KN5.26). KN5's stage 5
//! ([`crate::frame`]): [`CommandLayer::present`] reads the adapter's frame reports, only a frame that shows the pending
//! preview through the built-in chain presents it, and a word commit's token binds that frame's stamp (ONE.41).
//!
//! [`CommandLayer::open`] reopens a session from a [`Reopen`] store: it replays every durable transaction through the
//! kernel, restores the revision, the protected set, the move tree and the durable receipts, and draws a fresh random
//! epoch (KN5.23, L10).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use c600_kernel::json::{self, Value};
use c600_kernel::{
    Derivation, Effect, Kernel, ProtectedSet, Protection, ProtectionMode, Recipe, Snapshot, Step, SupportRow, Verdict,
};

use crate::frame::{FrameChain, FrameStamp, PresentedFrame, ShownPreview};
use crate::receipt::{
    code, digest, ActionHint, CancelReason, Completion, CompletionKind, Direction, Error, PreviewDeps, Receipt,
    ResultRef, Revision, Status,
};
use crate::store::{
    identity_json, policy_from_name, policy_name, Fate, Genesis, MemoryStore, Reopen, Store, Transaction,
    TransactionKind,
};
use crate::table::{Kind, Lane, Overlay, Row, Table};
use crate::token::{args_digest, fresh_epoch, Secret, Token, TokenBinding};
use crate::workspace::{Provenance, RecordRead, RecordValue, RecordWrite, ResolvedDraft, WorkspaceRecord, RECORD_ROW};

/// The mode of every revision here: the jumbling engine joins when M5 is on main.
pub const MODE: &str = "jumble_off";
/// H-08's row that sets the protection policy, Net or Strict.
pub const POLICY_ROW: &str = "protection.policy.set";
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

impl Caller {
    /// The caller's command-ID space (ruling 4): a record is keyed by it and the command ID, so the runtime's
    /// IDs never reach the shell's records.
    pub fn scope(&self) -> &'static str {
        match self {
            Caller::Shell => "shell",
            Caller::Runtime { .. } => "runtime",
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
}

enum JobKind {
    Preview { recipe: Recipe, draft: String, resolved: Option<ResolvedDraft>, caller: Caller },
    Program(ProgramJob),
}

struct Job {
    id: u64,
    command: String,
    /// The invoking caller's command-ID space.
    scope: &'static str,
    lane: Lane,
    kind: JobKind,
}

/// A node of the move tree: the start (node 0) or a commit, keyed by its action order. The journal keeps the recipe and
/// the digest, not the state.
#[derive(Clone, Debug)]
struct Node {
    parent: u64,
    recipe: Option<Recipe>,
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
    /// A protection-policy change: the policy before and after it.
    Policy {
        before: ProtectionMode,
        after: ProtectionMode,
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
    /// A workspace completion, which no undo reaches.
    None,
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

    /// A stored transaction's, for its replay: a replayed release needs no new confirmation.
    fn replay(transaction: &Transaction) -> Meta {
        Meta {
            caller: transaction.caller.clone(),
            command: transaction.command.clone(),
            row: transaction.row.clone(),
            request: transaction.request.clone(),
            confirm: true,
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
type Key = (&'static str, String);

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
        && epochless(&planned.revision) == epochless(&stored.revision)
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
    /// Every epoch of the session, so no open or restart draws one again (L10).
    epochs: BTreeSet<u64>,
    jobs_issued: u64,
    jobs: VecDeque<Job>,
    running: BTreeMap<u64, ProgramJob>,
    program_capacity: usize,
    slot: Option<Preview>,
    /// The newest frame the adapter reported (RM.26).
    frame: Option<PresentedFrame>,
    /// The pending preview's presentation run: its ID and the stamp of the frame that started showing it (ONE.41).
    run: Option<(u64, FrameStamp)>,
    /// The workspace records by name (KN5's stage 3).
    records: BTreeMap<String, WorkspaceRecord>,
    /// The host's provenance reader (PS1.16, KN5.26). Without one, record writes and record previews are refused.
    provenance: Option<Box<dyn Provenance>>,
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
        let start = Node { parent: 0, recipe: None, digest: initial.digest().to_owned(), depth: 0 };
        CommandLayer {
            kernel,
            table: table.with_drafts(),
            secret,
            store,
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
            jobs_issued: 0,
            jobs: VecDeque::new(),
            running: BTreeMap::new(),
            program_capacity: 1,
            slot: None,
            frame: None,
            run: None,
            records: BTreeMap::new(),
            provenance: None,
            history: BTreeMap::new(),
            used: BTreeSet::new(),
            receipts: BTreeMap::new(),
            lost: BTreeMap::new(),
            retained: BTreeMap::new(),
            commit_held: false,
            recovery: None,
            limbo: None,
            generation: 0,
        }
    }

    /// A new session `session` at `initial` with the protected set `protected`, in epoch 1. The store records the
    /// session's start ([`Store::begin`]); a store that does not answer durably leaves the layer in read-only recovery.
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
        let genesis = Genesis {
            session: session.to_owned(),
            identity: kernel.identity(),
            epoch: 1,
            labels: initial.labels().to_vec(),
            digest: initial.digest().to_owned(),
            orbits: protected.orbits().to_vec(),
        };
        let mut layer = Self::empty(kernel, table, secret, store, revision, initial, protected);
        let fate = layer.store.begin(&genesis);
        layer.settle_fate(fate, "the session's start");
        layer
    }

    /// Reopens the session a [`Reopen`] store holds (KN5.23). Every durable transaction is replayed through the kernel
    /// from the session's start and must reach its recorded digests and revision; the revision, the protected set, the
    /// move tree, the stacks and the durable receipts are restored, and the layer starts a fresh random epoch that no
    /// earlier open of the session drew (L10). A transaction that does not replay, or a record the store could not
    /// read, leaves the layer in read-only recovery after the last transaction that replayed. A command ID the store
    /// does not hold answers `unknown_outcome` ([`CommandLayer::kernel_outcome`], LI.30).
    pub fn open(kernel: Arc<Kernel>, table: Table, secret: Secret, mut store: S) -> Result<CommandLayer<S>, Error>
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
        let mut layer = Self::empty(kernel, table, secret, store, revision, initial, protected);
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
        if stored.order != self.order + 1 {
            return Err(broken("the action order is broken"));
        }
        let meta = Meta::replay(stored);
        let planned = match stored.kind {
            TransactionKind::Commit => {
                let recipe = self.kernel.recipe(&stored.recipe)?;
                let none = ProtectedSet::none();
                match self.kernel.derive(&self.current, &recipe, &none).map_err(kernel_error)? {
                    Derivation::Derived(d) => self.commit_adoption(meta, recipe, d.state, d.word, None),
                    Derivation::Refused(r) => return Err(r.refusal.into()),
                }
            }
            TransactionKind::Undo | TransactionKind::Redo => {
                let (direction, stack) = match stored.kind {
                    TransactionKind::Undo => (Direction::Undo, &self.undo),
                    _ => (Direction::Redo, &self.redo),
                };
                if stored.target != stack.last().copied() {
                    return Err(broken("the action does not invert or repeat the top of its stack"));
                }
                self.plan_step(meta, direction, &ProtectedSet::none())?
            }
            TransactionKind::Protection => {
                let orbits: Vec<i64> = stored.orbits.iter().map(|&orbit| orbit as i64).collect();
                let set = self.kernel.protected_set(&orbits)?;
                self.plan_orbits(meta, set)?
            }
            TransactionKind::Workspace => {
                let (Some(job), Some(result)) = (stored.job, stored.result.clone()) else {
                    return Err(broken("a workspace completion names no job and result"));
                };
                self.jobs_issued = self.jobs_issued.max(job);
                self.workspace_adoption(meta, job, result)
            }
            // The record's block was read when the record was written (PS1.16), so the replay needs no reader.
            TransactionKind::Record => match stored.record.clone() {
                Some(RecordWrite { name, value: Some(value) }) => self.plan_record(meta, name, value),
                _ => return Err(broken("a record write names no record and value")),
            },
            TransactionKind::Policy => match stored.policy {
                Some(policy) => self.plan_policy(meta, policy),
                None => return Err(broken("a policy change names no policy")),
            },
        };
        if !agrees(&planned.transaction, stored) {
            return Err(broken("the replay does not reach the recorded action"));
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

    pub fn snapshot(&self) -> &Snapshot {
        &self.current
    }

    pub fn protected(&self) -> &ProtectedSet {
        &self.protected
    }

    /// The protection policy in force, Net or Strict: Net from a session's start, then what the last
    /// `protection.policy.set`, or the undo or redo of one, left.
    pub fn policy(&self) -> ProtectionMode {
        self.policy
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
            Direction::Undo => &self.undo,
            Direction::Redo => &self.redo,
        };
        stack.last().map(|&target| ActionHint { order: self.order, target })
    }

    /// The shell's outcome lookup: [`CommandLayer::kernel_outcome`] for a shell command ID, or `None` when the shell's
    /// command-ID space never saw the ID in this layer's life.
    pub fn outcome(&self, command: &str, request: &str) -> Option<Receipt> {
        let scope = Caller::Shell.scope();
        let key = (scope, command.to_owned());
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
        if let Some(receipt) = self.retained.get(&(scope, request.to_owned())).filter(|r| r.command == command) {
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

    /// Applies a newer frame report to the pending preview and its presentation run.
    fn observe(&mut self, frame: &PresentedFrame) {
        let Some(p) = self.slot.as_mut() else {
            self.run = None;
            return;
        };
        if frame.presents(&ShownPreview::of(p)) {
            if p.state == PreviewState::Ready {
                p.state = PreviewState::Presented;
                self.generation += 1;
            }
            let continued =
                self.run.as_ref().is_some_and(|(run, first)| *run == p.id && first.same_surface(&frame.stamp));
            if !continued {
                self.run = Some((p.id, frame.stamp.clone()));
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
        let (Some((run, first)), Some(newest)) = (&self.run, &self.frame) else {
            return false;
        };
        *run == id
            && first.same_surface(stamp)
            && newest.stamp.same_surface(stamp)
            && first.sequence <= stamp.sequence
            && stamp.sequence <= newest.stamp.sequence
    }

    /// Gives the layer the host's provenance reader (PS1.16, KN5.26). Without one, `workspace.set` and a draft that
    /// names a record are refused.
    pub fn set_provenance(&mut self, reader: Box<dyn Provenance>) {
        self.provenance = Some(reader);
        self.generation += 1;
    }

    /// The workspace records by name, each with its version and value (KN5's stage 3).
    pub fn records(&self) -> &BTreeMap<String, WorkspaceRecord> {
        &self.records
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
    /// so no later action takes its place in the order; a later restart settles it.
    pub fn restart(&mut self, secret: Secret) {
        self.secret = secret;
        self.used.clear();
        self.retained.clear();
        if let Some(p) = self.slot.take() {
            self.history.insert(p.id, PreviewState::Discarded);
        }
        self.frame = None;
        self.run = None;
        self.jobs.clear();
        self.running.clear();
        self.commit_held = false;
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
            "protection.orbit.set" | POLICY_ROW => Ok(()),
            RECORD_ROW => match self.provenance {
                Some(_) => Ok(()),
                None => {
                    Err(Error::new(code::ROW_NOT_BUILT, format!("`{RECORD_ROW}` needs the host's provenance reader")))
                }
            },
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

    /// The request digest: the envelope without the token's authenticator (LI.5) and the caller the host stamped
    /// (ruling 4).
    fn request(&self, caller: &Caller, invocation: &Invocation) -> String {
        digest(&json::object([
            ("session", Value::String(self.revision.session.clone())),
            ("epoch", Value::Int(invocation.epoch as i64)),
            ("caller", caller.to_json()),
            ("command", Value::String(invocation.command.clone())),
            ("row", Value::String(invocation.row.clone())),
            ("args", invocation.args.clone()),
            ("token", invocation.token.as_ref().map_or(Value::Null, |t| t.binding.to_json())),
        ]))
    }

    /// The envelope's full digest: the request digest and the token's authenticator (ruling 4).
    fn envelope(invocation: &Invocation, request: &str) -> String {
        let authenticator = invocation.token.as_ref().map_or(Value::Null, |t| Value::String(t.authenticator()));
        digest(&json::object([("request", Value::String(request.to_owned())), ("authenticator", authenticator)]))
    }

    /// Whether the runtime may invoke a row: never a commit or app row (KN5.15), not `protection.orbit.set`, whose
    /// release confirmation is the solver's, not `protection.policy.set`, since the protection a commit is judged under
    /// is the solver's too, and not `workspace.set`, which M6 does not grant a program (draft).
    fn runtime_may(row: &Row) -> bool {
        !matches!(row.kind, Kind::Commit | Kind::App)
            && row.id != "protection.orbit.set"
            && row.id != POLICY_ROW
            && row.id != RECORD_ROW
    }

    /// Whether the envelope authenticates: its row exists, its caller may invoke the row and hold what it holds, and
    /// its token, where the row takes one, verifies under this launch's secret (ruling 4).
    fn authentic(&self, caller: &Caller, invocation: &Invocation) -> bool {
        let Some(row) = self.table.row(&invocation.row) else {
            return false;
        };
        match (caller, &invocation.token) {
            (Caller::Runtime { .. }, Some(_)) => false,
            (Caller::Runtime { .. }, None) => Self::runtime_may(row) && !row.takes_token(),
            (Caller::Shell, None) => !row.takes_token(),
            (Caller::Shell, Some(token)) => row.takes_token() && token.verify(&self.secret),
        }
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
        let scope = caller.scope();
        let request = self.request(caller, &invocation);
        let full = Self::envelope(&invocation, &request);
        let retained = self.retained.get(&(scope, full.clone())).filter(|r| r.command == invocation.command);
        if let Some(receipt) = retained {
            return receipt.clone();
        }
        let key = (scope, invocation.command.clone());
        if self.receipts.contains_key(&key) || self.lost.contains_key(&key) {
            return self.kernel_outcome(caller, &invocation.command, &request);
        }
        let (job, status) = match self.dispatch(caller, &invocation, &request) {
            Ok(answer) => answer,
            Err(error) => (None, Status::Refused(error)),
        };
        if matches!(status, Status::Refused(_)) && !self.authentic(caller, &invocation) {
            let receipt = Receipt { command: invocation.command.clone(), request: full.clone(), job, status };
            self.retained.insert((scope, full), receipt.clone());
            return receipt;
        }
        let receipt = Receipt { command: invocation.command.clone(), request: request.clone(), job, status };
        let record = self.receipts.entry(key).or_insert(Record { request, receipt, durable: false });
        record.receipt.clone()
    }

    fn dispatch(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        if invocation.epoch != self.revision.epoch {
            return refused(code::EPOCH_CHANGED, "the envelope belongs to another engine epoch");
        }
        let Some(row) = self.table.row(&invocation.row).cloned() else {
            return refused(code::UNKNOWN_ROW, format!("no row `{}`", invocation.row));
        };
        if let Caller::Runtime { .. } = caller {
            if matches!(row.kind, Kind::Commit | Kind::App) {
                return refused(code::COMMIT_ROW_REFUSED, format!("the runtime may not invoke `{}` (KN5.15)", row.id));
            }
            if !Self::runtime_may(&row) {
                return refused(code::NOT_GRANTED, format!("the runtime may not invoke `{}`", row.id));
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
        // A row that confirms a release takes an optional boolean `confirm` beside its table arguments (draft; LI.26).
        let confirmable = row.has_gate("confirm") || row.action_guard.is_some();
        if confirmable && args.get("confirm").is_some_and(|confirm| !matches!(confirm, Value::Bool(_))) {
            return refused(code::ARGUMENT_FORM, "`confirm` is a boolean");
        }
        let names = args.keys().map(String::as_str).filter(|name| !(confirmable && *name == "confirm"));
        if !names.eq(row.plain_arguments()) {
            return refused(code::ARGUMENT_FORM, format!("`{}` takes {:?}", row.id, row.arguments));
        }
        if row.takes_token() != invocation.token.is_some() {
            return match invocation.token {
                None => refused(code::TOKEN_INVALID, format!("`{}` needs a user-action token", row.id)),
                Some(_) => refused(code::ARGUMENT_FORM, format!("`{}` takes no token", row.id)),
            };
        }
        match row.id.as_str() {
            "operation.preview" => self.admit_preview(caller, invocation),
            "operation.execute" => self.execute(caller, invocation, request),
            "operation.preview.cancel" => self.cancel(invocation),
            "session.undo" => self.undo(caller, invocation, request, Direction::Undo),
            "session.redo" => self.undo(caller, invocation, request, Direction::Redo),
            "protection.orbit.set" => self.set_orbits(caller, invocation, request),
            POLICY_ROW => self.set_policy(caller, invocation, request),
            RECORD_ROW => self.write_record(caller, invocation, request),
            _ if row.overlay.is_some() => self.admit_program(caller, invocation, &row),
            _ => refused(code::ROW_NOT_BUILT, format!("`{}` is not built in the headless layer", row.id)),
        }
    }

    fn next_job(&mut self) -> u64 {
        self.jobs_issued += 1;
        self.jobs_issued
    }

    // ----- operation.preview -----

    /// Admits a draft: `{"recipe": [...]}`, an inline recipe whose draft digest is the draft's digest, or
    /// `{"record": name}`, a `ResolvedDraftRef` to a workspace record that holds a word's letters, resolved now
    /// ([`CommandLayer::resolve_record`]). A refused draft takes no job and leaves the slot as it was (KN5.11).
    fn admit_preview(&mut self, caller: &Caller, invocation: &Invocation) -> Result<(Option<u64>, Status), Error> {
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

    /// Resolves a draft `{"record": name}` (KN0.8, KN5.26): the record must hold a list of letters, as in M6
    /// (`KN2:input.named_object_invalid`), and the host's provenance reader must admit the record's block in this
    /// session, or the draft is refused with the reader's error (`LG3:capability.not_granted` without a reader). The
    /// record is checked before the reader, so a session without a reader answers `named_object_invalid` for a record
    /// that holds no letters. The read set holds the record's content digest and version.
    fn resolve_record(&self, draft: &Value, name: &str) -> Result<ResolvedDraft, Error> {
        let record = self.records.get(name);
        let Some(value) = record.and_then(|record| record.value.as_ref()) else {
            return Err(Error::new(code::NAMED_OBJECT_INVALID, "no such record"));
        };
        let Some(letters) = value.letters() else {
            return Err(Error::new(code::NAMED_OBJECT_INVALID, "the record does not hold a list of letters"));
        };
        let Some(reader) = &self.provenance else {
            return Err(Error::new(code::NOT_GRANTED, "a record's word needs the host's provenance reader (KN5.26)"));
        };
        reader.admit(value.provenance.as_bytes())?;
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
            }
        }
        self.jobs = keep;
        ran
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
            self.settle(&(job.scope, job.command), Status::Cancelled(CancelReason::ByUser));
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
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
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
        let now = self.deps(&self.draft_now(p));
        if p.deps != now || bound.deps != now {
            return refused(code::REVISION_CHANGED, "the preview's dependencies changed");
        }
        if self.current.digest() != p.base.digest() {
            return refused(code::STATE_HASH_MISMATCH, "the head hash differs from the preview's start");
        }
        // Step 8 judges the operation again under the policy in force: Net its effect, Strict every executed prefix of
        // its recipe's literal expansion.
        let protection = match self.policy {
            ProtectionMode::Net => self.kernel.protection(&p.effect, &self.protected),
            ProtectionMode::Strict => self.kernel.strict_protection(&p.recipe, &self.protected)?,
        };
        let end = match (&p.end, protection.verdict) {
            (Some(end), Verdict::Clear) => end.clone(),
            _ => return refused(code::ORBIT_WOULD_MOVE, c600_kernel::PROTECTED_MESSAGE),
        };
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
        let node = Node { parent: self.head, recipe: Some(recipe.clone()), digest: post.clone(), depth };
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
        };
        let change = Change::Head { before: self.head, after: order };
        Adoption {
            transaction,
            state: Some(end),
            protected: None,
            head: order,
            tree: Tree::Push { node: Some((order, node)), change },
            preview,
        }
    }

    /// Steps 9 and 10: writes the action and adopts it only when the store answers that it is durable.
    fn transact(&mut self, adoption: Adoption) -> Result<(Option<u64>, Status), Error> {
        match self.store.transact(&adoption.transaction) {
            Fate::Durable => {
                let receipt = self.adopt(adoption);
                Ok((receipt.job, receipt.status))
            }
            Fate::RolledBack(error) => Ok((None, Status::Failed(error))),
            Fate::Unknown => {
                self.recovery = Some(Error::new(code::COMMIT_FATE, "the store cannot say whether the action landed"));
                self.limbo = Some(adoption);
                self.generation += 1;
                Ok((None, Status::UnknownOutcome))
            }
        }
    }

    /// Step 10: applies a durable action, stores its durable receipt and returns it. Only an action that moves the
    /// puzzle ends the pending preview (committed when it is the one committed, discarded otherwise) and cancels the
    /// jobs on the replaced revision, except file jobs (KN5.18).
    fn adopt(&mut self, adoption: Adoption) -> Receipt {
        let Adoption { transaction, state, protected, head, tree, preview } = adoption;
        let moved = state.is_some();
        if let Some(state) = state {
            self.current = state;
        }
        if let Some(protected) = protected {
            self.protected = protected;
        }
        self.revision = transaction.revision.clone();
        self.order = transaction.order;
        self.head = head;
        // A record write, or its undo or redo: the record's version advances and never goes back (KN0.8).
        if let Some(RecordWrite { name, value }) = &transaction.record {
            let record = self.records.entry(name.clone()).or_default();
            record.version += 1;
            record.value = value.clone();
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
            Tree::None => {}
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

    /// Ends what read the state an action replaced: the pending preview, and every job but file jobs (KN5.18).
    fn end_revision(&mut self, committed: Option<u64>) {
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
            if job.lane == Lane::Exclusive {
                self.history.insert(job.id, PreviewState::Discarded);
            }
            self.settle(&(job.scope, job.command), Status::Cancelled(CancelReason::ReplacedRevision));
        }
        self.jobs = keep;
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
        let recipe = |node: &u64| self.nodes[node].recipe.as_ref().expect("every node but the start is a commit");
        let mut steps = Vec::new();
        for node in &up {
            steps.extend(steps_json(recipe(node), true));
        }
        for node in down.iter().rev() {
            steps.extend(steps_json(recipe(node), false));
        }
        Ok(self.kernel.recipe(&Value::Array(steps))?)
    }

    /// The state at node `to`, derived from the current state along the path under `protected` in the policy in force
    /// (LI.26's normal permission path); it must reach the node's recorded digest. Returns the state and the path's
    /// literal digest.
    fn walk(&self, to: u64, protected: &ProtectedSet) -> Result<(Snapshot, String), Error> {
        if self.current.digest() != self.nodes[&self.head].digest {
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
    /// its path under `protected`, and a protection change that releases an orbit needs `confirm` (LI.26).
    fn plan_step(&self, meta: Meta, direction: Direction, protected: &ProtectedSet) -> Result<Adoption, Error> {
        let (stack, kind, tree) = match direction {
            Direction::Undo => (&self.undo, TransactionKind::Undo, Tree::Undo),
            Direction::Redo => (&self.redo, TransactionKind::Redo, Tree::Redo),
        };
        let Some(&target) = stack.last() else {
            return Err(Error::new(code::UNDO_SUPERSEDED, "no eligible target"));
        };
        let change = self.changes.get(&target).expect("a stacked action keeps its change").clone();
        let order = self.order + 1;
        let digest = self.current.digest().to_owned();
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
        };
        match change {
            Change::Head { before, after } => {
                let to = if direction == Direction::Undo { before } else { after };
                let (state, literal) = self.walk(to, protected)?;
                transaction.literal = literal;
                transaction.post = state.digest().to_owned();
                transaction.revision.head = format!("e{to}");
                transaction.revision.state += 1;
                transaction.revision.digest = state.digest().to_owned();
                Ok(Adoption { transaction, state: Some(state), protected: None, head: to, tree, preview: None })
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
                Ok(Adoption { transaction, state: None, protected: Some(set), head: self.head, tree, preview: None })
            }
            // A workspace inverse: the record takes its value before or after the write again, and its version and the
            // workspace revision advance (KN5.20, LI.26).
            Change::Record { name, before, after } => {
                let to = if direction == Direction::Undo { before } else { after };
                transaction.record = Some(RecordWrite { name, value: to });
                transaction.revision.workspace += 1;
                Ok(Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None })
            }
            // A workspace inverse: the policy before or after the change is in force again, and the workspace and
            // protection revisions advance, as for a protection change. H-08 gives the row no `confirm` gate.
            Change::Policy { before, after } => {
                let to = if direction == Direction::Undo { before } else { after };
                transaction.policy = Some(to);
                transaction.revision.workspace += 1;
                transaction.revision.protection += 1;
                Ok(Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None })
            }
        }
    }

    fn undo(
        &mut self,
        caller: &Caller,
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
        // The hint before the bound revisions, so a raced undo is `undo_superseded` (draft ruling 2).
        if self.undo_hint(direction) != Some(hint) {
            return refused(code::UNDO_SUPERSEDED, "the hint's order or target changed");
        }
        if binding.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        let protected = self.protected.clone();
        let adoption = self.plan_step(Meta::of(caller, invocation, request), direction, &protected)?;
        self.transact(adoption)
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
        let digest = self.current.digest().to_owned();
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
        };
        let change = Change::Protection { before: self.protected.orbits().to_vec(), after: to };
        let tree = Tree::Push { node: None, change };
        Ok(Adoption { transaction, state: None, protected: Some(set), head: self.head, tree, preview: None })
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
    /// the state and the preview slot stay. It is undoable as a workspace inverse. H-08 gives the row no `confirm`
    /// gate, so no direction needs a confirmation.
    fn plan_policy(&self, meta: Meta, policy: ProtectionMode) -> Adoption {
        let order = self.order + 1;
        let digest = self.current.digest().to_owned();
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
            confirm: false,
            job: None,
            result: None,
            record: None,
            policy: Some(policy),
        };
        let tree = Tree::Push { node: None, change: Change::Policy { before: self.policy, after: policy } };
        Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None }
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
        let adoption = self.plan_policy(Meta::of(caller, invocation, request), policy);
        self.transact(adoption)
    }

    /// A language-defined workspace row's completion: one durable action that advances the workspace revision and
    /// keeps the state, the preview slot and the jobs (KN5.23).
    fn workspace_adoption(&self, meta: Meta, job: u64, result: String) -> Adoption {
        let order = self.order + 1;
        let digest = self.current.digest().to_owned();
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
        };
        Adoption { transaction, state: None, protected: None, head: self.head, tree: Tree::None, preview: None }
    }

    // ----- workspace.set -----

    /// `workspace.set` (M6's record write): writes `payload` with its provenance block under `record` as one durable
    /// action whose receipt is `completed(workspace)`. The host's provenance reader checks the block first (PS1.16); a
    /// block it refuses, a missing reader or a bad argument writes nothing.
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
        let digest = self.current.digest().to_owned();
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
            confirm: false,
            job: None,
            result: None,
            record: Some(RecordWrite { name: name.clone(), value: Some(value.clone()) }),
            policy: None,
        };
        let tree = Tree::Push { node: None, change: Change::Record { name, before, after: Some(value) } };
        Adoption { transaction, state: None, protected: None, head: self.head, tree, preview: None }
    }

    // ----- language-defined rows -----

    fn admit_program(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        row: &Row,
    ) -> Result<(Option<u64>, Status), Error> {
        let overlay = row.overlay.clone().expect("an overlay row");
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

    /// Takes the next queued program job for the host to run.
    pub fn take_program_job(&mut self) -> Option<ProgramJob> {
        let position = self.jobs.iter().position(|job| matches!(job.kind, JobKind::Program(_)))?;
        let job = self.jobs.remove(position)?;
        let JobKind::Program(program) = job.kind else { unreachable!("the position names a program job") };
        self.running.insert(program.job, program.clone());
        Some(program)
    }

    /// Reports a program job's result: the host's interface. A job a commit or undo cancelled meanwhile stays
    /// cancelled, and its result is dropped (`None`). A workspace row's completion is one durable action, written with
    /// its receipt (KN5.23, L8); in read-only recovery it fails. Other completions stay transient.
    pub fn finish_program_job(&mut self, job: u64, result: Result<Value, Error>) -> Option<Receipt> {
        let program = self.running.remove(&job)?;
        let key = (program.caller.scope(), program.command.clone());
        let status = match result {
            Err(error) => Status::Failed(error),
            Ok(_) if program.kind == Kind::Workspace && self.recovery.is_some() => {
                Status::Failed(Error::new(code::READ_ONLY_RECOVERY, "the session is in read-only recovery"))
            }
            Ok(value) if program.kind == Kind::Workspace => {
                let request = self.receipts.get(&key).map(|record| record.request.clone()).unwrap_or_default();
                let meta = Meta {
                    caller: program.caller.clone(),
                    command: program.command.clone(),
                    row: program.row.clone(),
                    request,
                    confirm: false,
                };
                let adoption = self.workspace_adoption(meta, job, digest(&value));
                match self.transact(adoption) {
                    Ok((_, status)) => status,
                    Err(error) => Status::Failed(error),
                }
            }
            Ok(value) => {
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
                let result = Some(ResultRef { job: Some(job), digest: digest(&value) });
                Status::Completed(Completion { kind, revision: program.revision.clone(), result })
            }
        };
        self.settle(&key, status);
        self.generation += 1;
        self.receipts.get(&key).map(|record| record.receipt.clone())
    }
}
