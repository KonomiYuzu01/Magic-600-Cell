//! Approximate scalars `Approx<p>` (LG1.7, LG2.19): binary floating point with p = 24, 53 or 113 significant bits and
//! round to nearest, ties to even. Every operation runs on exact integers; no host float is involved (PHS.19, "Exact
//! decision paths"). The exponent is unbounded, so no overflow, underflow, infinity, NaN or negative zero arises.
//! Publication checks that the value is a binary64 number (FP.6); see [`Approx::f64_bits`].

use std::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::value::sign_q5;

/// The precisions the IR draft allows.
pub const PRECISIONS: [u32; 3] = [24, 53, 113];

/// DET.57's declared width of `Approx<p>` in bytes.
pub fn width(precision: u32) -> u128 {
    match precision {
        24 => 4,
        53 => 8,
        _ => 16,
    }
}

/// `(-1)^negative · mantissa · 2^exponent`. Zero has mantissa 0, `negative` false and exponent 0; otherwise
/// 2^(p-1) ≤ mantissa < 2^p.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Approx {
    pub precision: u32,
    pub negative: bool,
    pub mantissa: u128,
    pub exponent: i128,
}

/// The largest decimal exponent plus digit count an approximate literal may have.
const MAX_DECIMAL: usize = 10_000;

fn bits(n: &BigInt) -> i128 {
    n.bits() as i128
}

/// Rounds `m · 2^exponent` (m ≥ 0) to p bits.
fn round_integer(precision: u32, negative: bool, m: BigInt, exponent: i128) -> Approx {
    if m.is_zero() {
        return Approx::zero(precision);
    }
    let p = precision as i128;
    let (mut q, mut e) = if bits(&m) <= p {
        let shift = p - bits(&m);
        (m << shift as usize, exponent - shift)
    } else {
        let shift = (bits(&m) - p) as usize;
        let mut q: BigInt = &m >> shift;
        let rem = &m - (&q << shift);
        match rem.cmp(&(BigInt::one() << (shift - 1))) {
            Ordering::Greater => q += 1u32,
            Ordering::Equal if q.is_odd() => q += 1u32,
            _ => {}
        }
        (q, exponent + shift as i128)
    };
    if bits(&q) > p {
        q >>= 1usize;
        e += 1;
    }
    Approx { precision, negative, mantissa: q.to_u128().expect("at most 113 bits"), exponent: e }
}

impl Approx {
    pub fn zero(precision: u32) -> Approx {
        Approx { precision, negative: false, mantissa: 0, exponent: 0 }
    }

    pub fn is_zero(&self) -> bool {
        self.mantissa == 0
    }

    /// Rounds (a + b√5)/d with d > 0 to p bits.
    ///
    /// With x = |a + b√5|/d and f(t) = floor(x · 2^t), computed as floor(floor(A + B√5)/D) on integers, the exponent is
    /// e = floor(log2 x) = bits(f(t0)) - 1 - t0 for any t0 with f(t0) ≥ 1; t0 = bits(d) + bits(|a| + 3|b|) + 1
    /// suffices because x > 1/(d · (|a| + 3|b|)). Then q = f(p - 1 - e) holds the p leading bits, and f one bit further
    /// decides the rounding. A tie needs x · 2^(t+1) to be an integer, which an irrational x never is.
    pub fn from_q5(precision: u32, a: &BigInt, b: &BigInt, d: &BigInt) -> Approx {
        let sign = sign_q5(a, b);
        if sign == Ordering::Equal {
            return Approx::zero(precision);
        }
        let negative = sign == Ordering::Less;
        let (a, b) = if negative { (-a, -b) } else { (a.clone(), b.clone()) };
        if b.is_zero() && d.is_one() {
            return round_integer(precision, negative, a, 0);
        }
        let floor = |t: i128| -> BigInt {
            let (aa, bb, dd) = if t >= 0 {
                (&a << t as usize, &b << t as usize, d.clone())
            } else {
                (a.clone(), b.clone(), d << (-t) as usize)
            };
            let root = (&bb * &bb * 5u32).sqrt();
            let whole = match bb.sign() {
                Sign::Plus => aa + root,
                Sign::Minus => aa - root - 1u32,
                Sign::NoSign => aa,
            };
            whole.div_floor(&dd)
        };
        let t0 = bits(d) + bits(&(a.abs() + b.abs() * 3u32)) + 1;
        let e = bits(&floor(t0)) - 1 - t0;
        let t = precision as i128 - 1 - e;
        let mut m = floor(t);
        if floor(t + 1).is_odd() {
            let s = t + 1;
            let tie = b.is_zero()
                && if s >= 0 { (&a << s as usize).is_multiple_of(d) } else { a.is_multiple_of(&(d << (-s) as usize)) };
            if !tie || m.is_odd() {
                m += 1u32;
            }
        }
        let mut exponent = -t;
        if bits(&m) > precision as i128 {
            m >>= 1usize;
            exponent += 1;
        }
        Approx { precision, negative, mantissa: m.to_u128().expect("at most 113 bits"), exponent }
    }

    /// Parses a finite decimal `[+-]digits[.digits][(e|E)[+-]digits]` and rounds it to p bits. `None` for another
    /// form, or when the digits and the decimal exponent together exceed 10,000 (a draft limit that keeps loading
    /// bounded).
    pub fn parse(precision: u32, decimal: &str) -> Option<Approx> {
        let (negative, rest) = match decimal.as_bytes().first()? {
            b'-' => (true, &decimal[1..]),
            b'+' => (false, &decimal[1..]),
            _ => (false, decimal),
        };
        let (mantissa, exponent) = match rest.find(['e', 'E']) {
            Some(i) => (&rest[..i], Some(&rest[i + 1..])),
            None => (rest, None),
        };
        let (whole, fraction) = match mantissa.find('.') {
            Some(i) => (&mantissa[..i], Some(&mantissa[i + 1..])),
            None => (mantissa, None),
        };
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
        if !digits(whole) || fraction.is_some_and(|f| !digits(f)) {
            return None;
        }
        let fraction = fraction.unwrap_or("");
        let exponent: i128 = match exponent {
            None => 0,
            Some(e) => {
                let body = e.strip_prefix(['+', '-']).unwrap_or(e);
                if !digits(body) || body.len() > 6 {
                    return None;
                }
                e.parse().ok()?
            }
        };
        let k = exponent - fraction.len() as i128;
        if whole.len() + fraction.len() + k.unsigned_abs() as usize > MAX_DECIMAL {
            return None;
        }
        let mut m: BigInt = format!("{whole}{fraction}").parse().ok()?;
        if negative {
            m = -m;
        }
        let ten = BigInt::from(10u32);
        Some(if k >= 0 {
            Approx::from_q5(precision, &(m * ten.pow(k as u32)), &BigInt::zero(), &BigInt::one())
        } else {
            Approx::from_q5(precision, &m, &BigInt::zero(), &ten.pow((-k) as u32))
        })
    }

    /// The rounded sum of two values of one precision. When the exponents differ by p + 2 or more, the smaller operand
    /// is below a quarter of the larger one's unit in the last place, so the sum rounds to the larger operand.
    pub fn add(self, other: Approx) -> Approx {
        let p = self.precision;
        if self.is_zero() {
            return other;
        }
        if other.is_zero() {
            return self;
        }
        let (hi, lo) = if self.exponent >= other.exponent { (self, other) } else { (other, self) };
        let gap = hi.exponent - lo.exponent;
        if gap >= p as i128 + 2 {
            return hi;
        }
        let signed = |x: Approx, shift: usize| {
            let m = BigInt::from(x.mantissa) << shift;
            if x.negative {
                -m
            } else {
                m
            }
        };
        let sum = signed(hi, gap as usize) + signed(lo, 0);
        if sum.is_zero() {
            return Approx::zero(p);
        }
        round_integer(p, sum.is_negative(), sum.abs(), lo.exponent)
    }

    /// The exact order of two values.
    pub fn compare(&self, other: &Approx) -> Ordering {
        let sign = |x: &Approx| {
            if x.is_zero() {
                0
            } else if x.negative {
                -1
            } else {
                1
            }
        };
        let (sa, sb) = (sign(self), sign(other));
        if sa != sb || sa == 0 {
            return sa.cmp(&sb);
        }
        let top = |x: &Approx| x.exponent + (128 - x.mantissa.leading_zeros()) as i128;
        let magnitude = top(self).cmp(&top(other)).then_with(|| {
            let e = self.exponent.min(other.exponent);
            let m = |x: &Approx| BigInt::from(x.mantissa) << (x.exponent - e) as usize;
            m(self).cmp(&m(other))
        });
        if sa < 0 {
            magnitude.reverse()
        } else {
            magnitude
        }
    }

    /// The binary64 bits of the value, or `None` when it is not exactly a binary64 number (FP.6). Subnormal binary64
    /// numbers are included; a value of precision 113 is publishable only when its trailing bits make it one.
    pub fn f64_bits(&self) -> Option<u64> {
        if self.is_zero() {
            return Some(0);
        }
        let tz = self.mantissa.trailing_zeros();
        let m = self.mantissa >> tz;
        let e = self.exponent + tz as i128;
        let n = 128 - m.leading_zeros() as i128;
        let top = e + n - 1;
        if n > 53 || top > 1023 || e < -1074 {
            return None;
        }
        let sign = (self.negative as u64) << 63;
        Some(if top >= -1022 {
            let fraction = (m << (53 - n) as u32) as u64 & ((1u64 << 52) - 1);
            sign | ((top + 1023) as u64) << 52 | fraction
        } else {
            sign | (m << (e + 1074) as u32) as u64
        })
    }
}
