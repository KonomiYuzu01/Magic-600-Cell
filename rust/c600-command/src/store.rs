//! Step 9's durable transaction (chapter 08 section 7; KN5.16) and the reopen (KN5.23).
//!
//! The command layer writes each durable action as one [`Transaction`] and adopts its result only when the store
//! answers [`Fate::Durable`]. The transaction carries the action's effect and its terminal receipt
//! ([`Transaction::receipt`] is a function of its fields), so both land in one write (KN5.23). [`Reopen::history`]
//! hands the layer the session's [`Genesis`] and every durable transaction in order, and
//! [`crate::CommandLayer::open`] replays them through the kernel. LP2's session store implements both traits over KN4's
//! journal (`c600_session::JournalStore`); [`MemoryStore`] keeps them in memory and can be told the fate of the next
//! ones, for the rollback and unknown-outcome rehearsals.
//!
//! A record write and the undo or redo of one carry the record's value after the action ([`Transaction::record`]). The
//! journal form names `record` only for them, so a journal written before KN5's stage 3 reads unchanged.

use std::collections::VecDeque;

use c600_kernel::json::{self, Value};
use c600_kernel::{IdentityBlock, Puzzle};

use crate::layer::Caller;
use crate::receipt::{CommitRecord, Completion, CompletionKind, Error, Receipt, ResultRef, Revision, Status};
use crate::workspace::RecordWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionKind {
    Commit,
    Undo,
    Redo,
    /// A change of the protected set through `protection.orbit.set`.
    Protection,
    /// A language-defined workspace row's completion (KN5.23).
    Workspace,
    /// A workspace record's write through `workspace.set` (KN5's stage 3).
    Record,
}

impl TransactionKind {
    pub fn name(self) -> &'static str {
        match self {
            TransactionKind::Commit => "commit",
            TransactionKind::Undo => "undo",
            TransactionKind::Redo => "redo",
            TransactionKind::Protection => "protection",
            TransactionKind::Workspace => "workspace",
            TransactionKind::Record => "record",
        }
    }

    pub fn from_name(name: &str) -> Option<TransactionKind> {
        [
            TransactionKind::Commit,
            TransactionKind::Undo,
            TransactionKind::Redo,
            TransactionKind::Protection,
            TransactionKind::Workspace,
            TransactionKind::Record,
        ]
        .into_iter()
        .find(|kind| kind.name() == name)
    }
}

/// One durable action: the journal event, its action order, its effect and the receipt fields that must agree (R-06).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub kind: TransactionKind,
    /// `e<order>`.
    pub event: String,
    /// The action-order number the action takes (LI.9).
    pub order: u64,
    /// For an undo or redo, the order of the action it inverts or repeats.
    pub target: Option<u64>,
    /// Who invoked. The receipt's record is keyed by the caller's command-ID space and the command ID (draft).
    pub caller: Caller,
    pub command: String,
    pub row: String,
    pub request: String,
    /// `Digest<literal word>` of what the action applies; empty for an action that moves no piece.
    pub literal: String,
    pub pre: String,
    pub post: String,
    /// The revision the action produces.
    pub revision: Revision,
    /// A commit's recipe in the kernel's JSON steps; `null` for every other kind (the move tree keeps it).
    pub recipe: Value,
    /// The protected orbits after the action.
    pub orbits: Vec<usize>,
    /// Whether the solver confirmed a release (LI.26).
    pub confirm: bool,
    /// A workspace completion's job.
    pub job: Option<u64>,
    /// A workspace completion's result digest.
    pub result: Option<String>,
    /// A record write's record and value, or for the undo or redo of one the record's value after the inverse or the
    /// repeat. `None` for every other action.
    pub record: Option<RecordWrite>,
}

impl Transaction {
    /// The terminal receipt the action answers: `committed` for a commit, undo or redo, `completed(workspace)` for a
    /// protection change, a record write or a workspace row. It is a function of the transaction, so it is durable with
    /// the effect (KN5.23).
    pub fn receipt(&self) -> Receipt {
        let status = match self.kind {
            TransactionKind::Commit | TransactionKind::Undo | TransactionKind::Redo => {
                Status::Committed(CommitRecord {
                    event: self.event.clone(),
                    pre: self.pre.clone(),
                    post: self.post.clone(),
                    revision: self.revision.clone(),
                })
            }
            TransactionKind::Protection | TransactionKind::Workspace | TransactionKind::Record => {
                Status::Completed(Completion {
                    kind: CompletionKind::Workspace,
                    revision: self.revision.clone(),
                    result: self.result.clone().map(|digest| ResultRef { job: self.job, digest }),
                })
            }
        };
        Receipt { command: self.command.clone(), request: self.request.clone(), job: self.job, status }
    }

    /// The journal form (draft). `record` appears only when the action carries a record write.
    pub fn to_json(&self) -> Value {
        let nat = |n: u64| Value::Int(n as i64);
        let text = |s: &str| Value::String(s.to_owned());
        let mut value = json::object([
            ("kind", text(self.kind.name())),
            ("event", text(&self.event)),
            ("order", nat(self.order)),
            ("target", self.target.map_or(Value::Null, nat)),
            ("caller", self.caller.to_json()),
            ("command", text(&self.command)),
            ("row", text(&self.row)),
            ("request", text(&self.request)),
            ("literal", text(&self.literal)),
            ("pre", text(&self.pre)),
            ("post", text(&self.post)),
            ("revision", self.revision.to_json()),
            ("recipe", self.recipe.clone()),
            ("orbits", Value::Array(self.orbits.iter().map(|&orbit| Value::Int(orbit as i64)).collect())),
            ("confirm", Value::Bool(self.confirm)),
            ("job", self.job.map_or(Value::Null, nat)),
            ("result", self.result.clone().map_or(Value::Null, Value::String)),
        ]);
        if let (Value::Object(fields), Some(record)) = (&mut value, &self.record) {
            fields.insert("record".to_owned(), record.to_json());
        }
        value
    }

    /// Reads the journal form strictly: every field, no other, and `record` only as a record write.
    pub fn from_json(value: &Value) -> Option<Transaction> {
        let object = value.as_object()?;
        let record = match value.get("record") {
            None => None,
            Some(record) => Some(RecordWrite::from_json(record)?),
        };
        (object.len() == 17 + usize::from(record.is_some())).then_some(())?;
        let text = |key: &str| value.get(key)?.as_str().map(str::to_owned);
        let nat = |key: &str| value.get(key)?.as_i64().and_then(|n| u64::try_from(n).ok());
        let optional = |key: &str| match value.get(key)? {
            Value::Null => Some(None),
            Value::Int(n) => u64::try_from(*n).ok().map(Some),
            _ => None,
        };
        let orbits = value.get("orbits")?.as_array()?;
        let orbits = orbits.iter().map(|o| o.as_i64().and_then(|n| usize::try_from(n).ok())).collect::<Option<_>>()?;
        let result = match value.get("result")? {
            Value::Null => None,
            Value::String(digest) => Some(digest.clone()),
            _ => return None,
        };
        let Value::Bool(confirm) = value.get("confirm")? else { return None };
        Some(Transaction {
            kind: TransactionKind::from_name(value.get("kind")?.as_str()?)?,
            event: text("event")?,
            order: nat("order")?,
            target: optional("target")?,
            caller: Caller::from_json(value.get("caller")?)?,
            command: text("command")?,
            row: text("row")?,
            request: text("request")?,
            literal: text("literal")?,
            pre: text("pre")?,
            post: text("post")?,
            revision: Revision::from_json(value.get("revision")?)?,
            recipe: value.get("recipe")?.clone(),
            orbits,
            confirm: *confirm,
            job: optional("job")?,
            result,
            record,
        })
    }
}

/// The identity block's canonical JSON (KN0.1). A token binds its digest (L9).
pub fn identity_json(block: &IdentityBlock) -> Value {
    let optional = |field: &Option<String>| field.clone().map_or(Value::Null, Value::String);
    let puzzle = match block.puzzle {
        Puzzle::Full => "full",
        Puzzle::Jumbling => "jumbling",
    };
    json::object([
        ("puzzle", Value::String(puzzle.into())),
        ("model_id", Value::String(block.model_id.clone())),
        ("menu_id", optional(&block.menu_id)),
        ("menu_order", optional(&block.menu_order)),
        ("contract_rev", optional(&block.contract_rev)),
    ])
}

/// Reads [`identity_json`]'s form strictly.
pub fn identity_from_json(value: &Value) -> Option<IdentityBlock> {
    (value.as_object()?.len() == 5).then_some(())?;
    let optional = |key: &str| match value.get(key)? {
        Value::Null => Some(None),
        Value::String(text) => Some(Some(text.clone())),
        _ => None,
    };
    let puzzle = match value.get("puzzle")?.as_str()? {
        "full" => Puzzle::Full,
        "jumbling" => Puzzle::Jumbling,
        _ => return None,
    };
    Some(IdentityBlock {
        puzzle,
        model_id: value.get("model_id")?.as_str()?.to_owned(),
        menu_id: optional("menu_id")?,
        menu_order: optional("menu_order")?,
        contract_rev: optional("contract_rev")?,
    })
}

/// A session's start, written once when the layer creates it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Genesis {
    pub session: String,
    pub identity: IdentityBlock,
    /// The epoch the session was created in.
    pub epoch: u64,
    /// The start state's labels.
    pub labels: Vec<i32>,
    /// The start state's label digest.
    pub digest: String,
    /// The protected orbits at the start.
    pub orbits: Vec<usize>,
}

/// What a reopen reads: the genesis, every epoch a layer opened, and the durable transactions in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    pub genesis: Genesis,
    pub epochs: Vec<u64>,
    pub transactions: Vec<Transaction>,
    /// A record after the listed transactions that the store could not read: continuity is broken (KN5.23).
    pub damaged: Option<Error>,
}

/// What the store reports for one transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Written and synced.
    Durable,
    /// Not written; the error says why.
    RolledBack(Error),
    /// The store cannot say whether the write happened.
    Unknown,
}

pub trait Store {
    fn transact(&mut self, transaction: &Transaction) -> Fate;

    /// Whether the event is durable: read at a restart to settle a transaction whose fate was unknown. `None` when the
    /// store cannot tell yet, so the fate stays unknown.
    fn durable(&self, event: &str) -> Option<bool>;

    /// Records a new session's start. A store that keeps no history may ignore it.
    fn begin(&mut self, _genesis: &Genesis) -> Fate {
        Fate::Durable
    }

    /// Records that a layer opened the session in `epoch`, so no later open draws it again (L10).
    fn begin_epoch(&mut self, _epoch: u64) -> Fate {
        Fate::Durable
    }
}

/// A store a session can be reopened from.
pub trait Reopen: Store {
    fn history(&mut self) -> Result<History, Error>;
}

/// The in-memory store.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    /// The durable transactions in order.
    pub log: Vec<Transaction>,
    /// Fates to report for the next transactions, in order; durable once empty.
    pub inject: VecDeque<Fate>,
    /// Whether an injected unknown fate still writes the transaction, as a write whose acknowledgement was lost.
    pub unknown_writes: bool,
    pub genesis: Option<Genesis>,
    pub epochs: Vec<u64>,
}

impl Store for MemoryStore {
    fn transact(&mut self, transaction: &Transaction) -> Fate {
        let fate = self.inject.pop_front().unwrap_or(Fate::Durable);
        if fate == Fate::Durable || (fate == Fate::Unknown && self.unknown_writes) {
            self.log.push(transaction.clone());
        }
        fate
    }

    fn durable(&self, event: &str) -> Option<bool> {
        Some(self.log.iter().any(|t| t.event == event))
    }

    fn begin(&mut self, genesis: &Genesis) -> Fate {
        if self.genesis.is_some() || !self.log.is_empty() {
            return Fate::RolledBack(Error::new(crate::receipt::code::SESSION_EXISTS, "the store holds a session"));
        }
        self.genesis = Some(genesis.clone());
        Fate::Durable
    }

    fn begin_epoch(&mut self, epoch: u64) -> Fate {
        self.epochs.push(epoch);
        Fate::Durable
    }
}

impl Reopen for MemoryStore {
    fn history(&mut self) -> Result<History, Error> {
        let Some(genesis) = self.genesis.clone() else {
            return Err(Error::new(crate::receipt::code::UNKNOWN_SESSION, "the store holds no session"));
        };
        Ok(History { genesis, epochs: self.epochs.clone(), transactions: self.log.clone(), damaged: None })
    }
}
