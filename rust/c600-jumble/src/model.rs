//! The read-only model of the jumbling puzzle, built from `assets/model.npz` alone (PHS.18: nothing comes from J1).
//!
//! The files are read through the kernel's asset checks ([`c600_kernel::verify_assets`]): the manifest names
//! 600-cell-Full, its model ID is the SHA-256 of its file list, and every listed file has its listed digest. The
//! identity block's model ID therefore always names the files that were read.
//!
//! The 600 pole normals are stored as binary64 values. Each is read as its exact dyadic rational and matched to the
//! unique (p + q√5)/2 with |q| ≤ 12 within 10⁻⁹, decided by exact comparisons (selection 1). Every pole then has
//! |n|² = 12 + 4√5, which is checked.
//!
//! ν's labelling ([`crate::nu`]) is read from `execution_trees.json.gz`, `seed_atlas.json` and `census.json`. The model
//! keeps their bytes from the same asset check, so the labelling, built on first use, comes from the files that the
//! model ID names.
//!
//! Charter E3's startup (a draft): the pole rows and the integer arrays are read side by side
//! ([`c600_kernel::parallel`]), each taken where it was read one after another, so a load gives the same model and the
//! same first error.

use std::collections::HashMap;
use std::path::Path;

use c600_kernel::archive::{self, Zip};
use c600_kernel::parallel;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::q5::{dot, sign5, Vec4, Q5};

/// Pieces, cell centres included.
pub const NP: usize = 177_120;
/// Labelled sticker slots.
pub const NS: usize = 259_800;
/// Slots per cell; slot s lies on facet s / 433.
pub const CELL_SLOTS: usize = 433;
/// Poles (facets, caps and grips).
pub const POLES: usize = 600;
/// Bit words of a signature set over the 600 poles.
pub const SIG_WORDS: usize = 10;

/// The cut offset κ = α|n|² = (1452 + 484√5)/125 as (numerator a, numerator b, denominator).
pub const KAPPA_FORM: (i64, i64, i64) = (1452, 484, 125);

/// The files the model reads besides the manifest: the model archive, and the three that ν's labelling reads.
const NEEDED: [&str; 4] = ["model.npz", "execution_trees.json.gz", "seed_atlas.json", "census.json"];

/// The verified bytes of the files ν's labelling is read from ([`crate::nu`]).
pub(crate) struct LabellingFiles {
    pub(crate) trees_gz: Vec<u8>,
    pub(crate) atlas: Vec<u8>,
    pub(crate) census: Vec<u8>,
}

#[derive(Debug)]
pub struct ModelError(pub String);

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ModelError {}

fn fail(message: impl Into<String>) -> ModelError {
    ModelError(message.into())
}

pub struct Model {
    /// The manifest's model identity.
    pub model_id: String,
    /// SHA-256 of `assets/model.npz` and `assets/primitives.npz`, J1's model identity.
    pub identity: Vec<(String, String)>,
    /// Exact poles n_e.
    pub poles: Vec<Vec4>,
    /// Integer pole forms: n = (n1 + n2√5)/2.
    pub n1: Vec<[i64; 4]>,
    pub n2: Vec<[i64; 4]>,
    pole_index: HashMap<[i64; 8], u16>,
    /// |n|² = 12 + 4√5.
    pub nn: Q5,
    /// κ = (121/125)|n|².
    pub kappa: Q5,
    pub alpha: Q5,
    mask_offsets: Vec<u32>,
    mask_values: Vec<u16>,
    face_offsets: Vec<u32>,
    face_values: Vec<u16>,
    pub slot_piece: Vec<u32>,
    pub orbit_id: Vec<i16>,
    pub centre_pieces: Vec<u32>,
    pub rotperms: Vec<Vec<u16>>,
    pub frameperms: Vec<Vec<u16>>,
    cap_offsets: Vec<u32>,
    cap_piece: Vec<u32>,
    sigbits: Vec<[u64; SIG_WORDS]>,
    pub(crate) labelling: LabellingFiles,
}

/// The exact value of a binary64 bit pattern as num / 2^shift; `None` for infinities and NaNs.
fn dyadic(bits: u64) -> Option<(BigInt, u32)> {
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    if exponent == 0x7ff {
        return None;
    }
    let (mantissa, power) = if exponent == 0 { (fraction, -1074) } else { (fraction | (1u64 << 52), exponent - 1075) };
    let mut num = BigInt::from(mantissa);
    if bits >> 63 == 1 {
        num = -num;
    }
    if power >= 0 {
        Some((num << power as usize, 0))
    } else {
        Some((num, (-power) as u32))
    }
}

/// ⌊√n⌋ for a small non-negative n.
fn isqrt(n: i64) -> i64 {
    let mut r = 0;
    while (r + 1) * (r + 1) <= n {
        r += 1;
    }
    r
}

/// J1's `exact.from_float(x, max_q=12)`, decided exactly: the unique (p + q√5)/2 with |q| ≤ 12 and
/// |(p + q√5)/2 − x| < 10⁻⁹.
pub fn pole_coordinate(bits: u64) -> Result<Q5, ModelError> {
    let (num, shift) = dyadic(bits).ok_or_else(|| fail("a normal is not finite"))?;
    let den = BigInt::one() << shift as usize;
    let two = BigInt::from(2);
    let two_x_floor = (&num * &two).div_floor(&den);
    let billion = BigInt::from(1_000_000_000u64);
    let mut hits = Vec::new();
    for q in -12i64..=12 {
        let root = isqrt(5 * q * q);
        let q_root_floor = if q >= 0 { root } else { -root - 1 };
        let centre = &two_x_floor - BigInt::from(q_root_floor);
        for offset in -2i64..=2 {
            let p = &centre + BigInt::from(offset);
            // |(p + q√5)/2 − num/den| < 10⁻⁹  ⇔  −2·den < 10⁹(p·den − 2·num) + 10⁹·q·den·√5 < 2·den.
            let a = &billion * (&p * &den - &num * &two);
            let b = &billion * BigInt::from(q) * &den;
            let margin = &den * &two;
            if sign5(&(&a + &margin), &b) > 0 && sign5(&(&a - &margin), &b) < 0 {
                hits.push(Q5::new(p, BigInt::from(q), BigInt::from(2)));
            }
        }
    }
    match hits.len() {
        1 => Ok(hits.pop().unwrap()),
        _ => Err(fail("a normal is not a unique (p + q sqrt5)/2 value")),
    }
}

struct Arrays<'a> {
    zip: Zip<'a>,
}

const ARRAY_LIMIT: usize = 1 << 28;

impl Arrays<'_> {
    fn ints(&self, name: &str, shape: &[usize]) -> Result<Vec<i32>, ModelError> {
        let bytes = self.zip.read(&format!("{name}.npy"), ARRAY_LIMIT).map_err(|e| fail(format!("{name}: {e}")))?;
        let npy = archive::read_npy(&bytes).map_err(|e| fail(format!("{name}: {e}")))?;
        if !shape.is_empty() && npy.shape != shape {
            return Err(fail(format!("{name} has an unexpected shape")));
        }
        Ok(npy.values)
    }

    fn bits(&self, name: &str, shape: &[usize]) -> Result<Vec<u64>, ModelError> {
        let bytes = self.zip.read(&format!("{name}.npy"), ARRAY_LIMIT).map_err(|e| fail(format!("{name}: {e}")))?;
        let npy = archive::read_npy_raw(&bytes).map_err(|e| fail(format!("{name}: {e}")))?;
        if npy.descr != "<f8" || npy.shape != shape {
            return Err(fail(format!("{name} is not a binary64 array of the expected shape")));
        }
        Ok(npy.data.chunks_exact(8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).collect())
    }
}

fn to_u16(values: &[i32], below: i32, what: &str) -> Result<Vec<u16>, ModelError> {
    values
        .iter()
        .map(|&v| if (0..below).contains(&v) { Ok(v as u16) } else { Err(fail(format!("{what} is out of range"))) })
        .collect()
}

fn offsets(values: &[i32], total: usize, what: &str) -> Result<Vec<u32>, ModelError> {
    let out: Vec<u32> = values.iter().map(|&v| v.max(0) as u32).collect();
    if values.iter().any(|&v| v < 0)
        || out.windows(2).any(|w| w[0] > w[1])
        || out.first() != Some(&0)
        || out.last().map(|&v| v as usize) != Some(total)
    {
        return Err(fail(format!("{what} are not monotone offsets")));
    }
    Ok(out)
}

/// One pole's row of four binary64 coordinates: its exact coordinates and its two integer forms, twice the rational
/// and twice the √5 part of each coordinate.
fn pole_row(row: &[u64]) -> Result<(Vec4, [i64; 4], [i64; 4]), ModelError> {
    let mut pole: Vec<Q5> = Vec::with_capacity(4);
    let (mut a1, mut a2) = ([0i64; 4], [0i64; 4]);
    for (j, &bits) in row.iter().enumerate() {
        let x = pole_coordinate(bits)?;
        let scale = BigInt::from(2) / x.d();
        a1[j] = (x.a() * &scale).to_i64().ok_or_else(|| fail("pole form overflows"))?;
        a2[j] = (x.b() * &scale).to_i64().ok_or_else(|| fail("pole form overflows"))?;
        if !(BigInt::from(2) % x.d()).is_zero() {
            return Err(fail("a pole coordinate has a denominator other than 1 or 2"));
        }
        pole.push(x);
    }
    Ok(([pole[0].clone(), pole[1].clone(), pole[2].clone(), pole[3].clone()], a1, a2))
}

impl Model {
    pub fn load(assets: &Path) -> Result<Model, ModelError> {
        // The kernel's asset checks: a manifest naming 600-cell-Full whose model ID is the SHA-256 of its file list,
        // and every listed file with its listed digest. J1's model identity is the digests of the two archives.
        let mut verified = c600_kernel::verify_assets(assets, &NEEDED).map_err(|e| fail(e.0))?;
        let mut take =
            |name: &str| verified.files.remove(name).ok_or_else(|| fail(format!("manifest.json does not list {name}")));
        let labelling = LabellingFiles {
            trees_gz: take("execution_trees.json.gz")?,
            atlas: take("seed_atlas.json")?,
            census: take("census.json")?,
        };
        let listed = |name: &str| {
            verified.digests.get(name).cloned().ok_or_else(|| fail(format!("manifest.json does not list {name}")))
        };
        let (model_digest, primitives_digest) = (listed("assets/model.npz")?, listed("assets/primitives.npz")?);
        let model_id = verified.model_id.clone();
        let model_bytes =
            verified.files.get("model.npz").ok_or_else(|| fail("manifest.json does not list model.npz"))?;
        let zip = Zip::parse(model_bytes).map_err(|e| fail(format!("model.npz: {e}")))?;
        let arrays = Arrays { zip };

        let normals = arrays.bits("normals", &[POLES, 4])?;
        // The rows are read side by side and taken in order, so the first failing row is the answer.
        let rows = parallel::map(normals.len() / 4, |i| pole_row(&normals[4 * i..4 * i + 4]));
        let mut poles = Vec::with_capacity(POLES);
        let (mut n1, mut n2) = (Vec::with_capacity(POLES), Vec::with_capacity(POLES));
        for row in rows {
            let (pole, a1, a2) = parallel::taken(row)?;
            poles.push(pole);
            n1.push(a1);
            n2.push(a2);
        }
        let nn = Q5::of(12, 4, 1);
        let shared = parallel::map(poles.len(), |i| dot(&poles[i], &poles[i]) == nn);
        if shared.into_iter().map(parallel::taken).any(|shared| !shared) {
            return Err(fail("the poles do not share |n|^2 = 12 + 4 sqrt5"));
        }
        let mut pole_index = HashMap::with_capacity(POLES);
        for i in 0..POLES {
            let key = [n1[i][0], n1[i][1], n1[i][2], n1[i][3], n2[i][0], n2[i][1], n2[i][2], n2[i][3]];
            if pole_index.insert(key, i as u16).is_some() {
                return Err(fail("two poles coincide"));
            }
        }
        let alpha = Q5::of(121, 0, 125);
        let kappa = alpha.mul(&nn);
        if kappa != Q5::of(KAPPA_FORM.0, KAPPA_FORM.1, KAPPA_FORM.2) {
            return Err(fail("kappa is not (1452 + 484 sqrt5)/125"));
        }

        // The integer arrays are unpacked side by side, and each is taken where it was read before, in this order.
        const INTS: [(&str, &[usize]); 8] = [
            ("mask_offsets", &[NP + 1]),
            ("mask_values", &[]),
            ("face_offsets", &[NP + 1]),
            ("face_values", &[]),
            ("slot_piece", &[NS]),
            ("orbit_id", &[NP]),
            ("rotperms", &[1200, POLES]),
            ("frameperms", &[POLES, POLES]),
        ];
        let unpacked = parallel::map(INTS.len(), |i| arrays.ints(INTS[i].0, INTS[i].1));
        let mut unpacked = INTS.iter().zip(unpacked);
        let mut ints = |name: &str| {
            let ((listed, _), result) = unpacked.next().expect("an array for each name");
            debug_assert_eq!(*listed, name);
            parallel::taken(result)
        };
        let raw = ints("mask_offsets")?;
        let mask_offsets = offsets(&raw, *raw.last().unwrap_or(&0) as usize, "mask offsets")?;
        let mask_values = to_u16(&ints("mask_values")?, POLES as i32, "a mask value")?;
        if mask_values.len() != *mask_offsets.last().unwrap() as usize {
            return Err(fail("mask values disagree with their offsets"));
        }
        let raw = ints("face_offsets")?;
        let face_offsets = offsets(&raw, *raw.last().unwrap_or(&0) as usize, "face offsets")?;
        let face_values = to_u16(&ints("face_values")?, POLES as i32, "a face value")?;
        if face_values.len() != *face_offsets.last().unwrap() as usize {
            return Err(fail("face values disagree with their offsets"));
        }
        let slot_piece: Vec<u32> = ints("slot_piece")?
            .iter()
            .map(|&v| if (0..NP as i32).contains(&v) { Ok(v as u32) } else { Err(fail("slot piece out of range")) })
            .collect::<Result<_, _>>()?;
        let orbit_id: Vec<i16> = ints("orbit_id")?
            .iter()
            .map(|&v| i16::try_from(v).map_err(|_| fail("orbit id out of range")))
            .collect::<Result<_, _>>()?;
        let rotperms = to_u16(&ints("rotperms")?, POLES as i32, "a rotation permutation")?
            .chunks_exact(POLES)
            .map(<[u16]>::to_vec)
            .collect();
        let frameperms = to_u16(&ints("frameperms")?, POLES as i32, "a frame permutation")?
            .chunks_exact(POLES)
            .map(<[u16]>::to_vec)
            .collect();

        let centre_pieces: Vec<u32> = (0..NP as u32).filter(|&p| orbit_id[p as usize] == -1).collect();
        if centre_pieces.len() != POLES
            || centre_pieces.iter().any(|&p| mask_offsets[p as usize + 1] - mask_offsets[p as usize] != 1)
        {
            return Err(fail("the model does not have 600 centre pieces with one signature pole each"));
        }

        // Cap membership: the pieces whose signature contains c, each list ascending.
        let mut counts = vec![0u32; POLES + 1];
        for &c in &mask_values {
            counts[c as usize + 1] += 1;
        }
        for c in 0..POLES {
            counts[c + 1] += counts[c];
        }
        let cap_offsets = counts.clone();
        let mut fill = counts;
        let mut cap_piece = vec![0u32; mask_values.len()];
        let mut sigbits = vec![[0u64; SIG_WORDS]; NP];
        for p in 0..NP {
            for &c in &mask_values[mask_offsets[p] as usize..mask_offsets[p + 1] as usize] {
                cap_piece[fill[c as usize] as usize] = p as u32;
                fill[c as usize] += 1;
                sigbits[p][c as usize / 64] |= 1u64 << (c % 64);
            }
        }

        Ok(Model {
            model_id,
            identity: vec![
                ("assets/model.npz".to_owned(), model_digest),
                ("assets/primitives.npz".to_owned(), primitives_digest),
            ],
            poles,
            n1,
            n2,
            pole_index,
            nn,
            kappa,
            alpha,
            mask_offsets,
            mask_values,
            face_offsets,
            face_values,
            slot_piece,
            orbit_id,
            centre_pieces,
            rotperms,
            frameperms,
            cap_offsets,
            cap_piece,
            sigbits,
            labelling,
        })
    }

    /// The signature M(p): the poles whose caps contain piece p, ascending as stored.
    pub fn signature(&self, p: usize) -> &[u16] {
        &self.mask_values[self.mask_offsets[p] as usize..self.mask_offsets[p + 1] as usize]
    }

    /// The facets that carry a sticker of piece p.
    pub fn hosts(&self, p: usize) -> &[u16] {
        &self.face_values[self.face_offsets[p] as usize..self.face_offsets[p + 1] as usize]
    }

    /// The pieces whose signature contains c, ascending.
    pub fn cap_members(&self, c: usize) -> &[u32] {
        &self.cap_piece[self.cap_offsets[c] as usize..self.cap_offsets[c + 1] as usize]
    }

    pub fn in_sig(&self, p: usize, c: usize) -> bool {
        self.sigbits[p][c / 64] >> (c % 64) & 1 == 1
    }

    pub fn sig_bits(&self, p: usize) -> &[u64; SIG_WORDS] {
        &self.sigbits[p]
    }

    /// The pole index of an exact vector, if it is a pole.
    pub fn pole_of(&self, v: &[Q5]) -> Option<usize> {
        let two = BigInt::from(2);
        let mut key = [0i64; 8];
        for (j, x) in v.iter().enumerate() {
            if !(&two % x.d()).is_zero() {
                return None;
            }
            let scale = &two / x.d();
            key[j] = (x.a() * &scale).to_i64()?;
            key[4 + j] = (x.b() * &scale).to_i64()?;
        }
        self.pole_index.get(&key).map(|&i| i as usize)
    }

    /// h_e(x) = n_e·x/|n|² − α, the signed offset of x from the cut of pole e.
    pub fn h(&self, e: usize, x: &[Q5]) -> Q5 {
        dot(&self.poles[e], x).div(&self.nn).expect("|n|^2 is not zero").sub(&self.alpha)
    }

    /// Exact check that x lies in the closed posed region g(A₀) of a piece with signature `sig`: z = gᵀx satisfies
    /// all 600 cut constraints (inside for the poles of the signature, outside otherwise) and all 600 facets.
    pub fn posed_point_check(&self, sig: &[u16], g: &crate::q5::Mat4, x: &Vec4) -> bool {
        let z = crate::q5::matvec(&crate::q5::transpose(g), x);
        let form = crate::q5::int_form(&z);
        let e = &form.d;
        let mut inside = [false; POLES];
        for &c in sig {
            inside[c as usize] = true;
        }
        let (k0, k1, k2) = (BigInt::from(KAPPA_FORM.0), BigInt::from(KAPPA_FORM.1), BigInt::from(KAPPA_FORM.2));
        for q in 0..POLES {
            // n_q·z = (p1 + p2√5)/(2e)
            let mut p1 = BigInt::zero();
            let mut p2 = BigInt::zero();
            for j in 0..4 {
                let (a, b) = (&form.a[j], &form.b[j]);
                p1 += a * self.n1[q][j] + BigInt::from(5) * b * self.n2[q][j];
                p2 += b * self.n1[q][j] + a * self.n2[q][j];
            }
            if sign5(&(&p1 - e * 24), &(&p2 - e * 8)) > 0 {
                return false;
            }
            let s = sign5(&(&k2 * &p1 - e * 2 * &k0), &(&k2 * &p2 - e * 2 * &k1));
            if (inside[q] && s < 0) || (!inside[q] && s > 0) {
                return false;
            }
        }
        true
    }
}

/// Least common multiple helper for integer forms.
pub fn lcm(a: &BigInt, b: &BigInt) -> BigInt {
    a.lcm(b)
}

/// |x| of a big integer as a u128 when it fits.
pub fn small_abs(x: &BigInt) -> Option<u128> {
    if x.is_negative() {
        (-x).to_u128()
    } else {
        x.to_u128()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_coordinates_from_binary64() {
        // 0.5 + 0.5√5 = φ: the nearest binary64 value of φ maps back to (1 + √5)/2.
        let phi_bits = 0x3FF9E3779B97F4A8u64;
        assert_eq!(pole_coordinate(phi_bits).unwrap(), Q5::of(1, 1, 2));
        assert_eq!(pole_coordinate(0x4000000000000000).unwrap(), Q5::int(2)); // 2.0
        assert_eq!(pole_coordinate(0).unwrap(), Q5::zero());
        assert_eq!(pole_coordinate(0x8000000000000000).unwrap(), Q5::zero()); // -0.0
                                                                              // 0.1 is no (p + q√5)/2 within 10⁻⁹.
        assert!(pole_coordinate(0x3FB999999999999A).is_err());
        assert!(pole_coordinate(0x7FF0000000000000).is_err());
    }
}
