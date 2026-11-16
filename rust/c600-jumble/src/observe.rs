//! Observations of a configuration (LP2 M5: JI.8 to JI.11, JI.14 and JI.16 in part, JI.53).
//!
//! Everything is exact. A pose's tree facts follow `research/jumbling/theory/trees.py`: the matrix is
//! x ↦ l x conj(r) / s for quaternions l and r, a height is the distance of a factor on the Bruhat-Tits tree
//! at 2, and a branch names the first step towards it by its tetrahedral subgroup of A5, in trees.py's order.
//! A domain ID is the JI.53 coset key: the SHA-256 of the sorted exact images of the 120 unit icosians, so it
//! does not depend on pose numbering. Seam facts follow `rim.py`: the lattice domain's frame is the identity,
//! every other domain's frame is the pose of its smallest piece, a piece occupies its home chamber moved by
//! its K⁺ part, and a cut cell is a pair of chambers whose signatures differ by one pole (the core lies below
//! the one-pole chambers and belongs to the lattice domain). A mismatched cell is on-grip when its transported
//! normal is a pole and off-grip otherwise.
//!
//! Batches (selection 5) run in chunks of at most `chunk` units with an optional budget and a cancellation
//! point between chunks; a batch that stops early reports its half-open coverage and no totals or final
//! claim (JI.14). Not built here (milestone M8): the holding caps of seam cells, tree positions past the
//! first step and the other JI.6 and JI.7 calls.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use c600_kernel::json::Value;
use num_bigint::BigInt;

use crate::codes;
use crate::kplus::ORDER;
use crate::model::{NP, POLES, SIG_WORDS};
use crate::q5::{det4, dot, identity4, matmul, matrix_compact, matrix_json, matvec, solve, transpose, Mat4, Vec4, Q5};
use crate::state::{Certificate, Pose, State, Status};
use crate::twist::Twist;
use crate::Context;

/// The X5 bins of left heights (`research/jumbling/differential/leads/x5_release.py`).
pub const COUNT_KEYS: [&str; 5] = ["n_0", "n_2", "n_4", "n_6", "n_above_6"];
/// The core: the chamber below every one-pole chamber, held by the lattice domain.
const CORE: u32 = NP as u32;

type Quat = Vec4;

// ------------------------------------------------------------------------------------------------ JSON helpers

fn int(v: i64) -> Value {
    Value::Int(v)
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn ids(values: &[u32]) -> Value {
    Value::Array(values.iter().map(|&v| int(v as i64)).collect())
}

fn pair(a: i64, b: i64) -> Value {
    Value::Array(vec![int(a), int(b)])
}

fn int_map(map: BTreeMap<String, i64>) -> Value {
    Value::Object(map.into_iter().map(|(k, v)| (k, int(v))).collect())
}

fn opt_int(v: Option<i64>) -> Value {
    v.map_or(Value::Null, int)
}

/// A refusal answer: the chapter 13 code and its message.
fn refusal(code: &str, message: &str) -> Value {
    obj(vec![("status", text("refused")), ("code", text(code)), ("message", text(message))])
}

/// The moved-set hash of the harness: SHA-256 of the ascending piece IDs as little-endian u32.
pub fn moved_set_hash(sorted: &[u32]) -> String {
    let bytes: Vec<u8> = sorted.iter().flat_map(|p| p.to_le_bytes()).collect();
    c600_digest::sha256_hex(&bytes)
}

// ------------------------------------------------------------------------------------------- exact ordering

fn cmp_q5(x: &Q5, y: &Q5) -> Ordering {
    x.sub(y).sign().cmp(&0)
}

fn cmp_quat(x: &Quat, y: &Quat) -> Ordering {
    (0..4).map(|k| cmp_q5(&x[k], &y[k])).find(|o| *o != Ordering::Equal).unwrap_or(Ordering::Equal)
}

/// Python's order of tuples of reduced triples (a, b, d): the integers, coordinate by coordinate.
fn cmp_triples(x: &Vec4, y: &Vec4) -> Ordering {
    (0..4)
        .map(|k| x[k].a().cmp(y[k].a()).then_with(|| x[k].b().cmp(y[k].b())).then_with(|| x[k].d().cmp(y[k].d())))
        .find(|o| *o != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

// --------------------------------------------------------------------------------------------- quaternions

fn one() -> Quat {
    [Q5::one(), Q5::zero(), Q5::zero(), Q5::zero()]
}

fn qmul(p: &Quat, r: &Quat) -> Quat {
    let m = |i: usize, j: usize| p[i].mul(&r[j]);
    [
        m(0, 0).sub(&m(1, 1)).sub(&m(2, 2)).sub(&m(3, 3)),
        m(0, 1).add(&m(1, 0)).add(&m(2, 3)).sub(&m(3, 2)),
        m(0, 2).sub(&m(1, 3)).add(&m(2, 0)).add(&m(3, 1)),
        m(0, 3).add(&m(1, 2)).sub(&m(2, 1)).add(&m(3, 0)),
    ]
}

fn qconj(p: &Quat) -> Quat {
    [p[0].clone(), p[1].neg(), p[2].neg(), p[3].neg()]
}

fn iqmul(p: [i64; 4], r: [i64; 4]) -> [i64; 4] {
    [
        p[0] * r[0] - p[1] * r[1] - p[2] * r[2] - p[3] * r[3],
        p[0] * r[1] + p[1] * r[0] + p[2] * r[3] - p[3] * r[2],
        p[0] * r[2] - p[1] * r[3] + p[2] * r[0] + p[3] * r[1],
        p[0] * r[3] + p[1] * r[2] - p[2] * r[1] + p[3] * r[0],
    ]
}

/// The sign-normalised key of ±q: its first non-zero coordinate is positive.
fn qkey(q: &Quat) -> Quat {
    match q.iter().find(|c| !c.is_zero()) {
        Some(c) if c.sign() < 0 => [q[0].neg(), q[1].neg(), q[2].neg(), q[3].neg()],
        _ => q.clone(),
    }
}

fn v2int(n: &BigInt) -> Option<i64> {
    n.trailing_zeros().map(|k| k as i64)
}

/// The 2-adic valuation on Q(√5) (2 is inert; Z[φ] has basis (1, φ) and √5 = 2φ − 1).
fn v2(x: &Q5) -> Result<i64, String> {
    if x.is_zero() {
        return Err("v2(0) is undefined".into());
    }
    let two = BigInt::from(2);
    let low = [v2int(&(x.a() - x.b())), v2int(&(x.b() * &two))].into_iter().flatten().min();
    match (low, v2int(x.d())) {
        (Some(low), Some(den)) => Ok(low - den),
        _ => Err("v2: malformed number".into()),
    }
}

// ------------------------------------------------------------------------------------------------ the trees

/// trees.py's fixed data: the 120 unit icosians, a local basis of the icosian ring at 2, the 60 elements of
/// A5 as sign-normalised units and its five tetrahedral subgroups in trees.py's order.
struct Trees {
    units: Vec<Quat>,
    binv: Mat4,
    punits: Vec<Quat>,
    tets: Vec<HashSet<Quat>>,
    em: [[[[i8; 4]; 4]; 4]; 4],
}

fn icosian_units() -> Result<Vec<Quat>, String> {
    let z = Q5::zero();
    let mut out: Vec<Quat> = Vec::new();
    for i in 0..4 {
        for s in [1, -1] {
            let mut v = [z.clone(), z.clone(), z.clone(), z.clone()];
            v[i] = Q5::int(s);
            out.push(v);
        }
    }
    for mask in 0..16 {
        let h = |k: usize| Q5::of(if mask >> k & 1 == 1 { -1 } else { 1 }, 0, 2);
        out.push([h(0), h(1), h(2), h(3)]);
    }
    // φ/2, 1/2 and (φ − 1)/2 in every even permutation with every sign.
    let (a, b, c) = (Q5::of(1, 1, 4), Q5::of(1, 0, 2), Q5::of(-1, 1, 4));
    for p in permutations4() {
        let inversions = (0..4).flat_map(|x| (x + 1..4).map(move |y| (x, y))).filter(|&(x, y)| p[x] > p[y]).count();
        if inversions % 2 != 0 {
            continue;
        }
        for signs in 0..8 {
            let s = |k: usize, v: &Q5| if signs >> k & 1 == 1 { v.neg() } else { v.clone() };
            let base = [s(0, &a), s(1, &b), s(2, &c), z.clone()];
            out.push([base[p[0]].clone(), base[p[1]].clone(), base[p[2]].clone(), base[p[3]].clone()]);
        }
    }
    let distinct: HashSet<Quat> = out.iter().cloned().collect();
    if distinct.len() != 120 || out.len() != 120 {
        return Err(format!("expected 120 unit icosians, built {} ({} distinct)", out.len(), distinct.len()));
    }
    out.sort_by(cmp_quat);
    Ok(out)
}

fn permutations4() -> Vec<[usize; 4]> {
    let mut out = Vec::new();
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let p = [a, b, c, d];
                    if (0..4).all(|x| p.contains(&x)) {
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

fn closure(gens: &[Quat]) -> HashSet<Quat> {
    let start = qkey(&one());
    let mut set = HashSet::from([start.clone()]);
    let mut frontier = vec![start];
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for a in &frontier {
            for g in gens {
                let b = qkey(&qmul(a, g));
                if set.insert(b.clone()) {
                    next.push(b);
                }
            }
        }
        frontier = next;
    }
    set
}

fn order(u: &Quat) -> usize {
    let target = qkey(&one());
    let (mut p, mut n) = (u.clone(), 1);
    while qkey(&p) != target && n <= 60 {
        p = qmul(&p, u);
        n += 1;
    }
    n
}

impl Trees {
    fn build() -> Result<Trees, String> {
        let units = icosian_units()?;
        // Any four units whose Gram determinant (2·qdot) is a 2-adic unit span the local icosian ring, so the
        // valuation of coordinates in that basis does not depend on the choice; take the first in index order.
        let mut basis = None;
        'search: for i in 0..120 {
            for j in i + 1..120 {
                for k in j + 1..120 {
                    for l in k + 1..120 {
                        let b = [&units[i], &units[j], &units[k], &units[l]];
                        let gram: Mat4 =
                            std::array::from_fn(|r| std::array::from_fn(|c| dot(b[r], b[c]).mul(&Q5::int(2))));
                        let g = det4(&gram);
                        if !g.is_zero() && v2(&g)? == 0 {
                            basis = Some([b[0].clone(), b[1].clone(), b[2].clone(), b[3].clone()]);
                            break 'search;
                        }
                    }
                }
            }
        }
        let basis = basis.ok_or("no local basis of unit icosians")?;
        let bt: Vec<Vec<Q5>> = (0..4).map(|r| (0..4).map(|c| basis[c][r].clone()).collect()).collect();
        let eye: Vec<Vec<Q5>> =
            (0..4).map(|r| (0..4).map(|c| if r == c { Q5::one() } else { Q5::zero() }).collect()).collect();
        let inv = solve(&bt, &eye).ok_or("the local basis is singular")?;
        let binv: Mat4 = std::array::from_fn(|r| std::array::from_fn(|c| inv[r][c].clone()));
        let btm: Mat4 = std::array::from_fn(|r| std::array::from_fn(|c| bt[r][c].clone()));
        if matmul(&binv, &btm) != identity4() {
            return Err("the local basis inverse does not check".into());
        }
        let mut seen = HashSet::new();
        let mut punits = Vec::new();
        for u in &units {
            let k = qkey(u);
            if seen.insert(k.clone()) {
                punits.push(k);
            }
        }
        if punits.len() != 60 {
            return Err(format!("expected 60 elements of A5, found {}", punits.len()));
        }
        // trees.py takes the groups generated by two commuting involutions and an element of order 3 that have
        // order 12. Each Klein group lies in exactly one A4 (its normaliser), so one such c per Klein group
        // gives the same five subgroups.
        let inv2: Vec<&Quat> = punits.iter().filter(|u| order(u) == 2).collect();
        let ord3: Vec<&Quat> = punits.iter().filter(|u| order(u) == 3).collect();
        let mut kleins: Vec<(Quat, Quat)> = Vec::new();
        let mut seen_klein: Vec<HashSet<Quat>> = Vec::new();
        for a in &inv2 {
            for b in &inv2 {
                if a == b {
                    continue;
                }
                let v = closure(&[(*a).clone(), (*b).clone()]);
                if v.len() == 4 && !seen_klein.contains(&v) {
                    seen_klein.push(v);
                    kleins.push(((*a).clone(), (*b).clone()));
                }
            }
        }
        let mut tets: Vec<Vec<Quat>> = Vec::new();
        for (a, b) in &kleins {
            for c in &ord3 {
                let h = closure(&[a.clone(), b.clone(), (*c).clone()]);
                if h.len() == 12 {
                    let mut members: Vec<Quat> = h.into_iter().collect();
                    members.sort_by(cmp_quat);
                    if !tets.contains(&members) {
                        tets.push(members);
                    }
                    break;
                }
            }
        }
        if tets.len() != 5 {
            return Err(format!("expected 5 tetrahedral subgroups, found {}", tets.len()));
        }
        // trees.py sorts the subgroups by their sorted members (as float tuples; exact order agrees).
        tets.sort_by(|x, y| {
            x.iter().zip(y).map(|(p, q)| cmp_quat(p, q)).find(|o| *o != Ordering::Equal).unwrap_or(Ordering::Equal)
        });
        let e = |i: usize| -> [i64; 4] { std::array::from_fn(|k| (k == i) as i64) };
        let mut em = [[[[0i8; 4]; 4]; 4]; 4];
        for (i, row) in em.iter_mut().enumerate() {
            for (j, m) in row.iter_mut().enumerate() {
                let conj = {
                    let v = e(j);
                    [v[0], -v[1], -v[2], -v[3]]
                };
                for b in 0..4 {
                    let col = iqmul(iqmul(e(i), e(b)), conj);
                    for a in 0..4 {
                        m[a][b] = col[a] as i8;
                    }
                }
            }
        }
        Ok(Trees { units, binv, punits, tets: tets.into_iter().map(|t| t.into_iter().collect()).collect(), em })
    }

    /// The largest k with 2⁻ᵏx in the icosian ring at 2.
    fn kval(&self, x: &Quat) -> Result<i64, String> {
        let mut low: Option<i64> = None;
        for c in matvec(&self.binv, x).iter().filter(|c| !c.is_zero()) {
            let v = v2(c)?;
            low = Some(low.map_or(v, |l| l.min(v)));
        }
        low.ok_or_else(|| "kval of the zero quaternion".into())
    }

    /// The tree distance from v₀ to x v₀.
    fn dist(&self, x: &Quat) -> Result<i64, String> {
        Ok(v2(&dot(x, x))? - 2 * self.kval(x)?)
    }

    /// trees.py's branch: the index of the tetrahedral subgroup fixing the first step, or −1 at distance 0.
    fn branch(&self, x: &Quat, d: i64) -> Result<i64, String> {
        if d == 0 {
            return Ok(-1);
        }
        let xi = qconj(x);
        let mut fix = HashSet::new();
        for u in &self.punits {
            if self.dist(&qmul(&qmul(&xi, u), x))? <= 2 * d - 2 {
                fix.insert(u.clone());
            }
        }
        self.tets.iter().position(|s| *s == fix).map(|t| t as i64).ok_or_else(|| "branch not identified".into())
    }

    /// (l, r) with M x proportional to l x conj(r), checked exactly.
    fn factors(&self, m: &Mat4) -> Result<(Quat, Quat), String> {
        let quarter = Q5::of(1, 0, 4);
        let mut a: [[Q5; 4]; 4] = std::array::from_fn(|_| std::array::from_fn(|_| Q5::zero()));
        for (i, row) in a.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                let mut s = Q5::zero();
                for (r, mrow) in m.iter().enumerate() {
                    for (c, x) in mrow.iter().enumerate() {
                        match self.em[i][j][r][c] {
                            1 => s = s.add(x),
                            -1 => s = s.sub(x),
                            _ => {}
                        }
                    }
                }
                *cell = s.mul(&quarter);
            }
        }
        let j = (0..4).find(|&j| (0..4).any(|i| !a[i][j].is_zero())).ok_or("the factor matrix is zero")?;
        let i = (0..4).find(|&i| (0..4).any(|j| !a[i][j].is_zero())).ok_or("the factor matrix is zero")?;
        let l: Quat = std::array::from_fn(|k| a[k][j].clone());
        let r: Quat = std::array::from_fn(|k| a[i][k].clone());
        // M e_b = s · l e_b conj(r) with one common non-zero scalar s.
        let rc = qconj(&r);
        let mut ratio: Option<Q5> = None;
        for b in 0..4 {
            let eb: Quat = std::array::from_fn(|k| if k == b { Q5::one() } else { Q5::zero() });
            let y = qmul(&qmul(&l, &eb), &rc);
            for (row, yv) in m.iter().zip(&y) {
                let mv = &row[b];
                if mv.is_zero() {
                    if !yv.is_zero() {
                        return Err("quaternion factor extraction failed".into());
                    }
                    continue;
                }
                let q = yv.div(mv).ok_or("quaternion factor extraction failed")?;
                match &ratio {
                    None => ratio = Some(q),
                    Some(r0) if *r0 != q => return Err("quaternion factor extraction failed".into()),
                    _ => {}
                }
            }
        }
        match ratio {
            Some(r0) if !r0.is_zero() => Ok((l, r)),
            _ => Err("quaternion factor extraction failed".into()),
        }
    }
}

/// The JI.53 coset key of a rotation: SHA-256 of `json.dumps(sorted images of the 120 unit icosians)`.
fn domain_key(units: &[Quat], m: &Mat4) -> String {
    let mut images: Vec<Vec4> = units.iter().map(|u| matvec(m, u)).collect();
    images.sort_by(cmp_triples);
    let mut out = String::from("[");
    for (i, v) in images.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        for (j, x) in v.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            x.write_compact(&mut out);
        }
        out.push(']');
    }
    out.push(']');
    c600_digest::sha256_hex(out.as_bytes())
}

// ------------------------------------------------------------------------------------------------ the tables

/// One exact pose: its pose key (the compact JSON matrix), domain ID, tree heights and branches.
pub struct Facts {
    pub key: String,
    pub domain: String,
    pub dl: i64,
    pub dr: i64,
    pub bl: i64,
    pub br: i64,
    left: Quat,
}

/// Model facts the observations need, built once per context on first use.
pub struct Tables {
    trees: Trees,
    lattice_key: String,
    sig_index: HashMap<[u64; SIG_WORDS], u32>,
    lo: Vec<u32>,
    up: Vec<u32>,
    pole: Vec<u16>,
    adj_start: Vec<usize>,
    adj: Vec<u32>,
    by_pole: Vec<Vec<u32>>,
    cls: Vec<OnceLock<Result<i64, String>>>,
    k_facts: Vec<OnceLock<Arc<Facts>>>,
    m_facts: Mutex<HashMap<String, Arc<Facts>>>,
}

/// The observation tables of a context.
pub fn tables(ctx: &Context) -> Result<&Tables, String> {
    ctx.observe.get_or_init(|| Tables::build(ctx)).as_ref().map_err(Clone::clone)
}

impl Tables {
    fn build(ctx: &Context) -> Result<Tables, String> {
        let model = &ctx.model;
        let trees = Trees::build()?;
        let lattice_key = domain_key(&trees.units, &identity4());
        // K⁺ preserves the unit icosians, so every K⁺ pose has the lattice key: check it on the generators.
        for &g in &ctx.kplus.gen_idx {
            if domain_key(&trees.units, ctx.kplus.matrix(g, model)) != lattice_key {
                return Err("a K⁺ generator moves the unit icosians".into());
            }
        }
        let mut sig_index = HashMap::with_capacity(NP);
        for p in 0..NP {
            if sig_index.insert(*model.sig_bits(p), p as u32).is_some() {
                return Err(format!("piece {p} repeats a signature"));
            }
        }
        let (mut lo, mut up, mut pole) = (Vec::new(), Vec::new(), Vec::new());
        for q in 0..NP {
            let bits = model.sig_bits(q);
            for &e in model.signature(q) {
                let mut lower = *bits;
                lower[e as usize / 64] &= !(1u64 << (e as usize % 64));
                let below = if lower.iter().all(|&w| w == 0) { Some(CORE) } else { sig_index.get(&lower).copied() };
                if let Some(below) = below {
                    lo.push(below);
                    up.push(q as u32);
                    pole.push(e);
                }
            }
        }
        let mut adj_start = vec![0usize; NP + 2];
        for t in 0..lo.len() {
            adj_start[lo[t] as usize + 1] += 1;
            adj_start[up[t] as usize + 1] += 1;
        }
        for q in 0..=NP {
            adj_start[q + 1] += adj_start[q];
        }
        let mut cursor = adj_start.clone();
        let mut adj = vec![0u32; adj_start[NP + 1]];
        let mut by_pole = vec![Vec::new(); POLES];
        for t in 0..lo.len() {
            for q in [lo[t], up[t]] {
                adj[cursor[q as usize]] = t as u32;
                cursor[q as usize] += 1;
            }
            by_pole[pole[t] as usize].push(t as u32);
        }
        Ok(Tables {
            trees,
            lattice_key,
            sig_index,
            lo,
            up,
            pole,
            adj_start,
            adj,
            by_pole,
            cls: (0..POLES).map(|_| OnceLock::new()).collect(),
            k_facts: (0..ORDER).map(|_| OnceLock::new()).collect(),
            m_facts: Mutex::new(HashMap::new()),
        })
    }

    fn adj_of(&self, q: u32) -> &[u32] {
        &self.adj[self.adj_start[q as usize]..self.adj_start[q as usize + 1]]
    }

    /// The domain ID of the lattice (JI.53).
    pub fn lattice_key(&self) -> &str {
        &self.lattice_key
    }

    /// CLS(c): the branch of the pole n_c as a quaternion (stage 14's blockmap).
    pub fn cls(&self, ctx: &Context, c: usize) -> Result<i64, String> {
        self.cls[c]
            .get_or_init(|| {
                let n = &ctx.model.poles[c];
                let d = self.trees.dist(n)?;
                self.trees.branch(n, d)
            })
            .clone()
    }

    /// The facts of one pose (cached by exact pose).
    pub fn facts(&self, ctx: &Context, pose: &Pose) -> Result<Arc<Facts>, String> {
        match pose {
            Pose::K(k) => Ok(self.k_facts[*k as usize]
                .get_or_init(|| {
                    Arc::new(Facts {
                        key: matrix_compact(ctx.kplus.matrix(*k, &ctx.model)),
                        domain: self.lattice_key.clone(),
                        dl: 0,
                        dr: 0,
                        bl: -1,
                        br: -1,
                        left: one(),
                    })
                })
                .clone()),
            Pose::M(m) => {
                let key = matrix_compact(m);
                if let Some(f) = self.m_facts.lock().map_err(|_| "facts cache poisoned")?.get(&key) {
                    return Ok(f.clone());
                }
                let (l, r) = self.trees.factors(m)?;
                let (dl, dr) = (self.trees.dist(&l)?, self.trees.dist(&r)?);
                let facts = Arc::new(Facts {
                    domain: domain_key(&self.trees.units, m),
                    bl: self.trees.branch(&l, dl)?,
                    br: self.trees.branch(&r, dr)?,
                    key: key.clone(),
                    dl,
                    dr,
                    left: l,
                });
                self.m_facts.lock().map_err(|_| "facts cache poisoned")?.insert(key, facts.clone());
                Ok(facts)
            }
        }
    }

    /// The chamber of piece p moved by the K⁺ element k: the piece whose signature is k(sig p).
    fn image(&self, ctx: &Context, p: usize, k: u16) -> Result<u32, String> {
        let perm = &ctx.kplus.perms[k as usize];
        let mut bits = [0u64; SIG_WORDS];
        for &e in ctx.model.signature(p) {
            let f = perm[e as usize] as usize;
            bits[f / 64] |= 1u64 << (f % 64);
        }
        self.sig_index.get(&bits).copied().ok_or_else(|| format!("no chamber for piece {p} under K⁺ element {k}"))
    }

    /// Tree facts of an exact rotation (the free-mode and test entry).
    pub fn tree_facts(&self, m: &Mat4) -> Result<(i64, i64, i64, i64), String> {
        let (l, r) = self.trees.factors(m)?;
        let (dl, dr) = (self.trees.dist(&l)?, self.trees.dist(&r)?);
        Ok((dl, dr, self.trees.branch(&l, dl)?, self.trees.branch(&r, dr)?))
    }

    /// The JI.53 domain ID of an exact rotation.
    pub fn domain_id(&self, m: &Mat4) -> String {
        domain_key(&self.trees.units, m)
    }
}

// ------------------------------------------------------------------------------------------- state summaries

fn pose_facts(ctx: &Context, t: &Tables, state: &State) -> Result<Vec<Arc<Facts>>, String> {
    state.poses().iter().map(|pose| t.facts(ctx, pose)).collect()
}

fn pose_counts(state: &State) -> Vec<i64> {
    let mut counts = vec![0i64; state.poses().len()];
    for &pid in state.pose_ids() {
        counts[pid as usize] += 1;
    }
    counts
}

/// Left heights by piece: counts by height, Φ = Σ max(0, h − 2) and the largest height.
struct LeftStats {
    counts: BTreeMap<i64, i64>,
    phi: i64,
    max: i64,
}

fn left_stats(facts: &[Arc<Facts>], counts: &[i64]) -> LeftStats {
    let mut out = LeftStats { counts: BTreeMap::new(), phi: 0, max: 0 };
    for (f, &n) in facts.iter().zip(counts) {
        if n == 0 {
            continue;
        }
        *out.counts.entry(f.dl).or_default() += n;
        out.phi += n * (f.dl - 2).max(0);
        out.max = out.max.max(f.dl);
    }
    out
}

fn stats_of(ctx: &Context, t: &Tables, state: &State) -> Result<LeftStats, String> {
    Ok(left_stats(&pose_facts(ctx, t, state)?, &pose_counts(state)))
}

/// `observations.left_counts`: X5's bins and maximum, with sparse odd bins kept.
fn left_counts_json(counts: &BTreeMap<i64, i64>) -> BTreeMap<String, i64> {
    let mut out: BTreeMap<String, i64> = COUNT_KEYS.iter().map(|k| (k.to_string(), 0)).collect();
    for (&h, &n) in counts {
        let key = if h > 6 { "n_above_6".to_string() } else { format!("n_{h}") };
        *out.entry(key).or_default() += n;
    }
    out
}

fn counts_value(counts: &BTreeMap<i64, i64>) -> Value {
    let mut map: BTreeMap<String, Value> = left_counts_json(counts).into_iter().map(|(k, v)| (k, int(v))).collect();
    let above = counts.iter().filter(|(h, n)| **h > 6 && **n > 0).map(|(h, _)| *h).max();
    map.insert("max_above_6".into(), opt_int(above));
    Value::Object(map)
}

/// `delta_n_h`: after minus before for every bin of either side, so a sparse odd bin that `left_counts_json` keeps
/// is in the delta too and the changes sum to zero (JI.6's sparse histogram; JI.10, JI.11). On I_a states only
/// the fixed bins occur, so the delta has J1's five keys there; J1's reference refuses an odd height.
fn delta_counts(before: &BTreeMap<i64, i64>, after: &BTreeMap<i64, i64>) -> Value {
    let (b, a) = (left_counts_json(before), left_counts_json(after));
    let keys: BTreeSet<&String> = b.keys().chain(a.keys()).collect();
    Value::Object(
        keys.into_iter()
            .map(|k| (k.clone(), int(a.get(k).copied().unwrap_or(0) - b.get(k).copied().unwrap_or(0))))
            .collect(),
    )
}

/// JI.6 heights: every occupied pose with its domain ID, heights and branches, the X5 counts and both
/// trees-fixtures histograms.
pub fn heights(ctx: &Context, state: &State) -> Result<Value, String> {
    let t = tables(ctx)?;
    let facts = pose_facts(ctx, t, state)?;
    let counts = pose_counts(state);
    let mut poses = BTreeMap::new();
    let (mut pieces, mut classes) = (BTreeMap::new(), BTreeMap::new());
    for (f, &n) in facts.iter().zip(&counts) {
        if n == 0 {
            continue;
        }
        poses.insert(
            f.key.clone(),
            obj(vec![
                ("domain", text(&f.domain)),
                ("heights", pair(f.dl, f.dr)),
                ("branches", pair(f.bl, f.br)),
                ("count", int(n)),
            ]),
        );
        let key = format!("{},{}", f.dl, f.dr);
        *pieces.entry(key.clone()).or_default() += n;
        *classes.entry(key).or_default() += 1;
    }
    let stats = left_stats(&facts, &counts);
    Ok(obj(vec![
        ("poses", Value::Object(poses)),
        ("height_counts", counts_value(&stats.counts)),
        (
            "histograms",
            obj(vec![("pieces_by_heights", int_map(pieces)), ("pose_classes_by_heights", int_map(classes))]),
        ),
        ("off", int(state.off_lattice_count() as i64)),
        ("phi", int(stats.phi)),
        ("max_left_height", int(stats.max)),
        ("lattice_key", text(&t.lattice_key)),
    ]))
}

// --------------------------------------------------------------------------------------------------- domains

/// The domains of a state: ID, frame and members, and the chamber every piece occupies in its frame.
struct Domains {
    keys: Vec<String>,
    reps: Vec<Mat4>,
    cham: Vec<u32>,
    members: Vec<Vec<u32>>,
}

fn domains(ctx: &Context, t: &Tables, state: &State, facts: &[Arc<Facts>]) -> Result<Domains, String> {
    let (poses, piece_pose) = (state.poses(), state.pose_ids());
    let mut first = vec![u32::MAX; poses.len()];
    for (p, &pid) in piece_pose.iter().enumerate() {
        if first[pid as usize] == u32::MAX {
            first[pid as usize] = p as u32;
        }
    }
    let used = |pid: usize| first[pid] != u32::MAX;
    let mut keys = vec![t.lattice_key.clone()];
    let mut others: Vec<&str> = (0..poses.len())
        .filter(|&pid| used(pid) && facts[pid].domain != t.lattice_key)
        .map(|pid| facts[pid].domain.as_str())
        .collect();
    others.sort_unstable();
    others.dedup();
    keys.extend(others.iter().map(|s| s.to_string()));
    let index: HashMap<&str, usize> = keys.iter().enumerate().map(|(i, k)| (k.as_str(), i)).collect();
    let mut domain_of_pose = vec![0usize; poses.len()];
    let mut rep_pose: Vec<Option<(u32, usize)>> = vec![None; keys.len()];
    for pid in (0..poses.len()).filter(|&pid| used(pid)) {
        let d = index[facts[pid].domain.as_str()];
        domain_of_pose[pid] = d;
        if d > 0 && rep_pose[d].is_none_or(|(f, _)| first[pid] < f) {
            rep_pose[d] = Some((first[pid], pid));
        }
    }
    let mut reps = vec![identity4()];
    for rep in rep_pose.iter().skip(1) {
        let (_, pid) = rep.ok_or("a domain without poses")?;
        reps.push(state.pose_matrix(ctx, pid as u32).clone());
    }
    let mut kpart = vec![0u16; poses.len()];
    for pid in (0..poses.len()).filter(|&pid| used(pid)) {
        let d = domain_of_pose[pid];
        kpart[pid] = if d == 0 {
            poses[pid].kidx().ok_or("a pose outside K⁺ has the lattice domain ID")?
        } else {
            let k = matmul(&transpose(&reps[d]), state.pose_matrix(ctx, pid as u32));
            ctx.kplus.index_of_matrix(&k, &ctx.model).ok_or("a pose lies outside its domain's coset")?
        };
    }
    let mut cham = vec![0u32; NP];
    let mut members = vec![Vec::new(); keys.len()];
    for (p, &pid) in piece_pose.iter().enumerate() {
        let k = kpart[pid as usize];
        cham[p] = if k == 0 { p as u32 } else { t.image(ctx, p, k)? };
        members[domain_of_pose[pid as usize]].push(p as u32);
    }
    Ok(Domains { keys, reps, cham, members })
}

/// Visits every occupied cut cell once, domain by domain and in chamber order: (domain, pair, occupancy),
/// occupancy 0 when both sides are held, 1 when only the lower and 2 when only the upper side is held.
fn seam_scan(t: &Tables, dom: &Domains, mut visit: impl FnMut(u32, u32, u8)) {
    let mut mark = vec![0u32; NP + 1];
    for (d, members) in dom.members.iter().enumerate() {
        let stamp = d as u32 + 1;
        let mut held: Vec<u32> = members.iter().map(|&p| dom.cham[p as usize]).collect();
        if d == 0 {
            held.push(CORE);
        }
        for &q in &held {
            mark[q as usize] = stamp;
        }
        held.sort_unstable();
        for &q in &held {
            for &ti in t.adj_of(q) {
                let (lo, up) = (t.lo[ti as usize], t.up[ti as usize]);
                let other = if lo == q { up } else { lo };
                let other_held = mark[other as usize] == stamp;
                if other_held && q != lo {
                    continue;
                }
                visit(
                    d as u32,
                    ti,
                    if other_held {
                        0
                    } else if q == lo {
                        1
                    } else {
                        2
                    },
                );
            }
        }
    }
}

/// N: the mismatched cut cells of every domain.
fn total_n(ctx: &Context, t: &Tables, state: &State) -> Result<i64, String> {
    let facts = pose_facts(ctx, t, state)?;
    let dom = domains(ctx, t, state, &facts)?;
    let mut n = 0i64;
    seam_scan(t, &dom, |_, _, occ| n += (occ != 0) as i64);
    Ok(n)
}

fn on_grip(ctx: &Context, dom: &Domains, d: u32, e: u16, cache: &mut HashMap<(u32, u16), bool>) -> bool {
    if d == 0 {
        return true;
    }
    *cache
        .entry((d, e))
        .or_insert_with(|| ctx.model.pole_of(&matvec(&dom.reps[d as usize], &ctx.model.poles[e as usize])).is_some())
}

// --------------------------------------------------------------------------------------------------- batches

/// A batch request (selection 5): at most `chunk` units per chunk, at most `budget` units in all, and a
/// cancellation observed after `cancel_after` chunks (a test hook for a cancellation between chunks).
pub struct Batch {
    pub chunk: u64,
    pub budget: Option<u64>,
    pub cancel_after: Option<u64>,
}

struct Progress {
    processed: u64,
    chunks: u64,
    cancelled: bool,
    cut_off: bool,
}

impl Progress {
    /// The end of the next chunk, or None when the batch stops (done, cancelled or out of budget).
    fn next(&mut self, batch: &Batch, total: u64) -> Option<u64> {
        if self.processed >= total {
            return None;
        }
        if batch.cancel_after.is_some_and(|c| self.chunks >= c) {
            self.cancelled = true;
            return None;
        }
        if batch.budget.is_some_and(|b| self.processed >= b) {
            self.cut_off = true;
            return None;
        }
        Some((self.processed + batch.chunk).min(total).min(batch.budget.unwrap_or(u64::MAX)))
    }

    fn fields(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("coverage", pair(0, self.processed as i64)),
            ("chunks", int(self.chunks as i64)),
            ("cancelled", Value::Bool(self.cancelled)),
            ("cut_off", Value::Bool(self.cut_off)),
        ]
    }
}

// ----------------------------------------------------------------------------------------------- seam facts

/// The scope of a seam-facts batch: every domain, or the cut of one named cap.
pub enum Scope {
    All,
    Cap(usize),
}

/// Which cells a seam-facts answer lists.
#[derive(Clone, Copy, PartialEq)]
pub enum Rows {
    None,
    Mismatched,
    All,
}

const OCCUPANCY: [&str; 3] = ["both", "lower", "upper"];

struct RowWriter<'a> {
    ctx: &'a Context,
    t: &'a Tables,
    dom: &'a Domains,
    holder: HashMap<(u32, u32), u32>,
}

impl<'a> RowWriter<'a> {
    fn new(ctx: &'a Context, t: &'a Tables, dom: &'a Domains) -> RowWriter<'a> {
        let mut holder = HashMap::new();
        for (d, members) in dom.members.iter().enumerate() {
            for &p in members {
                holder.insert((d as u32, dom.cham[p as usize]), p);
            }
        }
        RowWriter { ctx, t, dom, holder }
    }

    fn side(&self, d: u32, chamber: u32, held: bool) -> Value {
        if !held {
            Value::Null
        } else if chamber == CORE {
            text("core")
        } else {
            self.holder.get(&(d, chamber)).map_or(Value::Null, |&p| int(p as i64))
        }
    }

    fn row(&self, d: u32, ti: u32, occ: u8, on: bool) -> Value {
        let (lo, up, e) = (self.t.lo[ti as usize], self.t.up[ti as usize], self.t.pole[ti as usize]);
        let normal = matvec(&self.dom.reps[d as usize], &self.ctx.model.poles[e as usize]);
        obj(vec![
            ("domain", text(&self.dom.keys[d as usize])),
            ("cell", pair(lo as i64, up as i64)),
            ("pole", int(e as i64)),
            ("normal", Value::Array(normal.iter().map(Q5::to_json).collect())),
            ("occupancy", text(OCCUPANCY[occ as usize])),
            ("lower", self.side(d, lo, occ != 2)),
            ("upper", self.side(d, up, occ != 1)),
            ("aligned", Value::Bool(occ == 0)),
            ("grip", text(if on { "on" } else { "off" })),
        ])
    }
}

/// The cut cells of an admissible cap c as rim.py's `rim_sets` reads them: in every domain with members whose
/// frame carries n_c to a pole e, the cells at e that the domain holds on at least one side. They lie on c's
/// hyperplane, so they are on-grip.
fn cap_cells(ctx: &Context, t: &Tables, dom: &Domains, c: usize) -> Vec<(u32, u32, u8)> {
    let mut cells = Vec::new();
    let mut mark = vec![0u32; NP + 1];
    for (d, members) in dom.members.iter().enumerate() {
        if members.is_empty() {
            continue;
        }
        let Some(e) = ctx.model.pole_of(&matvec(&transpose(&dom.reps[d]), &ctx.model.poles[c])) else {
            continue;
        };
        let stamp = d as u32 + 1;
        for &p in members {
            mark[dom.cham[p as usize] as usize] = stamp;
        }
        if d == 0 {
            mark[CORE as usize] = stamp;
        }
        for &ti in &t.by_pole[e] {
            let lo_held = mark[t.lo[ti as usize] as usize] == stamp;
            let up_held = mark[t.up[ti as usize] as usize] == stamp;
            let occ = match (lo_held, up_held) {
                (false, false) => continue,
                (true, true) => 0u8,
                (true, false) => 1,
                (false, true) => 2,
            };
            cells.push((d as u32, ti, occ));
        }
    }
    cells
}

/// JI.8 seam facts, one batch of cells in chunks (selection 5). Scope all: the occupied cells of every domain;
/// a complete batch carries total N with its on-grip and off-grip parts and the aligned count. Scope cap c:
/// the cells of c's cut when c is admissible; a complete batch carries N_c and the aligned count, and total N
/// is undefined there. A batch that stops early carries no totals (JI.14).
pub fn seam_facts(
    ctx: &Context,
    state: &mut State,
    scope: Scope,
    batch: &Batch,
    rows: Rows,
    eval_budget: Option<u64>,
) -> Result<Value, String> {
    let t = tables(ctx)?;
    let mut fields: Vec<(&str, Value)> = Vec::new();
    let cap = match scope {
        Scope::All => {
            fields.push(("scope", text("all")));
            None
        }
        Scope::Cap(c) => {
            let status = state.classify(ctx, c, false, true, eval_budget).map_err(|e| e.0)?.status();
            fields.extend([("scope", int(c as i64)), ("status", text(status.as_str()))]);
            if status != Status::Admissible {
                // A blocked or invalid cap has no seam facts, which is a complete answer; an uncertain one (the
                // evaluation budget ran out) claims nothing (JI.14).
                fields.extend([
                    ("complete", Value::Bool(status != Status::Uncertain)),
                    ("totals", Value::Null),
                    ("undefined", text("cap_not_admissible")),
                ]);
                return Ok(obj(fields));
            }
            Some(c)
        }
    };
    let facts = pose_facts(ctx, t, state)?;
    let dom = domains(ctx, t, state, &facts)?;
    let cells = match cap {
        None => {
            let mut cells = Vec::new();
            seam_scan(t, &dom, |d, ti, occ| cells.push((d, ti, occ)));
            cells
        }
        Some(c) => cap_cells(ctx, t, &dom, c),
    };
    let writer = (rows != Rows::None).then(|| RowWriter::new(ctx, t, &dom));
    let total = cells.len() as u64;
    let mut progress = Progress { processed: 0, chunks: 0, cancelled: false, cut_off: false };
    let (mut on, mut off, mut aligned) = (0i64, 0i64, 0i64);
    let mut out = Vec::new();
    let mut cache = HashMap::new();
    while let Some(end) = progress.next(batch, total) {
        for &(d, ti, occ) in &cells[progress.processed as usize..end as usize] {
            let listed = rows == Rows::All || (rows == Rows::Mismatched && occ != 0);
            if occ == 0 {
                aligned += 1;
                if !listed {
                    continue;
                }
            }
            let grip = cap.is_some() || on_grip(ctx, &dom, d, t.pole[ti as usize], &mut cache);
            if occ != 0 {
                if grip {
                    on += 1;
                } else {
                    off += 1;
                }
            }
            if let (true, Some(w)) = (listed, &writer) {
                out.push(w.row(d, ti, occ, grip));
            }
        }
        progress.processed = end;
        progress.chunks += 1;
    }
    let complete = progress.processed == total;
    let totals = match (complete, cap) {
        (false, _) => Value::Null,
        (true, None) => obj(vec![
            ("total_N", int(on + off)),
            ("on_grip_mismatches", int(on)),
            ("off_grip_mismatches", int(off)),
            ("aligned", int(aligned)),
        ]),
        (true, Some(_)) => obj(vec![("N_c", int(on)), ("aligned", int(aligned)), ("total_N", Value::Null)]),
    };
    fields.extend([
        ("complete", Value::Bool(complete)),
        ("cells", int(total as i64)),
        ("domains", int(dom.keys.len() as i64)),
        ("totals", totals),
    ]);
    fields.extend(progress.fields());
    if rows != Rows::None {
        fields.push(("rows", Value::Array(out)));
    }
    Ok(obj(fields))
}

/// JI.8 totals over a supplied table of cells: each row names its occupancy and its grip side.
pub fn seam_totals(cells: &[Value]) -> Result<Value, String> {
    let (mut on, mut off, mut aligned) = (0i64, 0i64, 0i64);
    for (i, row) in cells.iter().enumerate() {
        let occupancy = row.get("occupancy").and_then(Value::as_str);
        let grip = row.get("grip").and_then(Value::as_str);
        match (occupancy, grip) {
            (Some("both"), Some("on" | "off")) => aligned += 1,
            (Some("lower" | "upper"), Some("on")) => on += 1,
            (Some("lower" | "upper"), Some("off")) => off += 1,
            _ => return Err(format!("cell {i} needs occupancy both, lower or upper and grip on or off")),
        }
    }
    Ok(obj(vec![
        ("total_N", int(on + off)),
        ("on_grip_mismatches", int(on)),
        ("off_grip_mismatches", int(off)),
        ("aligned", int(aligned)),
    ]))
}

// ---------------------------------------------------------------------------------------------------- units

/// The pieces of a named unit, or the refusal JI.9 names: non-empty, every piece off the lattice, one pose.
fn named_unit(state: &State, pieces: &[u32]) -> Result<Vec<u32>, String> {
    let mut set = pieces.to_vec();
    set.sort_unstable();
    set.dedup();
    if set.is_empty() {
        return Err("a named unit needs at least one piece".into());
    }
    if set.iter().any(|&p| p as usize >= NP) {
        return Err("a named unit holds piece IDs of the model".into());
    }
    if set.iter().any(|&p| state.is_lattice_piece(p as usize)) {
        return Err("a named unit holds only pieces off the lattice".into());
    }
    let pid = state.pose_ids()[set[0] as usize];
    if set.iter().any(|&p| state.pose_ids()[p as usize] != pid) {
        return Err("the pieces of a named unit share one exact pose".into());
    }
    Ok(set)
}

/// JI.9 units: the exact-pose groups of the off pieces (or one named unit), ordered by smallest member.
pub fn units(ctx: &Context, state: &State, named: Option<&[u32]>) -> Result<Value, String> {
    let t = tables(ctx)?;
    let groups: Vec<Vec<u32>> = match named {
        Some(pieces) => match named_unit(state, pieces) {
            Ok(set) => vec![set],
            Err(message) => return Ok(refusal(codes::NAMED_OBJECT_INVALID, &message)),
        },
        None => {
            let mut by_pose: HashMap<u32, Vec<u32>> = HashMap::new();
            for (p, &pid) in state.pose_ids().iter().enumerate() {
                if !state.is_lattice_piece(p) {
                    by_pose.entry(pid).or_default().push(p as u32);
                }
            }
            let mut groups: Vec<Vec<u32>> = by_pose.into_values().collect();
            groups.sort_by_key(|g| g[0]);
            groups
        }
    };
    let mut rows = Vec::new();
    for members in groups {
        let f = t.facts(ctx, state.pose_of(members[0] as usize))?;
        rows.push(obj(vec![
            ("members", ids(&members)),
            ("pose", text(&f.key)),
            ("domain", text(&f.domain)),
            ("heights", pair(f.dl, f.dr)),
            ("branches", pair(f.bl, f.br)),
        ]));
    }
    Ok(obj(vec![("status", text("ok")), ("units", Value::Array(rows)), ("complete", Value::Bool(true))]))
}

// ------------------------------------------------------------------------------------- prefix observations

/// Which optional parts a prefix observation computes. Φ and the largest left height are always observed.
pub struct Fields {
    /// N before and after (one full seam scan per letter).
    pub n: bool,
    /// The X5 histogram delta.
    pub heights: bool,
    /// F, D and their accounting.
    pub fd: bool,
    /// The piece sets behind F and D (`foreign_pieces`, `lowered_high`).
    pub sets: bool,
}

/// Stage 14's accounting of one admissible letter at grip x, read on the state before it acts (README section 1,
/// Lemma Phi, `scripts/t3p.py`): F counts the inside pieces of left height ≥ 2 off the class CLS(x); D counts
/// those of height ≥ 4 on it whose height the letter lowers by 2. Both need the I_a domain (the I_a menu, a
/// letter and inside pieces of right height 0); D is undefined for a retained letter.
fn accounting(
    ctx: &Context,
    t: &Tables,
    menu: Option<&crate::twist::Menu>,
    twist: &Twist,
    inside: &[u32],
    pose_ids: &[u32],
    facts: &[Arc<Facts>],
    sets: bool,
) -> Result<Vec<(&'static str, Value)>, String> {
    let mut out: Vec<(&'static str, Value)> = Vec::new();
    let put = |out: &mut Vec<(&'static str, Value)>, name: &'static str, set: Option<&[u32]>| {
        if sets {
            out.push((name, set.map_or(Value::Null, ids)));
        }
    };
    let of = |p: u32| &facts[pose_ids[p as usize] as usize];
    let (lt, rt) = t.trees.factors(&twist.matrix)?;
    let domain =
        menu.is_some_and(|m| m.name == "I_a") && t.trees.dist(&rt)? == 0 && inside.iter().all(|&p| of(p).dr == 0);
    if !domain {
        put(&mut out, "foreign_pieces", None);
        put(&mut out, "lowered_high", None);
        out.extend([("F", Value::Null), ("D", Value::Null), ("accounting", text("undefined(outside_ia_domain)"))]);
        return Ok(out);
    }
    let class = t.cls(ctx, twist.grip)?;
    let foreign: Vec<u32> = inside.iter().copied().filter(|&p| of(p).dl >= 2 && of(p).bl != class).collect();
    put(&mut out, "foreign_pieces", Some(&foreign));
    out.push(("F", int(foreign.len() as i64)));
    if twist.kidx.is_some() {
        put(&mut out, "lowered_high", None);
        out.extend([("D", Value::Null), ("accounting", text("undefined(retained_letter)"))]);
        return Ok(out);
    }
    let mut lowers: HashMap<u32, bool> = HashMap::new();
    let mut lowered = Vec::new();
    for &p in inside {
        let f = of(p);
        if f.dl < 4 || f.bl != class {
            continue;
        }
        let pid = pose_ids[p as usize];
        let low = match lowers.get(&pid) {
            Some(&v) => v,
            None => {
                let v = t.trees.dist(&qmul(&lt, &f.left))? == f.dl - 2;
                lowers.insert(pid, v);
                v
            }
        };
        if low {
            lowered.push(p);
        }
    }
    put(&mut out, "lowered_high", Some(&lowered));
    out.extend([("D", int(lowered.len() as i64)), ("accounting", text("defined"))]);
    Ok(out)
}

/// A prefix-observation request (JI.10): the literal letters, the optional fields, an optional named cap for
/// the straddler counts u, the batch limits, whether to keep the prefix states and a classification budget.
pub struct PrefixRequest<'a> {
    pub letters: &'a [([i64; 2], Twist)],
    pub fields: Fields,
    pub cap: Option<usize>,
    pub batch: Batch,
    pub keep: bool,
    pub eval_budget: Option<u64>,
}

/// The straddler count of a named cap (None without a cap); a budget that leaves it uncertain is an error.
fn straddler_count(
    ctx: &Context,
    state: &mut State,
    cap: Option<usize>,
    eval_budget: Option<u64>,
) -> Result<Option<i64>, String> {
    let Some(c) = cap else { return Ok(None) };
    let set = state.straddler_set(ctx, c, eval_budget).map_err(|e| e.0)?;
    if set.uncertain > 0 {
        return Err(format!("cap {c}: the straddler set is uncertain under the budget"));
    }
    Ok(Some(set.pieces.len() as i64))
}

/// JI.10 prefix observations of a word from `start` (which is not changed). Each row observes one letter;
/// the batch stops at the first refused letter. A complete batch carries the final claims: end digest, peak
/// left height (the start included), excursion (the greatest rise of Φ over the start) and tameness (no
/// letter raises Φ). With `keep`, the state after every admissible row is returned for the caller to store.
pub fn prefix_observations(ctx: &Context, start: &State, req: &PrefixRequest) -> Result<(Value, Vec<State>), String> {
    let t = tables(ctx)?;
    let (letters, fields) = (req.letters, &req.fields);
    let mut work = start.clone();
    let mut stats = stats_of(ctx, t, &work)?;
    let (phi0, mut peak, mut max_phi, mut tame) = (stats.phi, stats.max, stats.phi, true);
    let mut n_prev = if fields.n { Some(total_n(ctx, t, &work)?) } else { None };
    let mut u_prev = straddler_count(ctx, &mut work, req.cap, req.eval_budget)?;
    let start_fields = obj(vec![
        ("digest", text(&work.digest(ctx))),
        ("phi", int(stats.phi)),
        ("max_left_height", int(stats.max)),
        ("N", opt_int(n_prev)),
        ("u", opt_int(u_prev)),
    ]);
    let mut progress = Progress { processed: 0, chunks: 0, cancelled: false, cut_off: false };
    let mut rows = Vec::new();
    let mut kept = Vec::new();
    let mut refused: Option<usize> = None;
    'batch: while let Some(end) = progress.next(&req.batch, letters.len() as u64) {
        for i in progress.processed as usize..end as usize {
            let (letter, twist) = &letters[i];
            let before_digest = work.digest(ctx);
            let before = if fields.fd { Some((work.pose_ids().to_vec(), pose_facts(ctx, t, &work)?)) } else { None };
            let outcome = work.apply(ctx, twist, req.eval_budget, &[]).map_err(|e| e.0)?;
            let mut row = vec![
                ("index", int(i as i64)),
                ("letter", pair(letter[0], letter[1])),
                ("status", text(outcome.status.as_str())),
                ("code", outcome.refusal.as_ref().map_or(Value::Null, |(code, _)| text(code))),
                ("certificate", outcome.straddle.as_ref().map_or(Value::Null, Certificate::to_json)),
                ("before", text(&before_digest)),
                ("after", text(&work.digest(ctx))),
                ("moved", int(outcome.moved() as i64)),
                ("moved_set", text(&moved_set_hash(&outcome.inside))),
            ];
            if outcome.status != Status::Admissible {
                rows.push(obj(row));
                refused = Some(i);
                progress.processed = i as u64 + 1;
                progress.chunks += 1;
                break 'batch;
            }
            let after = stats_of(ctx, t, &work)?;
            row.extend([
                ("phi_before", int(stats.phi)),
                ("phi_after", int(after.phi)),
                ("delta_phi", int(after.phi - stats.phi)),
                ("max_left_height", int(after.max)),
            ]);
            if fields.heights {
                row.push(("delta_n_h", delta_counts(&stats.counts, &after.counts)));
            }
            if fields.n {
                let n = total_n(ctx, t, &work)?;
                row.extend([
                    ("N_before", opt_int(n_prev)),
                    ("N_after", int(n)),
                    ("delta_N", int(n - n_prev.unwrap_or(0))),
                ]);
                n_prev = Some(n);
            }
            if req.cap.is_some() {
                let u = straddler_count(ctx, &mut work, req.cap, req.eval_budget)?;
                let delta = u_prev.zip(u).map(|(b, a)| a - b);
                row.extend([("u_before", opt_int(u_prev)), ("u_after", opt_int(u)), ("delta_u", opt_int(delta))]);
                u_prev = u;
            }
            if let Some((ids_before, facts_before)) = &before {
                let menu = work.menu();
                row.extend(accounting(ctx, t, menu, twist, &outcome.inside, ids_before, facts_before, fields.sets)?);
            }
            if req.keep {
                row.push(("prefix_state", text(&format!("prefix/{i}"))));
                kept.push(work.clone());
            }
            tame &= after.phi <= stats.phi;
            peak = peak.max(after.max);
            max_phi = max_phi.max(after.phi);
            stats = after;
            rows.push(obj(row));
        }
        progress.processed = end;
        progress.chunks += 1;
    }
    let complete = progress.processed == letters.len() as u64 && refused.is_none();
    let final_claims = if complete {
        obj(vec![
            ("end_digest", text(&work.digest(ctx))),
            ("peak_left_height", int(peak)),
            ("excursion", int(max_phi - phi0)),
            ("tame", Value::Bool(tame)),
        ])
    } else {
        Value::Null
    };
    let mut out = vec![
        ("start", start_fields),
        ("letters", int(letters.len() as i64)),
        ("cap", opt_int(req.cap.map(|c| c as i64))),
        ("rows", Value::Array(rows)),
        ("complete", Value::Bool(complete)),
        ("first_refused", opt_int(refused.map(|i| i as i64))),
        ("final", final_claims),
    ];
    out.extend(progress.fields());
    Ok((obj(out), kept))
}

// ------------------------------------------------------------------------------------------------ word check

/// JI.11 word check from `start` (not changed), in the reference's terms (`differential/references/
/// word_check.py`): the unit's landing, lattice pieces lifted off, the other starting-off pieces landed,
/// exact or changed, the off counts, ΔN, the X5 height counts and, at a named cap, the straddler counts u.
/// The step flag is the LB3.8 conjunction and is undefined without a named unit; peak height, excursion
/// and tameness need the whole word applied.
pub fn word_check(
    ctx: &Context,
    start: &State,
    letters: &[([i64; 2], Twist)],
    unit: Option<&[u32]>,
    cap: Option<usize>,
    eval_budget: Option<u64>,
) -> Result<Value, String> {
    let t = tables(ctx)?;
    let unit = match unit.map(|u| named_unit(start, u)).transpose() {
        Ok(unit) => unit,
        Err(message) => return Ok(refusal(codes::NAMED_OBJECT_INVALID, &message)),
    };
    let lattice_before: Vec<bool> = (0..NP).map(|p| start.is_lattice_piece(p)).collect();
    let off0: Vec<u32> = (0..NP as u32).filter(|&p| !lattice_before[p as usize]).collect();
    let stats0 = stats_of(ctx, t, start)?;
    let n_before = total_n(ctx, t, start)?;
    let mut end = start.clone();
    let u_before = straddler_count(ctx, &mut end, cap, eval_budget)?;
    let mut counts = vec![off0.len() as i64];
    let (mut failed, mut failing_status, mut applied) = (None, None, 0usize);
    let (mut peak, mut max_phi, mut tame, mut phi_prev) = (stats0.max, stats0.phi, true, stats0.phi);
    for (index, (_, twist)) in letters.iter().enumerate() {
        let outcome = end.apply(ctx, twist, eval_budget, &[]).map_err(|e| e.0)?;
        if outcome.status != Status::Admissible {
            failed = Some(index);
            failing_status = Some(outcome.status.as_str());
            break;
        }
        applied += 1;
        counts.push(end.off_lattice_count() as i64);
        let s = stats_of(ctx, t, &end)?;
        tame &= s.phi <= phi_prev;
        phi_prev = s.phi;
        peak = peak.max(s.max);
        max_phi = max_phi.max(s.phi);
    }
    let lattice_after: Vec<bool> = (0..NP).map(|p| end.is_lattice_piece(p)).collect();
    let unit_set: HashSet<u32> = unit.iter().flatten().copied().collect();
    let other: Vec<u32> = off0.iter().copied().filter(|p| !unit_set.contains(p)).collect();
    let changed: Vec<u32> = off0
        .iter()
        .copied()
        .filter(|&p| !lattice_after[p as usize] && end.pose_of(p as usize) != start.pose_of(p as usize))
        .collect();
    let changed_set: HashSet<u32> = changed.iter().copied().collect();
    let others_changed: Vec<u32> = other.iter().copied().filter(|p| changed_set.contains(p)).collect();
    let exact = other.iter().filter(|&&p| !lattice_after[p as usize] && !changed_set.contains(&p)).count() as i64;
    let landed = other.iter().filter(|&&p| lattice_after[p as usize]).count() as i64;
    let collateral: Vec<u32> =
        (0..NP as u32).filter(|&p| lattice_before[p as usize] && !lattice_after[p as usize]).collect();
    let restored = off0.iter().filter(|&&p| lattice_after[p as usize]).count() as i64;
    let stats1 = stats_of(ctx, t, &end)?;
    let u_after = straddler_count(ctx, &mut end, cap, eval_budget)?;
    let admissible = failed.is_none();
    let on_lattice = unit.as_ref().map(|u| u.iter().filter(|&&p| lattice_after[p as usize]).count() as i64);
    let step_flag = match (&unit, on_lattice) {
        (Some(u), Some(on)) => Value::Bool(
            admissible && on == u.len() as i64 && collateral.is_empty() && exact + landed == other.len() as i64,
        ),
        _ => Value::Null,
    };
    let claims = |v: Value| if admissible { v } else { Value::Null };
    // The reference's word digest: SHA-256 of the chronological [grip, item] pairs as compact JSON.
    let pairs: Vec<String> = letters.iter().map(|([c, x], _)| format!("[{c},{x}]")).collect();
    let word_digest = c600_digest::sha256_hex(format!("[{}]", pairs.join(",")).as_bytes());
    Ok(obj(vec![
        ("status", text("ok")),
        ("start_digest", text(&start.digest(ctx))),
        ("end_digest", text(&end.digest(ctx))),
        ("letters", int(letters.len() as i64)),
        ("word_digest", text(&word_digest)),
        ("unit", unit.as_ref().map_or(Value::Null, |u| ids(u))),
        ("unit_size", opt_int(unit.as_ref().map(|u| u.len() as i64))),
        ("unit_on_lattice", opt_int(on_lattice)),
        ("unit_landed", unit.as_ref().map_or(Value::Null, |u| Value::Bool(on_lattice == Some(u.len() as i64)))),
        ("lattice_pieces_lifted_off", int(collateral.len() as i64)),
        ("collateral_ids", ids(&collateral)),
        ("others", int(other.len() as i64)),
        ("others_exact", int(exact)),
        ("others_landed", int(landed)),
        ("others_changed_ids", ids(&others_changed)),
        ("off_before", int(off0.len() as i64)),
        ("off_after", int(lattice_after.iter().filter(|&&on| !on).count() as i64)),
        ("off_min", int(*counts.iter().min().unwrap_or(&0))),
        ("off_max", int(*counts.iter().max().unwrap_or(&0))),
        ("off_pieces_landed", int(restored)),
        ("pose_changed", int(changed.len() as i64)),
        ("pose_changed_ids", ids(&changed)),
        ("delta_N", int(total_n(ctx, t, &end)? - n_before)),
        ("heights_before", counts_value(&stats0.counts)),
        ("heights_after", counts_value(&stats1.counts)),
        ("delta_n_h", delta_counts(&stats0.counts, &stats1.counts)),
        ("cap", opt_int(cap.map(|c| c as i64))),
        ("u_before", opt_int(u_before)),
        ("u_after", opt_int(u_after)),
        ("delta_u", opt_int(u_before.zip(u_after).map(|(b, a)| a - b))),
        ("admissible", Value::Bool(admissible)),
        ("failing_letter_index", opt_int(failed.map(|i| i as i64))),
        ("failing_status", failing_status.map_or(Value::Null, text)),
        ("applied_letters", int(applied as i64)),
        ("processed", pair(0, (applied + failed.is_some() as usize) as i64)),
        ("step_flag", step_flag),
        ("step_flag_undefined", if unit.is_none() { text("unit_not_named") } else { Value::Null }),
        ("peak_left_height", claims(int(peak))),
        ("excursion", claims(int(max_phi - stats0.phi))),
        ("tame", claims(Value::Bool(tame))),
    ]))
}

/// The exact rotation of a Cayley parameter at a grip, as a twist record (free-mode tests).
pub fn cayley_record(ctx: &Context, grip: usize, omega: &[Q5]) -> Value {
    let m = crate::twist::cayley_matrix(&ctx.model.poles[grip], omega, &ctx.model.nn);
    obj(vec![
        ("grip", int(grip as i64)),
        ("family", text("cayley")),
        ("params", obj(vec![("omega", Value::Array(omega.iter().map(Q5::to_json).collect()))])),
        ("matrix", matrix_json(&m)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_delta_keeps_odd_bins() {
        // One S4 letter from solved: 3,097 pieces move from height 0 to height 1 (the review's case on PR 167).
        let before = BTreeMap::from([(0, 3120)]);
        let after = BTreeMap::from([(0, 23), (1, 3097)]);
        let delta = delta_counts(&before, &after);
        let fields = delta.as_object().unwrap();
        assert_eq!(fields.get("n_0").and_then(Value::as_i64), Some(-3097));
        assert_eq!(fields.get("n_1").and_then(Value::as_i64), Some(3097));
        for key in COUNT_KEYS {
            assert!(fields.contains_key(key), "{key}");
        }
        assert_eq!(fields.values().map(|v| v.as_i64().unwrap()).sum::<i64>(), 0);
        // On I_a states the delta has J1's five keys only, and heights above 6 share one bin.
        let ia = delta_counts(&BTreeMap::from([(0, 10), (2, 4)]), &BTreeMap::from([(0, 8), (4, 2), (8, 4)]));
        let keys: Vec<&str> = ia.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, COUNT_KEYS.to_vec());
        assert_eq!(ia.as_object().unwrap().get("n_above_6").and_then(Value::as_i64), Some(4));
    }

    #[test]
    fn valuation_at_two() {
        assert_eq!(v2(&Q5::int(8)).unwrap(), 3);
        assert_eq!(v2(&Q5::of(1, 0, 4)).unwrap(), -2);
        // φ = (1 + √5)/2 is a unit; √5 = 2φ − 1 is a unit too.
        assert_eq!(v2(&Q5::of(1, 1, 2)).unwrap(), 0);
        assert_eq!(v2(&Q5::of(0, 1, 1)).unwrap(), 0);
        // 2 + 2√5 = 4φ.
        assert_eq!(v2(&Q5::of(2, 2, 1)).unwrap(), 2);
        assert!(v2(&Q5::zero()).is_err());
    }

    #[test]
    fn tree_data_of_trees_py() {
        let trees = Trees::build().unwrap();
        assert_eq!(trees.units.len(), 120);
        assert_eq!(trees.punits.len(), 60);
        assert_eq!(trees.tets.len(), 5);
        assert!(trees.tets.iter().all(|t| t.len() == 12));
        // Every unit icosian lies at distance 0; 1 + i has norm 2 and distance 1.
        for u in &trees.units {
            assert_eq!(trees.dist(u).unwrap(), 0);
        }
        let x = [Q5::int(1), Q5::int(1), Q5::zero(), Q5::zero()];
        assert_eq!(trees.dist(&x).unwrap(), 1);
        // The identity rotation factors as (1, 1) up to scale.
        let (l, r) = trees.factors(&identity4()).unwrap();
        assert_eq!((trees.dist(&l).unwrap(), trees.dist(&r).unwrap()), (0, 0));
    }

    #[test]
    fn batch_progress_reports_half_open_coverage() {
        let batch = Batch { chunk: 2, budget: Some(5), cancel_after: None };
        let mut p = Progress { processed: 0, chunks: 0, cancelled: false, cut_off: false };
        let mut ends = Vec::new();
        while let Some(end) = p.next(&batch, 9) {
            ends.push(end);
            p.processed = end;
            p.chunks += 1;
        }
        assert_eq!(ends, vec![2, 4, 5]);
        assert!(p.cut_off && !p.cancelled);
        let batch = Batch { chunk: 1, budget: None, cancel_after: Some(1) };
        let mut p = Progress { processed: 0, chunks: 0, cancelled: false, cut_off: false };
        while let Some(end) = p.next(&batch, 2) {
            p.processed = end;
            p.chunks += 1;
        }
        assert_eq!(p.processed, 1);
        assert!(p.cancelled);
    }
}
