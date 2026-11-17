//! LB1.10's trace key `trace-key/1` of 600-cell-Full words under the independence contract `full-disjoint-support/1`
//! (LP2 draft; the owner chose the key on 17 November 2026, and it freezes with the IR, PHS.49).
//!
//! A trace key ([`TraceKey`]) is the lexicographically least topological word of a literal word. For occurrences
//! i < j whose letters are dependent there is an edge i -> j, and the scan repeatedly emits the least available letter
//! in LG2.23's signed-generator order ([`letter_order`]), breaking ties between equal letters by their position. It
//! keeps every occurrence: it cancels nothing, merges nothing and does not start from the word key. Two words have one
//! key exactly when adjacent swaps of independent letters relate them, which is sound but incomplete for equality of
//! puzzle actions. A key has its own type: it is never a word or a word key, and it never executes.
//!
//! Under `full-disjoint-support/1` two distinct generators are independent exactly when their complete labelled-slot
//! supports in the pinned model are disjoint, whatever the signs, and a generator depends on itself. A host builds the
//! [`Relation`] from the 1,200 supports the kernel computes, so a caller cannot declare more independence. `c600-run`
//! does not depend on the kernel crates, so the identity block's fields are text here, as in [`crate::menu`].
//!
//! A key carries its header ([`TraceHeader`], LG2.23): KN1's identity block, the alphabet, the key kind and version,
//! the independence contract and the relation digest. A run pins at most one relation
//! ([`crate::interp::Options::relation`]), and every trace key in the run is a key under it: `trace_key` stops a run
//! that pins none, and a host's answer that holds a key under another header, or letters that are not the key of their
//! own trace, stops the run. A key has no canonical form, so no argument or result holds one. Two keys that a run
//! compares therefore have one header. Each of these stops has the code [`KEY_CONTRACT_MISMATCH`].
//!
//! The jumbling half, `jumble-locality/1` over menu letters, waits for an exact Δ(c,d) entry in `c600-jumble`.

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::value::Value;

/// The chapter 13 code of a trace key under another header than the run's relation, or of a run that pins no
/// relation where a trace key is needed (LG2.23, LB1.10). No equality or order result is returned.
pub const KEY_CONTRACT_MISMATCH: &str = "LB1:input.key_contract_mismatch";

/// LB1.10's key kind and rewrite version.
pub const TRACE_KEY: &str = "trace-key/1";

/// LB1.10's independence contract of 600-cell-Full.
pub const FULL_DISJOINT_SUPPORT: &str = "full-disjoint-support/1";

/// The generators of 600-cell-Full: its letters have magnitude 1 to 1200.
pub const FULL_GENERATORS: u32 = 1200;

/// The bound of a labelled-slot index in a support (LP2 draft): the pinned model has 259,800 slots, and the bound keeps
/// the two slot tables of [`Relation::full_disjoint_support`]'s counting sort at 8 MiB each.
pub const SLOT_LIMIT: i32 = 1 << 20;

/// A letter's place in LG2.23's signed-generator order: `2|g| - 1` for an inverse letter, `2|g|` for a forward one.
fn rank(letter: i32) -> u64 {
    2 * u64::from(letter.unsigned_abs()) - u64::from(letter < 0)
}

/// LG2.23's signed-generator order of Full letters: by absolute generator ID, then by sign with the inverse first, so
/// -1 < 1 < -2 < 2.
pub fn letter_order(a: i32, b: i32) -> Ordering {
    rank(a).cmp(&rank(b))
}

/// Two token sequences in LG2.23's order: lexicographically in [`letter_order`], with a proper prefix first.
pub fn tokens_order(a: &[i32], b: &[i32]) -> Ordering {
    a.iter().map(|&l| rank(l)).cmp(b.iter().map(|&l| rank(l)))
}

/// A trace key's header (LG2.23): KN1's identity block, the alphabet, the key kind with its rewrite version, and
/// LB1.10's independence contract with its relation digest. Keys compare only under one header.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraceHeader {
    /// KN1's identity block: the puzzle.
    pub puzzle: String,
    /// KN1's identity block: the `model_id` of the model's manifest.
    pub model_id: String,
    /// The alphabet: the signed generators of magnitude 1 to `generators`, in [`letter_order`].
    pub generators: u32,
    /// The key kind and its rewrite version, [`TRACE_KEY`].
    pub key: String,
    /// The independence contract, [`FULL_DISJOINT_SUPPORT`].
    pub contract: String,
    /// The relation digest ([`Relation::digest`]).
    pub digest: [u8; 32],
}

impl TraceHeader {
    /// The first field in which two headers differ, in LG2.23's order: `puzzle`, `model_id`, `generators`, `key`,
    /// `contract`, then `digest`. `None` when they are one header.
    pub fn differing(&self, other: &TraceHeader) -> Option<&'static str> {
        [
            ("puzzle", self.puzzle == other.puzzle),
            ("model_id", self.model_id == other.model_id),
            ("generators", self.generators == other.generators),
            ("key", self.key == other.key),
            ("contract", self.contract == other.contract),
            ("digest", self.digest == other.digest),
        ]
        .into_iter()
        .find(|(_, same)| !same)
        .map(|(field, _)| field)
    }
}

/// A trace key (LB1.10; LP2 draft): the letters of a Full word's trace in their least topological order, under the
/// header of the relation that ordered them. The header is the run's pinned metadata, shared by its keys, and not
/// payload: a key's logical size is that of its letters, 2 bytes each (DET.57). Keys compare by their headers, then by
/// their letters in LG2.23's order ([`tokens_order`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceKey {
    header: Arc<TraceHeader>,
    letters: Arc<[i32]>,
}

impl TraceKey {
    /// A key as a host or the wire hands it over: `letters` under `header`, unchecked. A run checks it against its
    /// relation when it reads it ([`check_keys`]).
    pub fn from_parts(header: Arc<TraceHeader>, letters: &[i32]) -> TraceKey {
        TraceKey { header, letters: letters.into() }
    }

    /// The header of the relation that ordered the letters.
    pub fn header(&self) -> &TraceHeader {
        &self.header
    }

    /// The header as the keys of one relation share it.
    pub fn shared_header(&self) -> &Arc<TraceHeader> {
        &self.header
    }

    /// The letters, in their least topological order.
    pub fn letters(&self) -> &[i32] {
        &self.letters
    }
}

impl Ord for TraceKey {
    fn cmp(&self, other: &TraceKey) -> Ordering {
        self.header.cmp(&other.header).then_with(|| tokens_order(&self.letters, &other.letters))
    }
}

impl PartialOrd for TraceKey {
    fn partial_cmp(&self, other: &TraceKey) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Why a trace relation cannot be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelationError {
    /// A field of the identity block is empty, so the relation names no model.
    Identity(&'static str),
    /// The independence contract is not [`FULL_DISJOINT_SUPPORT`], the only one this draft builds.
    Contract(String),
    /// Not 1,200 generators, one per generator of 600-cell-Full.
    Generators(u64),
    /// A generator moves no slot: its support is empty, which no generator of the pinned model's is.
    EmptySupport(u32),
    /// A support holds a slot index outside 0 to [`SLOT_LIMIT`].
    Slot { generator: u32, slot: i32 },
    /// The independence bits are not one bit per pair of distinct generators in whole bytes, or a padding bit is set.
    Bits,
}

impl fmt::Display for RelationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationError::Identity(field) => write!(f, "the identity block's {field} is empty: it names no model"),
            RelationError::Contract(contract) => {
                write!(f, "the independence contract {contract:?} is not {FULL_DISJOINT_SUPPORT}")
            }
            RelationError::Generators(n) => write!(f, "{n} generators, not the {FULL_GENERATORS} of 600-cell-Full"),
            RelationError::EmptySupport(g) => write!(f, "generator {g} moves no slot"),
            RelationError::Slot { generator, slot } => {
                write!(f, "generator {generator}'s support holds slot {slot}, outside 0 to {SLOT_LIMIT}")
            }
            RelationError::Bits => write!(f, "the independence bits are not one bit per pair of distinct generators"),
        }
    }
}

impl std::error::Error for RelationError {}

/// LB1.10's independence relation `full-disjoint-support/1` of one pinned 600-cell-Full model (LP2 draft), with the
/// header its keys carry. Two relations with one header are one relation: the digest covers the identity block, the
/// contract, the alphabet and every pair.
#[derive(Clone)]
pub struct Relation(Arc<Inner>);

struct Inner {
    header: Arc<TraceHeader>,
    /// One bit per pair (c, d) of generators with c < d, in ascending order of c and then of d, least significant bit
    /// first in each byte, set when the pair is independent. The last byte's unused bits are clear.
    bits: Vec<u8>,
    /// For each generator g, at g - 1, the other generators it depends on, counted from 0 and ascending.
    dependents: Vec<Vec<u16>>,
}

impl PartialEq for Relation {
    fn eq(&self, other: &Relation) -> bool {
        self.0.header == other.0.header && self.0.bits == other.0.bits
    }
}

impl Eq for Relation {}

impl fmt::Debug for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Relation").field("header", &self.0.header).finish_non_exhaustive()
    }
}

/// The bytes of one bit per pair of distinct generators among `n`.
fn pair_bytes(n: usize) -> usize {
    (n * n.saturating_sub(1) / 2).div_ceil(8)
}

/// The identity block's fields, refused when one is empty.
fn identity(puzzle: &str, model_id: &str) -> Result<(), RelationError> {
    match (puzzle.is_empty(), model_id.is_empty()) {
        (true, _) => Err(RelationError::Identity("puzzle")),
        (_, true) => Err(RelationError::Identity("model_id")),
        _ => Ok(()),
    }
}

/// The relation digest (LP2 draft layout): SHA-256 of the contract, the puzzle and the model ID, each as its UTF-8
/// length in 4 big-endian bytes and its bytes, then the generator count in 4 big-endian bytes, then the independence
/// bits as [`Relation::bits`] gives them.
fn digest(contract: &str, puzzle: &str, model_id: &str, generators: u32, bits: &[u8]) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(bits.len() + 64);
    for text in [contract, puzzle, model_id] {
        bytes.extend((text.len() as u32).to_be_bytes());
        bytes.extend(text.as_bytes());
    }
    bytes.extend(generators.to_be_bytes());
    bytes.extend(bits);
    c600_digest::sha256(&bytes)
}

impl Relation {
    /// The relation of the pinned model whose generator g moves the labelled slots `supports[g - 1]`, its complete
    /// support from the kernel, under KN1's identity block `puzzle` and `model_id`: two distinct generators are
    /// independent exactly when their supports share no slot. Refused unless there is one nonempty support per
    /// generator of 600-cell-Full, every slot is in 0 to [`SLOT_LIMIT`], and both identity fields are nonempty.
    ///
    /// Each slot lists the generators that move it, and each distinct list makes its generators pairwise dependent, so
    /// the work is linear in the supports and quadratic only in the distinct lists.
    pub fn full_disjoint_support(
        puzzle: &str,
        model_id: &str,
        supports: &[Vec<i32>],
    ) -> Result<Relation, RelationError> {
        identity(puzzle, model_id)?;
        let n = supports.len();
        if n != FULL_GENERATORS as usize {
            return Err(RelationError::Generators(n as u64));
        }
        let mut top = 0usize;
        for (g, slots) in supports.iter().enumerate() {
            let generator = g as u32 + 1;
            if slots.is_empty() {
                return Err(RelationError::EmptySupport(generator));
            }
            if let Some(&slot) = slots.iter().find(|s| !(0..SLOT_LIMIT).contains(*s)) {
                return Err(RelationError::Slot { generator, slot });
            }
            top = top.max(slots.iter().copied().max().unwrap_or(0) as usize);
        }
        // The generators that move each slot, ascending: a counting sort by slot, filled in generator order.
        let mut start = vec![0usize; top + 2];
        for &s in supports.iter().flatten() {
            start[s as usize + 1] += 1;
        }
        for s in 0..=top {
            start[s + 1] += start[s];
        }
        let mut fill = start.clone();
        let mut movers = vec![0u16; start[top + 1]];
        for (g, slots) in supports.iter().enumerate() {
            for &s in slots {
                movers[fill[s as usize]] = g as u16;
                fill[s as usize] += 1;
            }
        }
        let mut dependent = vec![false; n * n];
        let mut seen: HashSet<&[u16]> = HashSet::new();
        for s in 0..=top {
            let list = &movers[start[s]..start[s + 1]];
            if list.len() < 2 || !seen.insert(list) {
                continue;
            }
            for (i, &c) in list.iter().enumerate() {
                for &d in &list[i + 1..] {
                    // A support that names a slot twice lists its generator twice.
                    if c != d {
                        dependent[usize::from(c) * n + usize::from(d)] = true;
                    }
                }
            }
        }
        let mut bits = vec![0u8; pair_bytes(n)];
        let mut k = 0usize;
        for c in 0..n {
            for d in c + 1..n {
                if !dependent[c * n + d] {
                    bits[k / 8] |= 1 << (k % 8);
                }
                k += 1;
            }
        }
        Ok(Relation::build(puzzle, model_id, n, bits))
    }

    /// The relation the wire carries (C3; LP2 draft): its contract, identity block, generator count and independence
    /// bits as [`Relation::bits`] gives them. The digest is recomputed here, never taken from the sender. Refused for
    /// another contract than [`FULL_DISJOINT_SUPPORT`], another generator count than 1,200, an empty identity field,
    /// or bits of another length or with a padding bit set.
    pub fn from_parts(
        contract: &str,
        puzzle: &str,
        model_id: &str,
        generators: u32,
        bits: Vec<u8>,
    ) -> Result<Relation, RelationError> {
        if contract != FULL_DISJOINT_SUPPORT {
            return Err(RelationError::Contract(contract.to_string()));
        }
        identity(puzzle, model_id)?;
        if generators != FULL_GENERATORS {
            return Err(RelationError::Generators(u64::from(generators)));
        }
        let n = generators as usize;
        let pairs = n * (n - 1) / 2;
        if bits.len() != pair_bytes(n) || (pairs % 8 != 0 && bits[bits.len() - 1] >> (pairs % 8) != 0) {
            return Err(RelationError::Bits);
        }
        Ok(Relation::build(puzzle, model_id, n, bits))
    }

    fn build(puzzle: &str, model_id: &str, n: usize, bits: Vec<u8>) -> Relation {
        let mut dependents = vec![Vec::new(); n];
        let mut k = 0usize;
        for c in 0..n {
            for d in c + 1..n {
                if bits[k / 8] >> (k % 8) & 1 == 0 {
                    // Row c pushes its larger dependents in order, after every smaller one has pushed c.
                    dependents[c].push(d as u16);
                    dependents[d].push(c as u16);
                }
                k += 1;
            }
        }
        let generators = n as u32;
        let header = TraceHeader {
            puzzle: puzzle.to_string(),
            model_id: model_id.to_string(),
            generators,
            key: TRACE_KEY.to_string(),
            contract: FULL_DISJOINT_SUPPORT.to_string(),
            digest: digest(FULL_DISJOINT_SUPPORT, puzzle, model_id, generators, &bits),
        };
        Relation(Arc::new(Inner { header: Arc::new(header), bits, dependents }))
    }

    /// The header the relation's keys carry.
    pub fn header(&self) -> &TraceHeader {
        &self.0.header
    }

    /// The relation digest.
    pub fn digest(&self) -> &[u8; 32] {
        &self.0.header.digest
    }

    /// The independence bits: one per pair (c, d) of generators with c < d, in ascending order of c and then of d,
    /// least significant bit first in each byte, set when the pair is independent.
    pub fn bits(&self) -> &[u8] {
        &self.0.bits
    }

    /// Whether `letter` is of the relation's alphabet: its magnitude is 1 to the generator count.
    pub fn holds(&self, letter: i32) -> bool {
        (1..=self.0.header.generators).contains(&letter.unsigned_abs())
    }

    /// Whether two letters are independent: their generators are distinct and their supports disjoint. Letters
    /// outside the alphabet are independent of nothing.
    pub fn independent(&self, a: i32, b: i32) -> bool {
        let (c, d) = (a.unsigned_abs(), b.unsigned_abs());
        if c == d || !self.holds(a) || !self.holds(b) {
            return false;
        }
        let (c, d) = (c.min(d) as usize - 1, c.max(d) as usize - 1);
        let n = self.0.dependents.len();
        let k = c * (2 * n - c - 1) / 2 + (d - c - 1);
        self.0.bits[k / 8] >> (k % 8) & 1 == 1
    }

    /// The other generators that a letter's generator depends on, counted from 0.
    fn dependents(&self, letter: i32) -> &[u16] {
        &self.0.dependents[letter.unsigned_abs() as usize - 1]
    }

    /// What the scan of [`Relation::key`] reads, the unit that `trace_key` is priced in before the key is built (LP2
    /// draft): each generator once; each letter once into its generator's occurrences and once through the heap,
    /// which holds at most one occurrence per generator (a push and a pop of `⌊log₂ generators⌋ + 1` steps each); and
    /// each generator that a letter's generator depends on twice, when the letter becomes its generator's next
    /// occurrence and when it is emitted. A letter outside the alphabet counts once; the key refuses it.
    pub fn reads(&self, letters: &[i32]) -> u128 {
        let generators = self.0.dependents.len() as u128;
        let heap = 2 * u128::from(u128::BITS - generators.leading_zeros());
        letters.iter().fold(generators, |sum, &l| {
            let dependents = if self.holds(l) { self.dependents(l).len() as u128 } else { 0 };
            sum.saturating_add(1 + heap + 2 * dependents)
        })
    }

    /// LB1.10's trace key of `letters` under this relation. `None` when a letter is not of the relation's alphabet.
    pub fn key(&self, letters: &[i32]) -> Option<TraceKey> {
        let order = self.order(letters)?;
        Some(TraceKey { header: self.0.header.clone(), letters: order.into() })
    }

    /// The least topological word of `letters`. Only the next pending occurrence of each generator can be available,
    /// since a generator depends on itself, and it is available when no generator it depends on has a pending
    /// occurrence before it. `blocked[g]` counts those, so emitting an occurrence revisits only its generator's
    /// dependents.
    fn order(&self, letters: &[i32]) -> Option<Vec<i32>> {
        if !letters.iter().all(|&l| self.holds(l)) {
            return None;
        }
        let generators = self.0.dependents.len();
        let generator = |p: usize| letters[p].unsigned_abs() as usize - 1;
        // The occurrences of each generator in order: a counting sort, then `next[g]` walks `occurrences`.
        let mut start = vec![0usize; generators + 1];
        for p in 0..letters.len() {
            start[generator(p) + 1] += 1;
        }
        for g in 0..generators {
            start[g + 1] += start[g];
        }
        let mut next = start[..generators].to_vec();
        let mut occurrences = vec![0usize; letters.len()];
        for p in 0..letters.len() {
            let g = generator(p);
            occurrences[next[g]] = p;
            next[g] += 1;
        }
        next.copy_from_slice(&start[..generators]);
        let pending = |next: &[usize], g: usize| {
            if next[g] < start[g + 1] {
                occurrences[next[g]]
            } else {
                usize::MAX
            }
        };
        let blockers = |next: &[usize], g: usize, at: usize| {
            self.0.dependents[g].iter().filter(|&&d| pending(next, usize::from(d)) < at).count()
        };
        let mut blocked = vec![0usize; generators];
        let mut heap = BinaryHeap::new();
        for g in 0..generators {
            let at = pending(&next, g);
            if at != usize::MAX {
                blocked[g] = blockers(&next, g, at);
                if blocked[g] == 0 {
                    heap.push(Reverse((rank(letters[at]), at, g)));
                }
            }
        }
        let mut out = Vec::with_capacity(letters.len());
        while let Some(Reverse((_, at, g))) = heap.pop() {
            out.push(letters[at]);
            next[g] += 1;
            let after = pending(&next, g);
            // `g` was available, so every pending dependent occurs after `at` and counted `g` as a blocker.
            for &d in &self.0.dependents[g] {
                let d = usize::from(d);
                let their = pending(&next, d);
                if their != usize::MAX && after > their {
                    blocked[d] -= 1;
                    if blocked[d] == 0 {
                        heap.push(Reverse((rank(letters[their]), their, d)));
                    }
                }
            }
            if after != usize::MAX {
                blocked[g] = blockers(&next, g, after);
                if blocked[g] == 0 {
                    heap.push(Reverse((rank(letters[after]), after, g)));
                }
            }
        }
        Some(out)
    }
}

/// Checks that every trace key in `value`, in its records, receipts, lists, sets and maps, is a key under `relation`:
/// its header is the relation's and its letters are the key of their own trace. The error says why not: no pinned
/// relation, the first field in which the headers differ (LG2.23's order), or letters out of order. Its code is
/// [`KEY_CONTRACT_MISMATCH`]. The walk uses an explicit stack, so a deep value cannot overflow the host stack.
pub fn check_keys(value: &Value, relation: Option<&Relation>) -> Result<(), String> {
    let mut stack = vec![value];
    while let Some(v) = stack.pop() {
        match v {
            Value::TraceEqualityKey(k) => {
                let Some(relation) = relation else {
                    return Err("a trace key where the run pins no trace relation".into());
                };
                if let Some(field) = k.header().differing(relation.header()) {
                    return Err(format!("a trace key under another header: the headers differ in their {field}"));
                }
                if relation.order(k.letters()).as_deref() != Some(k.letters()) {
                    return Err("a trace key whose letters are not the key of their trace".into());
                }
            }
            Value::Record(r) | Value::Receipt(r) => stack.extend(r.fields().iter().map(|(_, v)| v)),
            Value::List(l) | Value::Set(l) | Value::Map(l) => stack.extend(l.items()),
            _ => {}
        }
    }
    Ok(())
}
