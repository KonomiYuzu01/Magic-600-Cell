//! The applied grip-slot map (EV4.8, PS1.26; DT-M1 Outcome K2, Q3 (a) and K6; LP4-8; draft): workspace state that only
//! the solver's rebind command, H-08's `grip.rebind` ([`GRIP_REBIND_ROW`]), changes.
//!
//! - The map is the one record `grip/map` ([`SLOT_MAP_RECORD`]). Before the first rebind no record holds it, and the
//!   host's default map applies (the input router's `Context`; DT-M1 K6: one default map with the solver's edits).
//! - `grip.rebind {anchor, map, expected_map}` writes it as one record write, undoable as a workspace inverse (H-08's
//!   undo class `workspace`). `anchor` is a `PieceId`, below the kernel's `PIECES`. `map` is the transported slot map
//!   in slot order: one to [`POLES`] slots, each exactly `{cap, frame}`, with `cap` a `CapId` below [`POLES`] and
//!   `frame` the slot's transported frame, kept as given (`FrameRef` has no settled form yet). A map of another form is
//!   refused with `KN5:input.argument_form`. `expected_map` is the digest of the map the rebind replaces
//!   ([`slot_map_digest`]): the SHA-256 of the record's canonical payload, or of `null` while the default applies. A
//!   rebind that names another map is refused with `KN5:stale.revision_changed`, as a work sheet at another version is
//!   (draft: chapter 13 registers no code of its own for a raced grip map).
//! - The record holds the anchor, the map and the model it was checked against (PS1's applied grip map: "absolute caps
//!   and transported frames, including model dependencies"), with a fixed provenance block naming the row
//!   ([`SLOT_MAP_PROVENANCE`]): the solver's own command wrote it, and no host block applies.
//! - Names under `grip/` are the map's: `workspace.set` and `worksheet.save` refuse them with
//!   `KN5:input.argument_form`, a program job's output that writes one fails with `KN2:input.named_object_invalid`,
//!   and a reopen admits a write of one only under `grip.rebind` from the shell (B7). Camera, filter, twist, focus and
//!   restart write no record, so none of them derives a new applied map.

use c600_jumble::model::POLES;
use c600_kernel::json::{self, Value};

use crate::receipt::{code, digest, Error};

/// H-08's rebind command: the only row that changes the applied grip-slot map (EV4.8).
pub const GRIP_REBIND_ROW: &str = "grip.rebind";
/// The prefix of the record names the applied grip-slot map reserves.
pub const GRIP_PREFIX: &str = "grip/";
/// The record that holds the applied grip-slot map after the first rebind.
pub const SLOT_MAP_RECORD: &str = "grip/map";
/// The provenance block of the map's record: the solver's own rebind command wrote it.
pub const SLOT_MAP_PROVENANCE: &str = "grip.rebind";

/// One slot of the applied map: an absolute cap and its transported frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// `CapId`: a cap of the 600-cell, below [`POLES`].
    pub cap: u32,
    /// `FrameRef`: the slot's transported frame, as the rebind gave it.
    pub frame: Value,
}

/// The applied grip-slot map a rebind wrote: its anchor, its slots in slot order and the model it was checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotMap {
    /// `PieceId`: the grip anchor the map was transported from.
    pub anchor: u64,
    pub slots: Vec<Slot>,
    /// The `model_id` of the model the map was checked against.
    pub model: String,
}

impl SlotMap {
    /// Reads `grip.rebind`'s `anchor` and `map` under the model `model`. Anything else is refused with
    /// `KN5:input.argument_form`.
    pub fn from_args(anchor: Option<&Value>, map: Option<&Value>, model: &str) -> Result<SlotMap, Error> {
        let form = |detail: &str| Error::new(code::ARGUMENT_FORM, format!("`{GRIP_REBIND_ROW}`: {detail}"));
        let anchor = anchor
            .and_then(Value::as_i64)
            .and_then(|n| u64::try_from(n).ok())
            .filter(|&n| usize::try_from(n).is_ok_and(|n| n < c600_kernel::PIECES))
            .ok_or_else(|| form("`anchor` is a piece below the model's pieces"))?;
        let Some(map) = map.and_then(Value::as_array).filter(|map| (1..=POLES).contains(&map.len())) else {
            return Err(form(&format!("`map` lists 1 to {POLES} slots")));
        };
        let slots = map.iter().map(slot).collect::<Option<Vec<Slot>>>();
        let slots =
            slots.ok_or_else(|| form(&format!("each slot is exactly `{{cap, frame}}`, a cap below {POLES}")))?;
        Ok(SlotMap { anchor, slots, model: model.to_owned() })
    }

    /// The record's payload: `{anchor, map: [{cap, frame}], model}`.
    pub fn to_payload(&self) -> Value {
        let slots = self
            .slots
            .iter()
            .map(|slot| json::object([("cap", Value::Int(i64::from(slot.cap))), ("frame", slot.frame.clone())]));
        json::object([
            ("anchor", Value::Int(self.anchor as i64)),
            ("map", Value::Array(slots.collect())),
            ("model", Value::String(self.model.clone())),
        ])
    }

    /// Reads a record's payload ([`SlotMap::to_payload`]); `None` for any other value.
    pub fn from_payload(payload: &Value) -> Option<SlotMap> {
        let fields = payload.as_object().filter(|fields| fields.len() == 3)?;
        let model = fields.get("model")?.as_str()?;
        SlotMap::from_args(fields.get("anchor"), fields.get("map"), model).ok()
    }
}

/// One slot `{cap, frame}`, with a cap below [`POLES`].
fn slot(value: &Value) -> Option<Slot> {
    let fields = value.as_object().filter(|fields| fields.len() == 2)?;
    let cap = fields.get("cap")?.as_i64().and_then(|n| u32::try_from(n).ok()).filter(|&n| (n as usize) < POLES)?;
    Some(Slot { cap, frame: fields.get("frame")?.clone() })
}

/// The digest `expected_map` names: the SHA-256 of the applied map's canonical payload, or of `null` while no rebind
/// applies one.
pub fn slot_map_digest(payload: Option<&Value>) -> String {
    digest(payload.unwrap_or(&Value::Null))
}
