//! The argument forms of H-08's types (draft; LP3's C6, its third common need). The layer checks them for a row it
//! admits as a program job: a stored row an entry serves, or an overlay row. The layer's own rows check their arguments
//! where they are built.

use std::collections::BTreeMap;

use c600_kernel::json::Value;

use crate::receipt::{code, Error, Revision};
use crate::table::Row;

/// The 600-cell's cells: a `CellId` is 0 to 599, as the kernel's model numbers them. The kernel does not export its
/// cell count.
const CELLS: i64 = 600;

/// The keys of KN0.2's `RevisionRef` that hold text; the other four hold naturals.
const REVISION_TEXTS: [&str; 5] = ["session", "identity", "mode", "head", "digest"];
const REVISION_NATS: [&str; 4] = ["epoch", "state", "workspace", "protection"];

/// Whether an argument of type `ty` may be left out: H-08 writes an optional type with a trailing `?`.
fn optional(ty: &str) -> bool {
    ty.ends_with('?')
}

/// Whether `names`, the argument names of an invocation, fit `row`: each is one of the row's arguments other than the
/// token, and every argument of the row other than the token and the optional ones is among them.
pub(crate) fn names_fit<'a>(row: &Row, names: impl Iterator<Item = &'a str>) -> bool {
    let names: Vec<&str> = names.collect();
    names.iter().all(|name| row.plain_arguments().any(|argument| argument == *name))
        && row
            .arguments
            .iter()
            .all(|(argument, ty)| ty == "UserActionToken" || optional(ty) || names.contains(&argument.as_str()))
}

/// The words of `ty` when it is a choice of words, such as `body|complete`, rather than a union of types, such as
/// `ResolvedDraftRef|WordRef`: every alternative is a lower-case word. Spaces around an alternative do not count.
fn choice(ty: &str) -> Option<Vec<&str>> {
    let words: Vec<&str> = ty.split('|').map(str::trim).collect();
    let word = |w: &&str| {
        w.starts_with(|c: char| c.is_ascii_lowercase()) && w.chars().all(|c| c.is_ascii_lowercase() || c == '_')
    };
    (words.len() > 1 && words.iter().all(word)).then_some(words)
}

fn nat(value: &Value) -> bool {
    matches!(value, Value::Int(n) if *n >= 0)
}

fn cell(value: &Value) -> bool {
    matches!(value, Value::Int(n) if (0..CELLS).contains(n))
}

fn revision(value: &Value) -> bool {
    value.as_object().is_some_and(|fields| {
        fields.len() == REVISION_TEXTS.len() + REVISION_NATS.len()
            && REVISION_TEXTS.iter().all(|key| fields.get(*key).is_some_and(|v| v.as_str().is_some()))
            && REVISION_NATS.iter().all(|key| fields.get(*key).is_some_and(nat))
    })
}

/// Whether `value` has the form of `ty`, or `None` when the layer does not know the type's form yet.
fn form(ty: &str, value: &Value) -> Option<bool> {
    let ty = ty.strip_suffix('?').unwrap_or(ty);
    Some(match ty {
        "Nat" => nat(value),
        "Text" => value.as_str().is_some(),
        "CellId" => cell(value),
        "CellIdPair" => value.as_array().is_some_and(|pair| pair.len() == 2 && pair.iter().all(cell)),
        "RevisionRef" => revision(value),
        _ => {
            let words = choice(ty)?;
            value.as_str().is_some_and(|given| words.contains(&given))
        }
    })
}

/// Checks `args` against `row`'s argument types.
///
/// - `Nat` is an integer of at least 0, `Text` a string, a choice of words one of its words, `CellId` an integer from
///   0 to 599, `CellIdPair` a list of two `CellId`s, and `RevisionRef` KN0.2's nine-key object, its text fields text
///   and its counters naturals. An optional type `T?` has `T`'s form when it is given.
/// - A type whose form the layer does not know yet passes unchecked.
/// - A malformed argument is refused with `KN5:input.argument_form`. Then a `RevisionRef` that names a revision other
///   than `current` is refused with `KN5:stale.revision_changed` (EV4.14: a named revision is current only if every
///   field is equal), so a malformed request is never answered as stale.
pub(crate) fn check(row: &Row, args: &BTreeMap<String, Value>, current: &Revision) -> Result<(), Error> {
    let typed = || args.iter().filter_map(|(name, value)| Some((name, row.arguments.get(name)?.as_str(), value)));
    for (name, ty, value) in typed() {
        if form(ty, value) == Some(false) {
            return Err(Error::new(code::ARGUMENT_FORM, format!("`{name}` is not a {ty}")));
        }
    }
    let now = current.to_json();
    for (name, ty, value) in typed() {
        if ty.strip_suffix('?').unwrap_or(ty) == "RevisionRef" && *value != now {
            return Err(Error::new(code::REVISION_CHANGED, format!("`{name}` names a revision that is not current")));
        }
    }
    Ok(())
}
