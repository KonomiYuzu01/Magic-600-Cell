//! Exact arithmetic in Q(√5) for chart-property checks (LB4.2, RPI.30): the field of the 600-cell's coordinates and of
//! the matrices of K⁺. Every operation is checked: an overflow gives `None`, and a check that meets one decides
//! nothing. Nothing here is a float except [`Q5::approx`], which is for display and decides nothing (RPI.29).

use std::collections::HashSet;
use std::fmt;

/// An element (a + b√5) / d of Q(√5) in lowest terms with d > 0, so equal values have equal fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Q5 {
    a: i128,
    b: i128,
    d: i128,
}

fn gcd(mut x: u128, mut y: u128) -> u128 {
    while y != 0 {
        (x, y) = (y, x % y);
    }
    x
}

impl Q5 {
    pub const ZERO: Q5 = Q5 { a: 0, b: 0, d: 1 };
    pub const ONE: Q5 = Q5 { a: 1, b: 0, d: 1 };

    /// (a + b√5) / d in lowest terms; `None` when d is zero or a value overflows.
    pub fn new(a: i128, b: i128, d: i128) -> Option<Q5> {
        if d == 0 {
            return None;
        }
        let (a, b, d) = if d < 0 { (a.checked_neg()?, b.checked_neg()?, d.checked_neg()?) } else { (a, b, d) };
        let g = i128::try_from(gcd(gcd(a.unsigned_abs(), b.unsigned_abs()), d.unsigned_abs())).ok()?;
        Some(Q5 { a: a / g, b: b / g, d: d / g })
    }

    pub const fn int(n: i128) -> Q5 {
        Q5 { a: n, b: 0, d: 1 }
    }

    /// The golden ratio φ = (1 + √5) / 2.
    pub const fn phi() -> Q5 {
        Q5 { a: 1, b: 1, d: 2 }
    }

    /// The fields (a, b, d) of (a + b√5) / d.
    pub fn parts(self) -> (i128, i128, i128) {
        (self.a, self.b, self.d)
    }

    pub fn is_zero(self) -> bool {
        self.a == 0 && self.b == 0
    }

    pub fn neg(self) -> Option<Q5> {
        Some(Q5 { a: self.a.checked_neg()?, b: self.b.checked_neg()?, d: self.d })
    }

    pub fn add(self, o: Q5) -> Option<Q5> {
        let a = self.a.checked_mul(o.d)?.checked_add(o.a.checked_mul(self.d)?)?;
        let b = self.b.checked_mul(o.d)?.checked_add(o.b.checked_mul(self.d)?)?;
        Q5::new(a, b, self.d.checked_mul(o.d)?)
    }

    pub fn sub(self, o: Q5) -> Option<Q5> {
        self.add(o.neg()?)
    }

    pub fn mul(self, o: Q5) -> Option<Q5> {
        let a = self.a.checked_mul(o.a)?.checked_add(self.b.checked_mul(o.b)?.checked_mul(5)?)?;
        let b = self.a.checked_mul(o.b)?.checked_add(self.b.checked_mul(o.a)?)?;
        Q5::new(a, b, self.d.checked_mul(o.d)?)
    }

    /// The inverse, (a + b√5)⁻¹ = (a − b√5) / (a² − 5b²); `None` for zero or on overflow.
    pub fn inv(self) -> Option<Q5> {
        if self.is_zero() {
            return None;
        }
        let norm = self.a.checked_mul(self.a)?.checked_sub(self.b.checked_mul(self.b)?.checked_mul(5)?)?;
        Q5::new(self.d.checked_mul(self.a)?, self.d.checked_mul(self.b)?.checked_neg()?, norm)
    }

    pub fn div(self, o: Q5) -> Option<Q5> {
        self.mul(o.inv()?)
    }

    /// The exact sign, -1, 0 or 1; `None` on overflow.
    pub fn sign(self) -> Option<i8> {
        let (sa, sb) = (self.a.signum(), self.b.signum());
        if sa >= 0 && sb >= 0 {
            return Some(if sa == 0 && sb == 0 { 0 } else { 1 });
        }
        if sa <= 0 && sb <= 0 {
            return Some(-1);
        }
        // Opposite signs: a + b√5 has the sign of a exactly when a² > 5b². √5 is irrational, so they never tie.
        let (a2, b2) = (self.a.checked_mul(self.a)?, self.b.checked_mul(self.b)?.checked_mul(5)?);
        let larger = if a2 > b2 { sa } else { sb };
        Some(larger as i8)
    }

    /// A binary64 value for display; it decides nothing (RPI.29).
    pub fn approx(self) -> f64 {
        (self.a as f64 + self.b as f64 * 5f64.sqrt()) / self.d as f64
    }
}

impl fmt::Display for Q5 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}{:+}r5)/{}", self.a, self.b, self.d)
    }
}

/// An exact point or direction of R⁴.
pub type V4 = [Q5; 4];

/// An exact 4 × 4 matrix, by rows.
pub type M4 = [[Q5; 4]; 4];

fn map4(f: impl Fn(usize) -> Option<Q5>) -> Option<V4> {
    Some([f(0)?, f(1)?, f(2)?, f(3)?])
}

pub fn dot(u: &V4, v: &V4) -> Option<Q5> {
    let mut sum = Q5::ZERO;
    for i in 0..4 {
        sum = sum.add(u[i].mul(v[i])?)?;
    }
    Some(sum)
}

pub fn add(u: &V4, v: &V4) -> Option<V4> {
    map4(|i| u[i].add(v[i]))
}

pub fn sub(u: &V4, v: &V4) -> Option<V4> {
    map4(|i| u[i].sub(v[i]))
}

pub fn scale(s: Q5, v: &V4) -> Option<V4> {
    map4(|i| s.mul(v[i]))
}

pub fn is_zero(v: &V4) -> bool {
    v.iter().all(|c| c.is_zero())
}

/// The image m v.
pub fn apply(m: &M4, v: &V4) -> Option<V4> {
    map4(|i| dot(&m[i], v))
}

/// The product m n.
pub fn compose(m: &M4, n: &M4) -> Option<M4> {
    let column = |j: usize| -> V4 { [n[0][j], n[1][j], n[2][j], n[3][j]] };
    Some([
        map4(|j| dot(&m[0], &column(j)))?,
        map4(|j| dot(&m[1], &column(j)))?,
        map4(|j| dot(&m[2], &column(j)))?,
        map4(|j| dot(&m[3], &column(j)))?,
    ])
}

pub fn identity() -> M4 {
    let mut m = [[Q5::ZERO; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = Q5::ONE;
    }
    m
}

fn det3(m: [[Q5; 3]; 3]) -> Option<Q5> {
    let t0 = m[0][0].mul(m[1][1].mul(m[2][2])?.sub(m[1][2].mul(m[2][1])?)?)?;
    let t1 = m[0][1].mul(m[1][0].mul(m[2][2])?.sub(m[1][2].mul(m[2][0])?)?)?;
    let t2 = m[0][2].mul(m[1][0].mul(m[2][1])?.sub(m[1][1].mul(m[2][0])?)?)?;
    t0.sub(t1)?.add(t2)
}

/// The determinant of the matrix whose rows are `m`.
pub fn det(m: &M4) -> Option<Q5> {
    let mut total = Q5::ZERO;
    for j in 0..4 {
        let mut minor = [[Q5::ZERO; 3]; 3];
        for r in 1..4 {
            let mut c = 0;
            for (k, value) in m[r].iter().enumerate() {
                if k != j {
                    minor[r - 1][c] = *value;
                    c += 1;
                }
            }
        }
        let term = m[0][j].mul(det3(minor)?)?;
        total = if j % 2 == 0 { total.add(term)? } else { total.sub(term)? };
    }
    Some(total)
}

/// Whether u = t v for some t > 0, for non-zero u and v.
pub fn same_ray(u: &V4, v: &V4) -> Option<bool> {
    if is_zero(u) || is_zero(v) {
        return Some(false);
    }
    for i in 0..4 {
        for j in i + 1..4 {
            if !u[i].mul(v[j])?.sub(u[j].mul(v[i])?)?.is_zero() {
                return Some(false);
            }
        }
    }
    Some(dot(u, v)?.sign()? > 0)
}

const fn negated(x: Q5) -> Q5 {
    Q5 { a: -x.a, b: -x.b, d: x.d }
}

fn even_permutations() -> Vec<[usize; 4]> {
    let mut out = Vec::new();
    for code in 0..256usize {
        let p = [code & 3, code >> 2 & 3, code >> 4 & 3, code >> 6 & 3];
        let distinct = (0..4).all(|i| (0..i).all(|j| p[i] != p[j]));
        let mut inversions = 0;
        for i in 0..4 {
            for j in i + 1..4 {
                if p[i] > p[j] {
                    inversions += 1;
                }
            }
        }
        if distinct && inversions % 2 == 0 {
            out.push(p);
        }
    }
    out
}

/// The 120 vertices of the 600-cell of circumradius 1, as the unit icosians of Conway and Smith: the 8 points
/// (±1, 0, 0, 0) permuted, the 16 points (±1/2, ±1/2, ±1/2, ±1/2) and the 96 even permutations of
/// (0, ±1/2, ±1/(2φ), ±φ/2). They form a group under quaternion multiplication, so [`left`] of each is in K⁺.
pub fn cell600() -> Vec<V4> {
    let half = Q5 { a: 1, b: 0, d: 2 };
    let small = Q5 { a: -1, b: 1, d: 4 };
    let large = Q5 { a: 1, b: 1, d: 4 };
    let mut out = Vec::with_capacity(120);
    for i in 0..4 {
        for unit in [Q5::ONE, negated(Q5::ONE)] {
            let mut v = [Q5::ZERO; 4];
            v[i] = unit;
            out.push(v);
        }
    }
    for signs in 0..16 {
        out.push(std::array::from_fn(|i| if signs >> i & 1 == 0 { half } else { negated(half) }));
    }
    let base = [Q5::ZERO, half, small, large];
    for permutation in even_permutations() {
        for signs in 0..8 {
            let mut v = [Q5::ZERO; 4];
            for (k, &position) in permutation.iter().enumerate() {
                let negative = k > 0 && signs >> (k - 1) & 1 == 1;
                v[position] = if negative { negated(base[k]) } else { base[k] };
            }
            out.push(v);
        }
    }
    out
}

/// The matrix of x ↦ q x for the quaternion q = (a, b, c, d) = a + bi + cj + dk.
pub fn left(q: &V4) -> Option<M4> {
    let [a, b, c, d] = *q;
    Some([[a, b.neg()?, c.neg()?, d.neg()?], [b, a, d.neg()?, c], [c, d, a, b.neg()?], [d, c.neg()?, b, a]])
}

/// Whether `m` is an element of K⁺, the rotation group of the 600-cell: determinant 1 and the 120 vertices mapped onto
/// themselves. `None` on overflow.
pub fn in_k_plus(m: &M4) -> Option<bool> {
    if det(m)? != Q5::ONE {
        return Some(false);
    }
    let vertices = cell600();
    let set: HashSet<V4> = vertices.iter().copied().collect();
    for v in &vertices {
        if !set.contains(&apply(m, v)?) {
            return Some(false);
        }
    }
    Some(true)
}

/// The canonical text of an exact vector.
pub fn text(v: &V4) -> String {
    v.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
}
