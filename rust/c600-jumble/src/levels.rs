//! Level crossings, end tests and shapes of the jumbling puzzle (LP2, milestone M8).
//!
//! Chapter 01, KN1.7 to KN1.11 and KN2.4; state contract A3 and A5. Everything is exact. Nothing here chooses, ranks
//! or outputs a twist: reachability is evidence the caller supplies or a derivation records ([`Reachability`]), and
//! the end tests replay only the witness they are given.
//!
//! # Levels (KN1.7 to KN1.10)
//!
//! A jumbling state holds poses, the finest level. Every other level is reached by an explicit projection
//! ([`project`]) along the maps of KN1.7, which go one way, finer to coarser ([`Level::maps`]). No call here takes a
//! value of a coarser level and returns a finer one (KN1.10). A projection of a state reads:
//!
//! - poses: the pose digest, total;
//! - sticker class: refused with `KN1:input.level_not_held`, because its key "does not exist yet" (KN1.8; a draft);
//! - labels: π of A3, computed as J1's `lattice_stickers` does, with every sticker frame checked against the slot
//!   layout. On the lattice it is total, and its digest is `Digest<labels>`, the SHA-256 of the int32 label array
//!   (KN1.8, the kernel's snapshot digest). Off the lattice it is a typed partial result: the stickers of lattice
//!   pieces cover their slots, and the missing part counts the pieces off the lattice, the slots nothing covers and
//!   the slots two pieces claim (KN1.9, CT-KN1-03);
//! - position: each piece's chamber, total on the lattice and partial off it in the same way;
//! - region class: each piece's posed region ([`RegionKey`]), total;
//! - coset and tree data: each piece's JI.53 domain ID, and its tree heights and branches ([`crate::observe`]), total;
//! - shape: the multiset of posed regions with the off count ([`Shape`]), total.
//!
//! A posed region is keyed exactly. A lattice piece with pose k sits in the chamber of signature k(M(A)). Off the
//! lattice the key is the SHA-256 of the sorted lowest-terms vertices of g(Ā₀); such a region is still recognised as
//! a chamber when its vertex set equals one (looked up by its vertex centroid, then compared vertex by vertex), so a
//! region key never depends on which pose produced it.
//!
//! # The slot action of K⁺ (A3)
//!
//! [`Tables::slot_image`] is A3's k·s for a lattice pose k: the slot that the sticker in home slot s reaches when its
//! piece turns by k, with the sticker's frame checked against the slot layout. It is a left action of K⁺ on the 259,800
//! slots, and [`Tables::piece_slots`] gives a piece's slots at a pose in the order of its home slots. The labels of a
//! state are computed through it. A pose off the lattice has no slot action, so the labels stay partial there.
//!
//! # End tests (A5; KN2.4 lists three separate rows)
//!
//! Each answers a verdict of KN0.5: yes or no with evidence, or unknown with a reason, never a boolean.
//!
//! - [`sticker_test`]: N = 0 and π(X) is the identity. It reads the sticker class only and makes no reachability
//!   claim (KN1.15, KN1.17).
//! - [`retained_checkpoint`]: section 5. Off the lattice, or with a centre outside its own chamber, the answer is no.
//!   Otherwise the witness, supplied or reduced from the journal as J1's `State.checkpoint` reduces it, is replayed
//!   exactly from solved, and equal pose digests answer yes; a witness that does not replay, or reaches another
//!   configuration, leaves the answer unknown.
//! - [`label_checkpoint`]: A5. The clauses that do not depend on reachability decide first: off the lattice, a
//!   displaced centre, or a known nonzero ν, no. ν is known as 0 from the identity labels or from a retained witness
//!   (card 1, F1), or from 45 values the caller supplies from the ν row. Then only a witnessed claim decides (ONE.28):
//!   open or promised reachability answers unknown, at ν = 0 too (CT-ONE-10); by theorem and refuted answer unknown,
//!   because this edition checks no theorem evidence (a draft). Witnessed with ν unknown, the answer is unknown. The
//!   certificate carries the pose digest, `Digest<labels>`, the frame agreement, ν with its source, the reachability
//!   claim and A5's contract revision. The C5 convention and the certificate program's SHA-256 belong to the ν row
//!   ([`crate::nu`]), which pins them; this crate reads `assets/` alone (PHS.18).
//!
//! # Derived states (KN2.7, KN2.8, KN2.11 to KN2.14; ONE.20 to ONE.28)
//!
//! [`derive`] executes a word's literal letters on a copy of its base: a [`Snapshot`] (solved, or a state that its
//! journal from solved witnesses) or an earlier [`Derived`] value. The base's identity block must be the one given
//! (KN1.4). A blocked or invalid letter refuses the word at that prefix with `KN2:input.word_blocked_prefix`, the
//! letter's outcome and its straddle certificate (KN2.8); nothing changes and no value is made. Protection is judged
//! at pose level in the kernel's modes ([`ProtectionMode`]): Net compares each protected piece's pose at the end of
//! the word with its pose in the base and refuses with no prefix; Strict judges every prefix and refuses at the first
//! letter that changes a protected piece's pose (KN2.7, LB2.3). Both refuse with `KN2:protection.piece_would_move`
//! and certify, per such piece, its pose before and after the judged operation. A derived value keeps its base's
//! identity and lineage, has no revision and is never adopted (KN2.12, ONE.22). Its claim is the base's, carried
//! along the word ([`Reachability::then`]): a witnessed claim stays witnessed, an open one stays open.
//!
//! [`import`] is KN2.14: a raw pose table enters as a derived value with open or promised reachability, never
//! witnessed, validated for form and identity only, with no derivation and the import's digest as its lineage. It
//! can be read, projected, compared and derived from (ONE.27), and the values derived from it keep its claim, so a
//! label checkpoint on them answers unknown at ν = 0 too. [`compare`] is ONE.25: both states are projected to one
//! level and compared there, totally or on the part that both cover.
//!
//! Drafts: a snapshot's lineage is its pose digest, since revisions come from the session store (M6); no
//! classification budget is taken (KN7 comes with LP3), so an uncertain letter is an engine error; and nothing here
//! proposes or adopts a derived value, which is the command layer's and the session store's work.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use c600_kernel::json::{self, Value};
use c600_kernel::{IdentityBlock, ProtectionMode, Puzzle, Refusal, Verdict as ProtectionVerdict};

use crate::kplus::{IntMat, ORDER};
use crate::model::{CELL_SLOTS, NP, NS, SIG_WORDS};
use crate::q5::{matmul, transpose, Mat4};
use crate::regions::VForm;
use crate::state::{EngineError, Pose, State, Status};
use crate::twist::{Menu, Twist};
use crate::{b64, observe, Context};

/// A level this puzzle or input does not hold (KN1.9).
pub const LEVEL_NOT_HELD: &str = "KN1:input.level_not_held";

/// The number of ν coordinates (A5: 35 sign bits, 8 frame bits and two values modulo 5).
pub const NU_LEN: usize = 45;

fn engine(message: impl Into<String>) -> EngineError {
    EngineError(message.into())
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn int(n: usize) -> Value {
    Value::Int(n as i64)
}

fn object(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

// ------------------------------------------------------------------------------------------------------ levels

/// The identity levels of KN1.7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Poses,
    StickerClass,
    Labels,
    RegionClass,
    Position,
    Coset,
    TreeData,
    Shape,
}

impl Level {
    pub const ALL: [Level; 8] = [
        Level::Poses,
        Level::StickerClass,
        Level::Labels,
        Level::RegionClass,
        Level::Position,
        Level::Coset,
        Level::TreeData,
        Level::Shape,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Level::Poses => "poses",
            Level::StickerClass => "sticker_class",
            Level::Labels => "labels",
            Level::RegionClass => "region_class",
            Level::Position => "position",
            Level::Coset => "coset",
            Level::TreeData => "tree_data",
            Level::Shape => "shape",
        }
    }

    pub fn parse(name: &str) -> Option<Level> {
        Level::ALL.into_iter().find(|level| level.name() == name)
    }

    /// The maps of KN1.7's table that leave this level, each one step from a finer level to a coarser one.
    pub fn maps(self) -> &'static [Level] {
        match self {
            Level::Poses => &[Level::StickerClass],
            Level::StickerClass => &[Level::Labels, Level::RegionClass],
            Level::Labels => &[Level::Position],
            Level::RegionClass => &[Level::Coset, Level::Shape],
            Level::Coset => &[Level::TreeData],
            Level::Position | Level::TreeData | Level::Shape => &[],
        }
    }

    /// Whether a chain of KN1.7's maps leads from this level to `to`; every level reaches itself.
    pub fn reaches(self, to: Level) -> bool {
        let mut stack = vec![self];
        let mut seen = Vec::new();
        while let Some(level) = stack.pop() {
            if level == to {
                return true;
            }
            if !seen.contains(&level) {
                seen.push(level);
                stack.extend_from_slice(level.maps());
            }
        }
        false
    }

    /// Whether the level is defined on lattice configurations only (KN1.7's table).
    pub fn lattice_only(self) -> bool {
        matches!(self, Level::Labels | Level::Position)
    }
}

/// The typed partial result of KN0.4: total, partial with what it covers and what is missing, or undefined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Partial<T> {
    Total(T),
    Partial { covered: T, missing: Missing },
    Undefined(String),
}

/// What a partial projection does not define.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Missing {
    /// The pieces off the lattice: N, the off count (KN1.9).
    pub off: usize,
    /// Slots (labels) or chambers (position) that no lattice piece occupies.
    pub uncovered: usize,
    /// Slots or chambers that two or more lattice pieces claim; only a raw table collides (KN2.14).
    pub collisions: usize,
}

impl Missing {
    pub fn to_json(&self) -> Value {
        object(vec![("off", int(self.off)), ("uncovered", int(self.uncovered)), ("collisions", int(self.collisions))])
    }
}

// ------------------------------------------------------------------------------------------------------ tables

/// Lookup tables of the level crossings, built once per model.
pub struct Tables {
    model_id: String,
    /// `piece_of[o * ORDER + k]`: the chamber of signature k(M(rep_o)).
    piece_of: Vec<u32>,
    /// The slots of piece p are `slot_list[slot_start[p]..slot_start[p + 1]]`, ascending.
    slot_start: Vec<u32>,
    slot_list: Vec<u32>,
    /// The position of a K⁺ element in the stabiliser of pole 0, or -1.
    stab0_pos: Vec<i8>,
    /// Per element of the stabiliser of pole 0: the slot of cell 0 each slot of cell 0 is carried to.
    base_region_perm: Vec<Vec<u16>>,
    /// Each centre piece with its pole.
    centres: Vec<(u32, u16)>,
    chambers: OnceLock<Result<Chambers, String>>,
}

/// The vertex centroid of every chamber, for recognising an off-lattice region that is a chamber.
struct Chambers {
    /// (lowest-terms vertex centroid, piece), sorted.
    by_centroid: Vec<([i64; 9], u32)>,
}

static TABLES: OnceLock<Mutex<Vec<Arc<Tables>>>> = OnceLock::new();

/// The level tables of a context's model, built on first use.
pub fn tables(ctx: &Context) -> Result<Arc<Tables>, EngineError> {
    let cell = TABLES.get_or_init(|| Mutex::new(Vec::new()));
    let mut cached = cell.lock().map_err(|_| engine("the level tables are poisoned"))?;
    if let Some(found) = cached.iter().find(|t| t.model_id == ctx.model.model_id) {
        return Ok(found.clone());
    }
    let built = Arc::new(Tables::build(ctx)?);
    cached.push(built.clone());
    Ok(built)
}

impl Tables {
    fn build(ctx: &Context) -> Result<Tables, EngineError> {
        let (model, kp, reg) = (&ctx.model, &ctx.kplus, &ctx.regions);
        let fail = |what: &str| engine(format!("level tables: {what}"));
        let mut sig_index: HashMap<[u64; SIG_WORDS], u32> = HashMap::with_capacity(NP);
        for p in 0..NP {
            sig_index.insert(*model.sig_bits(p), p as u32);
        }
        let mut piece_of = vec![0u32; reg.reps.len() * ORDER];
        for (o, &rep) in reg.reps.iter().enumerate() {
            let sig = model.signature(rep as usize);
            for (k, perm) in kp.perms.iter().enumerate() {
                let mut bits = [0u64; SIG_WORDS];
                for &e in sig {
                    let f = perm[e as usize] as usize;
                    bits[f / 64] |= 1u64 << (f % 64);
                }
                piece_of[o * ORDER + k] = *sig_index.get(&bits).ok_or_else(|| fail("a K+ image has no chamber"))?;
            }
        }
        for p in 0..NP {
            let (o, k) = reg.rep_of(p);
            if piece_of[o * ORDER + k as usize] != p as u32 {
                return Err(fail("a transport does not reach its piece"));
            }
        }
        let mut slot_start = vec![0u32; NP + 1];
        for &p in &model.slot_piece {
            slot_start[p as usize + 1] += 1;
        }
        for p in 0..NP {
            slot_start[p + 1] += slot_start[p];
        }
        let mut fill = slot_start.clone();
        let mut slot_list = vec![0u32; NS];
        for (s, &p) in model.slot_piece.iter().enumerate() {
            slot_list[fill[p as usize] as usize] = s as u32;
            fill[p as usize] += 1;
        }
        let mut stab0_pos = vec![-1i8; ORDER];
        for (i, &s) in kp.stab0.iter().enumerate() {
            stab0_pos[s as usize] = i as i8;
        }
        let centres = model.centre_pieces.iter().map(|&p| (p, model.signature(p as usize)[0])).collect();
        let mut tables = Tables {
            model_id: model.model_id.clone(),
            piece_of,
            slot_start,
            slot_list,
            stab0_pos,
            base_region_perm: Vec::new(),
            centres,
            chambers: OnceLock::new(),
        };
        for &s in &kp.stab0 {
            let mut row = vec![0u16; CELL_SLOTS];
            let mut seen = vec![false; CELL_SLOTS];
            for (j, entry) in row.iter_mut().enumerate() {
                let image = tables.image(ctx, model.slot_piece[j] as usize, s);
                let slot = tables.slot_of(image, 0).ok_or_else(|| fail("a base-cell image has no slot on cell 0"))?;
                if slot as usize >= CELL_SLOTS || std::mem::replace(&mut seen[slot as usize], true) {
                    return Err(fail("a base-cell region map is not a permutation"));
                }
                *entry = slot as u16;
            }
            tables.base_region_perm.push(row);
        }
        Ok(tables)
    }

    /// The chamber of piece p moved by the K⁺ element k: the piece whose signature is k(M(p)).
    pub fn image(&self, ctx: &Context, p: usize, k: u16) -> u32 {
        let (o, t) = ctx.regions.rep_of(p);
        self.piece_of[o * ORDER + ctx.kplus.compose(k, t) as usize]
    }

    /// The slots of piece p, ascending.
    pub fn slots(&self, p: usize) -> &[u32] {
        &self.slot_list[self.slot_start[p] as usize..self.slot_start[p + 1] as usize]
    }

    /// The slot of piece p on cell f, if p has a sticker there.
    pub fn slot_of(&self, p: u32, f: u16) -> Option<u32> {
        self.slots(p as usize).iter().copied().find(|&s| s as usize / CELL_SLOTS == f as usize)
    }

    /// The centre pieces, each with its pole.
    pub fn centres(&self) -> &[(u32, u16)] {
        &self.centres
    }

    /// The slot action of K⁺ (A3): the slot that the sticker whose home slot is `s` occupies when its piece has the
    /// lattice pose `k`. The slot is its piece's image chamber's slot on the image of its cell, and the sticker's frame
    /// is checked against the slot layout; a disagreement is an engine error. It is a left action:
    /// `slot_image(ctx, s, 0)` is `s`, and `slot_image(ctx, s, kp.compose(k1, k2))` is
    /// `slot_image(ctx, slot_image(ctx, s, k2)?, k1)`. A slot outside 0..NS or an element outside K⁺ is an engine error.
    pub fn slot_image(&self, ctx: &Context, s: u32, k: u16) -> Result<u32, EngineError> {
        let p = ctx.model.slot_piece.get(s as usize).ok_or_else(|| engine(format!("slot {s} is outside 0..{NS}")))?;
        if k as usize >= ORDER {
            return Err(engine(format!("K+ element {k} is outside 0..{ORDER}")));
        }
        if k == 0 {
            return Ok(s);
        }
        self.carry(ctx, s, k, self.image(ctx, *p as usize, k))
    }

    /// The slots of piece `p` at the lattice pose `k`, in the order of [`Tables::slots`]: entry i is
    /// [`Tables::slot_image`] of `slots(p)[i]`, so it holds the sticker whose home slot is `slots(p)[i]`.
    pub fn piece_slots(&self, ctx: &Context, p: usize, k: u16) -> Result<Vec<u32>, EngineError> {
        if p >= NP {
            return Err(engine(format!("piece {p} is outside 0..{NP}")));
        }
        if k as usize >= ORDER {
            return Err(engine(format!("K+ element {k} is outside 0..{ORDER}")));
        }
        let mut out = Vec::with_capacity(self.slots(p).len());
        self.place(ctx, p, k, |_, d| out.push(d))?;
        Ok(out)
    }

    /// Calls `each(s, k·s)` for every slot s of piece p, in the order of [`Tables::slots`]; p and k are in range.
    fn place(&self, ctx: &Context, p: usize, k: u16, mut each: impl FnMut(u32, u32)) -> Result<(), EngineError> {
        if k == 0 {
            self.slots(p).iter().for_each(|&s| each(s, s));
            return Ok(());
        }
        let to = self.image(ctx, p, k);
        for &s in self.slots(p) {
            each(s, self.carry(ctx, s, k, to)?);
        }
        Ok(())
    }

    /// k·s for a slot s whose piece k carries to the chamber `to`, with A3's frame check: the frame change
    /// l = F(f₂)⁻¹·k·F(f) of the sticker's cell f and its image cell f₂ must lie in the stabiliser of pole 0 and carry
    /// the sticker's place on its cell to the place of the slot found.
    fn carry(&self, ctx: &Context, s: u32, k: u16, to: u32) -> Result<u32, EngineError> {
        let kp = &ctx.kplus;
        let f = s as usize / CELL_SLOTS;
        let f2 = kp.perms[k as usize][f];
        let d = self.slot_of(to, f2).ok_or_else(|| engine(format!("chamber {to} has no slot on cell {f2}")))?;
        let l = kp.compose(kp.inv[kp.frame_idx[f2 as usize] as usize], kp.compose(k, kp.frame_idx[f]));
        let orient = self.stab0_pos[l as usize];
        if orient < 0 || self.base_region_perm[orient as usize][s as usize % CELL_SLOTS] as u32 != d % CELL_SLOTS as u32
        {
            return Err(engine(format!("the frame of sticker {s} disagrees with the slot layout (A3)")));
        }
        Ok(d)
    }

    fn chambers(&self, ctx: &Context) -> Result<&Chambers, EngineError> {
        self.chambers.get_or_init(|| Chambers::build(ctx)).as_ref().map_err(|e| engine(e.clone()))
    }
}

impl Chambers {
    fn build(ctx: &Context) -> Result<Chambers, String> {
        let (kp, reg, model) = (&ctx.kplus, &ctx.regions, &ctx.model);
        let reps: Vec<[i128; 9]> = reg
            .rep_vform
            .iter()
            .map(|form| centroid(form).ok_or("a centroid leaves i128"))
            .collect::<Result<_, _>>()?;
        let mut by_centroid = Vec::with_capacity(NP);
        for p in 0..NP {
            let (o, t) = reg.rep_of(p);
            let c = transport_point(kp.int_matrix(t, model), &reps[o]).ok_or("a chamber centroid leaves i128")?;
            by_centroid.push((small_key(&c).ok_or("a chamber centroid leaves i64")?, p as u32));
        }
        by_centroid.sort_unstable();
        if by_centroid.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err("two chambers share a vertex centroid".into());
        }
        Ok(Chambers { by_centroid })
    }

    fn lookup(&self, key: &[i64; 9]) -> Option<u32> {
        self.by_centroid.binary_search_by(|(k, _)| k.cmp(key)).ok().map(|i| self.by_centroid[i].1)
    }
}

// ------------------------------------------------------------------------------------------- exact vertex forms

fn row_i128(row: &[i64; 8]) -> [i128; 8] {
    row.map(|x| x as i128)
}

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        if a <= u64::MAX as u128 && b <= u64::MAX as u128 {
            let (mut x, mut y) = (a as u64, b as u64);
            while y != 0 {
                (x, y) = (y, x % y);
            }
            return x as u128;
        }
        (a, b) = (b, a % b);
    }
    a
}

/// The lowest-terms form (den, x₀..x₃, z₀..z₃) of the vector (x + z√5)/den, den > 0.
fn lowest_small(row: &[i128; 8], den: i128) -> Option<[i128; 9]> {
    if den <= 0 {
        return None;
    }
    let mut g = den.unsigned_abs();
    for x in row {
        if g == 1 {
            break;
        }
        g = gcd_u128(g, x.unsigned_abs());
    }
    let g = g as i128;
    let mut out = [0i128; 9];
    out[0] = den / g;
    for (o, x) in out[1..].iter_mut().zip(row) {
        *o = x / g;
    }
    Some(out)
}

/// The lowest-terms vertex centroid (den, x, z) of a vertex form: the mean of its rows.
fn centroid(form: &VForm) -> Option<[i128; 9]> {
    let mut sum = [0i128; 8];
    for row in &form.rows {
        for (acc, &x) in sum.iter_mut().zip(row) {
            *acc = acc.checked_add(x as i128)?;
        }
    }
    lowest_small(&sum, (form.e as i128).checked_mul(form.rows.len() as i128)?)
}

/// The lowest-terms image of a lowest-terms point (den, x, z) under the K⁺ matrix (g₁ + g₂√5)/d.
fn transport_point(r: &IntMat, c: &[i128; 9]) -> Option<[i128; 9]> {
    let mut y = [0i128; 8];
    for i in 0..4 {
        for j in 0..4 {
            let (g1, g2) = (r.g1[i][j] as i128, r.g2[i][j] as i128);
            let (x, z) = (c[1 + j], c[5 + j]);
            y[i] = y[i].checked_add(g1.checked_mul(x)?)?.checked_add(g2.checked_mul(z)?.checked_mul(5)?)?;
            y[4 + i] = y[4 + i].checked_add(g1.checked_mul(z)?)?.checked_add(g2.checked_mul(x)?)?;
        }
    }
    lowest_small(&y, c[0].checked_mul(r.d as i128)?)
}

/// The lowest-terms form of a big vector, as decimal texts (den first).
fn lowest_big(row: &[BigInt; 8], den: &BigInt) -> Vec<BigInt> {
    let mut g = den.abs();
    for x in row {
        g = g.gcd(x);
    }
    std::iter::once(den / &g).chain(row.iter().map(|v| v / &g)).collect()
}

fn small_key(form: &[i128; 9]) -> Option<[i64; 9]> {
    let mut key = [0i64; 9];
    for (k, &x) in key.iter_mut().zip(form) {
        *k = i64::try_from(x).ok()?;
    }
    Some(key)
}

fn form_text(form: &[i128; 9]) -> String {
    format!(
        "{}|{},{},{},{}|{},{},{},{}",
        form[0], form[1], form[2], form[3], form[4], form[5], form[6], form[7], form[8]
    )
}

fn big_text(row: &[BigInt; 8], den: &BigInt) -> String {
    let x: Vec<String> = lowest_big(row, den).iter().map(BigInt::to_string).collect();
    format!("{}|{},{},{},{}|{},{},{},{}", x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7], x[8])
}

/// An exact rotation in integer form (P + Q√5)/D, with an i128 copy when it is small.
struct IntRotation {
    p: [[BigInt; 4]; 4],
    q: [[BigInt; 4]; 4],
    d: BigInt,
    small: Option<([[i128; 4]; 4], [[i128; 4]; 4], i128)>,
}

impl IntRotation {
    fn new(m: &Mat4) -> IntRotation {
        let mut d = BigInt::one();
        for row in m {
            for x in row {
                d = d.lcm(x.d());
            }
        }
        let p: [[BigInt; 4]; 4] = std::array::from_fn(|i| std::array::from_fn(|j| m[i][j].a() * (&d / m[i][j].d())));
        let q: [[BigInt; 4]; 4] = std::array::from_fn(|i| std::array::from_fn(|j| m[i][j].b() * (&d / m[i][j].d())));
        const BOUND: i128 = 1 << 60;
        let fit = |x: &BigInt| x.to_i128().filter(|v| v.abs() < BOUND);
        let small = (|| {
            let mut sp = [[0i128; 4]; 4];
            let mut sq = [[0i128; 4]; 4];
            for i in 0..4 {
                for j in 0..4 {
                    sp[i][j] = fit(&p[i][j])?;
                    sq[i][j] = fit(&q[i][j])?;
                }
            }
            Some((sp, sq, fit(&d)?))
        })();
        IntRotation { p, q, d, small }
    }

    /// The image (y₁ + y₂√5)/(D e) of the point (x + z√5)/e, unreduced, when it stays small.
    fn apply_small(&self, row: &[i128; 8], e: i128) -> Option<([i128; 8], i128)> {
        let (p, q, d) = self.small.as_ref()?;
        let mut y = [0i128; 8];
        for i in 0..4 {
            let (mut y1, mut y2) = (0i128, 0i128);
            for j in 0..4 {
                let (x, z) = (row[j], row[4 + j]);
                y1 = y1.checked_add(p[i][j].checked_mul(x)?)?.checked_add(q[i][j].checked_mul(z)?.checked_mul(5)?)?;
                y2 = y2.checked_add(p[i][j].checked_mul(z)?)?.checked_add(q[i][j].checked_mul(x)?)?;
            }
            y[i] = y1;
            y[4 + i] = y2;
        }
        Some((y, d.checked_mul(e)?))
    }

    fn apply_big(&self, row: &[BigInt; 8], e: &BigInt) -> ([BigInt; 8], BigInt) {
        let mut y: [BigInt; 8] = std::array::from_fn(|_| BigInt::zero());
        for i in 0..4 {
            for j in 0..4 {
                let (x, z) = (&row[j], &row[4 + j]);
                y[i] += &self.p[i][j] * x + &self.q[i][j] * z * 5;
                y[4 + i] += &self.p[i][j] * z + &self.q[i][j] * x;
            }
        }
        (y, &self.d * e)
    }

    /// The lowest-terms text of the image of one vertex.
    fn vertex_text(&self, row: &[i64; 8], e: i64) -> String {
        if let Some((y, den)) = self.apply_small(&row_i128(row), e as i128) {
            if let Some(form) = lowest_small(&y, den) {
                return form_text(&form);
            }
        }
        let (y, den) = self.apply_big(&row.map(BigInt::from), &BigInt::from(e));
        big_text(&y, &den)
    }

    /// The lowest-terms image of a lowest-terms point (den, x, z), as a centroid key when it fits in i64.
    fn centroid_key(&self, c: &[i128; 9]) -> Option<[i64; 9]> {
        let row: [i128; 8] = std::array::from_fn(|j| c[1 + j]);
        if let Some(form) = self.apply_small(&row, c[0]).and_then(|(y, den)| lowest_small(&y, den)) {
            return small_key(&form);
        }
        let (y, den) = self.apply_big(&row.map(BigInt::from), &BigInt::from(c[0]));
        let form = lowest_big(&y, &den);
        let mut key = [0i64; 9];
        for (k, x) in key.iter_mut().zip(&form) {
            *k = x.to_i64()?;
        }
        Some(key)
    }
}

/// The sorted lowest-terms vertex texts of a region given by its vertex form.
fn identity_texts(form: &VForm) -> Vec<String> {
    let mut texts: Vec<String> = form
        .rows
        .iter()
        .map(|row| match lowest_small(&row_i128(row), form.e as i128) {
            Some(f) => form_text(&f),
            None => big_text(&row.map(BigInt::from), &BigInt::from(form.e)),
        })
        .collect();
    texts.sort_unstable();
    texts
}

fn texts_digest(texts: &[String]) -> String {
    c600_digest::sha256_hex(texts.join(";").as_bytes())
}

// ------------------------------------------------------------------------------------------------- region keys

/// The exact key of a posed region (KN1.7's region class of one piece).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegionKey {
    /// The home chamber of this piece: a lattice region.
    Chamber(u32),
    /// A region that is no chamber: the SHA-256 of its sorted lowest-terms vertex texts.
    Off(String),
}

impl RegionKey {
    pub fn text(&self) -> String {
        match self {
            RegionKey::Chamber(p) => format!("chamber:{p}"),
            RegionKey::Off(digest) => format!("region:{digest}"),
        }
    }

    pub fn is_chamber(&self) -> bool {
        matches!(self, RegionKey::Chamber(_))
    }
}

/// The key of the region g(Ā₀) of piece p under an exact rotation g.
pub fn region_key_of(ctx: &Context, t: &Tables, p: usize, g: &Pose) -> Result<RegionKey, EngineError> {
    match g {
        Pose::K(k) => Ok(RegionKey::Chamber(t.image(ctx, p, *k))),
        Pose::M(m) => off_region_key(ctx, t, p, &IntRotation::new(m)),
    }
}

fn off_region_key(ctx: &Context, t: &Tables, p: usize, rot: &IntRotation) -> Result<RegionKey, EngineError> {
    let (kp, model) = (&ctx.kplus, &ctx.model);
    let home = ctx.regions.vform(p, kp, model);
    let mut texts: Vec<String> = home.rows.iter().map(|row| rot.vertex_text(row, home.e)).collect();
    texts.sort_unstable();
    let chambers = t.chambers(ctx)?;
    let middle = centroid(&home).ok_or_else(|| engine(format!("the centroid of piece {p} leaves i128")))?;
    if let Some(b) = rot.centroid_key(&middle).and_then(|key| chambers.lookup(&key)) {
        if identity_texts(&ctx.regions.vform(b as usize, kp, model)) == texts {
            return Ok(RegionKey::Chamber(b));
        }
    }
    Ok(RegionKey::Off(texts_digest(&texts)))
}

/// Whether the rotation g carries the home region of piece a onto the home region of piece b.
pub fn carries(ctx: &Context, t: &Tables, a: usize, g: &Pose, b: usize) -> Result<bool, EngineError> {
    Ok(region_key_of(ctx, t, a, g)? == RegionKey::Chamber(b as u32))
}

/// Every piece's posed region key; each off-lattice pose's integer form is built once.
pub fn region_keys(ctx: &Context, t: &Tables, state: &State) -> Result<Vec<RegionKey>, EngineError> {
    let mut rotations: Vec<Option<IntRotation>> = (0..state.poses().len()).map(|_| None).collect();
    let mut keys = Vec::with_capacity(NP);
    for (p, &id) in state.pose_ids().iter().enumerate() {
        keys.push(match &state.poses()[id as usize] {
            Pose::K(k) => RegionKey::Chamber(t.image(ctx, p, *k)),
            Pose::M(m) => off_region_key(ctx, t, p, rotations[id as usize].get_or_insert_with(|| IntRotation::new(m)))?,
        });
    }
    Ok(keys)
}

// ------------------------------------------------------------------------------------------------- level values

/// π(X) on the slots it covers: `labels[i]` is the home slot of the sticker now in slot `slots[i]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Labels {
    pub slots: Vec<u32>,
    pub labels: Vec<i32>,
}

impl Labels {
    /// Of a total value, `Digest<labels>` (KN1.8): the SHA-256 of the labels as little-endian int32. Of a covered
    /// part, the SHA-256 of its (slot as u32, label as i32) little-endian pairs, which names no level of KN1.8 and
    /// compares only with another covered part.
    pub fn digest(&self) -> String {
        let total = self.slots.len() == NS;
        let mut bytes = Vec::with_capacity(self.labels.len() * if total { 4 } else { 8 });
        for (&s, &l) in self.slots.iter().zip(&self.labels) {
            if !total {
                bytes.extend_from_slice(&s.to_le_bytes());
            }
            bytes.extend_from_slice(&l.to_le_bytes());
        }
        c600_digest::sha256_hex(&bytes)
    }

    /// The slots whose label is not their own.
    pub fn displaced(&self) -> usize {
        self.slots.iter().zip(&self.labels).filter(|(&s, &l)| s as i64 != l as i64).count()
    }
}

/// Each lattice piece's chamber: piece `pieces[i]` sits in the home chamber of piece `chambers[i]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Positions {
    pub pieces: Vec<u32>,
    pub chambers: Vec<u32>,
}

impl Positions {
    /// The SHA-256 of the (piece, chamber) pairs as little-endian u32 (of a total value, the chambers alone).
    pub fn digest(&self) -> String {
        let total = self.pieces.len() == NP;
        let mut bytes = Vec::new();
        for (&p, &c) in self.pieces.iter().zip(&self.chambers) {
            if !total {
                bytes.extend_from_slice(&p.to_le_bytes());
            }
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        c600_digest::sha256_hex(&bytes)
    }
}

/// The multiset of posed regions (KN1.7), with the off count of KN1.15.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape {
    /// Each occupied region with the number of pieces in it, in key order.
    pub regions: Vec<(RegionKey, u32)>,
    /// The pieces whose region is no chamber: the off count read from the shape.
    pub off_regions: usize,
    /// The pieces whose pose is off the lattice (pose level); note N1, B.4 makes it equal `off_regions`.
    pub off_poses: usize,
    /// Regions that two or more pieces occupy; only a raw table collides (KN2.14).
    pub collisions: usize,
}

impl Shape {
    /// The SHA-256 of the lines "key TAB count LF" in key order.
    pub fn digest(&self) -> String {
        let mut out = String::new();
        for (key, n) in &self.regions {
            out.push_str(&key.text());
            out.push('\t');
            out.push_str(&n.to_string());
            out.push('\n');
        }
        c600_digest::sha256_hex(out.as_bytes())
    }

    /// The summary fields: the number of occupied regions, the off count read from the regions and from the poses,
    /// and the regions with more than one piece.
    fn fields(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("regions", int(self.regions.len())),
            ("off", int(self.off_regions)),
            ("off_poses", int(self.off_poses)),
            ("collisions", int(self.collisions)),
        ]
    }

    pub fn to_json(&self) -> Value {
        let mut fields = vec![("digest", text(&self.digest()))];
        fields.extend(self.fields());
        object(fields)
    }
}

/// The shape of a state.
pub fn shape(ctx: &Context, state: &State) -> Result<Shape, EngineError> {
    let t = tables(ctx)?;
    let keys = region_keys(ctx, &t, state)?;
    let mut counts: BTreeMap<RegionKey, u32> = BTreeMap::new();
    for key in keys {
        *counts.entry(key).or_default() += 1;
    }
    let off_regions = counts.iter().filter(|(k, _)| !k.is_chamber()).map(|(_, &n)| n as usize).sum();
    let collisions = counts.values().filter(|&&n| n > 1).count();
    Ok(Shape { regions: counts.into_iter().collect(), off_regions, off_poses: state.off_lattice_count(), collisions })
}

/// A value of one level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LevelValue {
    /// The pose digest of the state (the state of record).
    Poses {
        digest: String,
    },
    Labels(Labels),
    Position(Positions),
    /// Every piece's posed region key, by piece.
    RegionClass(Vec<RegionKey>),
    /// Every piece's JI.53 domain ID, by piece.
    Coset(Vec<Arc<str>>),
    /// Every piece's tree heights and branches (dl, dr, bl, br), by piece.
    TreeData(Vec<[i64; 4]>),
    Shape(Shape),
}

impl LevelValue {
    pub fn digest(&self) -> String {
        match self {
            LevelValue::Poses { digest } => digest.clone(),
            LevelValue::Labels(labels) => labels.digest(),
            LevelValue::Position(positions) => positions.digest(),
            LevelValue::RegionClass(keys) => {
                let lines: Vec<String> = keys.iter().map(RegionKey::text).collect();
                c600_digest::sha256_hex(lines.join("\n").as_bytes())
            }
            LevelValue::Coset(domains) => {
                let lines: Vec<&str> = domains.iter().map(|d| &**d).collect();
                c600_digest::sha256_hex(lines.join("\n").as_bytes())
            }
            LevelValue::TreeData(rows) => {
                let bytes: Vec<u8> = rows.iter().flat_map(|r| r.iter().flat_map(|x| x.to_le_bytes())).collect();
                c600_digest::sha256_hex(&bytes)
            }
            LevelValue::Shape(shape) => shape.digest(),
        }
    }

    /// The value's digest and summary; with `values`, the label or chamber arrays as base64 little-endian int32.
    pub fn to_json(&self, values: bool) -> Value {
        let mut fields = vec![("digest", text(&self.digest()))];
        let b64_u32 = |v: &[u32]| text(&b64::encode(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()));
        match self {
            LevelValue::Poses { .. } => {}
            LevelValue::Labels(labels) => {
                fields.push(("slots", int(labels.slots.len())));
                fields.push(("displaced", int(labels.displaced())));
                if values {
                    fields.push(("slot_ids", b64_u32(&labels.slots)));
                    let raw: Vec<u8> = labels.labels.iter().flat_map(|x| x.to_le_bytes()).collect();
                    fields.push(("labels", text(&b64::encode(&raw))));
                }
            }
            LevelValue::Position(positions) => {
                fields.push(("pieces", int(positions.pieces.len())));
                let home = positions.pieces.iter().zip(&positions.chambers).filter(|(p, c)| p == c).count();
                fields.push(("home", int(home)));
                if values {
                    fields.push(("piece_ids", b64_u32(&positions.pieces)));
                    fields.push(("chambers", b64_u32(&positions.chambers)));
                }
            }
            LevelValue::RegionClass(keys) => {
                fields.push(("chambers", int(keys.iter().filter(|k| k.is_chamber()).count())));
                fields.push(("off", int(keys.iter().filter(|k| !k.is_chamber()).count())));
            }
            LevelValue::Coset(domains) => {
                let mut counts: BTreeMap<String, Value> = BTreeMap::new();
                for d in domains {
                    let n = counts.get(&**d).and_then(Value::as_i64).unwrap_or(0);
                    counts.insert(d.to_string(), Value::Int(n + 1));
                }
                fields.push(("domains", Value::Object(counts)));
            }
            LevelValue::TreeData(rows) => {
                let mut counts: BTreeMap<String, Value> = BTreeMap::new();
                for r in rows {
                    let key = format!("{},{}", r[0], r[1]);
                    let n = counts.get(&key).and_then(Value::as_i64).unwrap_or(0);
                    counts.insert(key, Value::Int(n + 1));
                }
                fields.push(("pieces_by_heights", Value::Object(counts)));
            }
            LevelValue::Shape(shape) => fields.extend(shape.fields()),
        }
        object(fields)
    }
}

/// The answer of [`project`]: a typed partial value, or the refusal of a level that is not held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Projection {
    Value(Partial<LevelValue>),
    Refused(Refusal),
}

impl Projection {
    /// `{level, state, result, value, missing}`, or the refusal's code and message.
    pub fn to_json(&self, level: Level, state_digest: &str, values: bool) -> Value {
        let head = vec![("level", text(level.name())), ("state", text(state_digest))];
        let mut fields = head;
        match self {
            Projection::Refused(refusal) => {
                fields.push(("result", text("refused")));
                fields.push(("code", text(refusal.code)));
                fields.push(("message", text(&refusal.message)));
            }
            Projection::Value(Partial::Total(v)) => {
                fields.push(("result", text("total")));
                fields.push(("value", v.to_json(values)));
            }
            Projection::Value(Partial::Partial { covered, missing }) => {
                fields.push(("result", text("partial")));
                fields.push(("value", covered.to_json(values)));
                fields.push(("missing", missing.to_json()));
            }
            Projection::Value(Partial::Undefined(reason)) => {
                fields.push(("result", text("undefined")));
                fields.push(("reason", text(reason)));
            }
        }
        object(fields)
    }
}

/// Where each sticker sits: for home slot s, the slot it occupies now when its piece is on the lattice (A3). Every
/// sticker frame is checked against the slot layout; a disagreement is an engine error.
fn sticker_slots(ctx: &Context, t: &Tables, state: &State) -> Result<Vec<Option<u32>>, EngineError> {
    let mut dest = vec![None; NS];
    for p in 0..NP {
        let Some(k) = state.pose_of(p).kidx() else { continue };
        t.place(ctx, p, k, |s, d| dest[s as usize] = Some(d))?;
    }
    Ok(dest)
}

/// π(X) (A3): total on a collision-free lattice configuration, partial otherwise.
pub fn labels(ctx: &Context, state: &State) -> Result<Partial<Labels>, EngineError> {
    let t = tables(ctx)?;
    let dest = sticker_slots(ctx, &t, state)?;
    let mut claims = vec![0u8; NS];
    let mut label = vec![0i32; NS];
    for (s, d) in dest.iter().enumerate() {
        if let Some(d) = d {
            claims[*d as usize] = claims[*d as usize].saturating_add(1);
            label[*d as usize] = s as i32;
        }
    }
    let off = state.off_lattice_count();
    let collisions = claims.iter().filter(|&&c| c > 1).count();
    let uncovered = claims.iter().filter(|&&c| c == 0).count();
    if off == 0 && collisions == 0 && uncovered == 0 {
        return Ok(Partial::Total(Labels { slots: (0..NS as u32).collect(), labels: label }));
    }
    let slots: Vec<u32> = (0..NS as u32).filter(|&s| claims[s as usize] == 1).collect();
    let labels = slots.iter().map(|&s| label[s as usize]).collect();
    Ok(Partial::Partial { covered: Labels { slots, labels }, missing: Missing { off, uncovered, collisions } })
}

/// Each piece's chamber (KN1.7's position): total on a collision-free lattice configuration, partial otherwise.
pub fn positions(ctx: &Context, state: &State) -> Result<Partial<Positions>, EngineError> {
    let t = tables(ctx)?;
    let mut chamber = vec![None; NP];
    let mut claims = vec![0u8; NP];
    for (p, slot) in chamber.iter_mut().enumerate() {
        if let Some(k) = state.pose_of(p).kidx() {
            let c = t.image(ctx, p, k);
            claims[c as usize] = claims[c as usize].saturating_add(1);
            *slot = Some(c);
        }
    }
    let off = state.off_lattice_count();
    let collisions = claims.iter().filter(|&&c| c > 1).count();
    let uncovered = claims.iter().filter(|&&c| c == 0).count();
    let (mut pieces, mut chambers) = (Vec::new(), Vec::new());
    for (p, c) in chamber.iter().enumerate() {
        if let Some(c) = *c {
            if claims[c as usize] == 1 {
                pieces.push(p as u32);
                chambers.push(c);
            }
        }
    }
    let value = Positions { pieces, chambers };
    if off == 0 && collisions == 0 {
        return Ok(Partial::Total(value));
    }
    Ok(Partial::Partial { covered: value, missing: Missing { off, uncovered, collisions } })
}

/// KN1.9: the projection of a state to a level. A level whose value is undefined off the lattice gives a typed
/// partial result naming the pieces off; the sticker class, which has no key in this edition, is refused.
pub fn project(ctx: &Context, state: &State, to: Level) -> Result<Projection, EngineError> {
    let value = match to {
        Level::Poses => Partial::Total(LevelValue::Poses { digest: state.digest(ctx) }),
        Level::StickerClass => {
            return Ok(Projection::Refused(Refusal {
                code: LEVEL_NOT_HELD,
                message: "The sticker class has no key in this edition (KN1.8)".into(),
                fact: None,
            }))
        }
        Level::Labels => match labels(ctx, state)? {
            Partial::Total(v) => Partial::Total(LevelValue::Labels(v)),
            Partial::Partial { covered, missing } => Partial::Partial { covered: LevelValue::Labels(covered), missing },
            Partial::Undefined(reason) => Partial::Undefined(reason),
        },
        Level::Position => match positions(ctx, state)? {
            Partial::Total(v) => Partial::Total(LevelValue::Position(v)),
            Partial::Partial { covered, missing } => {
                Partial::Partial { covered: LevelValue::Position(covered), missing }
            }
            Partial::Undefined(reason) => Partial::Undefined(reason),
        },
        Level::RegionClass => {
            let t = tables(ctx)?;
            Partial::Total(LevelValue::RegionClass(region_keys(ctx, &t, state)?))
        }
        Level::Coset | Level::TreeData => {
            // The coset g_A·K⁺ and its tree data are functions of the pose alone: one computation per used pose.
            let obs = observe::tables(ctx).map_err(EngineError)?;
            let mut facts: Vec<Option<Arc<observe::Facts>>> = vec![None; state.poses().len()];
            for &id in state.pose_ids() {
                if facts[id as usize].is_none() {
                    facts[id as usize] = Some(obs.facts(ctx, &state.poses()[id as usize]).map_err(EngineError)?);
                }
            }
            let of = |id: u32| facts[id as usize].as_ref().expect("every used pose has its facts");
            let ids = state.pose_ids();
            if to == Level::Coset {
                Partial::Total(LevelValue::Coset(ids.iter().map(|&id| Arc::from(of(id).domain.as_str())).collect()))
            } else {
                let rows = ids.iter().map(|&id| of(id)).map(|f| [f.dl, f.dr, f.bl, f.br]).collect();
                Partial::Total(LevelValue::TreeData(rows))
            }
        }
        Level::Shape => Partial::Total(LevelValue::Shape(shape(ctx, state)?)),
    };
    Ok(Projection::Value(value))
}

// ---------------------------------------------------------------------------------------------------- verdicts

/// A verdict of KN0.5: yes or no with evidence, or unknown with a reason. Unknown is never false.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Yes(Value),
    No(Value),
    Unknown(String),
}

/// The level each end test reads, which its verdict carries (KN1.11, JI.4): the sticker test and the label checkpoint
/// read the labels, the retained checkpoint the poses.
pub const STICKER_TEST_LEVEL: Level = Level::Labels;
pub const RETAINED_CHECKPOINT_LEVEL: Level = Level::Poses;
pub const LABEL_CHECKPOINT_LEVEL: Level = Level::Labels;

impl Verdict {
    /// The verdict with the level it was reached at, `{level, verdict, evidence or reason}`: a verdict carries its
    /// level and never crosses over (KN1.11), and a read names its level (JI.4).
    pub fn to_json_at(&self, level: Level) -> Value {
        let mut fields = vec![("level", text(level.name())), ("verdict", text(self.name()))];
        match self {
            Verdict::Yes(evidence) | Verdict::No(evidence) => fields.push(("evidence", evidence.clone())),
            Verdict::Unknown(reason) => fields.push(("reason", text(reason))),
        }
        object(fields)
    }

    pub fn to_json(&self) -> Value {
        match self {
            Verdict::Yes(evidence) => object(vec![("verdict", text("yes")), ("evidence", evidence.clone())]),
            Verdict::No(evidence) => object(vec![("verdict", text("no")), ("evidence", evidence.clone())]),
            Verdict::Unknown(reason) => object(vec![("verdict", text("unknown")), ("reason", text(reason))]),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Verdict::Yes(_) => "yes",
            Verdict::No(_) => "no",
            Verdict::Unknown(_) => "unknown",
        }
    }
}

fn fact(name: &str, more: Vec<(&str, Value)>) -> Value {
    let mut fields = vec![("fact", text(name))];
    fields.extend(more);
    object(fields)
}

/// The centre pieces outside their own chamber, on a lattice configuration.
pub fn displaced_centres(ctx: &Context, state: &State) -> Result<Vec<u32>, EngineError> {
    let t = tables(ctx)?;
    let mut out = Vec::new();
    for &(piece, pole) in t.centres() {
        let k = state.pose_of(piece as usize).kidx().ok_or_else(|| engine("a centre piece is off the lattice"))?;
        if ctx.kplus.perms[k as usize][pole as usize] != pole {
            out.push(piece);
        }
    }
    Ok(out)
}

/// The facts that make any checkpoint impossible: pieces off the lattice, or a centre outside its own chamber.
fn not_a_checkpoint(ctx: &Context, state: &State) -> Result<Option<Verdict>, EngineError> {
    let off = state.off_lattice_count();
    if off > 0 {
        return Ok(Some(Verdict::No(fact("off_lattice", vec![("off", int(off))]))));
    }
    let displaced = displaced_centres(ctx, state)?;
    if !displaced.is_empty() {
        let ids = Value::Array(displaced.iter().map(|&p| int(p as usize)).collect());
        return Ok(Some(Verdict::No(fact("centre_displaced", vec![("centres", ids)]))));
    }
    Ok(None)
}

/// A5's end of a solve: N = 0 and every slot carries its home label. No reachability claim is made (KN1.17).
pub fn sticker_test(ctx: &Context, state: &State) -> Result<Verdict, EngineError> {
    let off = state.off_lattice_count();
    if off > 0 {
        return Ok(Verdict::No(fact("off_lattice", vec![("off", int(off))])));
    }
    match labels(ctx, state)? {
        Partial::Total(labels) => {
            let displaced = labels.displaced();
            if displaced > 0 {
                return Ok(Verdict::No(fact("labels_differ", vec![("slots", int(displaced))])));
            }
            Ok(Verdict::Yes(object(vec![
                ("off", int(0)),
                ("pose_digest", text(&state.digest(ctx))),
                ("labels_digest", text(&labels.digest())),
            ])))
        }
        Partial::Partial { missing, .. } => {
            Ok(Verdict::No(fact("labels_differ", vec![("collisions", int(missing.collisions))])))
        }
        Partial::Undefined(reason) => Ok(Verdict::Unknown(reason)),
    }
}

/// The retained twist of a signed 1-based primitive id (generator k is id k + 1), as J1's `primitive`.
pub fn primitive(ctx: &Context, mid: i64) -> Result<Twist, EngineError> {
    let kp = &ctx.kplus;
    let g = usize::try_from(mid.unsigned_abs())
        .ok()
        .and_then(|a| a.checked_sub(1))
        .filter(|&g| g < kp.gen_idx.len())
        .ok_or_else(|| engine(format!("primitive id {mid} is outside 1..1200 in absolute value")))?;
    let mut k = kp.gen_idx[g];
    if mid < 0 {
        k = kp.inv[k as usize];
    }
    let grip = g / 2;
    if kp.perms[k as usize][grip] as usize != grip {
        return Err(engine(format!("primitive {mid} does not fix the pole of grip {grip}")));
    }
    Ok(Twist { grip, matrix: Arc::new(kp.matrix(k, &ctx.model).clone()), kidx: Some(k) })
}

/// J1's journal witness: consecutive twists of one grip merge, (c, g₁)(c, g₂) = (c, g₂g₁), identity twists drop, and
/// what is left must lie in K⁺; the A4 words of those twists form the witness. `Err` names why there is none.
pub fn journal_word(ctx: &Context, journal: &[Twist]) -> Result<Vec<i64>, String> {
    let mut stack: Vec<(usize, Mat4)> = Vec::new();
    for twist in journal {
        let mut m = (*twist.matrix).clone();
        if stack.last().is_some_and(|(c, _)| *c == twist.grip) {
            let (_, prev) = stack.pop().expect("the stack has a last entry");
            m = matmul(&m, &prev);
        }
        if ctx.kplus.index_of_matrix(&m, &ctx.model) != Some(0) {
            stack.push((twist.grip, m));
        }
    }
    let mut word = Vec::new();
    for (c, m) in &stack {
        let k = ctx
            .kplus
            .index_of_matrix(m, &ctx.model)
            .ok_or("the journal keeps jumble twists that do not cancel, so it gives no retained word")?;
        word.extend(ctx.kplus.a4_word(*c, k).ok_or("a merged twist does not lie in A4 of its grip")?);
    }
    Ok(word)
}

/// `Digest<literal word>` of a jumbling word (a draft: the frozen text fixes the convention for 600-cell-Full's
/// primitive letters only): the SHA-256 over every letter's grip as little-endian u32, its compact exact matrix text
/// and a line feed.
pub fn word_digest(twists: &[Twist]) -> String {
    let mut bytes = Vec::new();
    for twist in twists {
        bytes.extend_from_slice(&(twist.grip as u32).to_le_bytes());
        bytes.extend_from_slice(crate::q5::matrix_compact(&twist.matrix).as_bytes());
        bytes.push(b'\n');
    }
    c600_digest::sha256_hex(&bytes)
}

/// Replays twists from solved, each admissible as recorded; `Err` names the first step that is not.
pub fn replay(
    ctx: &Context,
    menu: Option<Arc<Menu>>,
    twists: &[Twist],
    moved: Option<&[Option<usize>]>,
) -> Result<Result<State, String>, EngineError> {
    let mut state = State::new(menu);
    for (index, twist) in twists.iter().enumerate() {
        let outcome = state.apply(ctx, twist, None, &[])?;
        if outcome.status != Status::Admissible {
            return Ok(Err(format!("step {index} of the witness is {}", outcome.status.as_str())));
        }
        if let Some(Some(expected)) = moved.and_then(|m| m.get(index)) {
            if outcome.moved() != *expected {
                return Ok(Err(format!(
                    "step {index} of the witness moved {} pieces, not {expected}",
                    outcome.moved()
                )));
            }
        }
    }
    Ok(Ok(state))
}

/// What a retained checkpoint is shown from: a word of signed 1-based primitive ids, or the journal from solved.
pub enum Witness<'a> {
    Word(&'a [i64]),
    Journal(&'a [Twist]),
}

/// Section 5's retained-state checkpoint, with J1's `State.checkpoint` as the reference.
pub fn retained_checkpoint(ctx: &Context, state: &State, witness: Witness) -> Result<Verdict, EngineError> {
    if let Some(no) = not_a_checkpoint(ctx, state)? {
        return Ok(no);
    }
    let (word, source) = match witness {
        Witness::Word(word) => (word.to_vec(), "supplied"),
        Witness::Journal(journal) => match journal_word(ctx, journal) {
            Ok(word) => (word, "journal"),
            Err(reason) => return Ok(Verdict::Unknown(reason)),
        },
    };
    let twists = word.iter().map(|&mid| primitive(ctx, mid)).collect::<Result<Vec<_>, _>>()?;
    let reached = match replay(ctx, None, &twists, None)? {
        Ok(reached) => reached,
        Err(reason) => return Ok(Verdict::Unknown(reason)),
    };
    let digest = state.digest(ctx);
    if reached.digest(ctx) != digest {
        return Ok(Verdict::Unknown("the witness reaches another configuration".into()));
    }
    Ok(Verdict::Yes(object(vec![
        ("source", text(source)),
        ("witness_word", Value::Array(word.iter().map(|&m| Value::Int(m)).collect())),
        ("pose_digest", text(&digest)),
    ])))
}

// ---------------------------------------------------------------------------------------------------- reachability

/// The five values of a reachability claim (KN2.13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReachKind {
    Witnessed,
    ByTheorem,
    Promised,
    Refuted,
    Open,
}

impl ReachKind {
    pub fn name(self) -> &'static str {
        match self {
            ReachKind::Witnessed => "witnessed",
            ReachKind::ByTheorem => "by_theorem",
            ReachKind::Promised => "promised",
            ReachKind::Refuted => "refuted",
            ReachKind::Open => "open",
        }
    }

    pub fn parse(name: &str) -> Option<ReachKind> {
        [ReachKind::Witnessed, ReachKind::ByTheorem, ReachKind::Promised, ReachKind::Refuted, ReachKind::Open]
            .into_iter()
            .find(|k| k.name() == name)
    }
}

/// A reachability claim in the jumbling puzzle (KN2.13). The evidence is an envelope reference (chapter 06), passed
/// through as given; this crate never searches for a witness. A witnessed claim is made only by a replay from solved
/// ([`witness`]) or by a derivation from a witnessed base ([`derive`]), and it speaks only of the state it reached.
#[derive(Clone, Debug, PartialEq)]
pub struct Reachability {
    kind: ReachKind,
    evidence: Value,
    /// Witnessed only: every twist of the witness lies in K⁺, so it is a retained word and keeps ν = 0 (card 1, F1).
    retained: bool,
    /// The pose digest of the state that a claim this crate made speaks of; a stated claim names none. A witnessed
    /// claim holds for that state only (A4, A5).
    at: Option<String>,
}

impl Reachability {
    /// The solved state, witnessed by the empty word; the claim speaks of the solved state only.
    pub fn solved(ctx: &Context) -> Reachability {
        Reachability {
            kind: ReachKind::Witnessed,
            evidence: object(vec![("from", text("solved")), ("word", text(&word_digest(&[]))), ("letters", int(0))]),
            retained: true,
            at: Some(State::new(None).digest(ctx)),
        }
    }

    /// A claim the caller states: by theorem, promised, refuted or open, with its evidence reference. A witnessed claim
    /// cannot be stated; it is replayed ([`witness`]) or derived.
    pub fn stated(kind: ReachKind, evidence: Value) -> Option<Reachability> {
        (kind != ReachKind::Witnessed).then_some(Reachability { kind, evidence, retained: false, at: None })
    }

    pub fn kind(&self) -> ReachKind {
        self.kind
    }

    pub fn evidence(&self) -> &Value {
        &self.evidence
    }

    /// The pose digest of the state the claim speaks of, or `None` for a stated claim.
    pub fn at(&self) -> Option<&str> {
        self.at.as_deref()
    }

    /// Whether the claim is witnessed by a retained word.
    pub fn retained(&self) -> bool {
        self.kind == ReachKind::Witnessed && self.retained
    }

    /// The claim of the state `reached` that an admissible word reaches from the state `base_digest`, which has this
    /// claim. Reachability moves along admissible twists both ways (an admissible twist's inverse is admissible from
    /// its result), so every value carries over; a witnessed claim records the base and the word. Only a replay
    /// ([`witness`]) and a derivation ([`derive`]) call it, once they have reached `reached`.
    pub(crate) fn then(&self, base_digest: &str, twists: &[Twist], reached: &str) -> Reachability {
        let evidence = object(vec![
            ("base", text(base_digest)),
            ("base_claim", self.to_json()),
            ("word", text(&word_digest(twists))),
            ("letters", int(twists.len())),
        ]);
        let retained = self.retained && twists.iter().all(|t| t.kidx.is_some());
        Reachability { kind: self.kind, evidence, retained, at: Some(reached.to_string()) }
    }

    pub fn to_json(&self) -> Value {
        object(vec![
            ("puzzle", text("jumbling")),
            ("kind", text(self.kind.name())),
            ("evidence", self.evidence.clone()),
            ("retained", Value::Bool(self.retained())),
            ("at", self.at.as_deref().map_or(Value::Null, text)),
        ])
    }
}

/// Witnessed reachability of `state` from a journal of twists: the journal is replayed exactly from solved under
/// `menu`, each step admissible (with its recorded moved count when given), and must reach `state`'s pose digest.
pub fn witness(
    ctx: &Context,
    state: &State,
    menu: Option<Arc<Menu>>,
    journal: &[Twist],
    moved: Option<&[Option<usize>]>,
) -> Result<Result<Reachability, String>, EngineError> {
    let reached = match replay(ctx, menu, journal, moved)? {
        Ok(reached) => reached,
        Err(reason) => return Ok(Err(reason)),
    };
    let digest = state.digest(ctx);
    if reached.digest(ctx) != digest {
        return Ok(Err("the journal reaches another configuration".into()));
    }
    Ok(Ok(Reachability::solved(ctx).then(&State::new(None).digest(ctx), journal, &digest)))
}

/// A5's label checkpoint. `nu` is the ν of the state as the ν row computes it, when the caller has it (45 values).
///
/// The clauses that do not depend on reachability come first: off the lattice, a displaced centre, two pieces in one
/// chamber or a known nonzero ν answer no whatever the claim. Then only a witnessed claim decides (ONE.28): open,
/// promised, by theorem and refuted answer unknown, the last two because this edition checks no theorem evidence. A
/// witnessed claim speaks only of the state it reached (A4, A5): of another state it answers unknown, and its retained
/// witness shows no ν there. A witnessed claim with ν not shown answers unknown too. A supplied ν that differs from
/// the ν = 0 that the identity labels or a retained witness show is an engine error: the ν row and this one disagree.
/// A yes carries the engine's contract stamp ([`crate::CONTRACT_REVISION`]).
pub fn label_checkpoint(
    ctx: &Context,
    state: &State,
    reach: &Reachability,
    nu: Option<&[i64]>,
) -> Result<Verdict, EngineError> {
    if let Some(no) = not_a_checkpoint(ctx, state)? {
        return Ok(no);
    }
    let labels = match labels(ctx, state)? {
        Partial::Total(labels) => labels,
        // Every piece is on the lattice, so a partial labelling has two pieces in one chamber, which no word reaches;
        // KN2.14's validation does not test for it.
        Partial::Partial { missing, .. } => {
            return Ok(Verdict::No(fact("pieces_collide", vec![("collisions", int(missing.collisions))])))
        }
        Partial::Undefined(reason) => return Ok(Verdict::Unknown(reason)),
    };
    if let Some(nu) = nu {
        if nu.len() != NU_LEN {
            return Err(engine(format!("ν has {NU_LEN} coordinates, not {}", nu.len())));
        }
    }
    let digest = state.digest(ctx);
    let holds_here = reach.kind != ReachKind::Witnessed || reach.at.as_deref() == Some(digest.as_str());
    let shown = if labels.displaced() == 0 {
        Some((vec![0i64; NU_LEN], "identity_labels"))
    } else if holds_here && reach.retained() {
        Some((vec![0i64; NU_LEN], "retained_witness"))
    } else {
        nu.map(|nu| (nu.to_vec(), "supplied"))
    };
    if let (Some(supplied), Some((value, source))) = (nu, &shown) {
        if supplied != value.as_slice() {
            return Err(engine(format!("the supplied ν differs from the ν that {source} show")));
        }
    }
    let nu_json = |value: &[i64]| Value::Array(value.iter().map(|&x| Value::Int(x)).collect());
    if let Some((value, source)) = &shown {
        if value.iter().any(|&x| x != 0) {
            return Ok(Verdict::No(fact("nu_nonzero", vec![("nu", nu_json(value)), ("nu_source", text(source))])));
        }
    }
    match reach.kind {
        ReachKind::Witnessed if holds_here => {}
        ReachKind::Witnessed => return Ok(Verdict::Unknown("the witnessed claim speaks of another state".into())),
        ReachKind::Open => return Ok(Verdict::Unknown("reachability is open".into())),
        ReachKind::Promised => return Ok(Verdict::Unknown("reachability is promised, not shown".into())),
        ReachKind::ByTheorem | ReachKind::Refuted => {
            return Ok(Verdict::Unknown(format!(
                "a {} reachability claim is not checked in this edition",
                reach.kind.name()
            )))
        }
    }
    let Some((value, source)) = shown else {
        return Ok(Verdict::Unknown("ν is not shown: the ν row computes it".into()));
    };
    Ok(Verdict::Yes(object(vec![
        ("pose_digest", text(&digest)),
        ("labels_digest", text(&labels.digest())),
        ("frame_agrees", Value::Bool(true)),
        ("nu", nu_json(&value)),
        ("nu_source", text(source)),
        ("reachability", reach.to_json()),
        ("contract_revision", text(crate::CONTRACT_REVISION)),
    ])))
}

/// The record of a state built from per-piece poses (a raw table, or a gauge image): the identity first, then each
/// new pose in piece order. `State::from_record` reads it and validates form and identity only (KN2.14).
pub fn record_of(poses: &[Pose]) -> Value {
    let mut table: Vec<Pose> = vec![Pose::K(0)];
    let mut index: HashMap<Pose, u32> = HashMap::from([(Pose::K(0), 0)]);
    let mut ids = Vec::with_capacity(NP * 4);
    for pose in poses {
        let id = *index.entry(pose.clone()).or_insert_with(|| {
            table.push(pose.clone());
            (table.len() - 1) as u32
        });
        ids.extend_from_slice(&id.to_le_bytes());
    }
    let entries = table.iter().map(pose_json).collect();
    json::object([
        ("pose_table", Value::Array(entries)),
        (
            "pose_ids",
            json::object([
                ("encoding", text("u32le-base64")),
                ("count", int(NP)),
                ("sha256", text(&c600_digest::sha256_hex(&ids))),
                ("data", text(&b64::encode(&ids))),
            ]),
        ),
    ])
}

/// A state from per-piece poses, read back through `State::from_record` (form and identity only, KN2.14).
pub fn state_of(ctx: &Context, menu: Option<Arc<Menu>>, poses: &[Pose]) -> Result<State, String> {
    if poses.len() != NP {
        return Err(format!("a pose table names {NP} pieces"));
    }
    State::from_record(ctx, menu, &record_of(poses))
}

// ------------------------------------------------------------------------------------------------------ gauges

/// The three gauge groups of KN1.7, finest first: 𝒢_S keeps the sticker class, 𝒢_R the region class and 𝒢 the shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GaugeClass {
    /// σ = id and every t_A in S_A.
    S,
    /// σ = id and every t_A in R_A.
    R,
    /// Any gauge.
    All,
}

impl GaugeClass {
    pub fn name(self) -> &'static str {
        match self {
            GaugeClass::S => "S",
            GaugeClass::R => "R",
            GaugeClass::All => "all",
        }
    }
}

/// A gauge γ = (σ, t) of KN1.7: σ permutes the pieces, and t_A carries the home region of A onto the home region of σA.
/// Such a rotation carries a chamber onto a chamber, so it lies in K⁺ (note N1, B.4: a pose is off the lattice exactly
/// when its region is no chamber), and t_A is kept as its K⁺ index. γ acts on the right ([`act`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gauge {
    sigma: Vec<u32>,
    t: Vec<u16>,
}

impl Gauge {
    pub fn identity() -> Gauge {
        Gauge { sigma: (0..NP as u32).collect(), t: vec![0; NP] }
    }

    /// The gauge with σ and t, each listing every piece. `Err` names the first piece at which σ is no permutation, or
    /// t_A does not carry A's home region onto σA's.
    pub fn new(ctx: &Context, sigma: Vec<u32>, t: Vec<u16>) -> Result<Gauge, String> {
        if sigma.len() != NP || t.len() != NP {
            return Err(format!("a gauge lists all {NP} pieces in σ and in t"));
        }
        let tables = tables(ctx).map_err(|e| e.0)?;
        let mut taken = vec![false; NP];
        for (a, (&b, &k)) in sigma.iter().zip(&t).enumerate() {
            if b as usize >= NP || std::mem::replace(&mut taken[b as usize], true) {
                return Err(format!("σ is no permutation of the pieces: see piece {a}"));
            }
            if k as usize >= ORDER {
                return Err(format!("t of piece {a} is no K+ index"));
            }
            if tables.image(ctx, a, k) != b {
                return Err(format!("t of piece {a} does not carry its home region onto the home region of piece {b}"));
            }
        }
        Ok(Gauge { sigma, t })
    }

    pub fn sigma(&self) -> &[u32] {
        &self.sigma
    }

    pub fn t(&self) -> &[u16] {
        &self.t
    }

    /// The pieces with σA ≠ A.
    pub fn moved(&self) -> usize {
        self.sigma.iter().enumerate().filter(|(a, &b)| *a != b as usize).count()
    }

    /// The pieces with t_A ≠ id.
    pub fn turned(&self) -> usize {
        self.t.iter().filter(|&&k| k != 0).count()
    }

    /// The finest group the gauge lies in. With σ = id every t_A lies in R_A; it lies in S_A when it fixes the cell of
    /// each sticker of A, because a rotation of R_A that fixes a sticker's cell carries that sticker region onto
    /// itself.
    pub fn class(&self, ctx: &Context) -> Result<GaugeClass, EngineError> {
        if self.moved() > 0 {
            return Ok(GaugeClass::All);
        }
        let t = tables(ctx)?;
        let perms = &ctx.kplus.perms;
        for (a, &k) in self.t.iter().enumerate() {
            let fixes = |s: &u32| {
                let f = *s as usize / CELL_SLOTS;
                perms[k as usize][f] as usize == f
            };
            if k != 0 && !t.slots(a).iter().all(fixes) {
                return Ok(GaugeClass::R);
            }
        }
        Ok(GaugeClass::S)
    }

    /// `{class, moved, turned}`; with `values`, also σ and t as base64 little-endian u32 arrays, as a state record's
    /// pose IDs.
    pub fn to_json(&self, ctx: &Context, values: bool) -> Result<Value, EngineError> {
        fn raw(values: impl Iterator<Item = u32>) -> Value {
            text(&b64::encode(&values.flat_map(u32::to_le_bytes).collect::<Vec<u8>>()))
        }
        let mut fields = vec![
            ("class", text(self.class(ctx)?.name())),
            ("moved", int(self.moved())),
            ("turned", int(self.turned())),
        ];
        if values {
            fields.push(("sigma", raw(self.sigma.iter().copied())));
            fields.push(("t", raw(self.t.iter().map(|&k| k as u32))));
        }
        Ok(object(fields))
    }
}

/// R_A of KN1.7 for piece a, ascending: the rotations that carry A's home region onto itself, which lie in K⁺ (note N1,
/// B.4). With t the transport of A's orbit representative, R_A = t·R_rep·t⁻¹.
pub fn region_stabiliser(ctx: &Context, a: usize) -> Result<Vec<u16>, EngineError> {
    if a >= NP {
        return Err(engine(format!("piece {a} is outside 0..{NP}")));
    }
    let t = tables(ctx)?;
    let kp = &ctx.kplus;
    let (o, tr) = ctx.regions.rep_of(a);
    let row = &t.piece_of[o * ORDER..(o + 1) * ORDER];
    let back = kp.inv[tr as usize];
    let mut out: Vec<u16> =
        (0..ORDER).filter(|&s| row[s] == row[0]).map(|s| kp.compose(tr, kp.compose(s as u16, back))).collect();
    out.sort_unstable();
    if out.iter().any(|&k| t.image(ctx, a, k) != a as u32) {
        return Err(engine(format!("a conjugated stabiliser element moves piece {a}")));
    }
    Ok(out)
}

/// S_A of KN1.7 for piece a, ascending: the elements of R_A that carry each sticker region of A onto itself, that is,
/// that fix the cell of each of its stickers.
pub fn sticker_stabiliser(ctx: &Context, a: usize) -> Result<Vec<u16>, EngineError> {
    let r = region_stabiliser(ctx, a)?;
    let t = tables(ctx)?;
    let cells: Vec<usize> = t.slots(a).iter().map(|&s| s as usize / CELL_SLOTS).collect();
    let perms = &ctx.kplus.perms;
    Ok(r.into_iter().filter(|&k| cells.iter().all(|&f| perms[k as usize][f] as usize == f)).collect())
}

/// X·γ of KN1.7: piece A takes the pose X[σA]·t_A, so it occupies the region that σA occupies in X. The image keeps X's
/// menu and is read back like a raw table (form and identity only, KN2.14).
pub fn act(ctx: &Context, x: &State, gauge: &Gauge) -> Result<State, EngineError> {
    let kp = &ctx.kplus;
    let mut products: HashMap<(u32, u16), Pose> = HashMap::new();
    let mut poses = Vec::with_capacity(NP);
    for (&b, &k) in gauge.sigma.iter().zip(&gauge.t) {
        let id = x.pose_ids()[b as usize];
        poses.push(match &x.poses()[id as usize] {
            Pose::K(g) if k == 0 => Pose::K(*g),
            Pose::K(g) => Pose::K(kp.compose(*g, k)),
            Pose::M(m) if k == 0 => Pose::M(m.clone()),
            Pose::M(m) => products
                .entry((id, k))
                .or_insert_with(|| Pose::M(Arc::new(matmul(m, kp.matrix(k, &ctx.model)))))
                .clone(),
        });
    }
    state_of(ctx, x.menu_arc(), &poses).map_err(|e| engine(format!("a gauge image does not read back: {e}")))
}

/// A gauge γ with X·γ = Y, or `None` when X and Y have different shapes; equal multisets of posed regions always give
/// one (KN1.7). σ matches the regions: piece A goes to the lowest piece, not taken yet, that occupies Y's region of A
/// in X. Then t_A = X[σA]⁻¹·Y[A]. On a collision-free configuration σ is the only match, so γ is the only gauge. The
/// menus are not read: a gauge relates configurations.
pub fn gauge_between(ctx: &Context, x: &State, y: &State) -> Result<Option<Gauge>, EngineError> {
    let t = tables(ctx)?;
    let (from, to) = (region_keys(ctx, &t, x)?, region_keys(ctx, &t, y)?);
    let mut holders: HashMap<RegionKey, Vec<u32>> = HashMap::new();
    for (p, key) in from.into_iter().enumerate().rev() {
        holders.entry(key).or_default().push(p as u32);
    }
    let mut sigma = Vec::with_capacity(NP);
    for key in &to {
        match holders.get_mut(key).and_then(Vec::pop) {
            Some(b) => sigma.push(b),
            None => return Ok(None),
        }
    }
    let kp = &ctx.kplus;
    let mut quotients: HashMap<(u32, u32), u16> = HashMap::new();
    let mut turns = Vec::with_capacity(NP);
    for (a, &b) in sigma.iter().enumerate() {
        let (ix, iy) = (x.pose_ids()[b as usize], y.pose_ids()[a]);
        let k = match (&x.poses()[ix as usize], &y.poses()[iy as usize]) {
            (Pose::K(g), Pose::K(h)) => kp.compose(kp.inv[*g as usize], *h),
            _ => match quotients.get(&(ix, iy)) {
                Some(&k) => k,
                None => {
                    let m = matmul(&transpose(x.pose_matrix(ctx, ix)), y.pose_matrix(ctx, iy));
                    let k = kp.index_of_matrix(&m, &ctx.model).ok_or_else(|| {
                        engine(format!(
                            "piece {b} of X and piece {a} of Y share a region, but their poses differ outside K+"
                        ))
                    })?;
                    quotients.insert((ix, iy), k);
                    k
                }
            },
        };
        turns.push(k);
    }
    Gauge::new(ctx, sigma, turns).map(Some).map_err(|e| engine(format!("the matched gauge is no gauge: {e}")))
}

// ---------------------------------------------------------------------------------------------- derived states

/// An identity block as JSON: the puzzle, the manifest's `model_id`, and the menu identity, the menu order and the
/// contract revision, each null when the block has none.
fn identity_json(block: &IdentityBlock) -> Value {
    let opt = |value: &Option<String>| value.as_deref().map_or(Value::Null, text);
    object(vec![
        ("puzzle", text(if block.puzzle == Puzzle::Full { "full" } else { "jumbling" })),
        ("model_id", text(&block.model_id)),
        ("menu_id", opt(&block.menu_id)),
        ("menu_order", opt(&block.menu_order)),
        ("contract_rev", opt(&block.contract_rev)),
    ])
}

/// A pose as a pose-table entry: `{k}` for a K⁺ pose, `{m}` with its exact matrix otherwise.
fn pose_json(pose: &Pose) -> Value {
    match pose {
        Pose::K(k) => json::object([("k", Value::Int(*k as i64))]),
        Pose::M(m) => json::object([("m", crate::q5::matrix_json(m))]),
    }
}

/// The pose a twist gives a piece that it moves, g·g_A, kept in K⁺ when it lies there (as `State::apply` commits it).
fn turned(ctx: &Context, twist: &Twist, pose: &Pose) -> Pose {
    if let (Some(k), Some(t)) = (pose.kidx(), twist.kidx) {
        return Pose::K(ctx.kplus.compose(t, k));
    }
    let g: &Mat4 = match pose {
        Pose::K(k) => ctx.kplus.matrix(*k, &ctx.model),
        Pose::M(m) => m.as_ref(),
    };
    let m = matmul(&twist.matrix, g);
    match ctx.kplus.index_of_matrix(&m, &ctx.model) {
        Some(k) => Pose::K(k),
        None => Pose::M(Arc::new(m)),
    }
}

/// The protection report of a derivation at pose level: KN2.4's `protection` row in the kernel's modes, with the
/// kernel's verdicts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordProtection {
    pub mode: ProtectionMode,
    /// The protected pieces, ascending.
    pub protected: Vec<u32>,
    /// The protected pieces whose pose the judged operation changes, ascending: the whole word under Net, the first
    /// refused prefix under Strict.
    pub pieces: Vec<u32>,
    pub verdict: ProtectionVerdict,
    /// Strict only: the length of the shortest prefix that changes a protected piece's pose (1 for the first letter).
    /// `None` for a clear verdict and under Net, which judges only the end of the whole word.
    pub first_refused_prefix: Option<u64>,
}

impl WordProtection {
    fn clear(mode: ProtectionMode, protected: Vec<u32>) -> WordProtection {
        WordProtection {
            mode,
            protected,
            pieces: Vec::new(),
            verdict: ProtectionVerdict::Clear,
            first_refused_prefix: None,
        }
    }

    pub fn to_json(&self) -> Value {
        let ids = |pieces: &[u32]| Value::Array(pieces.iter().map(|&p| int(p as usize)).collect());
        let mode = match self.mode {
            ProtectionMode::Net => "net",
            ProtectionMode::Strict => "strict",
        };
        let verdict = match self.verdict {
            ProtectionVerdict::Clear => "clear",
            ProtectionVerdict::Moves => "moves",
        };
        object(vec![
            ("level", text("poses")),
            ("mode", text(mode)),
            ("protected", ids(&self.protected)),
            ("pieces", ids(&self.pieces)),
            ("verdict", text(verdict)),
            ("first_refused_prefix", self.first_refused_prefix.map_or(Value::Null, |n| Value::Int(n as i64))),
        ])
    }
}

/// Where a derived value's history starts (ONE.21): a snapshot, named by its pose digest in this edition, or an
/// import, named by the imported state's pose digest. Neither names a revision: an import has none, and a snapshot's
/// revision comes from the session store (M6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lineage {
    Snapshot(String),
    Import(String),
}

impl Lineage {
    pub fn to_json(&self) -> Value {
        let (root, digest) = match self {
            Lineage::Snapshot(digest) => ("snapshot", digest),
            Lineage::Import(digest) => ("import", digest),
        };
        object(vec![("root", text(root)), ("digest", text(digest)), ("revision", Value::Null)])
    }
}

/// KN2.11's `derivation`: the base's pose digest and `Digest<literal word>` ([`word_digest`]), with the word's
/// length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedFrom {
    pub base: String,
    pub word: String,
    pub letters: usize,
}

impl DerivedFrom {
    pub fn to_json(&self) -> Value {
        object(vec![("base", text(&self.base)), ("word", text(&self.word)), ("letters", int(self.letters))])
    }
}

/// A base that the caller holds, with its reachability claim: solved, or a state that its journal from solved
/// witnesses. A derivation changes only a copy of it.
#[derive(Clone)]
pub struct Snapshot {
    state: State,
    reach: Reachability,
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Snapshot")
            .field("menu", &self.state.menu().map(|m| m.name.as_str()))
            .field("off", &self.state.off_lattice_count())
            .field("reach", &self.reach)
            .finish_non_exhaustive()
    }
}

impl Snapshot {
    /// The solved state under `menu` (`None` in free mode), witnessed by the empty word.
    pub fn solved(ctx: &Context, menu: Option<Arc<Menu>>) -> Snapshot {
        Snapshot { state: State::new(menu), reach: Reachability::solved(ctx) }
    }

    /// `state` with the witnessed claim of its journal from solved, replayed exactly under the state's own menu, with
    /// the recorded moved counts when given ([`witness`]); `Err` names why the journal does not witness the state.
    pub fn witnessed(
        ctx: &Context,
        state: State,
        journal: &[Twist],
        moved: Option<&[Option<usize>]>,
    ) -> Result<Result<Snapshot, String>, EngineError> {
        let menu = state.menu_arc();
        Ok(witness(ctx, &state, menu, journal, moved)?.map(|reach| Snapshot { state, reach }))
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn reachability(&self) -> &Reachability {
        &self.reach
    }
}

/// A derived value (KN2.11, KN2.12): a state with its identity block, its derivation (none for an import), its
/// lineage, its reachability claim and its protection report (none for an import). It has no revision and is never
/// adopted; nothing here writes it to a session.
#[derive(Clone)]
pub struct Derived {
    state: State,
    identity: IdentityBlock,
    derivation: Option<DerivedFrom>,
    lineage: Lineage,
    reach: Reachability,
    protection: Option<WordProtection>,
}

impl std::fmt::Debug for Derived {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Derived")
            .field("identity", &self.identity)
            .field("derivation", &self.derivation)
            .field("lineage", &self.lineage)
            .field("reach", &self.reach)
            .field("protection", &self.protection)
            .field("off", &self.state.off_lattice_count())
            .finish_non_exhaustive()
    }
}

impl Derived {
    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn identity(&self) -> &IdentityBlock {
        &self.identity
    }

    pub fn derivation(&self) -> Option<&DerivedFrom> {
        self.derivation.as_ref()
    }

    pub fn lineage(&self) -> &Lineage {
        &self.lineage
    }

    pub fn reachability(&self) -> &Reachability {
        &self.reach
    }

    pub fn protection(&self) -> Option<&WordProtection> {
        self.protection.as_ref()
    }

    /// The pose digest of the value's state: the digest of KN2.11's `DerivedRef`.
    pub fn digest(&self, ctx: &Context) -> String {
        self.state.digest(ctx)
    }

    /// `{result: "derived", ref: {digest, level, identity}, derivation, lineage, reachability, protection, off}`.
    pub fn to_json(&self, ctx: &Context) -> Value {
        let reference = object(vec![
            ("digest", text(&self.digest(ctx))),
            ("level", text(Level::Poses.name())),
            ("identity", identity_json(&self.identity)),
        ]);
        object(vec![
            ("result", text("derived")),
            ("ref", reference),
            ("derivation", self.derivation.as_ref().map_or(Value::Null, DerivedFrom::to_json)),
            ("lineage", self.lineage.to_json()),
            ("reachability", self.reach.to_json()),
            ("protection", self.protection.as_ref().map_or(Value::Null, WordProtection::to_json)),
            ("off", int(self.state.off_lattice_count())),
        ])
    }
}

/// A refused derivation: KN2.11's `{first_refused_prefix, outcome, certificates}` with the refusal's code and
/// message. Nothing changes: the base keeps its digest, and no value is made (KN2.8).
#[derive(Clone, Debug, PartialEq)]
pub struct Refused {
    /// The base's pose digest.
    pub base: String,
    /// `Digest<literal word>` of the whole word.
    pub word: String,
    pub letters: usize,
    /// The length of the refused prefix (1 for the first letter). It ends at a blocked or invalid letter and, under
    /// Strict, at the first letter that changes a protected piece's pose. `None` for a Net refusal, which judges the
    /// whole word.
    pub first_refused_prefix: Option<u64>,
    /// Blocked or invalid, the outcome of the prefix's last letter; protected for a protection refusal.
    pub outcome: Status,
    pub refusal: Refusal,
    /// The blocked letter's straddle certificate; or, per protected piece that the judged operation moves, the piece
    /// with its pose before and after.
    pub certificates: Vec<Value>,
    /// The report of a protection refusal.
    pub protection: Option<WordProtection>,
}

impl Refused {
    pub fn to_json(&self) -> Value {
        object(vec![
            ("result", text("refused")),
            ("code", text(self.refusal.code)),
            ("message", text(&self.refusal.message)),
            ("base", text(&self.base)),
            ("word", text(&self.word)),
            ("letters", int(self.letters)),
            ("first_refused_prefix", self.first_refused_prefix.map_or(Value::Null, |n| Value::Int(n as i64))),
            ("outcome", text(self.outcome.as_str())),
            ("certificates", Value::Array(self.certificates.clone())),
            ("protection", self.protection.as_ref().map_or(Value::Null, WordProtection::to_json)),
        ])
    }
}

/// The answer of [`derive`].
#[derive(Clone, Debug)]
pub enum Derivation {
    Derived(Derived),
    Refused(Refused),
}

/// The base of a derivation, KN2.11's `StateRef`: a snapshot or an earlier derived value.
#[derive(Clone, Copy, Debug)]
pub enum Base<'a> {
    Snapshot(&'a Snapshot),
    Derived(&'a Derived),
}

/// A protected piece with its pose before and after the judged operation.
fn moved_piece(piece: u32, from: &Pose, to: &Pose) -> Value {
    object(vec![("piece", int(piece as usize)), ("from", pose_json(from)), ("to", pose_json(to))])
}

/// KN2.11's `derive(base, word) -> Derived | Refused`, with protection at pose level in the modes of KN2.7.
///
/// The inner `Err` refuses the request before any letter runs: the base's identity block is not `identity`
/// (`KN1:input.identity_mismatch`, KN1.4), or a protected piece names no piece (`KN2:input.named_object_invalid`).
/// Otherwise the letters execute in order on a copy of the base, each exactly as `State::apply` executes it with no
/// budget, and the word is refused whole, with nothing changed:
///
/// - at its first blocked or invalid letter, with `KN2:input.word_blocked_prefix`, the prefix's length, the letter's
///   outcome and its straddle certificate (KN2.8);
/// - under Strict, at the first letter that changes a protected piece's pose, with `KN2:protection.piece_would_move`;
/// - under Net, when a protected piece's pose at the end of the word is not its pose in the base, with the same code
///   and no prefix.
///
/// A derived value keeps its base's identity and lineage, and its claim is the base's carried along the word. Nothing
/// is searched or chosen: the word is the caller's.
pub fn derive(
    ctx: &Context,
    base: Base,
    identity: &IdentityBlock,
    twists: &[Twist],
    mode: ProtectionMode,
    protect: &[u32],
) -> Result<Result<Derivation, Refusal>, EngineError> {
    let (start, claim, root) = match base {
        Base::Snapshot(snapshot) => (&snapshot.state, &snapshot.reach, None),
        Base::Derived(derived) => (&derived.state, &derived.reach, Some(&derived.lineage)),
    };
    if let Err(refusal) = ctx.check_identity(start.menu(), identity) {
        return Ok(Err(refusal));
    }
    if let Some(&p) = protect.iter().find(|&&p| p as usize >= NP) {
        return Ok(Err(Refusal {
            code: crate::codes::NAMED_OBJECT_INVALID,
            message: format!("Protected piece {p} is outside 0..{}", NP - 1),
            fact: None,
        }));
    }
    let mut protected = protect.to_vec();
    protected.sort_unstable();
    protected.dedup();
    let base_digest = start.digest(ctx);
    if claim.kind() == ReachKind::Witnessed && claim.at() != Some(base_digest.as_str()) {
        return Err(engine("a witnessed claim speaks of another state than its base"));
    }
    let word = word_digest(twists);
    let refused = |first_refused_prefix: Option<u64>,
                   outcome: Status,
                   refusal: Refusal,
                   certificates: Vec<Value>,
                   protection: WordProtection|
     -> Result<Result<Derivation, Refusal>, EngineError> {
        let protection = (protection.verdict == ProtectionVerdict::Moves).then_some(protection);
        Ok(Ok(Derivation::Refused(Refused {
            base: base_digest.clone(),
            word: word.clone(),
            letters: twists.len(),
            first_refused_prefix,
            outcome,
            refusal,
            certificates,
            protection,
        })))
    };

    let strict = mode == ProtectionMode::Strict;
    let guard: &[u32] = if strict { &protected } else { &[] };
    let mut state = start.clone();
    for (index, twist) in twists.iter().enumerate() {
        let prefix = index as u64 + 1;
        let outcome = state.apply(ctx, twist, None, guard)?;
        match outcome.status {
            Status::Admissible => {}
            Status::Protected => {
                let report = outcome.protection.ok_or_else(|| engine("a protection refusal has no report"))?;
                let first = *report.pieces.first().ok_or_else(|| engine("a protection refusal names no piece"))?;
                let certificates = report
                    .pieces
                    .iter()
                    .map(|&p| {
                        let from = state.pose_of(p as usize);
                        moved_piece(p, from, &turned(ctx, twist, from))
                    })
                    .collect();
                let refusal = Refusal {
                    code: crate::codes::PIECE_WOULD_MOVE,
                    message: format!("Prefix {prefix} of the word would change the pose of protected piece {first}"),
                    fact: None,
                };
                let protection = WordProtection {
                    mode,
                    protected: report.protected,
                    pieces: report.pieces,
                    verdict: ProtectionVerdict::Moves,
                    first_refused_prefix: Some(prefix),
                };
                return refused(Some(prefix), Status::Protected, refusal, certificates, protection);
            }
            Status::Blocked | Status::Invalid => {
                let status = outcome.status.as_str();
                let message = match &outcome.refusal {
                    Some((_, reason)) => {
                        format!("The word is refused at prefix {prefix}: its last letter is {status} ({reason})")
                    }
                    None => format!("The word is refused at prefix {prefix}: its last letter is {status}"),
                };
                let certificates = outcome.straddle.iter().map(|c| c.to_json()).collect();
                let refusal = Refusal { code: crate::codes::WORD_BLOCKED_PREFIX, message, fact: None };
                return refused(
                    Some(prefix),
                    outcome.status,
                    refusal,
                    certificates,
                    WordProtection::clear(mode, vec![]),
                );
            }
            Status::Uncertain => return Err(engine("a classification with no budget answered uncertain")),
        }
    }

    // Net judges the end of the whole word; under Strict no prefix moved a protected piece, so none has moved.
    let moved: Vec<u32> =
        protected.iter().copied().filter(|&p| state.pose_of(p as usize) != start.pose_of(p as usize)).collect();
    if let Some(&first) = moved.first() {
        if strict {
            return Err(engine("a word that passed Strict changed a protected piece's pose"));
        }
        let certificates =
            moved.iter().map(|&p| moved_piece(p, start.pose_of(p as usize), state.pose_of(p as usize))).collect();
        let refusal = Refusal {
            code: crate::codes::PIECE_WOULD_MOVE,
            message: format!("The word would change the pose of protected piece {first}"),
            fact: None,
        };
        let protection = WordProtection {
            mode,
            protected,
            pieces: moved,
            verdict: ProtectionVerdict::Moves,
            first_refused_prefix: None,
        };
        return refused(None, Status::Protected, refusal, certificates, protection);
    }

    let reach = claim.then(&base_digest, twists, &state.digest(ctx));
    let lineage = root.cloned().unwrap_or_else(|| Lineage::Snapshot(base_digest.clone()));
    Ok(Ok(Derivation::Derived(Derived {
        identity: ctx.identity(state.menu()),
        derivation: Some(DerivedFrom { base: base_digest, word, letters: twists.len() }),
        lineage,
        reach,
        protection: Some(WordProtection::clear(mode, protected)),
        state,
    })))
}

/// KN2.14: a raw pose table enters as a derived value with reachability open or promised, read under `menu` and
/// validated for form and identity only (`State::from_record`). A claim of another kind, or a table that does not
/// read, is the outer `Err`: a raw table is never witnessed, and this edition checks no theorem evidence. Then its
/// identity block must be `identity` (KN1.4: the inner `Err`). It has no derivation and no protection report, and its
/// lineage is its own pose digest (ONE.21).
pub fn import(
    ctx: &Context,
    menu: Option<Arc<Menu>>,
    record: &Value,
    identity: &IdentityBlock,
    reach: Reachability,
) -> Result<Result<Derived, Refusal>, String> {
    if !matches!(reach.kind(), ReachKind::Open | ReachKind::Promised) {
        return Err(format!("a raw table enters with open or promised reachability, not {}", reach.kind().name()));
    }
    let state = State::from_record(ctx, menu, record)?;
    if let Err(refusal) = ctx.check_identity(state.menu(), identity) {
        return Ok(Err(refusal));
    }
    Ok(Ok(Derived {
        identity: ctx.identity(state.menu()),
        derivation: None,
        lineage: Lineage::Import(state.digest(ctx)),
        reach,
        protection: None,
        state,
    }))
}

/// The answer of [`compare`]: ONE.25's `Partial<Verdict>`.
#[derive(Clone, Debug, PartialEq)]
pub enum Comparison {
    /// Both projections are total: yes when their digests are equal, no otherwise.
    Total(Verdict),
    /// A projection is partial: the verdict on the keys that both cover (slots for labels, pieces for position), with
    /// what each side misses (nothing for a total side). With no key in common the verdict is unknown.
    Partial {
        covered: Verdict,
        a: Missing,
        b: Missing,
    },
    Undefined(String),
    /// The level is not held (`KN1:input.level_not_held`), or the identity blocks differ
    /// (`KN1:input.identity_mismatch`).
    Refused(Refusal),
}

impl Comparison {
    /// `{level, result, verdict, evidence or reason}`, with `missing_a` and `missing_b` for a partial result; a
    /// refusal gives its code and message.
    pub fn to_json(&self, level: Level) -> Value {
        let mut fields = vec![("level", text(level.name()))];
        let verdict = match self {
            Comparison::Total(verdict) => {
                fields.push(("result", text("total")));
                Some(verdict)
            }
            Comparison::Partial { covered, a, b } => {
                fields.push(("result", text("partial")));
                fields.push(("missing_a", a.to_json()));
                fields.push(("missing_b", b.to_json()));
                Some(covered)
            }
            Comparison::Undefined(reason) => {
                fields.push(("result", text("undefined")));
                fields.push(("reason", text(reason)));
                None
            }
            Comparison::Refused(refusal) => {
                fields.push(("result", text("refused")));
                fields.push(("code", text(refusal.code)));
                fields.push(("message", text(&refusal.message)));
                None
            }
        };
        let mut out = object(fields);
        if let (Some(verdict), Value::Object(map)) = (verdict, &mut out) {
            if let Value::Object(more) = verdict.to_json() {
                map.extend(more);
            }
        }
        out
    }
}

/// Two covered parts, each with its keys ascending: the keys both cover, those of them whose values differ, and the
/// keys only the first or only the second covers.
fn walk<T: PartialEq>(ka: &[u32], va: &[T], kb: &[u32], vb: &[T]) -> [usize; 4] {
    let (mut i, mut j) = (0, 0);
    let mut counts = [0usize; 4];
    while i < ka.len() && j < kb.len() {
        match ka[i].cmp(&kb[j]) {
            std::cmp::Ordering::Less => {
                counts[2] += 1;
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                counts[3] += 1;
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                counts[0] += 1;
                counts[1] += usize::from(va[i] != vb[j]);
                i += 1;
                j += 1;
            }
        }
    }
    counts[2] += ka.len() - i;
    counts[3] += kb.len() - j;
    counts
}

/// ONE.25's `compare(a, b, at: level) -> Partial<Verdict>`. The identity blocks must agree (KN1.4) and the level must
/// be held (KN1.9); both refusals are results. Both states are projected to the level ([`project`]). Two total values
/// answer yes when their digests are equal and no otherwise. When either is partial (labels or position off the
/// lattice), the values are compared on the keys that both cover and each side's missing part is reported: a
/// difference there answers no, agreement yes, and no common key unknown.
pub fn compare(ctx: &Context, a: &State, b: &State, at: Level) -> Result<Comparison, EngineError> {
    if let Err(refusal) = ctx.check_identity(a.menu(), &ctx.identity(b.menu())) {
        return Ok(Comparison::Refused(refusal));
    }
    let (pa, pb) = match (project(ctx, a, at)?, project(ctx, b, at)?) {
        (Projection::Refused(refusal), _) | (_, Projection::Refused(refusal)) => {
            return Ok(Comparison::Refused(refusal))
        }
        (Projection::Value(pa), Projection::Value(pb)) => (pa, pb),
    };
    let (ca, ma, cb, mb) = match (pa, pb) {
        (Partial::Undefined(reason), _) | (_, Partial::Undefined(reason)) => return Ok(Comparison::Undefined(reason)),
        (Partial::Total(va), Partial::Total(vb)) => {
            let (da, db) = (va.digest(), vb.digest());
            let evidence = object(vec![("level", text(at.name())), ("a", text(&da)), ("b", text(&db))]);
            return Ok(Comparison::Total(if da == db { Verdict::Yes(evidence) } else { Verdict::No(evidence) }));
        }
        (Partial::Total(va), Partial::Partial { covered, missing }) => (va, Missing::default(), covered, missing),
        (Partial::Partial { covered, missing }, Partial::Total(vb)) => (covered, missing, vb, Missing::default()),
        (Partial::Partial { covered: ca, missing: ma }, Partial::Partial { covered: cb, missing: mb }) => {
            (ca, ma, cb, mb)
        }
    };
    let [common, differing, a_only, b_only] = match (&ca, &cb) {
        (LevelValue::Labels(x), LevelValue::Labels(y)) => walk(&x.slots, &x.labels, &y.slots, &y.labels),
        (LevelValue::Position(x), LevelValue::Position(y)) => walk(&x.pieces, &x.chambers, &y.pieces, &y.chambers),
        _ => return Err(engine(format!("a partial value at {} has no keys to compare", at.name()))),
    };
    let evidence = object(vec![
        ("level", text(at.name())),
        ("common", int(common)),
        ("differing", int(differing)),
        ("a_only", int(a_only)),
        ("b_only", int(b_only)),
    ]);
    let covered = if common == 0 {
        Verdict::Unknown(format!("no key of {} is covered by both values", at.name()))
    } else if differing > 0 {
        Verdict::No(evidence)
    } else {
        Verdict::Yes(evidence)
    };
    Ok(Comparison::Partial { covered, a: ma, b: mb })
}
