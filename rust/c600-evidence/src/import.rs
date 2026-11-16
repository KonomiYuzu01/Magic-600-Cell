//! EVD.52's four rules for records on file (V14, P13, K1, H1) and EVD.47's import values, with their limits. This is
//! not the label-table importer: reading the corpus from Git blobs (EVD.48), the table itself (EVD.49) and the
//! whole-record rules (EVD.51) are not implemented here.

use crate::error::{EvidenceError, Inconsistency};
use crate::graph::{Graph, NodeId};
use crate::model::{Check, CheckFact, CheckMode, Covers, Form, Key, Statement};

/// A historical check on record: a reviewer record or a program checker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoricalCheck {
    /// A reviewer record, by its reviewer name.
    Reviewer(String),
    /// A program checker such as `replay_word.py`.
    Program(String),
}

/// V14: a historical `*-independent*` reviewer record maps to model review with a fresh exact audit. It never maps to
/// `independent checker`, which stays for a 1.0 checker under KN6, and the rule does not apply to program checkers.
/// `None` means V14 sets nothing.
pub fn v14(record: &HistoricalCheck) -> Option<(Check, CheckMode)> {
    match record {
        HistoricalCheck::Reviewer(name) if name.contains("-independent") => {
            Some((Check::ModelReview, CheckMode::FreshExactAudit))
        }
        _ => None,
    }
}

/// P13: a check mode set from a key's name. Draft vocabulary: `replay` reads actual replay; `reused` or `transport`
/// reads reused through exact transport; `checked`, `verified`, `recomputed` or `audit` reads fresh exact audit;
/// anything else sets no mode.
pub fn p13_mode(key_name: &str) -> Option<CheckMode> {
    let k = key_name.to_ascii_lowercase();
    if k.contains("replay") {
        Some(CheckMode::ActualReplay)
    } else if k.contains("reused") || k.contains("transport") {
        Some(CheckMode::ReusedExactTransport)
    } else if ["checked", "verified", "recomputed", "audit"].iter().any(|w| k.contains(w)) {
        Some(CheckMode::FreshExactAudit)
    } else {
        None
    }
}

/// P13's check fact. Its limit: a mode read from a key name never creates a transport edge.
pub fn p13_check_fact(key_name: &str, tool: &str) -> Option<CheckFact> {
    p13_mode(key_name).map(|mode| CheckFact {
        tool: tool.to_string(),
        common_base: Vec::new(),
        kind: key_name.to_string(),
        mode,
        transport: None,
    })
}

/// K1: a row may leave its check to the enclosing record, only when the enclosing record's check covers the row;
/// otherwise the check reads none. `enclosing` is the enclosing record's check and whether it covers the row.
pub fn k1_check(own: Option<&Check>, enclosing: Option<(&Check, bool)>) -> Check {
    match (own, enclosing) {
        (Some(c), _) => c.clone(),
        (None, Some((c, true))) => c.clone(),
        _ => Check::None,
    }
}

/// H1: a reachability row maps to open on one finite instance, as a claim of its own. It registers the statement and
/// the key's OR node and adds no OR child, so it can discharge nothing. A row whose scope is not one instance is
/// refused as I7 (c) (draft).
pub fn h1_reachability_row(g: &mut Graph, statement: Statement, key: &Key) -> Result<NodeId, EvidenceError> {
    if key.scope.covers != Covers::OneInstance || key.statement != statement.id {
        return Err(EvidenceError::inconsistent(
            vec![Inconsistency::I7],
            "H1 maps a reachability row to one finite instance of its own statement",
        ));
    }
    let statement = if statement.form.is_none() { statement.form(Form::Finite) } else { statement };
    g.add_statement(statement)?;
    Ok(g.root(key))
}
