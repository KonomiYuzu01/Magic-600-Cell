//! Kernel rows that the host bridge serves to a program's snapshot read with an argument (KN5; LP2 draft until the
//! command table freezes, PHS.52). Four rows so far, all of KN2.4. LB2's exact completion reads two of them
//! (`c600_library`'s `completion.exact_completion`; CT-LB2-04: exact completion reads identities, slots and
//! orientations): `positions`, the position of named pieces, and `labels`, the label of named slots. LB2's recipes
//! and reports read the third (`c600_library`'s `recipes`; CT-LB2-01): `effect`, a recipe's net map and what it
//! touches. LB2's Net and Strict protection reads the fourth, with `positions` and `labels` for its captures
//! (`c600_library`'s `protection`; CT-LB2-02): `protection`, a captured lock judged against a recipe at the head.
//! [`crate::bridge::Bridge`] serves `positions` and `labels` from the command layer's head: `positions` through
//! [`CommandLayer::positions`], and `labels` from the head's snapshot ([`CommandLayer::snapshot`]), because neither the
//! command layer nor the kernel has a `labels` row function. It serves `effect` through the kernel's
//! [`c600_kernel::Kernel::effect`], which reads no state, and `protection` from the head's snapshot and the kernel's
//! effect and expansion, because the kernel's protection rows judge pieces, not a lock's slot and label pairs.
//!
//! # `positions`
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
//!
//! # `labels`
//!
//! A program reads the row as `read_snapshot<T>("labels", slots)`. `slots` is a list of integers, such as a
//! `List<Nat>` or a `Set<Nat>` (in increasing order): slots 0 to 259,799, with repeats allowed. `T` is `List<Nat>` or
//! `List<Int>`, with no display qualifier. The answer holds one label per named slot, in the argument's order.
//!
//! A slot's label is the home slot of the sticker that the slot now holds ([`c600_kernel::Snapshot::labels`]): it
//! names the sticker by its identity, never by its colour, so two stickers of one colour have two labels. A slot holds
//! its own sticker exactly when its label is the slot itself, and in the solved state every slot does.
//!
//! The bridge refuses as for `positions`. Before the row runs, at the cost of the read's step: a declared type other
//! than these, `LG2:type.mismatch`; an argument that is not a list of integers, `KN5:input.argument_form`. Once the
//! read is charged: with Jumble on, `KN5:input.retained_tool_needs_jumble_off` (LG2.8), as
//! [`CommandLayer::positions`] refuses it, since the head's snapshot is then the retained state the crossing entered
//! from; and a slot outside 0 to 259,799, `KN2:input.named_object_invalid`, the kernel's code for a named object
//! outside the model, which it gives a piece outside 0 to 177,119. The tariff is [`labels_tariff`].
//!
//! # `effect`
//!
//! A program reads the row as `read_snapshot<T>("effect", recipe)`. `recipe` is a recipe in one of two forms:
//!
//! - a `Word` (or a `List<Int>`): a recipe of one word step, `[{"kind": "word", "moves": [...]}]`;
//! - a list of step records `record { kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }`, in 0.4's recipe
//!   form step by step. A word step has `kind = "word"`, its letters in `moves` and 0 in the three star fields; a star
//!   step has `kind = "star"`, an empty `moves` and its orbit, node and sign. `moves` may also be a `List<Int>`.
//!
//! The kernel normalizes the recipe with 0.4's checks, order and texts (LB2.4: 1 to 256 steps, at most 3,000,000
//! primitives, word chunks of at most 100,000), then computes its effect ([`c600_kernel::Kernel::effect`]). `T` is a
//! record with any nonempty set of these fields, in any order:
//!
//! - `source: List<Nat>`, the moved slots in increasing order, and `destination: List<Nat>`: `destination[i]` is where
//!   the sticker in `source[i]` goes. Together they are the complete source-to-destination map, every collateral
//!   effect included (LB2.2);
//! - `net_sha256: Text`, the map's digest, and `expansion_sha256: Text`, the digest of the expansion's letters in
//!   chronological order (the oracle's digests);
//! - `moved_stickers: Nat`, `moved_pieces: Nat` and `primitive_count: Nat`;
//! - `support: List<record { orbit: Int, pieces: Nat, stickers: Nat }>`, one row per orbit the map moves, in increasing
//!   orbit order, with its fields in any order.
//!
//! A whole-number field may also be declared with `Int`; no field carries a display qualifier. A program that needs no
//! map leaves out `source` and `destination`, and the row's result bound shrinks with them. The answer's fields are in
//! `T`'s order.
//!
//! The bridge refuses before the kernel runs, at the cost of the read's step: a declared type other than these,
//! `LG2:type.mismatch`; an argument in neither form, `KN5:input.argument_form`. Once the read is charged: with Jumble
//! on, `KN5:input.retained_tool_needs_jumble_off` (LB2.1: retained workflow tools require Jumble off); and a recipe the
//! kernel refuses, with the kernel's code. The charge stays, as for a refused proposal. The tariff is
//! [`effect_tariff`].
//!
//! # `protection`
//!
//! A program reads the row as `read_snapshot<T>("protection", check)`. LB2's Net and Strict protection reads it
//! (`c600_library`'s `protection`; CT-LB2-02). `check` is a record of three or four fields:
//!
//! - `lock: record { kind: Text, labels: List<Nat>, slots: List<Nat> }`, a captured lock as exact pairs: slot
//!   `slots[i]` must hold label `labels[i]`. The lists have one length, and repeats are allowed. `kind` names how the
//!   lock was captured, and the row does not read it.
//! - `mode: Text`, `"net"` or `"strict"`.
//! - `recipe`, in one of the `effect` row's two forms.
//! - `limit: Nat`, with `"strict"` only and optional: the last prefix judged, in primitives.
//!
//! Net judges the end of the complete operation at the head: the sticker that ends in slot `p` started in `σ⁻¹(p)`, so
//! `p`'s end label is the head's label at `σ⁻¹(p)`. Strict judges every executed prefix of the literal expansion, the
//! empty one first, up to `limit` primitives when one is given. `T` is a record with these six fields, in any order:
//!
//! - `level: Text`, `"labels"`: the level the row judges at. KN2.4 types the row by puzzle and level (labels for Full,
//!   poses for jumbling), and this row answers Full at labels, so a lock holds exact labels.
//! - `checked: Nat`, the number of states judged. Net judges one. Strict judges the prefixes up to the first one that
//!   breaks the lock, or up to the limit or the end.
//! - `conflicts: List<Nat>`, the lock's slots that fail at the judged state that fails, in the lock's order; empty when
//!   none fails.
//! - `refused_prefix: Int`, Strict's first prefix that breaks the lock: 0 when the head breaks it, and -1 when no judged
//!   prefix does and for Net.
//! - `expansion_sha256: Text` and `primitive_count: Nat`, the recipe's input: its expansion's digest and length.
//!
//! `checked`, `primitive_count` and the elements of `conflicts` may also be declared with `Int`; no field carries a
//! display qualifier. The row answers no verdict. A check covers the whole operation when Net judged its one state, or
//! when Strict judged `primitive_count + 1` states, and the library's report is never safe otherwise (LB2.3).
//!
//! The bridge refuses before the kernel runs, at the cost of the read's step: a declared type other than these,
//! `LG2:type.mismatch`; an argument of another form, `KN5:input.argument_form`. Once the read is charged: with Jumble
//! on, `KN5:input.retained_tool_needs_jumble_off` (LB2.1); a recipe the kernel refuses, with the kernel's code; and a
//! slot or a label outside 0 to 259,799, `KN2:input.named_object_invalid`. The charge stays, as for a refused proposal.
//! The tariff is [`protection_tariff`].

use std::sync::OnceLock;

use c600_check::TYPE_MISMATCH;
use c600_command::layer::CERTIFICATE;
use c600_command::{code, CommandLayer, Store};
use c600_ir::{QTy, Ty};
use c600_kernel::json::{self as kernel_json, Value as KernelJson};
use c600_kernel::{Effect, Kernel, PiecePosition, PIECES, SLOTS};
use c600_run::interp::RowTariff;
use c600_run::provenance::{self, Json};
use c600_run::value::Value;

/// The `positions` row's query: KN2.4's `positions`.
pub const POSITIONS: &str = "positions";

/// The `labels` row's query: KN2.4's `labels`.
pub const LABELS: &str = "labels";

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

/// `Nat` or `Int` with no display qualifier: a type that holds every whole number 0 to 259,799.
fn whole(ty: &QTy) -> bool {
    *ty == exact(Ty::Nat) || *ty == exact(Ty::Int)
}

/// A list of [`whole`] numbers with no display qualifier.
fn whole_list(ty: &QTy) -> bool {
    match &ty.ty {
        Ty::List(element) => !ty.display && whole(element),
        _ => false,
    }
}

/// Whether `declared` holds every value the row gives `field`: `Nat` or `Int` where no value is negative.
fn holds(field: Field, declared: &QTy) -> bool {
    match field {
        Field::Piece => whole(declared),
        Field::Orbit | Field::Position => *declared == exact(Ty::Int),
        Field::Home | Field::Slots => whole_list(declared),
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

/// The type the `labels` row answers: `List<Nat>`.
pub fn labels_type() -> QTy {
    list(Ty::Nat)
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

/// One canonical integer, `-?(0|[1-9][0-9]*)`, saturated to `i64`: an integer of any size outside a row's range stays
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

/// The integers of an argument's canonical bytes (DET.59): an array of exact integers such as
/// `[{"int":"3"},{"int":"0"},{"int":"3"}]` and a newline, as a list or a set encodes. Each integer is canonical,
/// `-?(0|[1-9][0-9]*)`, and is read saturated to `i64`. None for any other form.
fn integers(arg: &[u8]) -> Option<Vec<i64>> {
    let Json::List(items) = provenance::parse(arg)? else { return None };
    let item = |item: &Json| match item {
        Json::Object(fields) if fields.len() == 1 => integer(fields.get("int")?.as_text()?.as_bytes()),
        _ => None,
    };
    items.iter().map(item).collect()
}

/// The named pieces of a `positions` read: the argument's integers ([`integers`]), so one of any size outside 0 to
/// 177,119 stays outside it. None for an argument that is not a list of integers.
pub fn named_pieces(arg: &[u8]) -> Option<Vec<i64>> {
    integers(arg)
}

/// The named slots of a `labels` read: the argument's integers ([`integers`]), so one of any size outside 0 to 259,799
/// stays outside it. None for an argument that is not a list of integers.
pub fn named_slots(arg: &[u8]) -> Option<Vec<i64>> {
    integers(arg)
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

/// The logical size of an empty `labels` answer, and what one label adds to an answer at most: the largest label,
/// 259,799.
fn label_sizes() -> (u128, u128) {
    static SIZES: OnceLock<(u128, u128)> = OnceLock::new();
    *SIZES.get_or_init(|| {
        let empty = Value::list(Vec::new()).size();
        (empty, Value::list(vec![Value::int((SLOTS - 1) as i64)]).size() - empty)
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

/// The `labels` row's draft tariff for `slots` named slots (DET.55; the command table prices the row when it
/// freezes). The kernel cost counts the labels the row reads, one for each named slot: the row reads the head's label
/// array where it stands and inverts nothing. The result bound is the logical size of an answer of that many of the
/// largest labels, 259,799 each.
pub fn labels_tariff(slots: usize) -> RowTariff {
    let n = slots as u128;
    let (empty, label) = label_sizes();
    RowTariff { kernel_cost: n, result_bound: empty.saturating_add(n.saturating_mul(label)) }
}

/// The tariff of a `positions` read, or the refusal that costs the read's step only: `LG2:type.mismatch` for another
/// declared type, `KN5:input.argument_form` for an argument that is not a list of integers.
pub fn positions_read_tariff(arg: &[u8], ty: &QTy) -> Result<RowTariff, &'static str> {
    declared_fields(ty).ok_or(TYPE_MISMATCH)?;
    let pieces = named_pieces(arg).ok_or(code::ARGUMENT_FORM)?;
    Ok(positions_tariff(pieces.len()))
}

/// The tariff of a `labels` read, or the refusal that costs the read's step only: `LG2:type.mismatch` for a declared
/// type other than `List<Nat>` or `List<Int>`, `KN5:input.argument_form` for an argument that is not a list of
/// integers.
pub fn labels_read_tariff(arg: &[u8], ty: &QTy) -> Result<RowTariff, &'static str> {
    if !whole_list(ty) {
        return Err(TYPE_MISMATCH);
    }
    let slots = named_slots(arg).ok_or(code::ARGUMENT_FORM)?;
    Ok(labels_tariff(slots.len()))
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

/// The answer of a `positions` read at `layer`'s head, with [`positions_read_tariff`]'s refusals, the layer's refusal
/// with Jumble on and the kernel's refusal of a piece outside 0 to 177,119.
pub fn positions<S: Store>(layer: &CommandLayer<S>, arg: &[u8], ty: &QTy) -> Result<Value, &'static str> {
    let order = declared_fields(ty).ok_or(TYPE_MISMATCH)?;
    let pieces = named_pieces(arg).ok_or(code::ARGUMENT_FORM)?;
    let rows = layer.positions(&pieces).map_err(|error| error.code)?;
    Ok(Value::list(rows.iter().map(|row| record(row, &order)).collect()))
}

/// The answer of a `labels` read at `layer`'s head, with [`labels_read_tariff`]'s refusals, the refusal with Jumble on
/// that [`CommandLayer::positions`] gives, and the refusal of a slot outside 0 to 259,799. A read changes nothing.
pub fn labels<S: Store>(layer: &CommandLayer<S>, arg: &[u8], ty: &QTy) -> Result<Value, &'static str> {
    if !whole_list(ty) {
        return Err(TYPE_MISMATCH);
    }
    let slots = named_slots(arg).ok_or(code::ARGUMENT_FORM)?;
    if layer.jumbled().is_some() {
        return Err(code::RETAINED_TOOL_NEEDS_JUMBLE_OFF);
    }
    let labels = layer.snapshot().labels();
    let label = |slot: &i64| usize::try_from(*slot).ok().and_then(|slot| labels.get(slot)).map(|&l| Value::int(l));
    let answer: Option<Vec<Value>> = slots.iter().map(label).collect();
    answer.map(Value::list).ok_or(code::NAMED_OBJECT_INVALID)
}

/// The `effect` row's query: KN2.4's `effect`.
pub const EFFECT: &str = "effect";

/// The most primitives a recipe's expansion has under the retained recipe contract (LB2.4); the kernel refuses a
/// longer recipe. The `effect` row's result bound takes it for `primitive_count`.
pub const MAX_PRIMITIVES: usize = 3_000_000;

/// The most rows a support has: one per moving orbit, 0 to 34, and one for the fixed cell centres, orbit -1.
pub const MAX_SUPPORT_ROWS: usize = 36;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EffectField {
    Destination,
    ExpansionSha256,
    MovedPieces,
    MovedStickers,
    NetSha256,
    PrimitiveCount,
    Source,
    Support,
}

/// The fields of the `effect` row's record, by name.
const EFFECT_FIELDS: [(&str, EffectField); 8] = [
    ("destination", EffectField::Destination),
    ("expansion_sha256", EffectField::ExpansionSha256),
    ("moved_pieces", EffectField::MovedPieces),
    ("moved_stickers", EffectField::MovedStickers),
    ("net_sha256", EffectField::NetSha256),
    ("primitive_count", EffectField::PrimitiveCount),
    ("source", EffectField::Source),
    ("support", EffectField::Support),
];

/// The fields of a support row, by name.
const SUPPORT_FIELDS: [&str; 3] = ["orbit", "pieces", "stickers"];

/// A declared `effect` answer type: its fields in order, and the fields of its support rows in order when it has a
/// `support` field.
struct EffectShape {
    fields: Vec<EffectField>,
    support: Vec<&'static str>,
}

/// The fields of a declared support row type in its order; None unless it is a list of records with the three support
/// fields, `orbit: Int` and `pieces` and `stickers` [`whole`], and no display qualifier.
fn support_fields(ty: &QTy) -> Option<Vec<&'static str>> {
    let Ty::List(record) = &ty.ty else { return None };
    let Ty::Record(fields) = &record.ty else { return None };
    if ty.display || record.display || fields.len() != SUPPORT_FIELDS.len() {
        return None;
    }
    let mut order: Vec<&'static str> = Vec::with_capacity(SUPPORT_FIELDS.len());
    for (name, declared) in fields {
        let &known = SUPPORT_FIELDS.iter().find(|known| *known == name)?;
        let fits = if known == "orbit" { *declared == exact(Ty::Int) } else { whole(declared) };
        if order.contains(&known) || !fits {
            return None;
        }
        order.push(known);
    }
    Some(order)
}

/// The shape of a declared `effect` answer type; None unless it is a record with a nonempty set of the row's fields,
/// each declared with a type that holds its values, and no display qualifier.
fn effect_shape(ty: &QTy) -> Option<EffectShape> {
    let Ty::Record(fields) = &ty.ty else { return None };
    if ty.display || fields.is_empty() {
        return None;
    }
    let mut shape = EffectShape { fields: Vec::with_capacity(fields.len()), support: Vec::new() };
    for (name, declared) in fields {
        let &(_, field) = EFFECT_FIELDS.iter().find(|(known, _)| known == name)?;
        let fits = match field {
            EffectField::Destination | EffectField::Source => whole_list(declared),
            EffectField::ExpansionSha256 | EffectField::NetSha256 => *declared == exact(Ty::Text),
            EffectField::MovedPieces | EffectField::MovedStickers | EffectField::PrimitiveCount => whole(declared),
            EffectField::Support => match support_fields(declared) {
                Some(order) => {
                    shape.support = order;
                    true
                }
                None => false,
            },
        };
        if shape.fields.contains(&field) || !fits {
            return None;
        }
        shape.fields.push(field);
    }
    Some(shape)
}

/// The type the `effect` row answers with all its fields, in name order:
/// `record { destination: List<Nat>, expansion_sha256: Text, moved_pieces: Nat, moved_stickers: Nat, net_sha256: Text,
/// primitive_count: Nat, source: List<Nat>, support: List<record { orbit: Int, pieces: Nat, stickers: Nat }> }`.
pub fn effect_type() -> QTy {
    let support_row = |name: &str| (name.to_owned(), if name == "orbit" { exact(Ty::Int) } else { exact(Ty::Nat) });
    let support = list(Ty::Record(SUPPORT_FIELDS.iter().map(|&name| support_row(name)).collect()));
    let field = |field: EffectField| match field {
        EffectField::Destination | EffectField::Source => list(Ty::Nat),
        EffectField::ExpansionSha256 | EffectField::NetSha256 => exact(Ty::Text),
        EffectField::MovedPieces | EffectField::MovedStickers | EffectField::PrimitiveCount => exact(Ty::Nat),
        EffectField::Support => support.clone(),
    };
    exact(Ty::Record(EFFECT_FIELDS.iter().map(|&(name, f)| (name.to_owned(), field(f))).collect()))
}

/// An exact integer in an argument's canonical bytes, `{"int": "n"}`, saturated to `i64`.
fn exact_int(item: &Json) -> Option<i64> {
    let Json::Object(fields) = item else { return None };
    if fields.len() != 1 {
        return None;
    }
    integer(fields.get("int")?.as_text()?.as_bytes())
}

/// One letter of a word or a list of integers in an argument's canonical bytes: a JSON integer, as a word encodes its
/// letters, or `{"int": "n"}`, as a list of integers does; saturated to `i64`, so the kernel refuses one of any size.
fn letter(item: &Json) -> Option<i64> {
    match item {
        Json::Int(n) => Some(*n),
        _ => exact_int(item),
    }
}

/// The letters of a word or a list of integers, as kernel JSON.
fn moves(items: &[Json]) -> Option<KernelJson> {
    items.iter().map(|item| letter(item).map(KernelJson::Int)).collect::<Option<Vec<_>>>().map(KernelJson::Array)
}

/// One step record of an `effect` argument in 0.4's recipe form.
fn recipe_step(step: &Json) -> Option<KernelJson> {
    let Json::Object(fields) = step else { return None };
    let names = ["kind", "moves", "node", "orbit", "sign"];
    if fields.len() != names.len() || !names.iter().all(|name| fields.contains_key(*name)) {
        return None;
    }
    let Json::List(letters) = &fields["moves"] else { return None };
    let (node, orbit, sign) = (exact_int(&fields["node"])?, exact_int(&fields["orbit"])?, exact_int(&fields["sign"])?);
    match fields["kind"].as_text()? {
        "word" if (node, orbit, sign) == (0, 0, 0) => {
            Some(kernel_json::object([("kind", KernelJson::String("word".into())), ("moves", moves(letters)?)]))
        }
        "star" if letters.is_empty() => Some(kernel_json::object([
            ("kind", KernelJson::String("star".into())),
            ("node", KernelJson::Int(node)),
            ("orbit", KernelJson::Int(orbit)),
            ("sign", KernelJson::Int(sign)),
        ])),
        _ => None,
    }
}

/// The recipe of an `effect` read in 0.4's JSON form, from the argument's canonical bytes (DET.59): a word or a list of
/// integers gives one word step; a list of step records gives their steps in order. None for any other form.
pub fn effect_recipe(arg: &[u8]) -> Option<KernelJson> {
    recipe_of(&provenance::parse(arg)?)
}

/// A recipe in one of the `effect` row's two forms, read from a parsed value, in 0.4's JSON form; None for any other
/// form.
fn recipe_of(value: &Json) -> Option<KernelJson> {
    let Json::List(items) = value else { return None };
    // No step record reads as a letter, so the two forms never overlap; an empty list is the empty word.
    if let Some(letters) = moves(items) {
        let step = kernel_json::object([("kind", KernelJson::String("word".into())), ("moves", letters)]);
        return Some(KernelJson::Array(vec![step]));
    }
    items.iter().map(recipe_step).collect::<Option<Vec<_>>>().map(KernelJson::Array)
}

/// The logical size of an empty `effect` answer, and what each field adds to an answer at most, in
/// [`EFFECT_FIELDS`]'s order: `source` and `destination` with all 259,800 slots, each 259,799; `support` with
/// [`MAX_SUPPORT_ROWS`] rows whose integers are 259,799; the digests as 64 characters; `moved_stickers` 259,800,
/// `moved_pieces` 177,120 and `primitive_count` [`MAX_PRIMITIVES`].
fn effect_sizes() -> &'static (u128, [u128; 8]) {
    static SIZES: OnceLock<(u128, [u128; 8])> = OnceLock::new();
    SIZES.get_or_init(|| {
        let largest = Value::int((SLOTS - 1) as i64);
        let empty = Value::record(Vec::new()).size();
        let empty_list = Value::list(Vec::new()).size();
        let item = |value: Value| Value::list(vec![value]).size() - empty_list;
        let field = |value: Value| Value::record(vec![("field", value)]).size() - empty;
        let map = field(Value::list(Vec::new())) + (SLOTS as u128) * item(largest.clone());
        let row = Value::record(SUPPORT_FIELDS.iter().map(|&name| (name, largest.clone())).collect());
        let support = field(Value::list(Vec::new())) + (MAX_SUPPORT_ROWS as u128) * item(row);
        let digest = field(Value::text(&"0".repeat(64)));
        let size = |f: EffectField| match f {
            EffectField::Destination | EffectField::Source => map,
            EffectField::ExpansionSha256 | EffectField::NetSha256 => digest,
            EffectField::MovedPieces => field(Value::int(PIECES as u64)),
            EffectField::MovedStickers => field(Value::int(SLOTS as u64)),
            EffectField::PrimitiveCount => field(Value::int(MAX_PRIMITIVES as u64)),
            EffectField::Support => support,
        };
        (empty, EFFECT_FIELDS.map(|(_, f)| size(f)))
    })
}

/// The `effect` row's draft tariff for a recipe of `primitives` primitives and an answer with `fields` (DET.55; the
/// command table prices the row when it freezes). The kernel cost counts one pass over the 259,800 slots of the map and
/// one unit for each primitive of the expansion, as a proposal costs the length of its literal word (KN5.9). The
/// result bound is the logical size of the largest answer with these fields ([`effect_sizes`]).
fn effect_tariff_of(primitives: usize, fields: &[EffectField]) -> RowTariff {
    let (empty, sizes) = effect_sizes();
    let size = |field: &EffectField| sizes[EFFECT_FIELDS.iter().position(|(_, f)| f == field).expect("a field")];
    let result_bound = fields.iter().map(size).fold(*empty, u128::saturating_add);
    RowTariff { kernel_cost: (SLOTS as u128).saturating_add(primitives as u128), result_bound }
}

/// The `effect` row's draft tariff for a recipe of `primitives` primitives and an answer of the declared type `ty`;
/// None for a type the row does not answer.
pub fn effect_tariff(primitives: usize, ty: &QTy) -> Option<RowTariff> {
    effect_shape(ty).map(|shape| effect_tariff_of(primitives, &shape.fields))
}

/// The tariff of an `effect` read, or the refusal that costs the read's step only: `LG2:type.mismatch` for another
/// declared type, `KN5:input.argument_form` for an argument in neither form. The kernel normalizes the recipe to count
/// its primitives; a recipe it refuses is charged for no primitive, and the read then refuses it with the kernel's code.
pub fn effect_read_tariff(kernel: &Kernel, arg: &[u8], ty: &QTy) -> Result<RowTariff, &'static str> {
    let shape = effect_shape(ty).ok_or(TYPE_MISMATCH)?;
    let recipe = effect_recipe(arg).ok_or(code::ARGUMENT_FORM)?;
    let primitives = kernel.recipe(&recipe).map_or(0, |recipe| recipe.primitive_count());
    Ok(effect_tariff_of(primitives, &shape.fields))
}

/// The answer record of an effect with the declared shape.
fn effect_record(effect: &Effect, shape: &EffectShape) -> Value {
    let slots = |slots: &[i32]| Value::list(slots.iter().map(|&slot| Value::int(slot)).collect());
    let row = |row: &c600_kernel::SupportRow| {
        let value = |name: &str| match name {
            "orbit" => Value::int(row.orbit),
            "pieces" => Value::int(row.pieces as u64),
            _ => Value::int(row.stickers as u64),
        };
        Value::record(shape.support.iter().map(|&name| (name, value(name))).collect())
    };
    let value = |field: EffectField| match field {
        EffectField::Destination => slots(effect.destination()),
        EffectField::ExpansionSha256 => Value::text(&effect.expansion_sha256),
        EffectField::MovedPieces => Value::int(effect.moved_pieces as u64),
        EffectField::MovedStickers => Value::int(effect.moved_stickers() as u64),
        EffectField::NetSha256 => Value::text(&effect.net_sha256),
        EffectField::PrimitiveCount => Value::int(effect.primitive_count as u64),
        EffectField::Source => slots(effect.source()),
        EffectField::Support => Value::list(effect.support.iter().map(row).collect()),
    };
    let name =
        |field: EffectField| EFFECT_FIELDS.iter().find(|(_, f)| *f == field).map(|&(name, _)| name).expect("a field");
    Value::record(shape.fields.iter().map(|&field| (name(field), value(field))).collect())
}

/// The answer of an `effect` read, with [`effect_read_tariff`]'s refusals, the refusal with Jumble on (LB2.1) and the
/// kernel's refusal of the recipe. The effect reads no state, and a read changes nothing.
pub fn effect<S: Store>(layer: &CommandLayer<S>, arg: &[u8], ty: &QTy) -> Result<Value, &'static str> {
    let shape = effect_shape(ty).ok_or(TYPE_MISMATCH)?;
    let recipe = effect_recipe(arg).ok_or(code::ARGUMENT_FORM)?;
    if layer.jumbled().is_some() {
        return Err(code::RETAINED_TOOL_NEEDS_JUMBLE_OFF);
    }
    let kernel = layer.kernel();
    let recipe = kernel.recipe(&recipe).map_err(|refusal| refusal.code)?;
    let effect = kernel.effect(&recipe).map_err(|error| match error {
        c600_kernel::Error::Refusal(refusal) => refusal.code,
        c600_kernel::Error::Certificate(_) => CERTIFICATE,
    })?;
    Ok(effect_record(&effect, &shape))
}

/// The `protection` row's query: KN2.4's `protection`.
pub const PROTECTION: &str = "protection";

/// The generators of 600-cell-Full, ±1 to ±1,200; a letter and its inverse move the same slots. A Strict check finds
/// the slots each generator its word uses moves, at most once per generator.
pub const GENERATORS: usize = 1200;

/// The level the `protection` row judges at: 600-cell-Full's labels (KN2.4 types the row by puzzle and level).
pub const PROTECTION_LEVEL: &str = "labels";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProtectionField {
    Checked,
    Conflicts,
    ExpansionSha256,
    Level,
    PrimitiveCount,
    RefusedPrefix,
}

/// The fields of the `protection` row's record, by name.
const PROTECTION_FIELDS: [(&str, ProtectionField); 6] = [
    ("checked", ProtectionField::Checked),
    ("conflicts", ProtectionField::Conflicts),
    ("expansion_sha256", ProtectionField::ExpansionSha256),
    ("level", ProtectionField::Level),
    ("primitive_count", ProtectionField::PrimitiveCount),
    ("refused_prefix", ProtectionField::RefusedPrefix),
];

/// The fields of a declared `protection` answer type in its order; None unless it is a record with the row's six
/// fields, each declared with a type that holds its values, and no display qualifier.
fn protection_fields(ty: &QTy) -> Option<Vec<ProtectionField>> {
    let Ty::Record(fields) = &ty.ty else { return None };
    if ty.display || fields.len() != PROTECTION_FIELDS.len() {
        return None;
    }
    let mut order: Vec<ProtectionField> = Vec::with_capacity(PROTECTION_FIELDS.len());
    for (name, declared) in fields {
        let &(_, field) = PROTECTION_FIELDS.iter().find(|(known, _)| known == name)?;
        let fits = match field {
            ProtectionField::Checked | ProtectionField::PrimitiveCount => whole(declared),
            ProtectionField::Conflicts => whole_list(declared),
            ProtectionField::ExpansionSha256 | ProtectionField::Level => *declared == exact(Ty::Text),
            ProtectionField::RefusedPrefix => *declared == exact(Ty::Int),
        };
        if order.contains(&field) || !fits {
            return None;
        }
        order.push(field);
    }
    Some(order)
}

/// The type the `protection` row answers, with its fields in name order:
/// `record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, primitive_count: Nat,
/// refused_prefix: Int }`.
pub fn protection_type() -> QTy {
    let field = |field: ProtectionField| match field {
        ProtectionField::Checked | ProtectionField::PrimitiveCount => exact(Ty::Nat),
        ProtectionField::Conflicts => list(Ty::Nat),
        ProtectionField::ExpansionSha256 | ProtectionField::Level => exact(Ty::Text),
        ProtectionField::RefusedPrefix => exact(Ty::Int),
    };
    exact(Ty::Record(PROTECTION_FIELDS.iter().map(|&(name, f)| (name.to_owned(), field(f))).collect()))
}

/// What a `protection` read judges: Net, the end of the complete operation, or Strict, every executed prefix up to
/// `limit` letters (all of them when there is no limit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Net,
    Strict { limit: Option<u64> },
}

/// A `protection` read's argument: a captured lock's pairs, in its order, the mode and the recipe in 0.4's JSON form.
struct Check {
    pairs: Vec<(i64, i64)>,
    mode: Mode,
    recipe: KernelJson,
}

/// The integers of a list of exact integers in an argument's canonical bytes, saturated to `i64`.
fn exact_ints(value: &Json) -> Option<Vec<i64>> {
    let Json::List(items) = value else { return None };
    items.iter().map(exact_int).collect()
}

/// The pairs of a captured lock, `record { kind: Text, labels: List<Nat>, slots: List<Nat> }`: `slots[i]` must hold
/// the label `labels[i]`. None for another form or for lists of different lengths.
fn lock_pairs(lock: &Json) -> Option<Vec<(i64, i64)>> {
    let Json::Object(fields) = lock else { return None };
    let names = ["kind", "labels", "slots"];
    if fields.len() != names.len() || !names.iter().all(|name| fields.contains_key(*name)) {
        return None;
    }
    fields["kind"].as_text()?;
    let (slots, labels) = (exact_ints(&fields["slots"])?, exact_ints(&fields["labels"])?);
    (slots.len() == labels.len()).then(|| slots.into_iter().zip(labels).collect())
}

/// A `protection` read's argument from its canonical bytes (DET.59): a record `{ lock, mode: "net", recipe }`, or
/// `{ lock, mode: "strict", recipe }` with an optional `limit: Nat`. The recipe is in one of the `effect` row's two
/// forms. None for any other form.
fn protection_check(arg: &[u8]) -> Option<Check> {
    let Json::Object(fields) = provenance::parse(arg)? else { return None };
    let mode = match (fields.get("mode")?.as_text()?, fields.get("limit")) {
        ("net", None) => Mode::Net,
        ("strict", None) => Mode::Strict { limit: None },
        ("strict", Some(limit)) => Mode::Strict { limit: Some(u64::try_from(exact_int(limit)?).ok()?) },
        _ => return None,
    };
    let expected = if matches!(mode, Mode::Strict { limit: Some(_) }) { 4 } else { 3 };
    if fields.len() != expected {
        return None;
    }
    let pairs = lock_pairs(fields.get("lock")?)?;
    let recipe = recipe_of(fields.get("recipe")?)?;
    Some(Check { pairs, mode, recipe })
}

/// The logical size of an empty `protection` answer with all six fields at their largest but no conflict, and what
/// one conflict adds at most: a slot of 259,799.
fn protection_sizes() -> (u128, u128) {
    static SIZES: OnceLock<(u128, u128)> = OnceLock::new();
    *SIZES.get_or_init(|| {
        let most = Value::int(MAX_PRIMITIVES as u64 + 1);
        let value = |field: ProtectionField| match field {
            ProtectionField::Checked | ProtectionField::PrimitiveCount | ProtectionField::RefusedPrefix => most.clone(),
            ProtectionField::Conflicts => Value::list(Vec::new()),
            ProtectionField::ExpansionSha256 => Value::text(&"0".repeat(64)),
            ProtectionField::Level => Value::text(PROTECTION_LEVEL),
        };
        let empty = Value::record(PROTECTION_FIELDS.iter().map(|&(name, f)| (name, value(f))).collect()).size();
        let (none, one) = (Value::list(Vec::new()).size(), Value::list(vec![Value::int((SLOTS - 1) as i64)]).size());
        (empty, one - none)
    })
}

/// The `protection` row's draft tariff for a lock of `pairs` pairs and a recipe of `primitives` primitives (DET.55;
/// the command table prices the row when it freezes). The kernel cost counts the lock's pairs, one unit per primitive
/// of the expansion, one pass over the 259,800 slots for the net map or the held slots, and for Strict one more pass
/// for each generator the word may use, at most [`GENERATORS`]. The result bound is the logical size of an answer whose
/// integers are the largest a field holds and whose conflicts are all the pairs, each slot 259,799.
fn protection_tariff_of(pairs: usize, primitives: usize, mode: Mode) -> RowTariff {
    let generators = match mode {
        Mode::Net => 0,
        Mode::Strict { .. } => primitives.min(GENERATORS) as u128,
    };
    let passes = (SLOTS as u128).saturating_mul(generators.saturating_add(1));
    let kernel_cost = (pairs as u128).saturating_add(primitives as u128).saturating_add(passes);
    let (empty, conflict) = protection_sizes();
    RowTariff { kernel_cost, result_bound: empty.saturating_add((pairs as u128).saturating_mul(conflict)) }
}

/// The `protection` row's draft tariff for a lock of `pairs` pairs, a recipe of `primitives` primitives and the mode
/// (`strict` true for Strict).
pub fn protection_tariff(pairs: usize, primitives: usize, strict: bool) -> RowTariff {
    protection_tariff_of(pairs, primitives, if strict { Mode::Strict { limit: None } } else { Mode::Net })
}

/// The tariff of a `protection` read, or the refusal that costs the read's step only: `LG2:type.mismatch` for another
/// declared type, `KN5:input.argument_form` for an argument of another form. The kernel normalizes the recipe to count
/// its primitives; a recipe it refuses is charged for no primitive, and the read then refuses it with the kernel's code.
pub fn protection_read_tariff(kernel: &Kernel, arg: &[u8], ty: &QTy) -> Result<RowTariff, &'static str> {
    protection_fields(ty).ok_or(TYPE_MISMATCH)?;
    let check = protection_check(arg).ok_or(code::ARGUMENT_FORM)?;
    let primitives = kernel.recipe(&check.recipe).map_or(0, |recipe| recipe.primitive_count());
    Ok(protection_tariff_of(check.pairs.len(), primitives, check.mode))
}

/// A slot or a label of a lock as an index, or None outside 0 to 259,799.
fn slot_index(n: i64) -> Option<usize> {
    usize::try_from(n).ok().filter(|&n| n < SLOTS)
}

/// The judgement of a `protection` read: the prefixes judged, counting the empty one, the lock's slots that fail at
/// the judged prefix that fails, in the lock's order, and that prefix's length (Strict only).
struct Judgement {
    checked: u64,
    conflicts: Vec<i64>,
    refused_prefix: Option<u64>,
}

/// Net: the end of the complete operation. The sticker that ends in slot `p` started in `σ⁻¹(p)`, so the label there is
/// the base's label at `σ⁻¹(p)`.
fn net(labels: &[i32], effect: &Effect, pairs: &[(usize, i64)]) -> Judgement {
    let mut from: Vec<i32> = (0..SLOTS as i32).collect();
    for (&source, &destination) in effect.source().iter().zip(effect.destination()) {
        from[destination as usize] = source;
    }
    let ends = |&(slot, label): &(usize, i64)| i64::from(labels[from[slot] as usize]) != label;
    let conflicts = pairs.iter().filter(|pair| ends(pair)).map(|&(slot, _)| slot as i64).collect();
    Judgement { checked: 1, conflicts, refused_prefix: None }
}

/// Strict: every executed prefix up to `limit` letters, the empty one first. Letters run from source to destination
/// and labels are distinct, so while the lock holds, the next prefix breaks it exactly when its last letter moves a
/// locked slot; the first refused prefix ends at the first such letter, and its conflicts are the locked slots that
/// letter moves.
fn strict(
    kernel: &Kernel,
    labels: &[i32],
    recipe: &c600_kernel::Recipe,
    pairs: &[(usize, i64)],
    limit: Option<u64>,
) -> Result<Judgement, &'static str> {
    let failing: Vec<i64> =
        pairs.iter().filter(|&&(slot, label)| i64::from(labels[slot]) != label).map(|&(slot, _)| slot as i64).collect();
    if !failing.is_empty() {
        return Ok(Judgement { checked: 1, conflicts: failing, refused_prefix: Some(0) });
    }
    let mut held = vec![false; SLOTS];
    for &(slot, _) in pairs {
        held[slot] = true;
    }
    let letters = kernel.letters(recipe).map_err(|refusal| refusal.code)?;
    let end = limit.map_or(letters.len(), |limit| letters.len().min(usize::try_from(limit).unwrap_or(usize::MAX)));
    // The slots generator |m| moves, found once per generator.
    let mut moved: Vec<Option<Vec<i32>>> = vec![None; GENERATORS];
    for (index, &m) in letters[..end].iter().enumerate() {
        let seen = &mut moved[(m.unsigned_abs() - 1) as usize];
        if seen.is_none() {
            let word = kernel.word(&[i64::from(m.unsigned_abs())]).map_err(|refusal| refusal.code)?;
            let effect = kernel.effect(&word).map_err(|error| match error {
                c600_kernel::Error::Refusal(refusal) => refusal.code,
                c600_kernel::Error::Certificate(_) => CERTIFICATE,
            })?;
            *seen = Some(effect.source().to_vec());
        }
        let slots = seen.as_deref().expect("found");
        if slots.iter().any(|&slot| held[slot as usize]) {
            let conflicts = pairs
                .iter()
                .filter(|&&(slot, _)| slots.binary_search(&(slot as i32)).is_ok())
                .map(|&(slot, _)| slot as i64)
                .collect();
            let prefix = index as u64 + 1;
            return Ok(Judgement { checked: prefix + 1, conflicts, refused_prefix: Some(prefix) });
        }
    }
    Ok(Judgement { checked: end as u64 + 1, conflicts: Vec::new(), refused_prefix: None })
}

/// The answer of a `protection` read at `layer`'s head, with [`protection_read_tariff`]'s refusals, the refusal with
/// Jumble on (LB2.1), the kernel's refusal of the recipe and the refusal of a slot or a label outside 0 to 259,799. A
/// read changes nothing.
pub fn protection<S: Store>(layer: &CommandLayer<S>, arg: &[u8], ty: &QTy) -> Result<Value, &'static str> {
    let order = protection_fields(ty).ok_or(TYPE_MISMATCH)?;
    let check = protection_check(arg).ok_or(code::ARGUMENT_FORM)?;
    if layer.jumbled().is_some() {
        return Err(code::RETAINED_TOOL_NEEDS_JUMBLE_OFF);
    }
    let kernel = layer.kernel();
    let recipe = kernel.recipe(&check.recipe).map_err(|refusal| refusal.code)?;
    let named = |&(slot, label): &(i64, i64)| Some((slot_index(slot)?, slot_index(label).map(|_| label)?));
    let pairs: Vec<(usize, i64)> =
        check.pairs.iter().map(named).collect::<Option<_>>().ok_or(code::NAMED_OBJECT_INVALID)?;
    let effect = kernel.effect(&recipe).map_err(|error| match error {
        c600_kernel::Error::Refusal(refusal) => refusal.code,
        c600_kernel::Error::Certificate(_) => CERTIFICATE,
    })?;
    let snapshot = layer.snapshot();
    let labels = snapshot.labels();
    let judgement = match check.mode {
        Mode::Net => net(labels, &effect, &pairs),
        Mode::Strict { limit } => strict(kernel, labels, &recipe, &pairs, limit)?,
    };
    let value = |field: ProtectionField| match field {
        ProtectionField::Checked => Value::int(judgement.checked),
        ProtectionField::Conflicts => Value::list(judgement.conflicts.iter().map(|&slot| Value::int(slot)).collect()),
        ProtectionField::ExpansionSha256 => Value::text(&effect.expansion_sha256),
        ProtectionField::Level => Value::text(PROTECTION_LEVEL),
        ProtectionField::PrimitiveCount => Value::int(effect.primitive_count as u64),
        ProtectionField::RefusedPrefix => Value::int(judgement.refused_prefix.map_or(-1, |prefix| prefix as i64)),
    };
    let name = |field: ProtectionField| {
        PROTECTION_FIELDS.iter().find(|(_, f)| *f == field).map(|&(name, _)| name).expect("a field")
    };
    Ok(Value::record(order.iter().map(|&field| (name(field), value(field))).collect()))
}
