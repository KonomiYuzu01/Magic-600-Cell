//! Step 9's durable transaction (chapter 08 section 7; KN5.16).
//!
//! The command layer writes each commit, undo and redo as one [`Transaction`] and adopts its result only when the
//! store answers [`Fate::Durable`]. LP2's session store (M6) implements [`Store`] once it is merged on main; until
//! then [`MemoryStore`] keeps the transactions in memory and can be told the fate of the next ones, for the rollback and
//! unknown-outcome rehearsals.

use std::collections::VecDeque;

use crate::receipt::{Error, Revision};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionKind {
    Commit,
    Undo,
    Redo,
}

/// One durable action: the journal event, its action order and the receipt fields that must agree (R-06).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub kind: TransactionKind,
    pub event: String,
    /// The action-order number the action takes (LI.9).
    pub order: u64,
    /// For an undo or redo, the order of the action it inverts or repeats.
    pub target: Option<u64>,
    pub command: String,
    pub row: String,
    pub request: String,
    /// `Digest<literal word>` of what the action applies.
    pub literal: String,
    pub pre: String,
    pub post: String,
    /// The revision the action produces.
    pub revision: Revision,
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

    /// Whether the event is durable: read at a restart to settle a transaction whose fate was unknown.
    fn durable(&self, event: &str) -> bool;
}

/// The in-memory stand-in for M6's store.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    /// The durable transactions in order.
    pub log: Vec<Transaction>,
    /// Fates to report for the next transactions, in order; durable once empty.
    pub inject: VecDeque<Fate>,
    /// Whether an injected unknown fate still writes the transaction, as a write whose acknowledgement was lost.
    pub unknown_writes: bool,
}

impl Store for MemoryStore {
    fn transact(&mut self, transaction: &Transaction) -> Fate {
        let fate = self.inject.pop_front().unwrap_or(Fate::Durable);
        if fate == Fate::Durable || (fate == Fate::Unknown && self.unknown_writes) {
            self.log.push(transaction.clone());
        }
        fate
    }

    fn durable(&self, event: &str) -> bool {
        self.log.iter().any(|t| t.event == event)
    }
}
