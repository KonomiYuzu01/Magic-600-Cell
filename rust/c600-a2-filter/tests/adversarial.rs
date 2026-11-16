//! The A2 filter against the exact sign on adversarial sums of products.
//!
//! Every case is decided by the filter and checked against `c600_kernel::exact`; where that 128-bit path overflows, a
//! wide integer written here (tests only) gives the exact sign, and it agrees with the kernel wherever both decide. The
//! set is adversarial in four ways: the exact value is zero or tiny next to the terms (Lucas and Fibonacci pairs,
//! contacts of h_A's form and shallow crossings, cancelling products), binary64 cannot represent the inputs or the
//! partial sums (conversions that round, order-dependent sums), the 128-bit exact path overflows (the limits of
//! `i128`), and an intermediate value is not finite (∞, ∞ − ∞, ∞ · 0, and a finite value whose magnitude overflows).

use std::cmp::Ordering;

use c600_a2_filter::Factor::{Five, Int, Root5};
use c600_a2_filter::{
    Decision, Enclosure, Factor, Filtered, Outcome, Sum, FILTER_VERSION, MAX_FACTORS, MAX_TERMS, ROOT5_BITS,
};
use c600_kernel::exact::sign5;

type Term = (bool, Vec<Factor>);

fn pos(factors: &[Factor]) -> Term {
    (false, factors.to_vec())
}

fn neg(factors: &[Factor]) -> Term {
    (true, factors.to_vec())
}

fn sum(terms: &[Term]) -> Sum {
    let mut sum = Sum::new();
    for (negative, factors) in terms {
        sum.push(*negative, factors);
    }
    sum
}

fn parity(n: u32) -> i8 {
    if n.is_multiple_of(2) {
        1
    } else {
        -1
    }
}

/// A signed integer of any size: the exact reference where the 128-bit path overflows.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Wide {
    negative: bool,
    /// The magnitude, least significant 32-bit limb first, with no high zero limb; zero has no limbs.
    limbs: Vec<u32>,
}

impl Wide {
    fn new(negative: bool, mut limbs: Vec<u32>) -> Wide {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Wide { negative: negative && !limbs.is_empty(), limbs }
    }

    fn from_i128(v: i128) -> Wide {
        let m = v.unsigned_abs();
        Wide::new(v < 0, vec![m as u32, (m >> 32) as u32, (m >> 64) as u32, (m >> 96) as u32])
    }

    fn signum(&self) -> i8 {
        match (self.limbs.is_empty(), self.negative) {
            (true, _) => 0,
            (false, true) => -1,
            (false, false) => 1,
        }
    }

    fn negated(&self) -> Wide {
        Wide::new(!self.negative, self.limbs.clone())
    }

    fn plus(&self, other: &Wide) -> Wide {
        if self.negative == other.negative {
            return Wide::new(self.negative, add_magnitudes(&self.limbs, &other.limbs));
        }
        match compare_magnitudes(&self.limbs, &other.limbs) {
            Ordering::Less => Wide::new(other.negative, sub_magnitudes(&other.limbs, &self.limbs)),
            _ => Wide::new(self.negative, sub_magnitudes(&self.limbs, &other.limbs)),
        }
    }

    fn minus(&self, other: &Wide) -> Wide {
        self.plus(&other.negated())
    }

    fn times(&self, other: &Wide) -> Wide {
        let mut out = vec![0u32; self.limbs.len() + other.limbs.len()];
        for (i, &x) in self.limbs.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &y) in other.limbs.iter().enumerate() {
                let t = u64::from(x) * u64::from(y) + u64::from(out[i + j]) + carry;
                out[i + j] = t as u32;
                carry = t >> 32;
            }
            out[i + other.limbs.len()] = carry as u32;
        }
        Wide::new(self.negative != other.negative, out)
    }

    fn shifted(&self, bits: u32) -> Wide {
        let mut out = vec![0u32; (bits / 32) as usize];
        let mut carry = 0u64;
        for &x in &self.limbs {
            let t = (u64::from(x) << (bits % 32)) | carry;
            out.push(t as u32);
            carry = t >> 32;
        }
        out.push(carry as u32);
        Wide::new(self.negative, out)
    }
}

fn compare_magnitudes(a: &[u32], b: &[u32]) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

fn add_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut carry = 0u64;
    for i in 0..a.len().max(b.len()) {
        let t = u64::from(a.get(i).copied().unwrap_or(0)) + u64::from(b.get(i).copied().unwrap_or(0)) + carry;
        out.push(t as u32);
        carry = t >> 32;
    }
    out.push(carry as u32);
    out
}

/// a − b for magnitudes a ≥ b.
fn sub_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut borrow = false;
    for (i, &x) in a.iter().enumerate() {
        let (d, first) = x.overflowing_sub(b.get(i).copied().unwrap_or(0));
        let (d, second) = d.overflowing_sub(u32::from(borrow));
        out.push(d);
        borrow = first || second;
    }
    out
}

/// The exact value of a sum as x + y√5, with integers of any size.
fn wide_parts(sum: &Sum) -> (Wide, Wide) {
    let (mut x, mut y) = (Wide::from_i128(0), Wide::from_i128(0));
    for (negative, factors) in sum.terms() {
        let mut product = Wide::from_i128(if negative { -1 } else { 1 });
        let mut roots = 0;
        for factor in factors {
            match *factor {
                Int(v) => product = product.times(&Wide::from_i128(v)),
                Five => product = product.times(&Wide::from_i128(5)),
                Root5 => roots += 1,
            }
        }
        for _ in 0..roots / 2 {
            product = product.times(&Wide::from_i128(5));
        }
        if roots % 2 == 0 {
            x = x.plus(&product);
        } else {
            y = y.plus(&product);
        }
    }
    (x, y)
}

/// The sign of x + y√5: x² against 5y² when the signs differ, as `c600_kernel::exact::sign5` decides it in 256 bits.
fn wide_sign5(x: &Wide, y: &Wide) -> i8 {
    let (sx, sy) = (x.signum(), y.signum());
    if sy == 0 || sx == sy {
        return sx;
    }
    if sx == 0 {
        return sy;
    }
    let five_yy = y.times(y).times(&Wide::from_i128(5));
    if compare_magnitudes(&x.times(x).limbs, &five_yy.limbs) == Ordering::Greater {
        sx
    } else {
        sy
    }
}

/// A finite binary64 value as m·2^e.
fn dyadic(value: f64) -> (i128, i32) {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = i128::from(bits & ((1 << 52) - 1));
    assert_ne!(exponent, 0x7ff, "an enclosure is finite");
    let (m, e) = match (exponent, fraction) {
        (0, 0) => (0, 0),
        (0, _) => (fraction, -1074),
        _ => (fraction | (1 << 52), exponent - 1075),
    };
    (if bits >> 63 == 1 { -m } else { m }, e)
}

/// Whether the exact value x + y√5 lies in [value − bound, value + bound], decided exactly.
fn encloses(enclosure: Enclosure, x: &Wide, y: &Wide) -> bool {
    let ((mv, ev), (mb, eb)) = (dyadic(enclosure.value), dyadic(enclosure.bound));
    let low = ev.min(eb).min(0);
    let value = Wide::from_i128(mv).shifted((ev - low) as u32);
    let bound = Wide::from_i128(mb).shifted((eb - low) as u32);
    let (x, y) = (x.shifted(low.unsigned_abs()), y.shifted(low.unsigned_abs()));
    let above = wide_sign5(&x.minus(&value.minus(&bound)), &y) >= 0;
    let below = wide_sign5(&value.plus(&bound).minus(&x), &y.negated()) >= 0;
    above && below
}

/// How the adversarial cases went.
#[derive(Debug, Default)]
struct Tally {
    cases: usize,
    accepted: usize,
    near_zero: usize,
    non_finite: usize,
    outside_limits: usize,
    kernel_overflow: usize,
    /// Fallbacks where the sign of the binary64 value alone differs from the exact sign.
    float_alone_wrong: usize,
}

/// Decides a sum and checks the decision. Its sign is the exact sign: `c600_kernel::exact`'s where the 128-bit path
/// decides, the wide reference's otherwise, and the two agree where both decide. A finite enclosure holds the exact
/// value, an exact zero is never decided by the filter, and a fallback carries the exact path's result.
fn checked(sum: &Sum, tally: &mut Tally) -> (Decision, i8) {
    let decision = sum.decide();
    let (x, y) = wide_parts(sum);
    let exact = wide_sign5(&x, &y);
    let kernel = sum.to_exact().exact_sign();
    if let Some(sign) = kernel {
        assert_eq!(sign, exact, "the wide reference agrees with c600_kernel::exact");
    } else {
        tally.kernel_overflow += 1;
    }
    assert_eq!(decision.version, FILTER_VERSION);
    tally.cases += 1;
    match decision.filtered {
        Filtered::Sign(sign, enclosure) => {
            tally.accepted += 1;
            assert_eq!(sign, exact, "the filter decided against the exact sign: {enclosure:?}");
            assert!(enclosure.value.abs() > enclosure.bound);
            assert_eq!(decision.outcome, Outcome::Accepted(sign));
        }
        Filtered::NearZero(enclosure) => {
            tally.near_zero += 1;
            assert!(enclosure.value.abs() <= enclosure.bound);
            let alone = i8::from(enclosure.value > 0.0) - i8::from(enclosure.value < 0.0);
            tally.float_alone_wrong += usize::from(alone != exact);
        }
        Filtered::NonFinite => tally.non_finite += 1,
        Filtered::OutsideLimits => tally.outside_limits += 1,
    }
    if let Some(enclosure) = decision.filtered.enclosure() {
        assert!(encloses(enclosure, &x, &y), "the enclosure {enclosure:?} misses the exact value");
    }
    if !decision.filtered.is_decided() {
        assert_eq!(decision.outcome, kernel.map_or(Outcome::ExactOverflow, Outcome::Exact));
    }
    if exact == 0 {
        assert!(!decision.filtered.is_decided(), "an exact zero falls back");
    }
    match decision.outcome {
        Outcome::ExactOverflow => assert_eq!((decision.sign(), kernel), (None, None)),
        Outcome::Accepted(_) | Outcome::Exact(_) => assert_eq!(decision.sign(), Some(exact)),
    }
    (decision, exact)
}

/// (n, L_n, F_n) for every n whose Lucas number fits 128 bits. L_n − F_n√5 = 2ψ^n with ψ = (1 − √5)/2, so its sign is
/// (−1)^n and L_n² − 5F_n² = 4(−1)^n: the smallest margin a pair of this size can have.
fn lucas_fibonacci() -> Vec<(u32, i128, i128)> {
    let (mut f0, mut f1, mut l0, mut l1) = (0i128, 1i128, 2i128, 1i128);
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

/// A xorshift generator of integers in [−2^(bits − 1), 2^(bits − 1)).
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self, bits: u32) -> i128 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        i128::from(self.0 >> (64 - bits)) - (1i128 << (bits - 1))
    }
}

/// 125·Σ y_i·v_i − dy·dv·(1452 + 484√5) for y_i = a_i + b_i√5 and v_i = x_i + z_i√5. With positive dy and dv it has
/// the sign of y·v/(dy·dv) − κ, κ = α|n|² = (121/125)(12 + 4√5): h_A's form with its positive denominators cleared.
fn h_form(y: &[(i128, i128)], v: &[(i128, i128)], dy: i128, dv: i128) -> Sum {
    let mut terms = Vec::new();
    for (&(a, b), &(x, z)) in y.iter().zip(v) {
        terms.push(pos(&[Int(a), Int(x), Int(125)]));
        terms.push(pos(&[Five, Int(b), Int(z), Int(125)]));
        terms.push(pos(&[Int(a), Int(z), Root5, Int(125)]));
        terms.push(pos(&[Int(b), Int(x), Root5, Int(125)]));
    }
    terms.push(neg(&[Int(dy), Int(dv), Int(1452)]));
    terms.push(neg(&[Int(dy), Int(dv), Int(484), Root5]));
    sum(&terms)
}

/// Lucas and Fibonacci pairs at both signs up to the 128-bit limit: a − b√5 with a² − 5b² = ±4, the units
/// (L_3k − F_3k√5)/2 with a² − 5b² = ±1, and 5(L − F√5) written as L·√5·√5 − 5·F·√5, whose 5L overflows the 128-bit
/// path near the top. From n = 40 on the value is far below the rounding of its terms, and the filter falls back.
fn lucas_fibonacci_pairs(tally: &mut Tally) {
    let pairs = lucas_fibonacci();
    assert!(pairs.len() > 180, "the pairs reach the 128-bit limit");
    for &(n, l, f) in &pairs {
        let mut close = vec![
            (sum(&[pos(&[Int(l)]), neg(&[Int(f), Root5])]), parity(n)),
            (sum(&[neg(&[Int(l)]), pos(&[Int(f), Root5])]), -parity(n)),
            (sum(&[pos(&[Int(l), Root5, Root5]), neg(&[Five, Int(f), Root5])]), parity(n)),
        ];
        if n.is_multiple_of(3) {
            assert_eq!((l % 2, f % 2), (0, 0), "L_3k and F_3k are even");
            close.push((sum(&[pos(&[Int(l / 2)]), neg(&[Int(f / 2), Root5])]), parity(n)));
        }
        for (case, expected) in &close {
            let (decision, sign) = checked(case, tally);
            assert_eq!(sign, *expected, "n = {n}");
            if n >= 40 {
                assert!(!decision.filtered.is_decided(), "n = {n} falls back");
            }
            if n <= 20 {
                assert!(decision.filtered.is_decided(), "n = {n} is decided");
            }
        }
        let (_, sign) = checked(&sum(&[pos(&[Int(l)]), pos(&[Int(f), Root5])]), tally);
        assert_eq!(sign, 1);
    }
}

/// Exact contacts of h_A's form and crossings next to them. y_1 = a_1 + b_1√5 and v_2 to v_4 are drawn, with v_2 to
/// v_4 scaled by e = 125·|a_1² − 5b_1²|, and v_1 solves y·v = κ·dy·e exactly. Moving v_1 by ±(L_n − F_n√5) gives
/// shallow crossings, h = 125·y_1·(±2ψ^n), and moving it by ±2^k deeper ones.
fn contacts_and_crossings(tally: &mut Tally) {
    let pairs = lucas_fibonacci();
    let mut random = Xorshift(0x9e37_79b9_7f4a_7c15);
    let (mut decided, mut fell_back, mut contacts) = (0, 0, 0);
    for _ in 0..40 {
        let y: Vec<(i128, i128)> = (0..4).map(|_| (random.next(12), random.next(12))).collect();
        let rest: Vec<(i128, i128)> = (0..3).map(|_| (random.next(12), random.next(12))).collect();
        let dy = random.next(10).abs() + 1;
        let (a1, b1) = y[0];
        let d = a1 * a1 - 5 * b1 * b1;
        if d == 0 {
            continue;
        }
        let e = 125 * d.abs();
        let ra = 1452 * dy - 125 * y[1..].iter().zip(&rest).map(|(&(a, b), &(x, z))| a * x + 5 * b * z).sum::<i128>();
        let rb = 484 * dy - 125 * y[1..].iter().zip(&rest).map(|(&(a, b), &(x, z))| a * z + b * x).sum::<i128>();
        let mut v = vec![(d.signum() * (ra * a1 - 5 * rb * b1), d.signum() * (rb * a1 - ra * b1))];
        v.extend(rest.iter().map(|&(x, z)| (e * x, e * z)));
        let (decision, sign) = checked(&h_form(&y, &v, dy, e), tally);
        assert_eq!(sign, 0, "the contact is exact");
        assert!(!decision.filtered.is_decided());
        contacts += 1;
        let side = sign5(a1, b1);
        let mut crossings = Vec::new();
        for &(n, l, f) in &pairs[..61] {
            crossings.push(((l, -f), side * parity(n)));
            crossings.push(((-l, f), -side * parity(n)));
        }
        for k in 0..48 {
            crossings.push(((1 << k, 0), side));
            crossings.push(((-1 << k, 0), -side));
        }
        for ((dx, dz), expected) in crossings {
            let mut w = v.clone();
            w[0] = (w[0].0 + dx, w[0].1 + dz);
            let (decision, sign) = checked(&h_form(&y, &w, dy, e), tally);
            assert_eq!(sign, expected);
            if decision.filtered.is_decided() {
                decided += 1;
            } else {
                fell_back += 1;
            }
        }
    }
    assert!(contacts >= 30 && decided > 0 && fell_back > 0, "{contacts} contacts, {decided} decided, {fell_back}");
}

/// Conversions that round and the 128-bit limits, where the exact path's products and negations overflow.
fn conversions_and_limits(tally: &mut Tally) {
    let (max, min, big) = (i128::MAX, i128::MIN, |e: u32| 1i128 << e);
    // Each difference is ±1, which binary64 does not see after rounding the operands (2^64 + 2^11 is a tie).
    for (a, b) in [
        (big(53) + 1, big(53)),
        (big(100) + 1, big(100)),
        (big(64) + big(11), big(64) + big(11) - 1),
        (big(64) + big(11) + 1, big(64) + big(11)),
        (max, max - 1),
        (min + 1, min),
    ] {
        for (s, t) in [(a, b), (b, a)] {
            let (decision, sign) = checked(&sum(&[pos(&[Int(s)]), neg(&[Int(t)])]), tally);
            assert_eq!(sign, (s - t).signum() as i8);
            assert!(!decision.filtered.is_decided(), "{s} - {t} falls back");
        }
    }
    // Sixteen conversions that round the same way: (2^53 + 1)^16 − (2^53)^16 ≈ 2^799, computed as 0.
    let (up, down) = (vec![Int(big(53) + 1); MAX_FACTORS], vec![Int(big(53)); MAX_FACTORS]);
    for (case, expected) in [(sum(&[pos(&up), neg(&down)]), 1), (sum(&[pos(&down), neg(&up)]), -1)] {
        let (decision, sign) = checked(&case, tally);
        assert_eq!(sign, expected);
        assert_eq!(decision.filtered.enclosure().map(|e| e.value), Some(0.0));
    }
    let cases = [
        (sum(&[pos(&[Int(max)]), neg(&[Int(max)])]), 0),
        (sum(&[pos(&[Int(max)]), pos(&[Int(min)])]), -1),
        (sum(&[pos(&[Int(min + 1)]), pos(&[Int(max)])]), 0),
        (sum(&[neg(&[Int(min)])]), 1),
        (sum(&[pos(&[Int(max), Int(2)])]), 1),
        (sum(&[pos(&[Int(max), Int(2)]), neg(&[Int(max), Int(2)])]), 0),
        (sum(&[pos(&[Int(max), Int(max)]), neg(&[Int(max), Int(max - 1)])]), 1),
        (sum(&[pos(&[Int(min), Int(min)]), neg(&[Int(max), Int(max)])]), 1),
        (sum(&[pos(&[Int(max)]), pos(&[Int(min), Root5])]), -1),
        (sum(&[pos(&[Int(min)]), pos(&[Int(max), Root5])]), 1),
    ];
    for (case, expected) in &cases {
        assert_eq!(checked(case, tally).1, *expected);
    }
}

/// Cancellation-heavy sums: products and their negations in the reverse order, around a small residual, and sums
/// whose binary64 value depends on the order of addition.
fn cancellation(tally: &mut Tally) {
    // 2^53 absorbs each 1, so binary64 computes −2 where the exact value is 1, and 2 where it is −1.
    let unit = 1i128 << 53;
    for (negative, value) in [(false, -2.0), (true, 2.0)] {
        let mut terms = vec![(negative, vec![Int(unit)])];
        terms.extend(vec![(negative, vec![Int(1)]); 3]);
        terms.push((!negative, vec![Int(unit + 2)]));
        let (decision, sign) = checked(&sum(&terms), tally);
        assert_eq!(sign, if negative { -1 } else { 1 });
        assert_eq!(decision.filtered.enclosure().map(|e| e.value), Some(value));
        assert!(!decision.filtered.is_decided());
    }
    // 2^54 − 1 is a tie that rounds back to 2^54, so binary64 computes 36 for 2^54 − m − (2^54 − 36) at every m: the
    // error grows with the number of additions, which the bound counts.
    for m in [1, 35, 36, 37, 100, 1000] {
        let mut terms = vec![pos(&[Int(1 << 54)])];
        terms.extend(vec![neg(&[Int(1)]); m]);
        terms.push(neg(&[Int((1 << 54) - 36)]));
        let (decision, sign) = checked(&sum(&terms), tally);
        assert_eq!(sign, (36 - m as i128).signum() as i8);
        assert_eq!(decision.filtered.enclosure().map(|e| e.value), Some(36.0));
        assert!(!decision.filtered.is_decided());
    }
    let pairs = lucas_fibonacci();
    let mut random = Xorshift(0x2545_f491_4f6c_dd1d);
    for round in 0..200 {
        let products: Vec<Vec<Factor>> = (0..8)
            .map(|_| {
                let count = 1 + ((random.next(3) + 4) % 5) as usize;
                (0..count)
                    .map(|_| match random.next(3) {
                        -4 | -3 => Root5,
                        -2 => Five,
                        _ => Int(random.next(21)),
                    })
                    .collect()
            })
            .collect();
        let (n, l, f) = pairs[20 + round % 60];
        let residuals = [
            (vec![], 0),
            (vec![pos(&[Int(1)])], 1),
            (vec![neg(&[Int(1)])], -1),
            (vec![pos(&[Int(l)]), neg(&[Int(f), Root5])], parity(n)),
        ];
        for (residual, expected) in residuals {
            let mut terms: Vec<Term> = products.iter().map(|p| pos(p)).collect();
            terms.extend(residual);
            terms.extend(products.iter().rev().map(|p| neg(p)));
            assert_eq!(checked(&sum(&terms), tally).1, expected);
        }
    }
}

/// Non-finite intermediate values inside the limits, and finite values near the top of the binary64 range.
fn non_finite(tally: &mut Tally) {
    // (2^127 − 1)^9 overflows binary64; X = (2^126)^8·3·2^14 = 1.5·2^1023 is exact and 2X overflows.
    let huge = vec![Int(i128::MAX); 9];
    let times_zero = [huge.clone(), vec![Int(0)]].concat();
    let zero_first = [vec![Int(0)], huge.clone()].concat();
    let x = [vec![Int(1 << 126); 8], vec![Int(3 << 14)]].concat();
    let cases = [
        (sum(&[pos(&huge)]), 1, Filtered::NonFinite),
        (sum(&[pos(&huge), neg(&huge)]), 0, Filtered::NonFinite),
        (sum(&[pos(&times_zero)]), 0, Filtered::NonFinite),
        (sum(&[pos(&x), pos(&x)]), 1, Filtered::NonFinite),
        (sum(&[pos(&x), neg(&x), pos(&x)]), 1, Filtered::NonFinite),
    ];
    for (case, expected, filtered) in &cases {
        let (decision, sign) = checked(case, tally);
        assert_eq!((sign, decision.filtered), (*expected, *filtered));
    }
    // A zero factor first keeps every product finite: an exact zero, near zero.
    let (decision, sign) = checked(&sum(&[pos(&zero_first)]), tally);
    assert!(sign == 0 && matches!(decision.filtered, Filtered::NearZero(_)));
    // X alone is finite, and so is its bound: decided, where the 128-bit path overflows.
    let (decision, sign) = checked(&sum(&[pos(&x)]), tally);
    assert!(sign == 1 && decision.outcome == Outcome::Accepted(1));
}

/// Powers of √5, and the limits on factors and terms.
fn root5_powers_and_limits(tally: &mut Tally) {
    let roots = |count: usize| vec![Root5; count];
    let cases = [
        (sum(&[pos(&roots(16)), neg(&[Int(390_625)])]), 0, false),
        (sum(&[pos(&roots(15)), neg(&[Int(78_125), Root5])]), 0, false),
        (sum(&[pos(&roots(16)), neg(&[Int(390_624)])]), 1, true),
        (sum(&[pos(&roots(MAX_FACTORS))]), 1, true),
        (sum(&vec![pos(&[Int(1)]); MAX_TERMS]), 1, true),
    ];
    for (case, expected, decided) in &cases {
        let (decision, sign) = checked(case, tally);
        assert_eq!((sign, decision.filtered.is_decided()), (*expected, *decided));
    }
    // Past the limits; 5^100 also overflows the 128-bit path.
    let past =
        [sum(&[pos(&roots(MAX_FACTORS + 1))]), sum(&[pos(&roots(200))]), sum(&vec![pos(&[Int(1)]); MAX_TERMS + 1])];
    for case in &past {
        let (decision, sign) = checked(case, tally);
        assert_eq!((sign, decision.filtered), (1, Filtered::OutsideLimits));
    }
}

/// A pseudo-random family of h_A's form with 30-bit coordinates.
fn random_h_forms(tally: &mut Tally) {
    let mut random = Xorshift(0x2545_f491_4f6c_dd1d);
    for _ in 0..2000 {
        let y: Vec<(i128, i128)> = (0..4).map(|_| (random.next(30), random.next(30))).collect();
        let v: Vec<(i128, i128)> = (0..4).map(|_| (random.next(30), random.next(30))).collect();
        let (dy, dv) = (random.next(20).abs() + 1, random.next(20).abs() + 1);
        checked(&h_form(&y, &v, dy, dv), tally);
    }
}

/// CT-KN3-03 (part: the filter's decision equals the exact sign of `c600_kernel::exact`, or of a wide-integer
/// reference where the 128-bit path overflows, and every finite enclosure holds the exact value, on adversarial sums of
/// products: Lucas and Fibonacci pairs with |a² − 5b²| = 4 or 1 at both signs up to the 128-bit limit, alone and
/// times 5; exact contacts y·v = κ of h_A's cleared form with shallow and deeper crossings next to them; conversions
/// that round, the 128-bit limits, cancellation-heavy and order-dependent sums, powers of √5, non-finite intermediate
/// values and sums past the limits. The filter falls back on every exact zero and on the pairs from n = 40, both its
/// paths run, and binary64 alone gets some of these signs wrong; not run: the filter on the model's own vertex sets
/// and poses and on the state contract's E0 controls, because neither `c600-kernel` nor `c600-jumble` calls it yet)
#[test]
fn the_filter_never_differs_from_the_exact_sign() {
    let mut tally = Tally::default();
    lucas_fibonacci_pairs(&mut tally);
    contacts_and_crossings(&mut tally);
    conversions_and_limits(&mut tally);
    cancellation(&mut tally);
    non_finite(&mut tally);
    root5_powers_and_limits(&mut tally);
    random_h_forms(&mut tally);
    assert!(tally.accepted > 0 && tally.near_zero > 0 && tally.non_finite > 0 && tally.outside_limits > 0, "{tally:?}");
    assert!(tally.kernel_overflow > 0 && tally.float_alone_wrong > 0, "{tally:?}");
}

#[test]
fn the_constants_are_the_versioned_ones() {
    assert_eq!(FILTER_VERSION, "a2-binary64-sum-of-products-draft-1");
    assert_eq!((MAX_TERMS, MAX_FACTORS), (1 << 16, 16));
    // √5 = m·2^-51 rounded to nearest exactly when (2m − 1)² < 5·2^104 < (2m + 1)², checked in integers.
    assert_eq!(ROOT5_BITS >> 52, 0x400);
    let m = u128::from((ROOT5_BITS & ((1 << 52) - 1)) | (1 << 52));
    assert!((2 * m - 1) * (2 * m - 1) < 5 << 104 && 5 << 104 < (2 * m + 1) * (2 * m + 1));
    assert_eq!(f64::from_bits(ROOT5_BITS), 5f64.sqrt());
}

#[test]
fn the_decision_records_what_it_used() {
    // Decided: 3 − √5, with the enclosure the sign came from.
    let decision = sum(&[pos(&[Int(3)]), neg(&[Int(1), Root5])]).decide();
    assert_eq!((decision.version, decision.outcome, decision.sign()), (FILTER_VERSION, Outcome::Accepted(1), Some(1)));
    let enclosure = decision.filtered.enclosure().expect("a finite evaluation");
    assert_eq!(enclosure.value, 3.0 - f64::from_bits(ROOT5_BITS));
    assert!(enclosure.bound > 0.0 && enclosure.value > enclosure.bound);
    // Near zero: √5·√5 − 5 falls back, and the exact path's result is recorded with the enclosure that did not decide.
    let decision = sum(&[pos(&[Int(1), Root5, Root5]), neg(&[Five])]).decide();
    assert!(matches!(decision.filtered, Filtered::NearZero(e) if e.value.abs() <= e.bound));
    assert_eq!((decision.outcome, decision.sign()), (Outcome::Exact(0), Some(0)));
    // Not finite: no enclosure, and the 128-bit path overflows too.
    let huge = vec![Int(i128::MAX); 9];
    let decision = sum(&[pos(&huge), neg(&huge)]).decide();
    assert_eq!(
        (decision.filtered, decision.outcome, decision.sign()),
        (Filtered::NonFinite, Outcome::ExactOverflow, None)
    );
    assert_eq!(decision.filtered.enclosure(), None);
    // Past the limits nothing is evaluated.
    let decision = sum(&vec![pos(&[Int(1)]); MAX_TERMS + 1]).decide();
    assert_eq!((decision.filtered, decision.outcome), (Filtered::OutsideLimits, Outcome::Exact(1)));
    // The empty sum is an exact zero with the zero enclosure.
    let decision = Sum::new().decide();
    assert_eq!(decision.filtered, Filtered::NearZero(Enclosure { value: 0.0, bound: 0.0 }));
    assert_eq!(decision.outcome, Outcome::Exact(0));
}
