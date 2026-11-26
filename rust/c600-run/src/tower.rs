//! The identity tower's run-time values (T1c; T1 drafts until the IR freeze, PHS.49): canonical IDs, digests,
//! costs, certificates, sum values and the opaque handles of the values a host makes (plan section 2).
//!
//! - **Canonical IDs** ([`Value::Id`]): a piece, a sticker or a slot of a puzzle and its index (KN1.5, LG2.2). Logical
//!   size 4 bytes, the index; the puzzle is the type's index and no payload. Canonical form
//!   `{"<kind>":{"index":N,"puzzle":"<P>"}}`. Key order: kind, then puzzle, then index.
//! - **Pose-table indices** ([`Value::PoseTableIndex`]): a per-revision index (KN2.9), 4 bytes. It has an exact
//!   equality and no canonical form, and it is no key, so no set or map holds one in a checked program; its order,
//!   by index, only keeps the key order total.
//! - **Digests** ([`Digest`]): a puzzle, a level and lowercase hex of whole bytes (KN1.8). Logical size: its bytes,
//!   two hex digits each, with no length (one type has one width). Canonical form
//!   `{"digest":{"hex":"<hex>","level":"<L>","puzzle":"<P>"}}`. Key order: puzzle, level, then hex.
//! - **Costs** ([`Value::Cost`]): a puzzle, a unit and an exact natural count (LG2.6). Logical size: the count as an
//!   unbounded integer. Canonical form `{"cost":{"count":{"int":"N"},"puzzle":"<P>","unit":"<U>"}}`. Key order:
//!   puzzle, unit, then count.
//! - **Sum values** ([`Sum`]): a built-in or declared sum type, a constructor and its payload values (LG2.1, KN0.4,
//!   KN0.5, KN2.13, KN2.11). Logical size as DET.57's finite algebraic record: a 4-byte variant tag, an 8-byte length
//!   per variable sequence among the payloads, and the payloads. Only a partial result has a canonical form,
//!   `{"<constructor>":[payload, …]}`, and only when its payloads have one: T1a makes a `Partial` of portable data
//!   portable, and a verdict, a claim, a derivation and a declared sum type host values. A sum value has no exact
//!   equality and is no key (T1a); its order (type, constructor, payloads) only keeps the key order total.
//! - **Certificates** ([`Holds`]): a puzzle, a goal, the subject's digest and, for the low-height goal, its bound
//!   (LG2.10, LG2.11). Logical size: a 4-byte tag, the subject digest's bytes and the bound as an integer. Only the host
//!   makes one, and `imply` takes one along LG2.11's edges; it has no canonical form, no equality and no key.
//! - **Handles** ([`Handle`]): the values a host makes and keeps, a state, a level value, a pose table, evidence or a
//!   pose (KN0.3, KN1.9, KN2.12, KN2.14, KN0.5, KN1.5). A handle holds its type with its indices, a host-assigned id
//!   and the logical size the host declared (DET.24, DET.57), and a pose's handle also the pose's exact key (KN1.5)
//!   as the host gives it. A handle has no canonical form: it is never published, stored or handed to an exact sink.
//!   It is never keyed or compared by its id: a pose compares and orders by its exact key, and every other handle has
//!   no equality and is no key.
//!
//! The names, the sizes, the canonical forms and the key orders above are drafts that change freely until the IR
//! freeze.

use std::cmp::Ordering;
use std::sync::Arc;

use c600_ir::{CostUnit, Goal, Level, Puzzle, ReachKind, Ty};
use num_bigint::{BigInt, Sign};

use crate::value::{int_size, take_children, text_bound, Record, Text, Value};

/// The kind of a canonical ID (KN1.5, LG2.2; T1 draft).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdKind {
    Piece,
    Sticker,
    Slot,
}

impl IdKind {
    pub const ALL: [IdKind; 3] = [IdKind::Piece, IdKind::Sticker, IdKind::Slot];

    /// The kind's name, the key of its canonical form.
    pub fn name(self) -> &'static str {
        match self {
            IdKind::Piece => "piece",
            IdKind::Sticker => "sticker",
            IdKind::Slot => "slot",
        }
    }

    /// The ID type of a puzzle.
    pub fn ty(self, puzzle: Puzzle) -> Ty {
        match self {
            IdKind::Piece => Ty::Piece(puzzle),
            IdKind::Sticker => Ty::Sticker(puzzle),
            IdKind::Slot => Ty::Slot(puzzle),
        }
    }
}

/// A digest that names its puzzle and level (KN1.8; T1 draft): lowercase hex of whole bytes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest {
    puzzle: Puzzle,
    level: Level,
    hex: Box<str>,
}

impl Digest {
    /// The digest `hex` of a value of `puzzle` at `level`. `None` unless `hex` is a nonempty even number of lowercase
    /// hex digits.
    pub fn new(puzzle: Puzzle, level: Level, hex: &str) -> Option<Digest> {
        let digits = hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        (digits && !hex.is_empty() && hex.len() % 2 == 0).then(|| Digest { puzzle, level, hex: hex.into() })
    }

    pub fn puzzle(&self) -> Puzzle {
        self.puzzle
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn hex(&self) -> &str {
        &self.hex
    }

    /// DET.57: the digest's bytes, two hex digits each. One type has one width, so no length is counted.
    pub fn size(&self) -> u128 {
        self.hex.len() as u128 / 2
    }

    /// An upper bound of the canonical encoding: 72 bytes of keys, names and punctuation, and the hex digits.
    pub(crate) fn bound(&self) -> u128 {
        72 + self.hex.len() as u128
    }
}

/// A certificate that a goal holds of a state (LG2.10, LG2.11; T1 draft): the goal, the subject state's digest, whose
/// puzzle is the certificate's, and the exact bound of the low-height goal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Holds {
    goal: Goal,
    subject: Digest,
    bound: Option<BigInt>,
}

impl Holds {
    /// A certificate of `goal` for the state whose digest is `subject`. `None` unless the bound is given exactly for
    /// the low-height goal and is not negative, and the goal is defined on the subject's puzzle.
    pub fn new(goal: Goal, subject: Digest, bound: Option<BigInt>) -> Option<Holds> {
        let bounded = match &bound {
            Some(b) => goal.takes_bound() && b.sign() != Sign::Minus,
            None => !goal.takes_bound(),
        };
        (bounded && goal.puzzles().contains(&subject.puzzle)).then_some(Holds { goal, subject, bound })
    }

    pub fn puzzle(&self) -> Puzzle {
        self.subject.puzzle
    }

    pub fn goal(&self) -> Goal {
        self.goal
    }

    pub fn subject(&self) -> &Digest {
        &self.subject
    }

    pub fn bound(&self) -> Option<&BigInt> {
        self.bound.as_ref()
    }

    /// `imply`'s certificate of `to` for the same subject (LG2.11; T1 draft), with `bound` for the low-height goal.
    /// `Err(true)` when no chain of LG2.11's edges leads from this goal to `to` or `to` is not defined on the puzzle,
    /// which the checker proves; `Err(false)` for a pointwise failure: a negative bound, or a low-height certificate
    /// taken to another bound, since every implication keeps the named goal parameters (LG2.11).
    pub fn implied(&self, to: Goal, bound: Option<&BigInt>) -> Result<Holds, bool> {
        if !self.goal.implies(to) || !to.puzzles().contains(&self.puzzle()) || to.takes_bound() != bound.is_some() {
            return Err(true);
        }
        if bound.is_some_and(|b| b.sign() == Sign::Minus) {
            return Err(false);
        }
        if self.goal == Goal::LowHeight && self.bound.as_ref() != bound {
            return Err(false);
        }
        Ok(Holds { goal: to, subject: self.subject.clone(), bound: bound.cloned() })
    }

    /// DET.57: a 4-byte tag, the subject digest's bytes and the bound as an unbounded integer.
    pub fn size(&self) -> u128 {
        4 + self.subject.size() + self.bound.as_ref().map_or(0, int_size)
    }
}

/// The type of a handle with its indices (T1 draft): the values a host makes and keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HandleTy {
    State(Puzzle),
    LevelValue(Puzzle, Level),
    PoseTable,
    Evidence,
    Pose,
}

impl HandleTy {
    /// The IR type of the handle's values.
    pub fn ty(self) -> Ty {
        match self {
            HandleTy::State(p) => Ty::State(p),
            HandleTy::LevelValue(p, l) => Ty::LevelValue(p, l),
            HandleTy::PoseTable => Ty::PoseTable,
            HandleTy::Evidence => Ty::Evidence,
            HandleTy::Pose => Ty::Pose,
        }
    }

    /// The handle type of an IR type, if its values are handles.
    pub fn of(ty: &Ty) -> Option<HandleTy> {
        Some(match ty {
            Ty::State(p) => HandleTy::State(*p),
            Ty::LevelValue(p, l) => HandleTy::LevelValue(*p, *l),
            Ty::PoseTable => HandleTy::PoseTable,
            Ty::Evidence => HandleTy::Evidence,
            Ty::Pose => HandleTy::Pose,
            _ => return None,
        })
    }
}

/// An opaque value a host made and keeps (T1 draft): its type, a host-assigned id, the logical size the host declared
/// (DET.57) and, for a pose, its exact key (KN1.5). The id names the value to its host only: a handle is never
/// published, stored, keyed or compared by it.
#[derive(Clone, Debug)]
pub struct Handle {
    ty: HandleTy,
    id: u64,
    size: u128,
    key: Option<Box<str>>,
}

impl Handle {
    /// A handle of type `ty` with the host's id and declared logical size.
    pub fn new(ty: HandleTy, id: u64, size: u128) -> Handle {
        Handle { ty, id, size, key: None }
    }

    /// The handle with a pose's exact key (KN1.5), by which poses compare and order. `None` for a handle of another
    /// type, which has no equality, and for an empty key.
    pub fn with_key(mut self, key: &str) -> Option<Handle> {
        if self.ty != HandleTy::Pose || key.is_empty() {
            return None;
        }
        self.key = Some(key.into());
        Some(self)
    }

    pub fn ty(&self) -> HandleTy {
        self.ty
    }

    /// The host-assigned id, which only the host reads.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The logical size the host declared (DET.57).
    pub fn size(&self) -> u128 {
        self.size
    }

    /// A pose's exact key.
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// The order of two handles: by type and indices, then by a pose's key, never by id.
    pub(crate) fn order(&self, other: &Handle) -> Ordering {
        (self.ty, &self.key).cmp(&(other.ty, &other.key))
    }

    /// Whether two handles are one value: two poses of one exact key. No other handle has an equality.
    pub(crate) fn same(&self, other: &Handle) -> bool {
        self.key.is_some() && self.ty == other.ty && self.key == other.key
    }
}

/// The type of a sum value (T1 draft): a built-in sum type with its indices, or a declared one by its defining module.
/// A partial result's type is not recorded: its payload carries it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SumTy {
    Verdict(Puzzle, Level, Option<Goal>),
    Partial,
    Reachability(Puzzle),
    Derivation(Puzzle),
    Named { module: Box<str>, name: Box<str> },
}

impl SumTy {
    /// The sum type of an IR type, if it is one.
    pub fn of(ty: &Ty) -> Option<SumTy> {
        Some(match ty {
            Ty::Verdict(p, l, g) => SumTy::Verdict(*p, *l, *g),
            Ty::Partial(_) => SumTy::Partial,
            Ty::Reachability(p) => SumTy::Reachability(*p),
            Ty::Derivation(p) => SumTy::Derivation(*p),
            Ty::Named { module, name } => SumTy::Named { module: module.as_str().into(), name: name.as_str().into() },
            _ => return None,
        })
    }

    /// The constructors of a built-in sum type with their payload counts, in declared order (`Ty::builtin_ctors`);
    /// `None` for a declared one.
    pub fn builtin(&self) -> Option<Vec<(&'static str, usize)>> {
        let ty = match self {
            SumTy::Verdict(p, l, g) => Ty::Verdict(*p, *l, *g),
            SumTy::Partial => Ty::Partial(Box::new(c600_ir::QTy { ty: Ty::Unit, display: false })),
            SumTy::Reachability(p) => Ty::Reachability(*p),
            SumTy::Derivation(p) => Ty::Derivation(*p),
            SumTy::Named { .. } => return None,
        };
        let ctors = ty.builtin_ctors().expect("a built-in sum type");
        Some(ctors.into_iter().map(|(name, payload)| (name, payload.len())).collect())
    }
}

/// A sum value (LG2.1; T1 draft): its type, its constructor and the constructor's payload values, with its logical size
/// and an upper bound of its canonical encoding.
#[derive(Debug)]
pub struct Sum {
    ty: SumTy,
    ctor: Arc<Text>,
    payload: Vec<Value>,
    size: u128,
    bound: u128,
}

impl Sum {
    /// The value of `ty` made by `ctor` from `payload`. Whether the constructor and the payload fit the type is the
    /// checker's and the loader's question, and [`fits`]'s for a host's answer.
    pub fn new(ty: SumTy, ctor: Arc<Text>, payload: Vec<Value>) -> Sum {
        // DET.57's finite algebraic record: a 4-byte variant tag, an 8-byte length per variable sequence and the
        // owned payloads.
        let size = payload
            .iter()
            .fold(4u128, |s, v| s.saturating_add(v.size()).saturating_add(if v.is_sequence() { 8 } else { 0 }));
        // `{"<constructor>":[p, …]}` for a partial result; no other sum value has a canonical form.
        let bound = match ty {
            SumTy::Partial => payload
                .iter()
                .fold(text_bound(&ctor).saturating_add(5), |s, v| s.saturating_add(v.bound()).saturating_add(1)),
            _ => 0,
        };
        Sum { ty, ctor, payload, size, bound }
    }

    pub fn ty(&self) -> &SumTy {
        &self.ty
    }

    pub fn ctor(&self) -> &str {
        self.ctor.as_str()
    }

    pub fn payload(&self) -> &[Value] {
        &self.payload
    }

    /// DET.57's logical size.
    pub fn size(&self) -> u128 {
        self.size
    }

    pub(crate) fn bound(&self) -> u128 {
        self.bound
    }

    /// The logical size of a sum value's header over these payloads: the variant tag and the sequence lengths.
    pub fn header(payload: &[Value]) -> u128 {
        Record::header(payload)
    }

    /// Whether the value has a canonical form at all: a partial result does (when its payloads do), no other sum.
    pub fn portable(&self) -> bool {
        self.ty == SumTy::Partial
    }

    /// The sum's payloads, moved out for an explicit-stack drop.
    pub(crate) fn take_payload(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.payload)
    }
}

impl Drop for Sum {
    fn drop(&mut self) {
        let mut stack = std::mem::take(&mut self.payload);
        while let Some(v) = stack.pop() {
            take_children(v, &mut stack);
        }
    }
}

impl Value {
    /// A canonical ID of `puzzle` (KN1.5; T1 draft).
    pub fn id(kind: IdKind, puzzle: Puzzle, index: u32) -> Value {
        Value::Id(kind, puzzle, index)
    }

    /// A cost of `puzzle` in `unit` (LG2.6; T1 draft); `None` for a negative count.
    pub fn cost(puzzle: Puzzle, unit: CostUnit, count: impl Into<BigInt>) -> Option<Value> {
        let count = count.into();
        (count.sign() != Sign::Minus).then(|| Value::Cost(puzzle, unit, Arc::new(count)))
    }

    /// A digest of `puzzle` at `level` (KN1.8; T1 draft); `None` unless `hex` is lowercase hex of whole bytes.
    pub fn digest(puzzle: Puzzle, level: Level, hex: &str) -> Option<Value> {
        Digest::new(puzzle, level, hex).map(|d| Value::Digest(Arc::new(d)))
    }

    /// A sum value of `ty` by its constructor `ctor` (T1 draft).
    pub fn sum(ty: SumTy, ctor: &str, payload: Vec<Value>) -> Value {
        Value::Sum(Arc::new(Sum::new(ty, Text::new(ctor), payload)))
    }

    /// A handle (T1 draft).
    pub fn handle(handle: Handle) -> Value {
        Value::Handle(Arc::new(handle))
    }

    /// A certificate (T1 draft).
    pub fn holds(holds: Holds) -> Value {
        Value::Holds(Arc::new(holds))
    }
}

/// Whether a host's answer has the declared type `ty` (T1 draft): the shape a kernel row's result must have, with
/// every puzzle, level, goal and unit index, each constructor of a built-in sum type with its payloads, each record
/// field by name in order and each collection element. A declared sum type is checked by its type only, since its
/// declaration is the program's; a type variable takes any value. The check descends one type level per call, so
/// its depth is the type's.
pub fn fits(value: &Value, ty: &Ty) -> bool {
    match (ty, value) {
        (Ty::Var(_), _) => true,
        (Ty::Unit, Value::Unit) | (Ty::Bool, Value::Bool(_)) | (Ty::ApproxBool, Value::ApproxBool(_)) => true,
        (Ty::Nat, Value::Int(n)) => n.sign() != Sign::Minus,
        (Ty::Int | Ty::Budget, Value::Int(_)) => true,
        (Ty::Rat, Value::Int(_) | Value::Rat(_)) | (Ty::Q5, Value::Int(_) | Value::Rat(_) | Value::Q5(_)) => true,
        (Ty::Approx(p), Value::Approx(x)) => x.precision == *p,
        (Ty::Text, Value::Text(_))
        | (Ty::Word, Value::Word(_))
        | (Ty::Target, Value::Target(_))
        | (Ty::Status, Value::Tag(_))
        | (Ty::Colour, Value::Colour(_))
        | (Ty::Receipt, Value::Receipt(_))
        | (Ty::Fn(_), Value::Fn(_) | Value::Closure(_))
        | (Ty::WordEqualityKey(Puzzle::Full), Value::WordEqualityKey(_))
        | (Ty::WordEqualityKey(Puzzle::Jumbling), Value::MenuWordKey(_))
        | (Ty::CompressedWord, Value::Compressed(_))
        | (Ty::MenuWord, Value::MenuWord(_))
        | (Ty::TraceEqualityKey, Value::TraceEqualityKey(_))
        | (Ty::Generator, Value::Generator(_))
        | (Ty::Bytes, Value::Bytes(_))
        | (Ty::WordStream, Value::WordStream(_))
        | (Ty::PoseTableIndex, Value::PoseTableIndex(_)) => true,
        (Ty::EndTest(t), Value::EndTest(u, _)) => t == u,
        (Ty::Field(f), Value::Field(g, _)) => f == g,
        (Ty::Record(fields), Value::Record(r)) => {
            fields.len() == r.fields().len()
                && fields.iter().zip(r.fields()).all(|((n, t), (m, v))| n == m.as_str() && fits(v, &t.ty))
        }
        (Ty::List(e), Value::List(l)) | (Ty::Set(e), Value::Set(l)) => l.items().iter().all(|v| fits(v, &e.ty)),
        (Ty::Map(k, v), Value::Map(m)) => m
            .items()
            .iter()
            .all(|e| fits(crate::value::entry_key(e), &k.ty) && fits(crate::value::entry_value(e), &v.ty)),
        (Ty::Piece(p), Value::Id(IdKind::Piece, q, _))
        | (Ty::Sticker(p), Value::Id(IdKind::Sticker, q, _))
        | (Ty::Slot(p), Value::Id(IdKind::Slot, q, _)) => p == q,
        (Ty::Digest(p, l), Value::Digest(d)) => d.puzzle == *p && d.level == *l,
        (Ty::Cost(p, u), Value::Cost(q, v, _)) => p == q && u == v,
        (Ty::Holds(p, g), Value::Holds(h)) => h.puzzle() == *p && h.goal == *g,
        (_, Value::Handle(h)) => HandleTy::of(ty) == Some(h.ty),
        (_, Value::Sum(s)) => {
            if SumTy::of(ty).as_ref() != Some(&s.ty) {
                return false;
            }
            // A declared type's constructors are its declaration's.
            let Some(ctors) = ty.builtin_ctors() else { return true };
            ctors.into_iter().any(|(name, payload)| {
                name == s.ctor()
                    && payload.len() == s.payload.len()
                    && payload.iter().zip(&s.payload).all(|(t, v)| fits(v, &t.ty))
            })
        }
        _ => false,
    }
}

/// The reachability kind a claim's constructor names (KN2.13; T1 draft).
pub fn reach_kind(sum: &Sum) -> Option<ReachKind> {
    match sum.ty {
        SumTy::Reachability(_) => ReachKind::of_name(sum.ctor()),
        _ => None,
    }
}
