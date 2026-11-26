//! Value inspection (EV2.2; LP4-5): `inspect_value(run, value_reference) -> { value, type, identity, level, origin }`.
//!
//! An inspection keeps the value as the run holds it and says what kind of value it is. Exact values, approximate
//! display values with their precision, KN0.4's partial results and KN0.5's verdicts are distinct classes, and an
//! unestablished verdict stays its own outcome, never a failure. The shown text is a display: it may be cut at
//! [`SHOWN_LIMIT`] bytes and then says so, and an inspection has no equality, so a rendered coincidence or a cut text
//! never becomes equality or a proved claim (INV-4 to INV-6). Inspection runs no getter and no program code.
//!
//! # Drafts
//!
//! - **Type names** follow the surface spelling of chapter 02's types for the value as the run holds it; a record
//!   lists its fields' types.
//! - **Partial results.** A `Partial<T>` value of the identity tower (T1 draft) is read as KN0.4's partial result by
//!   its constructor, `total`, `partial` or `undefined`, and so is a record whose `result` field is the tag or text of
//!   one of these, as the jumbling rows answer.
//! - **The identity tower** (T1 drafts). Type names carry their puzzle, level, goal and unit indices as the checker
//!   writes them (`Piece<Full>`, `Digest<Full, labels>`, `WordEqualityKey<Jumbling>`). A verdict of the tower is read
//!   as KN0.5's verdict with its row `compare` and its constructor as the outcome; a host's handle and a certificate
//!   have no canonical form and are opaque, and a handle's id is never shown.
//! - **Identity and level.** A Full word names the session's pinned identity at level `labels`, and a menu word its
//!   own alphabet's identity block at level `poses`; other values carry none.

use c600_run::encode::encode;
use c600_run::meter::SourceRange;
use c600_run::step::Slot;
use c600_run::tower::{HandleTy, IdKind, SumTy};
use c600_run::value::Value;

use crate::{Identity, Pinned};

/// The most bytes of display text an inspection shows.
pub const SHOWN_LIMIT: usize = 4096;

/// KN0.4's three forms of a partial result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartialForm {
    Total,
    Partial,
    Undefined,
}

/// What kind of value an inspection shows (EV2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Class {
    /// An exact value.
    Exact,
    /// An approximate display value and its precision in bits.
    Approximate { precision: u32 },
    /// KN0.4's typed partial result.
    Partial(PartialForm),
    /// KN0.5's verdict: an end test and its outcome, `holds`, `fails` or `unestablished`.
    Verdict { test: &'static str, outcome: &'static str },
    /// A value with no canonical form to show: a function, closure, generator, stream or receipt.
    Opaque,
}

/// The display text of a value, and whether it was cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub text: String,
    pub truncated: bool,
}

/// Where an inspected value came from: the stop it was read at, its slot and source name, and the function and source
/// range of the stop's operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    pub stop: String,
    pub name: String,
    pub slot: Option<Slot>,
    pub module: Option<String>,
    pub function: Option<String>,
    pub source: Option<SourceRange>,
}

/// One inspection (EV2.2).
#[derive(Clone, Debug)]
pub struct Inspection {
    pub value: Value,
    pub ty: String,
    pub class: Class,
    pub shown: Shown,
    pub identity: Option<Identity>,
    pub level: Option<String>,
    pub origin: Origin,
}

/// Inspects `value`, read at `origin`, in a session whose pinned input is `input`.
pub fn inspect(value: &Value, origin: Origin, input: &Pinned) -> Inspection {
    let (identity, level) = match value {
        Value::Word(_) | Value::Compressed(_) | Value::WordEqualityKey(_) => {
            (Some(input.identity.clone()), Some("labels".to_owned()))
        }
        Value::MenuWord(w) => {
            let block = w.identity();
            let identity = Identity {
                puzzle: block.puzzle.clone(),
                model: block.model_id.clone(),
                menu: Some(block.menu_id.clone()),
                menu_order: Some(block.menu_order.clone()),
                contract: Some(block.contract_rev.clone()),
            };
            (Some(identity), Some("poses".to_owned()))
        }
        _ => (None, None),
    };
    Inspection {
        value: value.clone(),
        ty: type_name(value),
        class: class(value),
        shown: shown(value),
        identity,
        level,
        origin,
    }
}

/// The class of `value` (EV2.2).
pub fn class(value: &Value) -> Class {
    match value {
        Value::Approx(a) => Class::Approximate { precision: a.precision },
        Value::ApproxBool(_) => Class::Approximate { precision: 1 },
        Value::EndTest(test, outcome) => Class::Verdict { test: test.row(), outcome: outcome.tag() },
        Value::Record(r) => match r.get("result") {
            Some(Value::Tag(t) | Value::Text(t)) => partial_form(t.as_str()).map_or(Class::Exact, Class::Partial),
            _ => Class::Exact,
        },
        Value::Sum(s) => match s.ty() {
            SumTy::Partial => partial_form(s.ctor()).map_or(Class::Exact, Class::Partial),
            SumTy::Verdict(..) => {
                let ctors = s.ty().builtin().unwrap_or_default();
                match ctors.into_iter().find(|(name, _)| *name == s.ctor()) {
                    Some((outcome, _)) => Class::Verdict { test: "compare", outcome },
                    None => Class::Exact,
                }
            }
            _ => Class::Exact,
        },
        Value::Fn(_)
        | Value::Closure(_)
        | Value::Generator(_)
        | Value::WordStream(_)
        | Value::Receipt(_)
        | Value::Bytes(_)
        | Value::Handle(_)
        | Value::Holds(_) => Class::Opaque,
        _ => Class::Exact,
    }
}

/// KN0.4's form of a partial result by its constructor or result tag.
fn partial_form(name: &str) -> Option<PartialForm> {
    match name {
        "total" => Some(PartialForm::Total),
        "partial" => Some(PartialForm::Partial),
        "undefined" => Some(PartialForm::Undefined),
        _ => None,
    }
}

/// Chapter 02's type of `value` as the run holds it (see "Type names").
pub fn type_name(value: &Value) -> String {
    match value {
        Value::Unit => "Unit".into(),
        Value::Bool(_) => "Bool".into(),
        Value::Int(_) => "Int".into(),
        Value::Rat(_) => "Rat".into(),
        Value::Q5(_) => "Q5".into(),
        Value::Approx(a) => format!("Approx<{}>", a.precision),
        Value::ApproxBool(_) => "ApproxBool".into(),
        Value::Text(_) => "Text".into(),
        Value::Target(_) => "Target".into(),
        Value::Word(_) => "Word".into(),
        Value::Tag(_) => "Status".into(),
        Value::Colour(_) => "Colour".into(),
        Value::Record(r) => {
            let fields: Vec<String> =
                r.fields().iter().map(|(name, v)| format!("{}: {}", name.as_str(), type_name(v))).collect();
            format!("record {{ {} }}", fields.join(", "))
        }
        Value::Closure(_) | Value::Fn(_) => "Fn".into(),
        Value::Receipt(_) => "Receipt".into(),
        Value::List(l) => format!("List<{}>", l.items().first().map_or("_".into(), type_name)),
        Value::WordEqualityKey(_) => "WordEqualityKey<Full>".into(),
        Value::MenuWordKey(_) => "WordEqualityKey<Jumbling>".into(),
        Value::Set(l) => format!("Set<{}>", l.items().first().map_or("_".into(), type_name)),
        Value::Map(_) => "Map".into(),
        Value::Compressed(_) => "CompressedWord".into(),
        Value::MenuWord(_) => "MenuWord".into(),
        Value::EndTest(test, _) => test.type_name().into(),
        Value::TraceEqualityKey(_) => "TraceEqualityKey".into(),
        Value::Generator(_) => "Generator".into(),
        Value::Field(f, _) => format!("{f:?}"),
        Value::Bytes(_) => "Bytes".into(),
        Value::WordStream(_) => "WordStream".into(),
        Value::Id(kind, puzzle, _) => {
            let name = match kind {
                IdKind::Piece => "Piece",
                IdKind::Sticker => "Sticker",
                IdKind::Slot => "Slot",
            };
            format!("{name}<{}>", puzzle.name())
        }
        Value::PoseTableIndex(_) => "PoseTableIndex".into(),
        Value::Digest(d) => format!("Digest<{}, {}>", d.puzzle().name(), d.level().name()),
        Value::Cost(puzzle, unit, _) => format!("Cost<{}, {}>", puzzle.name(), unit.name()),
        Value::Holds(h) => format!("Holds<{}, {}>", h.puzzle().name(), h.goal().name()),
        Value::Handle(h) => match h.ty() {
            HandleTy::State(p) => format!("State<{}>", p.name()),
            HandleTy::LevelValue(p, l) => format!("LevelValue<{}, {}>", p.name(), l.name()),
            HandleTy::PoseTable => "PoseTable".into(),
            HandleTy::Evidence => "Evidence".into(),
            HandleTy::Pose => "Pose".into(),
        },
        Value::Sum(s) => match s.ty() {
            SumTy::Verdict(p, l, None) => format!("Verdict<{}, {}>", p.name(), l.name()),
            SumTy::Verdict(p, l, Some(g)) => format!("Verdict<{}, {}, {}>", p.name(), l.name(), g.name()),
            SumTy::Partial => format!("Partial<{}>", s.payload().first().map_or("_".into(), type_name)),
            SumTy::Reachability(p) => format!("Reachability<{}>", p.name()),
            SumTy::Derivation(p) => format!("Derivation<{}>", p.name()),
            SumTy::Named { module, name } => format!("{module}.{name}"),
        },
    }
}

/// The display text of `value`: its canonical encoding when it has one, cut at [`SHOWN_LIMIT`] bytes on a scalar
/// boundary, and a description otherwise.
pub fn shown(value: &Value) -> Shown {
    let mut bytes = Vec::new();
    let text = match encode(value, &mut bytes) {
        Ok(()) => String::from_utf8_lossy(&bytes).trim_end().to_owned(),
        Err(_) => match value {
            Value::Compressed(c) => format!("compressed word of {} parts, length {}", c.parts(), c.length()),
            other => format!("{} with no canonical form", type_name(other)),
        },
    };
    if text.len() <= SHOWN_LIMIT {
        return Shown { text, truncated: false };
    }
    let mut end = SHOWN_LIMIT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Shown { text: text[..end].to_owned(), truncated: true }
}
