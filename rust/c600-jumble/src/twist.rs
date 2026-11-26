//! Exact twists (grip, g) and twist menus (amendment A4 of the state contract).
//!
//! A twist's matrix is authoritative: it must be an exact rotation that fixes the grip's pole. Records of the
//! families `menu`, `matrix` and `cayley` are read (selection 2 of the LP2 plan, section 11); another family, or a
//! float request (`params.requested`), is an adapter error and no refusal. A twist's rotation plane, which a swept
//! preview needs, is named exactly by the engine ([`rotation_plane`]).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

use c600_kernel::json::Value;

use crate::model::POLES;
use crate::q5::{dot, is_rotation, matmul, matrix_compact, matrix_from_json, matvec, rank, transpose, Mat4, Vec4, Q5};
use crate::Context;

/// Why a record or menu cannot be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordError {
    /// The reference refuses it too: the step is `invalid`.
    Invalid(String),
    /// Outside what this candidate reads (selection 2): an adapter error, never a refusal.
    Unsupported(String),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordError::Invalid(m) => write!(f, "invalid: {m}"),
            RecordError::Unsupported(m) => write!(f, "unsupported: {m}"),
        }
    }
}

fn invalid(message: impl Into<String>) -> RecordError {
    RecordError::Invalid(message.into())
}

#[derive(Clone, Debug)]
pub struct Twist {
    pub grip: usize,
    pub matrix: Arc<Mat4>,
    /// The K⁺ index of the matrix, if it lies in K⁺.
    pub kidx: Option<u16>,
}

impl Twist {
    /// Checks the matrix exactly: gᵀg = I, det g = 1 and g n_grip = n_grip.
    pub fn new(ctx: &Context, grip: usize, matrix: Mat4) -> Result<Twist, RecordError> {
        if grip >= POLES {
            return Err(invalid("grip must be a pole index 0..599"));
        }
        if !is_rotation(&matrix) {
            return Err(invalid("matrix is not exactly a rotation (g^T g = I, det g = 1)"));
        }
        let n = &ctx.model.poles[grip];
        if &matvec(&matrix, n) != n {
            return Err(invalid("matrix does not fix the pole of the grip"));
        }
        let kidx = ctx.kplus.index_of_matrix(&matrix, &ctx.model);
        Ok(Twist { grip, matrix: Arc::new(matrix), kidx })
    }

    /// A journal record; the exact matrix is authoritative and a Cayley parameter must reproduce it exactly.
    pub fn from_record(ctx: &Context, rec: &Value) -> Result<Twist, RecordError> {
        let rec = rec.as_object().ok_or_else(|| invalid("a record must be a mapping"))?;
        // Selection 2: whether the record is one the candidate reads at all is settled before any refusal.
        let family = match rec.get("family") {
            None => "matrix",
            Some(Value::String(f)) => f.as_str(),
            Some(_) => return Err(RecordError::Unsupported("the record family is not a string".into())),
        };
        if !matches!(family, "menu" | "matrix" | "cayley") {
            return Err(RecordError::Unsupported(format!("record family {family:?} is not read")));
        }
        let params = match rec.get("params") {
            None | Some(Value::Null) => None,
            Some(Value::Object(p)) => Some(p),
            Some(_) => return Err(RecordError::Unsupported("record params must be a mapping".into())),
        };
        if params.is_some_and(|p| p.contains_key("requested")) {
            return Err(RecordError::Unsupported("float requests (params.requested) are not read".into()));
        }
        let grip = match rec.get("grip") {
            Some(Value::Int(g)) if (0..POLES as i64).contains(g) => *g as usize,
            _ => return Err(invalid("grip must be an integer pole index 0..599")),
        };
        let matrix =
            matrix_from_json(rec.get("matrix").ok_or_else(|| invalid("a record needs a matrix"))?).map_err(invalid)?;
        let twist = Twist::new(ctx, grip, matrix)?;
        match family {
            "cayley" => {
                let omega = params
                    .and_then(|p| p.get("omega"))
                    .and_then(Value::as_array)
                    .ok_or_else(|| invalid("cayley record: omega needs three components"))?;
                let omega: Vec<Q5> =
                    omega.iter().map(Q5::from_json_reduced).collect::<Result<_, _>>().map_err(invalid)?;
                if omega.len() != 3 || cayley_matrix(&ctx.model.poles[grip], &omega, &ctx.model.nn) != *twist.matrix {
                    return Err(invalid("cayley record: parameter does not reproduce the matrix"));
                }
            }
            _ => {}
        }
        if let Some(flag) = rec.get("retained") {
            if truthy(flag) != twist.kidx.is_some() {
                return Err(invalid("record retained flag disagrees with the matrix"));
            }
        }
        Ok(twist)
    }

    /// The engine-named plane of the twist's rotation ([`rotation_plane`]); `None` for the identity.
    pub fn plane(&self) -> Option<RotationPlane> {
        rotation_plane(&self.matrix)
    }
}

/// The plane a rotation turns, named by the engine so that its motion can be swept (render/1 section 4, RM.6).
///
/// Everything is exact, and the vectors are not normalized: |u| and |v| are square roots that Q(√5) need not hold.
/// u and v span the plane that g moves, the image of g − I, and u·v = 0. With θ in (0, π] the angle g turns that plane
/// by, cos θ = (tr g − 2)/2 and g u = cos θ·u + sin θ·(|u|/|v|)·v: the sweep R(φ) of render/1 section 4, built from
/// u/|u| and v/|v|, turns u toward v and equals g at φ = θ. A display normalizes the vectors and takes θ = acos(cos θ).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RotationPlane {
    /// The first nonzero column of g − I.
    pub u: Vec4,
    /// For θ < π, g u − cos θ·u, so g u = cos θ·u + v and |v| = sin θ·|u|. For a half-turn, the first later column of
    /// g − I that is independent of u, made orthogonal to u: g alone does not fix a half-turn's direction, so this is
    /// the engine's naming of it.
    pub v: Vec4,
    /// cos θ.
    pub cos: Q5,
}

impl RotationPlane {
    /// θ = π, so the sweep's direction is the engine's naming ([`RotationPlane::v`]).
    pub fn is_half_turn(&self) -> bool {
        self.cos == Q5::int(-1)
    }
}

/// The engine-named plane of a rotation g that fixes a plane pointwise ([`RotationPlane`]). `None` for the identity,
/// for a matrix that is not exactly a rotation, and for a rotation that turns two planes (g − I of rank 4), which has
/// no one plane. A twist fixes its grip's pole, so every twist but the identity has a plane.
pub fn rotation_plane(g: &Mat4) -> Option<RotationPlane> {
    if !is_rotation(g) {
        return None;
    }
    let one = Q5::one();
    let columns: Vec<Vec4> =
        (0..4).map(|j| std::array::from_fn(|i| if i == j { g[i][j].sub(&one) } else { g[i][j].clone() })).collect();
    if rank(&columns.iter().map(|c| c.to_vec()).collect::<Vec<_>>()) != 2 {
        return None;
    }
    let trace = (0..4).fold(Q5::zero(), |sum, i| sum.add(&g[i][i]));
    let cos = trace.sub(&Q5::int(2)).mul(&Q5::of(1, 0, 2));
    let first = columns.iter().position(|c| c.iter().any(|x| !x.is_zero()))?;
    let u = columns[first].clone();
    let v = if cos != Q5::int(-1) {
        let gu = matvec(g, &u);
        std::array::from_fn(|i| gu[i].sub(&cos.mul(&u[i])))
    } else {
        let uu = dot(&u, &u);
        columns[first + 1..].iter().find_map(|c| {
            let t = dot(c, &u).div(&uu)?;
            let w: Vec4 = std::array::from_fn(|i| c[i].sub(&t.mul(&u[i])));
            w.iter().any(|x| !x.is_zero()).then_some(w)
        })?
    };
    Some(RotationPlane { u, v, cos })
}

/// Python truthiness of a JSON value, as the reference reads the `retained` flag.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::Number(lexeme) => {
            let mantissa = lexeme.split(['e', 'E']).next().unwrap_or("");
            mantissa.bytes().any(|c| c.is_ascii_digit() && c != b'0')
        }
        Value::String(s) => !s.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

/// The exact frame (u₁, u₂, u₃) of n⊥; each |u_i|² = |n|².
pub fn cap_frame(n: &[Q5]) -> [Vec4; 3] {
    [
        [n[1].neg(), n[0].clone(), n[3].neg(), n[2].clone()],
        [n[2].neg(), n[3].clone(), n[0].clone(), n[1].neg()],
        [n[3].neg(), n[2].neg(), n[1].clone(), n[0].clone()],
    ]
}

/// The exact Cayley rotation fixing n with parameter ω in the frame `cap_frame(n)`.
pub fn cayley_matrix(n: &[Q5], omega: &[Q5], nn: &Q5) -> Mat4 {
    let (w1, w2, w3) = (&omega[0], &omega[1], &omega[2]);
    let z = Q5::zero();
    let w = [[z.clone(), w3.neg(), w2.clone()], [w3.clone(), z.clone(), w1.neg()], [w2.neg(), w1.clone(), z.clone()]];
    let w_sq: [[Q5; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| (0..3).fold(Q5::zero(), |s, k| s.add(&w[i][k].mul(&w[k][j])))));
    let norm = Q5::one().add(&w1.mul(w1)).add(&w2.mul(w2)).add(&w3.mul(w3));
    let f = Q5::int(2).div(&norm).expect("1 + |w|^2 > 0");
    let r: [[Q5; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let delta = if i == j { Q5::one() } else { Q5::zero() };
            delta.add(&f.mul(&w[i][j].add(&w_sq[i][j])))
        })
    });
    let u = cap_frame(n);
    let inv = nn.inv().expect("|n|^2 > 0");
    std::array::from_fn(|a| {
        std::array::from_fn(|b| {
            let mut s = n[a].mul(&n[b]);
            for i in 0..3 {
                for j in 0..3 {
                    s = s.add(&r[i][j].mul(&u[i][a]).mul(&u[j][b]));
                }
            }
            s.mul(&inv)
        })
    })
}

/// SHA-256 of `[` + the keys joined by commas + `]`: the menu identity (sorted keys) or order digest (record order).
pub fn keys_digest(keys: &[String]) -> String {
    let text = format!("[{}]", keys.join(","));
    c600_digest::sha256_hex(text.as_bytes())
}

/// A twist menu Λ₀ of rotations fixing n₀, transported to cap c by its retained frame: Λ_c = F_c Λ₀ F_c⁻¹.
pub struct Menu {
    pub name: String,
    pub items: Vec<(String, Mat4)>,
    keys: HashMap<Mat4, usize>,
    /// The order-free identity: SHA-256 over the sorted compact item keys.
    pub identity: String,
    /// The menu-order digest: the same keys in item order.
    pub order: String,
    /// The twists at the caps used most recently, newest first ([`Menu::for_cap`]).
    kept: Mutex<VecDeque<KeptCap>>,
}

/// How many caps' twists a menu keeps (C9, 26 Nov 2026). Building a cap's twists was the largest pure cost of a twist's
/// commit (about 5 ms for the 60-item menu); one cap's twists take about 200 KB.
const KEPT_CAPS: usize = 32;

/// One cap's twists with the model, frame and pole they were built from.
struct KeptCap {
    cap: usize,
    model: Vec<(String, String)>,
    frame: Mat4,
    pole: Vec4,
    twists: Arc<[Twist]>,
}

impl Menu {
    /// A menu record (`TwistMenu.from_record` with `close=False`): exact rotations fixing n₀, no duplicates,
    /// inverse-closed, A4-conjugation-invariant and containing A4; stated fields must agree.
    pub fn from_record(ctx: &Context, rec: &Value) -> Result<Menu, String> {
        let rec = rec.as_object().ok_or("a menu record must be a mapping")?;
        let name = rec.get("name").and_then(Value::as_str).ok_or("a menu record needs a name")?.to_owned();
        let base = rec
            .get("base")
            .and_then(Value::as_array)
            .filter(|b| !b.is_empty())
            .ok_or("menu base must be a non-empty list")?;
        let n0 = &ctx.model.poles[0];
        let mut items = Vec::with_capacity(base.len());
        let mut keys = HashMap::with_capacity(base.len());
        for (index, item) in base.iter().enumerate() {
            let label =
                item.get("label").and_then(Value::as_str).ok_or(format!("menu item {index} needs a string label"))?;
            let m = matrix_from_json(item.get("matrix").ok_or(format!("menu item {index} needs a matrix"))?)
                .map_err(|e| format!("menu item {index}: {e}"))?;
            if !is_rotation(&m) || &matvec(&m, n0) != n0 {
                return Err(format!("menu element {label} is not an exact rotation fixing n_0"));
            }
            if keys.insert(m.clone(), index).is_some() {
                return Err("the menu has duplicate items".into());
            }
            items.push((label.to_owned(), m));
        }
        let stab: Vec<&Mat4> = ctx.kplus.stab0.iter().map(|&s| ctx.kplus.matrix(s, &ctx.model)).collect();
        let inverse_closed = items.iter().all(|(_, m)| keys.contains_key(&transpose(m)));
        let a4_invariant =
            items.iter().all(|(_, m)| stab.iter().all(|s| keys.contains_key(&matmul(&matmul(s, m), &transpose(s)))));
        let contains_a4 = stab.iter().all(|s| keys.contains_key(*s));
        if !(inverse_closed && a4_invariant && contains_a4) {
            return Err("menu must be inverse-closed, A4-conjugation-invariant and contain A4".into());
        }
        let in_order: Vec<String> = items.iter().map(|(_, m)| matrix_compact(m)).collect();
        let mut sorted = in_order.clone();
        sorted.sort();
        let identity = keys_digest(&sorted);
        let order = keys_digest(&in_order);
        for (field, value) in [
            ("inverse_closed", inverse_closed),
            ("a4_invariant", a4_invariant),
            ("contains_a4", contains_a4),
            ("invariant", a4_invariant),
        ] {
            let agrees = match rec.get(field) {
                None => true,
                Some(Value::Bool(b)) => *b == value,
                Some(Value::Int(n)) => *n == value as i64,
                Some(_) => false,
            };
            if !agrees {
                return Err(format!("menu record {field} disagrees with exact matrices"));
            }
        }
        if let Some(stated) = rec.get("identity") {
            if stated.as_str() != Some(identity.as_str()) {
                return Err("menu record identity disagrees with exact matrices".into());
            }
        }
        Ok(Menu { name, items, keys, identity, order, kept: Mutex::new(VecDeque::new()) })
    }

    /// The menu's twists at cap c, in item order. The twists of the [`KEPT_CAPS`] caps used most recently are kept and
    /// reused only while `ctx`'s model identity (the digests of its model files, from which its K⁺ table is built) and
    /// its frame and pole at c equal the ones they were built from, so a kept answer is the one a fresh build gives. A
    /// build that fails is not kept.
    pub fn for_cap(&self, ctx: &Context, c: usize) -> Result<Vec<Twist>, RecordError> {
        let f = ctx.kplus.matrix(ctx.kplus.frame_idx[c], &ctx.model);
        let pole = ctx.model.poles.get(c);
        if let Some(twists) = pole.and_then(|pole| self.kept_at(c, &ctx.model.identity, f, pole)) {
            return Ok(twists.to_vec());
        }
        let ft = transpose(f);
        let twists: Vec<Twist> =
            self.items.iter().map(|(_, m)| Twist::new(ctx, c, matmul(&matmul(f, m), &ft))).collect::<Result<_, _>>()?;
        if let Some(pole) = pole {
            self.keep(c, &ctx.model.identity, f, pole, &twists);
        }
        Ok(twists)
    }

    fn kept_at(&self, cap: usize, model: &[(String, String)], frame: &Mat4, pole: &Vec4) -> Option<Arc<[Twist]>> {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        let at = kept.iter().position(|k| k.cap == cap && k.model == model && k.frame == *frame && k.pole == *pole)?;
        let entry = kept.remove(at)?;
        let twists = entry.twists.clone();
        kept.push_front(entry);
        Some(twists)
    }

    fn keep(&self, cap: usize, model: &[(String, String)], frame: &Mat4, pole: &Vec4, twists: &[Twist]) {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        kept.retain(|k| k.cap != cap);
        let model = model.to_vec();
        kept.push_front(KeptCap { cap, model, frame: frame.clone(), pole: pole.clone(), twists: twists.into() });
        kept.truncate(KEPT_CAPS);
    }

    pub fn contains(&self, ctx: &Context, twist: &Twist) -> bool {
        let f = ctx.kplus.matrix(ctx.kplus.frame_idx[twist.grip], &ctx.model);
        self.keys.contains_key(&matmul(&matmul(&transpose(f), &twist.matrix), f))
    }
}

/// The exact rotation of a K⁺ element as a twist at grip c, when it fixes n_c.
pub fn kplus_twist(ctx: &Context, c: usize, k: u16) -> Result<Twist, RecordError> {
    Twist::new(ctx, c, ctx.kplus.matrix(k, &ctx.model).clone())
}
