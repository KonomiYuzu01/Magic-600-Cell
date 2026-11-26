//! The domain contact graph of a jumbled state (ADR-D02.21; step B15 of the ADR-D02 build). Two alignment domains
//! touch when a seam face one of them holds on the upper side of a world hyperplane and a seam face the other holds on
//! the lower side of the same hyperplane share a part of positive 3-volume; contacts on off-grip cuts count. The seam
//! cells, their domains and their hyperplanes are exact ([`observe::seam_cells`]); only the overlap test runs in
//! binary64, with a tolerance far above rounding and far below a real overlap, so the graph is display data: it
//! colours the contact-class tint and decides nothing exact. The method is ADR-D02 A4's, with a separating-axis test
//! in place of A4's linear program.

use std::collections::{BTreeMap, HashMap};

use c600_jumble::faces::{self, Side};
use c600_jumble::observe;
use c600_jumble::q5::Mat4;
use c600_jumble::state::State;
use c600_jumble::Context as Engine;

use super::approximate64;

/// One edge of the graph: domains `a < b` (indices into [`ContactGraph::keys`]) and whether they touch on an on-grip
/// cut, an off-grip cut or both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contact {
    pub a: u32,
    pub b: u32,
    pub on_grip: bool,
    pub off_grip: bool,
}

/// The contact graph of one state.
#[derive(Clone, Debug)]
pub struct ContactGraph {
    /// The domains' JI.53 keys: the lattice domain first, the others in key order.
    pub keys: Vec<String>,
    /// The edges in (a, b) order.
    pub edges: Vec<Contact>,
    /// Mismatched cut cells (the rim's N).
    pub seam_faces: u64,
    /// World hyperplanes that carry a seam face.
    pub planes: u64,
    /// Face pairs that passed the box test and were tested for overlap.
    pub tests: u64,
}

type P3 = [f64; 3];
type F4 = [[f64; 4]; 4];

/// A seam face placed in its hyperplane's 3D frame: its vertices, the unit normals of its 2-faces, its unit edge
/// directions and its box.
struct Face {
    domain: u32,
    pts: Vec<P3>,
    normals: Vec<P3>,
    edges: Vec<P3>,
    lo: P3,
    hi: P3,
}

/// The relative tolerance: two faces touch only when every separating-axis overlap exceeds it times their size.
const TOLERANCE: f64 = 1e-7;

fn float4(m: &Mat4) -> Result<F4, String> {
    let mut out = [[0.0; 4]; 4];
    for (r, row) in m.iter().enumerate() {
        for (c, x) in row.iter().enumerate() {
            out[r][c] = approximate64(x).ok_or("a matrix entry has no finite value")?;
        }
    }
    Ok(out)
}

fn apply(m: &F4, v: &[f64; 4]) -> [f64; 4] {
    let mut out = [0.0; 4];
    for (r, row) in m.iter().enumerate() {
        out[r] = row[0] * v[0] + row[1] * v[1] + row[2] * v[2] + row[3] * v[3];
    }
    out
}

fn dot4(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

fn sub(a: &P3, b: &P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: &P3, b: &P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: &P3, b: &P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: P3) -> Option<P3> {
    let n = dot(&a, &a).sqrt();
    (n > 1e-12).then(|| [a[0] / n, a[1] / n, a[2] / n])
}

/// An orthonormal basis of the hyperplane orthogonal to `u` (a fixed function of `u`): Gram-Schmidt over `u` and
/// the coordinate axes, keeping the first three residuals that are not small.
fn basis(u: &[f64; 4]) -> [[f64; 4]; 3] {
    let n = dot4(u, u).sqrt();
    let mut kept = vec![[u[0] / n, u[1] / n, u[2] / n, u[3] / n]];
    for axis in 0..4 {
        let mut v = [0.0; 4];
        v[axis] = 1.0;
        for k in &kept {
            let p = dot4(&v, k);
            for (x, y) in v.iter_mut().zip(k) {
                *x -= p * y;
            }
        }
        let len = dot4(&v, &v).sqrt();
        if len > 0.3 && kept.len() < 4 {
            kept.push([v[0] / len, v[1] / len, v[2] / len, v[3] / len]);
        }
    }
    [kept[1], kept[2], kept[3]]
}

fn interval(pts: &[P3], n: &P3) -> (f64, f64) {
    pts.iter().map(|p| dot(p, n)).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| (lo.min(x), hi.max(x)))
}

fn apart(a: &Face, b: &Face, n: &P3, tol: f64) -> bool {
    let (a0, a1) = interval(&a.pts, n);
    let (b0, b1) = interval(&b.pts, n);
    a1 - b0 <= tol || b1 - a0 <= tol
}

/// Whether two convex 3-polytopes share a part of positive volume: no separating axis among the box axes, the
/// 2-face normals of either and the cross products of their edge directions leaves an overlap at or below `tol`.
fn overlap(a: &Face, b: &Face) -> bool {
    let size = (0..3).map(|i| (a.hi[i] - a.lo[i]).max(b.hi[i] - b.lo[i])).fold(0.0, f64::max);
    let tol = TOLERANCE * size;
    if (0..3).any(|i| a.hi[i] - b.lo[i] <= tol || b.hi[i] - a.lo[i] <= tol) {
        return false;
    }
    if a.normals.iter().chain(&b.normals).any(|n| apart(a, b, n, tol)) {
        return false;
    }
    for ea in &a.edges {
        for eb in &b.edges {
            if unit(cross(ea, eb)).is_some_and(|n| apart(a, b, &n, tol)) {
                return false;
            }
        }
    }
    true
}

/// The world vertices of the cut face of chamber `upper` on home pole `pole`, in its hyperplane's 3D frame.
#[allow(clippy::too_many_arguments)]
fn place(
    engine: &Engine,
    geometry: &faces::Geometry,
    vertices: &[Vec<[f64; 4]>],
    transports: &mut HashMap<u16, F4>,
    rep: &F4,
    frame: &[[f64; 4]; 3],
    domain: u32,
    upper: u32,
    pole: u16,
) -> Result<Face, String> {
    let face = geometry
        .piece_faces(engine, upper as usize)?
        .into_iter()
        .find(|f| f.pole == pole && f.side == Side::In)
        .ok_or_else(|| format!("chamber {upper} has no cut face on pole {pole}"))?;
    let k = match transports.get(&face.transport) {
        Some(k) => *k,
        None => {
            let k = float4(engine.kplus.matrix(face.transport, &engine.model))?;
            transports.insert(face.transport, k);
            k
        }
    };
    let region = &vertices[face.base.orbit as usize];
    let pts: Vec<P3> = face
        .base
        .vertices
        .iter()
        .map(|&v| {
            let w = apply(rep, &apply(&k, &region[v as usize]));
            [dot4(&frame[0], &w), dot4(&frame[1], &w), dot4(&frame[2], &w)]
        })
        .collect();
    let at = |v: u16| face.base.vertices.binary_search(&v).map(|i| pts[i]).map_err(|_| "a polygon vertex off its face");
    let (mut normals, mut edges) = (Vec::new(), Vec::new());
    let mut seen = Vec::new();
    for polygon in &face.base.polygons {
        let q: Vec<P3> = polygon.iter().map(|&v| at(v)).collect::<Result<_, _>>()?;
        let mut n = [0.0; 3];
        for i in 1..q.len().saturating_sub(1) {
            let c = cross(&sub(&q[i], &q[0]), &sub(&q[i + 1], &q[0]));
            n = [n[0] + c[0], n[1] + c[1], n[2] + c[2]];
        }
        normals.extend(unit(n));
        for i in 0..polygon.len() {
            let (x, y) = (polygon[i], polygon[(i + 1) % polygon.len()]);
            let key = (x.min(y), x.max(y));
            if !seen.contains(&key) {
                seen.push(key);
                edges.extend(unit(sub(&at(y)?, &at(x)?)));
            }
        }
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in &pts {
        for ((l, h), x) in lo.iter_mut().zip(hi.iter_mut()).zip(p) {
            *l = l.min(*x);
            *h = h.max(*x);
        }
    }
    Ok(Face { domain, pts, normals, edges, lo, hi })
}

/// The domain contact graph of a state (ADR-D02.21).
pub fn contact_graph(engine: &Engine, state: &State) -> Result<ContactGraph, String> {
    let seams = observe::seam_cells(engine, state)?;
    let geometry = faces::geometry(engine)?;
    let vertices: Vec<Vec<[f64; 4]>> = engine
        .regions
        .rep_vertices
        .iter()
        .map(|region| {
            region
                .iter()
                .map(|v| {
                    Ok([
                        approximate64(&v[0]).ok_or("a vertex has no finite value")?,
                        approximate64(&v[1]).ok_or("a vertex has no finite value")?,
                        approximate64(&v[2]).ok_or("a vertex has no finite value")?,
                        approximate64(&v[3]).ok_or("a vertex has no finite value")?,
                    ])
                })
                .collect::<Result<Vec<_>, &str>>()
        })
        .collect::<Result<_, _>>()?;
    let reps: Vec<F4> = seams.reps.iter().map(float4).collect::<Result<_, _>>()?;
    let frames: Vec<[[f64; 4]; 3]> = seams
        .normals
        .iter()
        .map(|n| {
            let u = [
                approximate64(&n[0]).ok_or("a normal has no finite value")?,
                approximate64(&n[1]).ok_or("a normal has no finite value")?,
                approximate64(&n[2]).ok_or("a normal has no finite value")?,
                approximate64(&n[3]).ok_or("a normal has no finite value")?,
            ];
            Ok(basis(&u))
        })
        .collect::<Result<_, &str>>()?;
    let mut transports = HashMap::new();
    // Per hyperplane: the faces held on the upper side and those held on the lower side.
    let mut by_plane: BTreeMap<u32, (Vec<Face>, Vec<Face>)> = BTreeMap::new();
    for cell in &seams.cells {
        let face = place(
            engine,
            geometry,
            &vertices,
            &mut transports,
            &reps[cell.domain as usize],
            &frames[cell.plane as usize],
            cell.domain,
            cell.upper,
            cell.pole,
        )?;
        let sides = by_plane.entry(cell.plane).or_default();
        if cell.upper_held {
            sides.0.push(face);
        } else {
            sides.1.push(face);
        }
    }
    let mut edges: BTreeMap<(u32, u32), Contact> = BTreeMap::new();
    let mut tests = 0u64;
    for (&plane, (upper, lower)) in &by_plane {
        let on_grip = seams.on_grip[plane as usize];
        let mut found: Vec<(u32, u32)> = Vec::new();
        for a in upper {
            for b in lower {
                if a.domain == b.domain {
                    continue;
                }
                let key = (a.domain.min(b.domain), a.domain.max(b.domain));
                if found.contains(&key) {
                    continue;
                }
                if (0..3).any(|i| a.hi[i] < b.lo[i] || b.hi[i] < a.lo[i]) {
                    continue;
                }
                tests += 1;
                if overlap(a, b) {
                    found.push(key);
                    let edge =
                        edges.entry(key).or_insert(Contact { a: key.0, b: key.1, on_grip: false, off_grip: false });
                    edge.on_grip |= on_grip;
                    edge.off_grip |= !on_grip;
                }
            }
        }
    }
    Ok(ContactGraph {
        keys: seams.keys,
        edges: edges.into_values().collect(),
        seam_faces: seams.cells.len() as u64,
        planes: by_plane.len() as u64,
        tests,
    })
}
