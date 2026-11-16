//! Exact piece regions: one double description per K⁺-orbit representative, transported exactly to every piece.
//!
//! The home region of piece A is the closure of the polytope cut by the caps of its signature M(A) and by the
//! complements of the other caps (state contract, section 2). A rotation r in K⁺ is a symmetry of the pole set, so
//! r(A₀(M)) is the chamber of signature r(M): one exact region per orbit gives every region by transport. The
//! representatives lie in cap 0, whose region bounds the candidate constraints; dropped constraints are implied
//! there, and the cap region itself is certified against all 600 facets.

use std::collections::HashMap;
use std::sync::OnceLock;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, ToPrimitive};

use crate::dd::{double_description, Constraint};
use crate::kplus::{IntMat, KPlus, ORDER};
use crate::model::{Model, ModelError, NP, POLES};
use crate::q5::{affine_rank, dot, matvec, Vec4, Q5};

fn fail(message: impl Into<String>) -> ModelError {
    ModelError(message.into())
}

/// The integer form of a vertex list: row j is (x₀..x₃, z₀..z₃), meaning vertex j = (x + z√5)/E.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VForm {
    pub e: i64,
    pub rows: Vec<[i64; 8]>,
}

/// The integer form of exact points with one common denominator; `None` if an entry leaves i64.
pub fn rows_form(points: &[Vec<Q5>]) -> Option<VForm> {
    let mut e = BigInt::one();
    for p in points {
        for x in p {
            e = e.lcm(x.d());
        }
    }
    let mut rows = Vec::with_capacity(points.len());
    for p in points {
        let mut row = [0i64; 8];
        for (j, x) in p.iter().enumerate() {
            let scale = &e / x.d();
            row[j] = (x.a() * &scale).to_i64()?;
            row[4 + j] = (x.b() * &scale).to_i64()?;
        }
        rows.push(row);
    }
    Some(VForm { e: e.to_i64()?, rows })
}

/// The integer rows of r·v for a K⁺ matrix r and an integer vertex form, with overflow checks.
pub fn transport_rows(r: &IntMat, v: &VForm) -> VForm {
    let rows = v
        .rows
        .iter()
        .map(|row| {
            let (x1, x2) = (&row[..4], &row[4..]);
            let mut out = [0i64; 8];
            for i in 0..4 {
                let mut y1: i128 = 0;
                let mut y2: i128 = 0;
                for j in 0..4 {
                    y1 += x1[j] as i128 * r.g1[i][j] as i128 + 5 * x2[j] as i128 * r.g2[i][j] as i128;
                    y2 += x2[j] as i128 * r.g1[i][j] as i128 + x1[j] as i128 * r.g2[i][j] as i128;
                }
                out[i] = i64::try_from(y1).expect("transported vertex fits in i64");
                out[4 + i] = i64::try_from(y2).expect("transported vertex fits in i64");
            }
            out
        })
        .collect();
    VForm { e: v.e.checked_mul(r.d).expect("transported denominator fits in i64"), rows }
}

pub struct Regions {
    pub reps: Vec<u32>,
    /// The K⁺-orbit of each piece.
    pub orbit_of: Vec<u16>,
    /// The smallest K⁺ index r with r(rep) = piece.
    pub transport: Vec<u16>,
    pub rep_vertices: Vec<Vec<Vec4>>,
    pub rep_vform: Vec<VForm>,
    pub cap_vertices: Vec<Vec4>,
    cap_vform0: VForm,
    cap_vforms: Vec<OnceLock<VForm>>,
    pub report: Vec<(String, String)>,
}

/// Poles whose normals make an angle with n_c of cosine above 3/5: c and the 80 poles nearest to it.
pub fn near_poles(model: &Model, c: usize) -> Vec<usize> {
    let threshold = Q5::of(3, 0, 5).mul(&model.nn);
    (0..POLES).filter(|&e| dot(&model.poles[e], &model.poles[c]) > threshold).collect()
}

fn as_vec4(v: &[Q5]) -> Vec4 {
    [v[0].clone(), v[1].clone(), v[2].clone(), v[3].clone()]
}

impl Regions {
    pub fn build(model: &Model, kp: &KPlus) -> Result<Regions, ModelError> {
        // Orbits: representatives are the first unassigned pieces of cap 0, transports the smallest K⁺ index.
        let mut sig_index: HashMap<&[u16], u32> = HashMap::with_capacity(NP);
        for p in 0..NP {
            if sig_index.insert(model.signature(p), p as u32).is_some() {
                return Err(fail("signatures are not distinct"));
            }
        }
        const UNSET: u16 = u16::MAX;
        let mut orbit_of = vec![UNSET; NP];
        let mut transport = vec![UNSET; NP];
        let mut reps = Vec::new();
        let mut image = Vec::new();
        for &p in model.cap_members(0) {
            if orbit_of[p as usize] != UNSET {
                continue;
            }
            let o = reps.len() as u16;
            let sig = model.signature(p as usize);
            for (k, perm) in kp.perms.iter().enumerate() {
                image.clear();
                image.extend(sig.iter().map(|&c| perm[c as usize]));
                image.sort_unstable();
                let q = *sig_index
                    .get(image.as_slice())
                    .ok_or_else(|| fail("the signature set is not closed under K+"))? as usize;
                if orbit_of[q] == UNSET {
                    orbit_of[q] = o;
                    transport[q] = k as u16;
                } else if orbit_of[q] != o {
                    return Err(fail("K+-orbits overlap"));
                }
            }
            reps.push(p);
        }
        if orbit_of.contains(&UNSET) {
            return Err(fail("some K+-orbit does not meet cap 0"));
        }
        debug_assert!(kp.perms.len() == ORDER);

        // The cap region of pole 0, certified against every facet.
        let near = near_poles(model, 0);
        let mut cons = vec![Constraint { a: model.poles[0].iter().map(Q5::neg).collect(), b: model.kappa.neg() }];
        cons.extend(near.iter().map(|&e| Constraint { a: model.poles[e].to_vec(), b: model.nn.clone() }));
        let cap = double_description(&cons, 4).map_err(|e| fail(format!("cap region: {}", e.0)))?;
        for v in &cap {
            if (0..POLES).any(|e| dot(&model.poles[e], v).sub(&model.nn).sign() > 0) {
                return Err(fail("the cap region leaves the polytope"));
            }
        }
        let implied: Vec<bool> =
            (0..POLES).map(|e| cap.iter().all(|v| dot(&model.poles[e], v).sub(&model.kappa).sign() <= 0)).collect();
        let candp: Vec<usize> = (0..POLES).filter(|&e| !implied[e] && e != 0).collect();
        let cap_vform0 = rows_form(&cap).ok_or_else(|| fail("cap vertices leave i64"))?;

        let mut rep_vertices = Vec::with_capacity(reps.len());
        let mut rep_vform = Vec::with_capacity(reps.len());
        let mut vertex_counts = Vec::with_capacity(reps.len());
        for &p in &reps {
            let sig = model.signature(p as usize);
            let mut sorted_sig = sig.to_vec();
            sorted_sig.sort_unstable();
            let mut cons: Vec<Constraint> = sorted_sig
                .iter()
                .map(|&e| Constraint { a: model.poles[e as usize].iter().map(Q5::neg).collect(), b: model.kappa.neg() })
                .collect();
            cons.extend(
                candp
                    .iter()
                    .filter(|&&e| !sig.contains(&(e as u16)))
                    .map(|&e| Constraint { a: model.poles[e].to_vec(), b: model.kappa.clone() }),
            );
            cons.extend(near.iter().map(|&e| Constraint { a: model.poles[e].to_vec(), b: model.nn.clone() }));
            let verts = double_description(&cons, 4).map_err(|e| fail(format!("piece {p}: {}", e.0)))?;
            if affine_rank(&verts) != 4 {
                return Err(fail(format!("the region of piece {p} is not full-dimensional")));
            }
            let mut three = Vec::new();
            for &e in &near {
                let on: Vec<Vec<Q5>> =
                    verts.iter().filter(|v| dot(&model.poles[e], v).sub(&model.nn).is_zero()).cloned().collect();
                if !on.is_empty() && affine_rank(&on) == 3 {
                    three.push(e as u16);
                }
            }
            if three != model.hosts(p as usize) {
                return Err(fail(format!("the facet patches of piece {p} differ from its stickers")));
            }
            rep_vform.push(rows_form(&verts).ok_or_else(|| fail("piece vertices leave i64"))?);
            vertex_counts.push(verts.len());
            rep_vertices.push(verts.iter().map(|v| as_vec4(v)).collect());
        }
        // A far facet cannot carry a 3-dimensional patch of any piece region inside cap 0.
        let mut far_rank = 0;
        for e in 0..POLES {
            if near.contains(&e) {
                continue;
            }
            let on: Vec<Vec<Q5>> =
                cap.iter().filter(|v| dot(&model.poles[e], v).sub(&model.nn).is_zero()).cloned().collect();
            if !on.is_empty() {
                far_rank = far_rank.max(affine_rank(&on));
            }
        }
        if far_rank >= 3 {
            return Err(fail("a far facet carries a 3-dimensional patch inside cap 0"));
        }
        let report = vec![
            ("kplus_orbits".to_owned(), reps.len().to_string()),
            ("cap_region_vertices".to_owned(), cap.len().to_string()),
            ("cap_candidate_poles".to_owned(), candp.len().to_string()),
            ("cap_near_facets".to_owned(), near.len().to_string()),
            ("rep_vertex_counts".to_owned(), format!("{vertex_counts:?}")),
        ];
        Ok(Regions {
            reps,
            orbit_of,
            transport,
            rep_vertices,
            rep_vform,
            cap_vertices: cap.iter().map(|v| as_vec4(v)).collect(),
            cap_vform0,
            cap_vforms: (0..POLES).map(|_| OnceLock::new()).collect(),
            report,
        })
    }

    /// The orbit and transport of piece p.
    pub fn rep_of(&self, p: usize) -> (usize, u16) {
        (self.orbit_of[p] as usize, self.transport[p])
    }

    /// The exact home vertices of piece p.
    pub fn vertices(&self, p: usize, kp: &KPlus, model: &Model) -> Vec<Vec4> {
        let (o, k) = self.rep_of(p);
        let m = kp.matrix(k, model);
        self.rep_vertices[o].iter().map(|v| matvec(m, v)).collect()
    }

    /// The integer form of the home vertices of piece p.
    pub fn vform(&self, p: usize, kp: &KPlus, model: &Model) -> VForm {
        let (o, k) = self.rep_of(p);
        transport_rows(kp.int_matrix(k, model), &self.rep_vform[o])
    }

    /// The integer form of the vertices of the cap region of pole a (the frame of a applied to cap 0's).
    pub fn cap_vform(&self, a: usize, kp: &KPlus, model: &Model) -> &VForm {
        self.cap_vforms[a].get_or_init(|| transport_rows(kp.int_matrix(kp.frame_idx[a], model), &self.cap_vform0))
    }
}
