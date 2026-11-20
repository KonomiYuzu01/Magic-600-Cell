//! KN5.6's availability table (LP2 draft, L13 (d) of the KN5 attack leads; its final form stays with EN4's
//! command-table freeze): one answer per binding the shell registers, stamped with the revision, KN5.25's generation
//! and the digest of the context and effective grants, all read at one moment (LI.19).
//! [`crate::CommandLayer::availability_table`] builds it from the layer's existing hints, so a palette reads it once
//! per generation and answers each keystroke from it with no engine call.

use std::collections::BTreeMap;

use crate::receipt::Revision;
use crate::table::Binding;
use crate::token::args_digest;

/// A binding's key in the table: the row and the digest of its absolute arguments, as a token binds them
/// ([`args_digest`]), or `None` for arguments a program computes. Two bindings of one row with other arguments have
/// other keys, so they never share an answer (LI.19).
pub type AvailabilityKey = (String, Option<String>);

/// KN5.6's table: `available` is `Ok(())`, `unavailable(Code)` is `Err(code)`. Like every hint it grants nothing: an
/// invocation checks its actual arguments and the current inputs again, so a table one generation old can mislead a
/// display and never a commit (KN5.25).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Availability {
    /// The revision the answers were computed at.
    pub revision: Revision,
    /// KN5.25's generation at the same moment; a newer generation replaces this table.
    pub generation: u64,
    /// The digest of the caller's context and effective grants.
    pub context: String,
    /// One answer per registered binding.
    pub rows: BTreeMap<AvailabilityKey, Result<(), &'static str>>,
}

impl Availability {
    /// The key of `row` bound as `binding`.
    pub fn key(row: &str, binding: &Binding) -> AvailabilityKey {
        let args = match binding {
            Binding::Static(args) => Some(args_digest(args)),
            Binding::Computed => None,
        };
        (row.to_owned(), args)
    }

    /// The table's answer for `row` bound as `binding`, or `None` when the shell did not register that binding: a
    /// palette's lookup, which reaches no engine (KN5.6). A binding the table lacks is asked with LI.18's hint:
    /// [`crate::CommandLayer::availability_for`] for static arguments, [`crate::CommandLayer::availability`] for
    /// computed ones.
    pub fn answer(&self, row: &str, binding: &Binding) -> Option<Result<(), &'static str>> {
        self.rows.get(&Self::key(row, binding)).copied()
    }
}
