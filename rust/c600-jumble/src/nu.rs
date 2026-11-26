//! ν, the label invariant of A5 (LP2: KN2.4's `nu` row, over KN3.2's F2 and F5).
//!
//! ν(X) is the invariant of the labels π(X) of a lattice configuration (state contract, amendments A3 and A5). It has
//! 45 coordinates ([`NAMES`]): the sign bits b0 to b34 and the frame bits of orbits 0, 3, 6, 8, 11, 22, 25 and 32
//! (f0, f3 and so on) in F2, and the values C5_17 and C5_28 in F5 ([`NuVector`]). ν is defined on lattice
//! configurations only: off the lattice it is undefined, never zeros, a partial vector or an old value (KN2.4, LB3.1).
//!
//! # Coordinates
//!
//! The coordinates are those of the research reference `nucoords.py` (restoration stage 03), read here from `assets/`
//! alone (PHS.18). Every piece other than a centre gets a labelling of its home slots: the seed's two buffers take the
//! atlas frames of the seed's first two target pieces, and every other piece the frame of the first execution-tree
//! node that names it. With L = π(X), where L[s] is the home slot of the sticker now in slot s:
//!
//! - b_o is the parity of the permutation of orbit o's positions that sends each position to the piece now in it;
//! - the frame value of orbit o adds, over the positions that hold a moved sticker, the class of the frame permutation
//!   of the piece there (it sends the label of each of the piece's home slots to the label of the slot that sticker
//!   now fills) in the abelianisation of the orbit's frame group H_o: C2 for orbits 0, 3, 8, 11, 22, 25 and 32 (an
//!   element other than the identity counts 1); D5 for orbit 6 (a reflection, an element of order 2, counts 1); C5
//!   for orbits 17 and 28, read in the generator r0, the least element of order 5 ([`C5_CONVENTION`]); and A5 for
//!   orbit 34, which is perfect, so its classes are 0. The other orbits have trivial frame groups and add nothing.
//!
//! H_o is read from the tree frames at the seed's third piece, in the labels of the atlas frame of that piece, and
//! its kind is fixed as `nucoords.py` fixes it ([`group_kind`]). Both are checked against the census
//! (`orientation_group` and `orientation_order`) when the tables are built, on first use ([`tables`]). A frame
//! permutation outside H_o, at an orbit whose group is not trivial, is an engine error.
//!
//! # Rows
//!
//! - [`value`]: ν of a state, total on a collision-free lattice configuration; undefined with [`OFF_LATTICE`] when a
//!   piece is off the lattice, and with [`PIECES_COLLIDE`] when a raw table puts two pieces in one chamber (KN2.14).
//! - [`nu`]: KN2.4's row on a snapshot or a derived value. A5 fixes nine coordinates at 0 on every reachable lattice
//!   configuration under I_a (stage 04, Theorems A and B5; [`I_A_FIXED`]). When the state's menu is I_a by its
//!   order-free identity ([`I_A_MENU_IDENTITY`]) and its claim is witnessed at that state, a nonzero fixed coordinate
//!   is refused with `KN2:consistency.fixed_nu_nonzero`, form freeze (chapter 13): the engine, its tables or the
//!   theorem is wrong, and the value is no valid state. Another menu, free mode, an open, promised, by-theorem,
//!   refuted or imported claim, and a witnessed claim made for another state make no claim on those coordinates, so
//!   their value comes back as it is.
//!
//! [`crate::levels::label_checkpoint`] takes the value as the ν row's 45 values: `Some(&value.values())`. Nothing here
//! chooses, ranks or proposes a correction or a twist.

use std::collections::{BTreeSet, HashMap};

use c600_kernel::exact::{F2, F5};
use c600_kernel::json::{self, Value};
use c600_kernel::{archive, Refusal};

use crate::levels::{self, Base, Partial, ReachKind, NU_LEN};
use crate::model::{NP, NS};
use crate::state::{EngineError, State};
use crate::Context;

/// The orbits of non-centre pieces.
const ORBITS: usize = 35;
/// The largest decompressed tree file the tables accept.
const LIMIT: usize = 1 << 30;
/// The sign bits b0 to b34 and the frame bits come first, in F2; the two C5 values last, in F5.
const BITS: usize = 43;

/// The coordinates in order (A5).
pub const NAMES: [&str; NU_LEN] = [
    "b0", "b1", "b2", "b3", "b4", "b5", "b6", "b7", "b8", "b9", "b10", "b11", "b12", "b13", "b14", "b15", "b16", "b17",
    "b18", "b19", "b20", "b21", "b22", "b23", "b24", "b25", "b26", "b27", "b28", "b29", "b30", "b31", "b32", "b33",
    "b34", "f0", "f3", "f6", "f8", "f11", "f22", "f25", FRAME_32, "C5_17", "C5_28",
];

/// The name of orbit 32's frame bit, spelled with `concat!` so that CT-KN2-05's scan for float types does not read it
/// as one.
const FRAME_32: &str = concat!("f", "32");

/// The modulus of each coordinate: 2 for the 43 bits, 5 for the two C5 values.
pub const MODULI: [i64; NU_LEN] = {
    let mut moduli = [2; NU_LEN];
    moduli[BITS] = 5;
    moduli[BITS + 1] = 5;
    moduli
};

/// The orbits of the eight frame bits, in coordinate order.
pub const FRAME_BIT_ORBITS: [usize; 8] = [0, 3, 6, 8, 11, 22, 25, 32];

/// The orbits of the two values modulo 5, in coordinate order.
pub const C5_ORBITS: [usize; 2] = [17, 28];

/// The coordinates that are 0 on every reachable lattice configuration under I_a (stage 04, Theorems A and B5):
/// b0, b1, b2, b3, b5, b6, f0, f3 and f6.
pub const I_A_FIXED: [usize; 9] = [0, 1, 2, 3, 5, 6, 35, 36, 37];

/// The C5 convention of A5: the generator the two values modulo 5 are read in.
pub const C5_CONVENTION: &str = "nucoords-r0: the least order-5 frame permutation of the third buffer";

/// SHA-256 of the certificate program A5 names (restoration stage 03, `certificates/certificates.json.gz`). It is
/// pinned here, not read: this crate reads `assets/` alone.
pub const CERTIFICATE_PROGRAM_SHA256: &str = "a11584e469da8b9461b2e73a291a31b7ec7be3ec6dc7ca00c5bb80ca4ce081af";

/// The order-free identity of the signed I_a menu, under which [`I_A_FIXED`] holds.
pub const I_A_MENU_IDENTITY: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";

/// The I_a menu's order digest: a menu letter (grip, item index) is read in this item order.
pub const I_A_ORDER_DIGEST: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";

/// The reason ν is undefined when a piece is off the lattice.
pub const OFF_LATTICE: &str = "off_lattice";

/// The reason ν is undefined when a raw table puts two pieces in one chamber.
pub const PIECES_COLLIDE: &str = "pieces_collide";

// --------------------------------------------------------------------------------------------------------- vector

/// The 45 coordinates of ν: 43 in F2 and 2 in F5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NuVector {
    bits: [F2; BITS],
    c5: [F5; 2],
}

impl NuVector {
    pub const ZERO: NuVector = NuVector { bits: [F2::ZERO; BITS], c5: [F5::ZERO; 2] };

    /// 45 values in coordinate order, each in 0..its modulus ([`MODULI`]); `None` otherwise.
    pub fn from_values(values: &[i64]) -> Option<NuVector> {
        if values.len() != NU_LEN || values.iter().zip(MODULI).any(|(&v, m)| !(0..m).contains(&v)) {
            return None;
        }
        let mut out = NuVector::ZERO;
        for (bit, &v) in out.bits.iter_mut().zip(values) {
            *bit = F2::new(v);
        }
        for (c5, &v) in out.c5.iter_mut().zip(&values[BITS..]) {
            *c5 = F5::new(v);
        }
        Some(out)
    }

    /// The 45 values in coordinate order.
    pub fn values(&self) -> [i64; NU_LEN] {
        let mut out = [0; NU_LEN];
        for (o, bit) in out.iter_mut().zip(&self.bits) {
            *o = bit.value() as i64;
        }
        for (o, c5) in out[BITS..].iter_mut().zip(&self.c5) {
            *o = c5.value() as i64;
        }
        out
    }

    /// The 35 sign bits and the eight frame bits, in F2.
    pub fn bits(&self) -> &[F2; BITS] {
        &self.bits
    }

    /// C5_17 and C5_28, in F5.
    pub fn c5(&self) -> &[F5; 2] {
        &self.c5
    }

    pub fn is_zero(&self) -> bool {
        *self == NuVector::ZERO
    }

    /// The nonzero coordinates with their values, in coordinate order.
    pub fn nonzero(&self) -> Vec<(&'static str, i64)> {
        NAMES.iter().zip(self.values()).filter(|(_, v)| *v != 0).map(|(&name, v)| (name, v)).collect()
    }

    /// The names of the coordinates of [`I_A_FIXED`] that are not 0.
    pub fn fixed_nonzero(&self) -> Vec<&'static str> {
        let values = self.values();
        I_A_FIXED.iter().filter(|&&i| values[i] != 0).map(|&i| NAMES[i]).collect()
    }

    /// self + m·row, coordinate by coordinate in F2 and F5, for a signed integer m.
    pub fn add_times(&self, row: &NuVector, m: i64) -> NuVector {
        let (m2, m5) = (F2::new(m), F5::new(m));
        let mut out = *self;
        for (a, &b) in out.bits.iter_mut().zip(&row.bits) {
            *a += m2 * b;
        }
        for (a, &b) in out.c5.iter_mut().zip(&row.c5) {
            *a += m5 * b;
        }
        out
    }

    /// The 45 values as a JSON list.
    pub fn to_json(&self) -> Value {
        Value::Array(self.values().iter().map(|&v| Value::Int(v)).collect())
    }
}

impl std::ops::Add for NuVector {
    type Output = NuVector;

    fn add(self, other: NuVector) -> NuVector {
        self.add_times(&other, 1)
    }
}

// --------------------------------------------------------------------------------------------------------- groups

/// The kind of an orbit's frame group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind {
    Trivial,
    C2,
    D5,
    C5,
    A5,
}

impl GroupKind {
    /// The census's name for it, which the census's `orientation_group` starts with.
    pub fn name(self) -> &'static str {
        match self {
            GroupKind::Trivial => "trivial",
            GroupKind::C2 => "C2",
            GroupKind::D5 => "D5",
            GroupKind::C5 => "C5",
            GroupKind::A5 => "A5",
        }
    }

    pub fn order(self) -> usize {
        match self {
            GroupKind::Trivial => 1,
            GroupKind::C2 => 2,
            GroupKind::D5 => 10,
            GroupKind::C5 => 5,
            GroupKind::A5 => 60,
        }
    }

    /// The modulus of the class an element adds: 2 for C2 and D5, 5 for C5, 1 (no class) for A5 and a trivial group.
    pub fn modulus(self) -> u8 {
        match self {
            GroupKind::C2 | GroupKind::D5 => 2,
            GroupKind::C5 => 5,
            GroupKind::Trivial | GroupKind::A5 => 1,
        }
    }
}

/// The kind of an orbit's frame group, as `nucoords.py` fixes it; the tables check it against the census.
pub fn group_kind(orbit: usize) -> GroupKind {
    match orbit {
        0 | 3 | 8 | 11 | 22 | 25 | 32 => GroupKind::C2,
        6 => GroupKind::D5,
        17 | 28 => GroupKind::C5,
        34 => GroupKind::A5,
        _ => GroupKind::Trivial,
    }
}

/// x then y: (x·y)[i] = y[x[i]], as `nucoords.py` composes frame permutations.
fn compose(x: &[u8], y: &[u8]) -> Vec<u8> {
    x.iter().map(|&i| y[i as usize]).collect()
}

/// The order of a permutation.
fn order(x: &[u8]) -> usize {
    let identity: Vec<u8> = (0..x.len() as u8).collect();
    let (mut y, mut n) = (x.to_vec(), 1);
    while y != identity {
        y = compose(&y, x);
        n += 1;
    }
    n
}

/// An orbit's frame group H_o: permutations of the labels 0..k of a piece's k slots, each with its class.
#[derive(Clone, Debug)]
pub struct FrameGroup {
    kind: GroupKind,
    slots: usize,
    classes: HashMap<Vec<u8>, u8>,
    generator: Option<Vec<u8>>,
}

impl FrameGroup {
    pub fn kind(&self) -> GroupKind {
        self.kind
    }

    /// k, the slots of each piece of the orbit.
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// The number of elements of H_o, as read from the trees; the census's `orientation_order` when the tables built.
    pub fn order(&self) -> usize {
        self.classes.len()
    }

    /// C5 only: r0, the least element of order 5 in the lexicographic order of its images (the C5 convention).
    pub fn generator(&self) -> Option<&[u8]> {
        self.generator.as_deref()
    }

    /// The class of an element of the group, below [`GroupKind::modulus`]; `None` for a permutation outside it.
    pub fn class(&self, h: &[u8]) -> Option<u8> {
        self.classes.get(h).copied()
    }

    /// The classes of a group of `kind` with the elements `elements`, as `nucoords.py` assigns them.
    fn new(orbit: usize, kind: GroupKind, slots: usize, elements: BTreeSet<Vec<u8>>) -> Result<FrameGroup, String> {
        let identity: Vec<u8> = (0..slots as u8).collect();
        if elements.len() != kind.order() || !elements.contains(&identity) {
            return Err(format!("orbit {orbit}'s frame group is not {} of order {}", kind.name(), kind.order()));
        }
        let mut generator = None;
        let classes: HashMap<Vec<u8>, u8> = match kind {
            GroupKind::Trivial | GroupKind::A5 => elements.into_iter().map(|h| (h, 0)).collect(),
            GroupKind::C2 => elements.into_iter().map(|h| (h.clone(), (h != identity) as u8)).collect(),
            GroupKind::D5 => elements.into_iter().map(|h| (h.clone(), (order(&h) == 2) as u8)).collect(),
            GroupKind::C5 => {
                // The least element of order 5; its powers r0^i have class i.
                let r0 = elements.iter().find(|h| order(h) == 5).cloned();
                let r0 = r0.ok_or_else(|| format!("orbit {orbit}'s frame group has no element of order 5"))?;
                let mut classes = HashMap::new();
                let mut x = identity.clone();
                for i in 0..5 {
                    classes.insert(x.clone(), i);
                    x = compose(&x, &r0);
                }
                if classes.len() != 5 || classes.keys().any(|h| !elements.contains(h)) {
                    return Err(format!("orbit {orbit}'s frame group is not generated by r0"));
                }
                generator = Some(r0);
                classes
            }
        };
        Ok(FrameGroup { kind, slots, classes, generator })
    }
}

// --------------------------------------------------------------------------------------------------------- tables

/// The labelling of every piece's home slots and the frame groups, built once per context from the verified bytes of
/// `execution_trees.json.gz`, `seed_atlas.json` and `census.json`.
pub struct Tables {
    /// The slots of piece p in label order are `lab_slots[lab_start[p]..lab_start[p + 1]]`; a centre has none.
    lab_start: Vec<u32>,
    lab_slots: Vec<u32>,
    /// The label of each slot of a non-centre piece in its piece's labelling; `u8::MAX` for a centre's slots.
    lab_index: Vec<u8>,
    groups: Vec<FrameGroup>,
}

/// ν's tables of a context, built on first use.
pub fn tables(ctx: &Context) -> Result<&Tables, EngineError> {
    ctx.nu.get_or_init(|| Tables::build(ctx)).as_ref().map_err(|e| EngineError(e.clone()))
}

fn ints(value: Option<&Value>, what: &str) -> Result<Vec<i64>, String> {
    let values = value.and_then(Value::as_array).ok_or_else(|| format!("{what} is not a list"))?;
    values.iter().map(|v| v.as_i64().ok_or_else(|| format!("{what} holds a value that is not an integer"))).collect()
}

fn slot_list(value: &Value, what: &str) -> Result<Vec<u32>, String> {
    ints(Some(value), what)?
        .into_iter()
        .map(|s| u32::try_from(s).ok().filter(|&s| (s as usize) < NS).ok_or_else(|| format!("{what} names no slot")))
        .collect()
}

impl Tables {
    /// The frame group of an orbit, 0 to 34.
    pub fn group(&self, orbit: usize) -> Option<&FrameGroup> {
        self.groups.get(orbit)
    }

    /// The home slots of a piece in label order; empty for a centre and for a number that is no piece.
    pub fn labelling(&self, piece: usize) -> &[u32] {
        if piece >= NP {
            return &[];
        }
        &self.lab_slots[self.lab_start[piece] as usize..self.lab_start[piece + 1] as usize]
    }

    fn build(ctx: &Context) -> Result<Tables, String> {
        let model = &ctx.model;
        let files = &model.labelling;
        let trees = archive::gunzip(&files.trees_gz, LIMIT).map_err(|e| format!("execution_trees.json.gz: {e}"))?;
        let trees = json::parse(&trees).map_err(|e| format!("execution_trees.json.gz: {e}"))?;
        let atlas = json::parse(&files.atlas).map_err(|e| format!("seed_atlas.json: {e}"))?;
        let census = json::parse(&files.census).map_err(|e| format!("census.json: {e}"))?;
        let trees = trees.as_array().filter(|rows| rows.len() == ORBITS);
        let trees = trees.ok_or("execution_trees.json.gz does not hold 35 trees")?;
        let atlas = atlas.as_array().filter(|rows| rows.len() == ORBITS);
        let atlas = atlas.ok_or("seed_atlas.json does not hold 35 orbits")?;
        let census = census.get("orbits").and_then(Value::as_array).filter(|rows| rows.len() == ORBITS);
        let census = census.ok_or("census.json does not hold 35 orbits")?;

        let mut home_slots = vec![0usize; NP];
        for &p in &model.slot_piece {
            home_slots[p as usize] += 1;
        }
        let mut lab: Vec<Option<Vec<u32>>> = vec![None; NP];
        let mut groups = Vec::with_capacity(ORBITS);
        for o in 0..ORBITS {
            let (tree, seed, row) = (&trees[o], &atlas[o], &census[o]);
            let numbered = |value: &Value, key: &str| value.get(key).and_then(Value::as_i64) == Some(o as i64);
            if !numbered(tree, "orbit") || !numbered(seed, "orbit") || !numbered(row, "id") {
                return Err(format!("the rows of orbit {o} are out of order"));
            }
            // A piece the orbit's rows name must be one of its pieces.
            let piece = |p: i64| {
                usize::try_from(p)
                    .ok()
                    .filter(|&p| p < NP && model.orbit_id[p] == o as i16)
                    .ok_or_else(|| format!("orbit {o}'s rows name {p}, which is not one of its pieces"))
            };
            let buffers = ints(tree.get("buffers"), "a tree's buffers")?;
            let third = tree.get("third").and_then(Value::as_i64).ok_or("a tree names no third piece")?;
            let positions = ints(tree.get("positions"), "a tree's positions")?;
            let frames = tree.get("frames").and_then(Value::as_array).ok_or("a tree's frames are not a list")?;
            let seed_frames = seed.get("frames").and_then(Value::as_array).filter(|f| f.len() == 3);
            let seed_frames = seed_frames.ok_or("seed_atlas.json frames are malformed")?;
            if buffers.len() != 2 || positions.len() != frames.len() {
                return Err(format!("orbit {o}'s execution tree is malformed"));
            }
            let (a, b, c) = (piece(buffers[0])?, piece(buffers[1])?, piece(third)?);
            if a == b {
                return Err(format!("orbit {o}'s buffers are one piece"));
            }
            let third_frame = slot_list(&seed_frames[2], "seed_atlas.json frame")?;
            let k = third_frame.len();
            let label: HashMap<u32, u8> = third_frame.iter().enumerate().map(|(j, &s)| (s, j as u8)).collect();
            if k == 0 || k >= u8::MAX as usize || label.len() != k {
                return Err(format!("orbit {o}'s third frame is malformed"));
            }
            // nucoords.py's labelling: the buffers first, then each position's first tree frame.
            lab[a] = Some(slot_list(&seed_frames[0], "seed_atlas.json frame")?);
            lab[b] = Some(slot_list(&seed_frames[1], "seed_atlas.json frame")?);
            let mut elements = BTreeSet::new();
            for (n, &p) in positions.iter().enumerate() {
                let p = piece(p)?;
                if lab[p].is_none() {
                    lab[p] = Some(slot_list(&frames[n], "a tree frame")?);
                }
                if p == c {
                    let h: Option<Vec<u8>> =
                        slot_list(&frames[n], "a tree frame")?.iter().map(|s| label.get(s).copied()).collect();
                    let h = h.filter(|h| h.len() == k && h.iter().collect::<BTreeSet<_>>().len() == k);
                    elements.insert(h.ok_or_else(|| format!("a tree frame at orbit {o}'s third piece is malformed"))?);
                }
            }
            let kind = group_kind(o);
            let named = row.get("orientation_group").and_then(Value::as_str).unwrap_or("");
            let stated = row.get("orientation_order").and_then(Value::as_i64);
            if !named.starts_with(kind.name()) || stated != Some(kind.order() as i64) {
                return Err(format!("the census names orbit {o}'s frame group otherwise than {}", kind.name()));
            }
            groups.push(FrameGroup::new(o, kind, k, elements)?);
        }

        // Every non-centre piece has a labelling, a permutation of its home slots as long as its orbit's k; no centre
        // has one.
        let mut lab_start = Vec::with_capacity(NP + 1);
        let mut lab_slots = Vec::with_capacity(NS);
        let mut lab_index = vec![u8::MAX; NS];
        lab_start.push(0u32);
        for (p, slots) in lab.into_iter().enumerate() {
            match (slots, usize::try_from(model.orbit_id[p])) {
                (None, Err(_)) => {}
                (Some(slots), Ok(o)) if slots.len() == groups[o].slots && home_slots[p] == slots.len() => {
                    for (t, &s) in slots.iter().enumerate() {
                        if model.slot_piece[s as usize] as usize != p || lab_index[s as usize] != u8::MAX {
                            return Err(format!("piece {p}'s labelling is not a permutation of its home slots"));
                        }
                        lab_index[s as usize] = t as u8;
                    }
                    lab_slots.extend_from_slice(&slots);
                }
                (None, Ok(_)) => return Err(format!("piece {p} has no labelling")),
                (Some(_), Err(_)) => return Err(format!("centre {p} has a labelling")),
                (Some(_), Ok(_)) => return Err(format!("piece {p}'s labelling has the wrong length")),
            }
            lab_start.push(lab_slots.len() as u32);
        }
        Ok(Tables { lab_start, lab_slots, lab_index, groups })
    }

    /// ν of a total labelling (`labels[s]` is the home slot of the sticker now in slot s), as `nucoords.py` computes it.
    fn of_labels(&self, ctx: &Context, labels: &[i32]) -> Result<NuVector, String> {
        let model = &ctx.model;
        if labels.len() != NS {
            return Err(format!("a labelling has {NS} slots, not {}", labels.len()));
        }
        let home = |s: u32| {
            let l = labels[s as usize];
            usize::try_from(l).ok().filter(|&l| l < NS).ok_or_else(|| format!("slot {s} holds the label {l}"))
        };
        let orbit = |p: usize| usize::try_from(model.orbit_id[p]).ok();
        // nucoords.py's X: the positions that hold a moved sticker.
        let mut moved = vec![false; NP];
        for (s, &l) in labels.iter().enumerate() {
            if l as i64 != s as i64 {
                moved[model.slot_piece[s] as usize] = true;
            }
        }
        // The piece now at a position: the home piece of the sticker in its first labelled slot.
        let piece_at = |x: usize| Ok::<usize, String>(model.slot_piece[home(self.labelling(x)[0])?] as usize);

        // Sign bits: the parity of each orbit's permutation of its positions, cycle by cycle.
        let mut sign = [F2::ZERO; ORBITS];
        let mut seen = vec![false; NP];
        for x in 0..NP {
            let Some(o) = orbit(x).filter(|_| moved[x] && !seen[x]) else { continue };
            let (mut length, mut y) = (0usize, x);
            while !seen[y] {
                seen[y] = true;
                length += 1;
                let next = piece_at(y)?;
                if orbit(next) != Some(o) {
                    return Err(format!("the piece at position {y} is not of orbit {o}"));
                }
                y = next;
            }
            if y != x {
                return Err(format!("the pieces of orbit {o} do not permute its positions"));
            }
            if length % 2 == 0 {
                sign[o] += F2::ONE;
            }
        }

        // Frame values: the classes of the moved positions' frame permutations.
        let mut total = [0u32; ORBITS];
        for x in 0..NP {
            let Some(o) = orbit(x).filter(|_| moved[x]) else { continue };
            let group = &self.groups[o];
            if group.kind == GroupKind::Trivial {
                continue;
            }
            let at = piece_at(x)?;
            let mut v = vec![u8::MAX; group.slots];
            for (t, &s) in self.labelling(x).iter().enumerate() {
                let from = home(s)?;
                let i = self.lab_index[from] as usize;
                if model.slot_piece[from] as usize != at || i >= group.slots || v[i] != u8::MAX {
                    return Err(format!("the stickers at position {x} are not one piece's"));
                }
                v[i] = t as u8;
            }
            let class = group.class(&v).ok_or_else(|| {
                format!("the frame of the piece at position {x} lies outside orbit {o}'s frame group")
            })?;
            total[o] += class as u32;
        }

        let mut out = NuVector::ZERO;
        out.bits[..ORBITS].copy_from_slice(&sign);
        for (i, &o) in FRAME_BIT_ORBITS.iter().enumerate() {
            out.bits[ORBITS + i] = F2::new(total[o] as i64);
        }
        for (i, &o) in C5_ORBITS.iter().enumerate() {
            out.c5[i] = F5::new(total[o] as i64);
        }
        Ok(out)
    }
}

// ----------------------------------------------------------------------------------------------------------- rows

/// ν of a state (A5): total on a collision-free lattice configuration; undefined with [`OFF_LATTICE`] when a piece is
/// off the lattice and with [`PIECES_COLLIDE`] when two pieces of a raw table share a chamber. It makes no claim about
/// reachability.
pub fn value(ctx: &Context, state: &State) -> Result<Partial<NuVector>, EngineError> {
    if state.off_lattice_count() > 0 {
        return Ok(Partial::Undefined(OFF_LATTICE.into()));
    }
    let labels = match levels::labels(ctx, state)? {
        Partial::Total(labels) => labels,
        Partial::Partial { .. } => return Ok(Partial::Undefined(PIECES_COLLIDE.into())),
        Partial::Undefined(reason) => return Ok(Partial::Undefined(reason)),
    };
    let t = tables(ctx)?;
    Ok(Partial::Total(t.of_labels(ctx, &labels.labels).map_err(EngineError)?))
}

/// The refusal of a value with a nonzero coordinate of [`I_A_FIXED`]: `KN2:consistency.fixed_nu_nonzero`.
fn consistency(value: &NuVector) -> Result<(), Refusal> {
    let fixed = value.fixed_nonzero();
    if fixed.is_empty() {
        return Ok(());
    }
    Err(Refusal {
        code: crate::codes::FIXED_NU_NONZERO,
        message: format!(
            "ν is nonzero in {} on a lattice configuration witnessed under I_a; stage 04 fixes these coordinates at 0",
            fixed.join(", ")
        ),
        fact: None,
    })
}

/// The row's decision on a value, apart from its state. [`I_A_FIXED`] binds a total value when the menu is I_a by its
/// order-free identity and the claim is witnessed at the value's own state (`at` equals the state's `digest`). A
/// witnessed claim made for another state binds nothing here, as the label checkpoint answers unknown for it (A4, A5).
fn decide(
    result: Partial<NuVector>,
    menu: Option<&str>,
    kind: ReachKind,
    at: Option<&str>,
    digest: impl FnOnce() -> String,
) -> Result<Partial<NuVector>, Refusal> {
    if let Partial::Total(value) = &result {
        if menu == Some(I_A_MENU_IDENTITY) && kind == ReachKind::Witnessed && at.is_some_and(|at| at == digest()) {
            consistency(value)?;
        }
    }
    Ok(result)
}

/// KN2.4's `nu` row on a snapshot or a derived value: [`value`], and under I_a with a claim witnessed at the value's
/// state, the refusal `KN2:consistency.fixed_nu_nonzero` when a coordinate of [`I_A_FIXED`] is not 0.
pub fn nu(ctx: &Context, base: Base) -> Result<Result<Partial<NuVector>, Refusal>, EngineError> {
    let (state, claim) = match base {
        Base::Snapshot(snapshot) => (snapshot.state(), snapshot.reachability()),
        Base::Derived(derived) => (derived.state(), derived.reachability()),
    };
    let result = value(ctx, state)?;
    let menu = state.menu().map(|menu| menu.identity.as_str());
    Ok(decide(result, menu, claim.kind(), claim.at(), || state.digest(ctx)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_follow_a5() {
        assert_eq!(NAMES.len(), NU_LEN);
        assert_eq!(MODULI.iter().filter(|&&m| m == 2).count(), BITS);
        assert_eq!(&MODULI[BITS..], &[5, 5]);
        for (i, &o) in FRAME_BIT_ORBITS.iter().enumerate() {
            assert_eq!(NAMES[ORBITS + i], format!("f{o}"));
            assert_eq!(group_kind(o).modulus(), 2);
        }
        for (i, &o) in C5_ORBITS.iter().enumerate() {
            assert_eq!(NAMES[BITS + i], format!("C5_{o}"));
            assert_eq!(group_kind(o), GroupKind::C5);
        }
        let fixed: Vec<&str> = I_A_FIXED.iter().map(|&i| NAMES[i]).collect();
        assert_eq!(fixed, ["b0", "b1", "b2", "b3", "b5", "b6", "f0", "f3", "f6"]);
    }

    #[test]
    fn vectors_add_in_f2_and_f5() {
        let mut values = [0i64; NU_LEN];
        values[4] = 1;
        values[ORBITS] = 1;
        values[BITS] = 3;
        let row = NuVector::from_values(&values).expect("a vector");
        assert_eq!(row.nonzero(), vec![("b4", 1), ("f0", 1), ("C5_17", 3)]);
        assert_eq!(row.fixed_nonzero(), vec!["f0"]);
        assert!(row.add_times(&row, 1).values()[..BITS].iter().all(|&v| v == 0));
        assert_eq!(row.add_times(&row, 1).values()[BITS], 1);
        assert_eq!(row.add_times(&row, -1), NuVector::ZERO);
        assert_eq!(row.add_times(&row, 1), row + row);
        // 10·row is 0 in F2 and in F5.
        assert_eq!(row.add_times(&row, 9), NuVector::ZERO);
        assert_eq!(NuVector::ZERO.add_times(&row, 4).values()[BITS..], [2, 0]);
        assert!(NuVector::ZERO.add_times(&row, 4).values()[..BITS].iter().all(|&v| v == 0));
        assert!(NuVector::from_values(&values[..44]).is_none());
        values[BITS] = 5;
        assert!(NuVector::from_values(&values).is_none());
        values[BITS] = 0;
        values[0] = -1;
        assert!(NuVector::from_values(&values).is_none());
        assert_eq!(NuVector::ZERO.to_json().canonical(), format!("[{}]", vec!["0"; NU_LEN].join(",")));
    }

    #[test]
    fn a_nonzero_fixed_coordinate_is_refused_with_its_names() {
        assert_eq!(consistency(&NuVector::ZERO), Ok(()));
        let mut values = [0i64; NU_LEN];
        values[4] = 1;
        values[BITS + 1] = 2;
        assert_eq!(consistency(&NuVector::from_values(&values).expect("a vector")), Ok(()), "b4 and C5_28 are free");
        values[1] = 1;
        values[37] = 1;
        let planted = NuVector::from_values(&values).expect("a vector");
        assert_eq!(planted.fixed_nonzero(), vec!["b1", "f6"]);
        let refusal = consistency(&planted).expect_err("b1 and f6 are fixed");
        assert_eq!(refusal.code, "KN2:consistency.fixed_nu_nonzero");
    }

    /// The row's decision with a planted value: only I_a by its identity, a witnessed claim and that claim made at the
    /// value's own state refuse it; every other case gives the value back as it is.
    #[test]
    fn only_a_claim_witnessed_at_the_state_under_i_a_binds_the_fixed_coordinates() {
        let mut values = [0i64; NU_LEN];
        values[0] = 1;
        let planted = Partial::Total(NuVector::from_values(&values).expect("a vector"));
        let here = || "the state's digest".to_string();
        let decide_with = |menu, kind, at| decide(planted.clone(), menu, kind, at, here);
        let refused = decide_with(Some(I_A_MENU_IDENTITY), ReachKind::Witnessed, Some("the state's digest"));
        assert_eq!(refused.expect_err("b0 is fixed under I_a").code, crate::codes::FIXED_NU_NONZERO);
        let other_menu = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";
        for (menu, kind, at) in [
            (Some(other_menu), ReachKind::Witnessed, Some("the state's digest")),
            (None, ReachKind::Witnessed, Some("the state's digest")),
            (Some(I_A_MENU_IDENTITY), ReachKind::Witnessed, Some("another state's digest")),
            (Some(I_A_MENU_IDENTITY), ReachKind::Witnessed, None),
            (Some(I_A_MENU_IDENTITY), ReachKind::Open, None),
            (Some(I_A_MENU_IDENTITY), ReachKind::Promised, None),
            (Some(I_A_MENU_IDENTITY), ReachKind::ByTheorem, None),
            (Some(I_A_MENU_IDENTITY), ReachKind::Refuted, None),
        ] {
            assert_eq!(decide_with(menu, kind, at), Ok(planted.clone()), "{menu:?} {kind:?} {at:?}");
        }
        let undefined = Partial::Undefined(OFF_LATTICE.to_string());
        let decided =
            decide(undefined.clone(), Some(I_A_MENU_IDENTITY), ReachKind::Witnessed, Some("d"), || "d".into());
        assert_eq!(decided, Ok(undefined));
    }

    #[test]
    fn the_c5_generator_is_the_least_element_of_order_5() {
        // The rotations of a pentagon's five labels: r0 is the least image list of order 5.
        let elements: BTreeSet<Vec<u8>> = (0..5u8).map(|i| (0..5u8).map(|j| (i + j) % 5).collect()).collect();
        let group = FrameGroup::new(17, GroupKind::C5, 5, elements).expect("C5");
        assert_eq!(group.generator(), Some(&[1u8, 2, 3, 4, 0][..]));
        assert_eq!(group.class(&[2, 3, 4, 0, 1]), Some(2));
        assert_eq!(group.class(&[0, 1, 2, 3, 4]), Some(0));
        assert_eq!(group.class(&[1, 0, 2, 3, 4]), None);
        let pair: BTreeSet<Vec<u8>> = [vec![0u8, 1], vec![1, 0]].into_iter().collect();
        assert!(FrameGroup::new(0, GroupKind::C5, 2, pair.clone()).is_err(), "a group of order 2 is no C5");
        let c2 = FrameGroup::new(0, GroupKind::C2, 2, pair).expect("C2");
        assert_eq!((c2.class(&[1, 0]), c2.class(&[0, 1])), (Some(1), Some(0)));
    }
}
