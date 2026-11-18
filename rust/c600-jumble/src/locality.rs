//! LB1.10's independence relation `jumble-locality/1` of the retained jumbling geometry (LP2 draft): which grips can
//! act on a common piece.
//!
//! Stage 14's Lemma L1 (`research/jumbling/restoration/14-blocking-structure/PROOFS.md`) puts, for grips c ≠ d,
//!
//! ```text
//! Δ(c, d) = R²(|n|² + n_c·n_d) − 2κ²  ∈ Q(√5),
//! ```
//!
//! with |n|² = 12 + 4√5, κ = (121/125)|n|² and R² the largest squared norm of a vertex of cap 0's region, which the
//! lemma states as 96 − 32√5. Distinct grips are independent exactly when Δ(c, d) ≤ 0: then no piece is ever inside
//! both caps, and a letter at one grip changes nothing at the other (L10). Distinct grips interact when Δ(c, d) > 0.
//! Letters at one grip are dependent, which is no pair of this table.
//!
//! [`Locality::build`] regenerates the table from the loaded model: R² from cap 0's exact region vertices, each pair's
//! n_c·n_d from the integer pole forms, and Δ in Q(√5) once per distinct n_c·n_d. It refuses a model whose R² is not
//! 96 − 32√5 or whose grips do not all have [`PARTNERS`] partners, the count K⁺'s transitivity on the grips gives and
//! stage 14 computed (`results/locality-summary.json`).

use std::collections::BTreeMap;

use crate::model::{ModelError, POLES};
use crate::q5::{dot, Q5};
use crate::Context;

/// LB1.10's independence contract of the retained jumbling geometry.
pub const JUMBLE_LOCALITY: &str = "jumble-locality/1";

/// The grips each grip interacts with (stage 14, section 5.3).
pub const PARTNERS: usize = 80;

/// R² = 96 − 32√5, as (a, b) of a + b√5.
pub const R2_FORM: (i64, i64) = (96, -32);

/// The exact locality table of the loaded model.
pub struct Locality {
    r2: Q5,
    /// One bit per pair (c, d) of grips with c < d, in ascending order of c and then of d, least significant bit first
    /// in each byte, set when the pair is independent. The last byte's unused bits are clear.
    bits: Vec<u8>,
    /// For each grip, the grips it interacts with, ascending.
    partners: Vec<Vec<u16>>,
    /// Each pair's n_c·n_d class, as an index into `dots`, by pair index.
    class: Vec<u8>,
    /// The distinct values of n_c·n_d over pairs of distinct grips, ascending, each with its Δ.
    dots: Vec<(Q5, Q5)>,
}

/// The index of the pair (c, d), c < d, among the pairs of `n` grips in ascending order.
fn pair_index(n: usize, c: usize, d: usize) -> usize {
    c * (2 * n - c - 1) / 2 + (d - c - 1)
}

impl Locality {
    /// The table of `ctx`'s model. Refused when R² is not 96 − 32√5, or when a grip does not interact with exactly
    /// [`PARTNERS`] others.
    pub fn build(ctx: &Context) -> Result<Locality, ModelError> {
        let model = &ctx.model;
        let n = model.poles.len();
        if n != POLES {
            return Err(ModelError(format!("{n} poles, not {POLES}")));
        }
        let r2 = ctx.regions.cap_vertices.iter().map(|v| dot(v, v)).max().unwrap_or_else(Q5::zero);
        if r2 != Q5::of(R2_FORM.0, R2_FORM.1, 1) {
            return Err(ModelError("the squared circumradius of cap 0's region is not 96 - 32 sqrt5".into()));
        }
        let two_kappa_sq = model.kappa.mul(&model.kappa).mul(&Q5::int(2));
        // n = (n1 + n2√5)/2, so 4 n_c·n_d = (n1c·n1d + 5 n2c·n2d) + (n1c·n2d + n2c·n1d)√5, in integers.
        let four_dot = |c: usize, d: usize| {
            let (a1, a2, b1, b2) = (&model.n1[c], &model.n2[c], &model.n1[d], &model.n2[d]);
            let (mut x, mut y) = (0i64, 0i64);
            for i in 0..4 {
                x += a1[i] * b1[i] + 5 * a2[i] * b2[i];
                y += a1[i] * b2[i] + a2[i] * b1[i];
            }
            (x, y)
        };
        let pairs = n * (n - 1) / 2;
        let mut forms: BTreeMap<(i64, i64), u8> = BTreeMap::new();
        let mut raw = Vec::with_capacity(pairs);
        for c in 0..n {
            for d in c + 1..n {
                let form = four_dot(c, d);
                let next = forms.len();
                let class = *forms.entry(form).or_insert_with(|| next as u8);
                if forms.len() > usize::from(u8::MAX) {
                    return Err(ModelError("more than 255 distinct pole products".into()));
                }
                raw.push(class);
            }
        }
        // The classes in the order of their n_c·n_d, with Δ for each.
        let mut values: Vec<(Q5, u8)> =
            forms.iter().map(|(&(x, y), &class)| (Q5::of(x, y, 4), class)).collect::<Vec<_>>();
        values.sort();
        let mut rank = vec![0u8; values.len()];
        let mut dots = Vec::with_capacity(values.len());
        for (i, (value, class)) in values.into_iter().enumerate() {
            let delta = r2.mul(&model.nn.add(&value)).sub(&two_kappa_sq);
            rank[usize::from(class)] = i as u8;
            dots.push((value, delta));
        }
        let class: Vec<u8> = raw.into_iter().map(|c| rank[usize::from(c)]).collect();
        let interacts: Vec<bool> = dots.iter().map(|(_, delta)| delta.sign() > 0).collect();
        let mut bits = vec![0u8; pairs.div_ceil(8)];
        let mut partners = vec![Vec::new(); n];
        let mut k = 0usize;
        for c in 0..n {
            for d in c + 1..n {
                if interacts[usize::from(class[k])] {
                    partners[c].push(d as u16);
                    partners[d].push(c as u16);
                } else {
                    bits[k / 8] |= 1 << (k % 8);
                }
                k += 1;
            }
        }
        for list in &mut partners {
            list.sort_unstable();
        }
        if let Some(c) = partners.iter().position(|p| p.len() != PARTNERS) {
            return Err(ModelError(format!("grip {c} interacts with {} grips, not {PARTNERS}", partners[c].len())));
        }
        Ok(Locality { r2, bits, partners, class, dots })
    }

    /// R², the squared circumradius of the solved body.
    pub fn r2(&self) -> &Q5 {
        &self.r2
    }

    /// The grip count.
    pub fn grips(&self) -> usize {
        self.partners.len()
    }

    /// The independence bits, in the layout of `c600_run::trace::Relation::bits`: one per pair (c, d) of grips with
    /// c < d, in ascending order of c and then of d, least significant bit first in each byte, set when the pair is
    /// independent.
    pub fn bits(&self) -> &[u8] {
        &self.bits
    }

    /// The grips that grip c interacts with, ascending.
    pub fn partners(&self, c: usize) -> &[u16] {
        &self.partners[c]
    }

    /// n_c·n_d and Δ(c, d) for distinct grips c and d; `None` when c = d or a grip is out of range.
    pub fn delta(&self, c: usize, d: usize) -> Option<(&Q5, &Q5)> {
        let n = self.grips();
        if c == d || c >= n || d >= n {
            return None;
        }
        let (c, d) = (c.min(d), c.max(d));
        let (dot, delta) = &self.dots[usize::from(self.class[pair_index(n, c, d)])];
        Some((dot, delta))
    }

    /// Whether distinct grips c and d are independent: Δ(c, d) ≤ 0. Letters at one grip are dependent, so c = d is
    /// never independent.
    pub fn independent(&self, c: usize, d: usize) -> bool {
        self.delta(c, d).is_some_and(|(_, delta)| delta.sign() <= 0)
    }

    /// The distinct values of n_c·n_d over pairs of distinct grips, ascending, each with its Δ.
    pub fn shells(&self) -> &[(Q5, Q5)] {
        &self.dots
    }
}
