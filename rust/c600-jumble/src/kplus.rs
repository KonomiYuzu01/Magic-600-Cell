//! The rotation group K⁺ of the 600-cell (order 7200), exactly, built from the stored rotation permutations.
//!
//! An element is its pole permutation (perm[x] = image of pole x). Elements are numbered in breadth-first order from
//! the identity over the generators 0 to 3, so the numbering does not depend on the basis poles that look elements
//! up. An element's exact matrix is the linear map sending four independent basis poles to their images.

use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, ToPrimitive};

use crate::model::{Model, ModelError, POLES};
use crate::q5::{dot, identity4, matmul, rank, solve, Mat4, Q5};

pub const ORDER: usize = 7200;

fn fail(message: &str) -> ModelError {
    ModelError(message.to_owned())
}

/// An exact K⁺ matrix as (G1 + G2√5)/D with small integers.
#[derive(Clone, Debug)]
pub struct IntMat {
    pub g1: [[i64; 4]; 4],
    pub g2: [[i64; 4]; 4],
    pub d: i64,
}

pub struct KPlus {
    pub basis: [usize; 4],
    pub perms: Vec<Vec<u16>>,
    sorted_keys: Vec<(u64, u16)>,
    pub inv: Vec<u16>,
    pub frame_idx: Vec<u16>,
    pub gen_idx: Vec<u16>,
    /// The 12 elements fixing pole 0, in increasing index order.
    pub stab0: Vec<u16>,
    binv: Mat4,
    matrices: Vec<OnceLock<Mat4>>,
    int_matrices: Vec<OnceLock<IntMat>>,
}

fn key_of(images: [u16; 4]) -> u64 {
    ((images[0] as u64 * 600 + images[1] as u64) * 600 + images[2] as u64) * 600 + images[3] as u64
}

impl KPlus {
    pub fn build(model: &Model) -> Result<KPlus, ModelError> {
        // Basis: pole 0 and the lowest-numbered poles of its face-neighbour class that keep the set independent.
        let nn = &model.nn;
        let mut best: Option<Q5> = None;
        for e in 1..POLES {
            let v = dot(&model.poles[e], &model.poles[0]);
            if &v != nn && best.as_ref().is_none_or(|b| v > *b) {
                best = Some(v);
            }
        }
        let best = best.ok_or_else(|| fail("pole 0 has no neighbours"))?;
        let mut basis = vec![0usize];
        for e in 1..POLES {
            if basis.len() == 4 {
                break;
            }
            if dot(&model.poles[e], &model.poles[0]) != best {
                continue;
            }
            let mut rows: Vec<Vec<Q5>> = basis.iter().map(|&b| model.poles[b].to_vec()).collect();
            rows.push(model.poles[e].to_vec());
            if rank(&rows) == rows.len() {
                basis.push(e);
            }
        }
        if basis.len() != 4 {
            return Err(fail("no independent basis of pole 0 and its face neighbours"));
        }
        let basis = [basis[0], basis[1], basis[2], basis[3]];
        let images = |perm: &[u16]| [perm[basis[0]], perm[basis[1]], perm[basis[2]], perm[basis[3]]];

        let gens: Vec<&[u16]> = (0..4).map(|k| model.rotperms[k].as_slice()).collect();
        let identity: Vec<u16> = (0..POLES as u16).collect();
        let mut seen: HashMap<u64, usize> = HashMap::new();
        seen.insert(key_of(images(&identity)), 0);
        let mut perms = vec![identity];
        let mut queue = VecDeque::from([0usize]);
        while let Some(i) = queue.pop_front() {
            for g in &gens {
                let q: Vec<u16> = perms[i].iter().map(|&x| g[x as usize]).collect();
                let k = key_of(images(&q));
                if let std::collections::hash_map::Entry::Vacant(slot) = seen.entry(k) {
                    slot.insert(perms.len());
                    perms.push(q);
                    queue.push_back(perms.len() - 1);
                }
                if perms.len() > ORDER {
                    return Err(fail("K+ closure exceeds order 7200"));
                }
            }
        }
        if perms.len() != ORDER {
            return Err(fail("K+ closure does not have order 7200"));
        }
        let mut sorted_keys: Vec<(u64, u16)> =
            perms.iter().enumerate().map(|(i, p)| (key_of(images(p)), i as u16)).collect();
        sorted_keys.sort_unstable();
        let mut kp = KPlus {
            basis,
            perms,
            sorted_keys,
            inv: Vec::new(),
            frame_idx: Vec::new(),
            gen_idx: Vec::new(),
            stab0: Vec::new(),
            binv: identity4(),
            matrices: (0..ORDER).map(|_| OnceLock::new()).collect(),
            int_matrices: (0..ORDER).map(|_| OnceLock::new()).collect(),
        };
        let mut inv = Vec::with_capacity(ORDER);
        for p in &kp.perms {
            let mut q = vec![0u16; POLES];
            for (x, &y) in p.iter().enumerate() {
                q[y as usize] = x as u16;
            }
            inv.push(kp.index_of_perm_images(images(&q)).ok_or_else(|| fail("K+ is not closed under inverses"))?);
        }
        kp.inv = inv;
        kp.frame_idx = model
            .frameperms
            .iter()
            .map(|f| kp.index_of_perm_images(images(f)).ok_or_else(|| fail("a retained frame is not in K+")))
            .collect::<Result<_, _>>()?;
        kp.gen_idx = model
            .rotperms
            .iter()
            .map(|g| kp.index_of_perm_images(images(g)).ok_or_else(|| fail("a retained generator is not in K+")))
            .collect::<Result<_, _>>()?;
        kp.stab0 = (0..ORDER as u16).filter(|&i| kp.perms[i as usize][0] == 0).collect();
        if kp.stab0.len() != 12 {
            return Err(fail("the stabiliser of pole 0 does not have 12 elements"));
        }
        let b: Vec<Vec<Q5>> = (0..4).map(|r| basis.iter().map(|&p| model.poles[p][r].clone()).collect()).collect();
        let ident: Vec<Vec<Q5>> =
            (0..4).map(|r| (0..4).map(|c| if r == c { Q5::one() } else { Q5::zero() }).collect()).collect();
        let binv = solve(&b, &ident).ok_or_else(|| fail("the basis poles are dependent"))?;
        kp.binv = std::array::from_fn(|r| std::array::from_fn(|c| binv[r][c].clone()));
        Ok(kp)
    }

    fn index_of_perm_images(&self, images: [u16; 4]) -> Option<u16> {
        let key = key_of(images);
        self.sorted_keys.binary_search_by_key(&key, |&(k, _)| k).ok().map(|pos| self.sorted_keys[pos].1)
    }

    /// The index of the element with the given basis images, if any.
    pub fn index_of_images(&self, images: [u16; 4]) -> Option<u16> {
        self.index_of_perm_images(images)
    }

    fn basis_images(&self, perm: &[u16]) -> [u16; 4] {
        [perm[self.basis[0]], perm[self.basis[1]], perm[self.basis[2]], perm[self.basis[3]]]
    }

    /// The index of (element i) ∘ (element j): j acts first.
    pub fn compose(&self, i: u16, j: u16) -> u16 {
        let (pi, pj) = (&self.perms[i as usize], &self.perms[j as usize]);
        let images = self.basis_images(pj).map(|x| pi[x as usize]);
        self.index_of_perm_images(images).expect("K+ is closed under composition")
    }

    /// The exact matrix of element i.
    pub fn matrix(&self, i: u16, model: &Model) -> &Mat4 {
        self.matrices[i as usize].get_or_init(|| {
            let perm = &self.perms[i as usize];
            let t: Mat4 =
                std::array::from_fn(|r| std::array::from_fn(|b| model.poles[perm[self.basis[b]] as usize][r].clone()));
            matmul(&t, &self.binv)
        })
    }

    /// The integer form (G1 + G2√5)/D of element i's matrix.
    pub fn int_matrix(&self, i: u16, model: &Model) -> &IntMat {
        self.int_matrices[i as usize].get_or_init(|| {
            let m = self.matrix(i, model);
            let mut d = BigInt::one();
            for row in m {
                for x in row {
                    d = d.lcm(x.d());
                }
            }
            let small = |x: BigInt| x.to_i64().expect("K+ matrix entries are small");
            let g1 = std::array::from_fn(|r| std::array::from_fn(|c| small(m[r][c].a() * (&d / m[r][c].d()))));
            let g2 = std::array::from_fn(|r| std::array::from_fn(|c| small(m[r][c].b() * (&d / m[r][c].d()))));
            IntMat { g1, g2, d: small(d) }
        })
    }

    /// The K⁺ index of an exact matrix, or `None` when it is not in K⁺ (exact test).
    pub fn index_of_matrix(&self, m: &Mat4, model: &Model) -> Option<u16> {
        let mut images = [0u16; 4];
        for (slot, &b) in self.basis.iter().enumerate() {
            let v: Vec<Q5> = (0..4).map(|r| dot(&m[r], &model.poles[b])).collect();
            images[slot] = model.pole_of(&v)? as u16;
        }
        let i = self.index_of_perm_images(images)?;
        (self.matrix(i, model) == m).then_some(i)
    }

    /// The 12 elements fixing pole c, conjugated by the frame of c from the stabiliser of pole 0 in its order.
    pub fn a4(&self, c: usize) -> Vec<u16> {
        let f = self.frame_idx[c];
        let finv = self.inv[f as usize];
        self.stab0.iter().map(|&s| self.compose(f, self.compose(s, finv))).collect()
    }

    /// The shortest word in the retained generators H_c, T_c, T_c⁻¹ for element k of A4_c, as signed 1-based
    /// primitive ids, found breadth first with the moves in that order.
    pub fn a4_word(&self, c: usize, k: u16) -> Option<Vec<i64>> {
        let h = self.gen_idx[2 * c];
        let t = self.gen_idx[2 * c + 1];
        let moves = [(h, 2 * c as i64 + 1), (t, 2 * c as i64 + 2), (self.inv[t as usize], -(2 * c as i64 + 2))];
        let mut words: HashMap<u16, Vec<i64>> = HashMap::from([(0, Vec::new())]);
        let mut queue = VecDeque::from([0u16]);
        while let Some(x) = queue.pop_front() {
            for &(g, mid) in &moves {
                let y = self.compose(g, x);
                if !words.contains_key(&y) {
                    let mut word = words[&x].clone();
                    word.push(mid);
                    words.insert(y, word);
                    queue.push_back(y);
                }
            }
        }
        words.remove(&k)
    }

    /// The exact matrix of a word of signed 1-based primitive ids, applied left to right.
    pub fn word_matrix(&self, word: &[i64], model: &Model) -> Option<Mat4> {
        let mut m = identity4();
        for &mid in word {
            let g = usize::try_from(mid.unsigned_abs()).ok()?.checked_sub(1)?;
            let mut k = *self.gen_idx.get(g)?;
            if mid < 0 {
                k = self.inv[k as usize];
            }
            m = matmul(self.matrix(k, model), &m);
        }
        Some(m)
    }
}
