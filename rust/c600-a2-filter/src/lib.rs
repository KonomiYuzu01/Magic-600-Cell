//! The A2 filter (state contract A2, KN3.5; LP2): a binary64 sign test with a proven forward error bound that falls
//! back to the exact path of `c600_kernel::exact`.
//!
//! The filter evaluates a sum of signed products of integers, √5 and 5 in binary64: the form that
//! `c600_kernel::exact::SumOfProducts` holds, h_A = n·Gv/|n|² − α at an exact vertex with its positive denominators
//! cleared. It accepts the sign of the computed value only when the value's magnitude exceeds a proven bound on its
//! error. Otherwise the exact path decides (KN3.1): when the value is near zero, when an intermediate value is not
//! finite, and when the sum is past the limits the proof covers. Its result therefore never differs from the exact one
//! (KN3.5), and a [`Decision`] records what it used: the arithmetic and error-bound version ([`FILTER_VERSION`]), the
//! computed enclosure, and the accepted sign or the exact fallback result. The exact inputs (pose and vertex
//! identities), the state and model identity and the coverage proof are the caller's half of the certificate.
//!
//! INV-4 and ADR-L03.1 allow the filter its float, with its proven bound. CT-KN2-05's scans keep every float type and
//! literal out of `c600-kernel` and `c600-jumble`, so the filter is a crate of its own, and neither of those crates
//! calls it yet.
//!
//! # The bound
//!
//! Binary64 rounds to nearest with unit roundoff u = 2^-53, and γ(k) = ku/(1 − ku). A term is a sign times a product
//! of m factors, multiplied in order from 1.0. An integer converts with a relative error of at most u (`as f64` rounds
//! to nearest), √5 is the correctly rounded constant [`ROOT5_BITS`] (relative error below u), 5 is exact, and each of
//! the m − 1 products after the first factor rounds once. So a term t_j is computed as t_j(1 + θ_j) with
//! |θ_j| ≤ γ(k_j), where k_j counts the term's integers, its √5 factors and its m − 1 products; an exact conversion
//! counts all the same. The value v̂ = fl(Σ ±t̂_j) and the magnitude Ŝ = fl(Σ |t̂_j|) are summed from 0.0 in the same
//! order, which adds at most T − 1 roundings for T terms. With K = max k_j + T − 1, N = Σ t_j and S = Σ |t_j|:
//!
//! - |v̂ − N| ≤ γ(K)·S and Ŝ ≥ (1 − γ(K))·S, so |v̂ − N| ≤ γ(K)/(1 − γ(K))·Ŝ = Ku/(1 − 2Ku)·Ŝ;
//! - the bound E = fl(Ŝ·4(K + 1)u), where 4(K + 1)u is exact, is at least 4(K + 1)(1 − u)u·Ŝ, which is at least
//!   3.99 times Ku/(1 − 2Ku)·Ŝ: the limits [`MAX_TERMS`] and [`MAX_FACTORS`] keep K below 2^17, so Ku is below 2^-36.
//!
//! So N lies in [v̂ − E, v̂ + E], the enclosure a certificate records, and when |v̂| > E the sign of N is the sign of
//! v̂ and N ≠ 0. A tie |v̂| = E falls back.
//!
//! The model above assumes no underflow and no overflow. Nothing underflows: every nonzero factor, and so every nonzero
//! computed product, has magnitude at least 1, an addition is exact whenever its result is subnormal, and Ŝ is 0 or at
//! least 1, so E is 0 or at least 2^-51. An overflow leaves v̂ or E infinite or NaN, because ∞ and NaN pass through
//! the remaining products and sums and E is a positive multiple of Ŝ; the filter compares only a finite v̂ with a
//! finite E and falls back otherwise.
//!
//! # Draft (LP2)
//!
//! - [`Sum`] keeps its own terms in the shape of `c600_kernel::exact::SumOfProducts` and builds that sum only on a
//!   fallback, because the kernel type's terms are private and this step changes nothing in `c600-kernel`. An accessor
//!   for those terms would let the filter read the kernel's sum directly.
//! - The limits are chosen, not frozen. Past them the filter evaluates nothing and falls back
//!   ([`Filtered::OutsideLimits`]); it never panics.
//! - √5 is a fixed bit pattern, checked against the correctly rounded value with integer arithmetic in the tests, so no
//!   platform square root enters the bound.
//! - The bound keeps a margin of almost 4 over the proven one and counts exact conversions as roundings.
//! - A non-finite evaluation records no value ([`Filtered::NonFinite`]): NaN payloads are not portable, and a
//!   certificate records a finite enclosure only.
//! - When the filter falls back and the 128-bit exact path overflows, the decision says so ([`Outcome::ExactOverflow`])
//!   and the caller decides with unbounded integers (`c600-jumble`'s `q5`); this crate uses the standard library and
//!   `c600-kernel` only.
//! - A change to the bound, the √5 constant, the evaluation order or the limits takes a new [`FILTER_VERSION`].

#![forbid(unsafe_code)]

pub use c600_kernel::exact::Factor;
use c600_kernel::exact::SumOfProducts;

/// The arithmetic and error-bound version a certificate records (state contract A2).
pub const FILTER_VERSION: &str = "a2-binary64-sum-of-products-draft-1";

/// The most terms the bound covers.
pub const MAX_TERMS: usize = 1 << 16;
/// The most factors of one term the bound covers.
pub const MAX_FACTORS: usize = 16;

/// √5 rounded to the nearest binary64 value, 5,035,177,455,121,576·2^-51.
pub const ROOT5_BITS: u64 = 0x4001_e377_9b97_f4a8;

/// A sum of signed products of integers, √5 and 5, in the shape of `c600_kernel::exact::SumOfProducts`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sum {
    terms: Vec<(bool, Vec<Factor>)>,
}

impl Sum {
    pub fn new() -> Sum {
        Sum::default()
    }

    /// Adds `+ Π factors` (or `− Π factors` when `negative`). Any size is accepted; past the limits the filter falls
    /// back.
    pub fn push(&mut self, negative: bool, factors: &[Factor]) {
        self.terms.push((negative, factors.to_vec()));
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// The terms in order, each as (negative, factors).
    pub fn terms(&self) -> impl Iterator<Item = (bool, &[Factor])> + '_ {
        self.terms.iter().map(|(negative, factors)| (*negative, factors.as_slice()))
    }

    /// The same sum for the exact path.
    pub fn to_exact(&self) -> SumOfProducts {
        let mut exact = SumOfProducts::new();
        for (negative, factors) in self.terms() {
            exact.push(negative, factors);
        }
        exact
    }

    /// The filter alone: the sign when the binary64 value exceeds the proven bound, otherwise why it falls back.
    pub fn filter(&self) -> Filtered {
        if self.terms.len() > MAX_TERMS || self.terms.iter().any(|(_, factors)| factors.len() > MAX_FACTORS) {
            return Filtered::OutsideLimits;
        }
        let root5 = f64::from_bits(ROOT5_BITS);
        let (mut value, mut magnitude) = (0.0f64, 0.0f64);
        let mut most = 0u32;
        for (negative, factors) in &self.terms {
            // Inside the limits every count fits a u32 exactly.
            let mut roundings = factors.len().saturating_sub(1) as u32;
            let mut term = 1.0f64;
            for factor in factors {
                term *= match *factor {
                    Factor::Int(v) => {
                        roundings += 1;
                        v as f64
                    }
                    Factor::Root5 => {
                        roundings += 1;
                        root5
                    }
                    Factor::Five => 5.0,
                };
            }
            most = most.max(roundings);
            value += if *negative { -term } else { term };
            magnitude += term.abs();
        }
        let k = most + (self.terms.len() as u32).saturating_sub(1);
        // 4(K + 1) < 2^19, so 4(K + 1)·2^-53 is exact.
        let bound = magnitude * (f64::from(4 * (k + 1)) * (0.5 * f64::EPSILON));
        if !value.is_finite() || !bound.is_finite() {
            return Filtered::NonFinite;
        }
        let enclosure = Enclosure { value, bound };
        if value.abs() > bound {
            Filtered::Sign(if value > 0.0 { 1 } else { -1 }, enclosure)
        } else {
            Filtered::NearZero(enclosure)
        }
    }

    /// The decision: the filter's sign when it decides, otherwise the exact path's (KN3.5).
    pub fn decide(&self) -> Decision {
        let filtered = self.filter();
        let outcome = if let Filtered::Sign(sign, _) = filtered {
            Outcome::Accepted(sign)
        } else {
            self.to_exact().exact_sign().map_or(Outcome::ExactOverflow, Outcome::Exact)
        };
        Decision { version: FILTER_VERSION, filtered, outcome }
    }
}

/// The computed enclosure: the exact value lies in [value − bound, value + bound]. Both are finite binary64 values,
/// which a certificate can write exactly, for example with `c600_kernel::request::token_of_binary64` on their bits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Enclosure {
    pub value: f64,
    pub bound: f64,
}

/// What the filter did with a sum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Filtered {
    /// |value| > bound: the sign of the value, -1 or 1, is the exact sign.
    Sign(i8, Enclosure),
    /// |value| ≤ bound: the exact value may be zero or of either sign, so the exact path decides.
    NearZero(Enclosure),
    /// An intermediate value overflowed, so the value or the bound is not finite and the exact path decides.
    NonFinite,
    /// The sum has more terms, or a term more factors, than the bound covers; nothing was evaluated.
    OutsideLimits,
}

impl Filtered {
    /// Whether the filter decided the sign.
    pub fn is_decided(&self) -> bool {
        matches!(self, Filtered::Sign(..))
    }

    /// The computed enclosure, when the evaluation was finite.
    pub fn enclosure(&self) -> Option<Enclosure> {
        match *self {
            Filtered::Sign(_, enclosure) | Filtered::NearZero(enclosure) => Some(enclosure),
            Filtered::NonFinite | Filtered::OutsideLimits => None,
        }
    }
}

/// The sign a decision gives, and where it comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The filter's sign, accepted.
    Accepted(i8),
    /// The exact fallback result of `c600_kernel::exact`: -1, 0 or 1.
    Exact(i8),
    /// The filter fell back and the 128-bit exact path overflowed: only unbounded integers decide.
    ExactOverflow,
}

/// A decision and what it used, for the certificate (state contract A2): the arithmetic and error-bound version, the
/// computed enclosure, and the accepted sign or the exact fallback result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decision {
    /// Always [`FILTER_VERSION`].
    pub version: &'static str,
    pub filtered: Filtered,
    pub outcome: Outcome,
}

impl Decision {
    /// The exact sign, -1, 0 or 1, or `None` when only unbounded integers can decide.
    pub fn sign(&self) -> Option<i8> {
        match self.outcome {
            Outcome::Accepted(sign) | Outcome::Exact(sign) => Some(sign),
            Outcome::ExactOverflow => None,
        }
    }
}
