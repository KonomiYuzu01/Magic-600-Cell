//! Run-time values (LP2, M2): their DET.57 logical sizes, exact literals, exact arithmetic and exact equality.
//!
//! A value that holds heap data shares it through an `Arc`, so a borrow or an alias copies a reference and never the
//! payload; the meter still charges an alias in full (DET.57). Records and closures drop their contents with an explicit
//! stack, and equality walks one, so a deep value cannot overflow the host stack.

use std::cmp::Ordering;
use std::sync::{Arc, Mutex, MutexGuard};

use c600_ir::{EndTest, EndTestOutcome, Field};
use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::approx::{self, Approx};
use crate::menu::{MenuIdentity, MENU_INDEX_LIMIT};
use crate::meter::{integer_size, DraftTariffs, IntOp, Tariffs, GENERATOR_SIZE};
use crate::rng::Cursor;
use crate::trace::{tokens_order, TraceKey};

/// The most decimal digits an exact literal may have: PS1.5's limit for persisted integers, applied to IR literals.
pub const MAX_DIGITS: usize = 10_000;

/// A run-time value. Values follow their static types: `Nat`, `Int` and `Budget` are integers, a `Rat` stays a rational
/// when its denominator is 1, and a `Q5` keeps a zero √5 coefficient.
#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(Arc<BigInt>),
    Rat(Arc<Ratio>),
    Q5(Arc<Quad>),
    Approx(Approx),
    ApproxBool(bool),
    Text(Arc<Text>),
    Target(Arc<Text>),
    /// Signed letters of magnitude 1 to 1200 in written order.
    Word(Arc<[i32]>),
    Tag(Arc<Text>),
    Colour(Arc<Text>),
    Record(Arc<Record>),
    Closure(Arc<Closure>),
    /// A named function, by its index in the loaded program.
    Fn(usize),
    /// A preview receipt (BND.42): the fields the host supplied. A field it withholds is absent.
    Receipt(Arc<Record>),
    /// A list (LP2 draft): its elements in order.
    List(Arc<List>),
    /// LG2.13's word equality key: the letters after LG2.18's free reduction. It is never a `Word`.
    WordEqualityKey(Arc<[i32]>),
    /// A set (LP2 draft): its distinct elements in canonical key order ([`key_order`], DET.9).
    Set(Arc<List>),
    /// A map (LP2 draft): its entries, `{ key, value }` records ([`entry`]), in the canonical key order of their keys.
    Map(Arc<List>),
    /// LG2.16's compressed word (LP2 draft): a straight-line program with its exact length.
    Compressed(Arc<Compressed>),
    /// LB1.8's literal word of jumbling menu letters (LP2 draft): its letters and its alphabet's identity block.
    MenuWord(Arc<MenuWord>),
    /// An end test's verdict (LB3.2; LP2 draft): the test it answers and its outcome. A program reads it from the
    /// test's engine row and uses it through a `match` on its outcome only (LG2.9).
    EndTest(EndTest, EndTestOutcome),
    /// LB1.10's trace equality key of a Full word (LP2 draft): its letters in the least topological order under the
    /// run's pinned independence relation, with that relation's header (LG2.23). It is never a `Word` or a
    /// `WordEqualityKey`.
    TraceEqualityKey(Arc<TraceKey>),
    /// DET.61's generator (LP2 draft): one owner's cursor on a sealed stream. A clone shares the cursor, as a visit, a
    /// borrow or a move does; [`Value::alias`] copies it into an independent cursor, as DET.61's alias does.
    Generator(Arc<Generator>),
    /// An element of F2 or F5 (KN3.2; LP2 draft): the field and its residue, which is below the field's prime. Build
    /// one with [`Value::field`].
    Field(Field, u8),
}

/// A generator's cursor (DET.61; LP2 draft). The checker keeps a generator a bare value, so no record, collection or
/// closure capture shares one.
#[derive(Debug)]
pub struct Generator {
    cursor: Mutex<Cursor>,
}

impl Generator {
    pub fn new(cursor: Cursor) -> Generator {
        Generator { cursor: Mutex::new(cursor) }
    }

    fn lock(&self) -> MutexGuard<'_, Cursor> {
        self.cursor.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The stream's next block, which advances the shared cursor; `None` once the stream is exhausted.
    pub fn draw(&self) -> Option<[u8; 32]> {
        self.lock().draw()
    }

    /// The cursor as it stands: the stream key and the counter of the next draw.
    pub fn state(&self) -> Cursor {
        self.lock().clone()
    }
}

/// A list: its elements in order, its logical size and an upper bound of its canonical encoding (LP2 draft).
#[derive(Debug)]
pub struct List {
    items: Vec<Value>,
    size: u128,
    bound: u128,
}

impl List {
    /// DET.57: a 4-byte variant tag and the owned elements, each variable sequence among them with its 8-byte
    /// length, as a record stores its fields. The list's own 8-byte length is counted where it is stored.
    pub fn new(items: Vec<Value>) -> List {
        let size = List::size_of(&items);
        let bound = items.iter().fold(2u128, |s, v| s.saturating_add(v.bound()).saturating_add(1));
        List { items, size, bound }
    }

    /// The logical size of a list, set or map over these items, before it is built.
    pub fn size_of(items: &[Value]) -> u128 {
        items.iter().fold(4u128, |s, v| s.saturating_add(v.size()).saturating_add(if v.is_sequence() { 8 } else { 0 }))
    }

    pub fn items(&self) -> &[Value] {
        &self.items
    }

    /// The logical size of a list header over these elements: the variant tag and the sequence lengths.
    pub fn header(items: &[Value]) -> u128 {
        Record::header(items)
    }
}

impl Drop for List {
    fn drop(&mut self) {
        let mut stack = std::mem::take(&mut self.items);
        while let Some(v) = stack.pop() {
            take_children(v, &mut stack);
        }
    }
}

/// A compressed word (LG2.16, KN2.10; LP2 draft): a straight-line program. Each part is a leaf of literal letters or
/// the concatenation of two earlier parts, and the last part is the word. Its exact length, an unbounded integer, is
/// known without expansion. Only the word itself may be empty: a concatenation never takes an empty part, so every
/// part a reading visits holds letters. Each part keeps its exact length, which a reading needs to find a position, so
/// the lengths are payload: a chain of n doublings keeps about n²/2 bits of them, and its logical size counts them. A
/// word and its joins share the stored lengths, and each still counts them in full (DET.57 gives sharing no discount).
/// The heights are derived host metadata of fixed width, not payload.
#[derive(Debug)]
pub struct Compressed {
    parts: Vec<Part>,
    /// Each part's exact length. A join copies the handles of its operands' lengths and adds one, so its work is one
    /// step per part, as `concat` is priced, and not one per bit kept.
    lengths: Vec<Arc<BigInt>>,
    heights: Vec<u128>,
    /// DET.57's payload of the parts: per part a 4-byte variant tag, then for a leaf its 8-byte letter count and two
    /// bytes per letter, as a word stores them, and for a concatenation its two 8-byte part numbers, and then the
    /// part's exact length as an unbounded integer.
    parts_size: u128,
}

/// A menu word (LB1.8; LP2 draft): letters `[cap, item]` in written order, each index below
/// [`MENU_INDEX_LIMIT`], and the identity block of the alphabet they belong to ([`crate::menu`]). The identity is the
/// run's pinned metadata, shared by the run's words, and not payload: a word's logical size is 4 bytes per letter, two
/// 2-byte indices (DET.57). Words compare by their identity blocks, then by their letters.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MenuWord {
    identity: Arc<MenuIdentity>,
    letters: Arc<[[u32; 2]]>,
}

impl MenuWord {
    /// The word of `letters` in the alphabet named by `identity`. `None` when an index is not below
    /// [`MENU_INDEX_LIMIT`]; whether a letter is in the alphabet is the run's check, against its pinned alphabet.
    pub fn new(identity: Arc<MenuIdentity>, letters: &[[u32; 2]]) -> Option<MenuWord> {
        let bounded = letters.iter().flatten().all(|&i| i < MENU_INDEX_LIMIT);
        bounded.then(|| MenuWord { identity, letters: letters.into() })
    }

    /// The identity block of the word's alphabet.
    pub fn identity(&self) -> &MenuIdentity {
        &self.identity
    }

    /// The letters `[cap, item]`, in order.
    pub fn letters(&self) -> &[[u32; 2]] {
        &self.letters
    }

    /// The same letters in the alphabet named by `identity`: a literal takes the run's alphabet when it is read.
    pub(crate) fn named(&self, identity: Arc<MenuIdentity>) -> MenuWord {
        MenuWord { identity, letters: self.letters.clone() }
    }

    /// An upper bound of the canonical encoding: `{"letters":[[c,i],…],"menu":{…}}` with the identity block's five
    /// fields under their names, at most 10 bytes per letter and 86 bytes of keys, brackets and punctuation
    /// (DET.59).
    fn bound(&self) -> u128 {
        let fields =
            self.identity.fields().iter().fold(0u128, |s, (_, v)| {
                s.saturating_add((v.chars().count() as u128).saturating_mul(6).saturating_add(2))
            });
        (self.letters.len() as u128).saturating_mul(10).saturating_add(86).saturating_add(fields)
    }
}

/// One part of a compressed word (LP2 draft).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    Leaf(Arc<[i32]>),
    /// The concatenation of two earlier parts, by their positions.
    Concat(usize, usize),
}

/// The logical payload of a concatenation part (DET.57) before its length: its tag and two part numbers.
const CONCAT_PART: u128 = 20;

impl Compressed {
    /// The word `letters` as a compressed word of one part.
    pub fn leaf(letters: Arc<[i32]>) -> Compressed {
        let mut c = Compressed { parts: Vec::new(), lengths: Vec::new(), heights: Vec::new(), parts_size: 0 };
        c.push(Part::Leaf(letters));
        c
    }

    /// `a` then `b`, without expansion. An empty operand gives the other. Otherwise the result holds the parts of `a`,
    /// then those of `b` unless the two are one value, then their concatenation: a word joined to itself adds one part.
    /// The result shares the operands' stored lengths and computes only its new part's.
    pub fn concat(a: &Arc<Compressed>, b: &Arc<Compressed>) -> Arc<Compressed> {
        if a.length().is_zero() {
            return b.clone();
        }
        if b.length().is_zero() {
            return a.clone();
        }
        let mut c = Compressed {
            parts: a.parts.clone(),
            lengths: a.lengths.clone(),
            heights: a.heights.clone(),
            parts_size: a.parts_size,
        };
        let first = a.parts.len() - 1;
        if !Arc::ptr_eq(a, b) && a.parts != b.parts {
            let shift = a.parts.len();
            c.parts.extend(b.parts.iter().map(|p| match p {
                Part::Leaf(l) => Part::Leaf(l.clone()),
                Part::Concat(i, j) => Part::Concat(i + shift, j + shift),
            }));
            c.lengths.extend(b.lengths.iter().cloned());
            c.heights.extend(b.heights.iter().copied());
            c.parts_size = c.parts_size.saturating_add(b.parts_size);
        }
        let second = c.parts.len() - 1;
        c.push(Part::Concat(first, second));
        Arc::new(c)
    }

    /// A compressed word from its parts in order, as [`Compressed::part_list`] gives them, for a transfer form. It
    /// derives the lengths, heights and logical size as `leaf` and `concat` do. It refuses a list without parts, a
    /// letter of magnitude 0 or above 1200, a concatenation of a part not before it or of an empty part, and a part the
    /// word does not reach, so only the word itself may be empty, as for a built word. `leaf` checks no letters, so a
    /// word built from letters outside `Value::Word`'s range is refused here, as a transfer form refuses such a word.
    /// Any other straight-line program is accepted, also a shape `concat` does not build; such a word equals only a
    /// word with the same parts. The checks run before any length is derived and use no recursion. The derived lengths
    /// grow as `concat`'s do, to about n²/2 bits for a chain of n doublings, so a caller decoding untrusted input
    /// bounds the parts.
    pub fn from_parts(parts: Vec<Part>) -> Result<Compressed, &'static str> {
        if parts.is_empty() {
            return Err("a compressed word without parts");
        }
        // A concatenation of two parts that hold letters holds letters, so only a leaf can be empty.
        let mut empty = Vec::with_capacity(parts.len());
        for (k, part) in parts.iter().enumerate() {
            match part {
                Part::Leaf(l) => {
                    if l.iter().any(|x| *x == 0 || x.unsigned_abs() > 1200) {
                        return Err("a letter outside 1 to 1200");
                    }
                    empty.push(l.is_empty());
                }
                Part::Concat(i, j) => {
                    if *i >= k || *j >= k {
                        return Err("a concatenation of a part not before it");
                    }
                    if empty[*i] || empty[*j] {
                        return Err("a concatenation of an empty part");
                    }
                    empty.push(false);
                }
            }
        }
        // A concatenation names only earlier parts, so one pass down from the word marks each part it reaches before
        // that part's turn comes.
        let last = parts.len() - 1;
        let mut reached = vec![false; parts.len()];
        reached[last] = true;
        for k in (0..=last).rev() {
            if !reached[k] {
                return Err("a part the word does not reach");
            }
            if let Part::Concat(i, j) = &parts[k] {
                reached[*i] = true;
                reached[*j] = true;
            }
        }
        let mut c = Compressed {
            parts: Vec::with_capacity(parts.len()),
            lengths: Vec::with_capacity(parts.len()),
            heights: Vec::with_capacity(parts.len()),
            parts_size: 0,
        };
        for part in parts {
            c.push(part);
        }
        Ok(c)
    }

    fn push(&mut self, part: Part) {
        let (length, height, size) = match &part {
            Part::Leaf(l) => (BigInt::from(l.len()), 0, 12 + 2 * l.len() as u128),
            Part::Concat(i, j) => {
                (&*self.lengths[*i] + &*self.lengths[*j], 1 + self.heights[*i].max(self.heights[*j]), CONCAT_PART)
            }
        };
        self.parts_size = self.parts_size.saturating_add(size).saturating_add(int_size(&length));
        self.parts.push(part);
        self.lengths.push(Arc::new(length));
        self.heights.push(height);
    }

    /// The exact expanded length (LG2.16).
    pub fn length(&self) -> &BigInt {
        self.lengths.last().expect("a compressed word has a part")
    }

    /// The number of parts.
    pub fn parts(&self) -> usize {
        self.parts.len()
    }

    /// The parts in order; the last is the word. [`Compressed::from_parts`] rebuilds the word from them.
    pub fn part_list(&self) -> &[Part] {
        &self.parts
    }

    /// The longest chain of concatenations from the word to a leaf.
    pub fn height(&self) -> u128 {
        *self.heights.last().expect("a compressed word has a part")
    }

    /// DET.57's logical size: a 4-byte variant tag and the parts, each with its exact length as an unbounded integer;
    /// the last part's length is the word's. The part count's 8-byte length is counted where the word is stored, as
    /// for a list.
    pub fn size(&self) -> u128 {
        4u128.saturating_add(self.parts_size)
    }

    /// The size of `concat(a, b)` when neither is empty and the two are different values: a bound for any
    /// concatenation of the two.
    pub fn concat_bound(a: &Compressed, b: &Compressed) -> u128 {
        let parts = a.parts_size.saturating_add(b.parts_size).saturating_add(CONCAT_PART);
        4u128.saturating_add(parts).saturating_add(int_size(&(a.length() + b.length())))
    }

    /// The `count` letters from the exact position `start` in chronological order (LG4.6), or `None` if the range
    /// passes the word's end. It descends to the leaf that holds `start`, then reads whole parts in order with an
    /// explicit stack, so it visits at most 2·(height + count) + 1 parts and expands nothing outside the range.
    pub fn expand(&self, start: &BigInt, count: usize) -> Option<Vec<i32>> {
        if start.is_negative() || &(start + BigInt::from(count)) > self.length() {
            return None;
        }
        let mut out = Vec::with_capacity(count);
        if count == 0 {
            return Some(out);
        }
        let mut pending = Vec::new();
        let (mut at, mut offset) = (self.parts.len() - 1, start.clone());
        let (leaf, from) = loop {
            match &self.parts[at] {
                Part::Concat(i, j) if offset < *self.lengths[*i] => {
                    pending.push(*j);
                    at = *i;
                }
                Part::Concat(i, j) => {
                    offset -= &*self.lengths[*i];
                    at = *j;
                }
                Part::Leaf(l) => break (l, usize::try_from(&offset).expect("an offset within its leaf")),
            }
        };
        out.extend_from_slice(&leaf[from..leaf.len().min(from.saturating_add(count))]);
        while out.len() < count {
            match &self.parts[pending.pop().expect("the range lies within the word")] {
                Part::Concat(i, j) => pending.extend([*j, *i]),
                Part::Leaf(l) => out.extend_from_slice(&l[..l.len().min(count - out.len())]),
            }
        }
        Some(out)
    }
}

/// LG2.18's key of a Full word: a stack scan in chronological order that cancels only adjacent inverse pairs.
pub fn free_reduction(letters: &[i32]) -> Vec<i32> {
    let mut stack: Vec<i32> = Vec::with_capacity(letters.len());
    for &l in letters {
        if stack.last() == Some(&-l) {
            stack.pop();
        } else {
            stack.push(l);
        }
    }
    stack
}

/// The inverse of a word: its letters in reverse order, each negated (LG2.15).
pub fn inverse(letters: &[i32]) -> Vec<i32> {
    letters.iter().rev().map(|l| -l).collect()
}

/// The canonical key order (DET.9; LP2 draft), a total order on values. Exact numbers compare by value, `false` comes
/// before `true`, texts compare by their scalar values, words and word keys by their letters, and lists, sets, maps,
/// records and closures by their parts in order, each lexicographically with a prefix first; values of different
/// kinds compare by kind. Equal numbers in different representations, an `Int` and a `Rat` of one value, have their
/// representation as the explicit secondary key: `Int`, then `Rat`, then `Q5`. An F2 or F5 element is no such number:
/// elements compare by their field, F2 first, then by their residue (LP2 draft).
pub fn key_order(a: &Value, b: &Value) -> Ordering {
    walk_order(a, b, false).then_with(|| walk_order(a, b, true))
}

/// The canonical key order without its secondary key: the order of key equality, under which an `Int` and a `Rat` of
/// one value are one key, as exact equality has it.
pub fn value_order(a: &Value, b: &Value) -> Ordering {
    walk_order(a, b, false)
}

/// One step of a key comparison.
enum Step<'a> {
    Pair(&'a Value, &'a Value),
    Name(&'a str, &'a str),
    Count(usize, usize),
}

/// The rank of a value's kind, which orders values of different kinds.
fn kind(v: &Value) -> u8 {
    match v {
        Value::Unit => 0,
        Value::Bool(_) => 1,
        Value::Int(_) | Value::Rat(_) | Value::Q5(_) => 2,
        Value::Approx(_) => 3,
        Value::ApproxBool(_) => 4,
        Value::Text(_) => 5,
        Value::Target(_) => 6,
        Value::Word(_) => 7,
        Value::Tag(_) => 8,
        Value::Colour(_) => 9,
        Value::Record(_) => 10,
        Value::Closure(_) => 11,
        Value::Fn(_) => 12,
        Value::Receipt(_) => 13,
        Value::List(_) => 14,
        Value::WordEqualityKey(_) => 15,
        Value::Set(_) => 16,
        Value::Map(_) => 17,
        Value::Compressed(_) => 18,
        Value::MenuWord(_) => 19,
        Value::EndTest(..) => 20,
        Value::TraceEqualityKey(_) => 21,
        Value::Generator(_) => 22,
        Value::Field(..) => 23,
    }
}

/// Two values in key order, walked with an explicit stack so that a deep key cannot overflow the host stack. With
/// `ranks`, equal numbers compare by their representation.
fn walk_order(a: &Value, b: &Value, ranks: bool) -> Ordering {
    let mut stack = vec![Step::Pair(a, b)];
    while let Some(step) = stack.pop() {
        let order = match step {
            Step::Count(x, y) => x.cmp(&y),
            Step::Name(x, y) => x.cmp(y),
            Step::Pair(a, b) => match (a, b) {
                (Value::Bool(x), Value::Bool(y)) | (Value::ApproxBool(x), Value::ApproxBool(y)) => x.cmp(y),
                (Value::Approx(x), Value::Approx(y)) => {
                    let parts = |v: &Approx| (v.precision, v.negative, v.mantissa, v.exponent);
                    parts(x).cmp(&parts(y))
                }
                (Value::Text(x), Value::Text(y))
                | (Value::Target(x), Value::Target(y))
                | (Value::Tag(x), Value::Tag(y))
                | (Value::Colour(x), Value::Colour(y)) => x.as_str().cmp(y.as_str()),
                (Value::Word(x), Value::Word(y)) => x.cmp(y),
                // LG2.23's token order: absolute generator ID, then sign, the inverse first.
                (Value::WordEqualityKey(x), Value::WordEqualityKey(y)) => tokens_order(x, y),
                // Headers first, then LG2.23's token order. A run pins one relation, so its trace keys share a header.
                (Value::TraceEqualityKey(x), Value::TraceEqualityKey(y)) => x.cmp(y),
                (Value::List(x), Value::List(y)) | (Value::Set(x), Value::Set(y)) | (Value::Map(x), Value::Map(y)) => {
                    parts(&mut stack, &x.items, &y.items);
                    Ordering::Equal
                }
                (Value::Record(x), Value::Record(y)) | (Value::Receipt(x), Value::Receipt(y)) => {
                    stack.push(Step::Count(x.fields.len(), y.fields.len()));
                    for ((kx, vx), (ky, vy)) in x.fields.iter().zip(&y.fields).rev() {
                        stack.push(Step::Pair(vx, vy));
                        stack.push(Step::Name(kx.as_str(), ky.as_str()));
                    }
                    Ordering::Equal
                }
                (Value::Closure(x), Value::Closure(y)) => {
                    parts(&mut stack, &x.captures, &y.captures);
                    x.code.cmp(&y.code)
                }
                (Value::Fn(x), Value::Fn(y)) => x.cmp(y),
                // By their parts: no key or equality of a checked program compares compressed words (LP2 draft).
                (Value::Compressed(x), Value::Compressed(y)) => x.parts.cmp(&y.parts),
                (Value::MenuWord(x), Value::MenuWord(y)) => x.cmp(y),
                (Value::EndTest(tx, ox), Value::EndTest(ty, oy)) => (tx, ox).cmp(&(ty, oy)),
                // By their cursors, each read on its own: no key or equality of a checked program holds a generator.
                (Value::Generator(x), Value::Generator(y)) => {
                    let (x, y) = (x.state(), y.state());
                    (x.key, x.counter).cmp(&(y.key, y.counter))
                }
                (Value::Field(f, x), Value::Field(g, y)) => (f, x).cmp(&(g, y)),
                _ => match (num(a), num(b)) {
                    (Some(x), Some(y)) if ranks => x.rank().cmp(&y.rank()),
                    (Some(_), Some(_)) => compare(a, b).expect("exact numbers"),
                    _ => kind(a).cmp(&kind(b)),
                },
            },
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    Ordering::Equal
}

/// Pushes two sequences for a lexicographic comparison: their parts in order, then their counts.
fn parts<'a>(stack: &mut Vec<Step<'a>>, x: &'a [Value], y: &'a [Value]) {
    stack.push(Step::Count(x.len(), y.len()));
    for (p, q) in x.iter().zip(y).rev() {
        stack.push(Step::Pair(p, q));
    }
}

/// A map entry: the record `{ key, value }` (LP2 draft).
pub fn entry(key: Value, value: Value) -> Value {
    Value::record(vec![("key", key), ("value", value)])
}

/// The key of a map entry.
pub fn entry_key(entry: &Value) -> &Value {
    match entry {
        Value::Record(r) if r.fields.len() == 2 => &r.fields[0].1,
        other => other,
    }
}

/// The value of a map entry.
pub fn entry_value(entry: &Value) -> &Value {
    match entry {
        Value::Record(r) if r.fields.len() == 2 => &r.fields[1].1,
        other => other,
    }
}

/// The position of the element or key `k` in a set or a map (LP2 draft): a binary search in canonical key order.
pub fn find(collection: &Value, k: &Value) -> Option<usize> {
    match collection {
        Value::Set(s) => s.items.binary_search_by(|e| value_order(e, k)).ok(),
        Value::Map(m) => m.items.binary_search_by(|e| value_order(entry_key(e), k)).ok(),
        _ => None,
    }
}

/// The set with the element `x` (LP2 draft). Of equal elements in different representations, the first in
/// [`key_order`] stays.
pub fn set_insert(set: &List, x: Value) -> Value {
    let mut items = set.items.clone();
    match set.items.binary_search_by(|e| value_order(e, &x)) {
        Ok(i) if key_order(&x, &items[i]) == Ordering::Less => items[i] = x,
        Ok(_) => {}
        Err(i) => items.insert(i, x),
    }
    Value::Set(Arc::new(List::new(items)))
}

/// The map with the value `v` under the key `k` (LP2 draft), which replaces a value under an equal key. Of equal keys
/// in different representations, the first in [`key_order`] stays.
pub fn map_insert(map: &List, k: Value, v: Value) -> Value {
    let mut items = map.items.clone();
    match map.items.binary_search_by(|e| value_order(entry_key(e), &k)) {
        Ok(i) => {
            let old = entry_key(&map.items[i]);
            let k = if key_order(&k, old) == Ordering::Less { k } else { old.clone() };
            items[i] = entry(k, v);
        }
        Err(i) => items.insert(i, entry(k, v)),
    }
    Value::Map(Arc::new(List::new(items)))
}

/// A text with its scalar count, which sizes and output bounds need without a walk (DET.57, DET.62).
#[derive(Debug)]
pub struct Text {
    text: Box<str>,
    scalars: u128,
}

impl Text {
    pub fn new(text: &str) -> Arc<Text> {
        Arc::new(Text { text: text.into(), scalars: text.chars().count() as u128 })
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn scalars(&self) -> u128 {
        self.scalars
    }
}

/// A rational in lowest terms with a positive denominator.
#[derive(Debug, PartialEq, Eq)]
pub struct Ratio {
    pub num: BigInt,
    pub den: BigInt,
}

impl Ratio {
    /// `num / den` in lowest terms; `den` is nonzero.
    pub fn new(num: BigInt, den: BigInt) -> Ratio {
        let (num, den) = if den.is_negative() { (-num, -den) } else { (num, den) };
        let g = num.gcd(&den);
        if g.is_one() {
            Ratio { num, den }
        } else {
            Ratio { num: num / &g, den: den / &g }
        }
    }
}

/// An element (a + b√5)/d of Q(√5) with d > 0 and gcd(a, b, d) = 1.
#[derive(Debug, PartialEq, Eq)]
pub struct Quad {
    pub a: BigInt,
    pub b: BigInt,
    pub d: BigInt,
}

impl Quad {
    /// `(a + b√5) / d` in lowest terms; `d` is nonzero.
    pub fn new(a: BigInt, b: BigInt, d: BigInt) -> Quad {
        let (a, b, d) = if d.is_negative() { (-a, -b, -d) } else { (a, b, d) };
        let g = a.gcd(&b).gcd(&d);
        if g.is_one() {
            Quad { a, b, d }
        } else {
            Quad { a: a / &g, b: b / &g, d: d / &g }
        }
    }
}

/// A record: its fields in IR order, its logical size and an upper bound of its canonical encoding.
#[derive(Debug)]
pub struct Record {
    fields: Vec<(Arc<Text>, Value)>,
    size: u128,
    bound: u128,
}

impl Record {
    pub fn new(fields: Vec<(Arc<Text>, Value)>) -> Record {
        let size = fields
            .iter()
            .fold(4u128, |s, (_, v)| s.saturating_add(v.size()).saturating_add(if v.is_sequence() { 8 } else { 0 }));
        let bound = fields
            .iter()
            .fold(2u128, |s, (k, v)| s.saturating_add(text_bound(k)).saturating_add(v.bound()).saturating_add(2));
        Record { fields, size, bound }
    }

    pub fn fields(&self) -> &[(Arc<Text>, Value)] {
        &self.fields
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.iter().find(|(k, _)| k.as_str() == name).map(|(_, v)| v)
    }

    /// DET.57: a 4-byte variant tag, an 8-byte length per variable sequence and the owned fields.
    pub fn header(fields: &[Value]) -> u128 {
        4 + 8 * fields.iter().filter(|v| v.is_sequence()).count() as u128
    }
}

/// A closure: its code and its captured values in declaration order.
#[derive(Debug)]
pub struct Closure {
    pub code: usize,
    captures: Vec<Value>,
    size: u128,
}

impl Closure {
    pub fn new(code: usize, captures: Vec<Value>) -> Closure {
        let size = captures.iter().fold(32u128, |s, v| s.saturating_add(v.size()));
        Closure { code, captures, size }
    }

    pub fn captures(&self) -> &[Value] {
        &self.captures
    }
}

/// Moves the children of a value out for an explicit-stack drop.
fn take_children(value: Value, out: &mut Vec<Value>) {
    match value {
        Value::Record(r) | Value::Receipt(r) => {
            if let Some(mut r) = Arc::into_inner(r) {
                out.extend(std::mem::take(&mut r.fields).into_iter().map(|(_, v)| v));
            }
        }
        Value::Closure(c) => {
            if let Some(mut c) = Arc::into_inner(c) {
                out.append(&mut c.captures);
            }
        }
        Value::List(l) | Value::Set(l) | Value::Map(l) => {
            if let Some(mut l) = Arc::into_inner(l) {
                out.append(&mut l.items);
            }
        }
        _ => {}
    }
}

impl Drop for Record {
    fn drop(&mut self) {
        let mut stack: Vec<Value> = std::mem::take(&mut self.fields).into_iter().map(|(_, v)| v).collect();
        while let Some(v) = stack.pop() {
            take_children(v, &mut stack);
        }
    }
}

impl Drop for Closure {
    fn drop(&mut self) {
        let mut stack = std::mem::take(&mut self.captures);
        while let Some(v) = stack.pop() {
            take_children(v, &mut stack);
        }
    }
}

fn int_size(n: &BigInt) -> u128 {
    integer_size(n.bits() as u128)
}

/// An upper bound of the decimal digits of `n`: log10(2) < 0.302.
fn digits_bound(n: &BigInt) -> u128 {
    (n.bits() as u128) * 302 / 1000 + 1
}

/// An upper bound of a JSON string's bytes: two quotes and at most six bytes per scalar (FP.5).
fn text_bound(t: &Text) -> u128 {
    t.scalars.saturating_mul(6).saturating_add(2)
}

impl Value {
    pub fn int(n: impl Into<BigInt>) -> Value {
        Value::Int(Arc::new(n.into()))
    }

    pub fn text(s: &str) -> Value {
        Value::Text(Text::new(s))
    }

    pub fn target(s: &str) -> Value {
        Value::Target(Text::new(s))
    }

    pub fn tag(s: &str) -> Value {
        Value::Tag(Text::new(s))
    }

    pub fn colour(s: &str) -> Value {
        Value::Colour(Text::new(s))
    }

    pub fn word(letters: &[i32]) -> Value {
        Value::Word(letters.into())
    }

    /// A menu word of `letters` in the alphabet named by `identity` (LP2 draft); `None` when an index is not below
    /// [`MENU_INDEX_LIMIT`].
    pub fn menu_word(identity: &Arc<MenuIdentity>, letters: &[[u32; 2]]) -> Option<Value> {
        MenuWord::new(identity.clone(), letters).map(|w| Value::MenuWord(Arc::new(w)))
    }

    /// The element of `field` that the integer `n` reduces to: its residue modulo the field's prime, which is in
    /// `0..p` for a negative `n` too (KN3.2; LP2 draft).
    pub fn field(field: Field, n: &BigInt) -> Value {
        let residue = n.mod_floor(&BigInt::from(field.modulus()));
        Value::Field(field, u8::try_from(&residue).expect("a residue below the prime"))
    }

    pub fn record(fields: Vec<(&str, Value)>) -> Value {
        Value::Record(Arc::new(Record::new(fields.into_iter().map(|(k, v)| (Text::new(k), v)).collect())))
    }

    /// A list of these elements in order (LP2 draft).
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Arc::new(List::new(items)))
    }

    /// A set of these elements (LP2 draft): each kept once, in canonical key order. Of equal elements in different
    /// representations, the first in [`key_order`] stays, so the set does not depend on the written order.
    pub fn set(mut items: Vec<Value>) -> Value {
        items.sort_by(key_order);
        items.dedup_by(|later, earlier| value_order(earlier, later) == Ordering::Equal);
        Value::Set(Arc::new(List::new(items)))
    }

    /// A map of these entries in written order (LP2 draft), kept in the canonical key order of their keys. A later
    /// entry's value replaces an earlier one's under an equal key; of equal keys in different representations, the
    /// first in [`key_order`] stays.
    pub fn map(mut entries: Vec<(Value, Value)>) -> Value {
        // A stable sort keeps the written order among equal keys, so the last of each run is the latest entry.
        entries.sort_by(|a, b| value_order(&a.0, &b.0));
        let mut kept: Vec<(Value, Value)> = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            match kept.last_mut() {
                Some(last) if value_order(&last.0, &k) == Ordering::Equal => {
                    if key_order(&k, &last.0) == Ordering::Less {
                        last.0 = k;
                    }
                    last.1 = v;
                }
                _ => kept.push((k, v)),
            }
        }
        Value::Map(Arc::new(List::new(kept.into_iter().map(|(k, v)| entry(k, v)).collect())))
    }

    pub fn receipt(fields: Vec<(&str, Value)>) -> Value {
        Value::Receipt(Arc::new(Record::new(fields.into_iter().map(|(k, v)| (Text::new(k), v)).collect())))
    }

    /// DET.57's logical size in bytes.
    pub fn size(&self) -> u128 {
        match self {
            Value::Unit => 0,
            Value::Bool(_) | Value::ApproxBool(_) => 1,
            Value::Int(n) => int_size(n),
            Value::Rat(r) => int_size(&r.num) + int_size(&r.den),
            Value::Q5(q) => int_size(&q.a) + int_size(&q.b) + int_size(&q.d),
            Value::Approx(x) => approx::width(x.precision),
            Value::Text(t) | Value::Target(t) => t.scalars.saturating_mul(4),
            Value::Tag(_) | Value::Colour(_) => 4,
            Value::Word(w) | Value::WordEqualityKey(w) => 2 * w.len() as u128,
            // Its letters, as a word's; the header is shared with the run's relation.
            Value::TraceEqualityKey(k) => 2 * k.letters().len() as u128,
            Value::Record(r) | Value::Receipt(r) => r.size,
            Value::Closure(c) => c.size,
            Value::Fn(_) => 32,
            Value::List(l) | Value::Set(l) | Value::Map(l) => l.size,
            Value::Compressed(c) => c.size(),
            Value::MenuWord(w) => 4 * w.letters.len() as u128,
            // One byte for the test and one for its outcome.
            Value::EndTest(..) => 2,
            // The 32-byte key, the 8-byte counter and the exhausted flag (DET.61).
            Value::Generator(_) => GENERATOR_SIZE,
            Value::Field(..) => 1,
        }
    }

    /// DET.61's alias of a value: a generator's cursor copied into an independent one, which draws the same stream on
    /// its own. Any other value is shared as it is.
    pub fn alias(&self) -> Value {
        match self {
            Value::Generator(g) => Value::Generator(Arc::new(Generator::new(g.state()))),
            other => other.clone(),
        }
    }

    /// Whether the value is a variable sequence, which a record field stores with an 8-byte length (DET.57).
    pub fn is_sequence(&self) -> bool {
        matches!(
            self,
            Value::Text(_)
                | Value::Target(_)
                | Value::Word(_)
                | Value::WordEqualityKey(_)
                | Value::List(_)
                | Value::Set(_)
                | Value::Map(_)
                | Value::Compressed(_)
                | Value::MenuWord(_)
                | Value::TraceEqualityKey(_)
        )
    }

    /// Whether the value is or holds an end test's verdict in a record field or a collection's element, key or value,
    /// at any depth (LB3.2; LP2 draft). A verdict is matched on its outcomes only (LG2.9); the run refuses an equality
    /// that the checker could not see, behind a type variable.
    pub fn holds_end_test(&self) -> bool {
        match self {
            Value::EndTest(..) => true,
            Value::Record(r) => r.fields().iter().any(|(_, v)| v.holds_end_test()),
            Value::List(l) | Value::Set(l) | Value::Map(l) => l.items().iter().any(Value::holds_end_test),
            _ => false,
        }
    }

    /// An upper bound of the value's canonical encoding in bytes, without the terminal line feed (FP.5, FP.6). It is
    /// retained metadata, so the encoding pass is priced before it runs (DET.59). A value with no canonical form has
    /// bound 0; its encoding fails.
    pub fn bound(&self) -> u128 {
        match self {
            Value::Unit => 4,
            Value::Bool(_) | Value::ApproxBool(_) => 5,
            Value::Int(n) => 11 + digits_bound(n),
            Value::Rat(r) => 16 + digits_bound(&r.num) + digits_bound(&r.den),
            Value::Q5(q) => 19 + digits_bound(&q.a) + digits_bound(&q.b) + digits_bound(&q.d),
            Value::Approx(_) => 26,
            Value::Text(t) | Value::Target(t) | Value::Tag(t) | Value::Colour(t) => text_bound(t),
            Value::Word(w) => 2 + 6 * w.len() as u128,
            Value::MenuWord(w) => w.bound(),
            // `{"outcome":"...","test":"..."}`: 24 bytes of keys and punctuation, and the two names.
            Value::EndTest(test, outcome) => 24 + (test.row().len() + outcome.tag().len()) as u128,
            Value::Record(r) => r.bound,
            Value::List(l) | Value::Set(l) | Value::Map(l) => l.bound,
            // A word key or a trace key has no canonical form in the LP2 draft: it is never output as letters (LG2.13,
            // LB1.10). Nor has a compressed word, whose output would be its expansion (DET.59), a generator, which is
            // a stream's cursor (DET.61), or an F2 or F5 element, which FP.6 does not wrap.
            Value::Closure(_)
            | Value::Fn(_)
            | Value::Receipt(_)
            | Value::WordEqualityKey(_)
            | Value::TraceEqualityKey(_)
            | Value::Compressed(_)
            | Value::Generator(_)
            | Value::Field(..) => 0,
        }
    }
}

/// An exact number viewed for widening along Int, Rat and Q5.
enum Num<'a> {
    Int(&'a BigInt),
    Rat(&'a Ratio),
    Q5(&'a Quad),
}

fn num(v: &Value) -> Option<Num<'_>> {
    match v {
        Value::Int(n) => Some(Num::Int(n)),
        Value::Rat(r) => Some(Num::Rat(r)),
        Value::Q5(q) => Some(Num::Q5(q)),
        _ => None,
    }
}

impl Num<'_> {
    fn rank(&self) -> u8 {
        match self {
            Num::Int(_) => 0,
            Num::Rat(_) => 1,
            Num::Q5(_) => 2,
        }
    }

    /// The components (a, b, d) of the number as an element of Q(√5).
    fn quad(&self) -> (BigInt, BigInt, BigInt) {
        match self {
            Num::Int(n) => ((*n).clone(), BigInt::zero(), BigInt::one()),
            Num::Rat(r) => (r.num.clone(), BigInt::zero(), r.den.clone()),
            Num::Q5(q) => (q.a.clone(), q.b.clone(), q.d.clone()),
        }
    }

    /// The size of the number widened to the given rank: a widened denominator is 1 and a widened √5 coefficient 0,
    /// each two bytes (DET.57).
    fn widened_size(&self, rank: u8) -> u128 {
        let (a, b, d) = self.quad();
        let mut size = int_size(&a);
        if rank >= 1 {
            size += int_size(&d);
        }
        if rank >= 2 {
            size += int_size(&b);
        }
        size
    }
}

fn exact_pair<'a>(a: &'a Value, b: &'a Value) -> Option<(Num<'a>, Num<'a>, u8)> {
    let (x, y) = (num(a)?, num(b)?);
    let rank = x.rank().max(y.rank());
    Some((x, y, rank))
}

/// The sign of a + b√5 for integers a and b.
pub fn sign_q5(a: &BigInt, b: &BigInt) -> Ordering {
    match (a.sign(), b.sign()) {
        (Sign::NoSign, Sign::NoSign) => Ordering::Equal,
        (Sign::Minus, Sign::Minus | Sign::NoSign) | (Sign::NoSign, Sign::Minus) => Ordering::Less,
        (Sign::Plus, Sign::Plus | Sign::NoSign) | (Sign::NoSign, Sign::Plus) => Ordering::Greater,
        (Sign::Plus, Sign::Minus) => (a * a).cmp(&(b * b * 5u32)),
        (Sign::Minus, Sign::Plus) => (b * b * 5u32).cmp(&(a * a)),
    }
}

/// Exact addition with widening along Int, Rat and Q5; `None` if an operand is not an exact number.
pub fn add(a: &Value, b: &Value) -> Option<Value> {
    let (x, y, rank) = exact_pair(a, b)?;
    Some(match (rank, &x, &y) {
        (0, Num::Int(m), Num::Int(n)) => Value::int(*m + *n),
        _ => {
            let ((xa, xb, xd), (ya, yb, yd)) = (x.quad(), y.quad());
            if rank == 1 {
                Value::Rat(Arc::new(Ratio::new(&xa * &yd + &ya * &xd, xd * yd)))
            } else {
                Value::Q5(Arc::new(Quad::new(&xa * &yd + &ya * &xd, &xb * &yd + &yb * &xd, xd * yd)))
            }
        }
    })
}

/// The kernel cost and the result reservation of an exact addition. Integers use the vectors' integer row; a rational
/// or Q(√5) sum costs `field_per_unit_pair` per pair of widened operand bytes and reserves the unreduced result.
pub fn add_price(a: &Value, b: &Value, t: &Tariffs, d: &DraftTariffs) -> Option<(u128, u128)> {
    let (x, y, rank) = exact_pair(a, b)?;
    if let (Num::Int(m), Num::Int(n)) = (&x, &y) {
        return Some(t.integer(IntOp::Add, m.bits() as u128, n.bits() as u128));
    }
    let cost = d.field_per_unit_pair.saturating_mul(x.widened_size(rank)).saturating_mul(y.widened_size(rank));
    let ((xa, xb, xd), (ya, yb, yd)) = (x.quad(), y.quad());
    let bits = |n: &BigInt| n.bits() as u128;
    let sum = |p: &BigInt, q: &BigInt| integer_size((bits(p) + bits(&yd)).max(bits(q) + bits(&xd)) + 1);
    let mut reservation = sum(&xa, &ya) + integer_size(bits(&xd) + bits(&yd));
    if rank == 2 {
        reservation += sum(&xb, &yb);
    }
    Some((cost, reservation))
}

/// Exact multiplication with widening along Int, Rat and Q5 (LP2 draft); `None` if an operand is not an exact number.
pub fn mul(a: &Value, b: &Value) -> Option<Value> {
    let (x, y, rank) = exact_pair(a, b)?;
    Some(match (rank, &x, &y) {
        (0, Num::Int(m), Num::Int(n)) => Value::int(*m * *n),
        _ => {
            let ((xa, xb, xd), (ya, yb, yd)) = (x.quad(), y.quad());
            if rank == 1 {
                Value::Rat(Arc::new(Ratio::new(&xa * &ya, xd * yd)))
            } else {
                Value::Q5(Arc::new(Quad::new(&xa * &ya + &xb * &yb * 5u32, &xa * &yb + &xb * &ya, xd * yd)))
            }
        }
    })
}

/// The kernel cost and the result reservation of an exact multiplication (LP2 draft). Integers use the vectors'
/// integer row (DET.57); a rational or Q(√5) product costs `field_per_unit_pair` per pair of widened operand bytes, as
/// a sum does, and reserves the unreduced result.
pub fn mul_price(a: &Value, b: &Value, t: &Tariffs, d: &DraftTariffs) -> Option<(u128, u128)> {
    let (x, y, rank) = exact_pair(a, b)?;
    if let (Num::Int(m), Num::Int(n)) = (&x, &y) {
        return Some(t.integer(IntOp::Mul, m.bits() as u128, n.bits() as u128));
    }
    let cost = d.field_per_unit_pair.saturating_mul(x.widened_size(rank)).saturating_mul(y.widened_size(rank));
    let ((xa, xb, xd), (ya, yb, yd)) = (x.quad(), y.quad());
    let bits = |n: &BigInt| n.bits() as u128;
    let mut reservation = integer_size(bits(&xd) + bits(&yd));
    if rank == 1 {
        reservation += integer_size(bits(&xa) + bits(&ya));
    } else {
        // a·a′ + 5·b·b′ and a·b′ + b·a′: the factor 5 adds at most three bits, and each sum one.
        reservation += integer_size((bits(&xa) + bits(&ya)).max(bits(&xb) + bits(&yb) + 3) + 1);
        reservation += integer_size((bits(&xa) + bits(&yb)).max(bits(&xb) + bits(&ya)) + 1);
    }
    Some((cost, reservation))
}

/// The exact order of two numbers; `None` if an operand is not an exact number.
pub fn compare(a: &Value, b: &Value) -> Option<Ordering> {
    let (x, y, rank) = exact_pair(a, b)?;
    Some(match (rank, &x, &y) {
        (0, Num::Int(m), Num::Int(n)) => m.cmp(n),
        _ => {
            let ((xa, xb, xd), (ya, yb, yd)) = (x.quad(), y.quad());
            sign_q5(&(&xa * &yd - &ya * &xd), &(&xb * &yd - &yb * &xd))
        }
    })
}

/// The kernel cost of an exact order: a walk over both integers, or a product of widened sizes for fields.
pub fn compare_price(a: &Value, b: &Value, d: &DraftTariffs) -> Option<u128> {
    let (x, y, rank) = exact_pair(a, b)?;
    Some(if rank == 0 {
        d.compare_per_unit.saturating_mul(a.size().saturating_add(b.size()))
    } else {
        d.field_per_unit_pair.saturating_mul(x.widened_size(rank)).saturating_mul(y.widened_size(rank))
    })
}

/// Exact equality. Numbers compare across Int, Rat and Q5; records, receipts and closures compare structurally, with an
/// explicit stack. Compressed words compare by their parts, which is no word equality: the checker gives them no exact
/// equality (LP2 draft), and the interpreter refuses one of two compressed words.
pub fn equal(a: &Value, b: &Value) -> bool {
    let mut stack = vec![(a, b)];
    while let Some((a, b)) = stack.pop() {
        let same = match (a, b) {
            (Value::Record(x), Value::Record(y)) | (Value::Receipt(x), Value::Receipt(y)) => {
                x.fields.len() == y.fields.len()
                    && x.fields.iter().zip(&y.fields).all(|((kx, vx), (ky, vy))| {
                        stack.push((vx, vy));
                        kx.as_str() == ky.as_str()
                    })
            }
            (Value::Closure(x), Value::Closure(y)) => {
                stack.extend(x.captures.iter().zip(&y.captures));
                x.code == y.code && x.captures.len() == y.captures.len()
            }
            (Value::Unit, Value::Unit) => true,
            (Value::Bool(x), Value::Bool(y)) | (Value::ApproxBool(x), Value::ApproxBool(y)) => x == y,
            (Value::Text(x), Value::Text(y))
            | (Value::Target(x), Value::Target(y))
            | (Value::Tag(x), Value::Tag(y))
            | (Value::Colour(x), Value::Colour(y)) => x.as_str() == y.as_str(),
            (Value::Word(x), Value::Word(y)) | (Value::WordEqualityKey(x), Value::WordEqualityKey(y)) => x == y,
            // Matching headers and token sequences, not only matching hashes (LG2.23).
            (Value::TraceEqualityKey(x), Value::TraceEqualityKey(y)) => x == y,
            (Value::List(x), Value::List(y)) | (Value::Set(x), Value::Set(y)) | (Value::Map(x), Value::Map(y)) => {
                stack.extend(x.items.iter().zip(&y.items));
                x.items.len() == y.items.len()
            }
            (Value::Fn(x), Value::Fn(y)) => x == y,
            (Value::Compressed(x), Value::Compressed(y)) => x.parts == y.parts,
            (Value::MenuWord(x), Value::MenuWord(y)) => x == y,
            (Value::EndTest(tx, ox), Value::EndTest(ty, oy)) => tx == ty && ox == oy,
            (Value::Approx(x), Value::Approx(y)) => x == y,
            // Each cursor read on its own; the checker gives a generator no equality (LP2 draft).
            (Value::Generator(x), Value::Generator(y)) => x.state() == y.state(),
            (Value::Field(f, x), Value::Field(g, y)) => f == g && x == y,
            _ => compare(a, b) == Some(Ordering::Equal),
        };
        if !same {
            return false;
        }
    }
    true
}

/// The kernel cost of an exact equality: DET.62's text tariff on both scalar lengths, nothing for fixed-size values,
/// and a walk over both values otherwise.
pub fn equal_price(a: &Value, b: &Value, t: &Tariffs, d: &DraftTariffs) -> u128 {
    match (a, b) {
        (Value::Text(x), Value::Text(y)) | (Value::Target(x), Value::Target(y)) => {
            t.text_per_scalar.saturating_mul(x.scalars.saturating_add(y.scalars))
        }
        (
            Value::Unit
            | Value::Bool(_)
            | Value::ApproxBool(_)
            | Value::Tag(_)
            | Value::Colour(_)
            | Value::Fn(_)
            | Value::EndTest(..)
            | Value::Field(..),
            Value::Unit
            | Value::Bool(_)
            | Value::ApproxBool(_)
            | Value::Tag(_)
            | Value::Colour(_)
            | Value::Fn(_)
            | Value::EndTest(..)
            | Value::Field(..),
        ) => 0,
        _ => d.compare_per_unit.saturating_mul(a.size().saturating_add(b.size())),
    }
}

/// The kernel cost of an explicit conversion to `Approx<p>`: `convert_per_unit_pair` per pair of bytes of the operand
/// and the result together.
pub fn convert_price(v: &Value, precision: u32, d: &DraftTariffs) -> u128 {
    let n = v.size().saturating_add(approx::width(precision));
    d.convert_per_unit_pair.saturating_mul(n.saturating_mul(n))
}

/// Converts an exact number to `Approx<p>`, rounding to nearest with ties to even.
pub fn to_approx(v: &Value, precision: u32) -> Option<Approx> {
    let (a, b, d) = num(v)?.quad();
    Some(Approx::from_q5(precision, &a, &b, &d))
}

/// Parses an exact integer in the IR's decimal form: `0`, or an optional `-` and digits without a leading zero.
pub fn parse_integer(s: &str) -> Option<BigInt> {
    let digits = s.strip_prefix('-').unwrap_or(s);
    let canonical = if digits == "0" { s == "0" } else { !digits.starts_with('0') };
    let ok =
        canonical && !digits.is_empty() && digits.len() <= MAX_DIGITS && digits.bytes().all(|c| c.is_ascii_digit());
    ok.then(|| s.parse::<BigInt>().ok()).flatten()
}

/// Parses a ratio literal: the sign on the numerator and a positive denominator (LG1.6).
pub fn parse_ratio(num: &str, den: &str) -> Option<(BigInt, BigInt)> {
    let (n, d) = (parse_integer(num)?, parse_integer(den)?);
    d.is_positive().then_some((n, d))
}

/// An exact literal of the IR as a value. `None` for a literal the IR draft does not allow.
pub fn literal(lit: &c600_ir::Lit) -> Option<Value> {
    use c600_ir::Lit;
    Some(match lit {
        Lit::Unit => Value::Unit,
        Lit::Bool(b) => Value::Bool(*b),
        Lit::Nat(n) => {
            let n = parse_integer(n)?;
            if n.is_negative() {
                return None;
            }
            Value::int(n)
        }
        Lit::Int(n) => Value::int(parse_integer(n)?),
        Lit::Rat { num, den } => {
            let (n, d) = parse_ratio(num, den)?;
            Value::Rat(Arc::new(Ratio::new(n, d)))
        }
        Lit::Q5 { one, sqrt5 } => {
            let ((n1, d1), (n2, d2)) = (parse_ratio(&one.0, &one.1)?, parse_ratio(&sqrt5.0, &sqrt5.1)?);
            let l = d1.lcm(&d2);
            Value::Q5(Arc::new(Quad::new(n1 * (&l / &d1), n2 * (&l / &d2), l)))
        }
        Lit::Approx { decimal, precision } => {
            if !approx::PRECISIONS.contains(precision) {
                return None;
            }
            Value::Approx(Approx::parse(*precision, decimal)?)
        }
        Lit::Text(s) => Value::text(s),
        Lit::Word(letters) => {
            if letters.iter().any(|l| *l == 0 || l.unsigned_abs() > 1200) {
                return None;
            }
            Value::word(letters)
        }
        Lit::Target(s) => Value::target(s),
        Lit::Tag(s) => Value::tag(s),
        Lit::Colour(s) => Value::colour(s),
        // A menu-word literal names no alphabet until a run reads it with its pinned one (LP2 draft).
        Lit::MenuWord(letters) => Value::menu_word(&Arc::default(), letters)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The meter's regression for a doubling chain (LP2 draft): a join keeps the handles of its operands' stored
    /// lengths and computes only its new part's, so its work is one step per part, as `concat` is priced, and not one
    /// per bit kept. A join of two different words shares the lengths of both.
    #[test]
    fn a_join_shares_its_operands_stored_lengths() {
        let shared = |a: &Compressed, b: &Compressed| a.lengths.iter().zip(&b.lengths).all(|(x, y)| Arc::ptr_eq(x, y));
        let mut c = Arc::new(Compressed::leaf([1].into()));
        for _ in 0..64 {
            let next = Compressed::concat(&c, &c);
            assert_eq!(next.lengths.len(), c.lengths.len() + 1);
            assert!(shared(&c, &next));
            c = next;
        }
        let u = Arc::new(Compressed::leaf([2, -3].into()));
        let joined = Compressed::concat(&u, &c);
        assert_eq!(joined.lengths.len(), u.lengths.len() + c.lengths.len() + 1);
        assert!(shared(&u, &joined));
        assert!(c.lengths.iter().zip(&joined.lengths[u.lengths.len()..]).all(|(x, y)| Arc::ptr_eq(x, y)));
        assert_eq!(**joined.lengths.last().unwrap(), (BigInt::from(1) << 64) + 2);
    }
}
