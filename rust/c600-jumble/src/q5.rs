//! Exact numbers of Q(√5) (selection 1 of the LP2 plan, section 10).
//!
//! A number is the reduced integer triple (a + b√5)/d with d > 0, on `num-bigint`. Vectors and matrices are arrays
//! of numbers; [`IntVec`] is the integer form (A, B, D) with entries (A_i + B_i√5)/D that the sign kernel works on.
//! Nothing here, or anywhere in this crate, has a float type.

use std::cmp::Ordering;
use std::fmt;

use c600_kernel::json::Value;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// An exact element of Q(√5).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Q5 {
    a: BigInt,
    b: BigInt,
    d: BigInt,
}

impl fmt::Debug for Q5 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Q5({},{},{})", self.a, self.b, self.d)
    }
}

/// The sign of x + y√5 for integers x and y.
pub fn sign5(x: &BigInt, y: &BigInt) -> i32 {
    let (sx, sy) = (sign_of(x), sign_of(y));
    if sy == 0 {
        return sx;
    }
    if sx == 0 || sx == sy {
        return if sx == 0 { sy } else { sx };
    }
    let t = x * x - BigInt::from(5) * y * y;
    let s = sign_of(&t);
    if sx > 0 {
        s
    } else {
        -s
    }
}

fn sign_of(x: &BigInt) -> i32 {
    if x.is_positive() {
        1
    } else if x.is_negative() {
        -1
    } else {
        0
    }
}

const MASK: u128 = (1 << 64) - 1;

/// The 256-bit product of two 128-bit integers, as (high, low).
fn mul_wide(a: u128, b: u128) -> (u128, u128) {
    let (a1, a0) = (a >> 64, a & MASK);
    let (b1, b0) = (b >> 64, b & MASK);
    let (p00, p01, p10, p11) = (a0 * b0, a0 * b1, a1 * b0, a1 * b1);
    let mid = (p00 >> 64) + (p01 & MASK) + (p10 & MASK);
    let lo = (p00 & MASK) | (mid << 64);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (hi, lo)
}

/// The largest magnitude [`sign5_small`] accepts: 2^126, so that 5y² stays below 2^256.
pub const SMALL_LIMIT: u128 = 1 << 126;

/// The sign of x + y√5 for |x|, |y| < 2^126, without allocating. Callers check the bound.
pub fn sign5_small(x: i128, y: i128) -> i32 {
    let (sx, sy) = (x.signum() as i32, y.signum() as i32);
    if sy == 0 {
        return sx;
    }
    if sx == 0 {
        return sy;
    }
    if sx == sy {
        return sx;
    }
    debug_assert!(x.unsigned_abs() < SMALL_LIMIT && y.unsigned_abs() < SMALL_LIMIT);
    let xx = mul_wide(x.unsigned_abs(), x.unsigned_abs());
    let (yh, yl) = mul_wide(y.unsigned_abs(), y.unsigned_abs());
    let (carry, lo5) = mul_wide(yl, 5);
    let yy5 = (yh * 5 + carry, lo5);
    let s = match xx.cmp(&yy5) {
        Ordering::Greater => 1,
        Ordering::Less => -1,
        Ordering::Equal => 0,
    };
    if sx > 0 {
        s
    } else {
        -s
    }
}

impl Q5 {
    /// (a + b√5)/d in lowest terms; `d` must not be zero.
    pub fn new(a: BigInt, b: BigInt, d: BigInt) -> Q5 {
        assert!(!d.is_zero(), "Q5 denominator is zero");
        let (mut a, mut b, mut d) = if d.is_negative() { (-a, -b, -d) } else { (a, b, d) };
        let g = a.gcd(&b).gcd(&d);
        if !g.is_one() {
            a /= &g;
            b /= &g;
            d /= &g;
        }
        Q5 { a, b, d }
    }

    pub fn int(n: i64) -> Q5 {
        Q5 { a: BigInt::from(n), b: BigInt::zero(), d: BigInt::one() }
    }

    pub fn of(a: i64, b: i64, d: i64) -> Q5 {
        Q5::new(BigInt::from(a), BigInt::from(b), BigInt::from(d))
    }

    pub fn zero() -> Q5 {
        Q5::int(0)
    }

    pub fn one() -> Q5 {
        Q5::int(1)
    }

    pub fn a(&self) -> &BigInt {
        &self.a
    }

    pub fn b(&self) -> &BigInt {
        &self.b
    }

    pub fn d(&self) -> &BigInt {
        &self.d
    }

    pub fn is_zero(&self) -> bool {
        self.a.is_zero() && self.b.is_zero()
    }

    pub fn add(&self, o: &Q5) -> Q5 {
        if self.d == o.d {
            return Q5::new(&self.a + &o.a, &self.b + &o.b, self.d.clone());
        }
        Q5::new(&self.a * &o.d + &o.a * &self.d, &self.b * &o.d + &o.b * &self.d, &self.d * &o.d)
    }

    pub fn neg(&self) -> Q5 {
        Q5 { a: -&self.a, b: -&self.b, d: self.d.clone() }
    }

    pub fn sub(&self, o: &Q5) -> Q5 {
        self.add(&o.neg())
    }

    pub fn mul(&self, o: &Q5) -> Q5 {
        Q5::new(&self.a * &o.a + BigInt::from(5) * &self.b * &o.b, &self.a * &o.b + &self.b * &o.a, &self.d * &o.d)
    }

    /// The inverse; `None` for zero.
    pub fn inv(&self) -> Option<Q5> {
        let n = &self.a * &self.a - BigInt::from(5) * &self.b * &self.b;
        if n.is_zero() {
            return None;
        }
        Some(Q5::new(&self.a * &self.d, -(&self.b * &self.d), n))
    }

    pub fn div(&self, o: &Q5) -> Option<Q5> {
        o.inv().map(|inv| self.mul(&inv))
    }

    pub fn sign(&self) -> i32 {
        sign5(&self.a, &self.b)
    }

    /// The JSON triple `[a, b, d]`.
    pub fn to_json(&self) -> Value {
        Value::Array(vec![big_json(&self.a), big_json(&self.b), big_json(&self.d)])
    }

    /// Python's `json.dumps` text of the triple with its default separators: `[a, b, d]`.
    pub fn write_spaced(&self, out: &mut String) {
        out.push('[');
        out.push_str(&self.a.to_string());
        out.push_str(", ");
        out.push_str(&self.b.to_string());
        out.push_str(", ");
        out.push_str(&self.d.to_string());
        out.push(']');
    }

    /// The compact JSON text of the triple: `[a,b,d]`.
    pub fn write_compact(&self, out: &mut String) {
        out.push('[');
        out.push_str(&self.a.to_string());
        out.push(',');
        out.push_str(&self.b.to_string());
        out.push(',');
        out.push_str(&self.d.to_string());
        out.push(']');
    }

    /// A stored triple: integers (never booleans or floats), d > 0 and lowest terms, as J1's `q5_from_json`.
    pub fn from_json_reduced(value: &Value) -> Result<Q5, String> {
        let (a, b, d) = triple(value)?;
        let x = Q5::new(a.clone(), b.clone(), d.clone());
        if x.a != a || x.b != b || x.d != d {
            return Err("a Q(sqrt5) triple is not in lowest terms".into());
        }
        Ok(x)
    }

    /// A triple that need not be in lowest terms (certificate points): integers and d > 0.
    pub fn from_json_any(value: &Value) -> Result<Q5, String> {
        let (a, b, d) = triple(value)?;
        Ok(Q5::new(a, b, d))
    }
}

fn triple(value: &Value) -> Result<(BigInt, BigInt, BigInt), String> {
    let items = value.as_array().filter(|items| items.len() == 3).ok_or("a Q(sqrt5) number needs [a, b, d]")?;
    let a = json_int(&items[0])?;
    let b = json_int(&items[1])?;
    let d = json_int(&items[2])?;
    if !d.is_positive() {
        return Err("a Q(sqrt5) denominator must be positive".into());
    }
    Ok((a, b, d))
}

/// A JSON integer of any size; booleans, floats and strings are refused.
pub fn json_int(value: &Value) -> Result<BigInt, String> {
    match value {
        Value::Int(n) => Ok(BigInt::from(*n)),
        Value::Number(lexeme) => {
            let digits = lexeme.strip_prefix('-').unwrap_or(lexeme);
            if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
                return Err("a number is not an integer".into());
            }
            lexeme.parse::<BigInt>().map_err(|_| "a number is not an integer".into())
        }
        _ => Err("a value is not an integer".into()),
    }
}

/// A JSON integer for a big integer.
pub fn big_json(x: &BigInt) -> Value {
    match x.to_i64() {
        Some(n) => Value::Int(n),
        None => Value::Number(x.to_string()),
    }
}

impl PartialOrd for Q5 {
    fn partial_cmp(&self, other: &Q5) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The order of the real numbers.
impl Ord for Q5 {
    fn cmp(&self, other: &Q5) -> Ordering {
        match self.sub(other).sign() {
            1 => Ordering::Greater,
            -1 => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
}

pub type Vec4 = [Q5; 4];
pub type Mat4 = [[Q5; 4]; 4];

pub fn zero4() -> Vec4 {
    [Q5::zero(), Q5::zero(), Q5::zero(), Q5::zero()]
}

pub fn dot(u: &[Q5], v: &[Q5]) -> Q5 {
    let mut s = Q5::zero();
    for (x, y) in u.iter().zip(v) {
        s = s.add(&x.mul(y));
    }
    s
}

pub fn matvec(m: &Mat4, v: &Vec4) -> Vec4 {
    [dot(&m[0], v), dot(&m[1], v), dot(&m[2], v), dot(&m[3], v)]
}

pub fn transpose(m: &Mat4) -> Mat4 {
    std::array::from_fn(|i| std::array::from_fn(|j| m[j][i].clone()))
}

pub fn matmul(a: &Mat4, b: &Mat4) -> Mat4 {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let mut s = Q5::zero();
            for k in 0..4 {
                s = s.add(&a[i][k].mul(&b[k][j]));
            }
            s
        })
    })
}

pub fn identity4() -> Mat4 {
    std::array::from_fn(|i| std::array::from_fn(|j| if i == j { Q5::one() } else { Q5::zero() }))
}

pub fn det4(m: &Mat4) -> Q5 {
    let det3 = |a: [[&Q5; 3]; 3]| {
        let t0 = a[1][1].mul(a[2][2]).sub(&a[1][2].mul(a[2][1]));
        let t1 = a[1][0].mul(a[2][2]).sub(&a[1][2].mul(a[2][0]));
        let t2 = a[1][0].mul(a[2][1]).sub(&a[1][1].mul(a[2][0]));
        a[0][0].mul(&t0).sub(&a[0][1].mul(&t1)).add(&a[0][2].mul(&t2))
    };
    let mut s = Q5::zero();
    for j in 0..4 {
        let cols: Vec<usize> = (0..4).filter(|&k| k != j).collect();
        let minor: [[&Q5; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|c| &m[i + 1][cols[c]]));
        let term = m[0][j].mul(&det3(minor));
        s = if j % 2 == 0 { s.add(&term) } else { s.sub(&term) };
    }
    s
}

/// gᵀg = I and det g = 1, exactly.
pub fn is_rotation(m: &Mat4) -> bool {
    matmul(&transpose(m), m) == identity4() && det4(m) == Q5::one()
}

/// A 4 × 4 matrix of stored triples (lowest terms), as J1's `matrix_from_json`.
pub fn matrix_from_json(value: &Value) -> Result<Mat4, String> {
    let rows = value.as_array().filter(|rows| rows.len() == 4).ok_or("a matrix needs four rows")?;
    let mut out = Vec::with_capacity(4);
    for row in rows {
        let entries = row.as_array().filter(|entries| entries.len() == 4).ok_or("a matrix row needs four entries")?;
        let parsed: Result<Vec<Q5>, String> = entries.iter().map(Q5::from_json_reduced).collect();
        let parsed = parsed?;
        out.push([parsed[0].clone(), parsed[1].clone(), parsed[2].clone(), parsed[3].clone()]);
    }
    Ok([out[0].clone(), out[1].clone(), out[2].clone(), out[3].clone()])
}

pub fn matrix_json(m: &Mat4) -> Value {
    Value::Array(m.iter().map(|row| Value::Array(row.iter().map(Q5::to_json).collect())).collect())
}

/// Python's `json.dumps(matrix_json(m))` with its default separators: J1's pose key and digest key.
pub fn matrix_spaced(m: &Mat4) -> String {
    let mut out = String::from("[");
    for (i, row) in m.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('[');
        for (j, x) in row.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            x.write_spaced(&mut out);
        }
        out.push(']');
    }
    out.push(']');
    out
}

/// `json.dumps(matrix_json(m), separators=(',', ':'))`: the menu identity's item key.
pub fn matrix_compact(m: &Mat4) -> String {
    let mut out = String::from("[");
    for (i, row) in m.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        for (j, x) in row.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            x.write_compact(&mut out);
        }
        out.push(']');
    }
    out.push(']');
    out
}

/// The integer form (A, B, D) of a vector: entry i is (A_i + B_i√5)/D with D the least common denominator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntVec {
    pub a: Vec<BigInt>,
    pub b: Vec<BigInt>,
    pub d: BigInt,
}

pub fn int_form(v: &[Q5]) -> IntVec {
    let mut d = BigInt::one();
    for x in v {
        d = d.lcm(&x.d);
    }
    let a = v.iter().map(|x| &x.a * (&d / &x.d)).collect();
    let b = v.iter().map(|x| &x.b * (&d / &x.d)).collect();
    IntVec { a, b, d }
}

/// Exact Gauss-Jordan elimination: solves m x = rhs for a square nonsingular m; `None` when m is singular.
pub fn solve(m: &[Vec<Q5>], rhs: &[Vec<Q5>]) -> Option<Vec<Vec<Q5>>> {
    let n = m.len();
    let mut a: Vec<Vec<Q5>> = (0..n).map(|i| m[i].iter().chain(rhs[i].iter()).cloned().collect()).collect();
    for col in 0..n {
        let piv = (col..n).find(|&r| !a[r][col].is_zero())?;
        a.swap(col, piv);
        let inv = a[col][col].inv()?;
        a[col] = a[col].iter().map(|x| x.mul(&inv)).collect();
        for r in 0..n {
            if r != col && !a[r][col].is_zero() {
                let f = a[r][col].clone();
                let pivot_row = a[col].clone();
                a[r] = a[r].iter().zip(&pivot_row).map(|(x, y)| x.sub(&f.mul(y))).collect();
            }
        }
    }
    Some(a.into_iter().map(|row| row[n..].to_vec()).collect())
}

pub fn rank(rows: &[Vec<Q5>]) -> usize {
    let mut a: Vec<Vec<Q5>> = rows.to_vec();
    if a.is_empty() {
        return 0;
    }
    let cols = a[0].len();
    let mut rk = 0;
    for col in 0..cols {
        let Some(piv) = (rk..a.len()).find(|&r| !a[r][col].is_zero()) else { continue };
        a.swap(rk, piv);
        let inv = a[rk][col].inv().expect("pivot is nonzero");
        a[rk] = a[rk].iter().map(|x| x.mul(&inv)).collect();
        for r in 0..a.len() {
            if r != rk && !a[r][col].is_zero() {
                let f = a[r][col].clone();
                let pivot_row = a[rk].clone();
                a[r] = a[r].iter().zip(&pivot_row).map(|(x, y)| x.sub(&f.mul(y))).collect();
            }
        }
        rk += 1;
    }
    rk
}

/// The dimension of the affine hull of a nonempty list of points.
pub fn affine_rank(points: &[Vec<Q5>]) -> usize {
    let base = &points[0];
    let diffs: Vec<Vec<Q5>> = points[1..].iter().map(|q| q.iter().zip(base).map(|(x, y)| x.sub(y)).collect()).collect();
    rank(&diffs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduction_and_signs() {
        assert_eq!(Q5::of(4, 2, -6), Q5::of(-2, -1, 3));
        assert_eq!(Q5::of(0, 0, 7), Q5::zero());
        assert_eq!(Q5::of(-2, 1, 1).sign(), 1); // √5 - 2 > 0
        assert_eq!(Q5::of(3, -1, 1).sign(), 1);
        assert_eq!(Q5::of(2, -1, 1).sign(), -1);
        let phi = Q5::of(1, 1, 2);
        assert_eq!(phi.mul(&phi), phi.add(&Q5::one()));
        assert_eq!(phi.mul(&phi.inv().unwrap()), Q5::one());
        for (x, y) in [(5i128, -2), (-5, 2), (2, -1), (-9, 4), (9, -4), (0, -3), (7, 0), (0, 0), (-1, -1)] {
            assert_eq!(sign5_small(x, y), sign5(&BigInt::from(x), &BigInt::from(y)), "{x} {y}");
        }
        // x + y√5 with |y| below and above x/√5 ≈ 0.447x, near the size limit of the small path.
        let big = (1i128 << 125) - 1;
        for (x, y, s) in [(big, -(big / 3), 1), (big, -(big / 2), -1), (-big, big / 3, -1), (-big, big / 2, 1)] {
            assert_eq!(sign5_small(x, y), s, "{x} {y}");
            assert_eq!(sign5(&BigInt::from(x), &BigInt::from(y)), s, "{x} {y}");
        }
    }

    #[test]
    fn json_triples() {
        let parse = |t: &str| c600_kernel::json::parse(t.as_bytes()).unwrap();
        assert!(Q5::from_json_reduced(&parse("[2,4,2]")).is_err());
        assert!(Q5::from_json_any(&parse("[2,4,2]")).is_ok());
        assert!(Q5::from_json_reduced(&parse("[1,0,0]")).is_err());
        assert!(Q5::from_json_reduced(&parse("[1.0,0,1]")).is_err());
        assert!(Q5::from_json_reduced(&parse("[true,0,1]")).is_err());
        let huge = format!("[{}1,0,1]", "9".repeat(40));
        let x = Q5::from_json_reduced(&parse(&huge)).unwrap();
        assert_eq!(x.to_json().canonical(), huge);
    }
}
