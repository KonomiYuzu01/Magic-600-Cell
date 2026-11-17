//! Exact sign tests in Q(√5) with fixed-width integers, and the finite fields F2 and F5 (KN3.1, KN3.2; LP2).
//!
//! Every decision is exact (KN3.1). This module holds the fixed-width half of KN3.2's Q(√5): the exact sign of
//! a + b√5 for any 128-bit a and b, without allocation, and checked evaluation of sums of products of integers, √5
//! and 5 that says nothing when a value would overflow. Unbounded integers live in `c600-jumble` (`q5`); a caller that
//! meets an overflow there decides with them.
//!
//! It also holds KN3.2's F2 and F5 ([`F2`], [`F5`]), the fields of ν and the 36 corrections (A5): ν's 43 bits live in
//! F2 and its two values modulo 5 in F5. Their operations are addition, subtraction, negation, multiplication, powers,
//! the inverse of a nonzero element and division by one, and equality. An element is one residue in 0..p, so every
//! operation has a constant cost whatever its operands (KN3.2's cost by operand size). Rows over these fields, and
//! elimination over them, are library code (KN3.3, LB1); they are not here.
//!
//! # Draft (LP2)
//!
//! The A2 filter (state contract A2, KN3.5) is not in this crate. INV-4 lets a float stand in it, but CT-KN2-05's two
//! scans (`tests/test_kernel_oracle.py`, `tests/test_jumbling_rust_candidate.py`) keep every float type and literal
//! out of `c600-kernel` and `c600-jumble`, so the filter needs a crate of its own; the sum of products here is the
//! exact path it falls back to. `rust/README.md` records the open placement.

/// The exact sign of a + b√5, -1, 0 or 1, for every pair of 128-bit integers (KN3.1).
pub fn sign5(a: i128, b: i128) -> i8 {
    let (sa, sb) = (a.signum() as i8, b.signum() as i8);
    if sb == 0 {
        return sa;
    }
    if sa == 0 || sa == sb {
        return if sa == 0 { sb } else { sa };
    }
    // Opposite signs: a + b√5 has the sign of a exactly when a² > 5b². √5 is irrational, so they never tie.
    let aa = mul_wide(a.unsigned_abs(), a.unsigned_abs());
    let (bh, bl) = mul_wide(b.unsigned_abs(), b.unsigned_abs());
    let (carry, low) = mul_wide(bl, 5);
    // a² ≤ 2^254 < 2^256, so a 5b² past 256 bits is larger.
    let a_larger = match bh.checked_mul(5).and_then(|h| h.checked_add(carry)) {
        Some(high) => aa > (high, low),
        None => false,
    };
    if a_larger {
        sa
    } else {
        sb
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

/// A factor of a term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Factor {
    /// An exact integer.
    Int(i128),
    /// √5.
    Root5,
    /// The constant 5.
    Five,
}

/// A sum of signed products of integers and √5: the form of h = n·Gv/|n|² − α after clearing positive denominators.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SumOfProducts {
    terms: Vec<(bool, Vec<Factor>)>,
}

impl SumOfProducts {
    pub fn new() -> SumOfProducts {
        SumOfProducts::default()
    }

    /// Adds `+ Π factors` (or `− Π factors` when `negative`).
    pub fn push(&mut self, negative: bool, factors: &[Factor]) {
        self.terms.push((negative, factors.to_vec()));
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// The value as x + y√5 with checked 128-bit arithmetic; `None` when a value overflows.
    pub fn exact_parts(&self) -> Option<(i128, i128)> {
        let (mut x, mut y) = (0i128, 0i128);
        for (negative, factors) in &self.terms {
            let mut product = 1i128;
            let mut roots = 0u32;
            for factor in factors {
                match factor {
                    Factor::Int(v) => product = product.checked_mul(*v)?,
                    Factor::Five => product = product.checked_mul(5)?,
                    Factor::Root5 => roots += 1,
                }
            }
            product = product.checked_mul(5i128.checked_pow(roots / 2)?)?;
            if *negative {
                product = product.checked_neg()?;
            }
            if roots % 2 == 0 {
                x = x.checked_add(product)?;
            } else {
                y = y.checked_add(product)?;
            }
        }
        Some((x, y))
    }

    /// The exact sign with checked 128-bit arithmetic; `None` when a value overflows (decide with big integers).
    pub fn exact_sign(&self) -> Option<i8> {
        self.exact_parts().map(|(x, y)| sign5(x, y))
    }
}

/// A prime field F_p for a small prime p: an element is its residue in 0..p, and every operation has a constant cost.
macro_rules! prime_field {
    ($(#[$doc:meta])* $name:ident, $p:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u8);

        impl $name {
            /// The characteristic p.
            pub const MODULUS: u8 = $p;
            pub const ZERO: $name = $name(0);
            pub const ONE: $name = $name(1);

            /// The residue of an integer, negative ones included.
            pub const fn new(n: i64) -> $name {
                $name(n.rem_euclid($p) as u8)
            }

            /// The residue in 0..p.
            pub const fn value(self) -> u8 {
                self.0
            }

            pub const fn is_zero(self) -> bool {
                self.0 == 0
            }

            /// self^e by squaring; self^0 is one, 0^0 included.
            pub const fn pow(self, mut e: u64) -> $name {
                let (mut base, mut out) = (self.0, 1u8);
                while e > 0 {
                    if e & 1 == 1 {
                        out = out * base % $p;
                    }
                    base = base * base % $p;
                    e >>= 1;
                }
                $name(out)
            }

            /// The inverse, a^(p − 2) by Fermat; `None` for zero.
            pub const fn inv(self) -> Option<$name> {
                if self.0 == 0 {
                    None
                } else {
                    Some(self.pow($p - 2))
                }
            }

            /// self / other; `None` when other is zero.
            pub const fn checked_div(self, other: $name) -> Option<$name> {
                match other.inv() {
                    Some(inverse) => Some($name(self.0 * inverse.0 % $p)),
                    None => None,
                }
            }
        }

        impl std::ops::Add for $name {
            type Output = $name;
            fn add(self, other: $name) -> $name {
                $name((self.0 + other.0) % $p)
            }
        }

        impl std::ops::Sub for $name {
            type Output = $name;
            fn sub(self, other: $name) -> $name {
                $name((self.0 + $p - other.0) % $p)
            }
        }

        impl std::ops::Neg for $name {
            type Output = $name;
            fn neg(self) -> $name {
                $name(($p - self.0) % $p)
            }
        }

        impl std::ops::Mul for $name {
            type Output = $name;
            fn mul(self, other: $name) -> $name {
                $name(self.0 * other.0 % $p)
            }
        }

        impl std::ops::AddAssign for $name {
            fn add_assign(&mut self, other: $name) {
                *self = *self + other;
            }
        }

        impl std::ops::SubAssign for $name {
            fn sub_assign(&mut self, other: $name) {
                *self = *self - other;
            }
        }

        impl std::ops::MulAssign for $name {
            fn mul_assign(&mut self, other: $name) {
                *self = *self * other;
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

prime_field!(
    /// F2, the field of ν's 35 sign bits and 8 frame bits (KN3.2, A5).
    F2,
    2
);

prime_field!(
    /// F5, the field of ν's two values modulo 5, C5_17 and C5_28 (KN3.2, A5).
    F5,
    5
);

#[cfg(test)]
mod tests {
    use super::*;

    /// Lucas and Fibonacci numbers: L_n − F_n√5 = 2ψ^n with ψ = −1/φ, so its sign is (−1)^n and
    /// |L_n² − 5F_n²| = 4, the smallest margin a pair of this size can have.
    fn lucas_fibonacci() -> Vec<(u32, i128, i128)> {
        let (mut f0, mut f1) = (0i128, 1i128);
        let (mut l0, mut l1) = (2i128, 1i128);
        let mut out = vec![(0, l0, f0)];
        let mut n = 1;
        loop {
            out.push((n, l1, f1));
            let (Some(f2), Some(l2)) = (f0.checked_add(f1), l0.checked_add(l1)) else {
                break;
            };
            (f0, f1, l0, l1) = (f1, f2, l1, l2);
            n += 1;
        }
        out
    }

    /// CT-KN3-01 (part: the fixed-width sign test at the smallest margins, |a² − 5b²| = 4 along Lucas and Fibonacci
    /// numbers up to the 128-bit limit, at both signs and at the limits of the range, and sums of products with exact
    /// zeros of contact type and a difference of one unit at 2^100; not run: the margins of the model's regions and
    /// the big-integer path, which `c600-jumble`'s `q5` decides).
    #[test]
    fn signs_at_the_smallest_margins_are_exact() {
        let pairs = lucas_fibonacci();
        assert!(pairs.len() > 170, "the pairs reach the 128-bit limit");
        for &(n, l, f) in &pairs {
            let expected = if n % 2 == 0 { 1 } else { -1 };
            assert_eq!(sign5(l, -f), expected, "L_{n} - F_{n} sqrt 5");
            assert_eq!(sign5(-l, f), -expected, "-L_{n} + F_{n} sqrt 5");
            assert_eq!(sign5(l, f), 1);
            assert_eq!(sign5(-l, -f), if l == 0 && f == 0 { 0 } else { -1 });
            let mut sum = SumOfProducts::new();
            sum.push(false, &[Factor::Int(l)]);
            sum.push(true, &[Factor::Int(f), Factor::Root5]);
            assert_eq!(sum.exact_sign(), Some(expected), "n = {n}");
        }
        assert_eq!(sign5(0, 0), 0);
        assert_eq!(sign5(i128::MAX, i128::MIN), -1);
        assert_eq!(sign5(i128::MIN, i128::MAX), 1);
        assert_eq!(sign5(i128::MIN, 0), -1);
        assert_eq!(sign5(0, i128::MIN), -1);
        // 2^126 against √5·2^125 ≈ 1.118·2^126: the √5 side is larger.
        assert_eq!(sign5(1 << 126, -(1 << 125)), -1);
        assert_eq!(sign5(3 << 125, -(1 << 125)), 1);

        // Contact: h = 0 exactly, with √5·√5 against 5.
        let mut zero = SumOfProducts::new();
        zero.push(false, &[Factor::Int(1 << 100), Factor::Root5, Factor::Root5]);
        zero.push(true, &[Factor::Five, Factor::Int(1 << 100)]);
        assert_eq!(zero.exact_parts(), Some((0, 0)));
        assert_eq!(zero.exact_sign(), Some(0));
        assert_eq!(SumOfProducts::new().exact_sign(), Some(0));
        // One unit at 2^100.
        let mut close = SumOfProducts::new();
        close.push(false, &[Factor::Int((1 << 100) + 1)]);
        close.push(true, &[Factor::Int(1 << 100)]);
        assert_eq!(close.exact_sign(), Some(1));
    }

    /// KN3.2's F2 and F5 against integer arithmetic reduced mod p: every operation on every pair (and every triple for
    /// the field laws), inverses, division, powers past p, and residues of negative and extreme integers.
    macro_rules! field_laws {
        ($field:ty, $p:literal) => {{
            let all: Vec<$field> = (0..$p).map(<$field>::new).collect();
            assert_eq!(<$field>::MODULUS, $p);
            assert_eq!(<$field>::new($p), <$field>::ZERO);
            assert_eq!(<$field>::new(-1).value(), $p - 1);
            assert_eq!(<$field>::new(i64::MIN).value() as i64, i64::MIN.rem_euclid($p));
            assert_eq!(<$field>::new(i64::MAX).value() as i64, i64::MAX.rem_euclid($p));
            for &a in &all {
                let x = a.value() as i64;
                assert_eq!(-a, <$field>::new(-x));
                assert_eq!(a + -a, <$field>::ZERO);
                assert_eq!(a.pow(0), <$field>::ONE);
                for e in 1..(3 * $p) {
                    assert_eq!(a.pow(e as u64), <$field>::new(x.pow(e as u32) % $p), "{x}^{e}");
                }
                match a.inv() {
                    None => assert!(a.is_zero()),
                    Some(inverse) => assert_eq!(a * inverse, <$field>::ONE),
                }
                for &b in &all {
                    let y = b.value() as i64;
                    assert_eq!(a + b, <$field>::new(x + y));
                    assert_eq!(a - b, <$field>::new(x - y));
                    assert_eq!(a * b, <$field>::new(x * y));
                    match a.checked_div(b) {
                        None => assert!(b.is_zero()),
                        Some(q) => assert_eq!(q * b, a),
                    }
                    let mut c = a;
                    c += b;
                    c -= b;
                    c *= <$field>::ONE;
                    assert_eq!(c, a);
                    for &z in &all {
                        assert_eq!((a + b) * z, a * z + b * z);
                        assert_eq!((a * b) * z, a * (b * z));
                    }
                }
            }
        }};
    }

    #[test]
    fn f2_and_f5_are_the_fields_of_nu() {
        field_laws!(F2, 2);
        field_laws!(F5, 5);
        assert_eq!(F5::new(2).inv(), Some(F5::new(3)));
        assert_eq!(F5::new(4).inv(), Some(F5::new(4)));
        assert_eq!(F2::new(3).to_string(), "1");
        assert_eq!(F5::default(), F5::ZERO);
    }

    #[test]
    fn checked_evaluation_reports_overflow_instead_of_wrapping() {
        let mut sum = SumOfProducts::new();
        sum.push(false, &[Factor::Int(i128::MAX), Factor::Int(2)]);
        assert_eq!(sum.exact_sign(), None);
        let mut cancel = SumOfProducts::new();
        cancel.push(false, &[Factor::Int(i128::MAX), Factor::Int(2)]);
        cancel.push(true, &[Factor::Int(i128::MAX), Factor::Int(2)]);
        assert_eq!(cancel.exact_sign(), None, "an overflowing term decides nothing, even when the sum cancels");
        let mut roots = SumOfProducts::new();
        roots.push(false, &[Factor::Root5; 200]);
        assert_eq!(roots.exact_sign(), None, "5^100 does not fit 128 bits");
    }
}
