//! The applied filter's display (LP3 plan, C6): the layer predicate that `filter.apply` applied, resolved on the
//! command layer to the members of its layer, and a representation's layout filtered to them (LB4.4).
//!
//! The display reads, in this order, and computes nothing else:
//!
//! 1. the record `filter.apply`, the `FilterPreviewRef` the tool's `apply_filter` wrote: the epoch and job of the
//!    filter preview the solver applied. With no such record no filter applies.
//! 2. The filter preview the layer keeps for that job (`CommandLayer::filter_preview`), made in the current epoch.
//! 3. The records its resolved filter read, each still at the version and content digest the preview read them at,
//!    among them the record `structure.layer` that the tool's `layer_predicate` wrote: a cell and a layer.
//! 4. The members of that layer: LP2's `CommandLayer::layer_members`, the positions that `filter.preview` counts and
//!    their home slots, which read no state and refuse with Jumble on.
//!
//! [`Applied::show`] then gives the representation with the occurrences [`Applied::keeps`] accepts
//! ([`Representation::filtered`]): its layout, instances and source map are rebuilt, and its analysed set stays as it
//! was (LB4.4, CT-LB4-03). An applied filter the display cannot resolve leaves the layout unfiltered and says why
//! ([`Unavailable`]). The drafts this module takes are in the crate documentation.

use std::collections::BTreeSet;

use c600_command::{CommandLayer, RecordRead, RecordValue, Store};
use c600_display::plan::ObjectId;

use crate::layout::{LayoutError, Representation};

/// The record the tool's `apply_filter` writes: the `FilterPreviewRef` of the applied filter preview.
pub const APPLIED_RECORD: &str = "filter.apply";

/// The record the tool's `layer_predicate` writes: a layer predicate, its cell and layer.
pub const PREDICATE_RECORD: &str = "structure.layer";

/// The members of an applied layer predicate, as LP2's `CommandLayer::layer_members` gives them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Members {
    pub cell: u32,
    pub layer: u32,
    /// The layer's positions, each named by its home position.
    pub pieces: BTreeSet<u32>,
    /// The positions' home slots.
    pub slots: BTreeSet<u32>,
}

/// Why the display does not show an applied filter. The layout is then shown unfiltered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// The record `filter.apply`, or the predicate's record, holds no value of the form the tool's entries write, or
    /// the applied filter read no record `structure.layer`.
    Form,
    /// The applied filter preview was made in another engine epoch: the session has been reopened since, and a reopen
    /// keeps no filter preview.
    OtherEpoch,
    /// The layer no longer keeps the applied filter preview: it went stale, and the layer dropped it when it kept a
    /// newer one.
    NotKept,
    /// A record the applied filter read has been written since the filter preview read it.
    PredicateWritten,
    /// The command layer refused the members with this code, one of `CommandLayer::layer_members`' refusals: with
    /// Jumble on, `KN5:input.retained_tool_needs_jumble_off`, and for a cell or a layer that names none,
    /// `KN2:input.named_object_invalid`.
    Refused(&'static str),
}

/// The applied filter as the display reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    /// No filter is applied: the session holds no record `filter.apply`.
    None,
    /// The members of the applied layer predicate.
    Members(Members),
    /// A filter is applied, and the display cannot show it.
    Unavailable(Unavailable),
}

impl Applied {
    /// The applied filter on `layer` now. It reads the records and the filter preview the layer holds and the members
    /// of one layer, and changes nothing.
    pub fn read<S: Store>(layer: &CommandLayer<S>) -> Applied {
        match resolve(layer) {
            Ok(Some(members)) => Applied::Members(members),
            Ok(None) => Applied::None,
            Err(why) => Applied::Unavailable(why),
        }
    }

    /// Whether the display shows an occurrence of `object`. With the members of an applied filter: a slot when it is a
    /// member, a sticker when its home slot is, a piece when its home position is, a certificate point when its
    /// witness piece's home position is, and no cell, grip, cap or orbit, which the members do not name. Without them,
    /// every occurrence.
    pub fn keeps(&self, object: &ObjectId) -> bool {
        let Applied::Members(members) = self else { return true };
        match object {
            ObjectId::Slot(slot) | ObjectId::Sticker(slot) => members.slots.contains(slot),
            ObjectId::Piece(piece) | ObjectId::Point { witness: piece, .. } => members.pieces.contains(piece),
            ObjectId::Cell(_) | ObjectId::Grip(_) | ObjectId::Cap(_) | ObjectId::Orbit(_) => false,
        }
    }

    /// `representation` as the display shows it: filtered to the occurrences [`Applied::keeps`] accepts while a filter
    /// applies, and as given otherwise. A filter leaves the analysed set as it was (LB4.4); it refuses a layout that
    /// [`Representation::filtered`] cannot filter.
    pub fn show(&self, representation: &Representation) -> Result<Representation, LayoutError> {
        match self {
            Applied::Members(_) => representation.filtered(|occurrence| self.keeps(&occurrence.object)),
            Applied::None | Applied::Unavailable(_) => Ok(representation.clone()),
        }
    }
}

/// Steps 1 to 4 of the module documentation.
fn resolve<S: Store>(layer: &CommandLayer<S>) -> Result<Option<Members>, Unavailable> {
    let Some(applied) = layer.record(APPLIED_RECORD) else { return Ok(None) };
    let [epoch, job] = naturals(applied, ["epoch", "job"]).ok_or(Unavailable::Form)?;
    if epoch != layer.revision().epoch {
        return Err(Unavailable::OtherEpoch);
    }
    let preview =
        layer.filter_preview(job).filter(|preview| preview.deps.epoch == epoch).ok_or(Unavailable::NotKept)?;
    let reads = &preview.resolved.reads;
    if reads.iter().any(|read| RecordRead::of(&read.record, layer.records().get(&read.record)) != *read) {
        return Err(Unavailable::PredicateWritten);
    }
    if !reads.iter().any(|read| read.record == PREDICATE_RECORD) {
        return Err(Unavailable::Form);
    }
    let predicate = layer.record(PREDICATE_RECORD).ok_or(Unavailable::Form)?;
    let [cell, named] = naturals(predicate, ["cell", "layer"]).ok_or(Unavailable::Form)?;
    let whole = |n: u64| i64::try_from(n).map_err(|_| Unavailable::Form);
    let members = layer.layer_members(whole(cell)?, whole(named)?).map_err(|error| Unavailable::Refused(error.code))?;
    let narrow = |items: &[usize]| items.iter().map(|&n| n as u32).collect();
    Ok(Some(Members {
        cell: members.cell as u32,
        layer: members.layer as u32,
        pieces: narrow(&members.pieces),
        slots: narrow(&members.slots),
    }))
}

/// The two natural fields of a record whose payload is a record of exactly those fields, each in the canonical form
/// the tool's entries write, `{"int": "n"}` with `n` in decimal digits without a sign or a leading zero.
fn naturals(record: &RecordValue, keys: [&str; 2]) -> Option<[u64; 2]> {
    (record.payload.as_object()?.len() == keys.len()).then_some(())?;
    let natural = |key: &str| -> Option<u64> {
        let int = record.payload.get(key)?;
        (int.as_object()?.len() == 1).then_some(())?;
        let digits = int.get("int")?.as_str()?;
        let n: u64 = digits.parse().ok()?;
        (n.to_string() == digits).then_some(n)
    };
    Some([natural(keys[0])?, natural(keys[1])?])
}

#[cfg(test)]
mod tests {
    use c600_kernel::json::{self, Value};

    use super::*;

    fn int(digits: &str) -> Value {
        json::object([("int", Value::String(digits.to_owned()))])
    }

    fn pair(epoch: Value, job: Value) -> Value {
        json::object([("epoch", epoch), ("job", job)])
    }

    /// A record's two naturals are read only in the form the tool's entries write: a record of exactly the two fields,
    /// each `{"int": "n"}` with canonical decimal digits that fit.
    #[test]
    fn a_record_is_read_only_in_the_form_the_entries_write() {
        let read = |payload: &Value| {
            naturals(&RecordValue { payload: payload.clone(), provenance: "block".to_owned() }, ["epoch", "job"])
        };
        assert_eq!(read(&pair(int("3"), int("12"))), Some([3, 12]));
        assert_eq!(read(&pair(int("0"), int("18446744073709551615"))), Some([0, u64::MAX]));
        let loose = json::object([("int", Value::String("3".to_owned())), ("more", Value::Null)]);
        for payload in [
            pair(Value::Int(3), Value::Int(12)),
            json::object([("epoch", int("3"))]),
            json::object([("epoch", int("3")), ("job", int("12")), ("more", int("1"))]),
            pair(int("-3"), int("12")),
            pair(int("+3"), int("12")),
            pair(int("007"), int("12")),
            pair(int("3"), int("00")),
            pair(int(" 3"), int("12")),
            pair(int(""), int("12")),
            pair(int("18446744073709551616"), int("12")),
            pair(loose, int("12")),
            Value::Null,
        ] {
            assert_eq!(read(&payload), None, "{payload:?}");
        }
    }
}
