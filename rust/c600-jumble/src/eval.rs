//! The exact sign kernel: signs of y·v − κ for a pulled-back cut normal y = Gᵀn_e over vertex sets.
//!
//! All poles share |n|² = 12 + 4√5, so the sign of h_e(G v) is the sign of y·v − κ with κ = (1452 + 484√5)/125.
//! A row is evaluated in 128-bit integers when every product fits, and in big integers otherwise; both give the
//! exact sign.

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::model::KAPPA_FORM;
use crate::q5::{dot, int_form, sign5, sign5_small, Mat4, Q5, SMALL_LIMIT};
use crate::regions::VForm;

/// The outcome of classifying one vertex set, with the number of exact evaluations used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Every vertex has h ≥ 0.
    In(u64),
    /// Every vertex has h ≤ 0.
    Out(u64),
    /// Vertices `below` (h < 0) and `above` (h > 0), the first of each sign.
    Straddle { n: u64, below: usize, above: usize },
    /// The budget ran out first.
    Uncertain(u64),
}

impl Class {
    pub fn evaluations(&self) -> u64 {
        match *self {
            Class::In(n) | Class::Out(n) | Class::Uncertain(n) | Class::Straddle { n, .. } => n,
        }
    }
}

pub struct Evaluator {
    a: [BigInt; 4],
    b: [BigInt; 4],
    dy: BigInt,
    small: Option<([i128; 4], [i128; 4], i128)>,
}

/// y = Gᵀn: y_j = Σ_i G[i][j]·n[i].
pub fn pullback(g: &Mat4, n: &[Q5]) -> Vec<Q5> {
    (0..4).map(|j| dot(&[g[0][j].clone(), g[1][j].clone(), g[2][j].clone(), g[3][j].clone()], n)).collect()
}

const SMALL_INPUT: i128 = 1 << 62;

impl Evaluator {
    pub fn new(y: &[Q5]) -> Evaluator {
        let form = int_form(y);
        let k3 = BigInt::from(KAPPA_FORM.2);
        let a: [BigInt; 4] = std::array::from_fn(|i| &form.a[i] * &k3);
        let b: [BigInt; 4] = std::array::from_fn(|i| &form.b[i] * &k3);
        let fits = |x: &BigInt| x.to_i128().filter(|v| v.unsigned_abs() < SMALL_INPUT as u128);
        let small = (|| {
            let sa = [fits(&a[0])?, fits(&a[1])?, fits(&a[2])?, fits(&a[3])?];
            let sb = [fits(&b[0])?, fits(&b[1])?, fits(&b[2])?, fits(&b[3])?];
            Some((sa, sb, fits(&form.d)?))
        })();
        Evaluator { a, b, dy: form.d, small }
    }

    fn sign_small(&self, e: i64, row: &[i64; 8]) -> Option<i32> {
        let (a, b, dy) = self.small.as_ref()?;
        let mut p: i128 = 0;
        let mut q: i128 = 0;
        for i in 0..4 {
            let (x, z) = (row[i] as i128, row[4 + i] as i128);
            p = p.checked_add(a[i].checked_mul(x)?)?.checked_add(b[i].checked_mul(z)?.checked_mul(5)?)?;
            q = q.checked_add(a[i].checked_mul(z)?)?.checked_add(b[i].checked_mul(x)?)?;
        }
        let de = dy.checked_mul(e as i128)?;
        let x = p.checked_sub(de.checked_mul(KAPPA_FORM.0 as i128)?)?;
        let y = q.checked_sub(de.checked_mul(KAPPA_FORM.1 as i128)?)?;
        if x.unsigned_abs() >= SMALL_LIMIT || y.unsigned_abs() >= SMALL_LIMIT {
            return None;
        }
        Some(sign5_small(x, y))
    }

    fn sign_big(&self, e: i64, row: &[i64; 8]) -> i32 {
        let mut p = BigInt::from(0);
        let mut q = BigInt::from(0);
        for i in 0..4 {
            let (x, z) = (BigInt::from(row[i]), BigInt::from(row[4 + i]));
            p += &self.a[i] * &x + BigInt::from(5) * &self.b[i] * &z;
            q += &self.a[i] * &z + &self.b[i] * &x;
        }
        let de = &self.dy * BigInt::from(e);
        sign5(&(p - &de * KAPPA_FORM.0), &(q - &de * KAPPA_FORM.1))
    }

    pub fn sign_row(&self, e: i64, row: &[i64; 8]) -> i32 {
        self.sign_small(e, row).unwrap_or_else(|| self.sign_big(e, row))
    }

    /// Scans the rows in order and stops as soon as both signs are seen; with a budget, `Uncertain` when it runs out.
    pub fn classify(&self, v: &VForm, budget: Option<u64>) -> Class {
        let mut below = None;
        let mut above = None;
        let mut n = 0u64;
        for (j, row) in v.rows.iter().enumerate() {
            if budget.is_some_and(|b| n >= b) {
                return Class::Uncertain(n);
            }
            let s = self.sign_row(v.e, row);
            n += 1;
            if s < 0 {
                let first_below = *below.get_or_insert(j);
                if let Some(first_above) = above {
                    return Class::Straddle { n, below: first_below, above: first_above };
                }
            } else if s > 0 {
                let first_above = *above.get_or_insert(j);
                if let Some(first_below) = below {
                    return Class::Straddle { n, below: first_below, above: first_above };
                }
            }
        }
        match (below, above) {
            (None, _) => Class::In(n),
            (_, None) => Class::Out(n),
            _ => unreachable!("both signs end the scan"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_and_big_paths_agree() {
        // y = (1, 0, 0, 0): the sign of x₀ − κ with κ ≈ 20.27, for x₀ = 20 and 21.
        let y = vec![Q5::one(), Q5::zero(), Q5::zero(), Q5::zero()];
        let ev = Evaluator::new(&y);
        let rows = vec![[20, 0, 0, 0, 0, 0, 0, 0], [21, 0, 0, 0, 0, 0, 0, 0]];
        for row in &rows {
            assert_eq!(ev.sign_small(1, row), Some(ev.sign_big(1, row)));
        }
        assert_eq!(ev.sign_row(1, &rows[0]), -1);
        assert_eq!(ev.sign_row(1, &rows[1]), 1);
        let v = VForm { e: 1, rows };
        assert_eq!(ev.classify(&v, None), Class::Straddle { n: 2, below: 0, above: 1 });
        assert_eq!(ev.classify(&v, Some(1)), Class::Uncertain(1));
        assert_eq!(ev.classify(&v, Some(0)), Class::Uncertain(0));
    }
}
