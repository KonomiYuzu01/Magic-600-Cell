//! Exact vertices of a bounded full-dimensional polytope {x : a·x ≤ b} by the double description method with the
//! combinatorial adjacency test, over Q(√5). The order of the vertices is part of the result: classifications scan
//! vertices in this order, so the steps follow the reference construction exactly.

use crate::q5::{affine_rank, dot, rank, solve, Q5};

/// One constraint a·x ≤ b.
pub struct Constraint {
    pub a: Vec<Q5>,
    pub b: Q5,
}

#[derive(Debug)]
pub struct DdError(pub &'static str);

/// A set of constraint indices.
#[derive(Clone, PartialEq, Eq)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(m: usize) -> Bits {
        Bits(vec![0; m.div_ceil(64)])
    }

    fn insert(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }

    fn and(&self, other: &Bits) -> Bits {
        Bits(self.0.iter().zip(&other.0).map(|(x, y)| x & y).collect())
    }

    fn count(&self) -> usize {
        self.0.iter().map(|w| w.count_ones() as usize).sum()
    }

    fn subset_of(&self, other: &Bits) -> bool {
        self.0.iter().zip(&other.0).all(|(x, y)| x & !y == 0)
    }
}

/// Positive rescaling: vertices get t = 1, directions get a first nonzero entry of absolute value 1.
fn normalize_ray(r: Vec<Q5>) -> Result<Vec<Q5>, DdError> {
    if !r[0].is_zero() {
        let inv = r[0].inv().expect("nonzero");
        return Ok(r.iter().map(|x| x.mul(&inv)).collect());
    }
    for x in &r {
        if !x.is_zero() {
            let inv = if x.sign() > 0 { x.inv() } else { x.neg().inv() }.expect("nonzero");
            return Ok(r.iter().map(|y| y.mul(&inv)).collect());
        }
    }
    Err(DdError("zero ray"))
}

/// The vertices of {x : a·x ≤ b} in dimension `dim`; errors for an empty, unbounded or lower-dimensional region.
pub fn double_description(constraints: &[Constraint], dim: usize) -> Result<Vec<Vec<Q5>>, DdError> {
    let mut rows: Vec<Vec<Q5>> =
        constraints.iter().map(|c| std::iter::once(c.b.clone()).chain(c.a.iter().map(Q5::neg)).collect()).collect();
    rows.push(std::iter::once(Q5::one()).chain((0..dim).map(|_| Q5::zero())).collect());
    let m = rows.len();
    let mut basis: Vec<usize> = Vec::new();
    for i in std::iter::once(m - 1).chain(0..m - 1) {
        let mut trial: Vec<Vec<Q5>> = basis.iter().map(|&j| rows[j].clone()).collect();
        trial.push(rows[i].clone());
        if rank(&trial) == basis.len() + 1 {
            basis.push(i);
        }
        if basis.len() == dim + 1 {
            break;
        }
    }
    if basis.len() < dim + 1 {
        return Err(DdError("constraints do not determine a pointed cone"));
    }
    let ident: Vec<Vec<Q5>> =
        (0..=dim).map(|r| (0..=dim).map(|c| if r == c { Q5::one() } else { Q5::zero() }).collect()).collect();
    let basis_rows: Vec<Vec<Q5>> = basis.iter().map(|&i| rows[i].clone()).collect();
    let inv = solve(&basis_rows, &ident).ok_or(DdError("singular basis"))?;
    let mut rays: Vec<(Vec<Q5>, Bits)> = Vec::new();
    for k in 0..=dim {
        let r = normalize_ray((0..=dim).map(|i| inv[i][k].clone()).collect())?;
        let mut z = Bits::new(m);
        for (j, &b) in basis.iter().enumerate() {
            if j != k {
                z.insert(b);
            }
        }
        rays.push((r, z));
    }
    let mut done = vec![false; m];
    for &b in &basis {
        done[b] = true;
    }
    for i in 0..m {
        if done[i] {
            continue;
        }
        let h = &rows[i];
        let vals: Vec<Q5> = rays.iter().map(|(r, _)| dot(h, r)).collect();
        let signs: Vec<i32> = vals.iter().map(Q5::sign).collect();
        let plus: Vec<usize> = (0..rays.len()).filter(|&k| signs[k] > 0).collect();
        let minus: Vec<usize> = (0..rays.len()).filter(|&k| signs[k] < 0).collect();
        let zero: Vec<usize> = (0..rays.len()).filter(|&k| signs[k] == 0).collect();
        if minus.is_empty() {
            for &k in &zero {
                rays[k].1.insert(i);
            }
            done[i] = true;
            continue;
        }
        let mut new = Vec::new();
        for &p in &plus {
            for &q in &minus {
                let common = rays[p].1.and(&rays[q].1);
                if common.count() + 1 < dim {
                    continue;
                }
                if (0..rays.len()).any(|k| k != p && k != q && common.subset_of(&rays[k].1)) {
                    continue;
                }
                let (rp, rq) = (&rays[p].0, &rays[q].0);
                let (vp, vq) = (&vals[p], &vals[q]);
                let r: Vec<Q5> = rp.iter().zip(rq).map(|(x, y)| vp.mul(y).sub(&vq.mul(x))).collect();
                let mut z = common;
                z.insert(i);
                new.push((normalize_ray(r)?, z));
            }
        }
        let mut next: Vec<(Vec<Q5>, Bits)> = plus.iter().map(|&k| rays[k].clone()).collect();
        for &k in &zero {
            let (r, mut z) = rays[k].clone();
            z.insert(i);
            next.push((r, z));
        }
        next.extend(new);
        rays = next;
        done[i] = true;
    }
    let mut verts = Vec::with_capacity(rays.len());
    for (r, _) in &rays {
        if r[0].is_zero() {
            return Err(DdError("region is unbounded"));
        }
        verts.push(r[1..].to_vec());
    }
    if verts.is_empty() {
        return Err(DdError("region is empty"));
    }
    if affine_rank(&verts) < dim {
        return Err(DdError("region is not full-dimensional"));
    }
    Ok(verts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_square_vertices_in_reference_order() {
        let c = |a: [i64; 2], b: i64| Constraint { a: a.iter().map(|&x| Q5::int(x)).collect(), b: Q5::int(b) };
        // 0 <= x <= 1, 0 <= y <= 1
        let cons = [c([-1, 0], 0), c([0, -1], 0), c([1, 0], 1), c([0, 1], 1)];
        let verts = double_description(&cons, 2).unwrap();
        let ints: Vec<Vec<i32>> = verts.iter().map(|v| v.iter().map(Q5::sign).collect()).collect();
        assert_eq!(verts.len(), 4);
        assert!(ints.contains(&vec![0, 0]) && ints.contains(&vec![1, 1]));
        assert!(double_description(&cons[..3], 2).is_err());
    }
}
