//! `jumble.repair.list` (BND.2, the 17 October exception; JI.12, JI.33; H-08's row; draft): the repair list at a cap
//! the solver names, as a job on the exclusive lane ([`REPAIR_LIST_ROW`]).
//!
//! The list is the kernel's (`c600_jumble::observe::cuts::repair_list`, M8): its rows are the pairs (x, d) whose
//! repair lowers N, each with its y and z and the change of N, by x in pinned menu order, then d in cap-ID order. The
//! layer passes it on whole and adds one field, `statement`, which words what the list says ([`worded`]). Nothing here
//! chooses, ranks, filters or reorders a pair, and no job starts for a cap the invocation does not name.

use std::collections::BTreeMap;

use c600_kernel::json::Value;

use crate::receipt::{code, Error};

/// H-08's row that lists the repairs at a cap the solver names.
pub const REPAIR_LIST_ROW: &str = "jumble.repair.list";

/// The caps of the 600-cell: a `CapId` is 0 to 599, as the jumbling engine numbers them (JI.45).
const CAPS: i64 = 600;

/// The cap `args` names, after the forms of the row's arguments: `cap` is a `CapId` (an integer from 0 to 599) and
/// `max_chunk` a `PositiveNat` (an integer of at least 1); otherwise `KN5:input.argument_form`. `snapshot` is checked
/// by the layer, which knows the current revision. The headless layer has no stream, so `max_chunk` bounds nothing yet:
/// the list is delivered whole when the job completes (draft).
pub(crate) fn named_cap(args: &BTreeMap<String, Value>) -> Result<i64, Error> {
    let cap = match args.get("cap") {
        Some(Value::Int(cap)) if (0..CAPS).contains(cap) => *cap,
        _ => return Err(Error::new(code::ARGUMENT_FORM, "`cap` is a CapId, an integer from 0 to 599")),
    };
    if !matches!(args.get("max_chunk"), Some(Value::Int(n)) if *n >= 1) {
        return Err(Error::new(code::ARGUMENT_FORM, "`max_chunk` is a PositiveNat, an integer of at least 1"));
    }
    Ok(cap)
}

/// `list`, the kernel's answer at `cap`, with the field `statement` added: what the list says in words.
///
/// - A completed list with no rows says that no pair at that cap lowers N, as BND.2 states it.
/// - A cancelled or cut-off list did not complete, so it makes no claim about the pairs at that cap (JI.12).
/// - Otherwise `statement` is `null`: a completed list's rows speak for themselves, and an error has its `message`.
pub(crate) fn worded(list: Value, cap: i64) -> Value {
    let Value::Object(mut fields) = list else {
        return list;
    };
    let status = fields.get("status").and_then(Value::as_str).unwrap_or_default();
    let empty = fields.get("rows").and_then(Value::as_array).is_some_and(|rows| rows.is_empty());
    let statement = match status {
        "completed" if empty => Value::String(format!("No pair at cap {cap} lowers N.")),
        "cancelled" | "cut_off" => Value::String(format!(
            "The list at cap {cap} did not complete, so it says nothing about the pairs at that cap."
        )),
        _ => Value::Null,
    };
    fields.insert("statement".to_owned(), statement);
    Value::Object(fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use c600_kernel::json;

    fn list(status: &str, rows: Value) -> Value {
        json::object([("status", Value::String(status.to_owned())), ("rows", rows)])
    }

    fn statement(list: Value) -> Value {
        worded(list, 57).as_object().and_then(|fields| fields.get("statement")).cloned().expect("a statement field")
    }

    /// JI.12: only a completed list with no rows makes the negative claim. A cancelled or cut-off list says that it
    /// makes none, with or without rows; an error and a list with rows carry no statement. The layer passes no budget,
    /// so this is the one place the stopped wording is checked.
    #[test]
    fn only_a_completed_empty_list_says_that_no_pair_lowers_n() {
        let empty = || Value::Array(Vec::new());
        let row = || Value::Array(vec![Value::Array([0, 86, 1, 2, -1].map(Value::Int).to_vec())]);
        let none = Value::String("No pair at cap 57 lowers N.".to_owned());
        let stopped = Value::String(
            "The list at cap 57 did not complete, so it says nothing about the pairs at that cap.".into(),
        );
        assert_eq!(statement(list("completed", empty())), none);
        assert_eq!(statement(list("completed", row())), Value::Null);
        for status in ["cancelled", "cut_off"] {
            assert_eq!(statement(list(status, Value::Null)), stopped, "{status}");
            assert_eq!(statement(list(status, empty())), stopped, "{status}: rows never make a stopped list's claim");
        }
        assert_eq!(statement(list("error", Value::Null)), Value::Null);
        assert_eq!(worded(Value::Null, 57), Value::Null, "a value that is not a list is passed on");
    }
}
