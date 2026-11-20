//! The canonical transfer encoding of run-time values, IR types, host requests and sinks, and the meter's run
//! summary (draft `c600-rt1-wire-1`, KI.15's transfer-encoding hole).
//!
//! `c600_run::encode` writes DET.59's output bytes, which drop variants (a `Nat` and an `Int` look alike, a closure
//! has no form), so it cannot carry a value between processes. This encoding keeps every variant: a tag byte, then
//! the payload. It accepts only values in their canonical form (a rational in lowest terms with a positive
//! denominator, an element of Q(√5) with `d > 0` and `gcd(a, b, d) = 1`, a normalised approximation of a supported
//! precision, letters of magnitude 1 to 1200, records whose field names are distinct, a freely reduced word key, a
//! set's elements and a map's keys strictly increasing in the canonical key order, a compressed word's parts as
//! `Compressed::from_parts` takes them), so equal values have equal bytes. Values are walked with an explicit stack
//! in both directions, so a deep value cannot overflow either process's stack. A compressed word crosses as its parts
//! in order, by which compressed words compare, and the receiver rebuilds it with `from_parts` once the lengths that
//! the payload's compressed words derive are within [`COMPRESSED_LENGTH_BITS`] together (C3 draft 15). A menu word
//! (LB1.8; LP2 draft) crosses with the identity block of its alphabet, and a pinned menu alphabet with its inverse
//! items and, when it carries them, its menu's item matrices (LG2.18), so the receiver checks them as the interpreter
//! does. An end test's verdict (LB3.2; LP2 draft) crosses as its test's and its outcome's indices in their declared
//! orders, and its type with the test's index. A trace key (LB1.10; LP2 draft) crosses with its header and its letters,
//! and a pinned trace relation with its contract, its identity block, its generator count and its independence bits;
//! the receiver recomputes the relation's digest, and the run checks a key against its relation, since the codec cannot
//! order letters without one. A menu word key (LG2.18; LP2 draft) crosses with its header and its tokens in their
//! canonical form, and the run checks its header against its pinned menu. A generator (DET.61; LP2 draft) has no
//! transfer form, since a copy would be a second cursor on its stream; its type is type tag 27. An F2 or F5 element
//! (KN3.2; LP2 draft) crosses as its field and its residue, which is below the field's prime.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::Arc;

use c600_check::Row as ClassRow;
use c600_ir::{Capability, Effect, EffectBound, EndTest, EndTestOutcome, ExactRole, Field, FnTy, QTy, Ty};
use c600_run::approx::{self, Approx};
use c600_run::interp::{self, Request, RowTariff, Sink};
use c600_run::menu::{ItemMatrix, MenuAlphabet, MenuIdentity, MENU_INDEX_LIMIT};
use c600_run::menu_key::{MenuWordKey, Token, WORD_KEY};
use c600_run::meter::{
    self, Amounts, ChildRecord, DraftTariffs, Interruption, Outcome, Position, Row, SourceRange, Summary, Tariffs,
};
use c600_run::trace::{MenuHeader, Relation, TraceHeader, TraceKey, JUMBLE_LOCALITY};
use c600_run::value::{entry, value_order, Closure, Compressed, List, Part, Quad, Ratio, Record, Text, Value};
use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::wire::{Dec, Enc, Malformed, MAX_FRAME};

/// Why a value or type has no transfer encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unencodable(pub &'static str);

/// The deepest IR type the encoding carries. Types come from checked signatures, so the bound is generous.
pub const TYPE_DEPTH: usize = 64;

mod tag {
    pub const UNIT: u8 = 0;
    pub const BOOL: u8 = 1;
    pub const INT: u8 = 2;
    pub const RAT: u8 = 3;
    pub const Q5: u8 = 4;
    pub const APPROX: u8 = 5;
    pub const APPROX_BOOL: u8 = 6;
    pub const TEXT: u8 = 7;
    pub const TARGET: u8 = 8;
    pub const WORD: u8 = 9;
    pub const TAG: u8 = 10;
    pub const COLOUR: u8 = 11;
    pub const RECORD: u8 = 12;
    pub const CLOSURE: u8 = 13;
    pub const FN: u8 = 14;
    pub const RECEIPT: u8 = 15;
    /// LP2's draft collections (C3 draft): a list's elements in order, a word key's letters, a set's elements, and a
    /// map's entries, each as its key then its value; a set's elements and a map's keys in canonical key order.
    pub const LIST: u8 = 16;
    pub const WORD_KEY: u8 = 17;
    pub const SET: u8 = 18;
    pub const MAP: u8 = 19;
    /// LP2's draft compressed word (C3 draft 15): its part count, then each part in order, a leaf as its part tag and
    /// its letters, a concatenation as its part tag and its two part numbers.
    pub const COMPRESSED: u8 = 20;
    /// LP2's draft menu word (LB1.8): the identity block of its alphabet, its five fields in KN1.4's order, then the
    /// letter count and each letter as its cap and its item, both below 600.
    pub const MENU_WORD: u8 = 21;
    /// LP2's draft end-test verdict (LB3.2): the test's index in `EndTest::ALL`, then the outcome's in
    /// `EndTestOutcome::ALL`.
    pub const END_TEST: u8 = 22;
    /// LP2's draft trace key (LB1.10): its header, the puzzle, the model ID, the generator count, the key kind, the
    /// independence contract and the 32 digest bytes, then its letters as a word's.
    pub const TRACE_KEY: u8 = 23;
    /// LP2's draft F2 or F5 element (KN3.2): its field's byte (`field_byte`), then its residue's byte, below the field's
    /// prime.
    pub const FIELD: u8 = 24;
    /// LP2's draft captured bytes (DET.62): the octets, length-prefixed.
    pub const BYTES: u8 = 25;
    /// LP2's draft trace key of menu letters (LB1.10, `jumble-locality/1`): its header, the identity block's five
    /// fields in KN1.4's order, the grip count, the item count, the key kind, the independence contract and the 32
    /// digest bytes, then its letters as a menu word's.
    pub const MENU_TRACE_KEY: u8 = 26;
    /// LP2's draft word key of menu letters (LG2.18, `word-key/1`): its header, the identity block's five fields in
    /// KN1.4's order, the key kind, the grip count and the item count, then its tokens: their count, and each token's
    /// grip and the sixteen entries of its product row-major, each as its three integers. Only a key in its canonical
    /// form is written or read ([`MenuWordKey::canonical`]).
    pub const MENU_WORD_KEY: u8 = 27;
}

/// A finite field's byte in a value or a type (LP2 draft): 0 for F2, 1 for F5.
fn field_byte(field: Field) -> u8 {
    match field {
        Field::F2 => 0,
        Field::F5 => 1,
    }
}

fn field_from(byte: u8) -> Result<Field, Malformed> {
    match byte {
        0 => Ok(Field::F2),
        1 => Ok(Field::F5),
        _ => Err(Malformed::new("an unknown field")),
    }
}

/// The tags of a compressed word's parts.
mod part_tag {
    pub const LEAF: u8 = 0;
    pub const CONCAT: u8 = 1;
}

// ---------------------------------------------------------------------------------------------------------------
// Values

/// A menu alphabet's identity block (LB1.8; LP2 draft): its five fields in KN1.4's order. A block with an empty field
/// names no alphabet and is refused.
fn encode_identity(m: &MenuIdentity, e: &mut Enc) -> Result<(), Unencodable> {
    if !m.names_alphabet() {
        return Err(Unencodable("a menu word that names no alphabet"));
    }
    for (_, text) in m.fields() {
        e.text(text);
    }
    Ok(())
}

/// An identity block's five fields in KN1.4's order, as written.
fn read_identity(d: &mut Dec) -> Result<MenuIdentity, Malformed> {
    let mut field = || Ok::<_, Malformed>(d.text()?.to_string());
    Ok(MenuIdentity {
        puzzle: field()?,
        model_id: field()?,
        menu_id: field()?,
        menu_order: field()?,
        contract_rev: field()?,
    })
}

/// A menu word's identity block, refused when it names no alphabet.
fn decode_identity(d: &mut Dec) -> Result<MenuIdentity, Malformed> {
    let m = read_identity(d)?;
    if !m.names_alphabet() {
        return Err(Malformed::new("a menu word that names no alphabet"));
    }
    Ok(m)
}

/// A menu word's letters: the count, then each letter's cap and item, both below [`MENU_INDEX_LIMIT`].
fn encode_menu_letters(letters: &[[u32; 2]], e: &mut Enc) -> Result<(), Unencodable> {
    if letters.iter().flatten().any(|&i| i >= MENU_INDEX_LIMIT) {
        return Err(Unencodable("a menu index of 600 or more"));
    }
    e.count(letters.len());
    for [cap, item] in letters {
        e.u32(*cap);
        e.u32(*item);
    }
    Ok(())
}

fn decode_menu_letters(d: &mut Dec) -> Result<Vec<[u32; 2]>, Malformed> {
    let n = d.count(8)?;
    let mut letters = Vec::with_capacity(n);
    for _ in 0..n {
        letters.push([d.u32()?, d.u32()?]);
    }
    if letters.iter().flatten().any(|&i| i >= MENU_INDEX_LIMIT) {
        return Err(Malformed::new("a menu index of 600 or more"));
    }
    Ok(letters)
}

/// A trace key's header (LB1.10, LG2.23; LP2 draft): its text fields, refused when one is empty, as a relation's
/// header never is, and its generator count and digest. A menu key's header ([`encode_menu_trace_header`]) has the
/// whole identity block and the item count as well.
fn encode_trace_header(h: &TraceHeader, e: &mut Enc) -> Result<(), Unencodable> {
    if h.menu.is_some() {
        return Err(Unencodable("a Full trace key under a menu header"));
    }
    if [&h.puzzle, &h.model_id, &h.key, &h.contract].iter().any(|t| t.is_empty()) {
        return Err(Unencodable("a trace key header with an empty field"));
    }
    e.text(&h.puzzle);
    e.text(&h.model_id);
    e.u32(h.generators);
    e.text(&h.key);
    e.text(&h.contract);
    e.bytes(&h.digest);
    Ok(())
}

fn decode_trace_header(d: &mut Dec) -> Result<TraceHeader, Malformed> {
    let puzzle = d.text()?.to_string();
    let model_id = d.text()?.to_string();
    let generators = d.u32()?;
    let key = d.text()?.to_string();
    let contract = d.text()?.to_string();
    let digest = decode_digest(d)?;
    if [&puzzle, &model_id, &key, &contract].iter().any(|t| t.is_empty()) {
        return Err(Malformed::new("a trace key header with an empty field"));
    }
    Ok(TraceHeader { puzzle, model_id, menu: None, generators, key, contract, digest })
}

/// A menu word key's header and tokens ([`tag::MENU_WORD_KEY`]), refused under another key kind, under an identity
/// block that names no alphabet, or out of its canonical form.
fn decode_menu_word_key(d: &mut Dec) -> Result<MenuWordKey, Malformed> {
    let identity = decode_identity(d)?;
    if d.text()? != WORD_KEY {
        return Err(Malformed::new("a word key of another kind"));
    }
    let (grips, items) = (d.u32()?, d.u32()?);
    // A token takes at least its grip and 48 integers, each a sign byte and a length.
    let n = d.count(4 + 48 * 9)?;
    let mut tokens = Vec::with_capacity(n);
    for _ in 0..n {
        let grip = d.u32()?;
        tokens.push(Token { grip, product: decode_matrix(d)? });
    }
    let key = MenuWordKey::from_parts(Arc::new(identity), grips, items, tokens);
    if !key.canonical() {
        return Err(Malformed::new("a menu word key outside its canonical form"));
    }
    Ok(key)
}

fn decode_digest(d: &mut Dec) -> Result<[u8; 32], Malformed> {
    d.bytes()?.try_into().map_err(|_| Malformed::new("a relation digest of other than 32 bytes"))
}

/// A menu trace key's header (LB1.10, LG2.23; LP2 draft): the identity block's five fields in KN1.4's order, the grip
/// count, the item count, the key kind, the independence contract and the digest, refused when a text field is empty.
fn encode_menu_trace_header(h: &TraceHeader, e: &mut Enc) -> Result<(), Unencodable> {
    let (Some(identity), Some(menu)) = (h.menu_identity(), h.menu.as_ref()) else {
        return Err(Unencodable("a menu trace key under a Full header"));
    };
    if h.key.is_empty() || h.contract.is_empty() {
        return Err(Unencodable("a trace key header with an empty field"));
    }
    encode_identity(&identity, e).map_err(|_| Unencodable("a trace key header with an empty field"))?;
    e.u32(h.generators);
    e.u32(menu.items);
    e.text(&h.key);
    e.text(&h.contract);
    e.bytes(&h.digest);
    Ok(())
}

fn decode_menu_trace_header(d: &mut Dec) -> Result<TraceHeader, Malformed> {
    let identity = read_identity(d)?;
    let (generators, items) = (d.u32()?, d.u32()?);
    let key = d.text()?.to_string();
    let contract = d.text()?.to_string();
    let digest = decode_digest(d)?;
    if !identity.names_alphabet() || key.is_empty() || contract.is_empty() {
        return Err(Malformed::new("a trace key header with an empty field"));
    }
    let MenuIdentity { puzzle, model_id, menu_id, menu_order, contract_rev } = identity;
    let menu = Some(MenuHeader { menu_id, menu_order, contract_rev, items });
    Ok(TraceHeader { puzzle, model_id, menu, generators, key, contract, digest })
}

/// A pinned trace relation (LB1.10; LP2 draft): its independence contract, its identity block's puzzle and model ID,
/// its generator count, then its independence bits as `Relation::bits` gives them. A jumbling relation
/// (`jumble-locality/1`) has the identity block's other three fields after the model ID and the item count after the
/// grip count. The digest is not sent.
pub fn encode_relation(r: &Relation, e: &mut Enc) {
    let h = r.header();
    e.text(&h.contract);
    e.text(&h.puzzle);
    e.text(&h.model_id);
    if let Some(menu) = &h.menu {
        e.text(&menu.menu_id);
        e.text(&menu.menu_order);
        e.text(&menu.contract_rev);
    }
    e.u32(h.generators);
    if let Some(menu) = &h.menu {
        e.u32(menu.items);
    }
    e.bytes(r.bits());
}

/// Reads a pinned trace relation and refuses one that `Relation::from_parts` or `Relation::jumble_locality` refuses,
/// with its reason. The digest is recomputed from what was read.
pub fn decode_relation(d: &mut Dec) -> Result<Relation, Malformed> {
    let contract = d.text()?.to_string();
    let puzzle = d.text()?.to_string();
    let model_id = d.text()?.to_string();
    if contract == JUMBLE_LOCALITY {
        let mut field = || Ok::<_, Malformed>(d.text()?.to_string());
        let (menu_id, menu_order, contract_rev) = (field()?, field()?, field()?);
        let identity = MenuIdentity { puzzle, model_id, menu_id, menu_order, contract_rev };
        let (grips, items) = (d.u32()?, d.u32()?);
        let bits = d.bytes()?.to_vec();
        return Relation::jumble_locality(&identity, grips, items, bits).map_err(|why| Malformed::new(why.to_string()));
    }
    let generators = d.u32()?;
    let bits = d.bytes()?.to_vec();
    Relation::from_parts(&contract, &puzzle, &model_id, generators, bits).map_err(|why| Malformed::new(why.to_string()))
}

/// A pinned menu alphabet (LB1.8, LB1.9, LG2.18; LP2 draft): its identity block, its cap count, the item count and
/// the item of each item's inverse rotation, by item, then a flag: 0 without the menu's item matrices, 1 with them,
/// followed by their count and each item's matrix in item order ([`encode_matrix`]).
pub fn encode_alphabet(m: &MenuAlphabet, e: &mut Enc) -> Result<(), Unencodable> {
    encode_identity(m.identity(), e)?;
    e.u32(m.caps());
    e.count(m.items());
    for item in m.inverse_items() {
        e.u32(*item);
    }
    match m.item_matrices() {
        None => e.u8(0),
        Some(items) => {
            e.u8(1);
            e.count(items.len());
            for m in items {
                encode_matrix(m, e);
            }
        }
    }
    Ok(())
}

/// Reads a pinned menu alphabet and refuses one that `MenuAlphabet::new` or `MenuAlphabet::with_matrices` refuses,
/// with its reason: the first empty field of the identity block by name, too many caps or items, inverse items that
/// are no involution, not one matrix per item, an entry outside Q(√5)'s lowest terms, an inverse item's matrix that is
/// not the item's transpose, or matrices that do not give the block's `menu_id` or `menu_order`. A matrices flag other
/// than 0 or 1 is refused.
pub fn decode_alphabet(d: &mut Dec) -> Result<MenuAlphabet, Malformed> {
    let identity = read_identity(d)?;
    let caps = d.u32()?;
    let n = d.count(4)?;
    let inverse = (0..n).map(|_| d.u32()).collect::<Result<Vec<_>, _>>()?;
    let alphabet = MenuAlphabet::new(identity, caps, inverse).map_err(|why| Malformed::new(why.to_string()))?;
    match d.u8()? {
        0 => Ok(alphabet),
        1 => {
            // A matrix takes at least 48 integers, each a sign byte and a length.
            let n = d.count(48 * 9)?;
            let items = (0..n).map(|_| decode_matrix(d)).collect::<Result<Vec<_>, _>>()?;
            alphabet.with_matrices(items).map_err(|why| Malformed::new(why.to_string()))
        }
        _ => Err(Malformed::new("a matrices flag other than 0 or 1")),
    }
}

/// An item matrix or a token's product (LG2.18; LP2 draft): its sixteen entries rows first, each as the payload of
/// [`tag::Q5`], its `a`, `b` and `d`. The reader checks no entry's form; the alphabet or the key that holds the matrix
/// does.
fn encode_matrix(m: &ItemMatrix, e: &mut Enc) {
    for q in m.iter().flatten() {
        bigint(&q.a, e);
        bigint(&q.b, e);
        bigint(&q.d, e);
    }
}

/// Reads [`encode_matrix`]'s sixteen entries.
fn decode_matrix(d: &mut Dec) -> Result<ItemMatrix, Malformed> {
    let mut entries = Vec::with_capacity(16);
    for _ in 0..16 {
        entries.push(Quad { a: decode_bigint(d)?, b: decode_bigint(d)?, d: decode_bigint(d)? });
    }
    let mut entries = entries.into_iter();
    Ok(std::array::from_fn(|_| std::array::from_fn(|_| entries.next().expect("sixteen entries"))))
}

fn bigint(n: &BigInt, e: &mut Enc) {
    let (sign, magnitude) = n.to_bytes_le();
    match sign {
        Sign::NoSign => {
            e.u8(0);
            e.count(0);
        }
        Sign::Plus => {
            e.u8(1);
            e.bytes(&magnitude);
        }
        Sign::Minus => {
            e.u8(2);
            e.bytes(&magnitude);
        }
    }
}

fn decode_bigint(d: &mut Dec) -> Result<BigInt, Malformed> {
    let sign = d.u8()?;
    let magnitude = d.bytes()?;
    let minimal = !magnitude.is_empty() && magnitude.last() != Some(&0);
    match sign {
        0 if magnitude.is_empty() => Ok(BigInt::zero()),
        1 if minimal => Ok(BigInt::from_bytes_le(Sign::Plus, magnitude)),
        2 if minimal => Ok(BigInt::from_bytes_le(Sign::Minus, magnitude)),
        _ => Err(Malformed::new("an integer that is not in its minimal form")),
    }
}

fn canonical_ratio(r: &Ratio) -> bool {
    r.den.is_positive() && r.num.gcd(&r.den).is_one()
}

fn canonical_quad(q: &Quad) -> bool {
    q.d.is_positive() && q.a.gcd(&q.b).gcd(&q.d).is_one()
}

fn canonical_approx(x: &Approx) -> bool {
    if !approx::PRECISIONS.contains(&x.precision) {
        return false;
    }
    if x.mantissa == 0 {
        return !x.negative && x.exponent == 0;
    }
    let p = x.precision;
    x.mantissa >= 1u128 << (p - 1) && (p >= 128 || x.mantissa < 1u128 << p)
}

fn canonical_word(letters: &[i32]) -> bool {
    letters.iter().all(|l| (1..=1200).contains(&l.unsigned_abs()))
}

/// LG2.18: no letter is followed by its inverse. It never overflows, so a letter out of range reaches its own refusal.
fn freely_reduced(letters: &[i32]) -> bool {
    letters.windows(2).all(|w| w[0].checked_neg() != Some(w[1]))
}

/// Strictly increasing in the canonical key order without its secondary key (DET.9; `c600_run::value::value_order`),
/// so no two are one key: an `Int` and a `Rat` of one number are one.
fn increasing<'a>(mut keys: impl Iterator<Item = &'a Value>) -> bool {
    let Some(mut last) = keys.next() else { return true };
    keys.all(|k| {
        let less = value_order(last, k) == Ordering::Less;
        last = k;
        less
    })
}

/// A map entry's key and value: the record `{ key, value }` with exactly these fields in this order
/// (`c600_run::value::entry`).
fn entry_parts(item: &Value) -> Option<(&Value, &Value)> {
    let Value::Record(r) = item else { return None };
    match r.fields() {
        [(k, key), (v, value)] if k.as_str() == "key" && v.as_str() == "value" => Some((key, value)),
        _ => None,
    }
}

/// A word's or a word key's letters: the tag, the count, then each letter.
fn encode_letters(t: u8, letters: &[i32], e: &mut Enc) -> Result<(), Unencodable> {
    if !canonical_word(letters) {
        return Err(Unencodable("a letter outside 1 to 1200"));
    }
    e.u8(t);
    e.count(letters.len());
    for l in letters {
        e.i32(*l);
    }
    Ok(())
}

fn decode_letters(d: &mut Dec) -> Result<Vec<i32>, Malformed> {
    let n = d.count(4)?;
    let mut letters = Vec::with_capacity(n);
    for _ in 0..n {
        letters.push(d.i32()?);
    }
    if !canonical_word(&letters) {
        return Err(Malformed::new("a letter outside 1 to 1200"));
    }
    Ok(letters)
}

/// The most bits that the derived part lengths of all the compressed words in one payload may take together: as many
/// as one frame holds (C3 draft 15). `Compressed::from_parts` derives a length for every part, and a chain of n
/// doublings derives about n²/2 bits in all, so both sides count them in the payload's encoder or decoder before a
/// word is rebuilt: compressed words above the bound are refused when encoded and when decoded. The bound counts bits:
/// a length is stored in whole 64-bit limbs, which adds at most one limb per part, and a part takes at least 9 bytes
/// of the payload. The interpreter builds words without this bound.
pub const COMPRESSED_LENGTH_BITS: u64 = 8 * MAX_FRAME as u64;

/// The bits of one compressed word's derived lengths, part by part: a leaf's length has its letter count's bits, and
/// a concatenation's at most one more than its larger operand's.
#[derive(Default)]
struct LengthBits {
    bits: Vec<u32>,
}

impl LengthBits {
    /// Adds the next part's bits to `total`, the payload's running count. It refuses a concatenation of a part not
    /// before it, as `from_parts` does, and a count above [`COMPRESSED_LENGTH_BITS`]; no part's bits can then
    /// overflow.
    fn push(&mut self, part: &Part, total: &mut u64) -> Result<(), &'static str> {
        let bits = match part {
            Part::Leaf(l) => usize::BITS - l.len().leading_zeros(),
            Part::Concat(i, j) => match (self.bits.get(*i), self.bits.get(*j)) {
                (Some(a), Some(b)) => 1 + *a.max(b),
                _ => return Err("a concatenation of a part not before it"),
            },
        };
        *total += u64::from(bits);
        if *total > COMPRESSED_LENGTH_BITS {
            return Err("compressed words above the transfer bound");
        }
        self.bits.push(bits);
        Ok(())
    }
}

enum Item<'a> {
    Value(&'a Value),
    Name(&'a str),
}

/// Appends a value's transfer encoding. A value outside its canonical form is refused.
pub fn encode_value(value: &Value, e: &mut Enc) -> Result<(), Unencodable> {
    let mut stack = vec![Item::Value(value)];
    while let Some(item) = stack.pop() {
        let value = match item {
            Item::Name(name) => {
                e.text(name);
                continue;
            }
            Item::Value(v) => v,
        };
        match value {
            Value::Unit => e.u8(tag::UNIT),
            Value::Bool(b) => {
                e.u8(tag::BOOL);
                e.bool(*b);
            }
            Value::Int(n) => {
                e.u8(tag::INT);
                bigint(n, e);
            }
            Value::Rat(r) => {
                if !canonical_ratio(r) {
                    return Err(Unencodable("a rational outside its lowest terms"));
                }
                e.u8(tag::RAT);
                bigint(&r.num, e);
                bigint(&r.den, e);
            }
            Value::Q5(q) => {
                if !canonical_quad(q) {
                    return Err(Unencodable("an element of Q(√5) outside its lowest terms"));
                }
                e.u8(tag::Q5);
                bigint(&q.a, e);
                bigint(&q.b, e);
                bigint(&q.d, e);
            }
            Value::Approx(x) => {
                if !canonical_approx(x) {
                    return Err(Unencodable("an approximation that is not normalised"));
                }
                e.u8(tag::APPROX);
                e.u32(x.precision);
                e.bool(x.negative);
                e.u128(x.mantissa);
                e.i128(x.exponent);
            }
            Value::ApproxBool(b) => {
                e.u8(tag::APPROX_BOOL);
                e.bool(*b);
            }
            Value::Text(t) => {
                e.u8(tag::TEXT);
                e.text(t.as_str());
            }
            Value::Target(t) => {
                e.u8(tag::TARGET);
                e.text(t.as_str());
            }
            Value::Tag(t) => {
                e.u8(tag::TAG);
                e.text(t.as_str());
            }
            Value::Colour(t) => {
                e.u8(tag::COLOUR);
                e.text(t.as_str());
            }
            Value::Word(w) => encode_letters(tag::WORD, w, e)?,
            Value::MenuWord(w) => {
                e.u8(tag::MENU_WORD);
                encode_identity(w.identity(), e)?;
                encode_menu_letters(w.letters(), e)?;
            }
            Value::EndTest(test, outcome) => {
                e.u8(tag::END_TEST);
                e.u8(end_test_code(*test));
                e.u8(verdict_code(*outcome));
            }
            Value::TraceEqualityKey(k) if k.is_menu() => {
                e.u8(tag::MENU_TRACE_KEY);
                encode_menu_trace_header(k.header(), e)?;
                encode_menu_letters(k.menu_letters(), e)?;
            }
            Value::MenuWordKey(k) => {
                if !k.canonical() {
                    return Err(Unencodable("a menu word key outside its canonical form"));
                }
                e.u8(tag::MENU_WORD_KEY);
                encode_identity(k.identity(), e)?;
                e.text(WORD_KEY);
                e.u32(k.grips());
                e.u32(k.items());
                e.count(k.tokens().len());
                for t in k.tokens() {
                    e.u32(t.grip);
                    encode_matrix(&t.product, e);
                }
            }
            Value::TraceEqualityKey(k) => {
                if !canonical_word(k.letters()) {
                    return Err(Unencodable("a letter outside 1 to 1200"));
                }
                e.u8(tag::TRACE_KEY);
                encode_trace_header(k.header(), e)?;
                e.count(k.letters().len());
                for l in k.letters() {
                    e.i32(*l);
                }
            }
            Value::Field(field, residue) => {
                if *residue >= field.modulus() {
                    return Err(Unencodable("a residue outside its field"));
                }
                e.u8(tag::FIELD);
                e.u8(field_byte(*field));
                e.u8(*residue);
            }
            Value::Bytes(b) => {
                e.u8(tag::BYTES);
                e.bytes(b);
            }
            Value::Record(r) | Value::Receipt(r) => {
                if r.fields().iter().map(|(n, _)| n.as_str()).collect::<BTreeSet<_>>().len() != r.fields().len() {
                    return Err(Unencodable("a record with a repeated field"));
                }
                e.u8(if matches!(value, Value::Receipt(_)) { tag::RECEIPT } else { tag::RECORD });
                e.count(r.fields().len());
                for (name, v) in r.fields().iter().rev() {
                    stack.push(Item::Value(v));
                    stack.push(Item::Name(name.as_str()));
                }
            }
            Value::Closure(c) => {
                e.u8(tag::CLOSURE);
                e.u64(c.code as u64);
                e.count(c.captures().len());
                for v in c.captures().iter().rev() {
                    stack.push(Item::Value(v));
                }
            }
            Value::Fn(f) => {
                e.u8(tag::FN);
                e.u64(*f as u64);
            }
            // A generator is one owner's cursor on a sealed stream (DET.61; LP2 draft). It stays in its run: a copy
            // in another process would be a second cursor, which only DET.61's alias makes.
            Value::Generator(_) => return Err(Unencodable("a generator, which stays in its run")),
            // LP2's draft word stream (DET.63) stays in its run too: its digest crosses as text.
            Value::WordStream(_) => return Err(Unencodable("a word stream, which stays in its run")),
            Value::List(l) => {
                e.u8(tag::LIST);
                e.count(l.items().len());
                for v in l.items().iter().rev() {
                    stack.push(Item::Value(v));
                }
            }
            Value::WordEqualityKey(w) => {
                if !freely_reduced(w) {
                    return Err(Unencodable("a word key that is not freely reduced"));
                }
                encode_letters(tag::WORD_KEY, w, e)?;
            }
            Value::Set(s) => {
                if !increasing(s.items().iter()) {
                    return Err(Unencodable("a set outside the canonical key order"));
                }
                e.u8(tag::SET);
                e.count(s.items().len());
                for v in s.items().iter().rev() {
                    stack.push(Item::Value(v));
                }
            }
            Value::Map(m) => {
                let Some(entries) = m.items().iter().map(entry_parts).collect::<Option<Vec<_>>>() else {
                    return Err(Unencodable("a map entry that is not a { key, value } record"));
                };
                if !increasing(entries.iter().map(|(k, _)| *k)) {
                    return Err(Unencodable("a map outside the canonical key order"));
                }
                e.u8(tag::MAP);
                e.count(entries.len());
                for (k, v) in entries.into_iter().rev() {
                    stack.push(Item::Value(v));
                    stack.push(Item::Value(k));
                }
            }
            // A built word's parts are as `from_parts` takes them, except for letters, which `leaf` does not check, and
            // the bound, which the interpreter does not apply.
            Value::Compressed(c) => {
                let parts = c.part_list();
                e.u8(tag::COMPRESSED);
                e.count(parts.len());
                let mut bound = LengthBits::default();
                for part in parts {
                    bound.push(part, &mut e.derived_bits).map_err(Unencodable)?;
                    match part {
                        Part::Leaf(l) => encode_letters(part_tag::LEAF, l, e)?,
                        Part::Concat(i, j) => {
                            e.u8(part_tag::CONCAT);
                            e.u64(*i as u64);
                            e.u64(*j as u64);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// A value's transfer encoding on its own.
pub fn value_bytes(value: &Value) -> Result<Vec<u8>, Unencodable> {
    let mut e = Enc::new();
    encode_value(value, &mut e)?;
    Ok(e.bytes)
}

/// What a sequence frame builds.
#[derive(Clone, Copy)]
enum Seq {
    List,
    Set,
    /// Keys and values alternate.
    Map,
}

enum Frame {
    Record { receipt: bool, left: usize, fields: Vec<(Arc<Text>, Value)>, name: Option<Arc<Text>> },
    Closure { code: usize, left: usize, captures: Vec<Value> },
    Seq { kind: Seq, left: usize, items: Vec<Value> },
}

/// A decoded list, set or map. A set's elements and a map's keys must be strictly increasing in the canonical key
/// order.
fn sequence(kind: Seq, items: Vec<Value>) -> Result<Value, Malformed> {
    Ok(match kind {
        Seq::List => Value::List(Arc::new(List::new(items))),
        Seq::Set => {
            if !increasing(items.iter()) {
                return Err(Malformed::new("a set outside the canonical key order"));
            }
            Value::Set(Arc::new(List::new(items)))
        }
        Seq::Map => {
            if !increasing(items.iter().step_by(2)) {
                return Err(Malformed::new("a map outside the canonical key order"));
            }
            let mut items = items.into_iter();
            let mut entries = Vec::with_capacity(items.len() / 2);
            while let (Some(k), Some(v)) = (items.next(), items.next()) {
                entries.push(entry(k, v));
            }
            Value::Map(Arc::new(List::new(entries)))
        }
    })
}

fn text_value(d: &mut Dec) -> Result<Arc<Text>, Malformed> {
    Ok(Text::new(d.text()?))
}

/// Reads one value. It refuses a value outside its canonical form.
pub fn decode_value(d: &mut Dec) -> Result<Value, Malformed> {
    let mut stack: Vec<Frame> = Vec::new();
    loop {
        if let Some(Frame::Record { name: name @ None, .. }) = stack.last_mut() {
            *name = Some(text_value(d)?);
        }
        let tag = d.u8()?;
        let mut value = match tag {
            tag::UNIT => Value::Unit,
            tag::BOOL => Value::Bool(d.bool()?),
            tag::INT => Value::Int(Arc::new(decode_bigint(d)?)),
            tag::RAT => {
                let r = Ratio { num: decode_bigint(d)?, den: decode_bigint(d)? };
                if !canonical_ratio(&r) {
                    return Err(Malformed::new("a rational outside its lowest terms"));
                }
                Value::Rat(Arc::new(r))
            }
            tag::Q5 => {
                let q = Quad { a: decode_bigint(d)?, b: decode_bigint(d)?, d: decode_bigint(d)? };
                if !canonical_quad(&q) {
                    return Err(Malformed::new("an element of Q(√5) outside its lowest terms"));
                }
                Value::Q5(Arc::new(q))
            }
            tag::APPROX => {
                let x = Approx { precision: d.u32()?, negative: d.bool()?, mantissa: d.u128()?, exponent: d.i128()? };
                if !canonical_approx(&x) {
                    return Err(Malformed::new("an approximation that is not normalised"));
                }
                Value::Approx(x)
            }
            tag::APPROX_BOOL => Value::ApproxBool(d.bool()?),
            tag::TEXT => Value::Text(text_value(d)?),
            tag::TARGET => Value::Target(text_value(d)?),
            tag::TAG => Value::Tag(text_value(d)?),
            tag::COLOUR => Value::Colour(text_value(d)?),
            tag::WORD => Value::Word(decode_letters(d)?.into()),
            tag::MENU_WORD => {
                let identity = Arc::new(decode_identity(d)?);
                let letters = decode_menu_letters(d)?;
                Value::menu_word(&identity, &letters).ok_or_else(|| Malformed::new("a menu index of 600 or more"))?
            }
            tag::END_TEST => Value::EndTest(end_test_from(d.u8()?)?, verdict_from(d.u8()?)?),
            tag::TRACE_KEY => {
                let header = Arc::new(decode_trace_header(d)?);
                Value::TraceEqualityKey(Arc::new(TraceKey::from_parts(header, &decode_letters(d)?)))
            }
            tag::MENU_TRACE_KEY => {
                let header = Arc::new(decode_menu_trace_header(d)?);
                Value::TraceEqualityKey(Arc::new(TraceKey::from_menu_parts(header, &decode_menu_letters(d)?)))
            }
            tag::MENU_WORD_KEY => Value::MenuWordKey(Arc::new(decode_menu_word_key(d)?)),
            tag::FIELD => {
                let field = field_from(d.u8()?)?;
                let residue = d.u8()?;
                if residue >= field.modulus() {
                    return Err(Malformed::new("a residue outside its field"));
                }
                Value::Field(field, residue)
            }
            tag::BYTES => Value::Bytes(d.bytes()?.into()),
            tag::WORD_KEY => {
                let letters = decode_letters(d)?;
                if !freely_reduced(&letters) {
                    return Err(Malformed::new("a word key that is not freely reduced"));
                }
                Value::WordEqualityKey(letters.into())
            }
            tag::RECORD | tag::RECEIPT => {
                let receipt = tag == tag::RECEIPT;
                // A field takes at least a name length and a value tag.
                let n = d.count(9)?;
                if n == 0 {
                    let record = Arc::new(Record::new(Vec::new()));
                    if receipt {
                        Value::Receipt(record)
                    } else {
                        Value::Record(record)
                    }
                } else {
                    stack.push(Frame::Record { receipt, left: n, fields: Vec::with_capacity(n.min(64)), name: None });
                    continue;
                }
            }
            tag::CLOSURE => {
                let code = d.usize()?;
                let n = d.count(1)?;
                if n == 0 {
                    Value::Closure(Arc::new(Closure::new(code, Vec::new())))
                } else {
                    stack.push(Frame::Closure { code, left: n, captures: Vec::with_capacity(n.min(64)) });
                    continue;
                }
            }
            tag::FN => Value::Fn(d.usize()?),
            tag::COMPRESSED => {
                // A part takes at least its tag and a letter count or a part number.
                let n = d.count(9)?;
                let mut parts = Vec::with_capacity(n.min(64));
                let mut bound = LengthBits::default();
                for _ in 0..n {
                    let part = match d.u8()? {
                        part_tag::LEAF => Part::Leaf(decode_letters(d)?.into()),
                        part_tag::CONCAT => Part::Concat(d.usize()?, d.usize()?),
                        _ => return Err(Malformed::new("an unknown part tag")),
                    };
                    bound.push(&part, &mut d.derived_bits).map_err(Malformed::new)?;
                    parts.push(part);
                }
                Value::Compressed(Arc::new(Compressed::from_parts(parts).map_err(Malformed::new)?))
            }
            tag::LIST | tag::SET | tag::MAP => {
                // An element takes at least its tag; a map entry, its key's and its value's.
                let (kind, per) = match tag {
                    tag::LIST => (Seq::List, 1),
                    tag::SET => (Seq::Set, 1),
                    _ => (Seq::Map, 2),
                };
                let left = d.count(per)? * per;
                if left == 0 {
                    sequence(kind, Vec::new())?
                } else {
                    stack.push(Frame::Seq { kind, left, items: Vec::with_capacity(left.min(64)) });
                    continue;
                }
            }
            _ => return Err(Malformed::new("an unknown value tag")),
        };
        loop {
            let Some(top) = stack.last_mut() else { return Ok(value) };
            let left = match top {
                Frame::Record { left, fields, name, .. } => {
                    let Some(name) = name.take() else { return Err(Malformed::new("a field without a name")) };
                    fields.push((name, value));
                    *left -= 1;
                    *left
                }
                Frame::Closure { left, captures, .. } => {
                    captures.push(value);
                    *left -= 1;
                    *left
                }
                Frame::Seq { left, items, .. } => {
                    items.push(value);
                    *left -= 1;
                    *left
                }
            };
            if left > 0 {
                break;
            }
            value = match stack.pop() {
                Some(Frame::Record { fields, .. })
                    if fields.iter().map(|(n, _)| n.as_str()).collect::<BTreeSet<_>>().len() != fields.len() =>
                {
                    return Err(Malformed::new("a record with a repeated field"));
                }
                Some(Frame::Record { receipt: false, fields, .. }) => Value::Record(Arc::new(Record::new(fields))),
                Some(Frame::Record { receipt: true, fields, .. }) => Value::Receipt(Arc::new(Record::new(fields))),
                Some(Frame::Closure { code, captures, .. }) => Value::Closure(Arc::new(Closure::new(code, captures))),
                Some(Frame::Seq { kind, items, .. }) => sequence(kind, items)?,
                None => return Err(Malformed::new("a value frame went missing")),
            };
        }
    }
}

/// Reads a whole payload as one value.
pub fn value_from_bytes(bytes: &[u8]) -> Result<Value, Malformed> {
    let mut d = Dec::new(bytes);
    let value = decode_value(&mut d)?;
    d.end()?;
    Ok(value)
}

/// Whether two values are the same variant with the same payload, which equal transfer bytes establish. Unlike
/// `c600_run::value::equal`, an `Int` and a `Rat` of the same number differ here.
pub fn same_value(a: &Value, b: &Value) -> bool {
    matches!((value_bytes(a), value_bytes(b)), (Ok(x), Ok(y)) if x == y)
}

// ---------------------------------------------------------------------------------------------------------------
// IR types

fn effect_code(e: Effect) -> u8 {
    match e {
        Effect::ReadSnapshot => 0,
        Effect::ChangeView => 1,
        Effect::ChangeWorkspace => 2,
        Effect::PreparePreview => 3,
        Effect::GrantedFile => 4,
        Effect::SeededRandomness => 5,
        Effect::Budgets => 6,
    }
}

fn effect_from(code: u8) -> Result<Effect, Malformed> {
    Ok(match code {
        0 => Effect::ReadSnapshot,
        1 => Effect::ChangeView,
        2 => Effect::ChangeWorkspace,
        3 => Effect::PreparePreview,
        4 => Effect::GrantedFile,
        5 => Effect::SeededRandomness,
        6 => Effect::Budgets,
        _ => return Err(Malformed::new("an unknown effect")),
    })
}

pub fn capability_code(c: Capability) -> u8 {
    match c {
        Capability::File => 0,
        Capability::Clock => 1,
        Capability::Randomness => 2,
        Capability::Network => 3,
        Capability::Process => 4,
        Capability::SessionFile => 5,
    }
}

pub fn capability_from(code: u8) -> Result<Capability, Malformed> {
    Ok(match code {
        0 => Capability::File,
        1 => Capability::Clock,
        2 => Capability::Randomness,
        3 => Capability::Network,
        4 => Capability::Process,
        5 => Capability::SessionFile,
        _ => return Err(Malformed::new("an unknown capability")),
    })
}

/// One row of a built-in tool's class (`c600_check::Row`, LP2 draft): a tag in the variants' declared order, then the
/// variant's text. The tags follow the rows' order, so a set's rows in increasing order are its bytes' order too.
pub fn encode_row(row: &ClassRow, e: &mut Enc) {
    let (tag, text) = match row {
        ClassRow::Read(query) => (0, Some(query)),
        ClassRow::Outcome => (1, None),
        ClassRow::Declassified(id) => (2, Some(id)),
        ClassRow::View => (3, None),
        ClassRow::Workspace(record) => (4, Some(record)),
        ClassRow::Preview(target) => (5, Some(target)),
        ClassRow::Unknown => (6, None),
        ClassRow::RepairList(query) => (7, Some(query)),
        ClassRow::SetupSearch(query) => (8, Some(query)),
    };
    e.u8(tag);
    if let Some(text) = text {
        e.text(text);
    }
}

pub fn decode_row(d: &mut Dec) -> Result<ClassRow, Malformed> {
    Ok(match d.u8()? {
        0 => ClassRow::Read(d.text()?.to_string()),
        1 => ClassRow::Outcome,
        2 => ClassRow::Declassified(d.text()?.to_string()),
        3 => ClassRow::View,
        4 => ClassRow::Workspace(d.text()?.to_string()),
        5 => ClassRow::Preview(d.text()?.to_string()),
        6 => ClassRow::Unknown,
        7 => ClassRow::RepairList(d.text()?.to_string()),
        8 => ClassRow::SetupSearch(d.text()?.to_string()),
        _ => return Err(Malformed::new("an unknown class row")),
    })
}

fn role_code(r: ExactRole) -> u8 {
    match r {
        ExactRole::Budget => 0,
        ExactRole::Goal => 1,
        ExactRole::Evidence => 2,
    }
}

fn role_from(code: u8) -> Result<ExactRole, Malformed> {
    Ok(match code {
        0 => ExactRole::Budget,
        1 => ExactRole::Goal,
        2 => ExactRole::Evidence,
        _ => return Err(Malformed::new("an unknown exact role")),
    })
}

/// An end test's index in [`EndTest::ALL`] (LP2 draft).
fn end_test_code(t: EndTest) -> u8 {
    match t {
        EndTest::RetainedCheckpoint => 0,
        EndTest::LabelCheckpoint => 1,
        EndTest::StickerTest => 2,
    }
}

fn end_test_from(code: u8) -> Result<EndTest, Malformed> {
    Ok(match code {
        0 => EndTest::RetainedCheckpoint,
        1 => EndTest::LabelCheckpoint,
        2 => EndTest::StickerTest,
        _ => return Err(Malformed::new("an unknown end test")),
    })
}

/// An end test's outcome's index in [`EndTestOutcome::ALL`] (LP2 draft).
fn verdict_code(o: EndTestOutcome) -> u8 {
    match o {
        EndTestOutcome::Holds => 0,
        EndTestOutcome::Fails => 1,
        EndTestOutcome::Unestablished => 2,
    }
}

fn verdict_from(code: u8) -> Result<EndTestOutcome, Malformed> {
    Ok(match code {
        0 => EndTestOutcome::Holds,
        1 => EndTestOutcome::Fails,
        2 => EndTestOutcome::Unestablished,
        _ => return Err(Malformed::new("an unknown end-test outcome")),
    })
}

pub fn encode_qty(t: &QTy, e: &mut Enc) -> Result<(), Unencodable> {
    qty(t, e, 0)
}

fn qty(t: &QTy, e: &mut Enc, depth: usize) -> Result<(), Unencodable> {
    ty(&t.ty, e, depth)?;
    e.bool(t.display);
    Ok(())
}

pub fn encode_ty(t: &Ty, e: &mut Enc) -> Result<(), Unencodable> {
    ty(t, e, 0)
}

fn ty(t: &Ty, e: &mut Enc, depth: usize) -> Result<(), Unencodable> {
    if depth > TYPE_DEPTH {
        return Err(Unencodable("a type deeper than the transfer bound"));
    }
    match t {
        Ty::Unit => e.u8(0),
        Ty::Bool => e.u8(1),
        Ty::Nat => e.u8(2),
        Ty::Int => e.u8(3),
        Ty::Rat => e.u8(4),
        Ty::Q5 => e.u8(5),
        Ty::Text => e.u8(6),
        Ty::Word => e.u8(7),
        Ty::Target => e.u8(8),
        Ty::Budget => e.u8(9),
        Ty::Status => e.u8(10),
        Ty::Colour => e.u8(11),
        Ty::Approx(p) => {
            e.u8(12);
            e.u32(*p);
        }
        Ty::ApproxBool => e.u8(13),
        Ty::Record(fields) => {
            e.u8(14);
            e.count(fields.len());
            for (name, field) in fields {
                e.text(name);
                qty(field, e, depth + 1)?;
            }
        }
        Ty::Fn(f) => {
            e.u8(15);
            e.count(f.params.len());
            for p in &f.params {
                qty(p, e, depth + 1)?;
            }
            qty(&f.result, e, depth + 1)?;
            e.count(f.effects.effects.len());
            for x in &f.effects.effects {
                e.u8(effect_code(*x));
            }
            e.count(f.effects.capabilities.len());
            for c in &f.effects.capabilities {
                e.u8(capability_code(*c));
            }
            e.count(f.effects.vars.len());
            for v in &f.effects.vars {
                e.text(v);
            }
        }
        Ty::Receipt => e.u8(16),
        Ty::Var(name) => {
            e.u8(17);
            e.text(name);
        }
        // LP2's draft collection types and the compressed word's.
        Ty::List(element) => {
            e.u8(18);
            qty(element, e, depth + 1)?;
        }
        Ty::WordEqualityKey => e.u8(19),
        Ty::Set(element) => {
            e.u8(20);
            qty(element, e, depth + 1)?;
        }
        Ty::Map(key, value) => {
            e.u8(21);
            qty(key, e, depth + 1)?;
            qty(value, e, depth + 1)?;
        }
        Ty::CompressedWord => e.u8(22),
        Ty::MenuWord => e.u8(23),
        // LP2's draft end-test verdict (LB3.2): the tag, then the test's index.
        Ty::EndTest(test) => {
            e.u8(24);
            e.u8(end_test_code(*test));
        }
        // LP2's draft trace key (LB1.10).
        Ty::TraceEqualityKey => e.u8(25),
        // LP2's draft F2 or F5 element (KN3.2): the tag, then the field's byte.
        Ty::Field(field) => {
            e.u8(26);
            e.u8(field_byte(*field));
        }
        // LP2's draft generator (DET.61).
        Ty::Generator => e.u8(27),
        // LP2's draft captured bytes (DET.62).
        Ty::Bytes => e.u8(28),
        // LP2's draft word stream (DET.63).
        Ty::WordStream => e.u8(29),
    }
    Ok(())
}

pub fn decode_qty(d: &mut Dec) -> Result<QTy, Malformed> {
    decode_qty_at(d, 0)
}

fn decode_qty_at(d: &mut Dec, depth: usize) -> Result<QTy, Malformed> {
    let ty = decode_ty_at(d, depth)?;
    Ok(QTy { ty, display: d.bool()? })
}

pub fn decode_ty(d: &mut Dec) -> Result<Ty, Malformed> {
    decode_ty_at(d, 0)
}

/// Reads an ordered set and refuses one that is not strictly increasing, so it has one encoding.
fn ordered<T: Ord>(items: Vec<T>) -> Result<BTreeSet<T>, Malformed> {
    if items.windows(2).any(|w| w[0] >= w[1]) {
        return Err(Malformed::new("a set out of order"));
    }
    Ok(items.into_iter().collect())
}

fn decode_ty_at(d: &mut Dec, depth: usize) -> Result<Ty, Malformed> {
    if depth > TYPE_DEPTH {
        return Err(Malformed::new("a type deeper than the transfer bound"));
    }
    Ok(match d.u8()? {
        0 => Ty::Unit,
        1 => Ty::Bool,
        2 => Ty::Nat,
        3 => Ty::Int,
        4 => Ty::Rat,
        5 => Ty::Q5,
        6 => Ty::Text,
        7 => Ty::Word,
        8 => Ty::Target,
        9 => Ty::Budget,
        10 => Ty::Status,
        11 => Ty::Colour,
        12 => Ty::Approx(d.u32()?),
        13 => Ty::ApproxBool,
        14 => {
            let n = d.count(10)?;
            let mut fields = Vec::with_capacity(n);
            for _ in 0..n {
                let name = d.text()?.to_string();
                fields.push((name, decode_qty_at(d, depth + 1)?));
            }
            Ty::Record(fields)
        }
        15 => {
            let n = d.count(2)?;
            let mut params = Vec::with_capacity(n);
            for _ in 0..n {
                params.push(decode_qty_at(d, depth + 1)?);
            }
            let result = decode_qty_at(d, depth + 1)?;
            let n = d.count(1)?;
            let effects = (0..n).map(|_| effect_from(d.u8()?)).collect::<Result<Vec<_>, _>>()?;
            let n = d.count(1)?;
            let capabilities = (0..n).map(|_| capability_from(d.u8()?)).collect::<Result<Vec<_>, _>>()?;
            let n = d.count(8)?;
            let vars = (0..n).map(|_| Ok(d.text()?.to_string())).collect::<Result<Vec<_>, Malformed>>()?;
            let effects =
                EffectBound { effects: ordered(effects)?, capabilities: ordered(capabilities)?, vars: ordered(vars)? };
            Ty::Fn(Box::new(FnTy { params, result, effects }))
        }
        16 => Ty::Receipt,
        17 => Ty::Var(d.text()?.to_string()),
        18 => Ty::List(Box::new(decode_qty_at(d, depth + 1)?)),
        19 => Ty::WordEqualityKey,
        20 => Ty::Set(Box::new(decode_qty_at(d, depth + 1)?)),
        21 => {
            let key = decode_qty_at(d, depth + 1)?;
            Ty::Map(Box::new(key), Box::new(decode_qty_at(d, depth + 1)?))
        }
        22 => Ty::CompressedWord,
        23 => Ty::MenuWord,
        24 => Ty::EndTest(end_test_from(d.u8()?)?),
        25 => Ty::TraceEqualityKey,
        26 => Ty::Field(field_from(d.u8()?)?),
        27 => Ty::Generator,
        28 => Ty::Bytes,
        29 => Ty::WordStream,
        _ => return Err(Malformed::new("an unknown type tag")),
    })
}

// ---------------------------------------------------------------------------------------------------------------
// Host requests and sinks

/// A host request as the parent receives it. [`OwnedRequest::as_request`] lends it to the caller's host in the form
/// the interpreter built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnedRequest {
    Snapshot {
        query: String,
        ty: QTy,
    },
    Declassified {
        id: String,
        ty: Ty,
    },
    FileRead {
        path: String,
        ty: Ty,
    },
    Clock,
    Propose {
        word: Vec<i32>,
        target: String,
        request: Vec<u8>,
    },
    /// A read whose query takes an argument (LP2 draft): the argument's canonical bytes, as the interpreter encoded
    /// them. Neither side parses them; a host that answers the read reads them with `c600_run::provenance::parse`.
    SnapshotWith {
        query: String,
        arg: Vec<u8>,
        ty: QTy,
    },
    /// A menu-word preview (LB1.8; LP2 draft): the letters, the identity block of their alphabet, the target and the
    /// canonical request bytes.
    ProposeMenu {
        letters: Vec<[u32; 2]>,
        menu: MenuIdentity,
        target: String,
        request: Vec<u8>,
    },
}

impl OwnedRequest {
    pub fn as_request(&self) -> Request<'_> {
        match self {
            OwnedRequest::Snapshot { query, ty } => Request::Snapshot { query, ty },
            OwnedRequest::SnapshotWith { query, arg, ty } => Request::SnapshotWith { query, arg, ty },
            OwnedRequest::Declassified { id, ty } => Request::Declassified { id, ty },
            OwnedRequest::FileRead { path, ty } => Request::FileRead { path, ty },
            OwnedRequest::Clock => Request::Clock,
            OwnedRequest::Propose { word, target, request } => Request::Propose { word, target, request },
            OwnedRequest::ProposeMenu { letters, menu, target, request } => {
                Request::ProposeMenu { letters, menu, target, request }
            }
        }
    }
}

/// A sink as the parent receives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnedSink {
    View,
    Workspace { record: String, ty: QTy },
    File { path: String },
    Exact { role: ExactRole },
}

impl OwnedSink {
    pub fn as_sink(&self) -> Sink<'_> {
        match self {
            OwnedSink::View => Sink::View,
            OwnedSink::Workspace { record, ty } => Sink::Workspace { record, ty },
            OwnedSink::File { path } => Sink::File { path },
            OwnedSink::Exact { role } => Sink::Exact { role: *role },
        }
    }
}

pub fn encode_request(r: &Request, e: &mut Enc) -> Result<(), Unencodable> {
    match r {
        Request::Snapshot { query, ty } => {
            e.field("request.snapshot");
            e.u8(0);
            e.field("request.snapshot.query");
            e.text(query);
            e.field("request.snapshot.type");
            encode_qty(ty, e)?;
        }
        Request::Declassified { id, ty } => {
            e.field("request.declassified");
            e.u8(1);
            e.field("request.declassified.id");
            e.text(id);
            e.field("request.declassified.type");
            encode_ty(ty, e)?;
        }
        Request::FileRead { path, ty } => {
            e.field("request.file_read");
            e.u8(2);
            e.field("request.file_read.path");
            e.text(path);
            e.field("request.file_read.type");
            encode_ty(ty, e)?;
        }
        Request::Clock => {
            e.field("request.clock");
            e.u8(3);
        }
        Request::Propose { word, target, request } => {
            if !canonical_word(word) {
                return Err(Unencodable("a letter outside 1 to 1200"));
            }
            e.field("request.propose");
            e.u8(4);
            e.field("request.propose.word");
            e.count(word.len());
            for l in word.iter() {
                e.i32(*l);
            }
            e.field("request.propose.target");
            e.text(target);
            e.field("request.propose.request");
            e.bytes(request);
        }
        Request::SnapshotWith { query, arg, ty } => {
            e.field("request.snapshot_with");
            e.u8(5);
            e.field("request.snapshot_with.query");
            e.text(query);
            e.field("request.snapshot_with.arg");
            e.bytes(arg);
            e.field("request.snapshot_with.type");
            encode_qty(ty, e)?;
        }
        Request::ProposeMenu { letters, menu, target, request } => {
            e.field("request.propose_menu");
            e.u8(6);
            e.field("request.propose_menu.letters");
            encode_menu_letters(letters, e)?;
            e.field("request.propose_menu.menu");
            encode_identity(menu, e)?;
            e.field("request.propose_menu.target");
            e.text(target);
            e.field("request.propose_menu.request");
            e.bytes(request);
        }
    }
    Ok(())
}

pub fn decode_request(d: &mut Dec) -> Result<OwnedRequest, Malformed> {
    Ok(match d.u8()? {
        0 => OwnedRequest::Snapshot { query: d.text()?.to_string(), ty: decode_qty(d)? },
        1 => OwnedRequest::Declassified { id: d.text()?.to_string(), ty: decode_ty(d)? },
        2 => OwnedRequest::FileRead { path: d.text()?.to_string(), ty: decode_ty(d)? },
        3 => OwnedRequest::Clock,
        4 => {
            let n = d.count(4)?;
            let word = (0..n).map(|_| d.i32()).collect::<Result<Vec<_>, _>>()?;
            if !canonical_word(&word) {
                return Err(Malformed::new("a letter outside 1 to 1200"));
            }
            OwnedRequest::Propose { word, target: d.text()?.to_string(), request: d.bytes()?.to_vec() }
        }
        5 => OwnedRequest::SnapshotWith { query: d.text()?.to_string(), arg: d.bytes()?.to_vec(), ty: decode_qty(d)? },
        6 => {
            let letters = decode_menu_letters(d)?;
            let menu = decode_identity(d)?;
            OwnedRequest::ProposeMenu { letters, menu, target: d.text()?.to_string(), request: d.bytes()?.to_vec() }
        }
        _ => return Err(Malformed::new("an unknown request")),
    })
}

pub fn encode_sink(s: &Sink, e: &mut Enc) -> Result<(), Unencodable> {
    match s {
        Sink::View => {
            e.field("sink.view");
            e.u8(0);
        }
        Sink::Workspace { record, ty } => {
            e.field("sink.workspace");
            e.u8(1);
            e.field("sink.workspace.record");
            e.text(record);
            e.field("sink.workspace.type");
            encode_qty(ty, e)?;
        }
        Sink::File { path } => {
            e.field("sink.file");
            e.u8(2);
            e.field("sink.file.path");
            e.text(path);
        }
        Sink::Exact { role } => {
            e.field("sink.exact");
            e.u8(3);
            e.field("sink.exact.role");
            e.u8(role_code(*role));
        }
    }
    Ok(())
}

pub fn decode_sink(d: &mut Dec) -> Result<OwnedSink, Malformed> {
    Ok(match d.u8()? {
        0 => OwnedSink::View,
        1 => OwnedSink::Workspace { record: d.text()?.to_string(), ty: decode_qty(d)? },
        2 => OwnedSink::File { path: d.text()?.to_string() },
        3 => OwnedSink::Exact { role: role_from(d.u8()?)? },
        _ => return Err(Malformed::new("an unknown sink")),
    })
}

pub fn encode_tariff(t: &RowTariff, e: &mut Enc) {
    e.u128(t.kernel_cost);
    e.u128(t.result_bound);
}

pub fn decode_tariff(d: &mut Dec) -> Result<RowTariff, Malformed> {
    Ok(RowTariff { kernel_cost: d.u128()?, result_bound: d.u128()? })
}

// ---------------------------------------------------------------------------------------------------------------
// Limits, tariffs and the run summary

pub fn encode_amounts(a: &Amounts, e: &mut Enc) {
    e.u128(a.steps);
    e.u128(a.kernel_cost);
    e.u128(a.memory);
    e.u128(a.output);
}

pub fn decode_amounts(d: &mut Dec) -> Result<Amounts, Malformed> {
    Ok(Amounts { steps: d.u128()?, kernel_cost: d.u128()?, memory: d.u128()?, output: d.u128()? })
}

pub fn encode_tariffs(t: &Tariffs, e: &mut Enc) {
    for v in [
        t.kernel_base,
        t.kernel_per_item,
        t.limb_bits,
        t.add_per_limb,
        t.mul_per_limb_pair,
        t.text_per_scalar,
        t.decode_per_byte,
        t.encode_per_byte,
        t.rng_init_per_byte,
        t.rng_block,
        t.stream_setup,
        t.expand_per_letter,
        t.replay_per_letter,
    ] {
        e.u128(v);
    }
}

pub fn decode_tariffs(d: &mut Dec) -> Result<Tariffs, Malformed> {
    Ok(Tariffs {
        kernel_base: d.u128()?,
        kernel_per_item: d.u128()?,
        limb_bits: d.u128()?,
        add_per_limb: d.u128()?,
        mul_per_limb_pair: d.u128()?,
        text_per_scalar: d.u128()?,
        decode_per_byte: d.u128()?,
        encode_per_byte: d.u128()?,
        rng_init_per_byte: d.u128()?,
        rng_block: d.u128()?,
        stream_setup: d.u128()?,
        expand_per_letter: d.u128()?,
        replay_per_letter: d.u128()?,
    })
}

pub fn encode_draft(t: &DraftTariffs, e: &mut Enc) {
    e.u128(t.compare_per_unit);
    e.u128(t.field_per_unit_pair);
    e.u128(t.convert_per_unit_pair);
}

pub fn decode_draft(d: &mut Dec) -> Result<DraftTariffs, Malformed> {
    Ok(DraftTariffs { compare_per_unit: d.u128()?, field_per_unit_pair: d.u128()?, convert_per_unit_pair: d.u128()? })
}

pub fn interruption_code(why: Interruption) -> u8 {
    match why {
        Interruption::ByUser => 1,
        Interruption::RevisionReplaced => 2,
    }
}

pub fn interruption_from(code: u8) -> Result<Interruption, Malformed> {
    match code {
        1 => Ok(Interruption::ByUser),
        2 => Ok(Interruption::RevisionReplaced),
        _ => Err(Malformed::new("an unknown interruption")),
    }
}

fn outcome_code(o: Outcome) -> u8 {
    match o {
        Outcome::Completed => 0,
        Outcome::CutOff => 1,
        Outcome::Error => 2,
        Outcome::Cancelled => 3,
    }
}

fn outcome_from(code: u8) -> Result<Outcome, Malformed> {
    Ok(match code {
        0 => Outcome::Completed,
        1 => Outcome::CutOff,
        2 => Outcome::Error,
        3 => Outcome::Cancelled,
        _ => return Err(Malformed::new("an unknown outcome")),
    })
}

/// The stop phases a [`Position`] may name (chapter 09: a dimension, `effect` or `interruption`).
pub const PHASES: [&str; 6] = ["steps", "kernel_cost", "memory", "output", "effect", "interruption"];

/// The stop codes the meter and the interpreter raise themselves. A host adds its own refusal codes. LP2's menu words
/// (LB1.8; draft) add the code of a letter outside the pinned menu, and its trace keys (LB1.10; draft) the code of a
/// key under another header than the pinned relation's.
pub const STATIC_CODES: [&str; 17] = [
    meter::code::STEPS,
    meter::code::KERNEL_COST,
    meter::code::MEMORY,
    meter::code::OUTPUT,
    meter::code::CANCELLED_BY_USER,
    meter::code::CANCELLED_REVISION_REPLACED,
    meter::code::POINTWISE_FAILED,
    meter::code::GENERATOR_INPUT,
    meter::code::GENERATOR_EXHAUSTED,
    meter::code::TEXT_SCALAR_INVALID,
    meter::code::NOT_GRANTED,
    interp::code::IR_VERSION_UNSUPPORTED,
    interp::code::IR_REJECTED,
    interp::code::NONCANONICAL_VALUE,
    interp::code::HOST_EXHAUSTED,
    interp::code::INCOMPATIBLE_ALPHABET,
    interp::code::KEY_CONTRACT_MISMATCH,
];

/// The stop codes a summary from the child may name: the static ones and the codes this run's host returned. A code
/// outside both is malformed, so the parent never invents a code and never leaks memory for one.
#[derive(Clone, Debug, Default)]
pub struct Codes {
    host: Vec<&'static str>,
}

impl Codes {
    /// Admits a code the parent's host returned.
    pub fn admit(&mut self, code: &'static str) {
        if !STATIC_CODES.contains(&code) && !self.host.contains(&code) {
            self.host.push(code);
        }
    }

    pub fn resolve(&self, text: &str) -> Option<&'static str> {
        STATIC_CODES.iter().chain(self.host.iter()).copied().find(|c| *c == text)
    }
}

fn encode_position(p: &Position, e: &mut Enc) {
    e.text(&p.at);
    e.text(p.phase);
    e.text(&p.scope);
    match &p.source {
        None => e.u8(0),
        Some(s) => {
            e.u8(1);
            e.text(&s.digest);
            e.u64(s.start as u64);
            e.u64(s.end as u64);
        }
    }
}

fn decode_position(d: &mut Dec) -> Result<Position, Malformed> {
    let at = d.text()?.to_string();
    let phase = d.text()?;
    let phase = PHASES.iter().copied().find(|p| *p == phase).ok_or_else(|| Malformed::new("an unknown phase"))?;
    let scope = d.text()?.to_string();
    let source = match d.u8()? {
        0 => None,
        1 => Some(SourceRange { digest: d.text()?.to_string(), start: d.usize()?, end: d.usize()? }),
        _ => return Err(Malformed::new("an option flag other than 0 or 1")),
    };
    Ok(Position { at, phase, scope, source })
}

fn encode_code(code: Option<&'static str>, e: &mut Enc) {
    match code {
        None => e.u8(0),
        Some(c) => {
            e.u8(1);
            e.text(c);
        }
    }
}

fn decode_code(d: &mut Dec, codes: &Codes) -> Result<Option<&'static str>, Malformed> {
    match d.u8()? {
        0 => Ok(None),
        1 => {
            let text = d.text()?;
            codes.resolve(text).map(Some).ok_or_else(|| Malformed::new(format!("an unknown stop code `{text}`")))
        }
        _ => Err(Malformed::new("an option flag other than 0 or 1")),
    }
}

fn encode_stop(p: &Option<Position>, e: &mut Enc) {
    match p {
        None => e.u8(0),
        Some(p) => {
            e.u8(1);
            encode_position(p, e);
        }
    }
}

fn decode_stop(d: &mut Dec) -> Result<Option<Position>, Malformed> {
    match d.u8()? {
        0 => Ok(None),
        1 => Ok(Some(decode_position(d)?)),
        _ => Err(Malformed::new("an option flag other than 0 or 1")),
    }
}

/// Writes the meter's terminal record: outcome, code, stop, used, live charges, rows and children.
pub fn encode_summary(s: &Summary, e: &mut Enc) {
    e.u8(outcome_code(s.outcome));
    encode_code(s.code, e);
    encode_stop(&s.stop, e);
    encode_amounts(&s.used, e);
    e.u128(s.live_before_cleanup);
    e.u128(s.live_after_cleanup);
    e.count(s.rows.len());
    for r in &s.rows {
        e.text(&r.at);
        e.u128(r.steps);
        e.u128(r.kernel_cost);
        e.u128(r.memory);
        e.u128(r.output);
        e.u128(r.released);
    }
    e.count(s.children.len());
    for c in &s.children {
        e.text(&c.at);
        encode_amounts(&c.requested, e);
        encode_amounts(&c.effective, e);
        e.u8(outcome_code(c.outcome));
        encode_code(c.code, e);
        encode_stop(&c.stop, e);
        encode_amounts(&c.used, e);
        e.u128(c.returned_memory);
        e.u128(c.live_after_return);
    }
}

pub fn decode_summary(d: &mut Dec, codes: &Codes) -> Result<Summary, Malformed> {
    let outcome = outcome_from(d.u8()?)?;
    let code = decode_code(d, codes)?;
    let stop = decode_stop(d)?;
    let used = decode_amounts(d)?;
    let live_before_cleanup = d.u128()?;
    let live_after_cleanup = d.u128()?;
    let n = d.count(8 + 5 * 16)?;
    let mut rows = Vec::with_capacity(n);
    for _ in 0..n {
        rows.push(Row {
            at: d.text()?.to_string(),
            steps: d.u128()?,
            kernel_cost: d.u128()?,
            memory: d.u128()?,
            output: d.u128()?,
            released: d.u128()?,
        });
    }
    let n = d.count(8 + 3 * 64)?;
    let mut children = Vec::with_capacity(n);
    for _ in 0..n {
        children.push(ChildRecord {
            at: d.text()?.to_string(),
            requested: decode_amounts(d)?,
            effective: decode_amounts(d)?,
            outcome: outcome_from(d.u8()?)?,
            code: decode_code(d, codes)?,
            stop: decode_stop(d)?,
            used: decode_amounts(d)?,
            returned_memory: d.u128()?,
            live_after_return: d.u128()?,
        });
    }
    Ok(Summary { outcome, code, stop, used, live_before_cleanup, live_after_cleanup, rows, children })
}

/// The transfer forms of LB1.10's jumbling relation and of its trace keys over menu letters (LP2 draft). LP3's protocol
/// tests cover the Full forms, whose layout these additions keep.
#[cfg(test)]
mod menu_trace_tests {
    use super::*;
    use c600_run::trace::{FULL_DISJOINT_SUPPORT, FULL_GENERATORS};

    fn identity(menu_id: &str) -> MenuIdentity {
        MenuIdentity {
            puzzle: "600-cell-jumbled".into(),
            model_id: "model-1".into(),
            menu_id: menu_id.into(),
            menu_order: "order-1".into(),
            contract_rev: "rev-1".into(),
        }
    }

    /// One bit per pair of `n` symbols, every pair independent but (0, 2).
    fn bits(n: usize) -> Vec<u8> {
        let pairs = n * (n - 1) / 2;
        let mut bits = vec![0u8; pairs.div_ceil(8)];
        for k in 0..pairs {
            bits[k / 8] |= 1 << (k % 8);
        }
        bits[0] &= !0b10;
        bits
    }

    fn relation(menu_id: &str) -> Relation {
        Relation::jumble_locality(&identity(menu_id), 600, 3, bits(600)).expect("a relation")
    }

    fn relation_bytes(r: &Relation) -> Vec<u8> {
        let mut e = Enc::new();
        encode_relation(r, &mut e);
        e.bytes
    }

    fn relation_from(bytes: &[u8]) -> Result<Relation, Malformed> {
        let mut d = Dec::new(bytes);
        let r = decode_relation(&mut d)?;
        d.end()?;
        Ok(r)
    }

    /// The wire parts of a jumbling relation, as [`encode_relation`] writes them.
    fn jumbling_parts(identity: &MenuIdentity, grips: u32, items: u32, bits: &[u8]) -> Vec<u8> {
        let mut e = Enc::new();
        e.text(JUMBLE_LOCALITY);
        for (_, text) in identity.fields() {
            e.text(text);
        }
        e.u32(grips);
        e.u32(items);
        e.bytes(bits);
        e.bytes
    }

    #[test]
    fn a_jumbling_relation_crosses_with_its_menu_and_its_digest_is_recomputed() {
        let r = relation("menu-a");
        let back = relation_from(&relation_bytes(&r)).expect("a relation");
        assert_eq!((&back, back.digest(), back.header()), (&r, r.digest(), r.header()));
        assert!(back.is_menu() && !back.menu_independent([0, 1], [2, 0]) && back.menu_independent([0, 1], [1, 0]));
        assert_eq!(relation_bytes(&r), jumbling_parts(&identity("menu-a"), 600, 3, &bits(600)));

        // A Full relation crosses as before: contract, puzzle, model ID, generator count and bits.
        let full = Relation::from_parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "model-1", FULL_GENERATORS, bits(1200));
        let full = full.expect("a relation");
        let mut e = Enc::new();
        for text in [FULL_DISJOINT_SUPPORT, "600-cell-Full", "model-1"] {
            e.text(text);
        }
        e.u32(FULL_GENERATORS);
        e.bytes(&bits(1200));
        assert_eq!(relation_bytes(&full), e.bytes);
        assert_eq!(relation_from(&e.bytes).expect("a relation"), full);

        // The receiver refuses what `Relation::jumble_locality` refuses.
        let mut unnamed = identity("menu-a");
        unnamed.menu_order.clear();
        let mut padded = bits(600);
        *padded.last_mut().unwrap() |= 0x80;
        for (bytes, why) in [
            (jumbling_parts(&identity("menu-a"), 599, 3, &bits(599)), "599 grips"),
            (jumbling_parts(&identity("menu-a"), 600, 0, &bits(600)), "0 menu items"),
            (jumbling_parts(&identity("menu-a"), 600, 601, &bits(600)), "601 menu items"),
            (jumbling_parts(&unnamed, 600, 3, &bits(600)), "menu_order is empty"),
            (jumbling_parts(&identity("menu-a"), 600, 3, &bits(600)[1..]), "bits"),
            (jumbling_parts(&identity("menu-a"), 600, 3, &padded), "bits"),
        ] {
            let refused = relation_from(&bytes).expect_err(why);
            assert!(refused.0.contains(why), "{why}: {refused}");
        }
    }

    #[test]
    fn a_menu_trace_key_crosses_with_its_header_and_its_letters() {
        let r = relation("menu-a");
        let key = r.menu_key(&[[2, 1], [0, 2], [1, 1]]).expect("letters of the alphabet");
        assert_eq!(key.menu_letters(), [[1, 1], [2, 1], [0, 2]]);
        let value = Value::TraceEqualityKey(Arc::new(key.clone()));
        let bytes = value_bytes(&value).expect("a transfer form");
        assert_eq!(bytes[0], tag::MENU_TRACE_KEY);
        let back = value_from_bytes(&bytes).expect("a value");
        assert!(same_value(&back, &value));
        let Value::TraceEqualityKey(back) = back else { panic!("a trace key") };
        assert_eq!((back.header(), back.menu_letters(), back.is_menu()), (key.header(), key.menu_letters(), true));
        // A key under another menu crosses with that menu's header.
        let other = Value::TraceEqualityKey(Arc::new(relation("menu-b").menu_key(&[[1, 1]]).expect("a key")));
        let other = value_from_bytes(&value_bytes(&other).expect("a transfer form")).expect("a value");
        let Value::TraceEqualityKey(other) = other else { panic!("a trace key") };
        assert_eq!(other.header().differing(key.header()), Some("menu_id"));

        // Letters of one alphabet under the other's header have no transfer form.
        let menu_header = Arc::new(r.header().clone());
        let full = Relation::from_parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "model-1", FULL_GENERATORS, bits(1200));
        let full_header = Arc::new(full.expect("a relation").header().clone());
        let refused = |k: TraceKey| value_bytes(&Value::TraceEqualityKey(Arc::new(k))).err();
        assert!(refused(TraceKey::from_parts(menu_header.clone(), &[1])).is_some());
        assert!(refused(TraceKey::from_menu_parts(full_header, &[[0, 1]])).is_some());
        assert!(refused(TraceKey::from_menu_parts(menu_header, &[[600, 0]])).is_some());

        // The receiver refuses a menu index of 600 and a header field that is empty.
        let at = bytes.len() - 8;
        let mut outside = bytes.clone();
        outside[at..at + 4].copy_from_slice(&600u32.to_le_bytes());
        assert!(value_from_bytes(&outside).unwrap_err().0.contains("600 or more"));
        let mut e = Enc::new();
        e.u8(tag::MENU_TRACE_KEY);
        let mut unnamed = identity("menu-a");
        unnamed.contract_rev.clear();
        for (_, text) in unnamed.fields() {
            e.text(text);
        }
        e.u32(600);
        e.u32(3);
        e.text(c600_run::trace::TRACE_KEY);
        e.text(JUMBLE_LOCALITY);
        e.bytes(r.digest());
        e.count(0);
        assert!(value_from_bytes(&e.bytes).unwrap_err().0.contains("empty field"));
    }
}

/// The transfer form of LG2.18's word key over menu letters (LP2 draft): tag 27, after the menu trace key's.
#[cfg(test)]
mod menu_word_key_tests {
    use super::*;
    use c600_run::menu::menu_digests;
    use c600_run::menu_key;

    /// A rotation by the angle with cosine c/d and sine s/d in the plane of axes 0 and 1.
    fn rotation(c: i64, s: i64, d: i64) -> ItemMatrix {
        let q = |a: i64, d: i64| Quad::new(BigInt::from(a), BigInt::zero(), BigInt::from(d));
        std::array::from_fn(|i| {
            std::array::from_fn(|j| match (i, j) {
                (0, 0) | (1, 1) => q(c, d),
                (0, 1) => q(-s, d),
                (1, 0) => q(s, d),
                _ if i == j => q(1, 1),
                _ => q(0, 1),
            })
        })
    }

    /// The identity, the 3-4-5 rotation and its inverse.
    fn items() -> Vec<ItemMatrix> {
        vec![rotation(1, 0, 1), rotation(3, 4, 5), rotation(3, -4, 5)]
    }

    fn alphabet() -> MenuAlphabet {
        let (menu_id, menu_order) = menu_digests(&items());
        let identity = MenuIdentity {
            puzzle: "600-cell-jumbled".into(),
            model_id: "model-1".into(),
            menu_id,
            menu_order,
            contract_rev: "rev-1".into(),
        };
        let bare = MenuAlphabet::new(identity, 600, vec![0, 2, 1]).expect("an alphabet");
        bare.with_matrices(items()).expect("the menu's matrices")
    }

    /// The transfer form of a key's parts, written field by field.
    fn parts(identity: &MenuIdentity, kind: &str, grips: u32, tokens: &[Token]) -> Vec<u8> {
        let mut e = Enc::new();
        e.u8(tag::MENU_WORD_KEY);
        for (_, text) in identity.fields() {
            e.text(text);
        }
        e.text(kind);
        e.u32(grips);
        e.u32(3);
        e.count(tokens.len());
        for t in tokens {
            e.u32(t.grip);
            for q in t.product.iter().flatten() {
                bigint(&q.a, &mut e);
                bigint(&q.b, &mut e);
                bigint(&q.d, &mut e);
            }
        }
        e.bytes
    }

    #[test]
    fn a_menu_word_key_crosses_with_its_header_and_its_tokens() {
        let alphabet = alphabet();
        let key = menu_key::key(&alphabet, &[[4, 1], [4, 1], [2, 2], [2, 1]]).expect("letters of the alphabet");
        assert_eq!(key.tokens().len(), 2);
        let bytes = parts(alphabet.identity(), WORD_KEY, 600, key.tokens());
        let value = Value::MenuWordKey(Arc::new(key));
        assert_eq!(value_bytes(&value).expect("a transfer form"), bytes);
        let back = value_from_bytes(&bytes).expect("a value");
        assert!(same_value(&back, &value));
        let (Value::MenuWordKey(back), Value::MenuWordKey(key)) = (&back, &value) else { panic!("menu word keys") };
        assert_eq!(back, key);
        // The empty key crosses as its header alone.
        let empty = Value::MenuWordKey(Arc::new(menu_key::key(&alphabet, &[]).expect("a key")));
        let back = value_from_bytes(&value_bytes(&empty).expect("a transfer form")).expect("a value");
        assert!(same_value(&back, &empty));

        // A key outside its canonical form, or under a block that names no alphabet, has no transfer form, and the
        // receiver refuses what the sender would not write.
        let token = |grip: u32, product: ItemMatrix| Token { grip, product };
        let mut halves = rotation(3, 4, 5);
        halves[0][0] = Quad { a: BigInt::from(6), b: BigInt::zero(), d: BigInt::from(10) };
        let mut unnamed = alphabet.identity().clone();
        unnamed.contract_rev.clear();
        let named = alphabet.identity();
        let cases: [(MenuIdentity, &str, u32, Vec<Token>, &str); 5] = [
            (named.clone(), WORD_KEY, 600, vec![token(600, rotation(3, 4, 5))], "canonical form"),
            (
                named.clone(),
                WORD_KEY,
                600,
                vec![token(4, rotation(3, 4, 5)), token(4, rotation(1, 0, 1))],
                "canonical form",
            ),
            (named.clone(), WORD_KEY, 600, vec![token(4, halves)], "canonical form"),
            (unnamed, WORD_KEY, 600, vec![token(4, rotation(3, 4, 5))], "names no alphabet"),
            (named.clone(), "word-key/2", 600, vec![], "another kind"),
        ];
        for (identity, kind, grips, tokens, why) in cases {
            let bytes = parts(&identity, kind, grips, &tokens);
            assert!(value_from_bytes(&bytes).unwrap_err().0.contains(why), "{why}");
            if kind == WORD_KEY {
                let key = MenuWordKey::from_parts(Arc::new(identity), grips, 3, tokens);
                assert!(value_bytes(&Value::MenuWordKey(Arc::new(key))).is_err(), "{why}");
            }
        }
        // A token count beyond the payload.
        let mut e = Enc::new();
        e.u8(tag::MENU_WORD_KEY);
        for (_, text) in named.fields() {
            e.text(text);
        }
        e.text(WORD_KEY);
        e.u32(600);
        e.u32(3);
        e.count(1);
        e.u32(0);
        assert!(value_from_bytes(&e.bytes).unwrap_err().0.contains("beyond the payload"));
    }
}
