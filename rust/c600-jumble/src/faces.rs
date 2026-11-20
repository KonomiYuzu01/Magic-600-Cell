//! Exposed-face geometry (render feature J10; JI.41, JI.45): M5's exact geometry producer.
//!
//! An exposed face is a cut 3-face of a piece region: the face of piece p on the cut hyperplane H_e: n_e·x = κ of a
//! pole e (jumbling-observation, sections 4 and 8). It is one side of the cut cell (s, s ∪ {e}) of the chamber tiling:
//! side `in` when e is in p's signature (p is the upper chamber), `out` otherwise (p is the lower chamber). The 600
//! cells whose lower chamber is the core have no `out` face. Faces carry no sticker; the host faces on the facets
//! n_e·x = |n|² are the stickers and are counted here only to certify the regions' boundaries.
//!
//! # Source
//!
//! The faces are derived at load from M5's exact regions ([`crate::regions`]), built from `assets/` alone: one table
//! per K⁺-orbit representative (36), transported exactly to every piece by its K⁺ element r, which carries the face on
//! pole e to the face on pole r(e) and keeps the side (RPI.18: engine-supplied geometry). No mesh asset is read or
//! written, and the model identity, the manifest and render/1 are unchanged. The exact-source digest
//! ([`Geometry::digest`]) is the SHA-256 of the canonical exact encoding of the base table ([`Geometry::canonical`]),
//! which names the manifest's `model_id`, each representative's exact region vertices and each face's oriented
//! 2-faces.
//!
//! # Identity and topology
//!
//! A face is named by (piece, home pole, side) and lies on the cell [lower, upper] of chambers (the piece IDs whose
//! home regions they are, [`CORE`] for the core), the identity J08's seam rows use in a domain's frame. Its topology
//! is its 2-faces, each an oriented cycle of its vertices: for consecutive vertices a, b and any other vertex w of the
//! cycle, det(b − a, w − a, a − g, N) > 0, with g the vertex centroid of the face and N the region's outward normal on
//! it (−n_e for side `in`, n_e for `out`). The fan (c₀, cᵢ, cᵢ₊₁) of each cycle triangulates the face: 2V − 4
//! triangles for V vertices. Rotations in K⁺ keep these signs, so transported faces keep their orientation.
//!
//! # Certification
//!
//! [`certify`] checks a base table in exact Q(√5), independently of [`build`] and of how the table was made, though it
//! shares with [`build`] the tight-set enumeration of a region's 3-faces and 2-faces (`facets`, `ridges`); the test
//! `base_table_invariants_hold_without_the_builder` recomputes the tight sets with exact dot products on its own:
//! 1. each orbit's faces are exactly the cut 3-faces of its region: every listed vertex lies on its hyperplane, the
//!    region lies on the side the face names, the vertex set is the whole tight set, of affine rank 3 with at least 4
//!    vertices, and no cut 3-face is missing;
//! 2. each face's cycles are exactly its 2-faces (the rank-2 intersections with the region's other 3-faces); every
//!    edge lies in exactly two of them, V − E + F = 2, the fan has 2V − 4 triangles, and every cycle is strictly convex
//!    and positively oriented by exact determinant signs;
//! 3. each region's host 3-faces are the model's sticker hosts, and every 2-face of the region lies in exactly two of
//!    its 3-faces, cut or host: the boundary closes;
//! 4. cut cells pair up: a face's opposite chamber exists (or is the core), carries the opposite face on the same
//!    pole with exactly the same vertices, and the faces of every representative are exactly the cells the signature
//!    table predicts;
//! 5. the counts weighted by orbit sizes cover the 177,120 pieces, 259,800 host faces (one per sticker) and one `out`
//!    face for every cell off the core.
//!
//! A table that fails any check is no geometry, and the answer is `KN2:input.exposed_geometry_unavailable` naming
//! `certified_geometry`. A certified table whose measured size differs from the admitted profile [`PROFILE`] is
//! refused naming `size_profile`. [`PROFILE`] agrees with EN2's reference counts (EH-05,
//! `research/jumbling/differential/leads/meshes.py`), and the measurement is recorded in
//! `docs/progress/1.0/evidence/j10-exposed-face-size-profile.json`. Nothing here is renderer, Windows/DirectX or
//! performance evidence, and no backend support for J10 is claimed (RM.15).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use c600_digest::sha256_hex;
use c600_kernel::json::Value;

use crate::kplus::ORDER;
use crate::model::{Model, KAPPA_FORM, NP, NS, POLES, SIG_WORDS};
use crate::q5::{affine_rank, det4, dot, matvec, Mat4, Vec4, Q5};
use crate::regions::VForm;
use crate::state::State;
use crate::{codes, Context, CONTRACT_REVISION};

pub const FEATURE: &str = "exposed_face_meshes";
pub const RENDER_ROW: &str = "J10";
/// The prerequisites a refusal can name (JI.45: which prerequisite is missing).
pub const MISSING: [&str; 2] = [MISSING_GEOMETRY, MISSING_PROFILE];
pub const MISSING_GEOMETRY: &str = "certified_geometry";
pub const MISSING_PROFILE: &str = "size_profile";
pub const CONTINUATION: &str = "report the feature unavailable; request it again only after certified geometry and \
                                an admitted size profile exist, or choose a supported feature";
/// The format of the canonical exact encoding of the base table.
pub const FORMAT: &str = "magic600-jumbling-exposed-faces/1";
/// The format of the canonical size profile.
pub const PROFILE_FORMAT: &str = "magic600-jumbling-exposed-faces-profile/1";
/// The chamber ID of the core in a cell, as in J08's seam rows.
pub const CORE: u32 = NP as u32;

/// The side of a cut face: `in` when the pole is in the piece's signature (the piece is the upper chamber of the
/// cell), `out` otherwise (the lower chamber).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    Out,
    In,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Side::Out => "out",
            Side::In => "in",
        }
    }
}

/// A cut 3-face of a K⁺-orbit representative's home region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseFace {
    pub orbit: u16,
    pub pole: u16,
    pub side: Side,
    /// The face's vertices: indices into the representative's region vertices
    /// ([`crate::regions::Regions::rep_vertices`]), ascending.
    pub vertices: Vec<u16>,
    /// The face's 2-faces, each an oriented cycle of region vertex indices starting at its least index; sorted.
    pub polygons: Vec<Vec<u16>>,
}

impl BaseFace {
    /// The number of fan triangles.
    pub fn triangles(&self) -> usize {
        self.polygons.iter().map(|c| c.len().saturating_sub(2)).sum()
    }

    /// The oriented fan triangles of the face, as region vertex indices.
    pub fn fan(&self) -> Vec<[u16; 3]> {
        let mut out = Vec::with_capacity(self.triangles());
        for c in &self.polygons {
            for i in 1..c.len().saturating_sub(1) {
                out.push([c[0], c[i], c[i + 1]]);
            }
        }
        out
    }
}

/// The base table: the cut faces of every representative, sorted by orbit and pole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub faces: Vec<BaseFace>,
}

/// Face, vertex, 2-face and triangle counts, vertices and 2-faces counted per 3-face as a mesh stores them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub faces3: u64,
    pub vertices: u64,
    pub faces2: u64,
    pub triangles: u64,
}

impl Counts {
    const fn new(faces3: u64, vertices: u64, faces2: u64, triangles: u64) -> Counts {
        Counts { faces3, vertices, faces2, triangles }
    }

    fn add(&mut self, other: Counts, factor: u64) {
        self.faces3 += other.faces3 * factor;
        self.vertices += other.vertices * factor;
        self.faces2 += other.faces2 * factor;
        self.triangles += other.triangles * factor;
    }

    fn sum(a: Counts, b: Counts) -> Counts {
        let mut s = a;
        s.add(b, 1);
        s
    }

    fn to_json(self) -> Value {
        obj(vec![
            ("faces3", int(self.faces3)),
            ("vertices", int(self.vertices)),
            ("faces2", int(self.faces2)),
            ("triangles", int(self.triangles)),
        ])
    }
}

/// The size profile of the exposed-face geometry: counts of the base table (one region per K⁺ orbit) and of the full
/// table (every piece), cut faces by side and host faces, and the byte sizes they imply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    pub orbits: u64,
    pub pieces: u64,
    pub base_in: Counts,
    pub base_out: Counts,
    pub base_host: Counts,
    pub full_in: Counts,
    pub full_out: Counts,
    pub full_host: Counts,
    /// Cut cells: one `in` face each.
    pub cells: u64,
    /// Cells whose lower chamber is the core: no `out` face.
    pub core_cells: u64,
    /// The length of the canonical exact encoding of the base table.
    pub exact_bytes: u64,
}

/// The admitted size profile (EH-05): EN2's reference counts for the cut and host faces, measured again by
/// [`certify`] and recorded in `docs/progress/1.0/evidence/j10-exposed-face-size-profile.json`.
pub const PROFILE: Profile = Profile {
    orbits: 36,
    pieces: NP as u64,
    base_in: Counts::new(122, 545, 540, 602),
    base_out: Counts::new(113, 515, 508, 578),
    base_host: Counts::new(80, 393, 390, 466),
    full_in: Counts::new(469_800, 2_020_800, 2_011_200, 2_162_400),
    full_out: Counts::new(469_200, 2_018_400, 2_008_800, 2_160_000),
    full_host: Counts::new(259_800, 1_238_400, 1_250_400, 1_437_600),
    cells: 469_800,
    core_cells: 600,
    exact_bytes: 36_258,
};

impl Profile {
    /// The cut faces of the base table.
    pub fn base_cut(&self) -> Counts {
        Counts::sum(self.base_in, self.base_out)
    }

    /// The cut faces of the full table.
    pub fn full_cut(&self) -> Counts {
        Counts::sum(self.full_in, self.full_out)
    }

    /// The bytes of the full table expanded per piece: per face its piece (u32), home pole (u32), side (u8), cell
    /// (2 × u32) and vertex offset (u32, one more than the faces); per vertex four binary32 coordinates; per triangle
    /// three u32 indices. A sizing figure for the renderer line's layout choice; nothing produces this layout.
    pub fn indexed_bytes(&self) -> u64 {
        let c = self.full_cut();
        17 * c.faces3 + 4 * (c.faces3 + 1) + 16 * c.vertices + 12 * c.triangles
    }

    /// The bytes of the instanced display payload (`c600-display`'s `jumble::exposed_faces`): the base cut faces'
    /// vertices (4 binary32 coordinates each), their u32 triangles (3 each) and two u32 offset arrays (one more than
    /// the base faces each); the K⁺ matrices (16 binary32 entries each); each piece's K⁺ transport (u32); and per
    /// face of the full table its piece, base face and home pole (u32 each), side (u8) and cell (2 × u32).
    pub fn instanced_bytes(&self) -> u64 {
        let (base, full) = (self.base_cut(), self.full_cut());
        16 * base.vertices
            + 12 * base.triangles
            + 8 * (base.faces3 + 1)
            + 64 * ORDER as u64
            + 4 * self.pieces
            + 21 * full.faces3
    }

    /// The bytes of the full cut faces as a binary32 triangle soup (three corners of four coordinates), EN2's measure.
    pub fn soup_bytes(&self) -> u64 {
        48 * self.full_cut().triangles
    }

    /// The bytes of the base cut faces as a binary32 triangle soup.
    pub fn base_soup_bytes(&self) -> u64 {
        48 * self.base_cut().triangles
    }

    pub fn to_json(&self) -> Value {
        obj(vec![
            ("format", text(PROFILE_FORMAT)),
            ("orbits", int(self.orbits)),
            ("pieces", int(self.pieces)),
            (
                "base",
                obj(vec![
                    ("in", self.base_in.to_json()),
                    ("out", self.base_out.to_json()),
                    ("cut", self.base_cut().to_json()),
                    ("host", self.base_host.to_json()),
                ]),
            ),
            (
                "full",
                obj(vec![
                    ("in", self.full_in.to_json()),
                    ("out", self.full_out.to_json()),
                    ("cut", self.full_cut().to_json()),
                    ("host", self.full_host.to_json()),
                ]),
            ),
            ("cells", int(self.cells)),
            ("core_cells", int(self.core_cells)),
            (
                "bytes",
                obj(vec![
                    ("exact_base_table", int(self.exact_bytes)),
                    ("binary32_base_soup", int(self.base_soup_bytes())),
                    ("binary32_indexed", int(self.indexed_bytes())),
                    ("binary32_instanced", int(self.instanced_bytes())),
                    ("binary32_soup", int(self.soup_bytes())),
                ]),
            ),
        ])
    }

    /// The profile ID: the SHA-256 of the canonical profile.
    pub fn id(&self) -> String {
        format!("sha256:{}", sha256_hex(self.to_json().canonical().as_bytes()))
    }

    /// The admitted profile admits a measured one when they are equal.
    pub fn admits(&self, measured: &Profile) -> bool {
        self == measured
    }
}

/// Certified exposed-face geometry: the base table, its canonical encoding and digest, and its measured profile.
pub struct Geometry {
    pub table: Table,
    /// The canonical exact encoding of the base table (the certified bytes).
    pub canonical: Vec<u8>,
    /// SHA-256 of [`Geometry::canonical`]: the exact-source digest.
    pub digest: String,
    pub profile: Profile,
    by_orbit: Vec<(usize, usize)>,
    sig_index: SigIndex,
}

/// One face of one piece: its base face transported by the piece's K⁺ element.
#[derive(Clone, Copy, Debug)]
pub struct PieceFace<'a> {
    pub base: &'a BaseFace,
    /// The index of [`PieceFace::base`] in the base table.
    pub base_index: u32,
    pub piece: u32,
    /// The K⁺ element carrying the representative's region to the piece's.
    pub transport: u16,
    /// The home pole of the face: the image of the base face's pole.
    pub pole: u16,
    pub side: Side,
    /// The cell [lower, upper], with [`CORE`] for the core.
    pub cell: [u32; 2],
}

impl Geometry {
    fn new(ctx: &Context) -> Result<Geometry, String> {
        let table = build(ctx)?;
        let sig_index = SigIndex::new(&ctx.model)?;
        let profile = certify_with(ctx, &table, &sig_index)?;
        let canonical = encode(ctx, &table);
        let digest = sha256_hex(&canonical);
        let mut by_orbit = vec![(0usize, 0usize); ctx.regions.reps.len()];
        for (i, face) in table.faces.iter().enumerate() {
            let range = &mut by_orbit[face.orbit as usize];
            if range.1 == 0 {
                *range = (i, i + 1);
            } else {
                range.1 = i + 1;
            }
        }
        Ok(Geometry { table, canonical, digest, profile, by_orbit, sig_index })
    }

    /// The faces of the representative of an orbit.
    pub fn orbit_faces(&self, orbit: usize) -> &[BaseFace] {
        let (a, b) = self.by_orbit[orbit];
        &self.table.faces[a..b]
    }

    /// The faces of piece p, in its orbit's base order.
    pub fn piece_faces(&self, ctx: &Context, p: usize) -> Result<Vec<PieceFace<'_>>, String> {
        let (o, k) = ctx.regions.rep_of(p);
        let perm = &ctx.kplus.perms[k as usize];
        let bits = ctx.model.sig_bits(p);
        let first = self.by_orbit[o].0;
        self.orbit_faces(o)
            .iter()
            .enumerate()
            .map(|(i, base)| {
                let pole = perm[base.pole as usize];
                let other = self.sig_index.toggled(bits, pole as usize);
                let cell = match base.side {
                    Side::In => [other.ok_or_else(|| format!("piece {p}, pole {pole}: no lower chamber"))?, p as u32],
                    Side::Out => {
                        let up = other.filter(|&q| q != CORE);
                        [p as u32, up.ok_or_else(|| format!("piece {p}, pole {pole}: no upper chamber"))?]
                    }
                };
                let base_index = (first + i) as u32;
                Ok(PieceFace { base, base_index, piece: p as u32, transport: k, pole, side: base.side, cell })
            })
            .collect()
    }
}

/// The certified geometry of a context, built and certified on first use.
pub fn geometry(ctx: &Context) -> Result<&Geometry, String> {
    ctx.faces.get_or_init(|| Geometry::new(ctx)).as_ref().map_err(Clone::clone)
}

// ------------------------------------------------------------------------------------------------- the answer

fn int(v: u64) -> Value {
    Value::Int(v as i64)
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect::<BTreeMap<_, _>>())
}

fn source(ctx: &Context, state: &State) -> Value {
    let menu = state.menu();
    obj(vec![
        ("model_id", text(&ctx.model.model_id)),
        ("menu_id", menu.map_or(Value::Null, |m| text(&m.identity))),
        ("menu_order", menu.map_or(Value::Null, |m| text(&m.order))),
        ("contract_rev", text(CONTRACT_REVISION)),
        ("state_digest", text(&state.digest(ctx))),
    ])
}

/// J10's exposed faces at `state` with this context's certified geometry and the admitted profile [`PROFILE`].
pub fn exposed_faces(ctx: &Context, state: &State) -> Value {
    let certified = geometry(ctx);
    answer(ctx, state, certified.as_ref().map(|g| *g).map_err(String::as_str), Some(&PROFILE))
}

/// The producer's answer for a geometry and an admitted profile (JI.45). Available: the state's exact input
/// reference, the geometry's format, model and exact-source digest, and the measured profile with its ID; the faces
/// themselves are read through [`geometry`]. Refused, with `KN2:input.exposed_geometry_unavailable`: the feature, the
/// exact input reference, the missing prerequisites (`certified_geometry` when the geometry failed certification,
/// `size_profile` when no profile is admitted or the measured one differs), the failed fact, no draft and the
/// continuation. No mesh, face or size is invented, and nothing changes.
pub fn answer(ctx: &Context, state: &State, geometry: Result<&Geometry, &str>, admitted: Option<&Profile>) -> Value {
    let (missing, failed): (Vec<&str>, String) = match (geometry, admitted) {
        (Err(reason), None) => (vec![MISSING_GEOMETRY, MISSING_PROFILE], reason.to_owned()),
        (Err(reason), Some(_)) => (vec![MISSING_GEOMETRY], reason.to_owned()),
        (Ok(_), None) => (vec![MISSING_PROFILE], "no size profile is admitted".to_owned()),
        (Ok(g), Some(p)) if !p.admits(&g.profile) => (
            vec![MISSING_PROFILE],
            format!("the measured profile {} is not the admitted profile {}", g.profile.id(), p.id()),
        ),
        (Ok(g), Some(_)) => {
            return obj(vec![
                ("status", text("available")),
                ("feature", text(FEATURE)),
                ("render_row", text(RENDER_ROW)),
                ("source", source(ctx, state)),
                (
                    "geometry",
                    obj(vec![
                        ("format", text(FORMAT)),
                        ("model_id", text(&ctx.model.model_id)),
                        ("digest", text(&g.digest)),
                        ("orbits", int(ctx.regions.reps.len() as u64)),
                        ("base_faces", int(g.table.faces.len() as u64)),
                    ]),
                ),
                ("profile_id", text(&g.profile.id())),
                ("profile", g.profile.to_json()),
            ]);
        }
    };
    let message = match missing.as_slice() {
        [MISSING_PROFILE] => "the exposed-face geometry has no admitted size profile",
        [MISSING_GEOMETRY] => "certified exposed-face geometry is absent",
        _ => "certified exposed-face geometry and an admitted size profile are absent",
    };
    obj(vec![
        ("status", text("refused")),
        ("code", text(codes::EXPOSED_GEOMETRY_UNAVAILABLE)),
        ("message", text(message)),
        ("feature", text(FEATURE)),
        ("render_row", text(RENDER_ROW)),
        ("source", source(ctx, state)),
        ("missing", Value::Array(missing.iter().map(|m| text(m)).collect())),
        ("failed", text(&failed)),
        ("draft", Value::Null),
        ("continuation", text(CONTINUATION)),
    ])
}

// ---------------------------------------------------------------------------------------------- exact helpers

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Cut,
    Host,
}

/// A 3-face of a region: its hyperplane and its vertex set (ascending region vertex indices).
struct Facet {
    kind: Kind,
    pole: u16,
    set: Vec<u16>,
}

/// The vertices of an integer vertex form on the hyperplane n_e·x = κ (cut) or n_e·x = |n|² (host), ascending.
/// With n_e = (n1 + n2√5)/2 and x = (a + b√5)/E, n_e·x = (p1 + p2√5)/(2E).
fn tight(model: &Model, form: &VForm, kind: Kind, e: usize) -> Vec<u16> {
    let (k0, k1, k2) = (KAPPA_FORM.0 as i128, KAPPA_FORM.1 as i128, KAPPA_FORM.2 as i128);
    let big_e = form.e as i128;
    let (n1, n2) = (model.n1[e], model.n2[e]);
    let mut out = Vec::new();
    for (i, row) in form.rows.iter().enumerate() {
        let (mut p1, mut p2) = (0i128, 0i128);
        for j in 0..4 {
            let (a, b) = (row[j] as i128, row[4 + j] as i128);
            p1 += a * n1[j] as i128 + 5 * b * n2[j] as i128;
            p2 += b * n1[j] as i128 + a * n2[j] as i128;
        }
        let on = match kind {
            Kind::Cut => k2 * p1 == 2 * big_e * k0 && k2 * p2 == 2 * big_e * k1,
            Kind::Host => p1 == 24 * big_e && p2 == 8 * big_e,
        };
        if on {
            out.push(i as u16);
        }
    }
    out
}

fn points(verts: &[Vec4], set: &[u16]) -> Vec<Vec<Q5>> {
    set.iter().map(|&i| verts[i as usize].to_vec()).collect()
}

/// The dimension of the affine hull of a vertex set; -1 for the empty set.
fn dimension(verts: &[Vec4], set: &[u16]) -> i64 {
    if set.is_empty() {
        -1
    } else {
        affine_rank(&points(verts, set)) as i64
    }
}

/// The 3-faces of a region: the hyperplanes whose tight sets have at least 4 vertices and affine rank 3.
fn facets(model: &Model, verts: &[Vec4], form: &VForm) -> Vec<Facet> {
    let mut out = Vec::new();
    for kind in [Kind::Cut, Kind::Host] {
        for e in 0..POLES {
            let set = tight(model, form, kind, e);
            if set.len() >= 4 && dimension(verts, &set) == 3 {
                out.push(Facet { kind, pole: e as u16, set });
            }
        }
    }
    out
}

fn intersect(a: &[u16], b: &[u16]) -> Vec<u16> {
    a.iter().filter(|v| b.binary_search(v).is_ok()).copied().collect()
}

/// The 2-faces of facet i: its rank-2 intersections with the region's other 3-faces, as sorted vertex sets.
fn ridges(verts: &[Vec4], facets: &[Facet], i: usize) -> Vec<Vec<u16>> {
    let mut out = BTreeSet::new();
    for (j, other) in facets.iter().enumerate() {
        if j != i {
            let common = intersect(&facets[i].set, &other.set);
            if common.len() >= 3 && dimension(verts, &common) == 2 {
                out.insert(common);
            }
        }
    }
    out.into_iter().collect()
}

fn sub4(a: &Vec4, b: &Vec4) -> Vec4 {
    std::array::from_fn(|j| a[j].sub(&b[j]))
}

/// The orientation frame of a face: m times the vertex sum's divisor, the vertex sum and the outward normal.
struct Frame {
    m: Q5,
    sum: Vec4,
    normal: Vec4,
}

impl Frame {
    fn new(model: &Model, verts: &[Vec4], set: &[u16], pole: u16, side: Side) -> Frame {
        let mut sum: Vec4 = std::array::from_fn(|_| Q5::zero());
        for &i in set {
            sum = std::array::from_fn(|j| sum[j].add(&verts[i as usize][j]));
        }
        let n = &model.poles[pole as usize];
        let normal = match side {
            Side::In => std::array::from_fn(|j| n[j].neg()),
            Side::Out => n.clone(),
        };
        Frame { m: Q5::int(set.len() as i64), sum, normal }
    }

    /// The sign of det(b − a, w − a, m·a − Σ, N): positive when w lies to the left of a → b seen from outside.
    fn sign(&self, a: &Vec4, b: &Vec4, w: &Vec4) -> i32 {
        let x: Vec4 = std::array::from_fn(|j| self.m.mul(&a[j]).sub(&self.sum[j]));
        let rows: Mat4 = [sub4(b, a), sub4(w, a), x, self.normal.clone()];
        det4(&rows).sign()
    }
}

/// The cycle of a 2-face from the face's other 2-faces: two 2-faces of a 3-face sharing exactly two vertices share
/// the edge between them. Unoriented, starting at the least vertex.
fn cycle_of(polygon: &[u16], all: &[Vec<u16>]) -> Result<Vec<u16>, String> {
    let mut adj: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
    for other in all {
        if other.as_slice() != polygon {
            let common = intersect(polygon, other);
            if common.len() == 2 {
                adj.entry(common[0]).or_default().push(common[1]);
                adj.entry(common[1]).or_default().push(common[0]);
            }
        }
    }
    if polygon.iter().any(|v| adj.get(v).map_or(0, Vec::len) != 2) {
        return Err(format!("the 2-face {polygon:?} has a vertex without exactly two edges"));
    }
    let start = polygon[0];
    let mut cycle = vec![start];
    let (mut prev, mut cur) = (start, adj[&start][0]);
    while cur != start {
        if cycle.len() >= polygon.len() {
            return Err(format!("the edges of the 2-face {polygon:?} do not close"));
        }
        cycle.push(cur);
        let next = adj[&cur].iter().copied().find(|&n| n != prev).unwrap_or(prev);
        (prev, cur) = (cur, next);
    }
    if cycle.len() != polygon.len() {
        return Err(format!("the edges of the 2-face {polygon:?} form more than one cycle"));
    }
    Ok(cycle)
}

/// Rotates a cycle to start at its least vertex, keeping its direction.
fn canonical_cycle(cycle: &[u16]) -> Vec<u16> {
    let at = cycle.iter().enumerate().min_by_key(|&(_, v)| *v).map_or(0, |(i, _)| i);
    cycle[at..].iter().chain(&cycle[..at]).copied().collect()
}

/// Lookup of chambers by signature bits.
struct SigIndex(HashMap<[u64; SIG_WORDS], u32>);

impl SigIndex {
    fn new(model: &Model) -> Result<SigIndex, String> {
        let mut map = HashMap::with_capacity(NP);
        for p in 0..NP {
            if map.insert(*model.sig_bits(p), p as u32).is_some() {
                return Err(format!("piece {p} repeats a signature"));
            }
        }
        Ok(SigIndex(map))
    }

    /// The chamber whose signature is `bits` with pole e toggled: a piece, [`CORE`] for the empty signature, or `None`.
    fn toggled(&self, bits: &[u64; SIG_WORDS], e: usize) -> Option<u32> {
        let mut key = *bits;
        key[e / 64] ^= 1u64 << (e % 64);
        if key.iter().all(|&w| w == 0) {
            Some(CORE)
        } else {
            self.0.get(&key).copied()
        }
    }
}

// ------------------------------------------------------------------------------------------------- the builder

/// Derives the base table from the context's exact regions: the cut 3-faces of each representative, their 2-faces
/// as cycles ordered by their edges and oriented by exact determinant signs. [`certify`] checks the result.
pub fn build(ctx: &Context) -> Result<Table, String> {
    let (model, regions) = (&ctx.model, &ctx.regions);
    let mut faces = Vec::new();
    for (o, &rep) in regions.reps.iter().enumerate() {
        let verts = &regions.rep_vertices[o];
        let all = facets(model, verts, &regions.rep_vform[o]);
        for (i, facet) in all.iter().enumerate() {
            if facet.kind != Kind::Cut {
                continue;
            }
            let side = if model.in_sig(rep as usize, facet.pole as usize) { Side::In } else { Side::Out };
            let frame = Frame::new(model, verts, &facet.set, facet.pole, side);
            let sets = ridges(verts, &all, i);
            let mut polygons = Vec::with_capacity(sets.len());
            for set in &sets {
                let mut cycle = cycle_of(set, &sets).map_err(|e| format!("orbit {o}, pole {}: {e}", facet.pole))?;
                let v = |k: usize| &verts[cycle[k] as usize];
                if frame.sign(v(0), v(1), v(2)) < 0 {
                    cycle.reverse();
                }
                polygons.push(canonical_cycle(&cycle));
            }
            polygons.sort();
            faces.push(BaseFace { orbit: o as u16, pole: facet.pole, side, vertices: facet.set.clone(), polygons });
        }
    }
    Ok(Table { faces })
}

// --------------------------------------------------------------------------------------------- certification

/// Certifies a base table against the context's exact regions (checks 1 to 5 of the module documentation) and
/// returns its measured profile; the error names the first failed fact.
pub fn certify(ctx: &Context, table: &Table) -> Result<Profile, String> {
    certify_with(ctx, table, &SigIndex::new(&ctx.model)?)
}

fn certify_with(ctx: &Context, table: &Table, index: &SigIndex) -> Result<Profile, String> {
    let (model, regions) = (&ctx.model, &ctx.regions);
    let orbits = regions.reps.len();
    let keys: Vec<(u16, u16)> = table.faces.iter().map(|f| (f.orbit, f.pole)).collect();
    if keys.windows(2).any(|w| w[0] >= w[1]) {
        return Err("the faces are not sorted by orbit and pole without repeats".into());
    }
    if keys.iter().any(|&(o, _)| o as usize >= orbits) {
        return Err("a face names an orbit outside the model".into());
    }
    let mut sizes = vec![0u64; orbits];
    for p in 0..NP {
        sizes[regions.orbit_of[p] as usize] += 1;
    }
    let mut lookup: HashMap<(u16, u16), &BaseFace> = HashMap::new();
    for face in &table.faces {
        lookup.insert((face.orbit, face.pole), face);
    }
    let (mut base_in, mut base_out, mut base_host) = (Counts::default(), Counts::default(), Counts::default());
    let (mut full_in, mut full_out, mut full_host) = (Counts::default(), Counts::default(), Counts::default());
    let mut core_cells = 0u64;
    for (o, &rep) in regions.reps.iter().enumerate() {
        let rep = rep as usize;
        let verts = &regions.rep_vertices[o];
        let all = facets(model, verts, &regions.rep_vform[o]);
        let listed: Vec<&BaseFace> = table.faces.iter().filter(|f| f.orbit as usize == o).collect();
        let cut: Vec<(usize, &Facet)> = all.iter().enumerate().filter(|(_, f)| f.kind == Kind::Cut).collect();
        // Check 1: the listed faces are exactly the cut 3-faces, each on its plane and side.
        for face in &listed {
            let at = format!("orbit {o}, pole {}", face.pole);
            if face.pole as usize >= POLES || face.vertices.iter().any(|&v| v as usize >= verts.len()) {
                return Err(format!("{at}: an index outside the model"));
            }
            let n = &model.poles[face.pole as usize];
            if face.vertices.iter().any(|&v| dot(n, &verts[v as usize]) != model.kappa) {
                return Err(format!("{at}: a vertex lies off the cut hyperplane"));
            }
            let side = if model.in_sig(rep, face.pole as usize) { Side::In } else { Side::Out };
            if face.side != side {
                return Err(format!("{at}: the side differs from the signature"));
            }
            let wrong = |v: &Vec4| {
                let s = dot(n, v).sub(&model.kappa).sign();
                (side == Side::In && s < 0) || (side == Side::Out && s > 0)
            };
            if verts.iter().any(wrong) {
                return Err(format!("{at}: the region lies across the face's hyperplane"));
            }
        }
        let cut_poles: Vec<u16> = cut.iter().map(|(_, f)| f.pole).collect();
        let listed_poles: Vec<u16> = listed.iter().map(|f| f.pole).collect();
        if cut_poles != listed_poles {
            return Err(format!("orbit {o}: the listed faces are not the region's cut 3-faces"));
        }
        for (&(i, facet), face) in cut.iter().zip(&listed) {
            let at = format!("orbit {o}, pole {}", face.pole);
            if face.vertices != facet.set {
                return Err(format!("{at}: the vertices are not the face's whole tight set"));
            }
            if face.vertices.len() < 4 || dimension(verts, &face.vertices) != 3 {
                return Err(format!("{at}: the face is not three-dimensional"));
            }
            // Check 2: 2-faces, edges, Euler, triangles, convexity and orientation.
            let mut sets: Vec<Vec<u16>> = face
                .polygons
                .iter()
                .map(|c| {
                    let mut s = c.clone();
                    s.sort_unstable();
                    s
                })
                .collect();
            sets.sort();
            if sets != ridges(verts, &all, i) {
                return Err(format!("{at}: the cycles are not the face's 2-faces"));
            }
            let mut edges: BTreeMap<(u16, u16), u32> = BTreeMap::new();
            for c in &face.polygons {
                let mut seen = c.clone();
                seen.sort_unstable();
                seen.dedup();
                if c.len() < 3 || seen.len() != c.len() || canonical_cycle(c) != *c {
                    return Err(format!("{at}: a cycle is not a canonical cycle of distinct vertices"));
                }
                for k in 0..c.len() {
                    let (a, b) = (c[k], c[(k + 1) % c.len()]);
                    *edges.entry((a.min(b), a.max(b))).or_default() += 1;
                }
            }
            if edges.values().any(|&n| n != 2) {
                return Err(format!("{at}: an edge does not lie in exactly two 2-faces"));
            }
            let (v, e, f) = (face.vertices.len() as i64, edges.len() as i64, face.polygons.len() as i64);
            if v - e + f != 2 || face.triangles() as i64 != 2 * v - 4 {
                return Err(format!("{at}: V - E + F or the triangle count fails"));
            }
            let frame = Frame::new(model, verts, &face.vertices, face.pole, face.side);
            for c in &face.polygons {
                for k in 0..c.len() {
                    let (a, b) = (&verts[c[k] as usize], &verts[c[(k + 1) % c.len()] as usize]);
                    for (w, &x) in c.iter().enumerate() {
                        if w != k && w != (k + 1) % c.len() && frame.sign(a, b, &verts[x as usize]) <= 0 {
                            return Err(format!("{at}: a 2-face is not strictly convex and positively oriented"));
                        }
                    }
                }
            }
            let counts = Counts::new(1, v as u64, f as u64, face.triangles() as u64);
            match face.side {
                Side::In => base_in.add(counts, 1),
                Side::Out => base_out.add(counts, 1),
            }
            match face.side {
                Side::In => full_in.add(counts, sizes[o]),
                Side::Out => full_out.add(counts, sizes[o]),
            }
        }
        // Check 3: the hosts are the stickers, and the boundary closes.
        let hosts: Vec<u16> = all.iter().filter(|f| f.kind == Kind::Host).map(|f| f.pole).collect();
        if hosts != model.hosts(rep) {
            return Err(format!("orbit {o}: the host faces are not the piece's stickers"));
        }
        let mut ridge_count: BTreeMap<Vec<u16>, u32> = BTreeMap::new();
        for i in 0..all.len() {
            let sets = ridges(verts, &all, i);
            if all[i].kind == Kind::Host {
                let v = all[i].set.len() as u64;
                let f = sets.len() as u64;
                let t = sets.iter().map(|s| s.len() as u64 - 2).sum();
                base_host.add(Counts::new(1, v, f, t), 1);
                full_host.add(Counts::new(1, v, f, t), sizes[o]);
            }
            for s in sets {
                *ridge_count.entry(s).or_default() += 1;
            }
        }
        if ridge_count.values().any(|&n| n != 2) {
            return Err(format!("orbit {o}: a 2-face of the region does not lie in exactly two 3-faces"));
        }
        // Check 4: the cells the signature table predicts, each paired with the opposite face.
        let bits = model.sig_bits(rep);
        for e in 0..POLES {
            let other = index.toggled(bits, e);
            let inside = model.in_sig(rep, e);
            let predicted = match other {
                Some(CORE) => inside,
                Some(_) => true,
                None => false,
            };
            if predicted != listed_poles.contains(&(e as u16)) {
                return Err(format!("orbit {o}, pole {e}: the faces differ from the cells of the signature table"));
            }
            let Some(q) = other else { continue };
            if q == CORE {
                core_cells += sizes[o];
                continue;
            }
            let face = lookup[&(o as u16, e as u16)];
            let (o2, k) = regions.rep_of(q as usize);
            let perm = &ctx.kplus.perms[k as usize];
            let f = perm.iter().position(|&x| x as usize == e).ok_or("a K+ permutation is not onto")?;
            let opposite = lookup
                .get(&(o2 as u16, f as u16))
                .ok_or_else(|| format!("orbit {o}, pole {e}: the opposite chamber has no face on the cell"))?;
            if opposite.side == face.side {
                return Err(format!("orbit {o}, pole {e}: the opposite face has the same side"));
            }
            let m = ctx.kplus.matrix(k, model);
            let mut there: Vec<Vec4> =
                opposite.vertices.iter().map(|&v| matvec(m, &regions.rep_vertices[o2][v as usize])).collect();
            let mut here: Vec<Vec4> = face.vertices.iter().map(|&v| verts[v as usize].clone()).collect();
            let key = |v: &Vec4| v.iter().map(|x| format!("{x:?}")).collect::<Vec<_>>();
            there.sort_by_key(key);
            here.sort_by_key(key);
            if there != here {
                return Err(format!("orbit {o}, pole {e}: the two sides of the cell have different vertices"));
            }
        }
    }
    // Check 5: coverage.
    let cells = full_in.faces3;
    if sizes.iter().sum::<u64>() != NP as u64 || full_host.faces3 != NS as u64 || full_out.faces3 + core_cells != cells
    {
        return Err("the faces do not cover the pieces, the stickers and the cells".into());
    }
    Ok(Profile {
        orbits: orbits as u64,
        pieces: NP as u64,
        base_in,
        base_out,
        base_host,
        full_in,
        full_out,
        full_host,
        cells,
        core_cells,
        exact_bytes: encode(ctx, table).len() as u64,
    })
}

// ------------------------------------------------------------------------------------------------- encoding

/// The canonical exact encoding of a base table: a format line, the model, then per orbit its representative, its
/// exact region vertices as `[a,b,d]` triples (value (a + b√5)/d in lowest terms) and its faces with their cycles.
pub fn encode(ctx: &Context, table: &Table) -> Vec<u8> {
    let mut out = String::new();
    out.push_str(FORMAT);
    out.push('\n');
    out.push_str("model_id ");
    out.push_str(&ctx.model.model_id);
    out.push('\n');
    for (o, &rep) in ctx.regions.reps.iter().enumerate() {
        let verts = &ctx.regions.rep_vertices[o];
        out.push_str(&format!("orbit {o} piece {rep} vertices {}\n", verts.len()));
        for v in verts {
            out.push('v');
            for x in v {
                out.push(' ');
                x.write_compact(&mut out);
            }
            out.push('\n');
        }
        for face in table.faces.iter().filter(|f| f.orbit as usize == o) {
            let list = |s: &[u16]| s.iter().map(u16::to_string).collect::<Vec<_>>().join(" ");
            out.push_str(&format!("face {} {} {}\n", face.pole, face.side.as_str(), list(&face.vertices)));
            for c in &face.polygons {
                out.push_str(&format!("polygon {}\n", list(c)));
            }
        }
    }
    out.into_bytes()
}
