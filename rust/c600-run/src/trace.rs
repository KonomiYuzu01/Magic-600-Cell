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
//! The jumbling half (LP2 draft) keys menu words ([`crate::value::MenuWord`]) under `jumble-locality/1`
//! ([`Relation::jumble_locality`]). Distinct grips are independent exactly when stage 14's exact Δ(c,d) is at most
//! zero; a host regenerates that table from the pinned model with `c600_jumble::locality` and passes its bits, so here,
//! too, a caller declares no independence of its own. Letters at one grip are dependent, even when one is the identity
//! item. The letters `[grip, item]` order by grip, then by item, each in natural integer order, and the header carries
//! the whole jumbling identity block, the menu-order digest among it, the grip count and the menu's item count. The
//! scan is the one above, with the grip in the generator's place.

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::menu::{MenuAlphabet, MenuIdentity, MENU_INDEX_LIMIT};
use crate::value::Value;

/// The chapter 13 code of a trace key under another header than the run's relation, or of a run that pins no
/// relation where a trace key is needed (LG2.23, LB1.10). No equality or order result is returned.
pub const KEY_CONTRACT_MISMATCH: &str = "LB1:input.key_contract_mismatch";

/// LB1.10's key kind and rewrite version.
pub const TRACE_KEY: &str = "trace-key/1";

/// LB1.10's independence contract of 600-cell-Full.
pub const FULL_DISJOINT_SUPPORT: &str = "full-disjoint-support/1";

/// LB1.10's independence contract of the retained jumbling geometry.
pub const JUMBLE_LOCALITY: &str = "jumble-locality/1";

/// The generators of 600-cell-Full: its letters have magnitude 1 to 1200.
pub const FULL_GENERATORS: u32 = 1200;

/// The grips of the jumbling puzzle, one per pole of the pinned model: a menu letter's grip is 0 to 599.
pub const JUMBLE_GRIPS: u32 = 600;

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
    /// The jumbling half (LP2 draft): the rest of the identity block and the menu's item count. `None` for Full.
    pub menu: Option<MenuHeader>,
    /// The alphabet: for Full, the signed generators of magnitude 1 to `generators`, in [`letter_order`]; for the
    /// jumbling puzzle, the grip count, the letters `[grip, item]` having a grip below it.
    pub generators: u32,
    /// The key kind and its rewrite version, [`TRACE_KEY`].
    pub key: String,
    /// The independence contract, [`FULL_DISJOINT_SUPPORT`] or [`JUMBLE_LOCALITY`].
    pub contract: String,
    /// The relation digest ([`Relation::digest`]).
    pub digest: [u8; 32],
}

/// The jumbling half of a trace key's header (LG2.23; LP2 draft): the identity block's menu identity, menu-order
/// digest and contract revision, then the menu's item count. A menu letter's item is below `items`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MenuHeader {
    /// The order-free menu identity.
    pub menu_id: String,
    /// The menu-order digest.
    pub menu_order: String,
    /// The state contract revision.
    pub contract_rev: String,
    /// The menu's item count.
    pub items: u32,
}

impl TraceHeader {
    /// The first field in which two headers differ, in LG2.23's order: `puzzle`, `model_id`, `menu_id`, `menu_order`,
    /// `contract_rev`, `generators`, `items`, `key`, `contract`, then `digest`. `None` when they are one header. A
    /// Full header has no menu fields, so it differs from a jumbling header at `puzzle` or, failing that, `menu_id`.
    pub fn differing(&self, other: &TraceHeader) -> Option<&'static str> {
        let (a, b) = (self.menu.as_ref(), other.menu.as_ref());
        [
            ("puzzle", self.puzzle == other.puzzle),
            ("model_id", self.model_id == other.model_id),
            ("menu_id", a.map(|m| &m.menu_id) == b.map(|m| &m.menu_id)),
            ("menu_order", a.map(|m| &m.menu_order) == b.map(|m| &m.menu_order)),
            ("contract_rev", a.map(|m| &m.contract_rev) == b.map(|m| &m.contract_rev)),
            ("generators", self.generators == other.generators),
            ("items", a.map(|m| m.items) == b.map(|m| m.items)),
            ("key", self.key == other.key),
            ("contract", self.contract == other.contract),
            ("digest", self.digest == other.digest),
        ]
        .into_iter()
        .find(|(_, same)| !same)
        .map(|(field, _)| field)
    }

    /// The jumbling identity block of a menu header; `None` for Full.
    pub fn menu_identity(&self) -> Option<MenuIdentity> {
        self.menu.as_ref().map(|m| MenuIdentity {
            puzzle: self.puzzle.clone(),
            model_id: self.model_id.clone(),
            menu_id: m.menu_id.clone(),
            menu_order: m.menu_order.clone(),
            contract_rev: m.contract_rev.clone(),
        })
    }
}

/// A menu letter's place in LB1.10's jumbling order: by grip, then by item, in natural integer order.
fn menu_rank(letter: [u32; 2]) -> u64 {
    u64::from(letter[0]) << 32 | u64::from(letter[1])
}

/// A trace key (LB1.10; LP2 draft): the letters of a Full word's or a menu word's trace in their least topological
/// order, under the header of the relation that ordered them. The header is the run's pinned metadata, shared by its
/// keys, and not payload: a key's logical size is that of its letters, 2 bytes each for Full and 4 for menu letters, as
/// in the words (DET.57). Keys compare by their headers, then by their letters: Full letters in LG2.23's order
/// ([`tokens_order`]), menu letters by grip and then item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceKey {
    header: Arc<TraceHeader>,
    letters: Letters,
}

/// A key's letters: Full letters, or menu letters `[grip, item]`.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Letters {
    Full(Arc<[i32]>),
    Menu(Arc<[[u32; 2]]>),
}

impl TraceKey {
    /// A Full key as a host or the wire hands it over: `letters` under `header`, unchecked. A run checks it against its
    /// relation when it reads it ([`check_keys`]).
    pub fn from_parts(header: Arc<TraceHeader>, letters: &[i32]) -> TraceKey {
        TraceKey { header, letters: Letters::Full(letters.into()) }
    }

    /// A menu key as a host or the wire hands it over (LP2 draft): `letters` under `header`, unchecked, as
    /// [`TraceKey::from_parts`].
    pub fn from_menu_parts(header: Arc<TraceHeader>, letters: &[[u32; 2]]) -> TraceKey {
        TraceKey { header, letters: Letters::Menu(letters.into()) }
    }

    /// The header of the relation that ordered the letters.
    pub fn header(&self) -> &TraceHeader {
        &self.header
    }

    /// The header as the keys of one relation share it.
    pub fn shared_header(&self) -> &Arc<TraceHeader> {
        &self.header
    }

    /// A Full key's letters, in their least topological order; empty for a menu key.
    pub fn letters(&self) -> &[i32] {
        match &self.letters {
            Letters::Full(l) => l,
            Letters::Menu(_) => &[],
        }
    }

    /// A menu key's letters, in their least topological order; empty for a Full key.
    pub fn menu_letters(&self) -> &[[u32; 2]] {
        match &self.letters {
            Letters::Menu(l) => l,
            Letters::Full(_) => &[],
        }
    }

    /// Whether the letters are menu letters.
    pub fn is_menu(&self) -> bool {
        matches!(self.letters, Letters::Menu(_))
    }

    /// The letter count.
    pub fn len(&self) -> usize {
        match &self.letters {
            Letters::Full(l) => l.len(),
            Letters::Menu(l) => l.len(),
        }
    }

    /// Whether the key has no letter.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The logical size of the letters (DET.57): 2 bytes per Full letter, 4 per menu letter.
    pub fn size(&self) -> u128 {
        let per_letter = if self.is_menu() { 4 } else { 2 };
        per_letter * self.len() as u128
    }
}

impl Ord for TraceKey {
    fn cmp(&self, other: &TraceKey) -> Ordering {
        self.header.cmp(&other.header).then_with(|| match (&self.letters, &other.letters) {
            (Letters::Full(a), Letters::Full(b)) => tokens_order(a, b),
            (Letters::Menu(a), Letters::Menu(b)) => {
                a.iter().map(|&l| menu_rank(l)).cmp(b.iter().map(|&l| menu_rank(l)))
            }
            (Letters::Full(_), Letters::Menu(_)) => Ordering::Less,
            (Letters::Menu(_), Letters::Full(_)) => Ordering::Greater,
        })
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
    /// Not 600 grips, one per pole of the jumbling model (LP2 draft).
    Grips(u64),
    /// A menu item count of 0, or above [`MENU_INDEX_LIMIT`] (LP2 draft).
    Items(u64),
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
            RelationError::Grips(n) => write!(f, "{n} grips, not the {JUMBLE_GRIPS} of the jumbling model"),
            RelationError::Items(n) => write!(f, "{n} menu items, not 1 to {MENU_INDEX_LIMIT}"),
            RelationError::EmptySupport(g) => write!(f, "generator {g} moves no slot"),
            RelationError::Slot { generator, slot } => {
                write!(f, "generator {generator}'s support holds slot {slot}, outside 0 to {SLOT_LIMIT}")
            }
            RelationError::Bits => write!(f, "the independence bits are not one bit per pair of distinct generators"),
        }
    }
}

impl std::error::Error for RelationError {}

/// LB1.10's independence relation (LP2 draft), with the header its keys carry: `full-disjoint-support/1` of one pinned
/// 600-cell-Full model, or `jumble-locality/1` of the jumbling model under one pinned menu. Two relations with one
/// header are one relation: the digest covers the model's identity, the contract, the symbol count and every pair, and
/// the header adds the rest of the identity block and the alphabet.
#[derive(Clone)]
pub struct Relation(Arc<Inner>);

struct Inner {
    header: Arc<TraceHeader>,
    /// One bit per pair (c, d) of symbols with c < d, in ascending order of c and then of d, least significant bit
    /// first in each byte, set when the pair is independent. The last byte's unused bits are clear. A symbol is a
    /// Full letter's generator, counted from 0, or a menu letter's grip.
    bits: Vec<u8>,
    /// For each symbol, the other symbols it depends on, ascending.
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
        Ok(Relation::build(puzzle, model_id, None, FULL_GENERATORS, bits))
    }

    /// The relation the wire carries (C3; LP2 draft): its contract, identity block, generator count and independence
    /// bits as [`Relation::bits`] gives them. The digest is recomputed here, never taken from the sender. Refused for
    /// another contract than [`FULL_DISJOINT_SUPPORT`], another generator count than 1,200, an empty identity field,
    /// or bits of another length or with a padding bit set. A jumbling relation comes through
    /// [`Relation::jumble_locality`].
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
        check_bits(generators as usize, &bits)?;
        Ok(Relation::build(puzzle, model_id, None, generators, bits))
    }

    /// LB1.10's `jumble-locality/1` of the jumbling model named by `identity`, for the letters of its menu (LP2 draft):
    /// grips `c` and `d` with `c < d` are independent exactly when their bit in `bits` is set, in the layout of
    /// [`Relation::bits`], and letters at one grip are dependent. A host passes the bits that
    /// `c600_jumble::locality::Locality` regenerates from the pinned model and the exact Δ(c,d), and `items` is the
    /// pinned menu's item count. The wire carries the same parts, and the digest is recomputed here, over the contract,
    /// the puzzle, the model ID, the grip count and the bits: the table is the model's, and the menu's fields are in
    /// the header. Refused for an empty identity field, another grip count than 600, no item or more than
    /// [`MENU_INDEX_LIMIT`], or bits of another length or with a padding bit set.
    pub fn jumble_locality(
        identity: &MenuIdentity,
        grips: u32,
        items: u32,
        bits: Vec<u8>,
    ) -> Result<Relation, RelationError> {
        if let Some(&(field, _)) = identity.fields().iter().find(|(_, text)| text.is_empty()) {
            return Err(RelationError::Identity(field));
        }
        if grips != JUMBLE_GRIPS {
            return Err(RelationError::Grips(u64::from(grips)));
        }
        if !(1..=MENU_INDEX_LIMIT).contains(&items) {
            return Err(RelationError::Items(u64::from(items)));
        }
        check_bits(grips as usize, &bits)?;
        let menu = MenuHeader {
            menu_id: identity.menu_id.clone(),
            menu_order: identity.menu_order.clone(),
            contract_rev: identity.contract_rev.clone(),
            items,
        };
        Ok(Relation::build(&identity.puzzle, &identity.model_id, Some(menu), grips, bits))
    }

    fn build(puzzle: &str, model_id: &str, menu: Option<MenuHeader>, symbols: u32, bits: Vec<u8>) -> Relation {
        let n = symbols as usize;
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
        let contract = if menu.is_some() { JUMBLE_LOCALITY } else { FULL_DISJOINT_SUPPORT };
        let header = TraceHeader {
            puzzle: puzzle.to_string(),
            model_id: model_id.to_string(),
            menu,
            generators: symbols,
            key: TRACE_KEY.to_string(),
            contract: contract.to_string(),
            digest: digest(contract, puzzle, model_id, symbols, &bits),
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

    /// The independence bits: one per pair (c, d) of symbols with c < d, in ascending order of c and then of d, least
    /// significant bit first in each byte, set when the pair is independent. A symbol is a Full letter's generator,
    /// counted from 0, or a menu letter's grip.
    pub fn bits(&self) -> &[u8] {
        &self.0.bits
    }

    /// Whether the relation is a jumbling one, for menu letters.
    pub fn is_menu(&self) -> bool {
        self.0.header.menu.is_some()
    }

    /// Whether the relation keys the menu words of `alphabet`: it is a jumbling relation under the same identity
    /// block, for the same grip count and item count.
    pub fn keys_menu(&self, alphabet: &MenuAlphabet) -> bool {
        let header = &self.0.header;
        header.menu_identity().as_ref() == Some(alphabet.identity())
            && header.generators == alphabet.caps()
            && header.menu.as_ref().map(|m| m.items as usize) == Some(alphabet.items())
    }

    /// Whether `letter` is of the relation's alphabet: a Full relation's, its magnitude 1 to the generator count. No
    /// Full letter is of a jumbling relation's alphabet.
    pub fn holds(&self, letter: i32) -> bool {
        !self.is_menu() && (1..=self.0.header.generators).contains(&letter.unsigned_abs())
    }

    /// Whether a menu letter is of a jumbling relation's alphabet: its grip is below the grip count and its item below
    /// the item count. No menu letter is of a Full relation's alphabet.
    pub fn holds_menu(&self, letter: [u32; 2]) -> bool {
        let header = &self.0.header;
        header.menu.as_ref().is_some_and(|m| letter[0] < header.generators && letter[1] < m.items)
    }

    /// Whether two distinct symbols, counted from 0, are independent.
    fn pair(&self, c: usize, d: usize) -> bool {
        let (c, d) = (c.min(d), c.max(d));
        let n = self.0.dependents.len();
        let k = c * (2 * n - c - 1) / 2 + (d - c - 1);
        self.0.bits[k / 8] >> (k % 8) & 1 == 1
    }

    /// Whether two Full letters are independent: their generators are distinct and their supports disjoint. Letters
    /// outside the alphabet are independent of nothing.
    pub fn independent(&self, a: i32, b: i32) -> bool {
        let (c, d) = (a.unsigned_abs(), b.unsigned_abs());
        c != d && self.holds(a) && self.holds(b) && self.pair(c as usize - 1, d as usize - 1)
    }

    /// Whether two menu letters are independent: their grips are distinct and do not interact. Letters at one grip are
    /// dependent, and letters outside the alphabet are independent of nothing.
    pub fn menu_independent(&self, a: [u32; 2], b: [u32; 2]) -> bool {
        a[0] != b[0] && self.holds_menu(a) && self.holds_menu(b) && self.pair(a[0] as usize, b[0] as usize)
    }

    /// What the scan of [`Relation::key`] reads, the unit that `trace_key` is priced in before the key is built (LP2
    /// draft): each generator once; each letter once into its generator's occurrences and once through the heap,
    /// which holds at most one occurrence per generator (a push and a pop of `⌊log₂ generators⌋ + 1` steps each); and
    /// each generator that a letter's generator depends on twice, when the letter becomes its generator's next
    /// occurrence and when it is emitted. A letter outside the alphabet counts once; the key refuses it.
    pub fn reads(&self, letters: &[i32]) -> u128 {
        let symbols = letters.iter().map(|&l| self.holds(l).then(|| l.unsigned_abs() as usize - 1));
        self.reads_of(symbols)
    }

    /// [`Relation::reads`] for menu letters, with the grip in the generator's place (LP2 draft).
    pub fn menu_reads(&self, letters: &[[u32; 2]]) -> u128 {
        self.reads_of(letters.iter().map(|&l| self.holds_menu(l).then_some(l[0] as usize)))
    }

    fn reads_of(&self, symbols: impl Iterator<Item = Option<usize>>) -> u128 {
        let n = self.0.dependents.len() as u128;
        let heap = 2 * u128::from(u128::BITS - n.leading_zeros());
        symbols.fold(n, |sum, s| {
            let dependents = s.map_or(0, |s| self.0.dependents[s].len() as u128);
            sum.saturating_add(1 + heap + 2 * dependents)
        })
    }

    /// LB1.10's trace key of Full `letters` under this relation. `None` when a letter is not of the relation's
    /// alphabet, as every Full letter is not of a jumbling relation's.
    pub fn key(&self, letters: &[i32]) -> Option<TraceKey> {
        let order = self.order(letters)?;
        Some(TraceKey { header: self.0.header.clone(), letters: Letters::Full(order.into()) })
    }

    /// LB1.10's trace key of menu `letters` under a jumbling relation (LP2 draft). `None` when a letter is not of the
    /// relation's alphabet, as every menu letter is not of a Full relation's.
    pub fn menu_key(&self, letters: &[[u32; 2]]) -> Option<TraceKey> {
        let order = self.menu_order(letters)?;
        Some(TraceKey { header: self.0.header.clone(), letters: Letters::Menu(order.into()) })
    }

    fn order(&self, letters: &[i32]) -> Option<Vec<i32>> {
        if !letters.iter().all(|&l| self.holds(l)) {
            return None;
        }
        Some(self.order_by(letters, |l| l.unsigned_abs() as usize - 1, rank))
    }

    fn menu_order(&self, letters: &[[u32; 2]]) -> Option<Vec<[u32; 2]>> {
        if !letters.iter().all(|&l| self.holds_menu(l)) {
            return None;
        }
        Some(self.order_by(letters, |l| l[0] as usize, menu_rank))
    }

    /// The least topological word of `letters`, whose symbols (generators or grips) are all of the relation. Only the
    /// next pending occurrence of each symbol can be available, since a symbol depends on itself, and it is available
    /// when no symbol it depends on has a pending occurrence before it. `blocked[g]` counts those, so emitting an
    /// occurrence revisits only its symbol's dependents.
    fn order_by<L: Copy>(&self, letters: &[L], symbol: impl Fn(L) -> usize, rank: impl Fn(L) -> u64) -> Vec<L> {
        let symbols = self.0.dependents.len();
        let generator = |p: usize| symbol(letters[p]);
        // The occurrences of each symbol in order: a counting sort, then `next[g]` walks `occurrences`.
        let mut start = vec![0usize; symbols + 1];
        for p in 0..letters.len() {
            start[generator(p) + 1] += 1;
        }
        for g in 0..symbols {
            start[g + 1] += start[g];
        }
        let mut next = start[..symbols].to_vec();
        let mut occurrences = vec![0usize; letters.len()];
        for p in 0..letters.len() {
            let g = generator(p);
            occurrences[next[g]] = p;
            next[g] += 1;
        }
        next.copy_from_slice(&start[..symbols]);
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
        let mut blocked = vec![0usize; symbols];
        let mut heap = BinaryHeap::new();
        for g in 0..symbols {
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
        out
    }
}

/// Refuses independence bits that are not one bit per pair of distinct symbols among `n` in whole bytes, or that set a
/// padding bit.
fn check_bits(n: usize, bits: &[u8]) -> Result<(), RelationError> {
    let pairs = n * n.saturating_sub(1) / 2;
    let padded = pairs % 8 != 0 && bits.last().is_some_and(|last| last >> (pairs % 8) != 0);
    if bits.len() != pair_bytes(n) || padded {
        return Err(RelationError::Bits);
    }
    Ok(())
}

/// Checks that every trace key in `value`, in its records, receipts, lists, sets and maps, is a key under `relation`:
/// its header is the relation's and its letters are the key of their own trace. The error says why not: no pinned
/// relation, the first field in which the headers differ (LG2.23's order), letters of the other alphabet, or letters
/// out of order. Its code is [`KEY_CONTRACT_MISMATCH`]. The walk uses an explicit stack, so a deep value cannot
/// overflow the host stack.
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
                if k.is_menu() != relation.is_menu() {
                    return Err("a trace key whose letters are not of its header's alphabet".into());
                }
                let ordered = if k.is_menu() {
                    relation.menu_order(k.menu_letters()).as_deref() == Some(k.menu_letters())
                } else {
                    relation.order(k.letters()).as_deref() == Some(k.letters())
                };
                if !ordered {
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
