//! Kernel rows that the host bridge serves to a program's snapshot read with an argument (KN5; LP2 draft until the
//! command table freezes, PHS.52). One row so far: KN2.4's `positions`, the position of named pieces, which library
//! entries read (CT-LB2-04: exact completion reads identities, slots and orientations).
//! [`crate::bridge::Bridge`] serves it from the command layer's head ([`CommandLayer::positions`]).
//!
//! A program reads the row as `read_snapshot<T>("positions", pieces)`. `pieces` is a list of integers, such as a
//! `List<Nat>` or a `Set<Nat>` (in increasing order): pieces named by their home positions, 0 to 177,119, with repeats
//! allowed. `T` is a list of records with these five fields, in any order:
//!
//! - `piece: Nat`, the named piece;
//! - `orbit: Int`, its orbit, -1 for a fixed cell centre;
//! - `home: List<Nat>`, its home slots in increasing order;
//! - `slots: List<Nat>`: `slots[i]` is the slot that now holds the sticker whose home slot is `home[i]`, so the
//!   slots and the orientation together;
//! - `position: Int`, the position whose slots hold all of its stickers, or -1 when they are split over positions
//!   (the IR has no option type, and `orbit` already uses -1).
//!
//! `piece`, `home` and `slots` may also be declared with `Int`, which holds the same values; no field carries a display
//! qualifier. The answer holds one record per named piece, in the argument's order, with its fields in `T`'s order.
//!
//! The bridge refuses before the kernel runs, at the cost of the read's step: a declared type other than these,
//! `LG2:type.mismatch`; an argument that is not a list of integers, `KN5:input.argument_form`. Once the read is
//! charged, the layer refuses it with Jumble on, `KN5:input.retained_tool_needs_jumble_off` (LG2.8), and the kernel
//! refuses a piece outside 0 to 177,119, `KN2:input.named_object_invalid`; the charge stays, as for a refused
//! proposal. The tariff is [`positions_tariff`].

use std::sync::OnceLock;

use c600_check::TYPE_MISMATCH;
use c600_command::{code, CommandLayer, Store};
use c600_ir::{QTy, Ty};
use c600_kernel::{PiecePosition, SLOTS};
use c600_run::interp::RowTariff;
use c600_run::provenance::{self, Json};
use c600_run::value::Value;

/// The row's query: KN2.4's `positions`.
pub const POSITIONS: &str = "positions";

/// The most stickers one piece of 600-cell-Full has (the kernel's largest piece; a test checks it). The row's tariff
/// takes it for every named piece.
pub const MAX_PIECE_STICKERS: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Home,
    Orbit,
    Piece,
    Position,
    Slots,
}

/// The fields of the row's records, by name.
const FIELDS: [(&str, Field); 5] = [
    ("home", Field::Home),
    ("orbit", Field::Orbit),
    ("piece", Field::Piece),
    ("position", Field::Position),
    ("slots", Field::Slots),
];

fn exact(ty: Ty) -> QTy {
    QTy { ty, display: false }
}

fn list(element: Ty) -> QTy {
    exact(Ty::List(Box::new(exact(element))))
}

/// Whether `declared` holds every value the row gives `field`: `Nat` or `Int` where no value is negative.
fn holds(field: Field, declared: &QTy) -> bool {
    let whole = |ty: &QTy| *ty == exact(Ty::Nat) || *ty == exact(Ty::Int);
    match field {
        Field::Piece => whole(declared),
        Field::Orbit | Field::Position => *declared == exact(Ty::Int),
        Field::Home | Field::Slots => match &declared.ty {
            Ty::List(element) => !declared.display && whole(element),
            _ => false,
        },
    }
}

/// The type the row answers, with its fields in name order:
/// `List<record { home: List<Nat>, orbit: Int, piece: Nat, position: Int, slots: List<Nat> }>`.
pub fn positions_type() -> QTy {
    let field = |field: Field| match field {
        Field::Piece => exact(Ty::Nat),
        Field::Orbit | Field::Position => exact(Ty::Int),
        Field::Home | Field::Slots => list(Ty::Nat),
    };
    list(Ty::Record(FIELDS.iter().map(|&(name, f)| (name.to_owned(), field(f))).collect()))
}

/// The fields of a declared answer type in its order; None unless it is a list of records with the row's five fields,
/// each declared with a type that holds its values, and no display qualifier.
fn declared_fields(ty: &QTy) -> Option<Vec<Field>> {
    let Ty::List(record) = &ty.ty else { return None };
    let Ty::Record(fields) = &record.ty else { return None };
    if ty.display || record.display || fields.len() != FIELDS.len() {
        return None;
    }
    let mut order: Vec<Field> = Vec::with_capacity(FIELDS.len());
    for (name, declared) in fields {
        let &(_, field) = FIELDS.iter().find(|(known, _)| known == name)?;
        if order.contains(&field) || !holds(field, declared) {
            return None;
        }
        order.push(field);
    }
    Some(order)
}

/// One canonical integer, `-?(0|[1-9][0-9]*)`, saturated to `i64`: an integer of any size outside 0 to 177,119 stays
/// outside it.
fn integer(text: &[u8]) -> Option<i64> {
    let (negative, digits) = match text.strip_prefix(b"-") {
        Some(digits) => (true, digits),
        None => (false, text),
    };
    let canonical = match digits {
        [] | [b'0', _, ..] => false,
        [b'0'] => !negative,
        _ => digits.iter().all(u8::is_ascii_digit),
    };
    if !canonical {
        return None;
    }
    let magnitude = digits.iter().fold(0i64, |n, &d| n.saturating_mul(10).saturating_add(i64::from(d - b'0')));
    Some(if negative { -magnitude } else { magnitude })
}

/// The named pieces: the argument's canonical bytes (DET.59), an array of exact integers such as
/// `[{"int":"3"},{"int":"0"},{"int":"3"}]` and a newline, as a list or a set encodes. Each integer is canonical,
/// `-?(0|[1-9][0-9]*)`, and is read saturated to `i64`, so one of any size outside 0 to 177,119 stays outside it. None
/// for any other form.
pub fn named_pieces(arg: &[u8]) -> Option<Vec<i64>> {
    let Json::List(items) = provenance::parse(arg)? else { return None };
    let piece = |item: &Json| match item {
        Json::Object(fields) if fields.len() == 1 => integer(fields.get("int")?.as_text()?.as_bytes()),
        _ => None,
    };
    items.iter().map(piece).collect()
}

/// The logical size of an empty answer, and what one record adds to an answer at most: a record whose integers all
/// have the largest magnitude a field holds, 259,799 (no piece, position or orbit reaches it), with
/// [`MAX_PIECE_STICKERS`] slots in `home` and in `slots`.
fn answer_sizes() -> (u128, u128) {
    static SIZES: OnceLock<(u128, u128)> = OnceLock::new();
    *SIZES.get_or_init(|| {
        let largest = (SLOTS - 1) as i64;
        let stickers = Value::list(vec![Value::int(largest); MAX_PIECE_STICKERS]);
        let value = |field: Field| match field {
            Field::Home | Field::Slots => stickers.clone(),
            _ => Value::int(largest),
        };
        let record = Value::record(FIELDS.iter().map(|&(name, field)| (name, value(field))).collect());
        let empty = Value::list(Vec::new()).size();
        (empty, Value::list(vec![record]).size() - empty)
    })
}

/// The row's draft tariff for `pieces` named pieces (DET.55; the command table prices the row when it freezes). The
/// kernel cost counts the slots the row reads: one pass over the state's 259,800 labels, then at most
/// [`MAX_PIECE_STICKERS`] for each named piece. The result bound is the logical size of an answer of that many of the
/// largest records: records whose integers all have magnitude 259,799 and whose `home` and `slots` hold
/// [`MAX_PIECE_STICKERS`] slots each.
pub fn positions_tariff(pieces: usize) -> RowTariff {
    let n = pieces as u128;
    let kernel_cost = (SLOTS as u128).saturating_add(n.saturating_mul(MAX_PIECE_STICKERS as u128));
    let (empty, record) = answer_sizes();
    RowTariff { kernel_cost, result_bound: empty.saturating_add(n.saturating_mul(record)) }
}

/// The tariff of a `positions` read, or the refusal that costs the read's step only: `LG2:type.mismatch` for another
/// declared type, `KN5:input.argument_form` for an argument that is not a list of integers.
pub fn tariff(arg: &[u8], ty: &QTy) -> Result<RowTariff, &'static str> {
    declared_fields(ty).ok_or(TYPE_MISMATCH)?;
    let pieces = named_pieces(arg).ok_or(code::ARGUMENT_FORM)?;
    Ok(positions_tariff(pieces.len()))
}

/// One record of the answer, its fields in `order`.
fn record(row: &PiecePosition, order: &[Field]) -> Value {
    let slots = |slots: &[i32]| Value::list(slots.iter().map(|&slot| Value::int(slot)).collect());
    let value = |field: Field| match field {
        Field::Home => slots(&row.home),
        Field::Orbit => Value::int(row.orbit),
        Field::Piece => Value::int(row.piece as u64),
        Field::Position => Value::int(row.position.map_or(-1, |position| position as i64)),
        Field::Slots => slots(&row.slots),
    };
    let name = |field: Field| FIELDS.iter().find(|(_, f)| *f == field).map(|&(name, _)| name).expect("a field");
    Value::record(order.iter().map(|&field| (name(field), value(field))).collect())
}

/// The answer of a `positions` read at `layer`'s head, with [`tariff`]'s refusals, the layer's refusal with Jumble on
/// and the kernel's refusal of a piece outside 0 to 177,119.
pub fn positions<S: Store>(layer: &CommandLayer<S>, arg: &[u8], ty: &QTy) -> Result<Value, &'static str> {
    let order = declared_fields(ty).ok_or(TYPE_MISMATCH)?;
    let pieces = named_pieces(arg).ok_or(code::ARGUMENT_FORM)?;
    let rows = layer.positions(&pieces).map_err(|error| error.code)?;
    Ok(Value::list(rows.iter().map(|row| record(row, &order)).collect()))
}
