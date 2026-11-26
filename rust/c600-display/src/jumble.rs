//! Producers for the jumbling puzzle (RPI.15, RM.34 rows J02 to J05 and J13, and the payloads of J01 and J06 to J12),
//! built from M5's kernel (`c600_jumble`). They copy what the engine certified and convert exact values to binary32 for display; they
//! classify nothing, infer no inside set, plane or slot, and truncate no set. What the engine does not supply is an
//! explicit [`Missing`] value or an `undefined` projection, never a guessed payload.
//!
//! - [`posed`] builds the posed-sticker row (J02, render/1 section 3) from a state's canonical pose table;
//!   [`revision`] and [`bundle`] bind it to its pose-level revision (RPI.9, render/1 section 7).
//! - [`transform`] builds the certified transform row (J03) from an applied twist's certified inside set and its
//!   engine-named plane ([`swept_plane`], render/1 section 4).
//! - [`named_cap_set`], [`witness`] and [`named_cut`] build the certified overlay row (J04).
//! - [`seam_table`] reads the seam table of J08 from the engine's `seam_facts` answer, and [`observations`] reads the
//!   payloads of J07, J09, J11 and J12. Like J08 they are payloads, not supported rows.
//! - [`exposed_faces`] builds J10's exposed-face payload from M5's certified exact geometry and its admitted size
//!   profile, and [`check_exposed_faces`] is its host-side check; a refusal of the producer passes through (JI.45).
//! - [`shrink_anchors`] computes J01's shrink anchors from the unchanged mesh, and [`readback_check`] is J06's
//!   first-frame readback check of a revision's bound arrays.
//! - [`PoseProjection`] is the kernel's exact projection to slots for a jumbling subject (RPI.23, J13), with M8's slot
//!   action of K⁺ for a piece at a lattice pose.
//! - [`support_report`] and [`fixture_context`] are declared fixture values, as in [`crate::produce`] (RPI.50).

use c600_digest::sha256_hex;
use c600_jumble::kplus::ORDER;
use c600_jumble::levels;
use c600_jumble::model::{NP, POLES};
use c600_jumble::q5::{identity4, Vec4, Q5};
use c600_jumble::state::{Certificate, Classification, Outcome, State, Status};
use c600_jumble::twist::Twist;
use c600_jumble::Context as Engine;
use c600_kernel::json::Value;
use c600_kernel::{IdentityBlock, Puzzle};

use crate::array::{Array, Elem};
use crate::crosswalk::{self, Seam, SeamTable};
use crate::error::{codes, Error};
use crate::pick::{Partial, Projection};
use crate::plan::{
    piece_set_digest, CertKind, CertOverlay, CertPoint, GripStatus, Level, Mode, NamedCut, ObjectId, PointPair,
    PosedInput, RevisionRef, StateRef, TransformInput,
};
use crate::produce::{self, GeometryInput};
use crate::validate::{bundle_names, BoundArray, Context, EngineBundle, LimitProfile, Support, SupportReport};

/// Data a jumbling producer could not supply. It is a producer outcome, not a host error: no payload is built from
/// it, and the host never sees a partial one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    /// An entry of this pose has no finite binary32 value.
    NonFinite { pose: u32 },
    /// The named-cap set left this many pieces unclassified under its budget; no shorter set stands for it (J04).
    Uncertain { cap: u32, pieces: u64 },
    /// The cap is not a pole of the model.
    NoSuchCap { cap: u32 },
    /// The engine reported an error.
    Engine(String),
    /// The plane the engine names for the twist at this grip has no finite binary32 value (J03).
    NonFinitePlane { grip: u32 },
    /// The twist at this grip is not the identity, yet the engine names no plane for it: a matrix that is not
    /// exactly a rotation, or one that turns two planes. It has no swept preview and is never shown as the identity
    /// (J03). `Twist::new` admits neither, since a twist fixes its grip's pole.
    NoPlane { grip: u32 },
    /// The twist given for an outcome turns another grip, so it is not the letter the outcome certified, and no plane
    /// is taken from it (J03).
    OtherGrip { outcome: u32, twist: u32 },
    /// The engine's answer for this payload row did not complete: a job cut off by its budget, a cancelled job, a
    /// refusal or an error status, with the answer's status. No partial table stands for it (JI.14).
    Incomplete { row: &'static str, status: String },
    /// The engine's answer for this payload row lacks a field the payload copies, or the field has another form.
    /// Nothing is filled in.
    Malformed { row: &'static str, field: &'static str },
    /// The engine's value is undefined for a reason this payload has no form for (ν's `pieces_collide`).
    Undefined { row: &'static str, reason: String },
}

pub mod contacts;
pub mod observations;

/// The binary64 value of an exact number: (a + b√5)/d, each operation rounded. `None` when a coefficient does not
/// fit binary64 or the value is not finite.
fn approximate64(x: &Q5) -> Option<f64> {
    let part = |n: String| n.parse::<f64>().ok();
    let (a, b, d) = (part(x.a().to_string())?, part(x.b().to_string())?, part(x.d().to_string())?);
    let value = (a + b * 5f64.sqrt()) / d;
    value.is_finite().then_some(value)
}

/// The binary32 display value of an exact number: (a + b√5)/d evaluated in binary64, each operation rounded, then
/// rounded to binary32, so it may differ from the correctly rounded value in the last place. `None` when a
/// coefficient does not fit binary64 or the value is not finite. It is display data only; nothing exact is decided
/// from it.
pub fn approximate(x: &Q5) -> Option<f32> {
    let value = approximate64(x)? as f32;
    value.is_finite().then_some(value)
}

fn approximate4(v: &Vec4) -> Option<[f32; 4]> {
    Some([approximate(&v[0])?, approximate(&v[1])?, approximate(&v[2])?, approximate(&v[3])?])
}

/// The compact exact text of a vector: its four `[a,b,d]` triples.
fn exact_text(v: &[Q5]) -> String {
    let mut out = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        x.write_compact(&mut out);
    }
    out.push(']');
    out
}

/// The posed-sticker row of a state (J02): `pose_index` i32 [177120], `poses` f32 [P, 4, 4] row-major acting on
/// column vectors, and `pose_lattice` i32 [P] with the K⁺ index of each pose or -1 off the lattice. The table is the
/// engine's canonical table, so equal states give equal index and lattice arrays.
pub fn posed(engine: &Engine, state: &State) -> Result<PosedInput, Missing> {
    let table = state.poses();
    let index: Vec<i32> = state.pose_ids().iter().map(|&id| id as i32).collect();
    let mut matrices = Vec::with_capacity(table.len() * 16);
    let mut lattice = Vec::with_capacity(table.len());
    for (id, pose) in table.iter().enumerate() {
        lattice.push(pose.kidx().map_or(-1, i32::from));
        for row in state.pose_matrix(engine, id as u32) {
            for entry in row {
                matrices.push(approximate(entry).ok_or(Missing::NonFinite { pose: id as u32 })?);
            }
        }
    }
    Ok(PosedInput {
        pose_index: Array::i32s(vec![NP as u64], &index),
        poses: Array::f32s(vec![table.len() as u64, 4, 4], &matrices),
        pose_lattice: Array::i32s(vec![table.len() as u64], &lattice),
    })
}

/// A KN0.2 revision of a fixture session with Jumble on, at state revision `number`: the identity block names the
/// state's menu, and the digest is the engine's canonical pose digest.
pub fn revision(engine: &Engine, state: &State, number: u64) -> RevisionRef {
    RevisionRef {
        session: "fixture".to_owned(),
        epoch: 1,
        identity: engine.identity(state.menu()),
        mode: Mode::JumbleOn,
        head: number,
        state: number,
        workspace: 1,
        protection: 0,
        digest: state.digest(engine),
    }
}

/// The engine bundle of a posed row at its revision (RPI.9): the three arrays, each bound to that state revision.
pub fn bundle(revision: &RevisionRef, posed: &PosedInput) -> EngineBundle {
    let arrays = bundle_names(Puzzle::Jumbling)
        .into_iter()
        .zip([&posed.pose_index, &posed.poses, &posed.pose_lattice])
        .map(|(name, array)| BoundArray { name: name.to_owned(), revision: revision.state, array: array.clone() })
        .collect();
    EngineBundle { revision: revision.clone(), arrays }
}

/// A twist's engine-named plane as display values (J03, render/1 section 4): the rows of RM.6's f32 [2, 4] plane
/// and the angle. The sweep R(φ) = I + (cos φ − 1)(uuᵀ + vvᵀ) + sin φ (vuᵀ − uvᵀ) built from them turns u toward v
/// and is the twist at φ = θ, up to binary32 rounding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweptPlane {
    /// u/|u| and v/|v| of `c600_jumble::twist::RotationPlane`.
    pub plane: [[f32; 4]; 2],
    /// θ in radians, in (0, π]; π for a half-turn, whose turning direction is the engine's naming of v.
    pub angle: f32,
}

/// The display values of `twist`'s engine-named plane ([`SweptPlane`], `Twist::plane`). The engine names u, v and
/// cos θ exactly, and u and v unnormalized; here each coordinate is evaluated in binary64 as [`approximate`] does,
/// each vector is normalized in binary64 and rounded to binary32, and θ is the binary64 arccosine of cos θ (clamped
/// to [−1, 1]) rounded to binary32. `Ok(None)` for the identity, which turns no plane; another matrix the engine names
/// no plane for is [`Missing::NoPlane`]; a vector or angle with no finite value is [`Missing::NonFinitePlane`].
/// Display data only: no direction is invented and nothing exact is decided from it.
pub fn swept_plane(twist: &Twist) -> Result<Option<SweptPlane>, Missing> {
    let Some(named) = twist.plane() else {
        let grip = twist.grip as u32;
        return if *twist.matrix == identity4() { Ok(None) } else { Err(Missing::NoPlane { grip }) };
    };
    let unit = |v: &Vec4| -> Option<[f32; 4]> {
        let x = [approximate64(&v[0])?, approximate64(&v[1])?, approximate64(&v[2])?, approximate64(&v[3])?];
        let norm = x.iter().map(|c| c * c).sum::<f64>().sqrt();
        let unit = x.map(|c| (c / norm) as f32);
        (norm.is_finite() && norm > 0.0 && unit.iter().all(|c| c.is_finite())).then_some(unit)
    };
    let swept = || {
        let (u, v, cos) = (unit(&named.u)?, unit(&named.v)?, approximate64(&named.cos)?);
        Some(SweptPlane { plane: [u, v], angle: cos.clamp(-1.0, 1.0).acos() as f32 })
    };
    swept().map(Some).ok_or(Missing::NonFinitePlane { grip: twist.grip as u32 })
}

/// The certified transform row of an applied twist (J03): the grip, the engine's certified inside set, sorted, and the
/// display values of the twist's engine-named plane and angle ([`swept_plane`]). `twist` must be the letter the outcome
/// certified. The outcome holds no matrix, so only the grip is checked: a twist of another grip is
/// [`Missing::OtherGrip`], and no plane is taken from it; another twist of the same grip is the caller's error, which
/// this cannot see. Only an admissible outcome has a row; a blocked, uncertain, refused or protected outcome has no
/// moving set, and none is guessed. The identity turns no plane: its row has none and the angle 0, so it has no swept
/// preview ([`crate::validate::swept_preview`]).
pub fn transform(outcome: &Outcome, twist: &Twist, endpoints: (u64, u64)) -> Result<Option<TransformInput>, Missing> {
    if twist.grip != outcome.grip {
        return Err(Missing::OtherGrip { outcome: outcome.grip as u32, twist: twist.grip as u32 });
    }
    if outcome.status != Status::Admissible {
        return Ok(None);
    }
    let swept = swept_plane(twist)?;
    Ok(Some(TransformInput {
        grip: outcome.grip as u32,
        plane: swept.map(|s| Array::f32s(vec![2, 4], &s.plane.concat())),
        angle: swept.map_or(0.0, |s| s.angle),
        moving_digest: piece_set_digest(&outcome.inside),
        moving: Array::u32s(vec![outcome.inside.len() as u64], &outcome.inside),
        from: endpoints.0,
        to: endpoints.1,
    }))
}

fn pair(certificate: &Certificate) -> Option<PointPair> {
    let point = |v: &Vec4| Some(CertPoint { exact: exact_text(v), vector: approximate4(v)? });
    Some(PointPair {
        piece: certificate.piece,
        points: [point(&certificate.point_below)?, point(&certificate.point_above)?],
    })
}

/// The named cut of a cap (J04): its pole, the exact plane as the pole normal and κ, and α as a rational. `None`
/// when the cap is not a pole or α is not a rational that fits.
pub fn named_cut(engine: &Engine, cap: usize) -> Option<NamedCut> {
    let model = &engine.model;
    let normal = model.poles.get(cap)?;
    let alpha = &model.alpha;
    let part = |n: String| n.parse::<i64>().ok();
    if part(alpha.b().to_string())? != 0 {
        return None;
    }
    let mut plane = exact_text(normal);
    plane.push(';');
    model.kappa.write_compact(&mut plane);
    Some(NamedCut { pole: cap as u32, plane, alpha: (part(alpha.a().to_string())?, part(alpha.d().to_string())?) })
}

/// The full named-cap straddler set of `cap` (J04): every certified straddling piece with both of its exactly
/// checked points and their converted vectors, and the named cut. A set the budget left incomplete is
/// [`Missing::Uncertain`]: no shorter set, and no single witness, stands for it. The survey is not part of it.
pub fn named_cap_set(
    engine: &Engine,
    state: &mut State,
    cap: usize,
    budget: Option<u64>,
) -> Result<CertOverlay, Missing> {
    let cut = named_cut(engine, cap).ok_or(Missing::NoSuchCap { cap: cap as u32 })?;
    let set = state.straddler_set(engine, cap, budget).map_err(|error| Missing::Engine(error.0))?;
    if set.uncertain > 0 {
        return Err(Missing::Uncertain { cap: cap as u32, pieces: set.uncertain });
    }
    let pairs = set
        .certificates
        .iter()
        .map(|certificate| {
            pair(certificate).ok_or(Missing::NonFinite { pose: state.pose_ids()[certificate.piece as usize] })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CertOverlay {
        grip: cap as u32,
        cap: cap as u32,
        kind: CertKind::FullNamedCapSet,
        set_digest: piece_set_digest(&set.pieces),
        pieces: set.pieces,
        pairs,
        survey: Vec::new(),
        cut: Some(cut),
    })
}

/// The cap-map witness of a blocked classification (J04): its one straddling piece with both points. It is not the
/// full named-cap set and cannot replace it. `None` when the classification found no straddler.
pub fn witness(classification: &Classification) -> Option<CertOverlay> {
    let certificate = classification.straddle.as_ref()?;
    Some(CertOverlay {
        grip: classification.grip as u32,
        cap: classification.grip as u32,
        kind: CertKind::CapMapWitness,
        pieces: vec![certificate.piece],
        set_digest: piece_set_digest(&[certificate.piece]),
        pairs: vec![pair(certificate)?],
        survey: Vec::new(),
        cut: None,
    })
}

/// The survey status of an engine status: certified a or b, and anything else uncertain. Nothing rounds an
/// uncertain grip to a or b.
pub fn grip_status(status: Status) -> GripStatus {
    match status {
        Status::Admissible => GripStatus::A,
        Status::Blocked => GripStatus::B,
        _ => GripStatus::Uncertain,
    }
}

/// The seam table of a `seam_facts` answer that lists its rows (J08): one entry per mismatched cell, on-grip and
/// off-grip alike, in the engine's order, and complete only when the engine's batch was. A domain is named by the
/// rank of its exact key among the table's domains and a cell by the rank of its chamber pair among the table's
/// cells: identities local to this table, enough to keep entries distinct. `None` when the answer is not for scope
/// `all` (a cap's answer counts its N_c, not total N), lists no rows or a row lacks a field; nothing is filled in.
pub fn seam_table(facts: &Value) -> Option<SeamTable> {
    if facts.get("scope").and_then(Value::as_str) != Some("all") {
        return None;
    }
    let complete = facts.get("complete") == Some(&Value::Bool(true));
    let mut rows = Vec::new();
    for row in facts.get("rows")?.as_array()? {
        if row.get("aligned") == Some(&Value::Bool(true)) {
            continue;
        }
        let domain = row.get("domain")?.as_str()?;
        let cell = row.get("cell")?.as_array()?;
        let cell = (cell.first()?.as_i64()?, cell.get(1)?.as_i64()?);
        let on_grip = match row.get("grip")?.as_str()? {
            "on" => true,
            "off" => false,
            _ => return None,
        };
        rows.push((domain, cell, on_grip));
    }
    let mut domains: Vec<&str> = rows.iter().map(|row| row.0).collect();
    domains.sort_unstable();
    domains.dedup();
    let mut cells: Vec<(i64, i64)> = rows.iter().map(|row| row.1).collect();
    cells.sort_unstable();
    cells.dedup();
    let seams = rows
        .iter()
        .map(|&(domain, cell, on_grip)| {
            Some(Seam {
                domain: domains.binary_search(&domain).ok()? as u32,
                cell: cells.binary_search(&cell).ok()? as u32,
                on_grip,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(SeamTable { seams, complete })
}

/// J10's exposed-face geometry (JI.41 "Exposed-face meshes", JI.45) in the instanced layout
/// `magic600-jumbling-exposed-faces/1`, a payload and not a supported row (RM.15). Every array is a copy of what M5's
/// certified producer (`c600_jumble::faces`) derived from `assets/`, with exact values converted to binary32 as
/// [`approximate`] does; nothing is generated from a mesh, a float test or another frame.
///
/// - `base_vertices` f32 [V, 4]: the vertices of each certified base cut face, face after face, in the face's
///   ascending region order, in the representative's home frame and asset units; `base_vertex_offsets` u32 [B + 1]
///   cuts them into the B base faces.
/// - `base_triangles` u32 [T, 3]: each base face's fan triangles over its 2-faces, indices into `base_vertices` inside
///   the face's range, wound by the exact orientation of `c600_jumble::faces`; `base_triangle_offsets` u32 [B + 1]
///   cuts them into the base faces.
/// - `transports` f32 [7200, 4, 4]: the K⁺ matrices, row-major acting on column vectors as J02's poses are;
///   `piece_transport` u32 [177120]: the element carrying each piece's orbit representative to the piece. A face's
///   home-frame vertices are its piece's transport applied to its base face's, and J02's pose moves them as it moves
///   the piece's stickers.
/// - The face table, one row per cut face of every piece (F rows): `face_piece`, `face_base` (index of the base face),
///   `face_pole` (the home pole of its cut hyperplane), each u32 [F]; `face_side` u8 [F], 1 for `in` (the pole is in
///   the piece's signature, so the piece is the cell's upper chamber) and 0 for `out`; `face_cell` u32 [F, 2], the
///   cell's chambers [lower, upper] by piece ID, 177120 for the core. J08's seam rows name cells by the same chamber
///   pair.
#[derive(Clone, Debug, PartialEq)]
pub struct ExposedFaces {
    /// The exact-source digest: SHA-256 of the certified canonical exact encoding of the base table.
    pub geometry_digest: String,
    /// The ID of the admitted size profile (`c600_jumble::faces::PROFILE`).
    pub profile_id: String,
    /// The exact input: the digest of the state the producer was asked about.
    pub state_digest: String,
    pub base_vertices: Array,
    pub base_vertex_offsets: Array,
    pub base_triangles: Array,
    pub base_triangle_offsets: Array,
    pub transports: Array,
    pub piece_transport: Array,
    pub face_piece: Array,
    pub face_base: Array,
    pub face_pole: Array,
    pub face_side: Array,
    pub face_cell: Array,
}

impl ExposedFaces {
    /// The arrays and their names, in layout order.
    pub fn arrays(&self) -> [(&'static str, &Array); 11] {
        [
            ("base_vertices", &self.base_vertices),
            ("base_vertex_offsets", &self.base_vertex_offsets),
            ("base_triangles", &self.base_triangles),
            ("base_triangle_offsets", &self.base_triangle_offsets),
            ("transports", &self.transports),
            ("piece_transport", &self.piece_transport),
            ("face_piece", &self.face_piece),
            ("face_base", &self.face_base),
            ("face_pole", &self.face_pole),
            ("face_side", &self.face_side),
            ("face_cell", &self.face_cell),
        ]
    }
}

fn faces_mismatch(summary: String) -> Error {
    Error::new(codes::BUNDLE_MISMATCH, summary).with("feature", "J10")
}

/// J10's exposed faces at `state`: [`exposed_faces_from`] over the answer of M5's producer,
/// `c600_jumble::faces::exposed_faces`.
pub fn exposed_faces(engine: &Engine, state: &State) -> Result<ExposedFaces, Error> {
    exposed_faces_from(engine, state, &c600_jumble::faces::exposed_faces(engine, state))
}

/// J10's exposed faces at `state` from the producer's `answer` (JI.45). An available answer is bound to the certified
/// geometry through [`crosswalk::exposed_geometry`]: its source must name `state`, its geometry digest the SHA-256 of
/// the certified bytes, and its profile the admitted one; the arrays are then built from the certified table and must
/// pass [`check_exposed_faces`]. Any other binding is `EV3:input.display_bundle_mismatch` and nothing is built. A
/// refused answer, or any other status, is `KN2:input.exposed_geometry_unavailable` from
/// [`crosswalk::exposed_geometry`] with what the producer named as missing (both when it named nothing), the state
/// digest the producer was asked about (`source`) and its failed fact (`failed`); a refusal that names an input J10
/// does not have is a binding failure. Nothing is drawn, and no mesh, face or size is invented.
pub fn exposed_faces_from(engine: &Engine, state: &State, answer: &Value) -> Result<ExposedFaces, Error> {
    use c600_jumble::faces;
    let field = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    let source = answer.get("source").and_then(|source| field(source, "state_digest"));
    if field(answer, "status").as_deref() != Some("available") {
        let listed = answer.get("missing").and_then(Value::as_array);
        let named: Vec<&str> = listed.map_or(Vec::new(), |m| m.iter().filter_map(Value::as_str).collect());
        if named.iter().any(|what| !faces::MISSING.contains(what)) {
            return Err(faces_mismatch("the producer's refusal names an input J10 does not have".to_owned()));
        }
        let missing = |what: &str| named.is_empty() || named.contains(&what);
        let certified = if missing(faces::MISSING_GEOMETRY) { None } else { faces::geometry(engine).ok() };
        let profile = (!missing(faces::MISSING_PROFILE)).then(|| faces::PROFILE.id());
        let bytes = certified.map(|geometry| geometry.canonical.as_slice());
        let mut error = match crosswalk::exposed_geometry(bytes, profile.as_deref()) {
            Err(error) => error,
            Ok(_) => crosswalk::exposed_geometry(None, None).expect_err("nothing is supplied"),
        };
        if let Some(digest) = source {
            error = error.with("source", digest);
        }
        if let Some(failed) = field(answer, "failed") {
            error = error.with("failed", failed);
        }
        return Err(error);
    }
    let state_digest = state.digest(engine);
    if source.as_deref() != Some(state_digest.as_str()) {
        return Err(faces_mismatch("the producer's answer names another state".to_owned()));
    }
    let geometry =
        faces::geometry(engine).map_err(|reason| faces_mismatch(format!("no certified geometry: {reason}")))?;
    let profile_id = faces::PROFILE.id();
    if field(answer, "profile_id").as_deref() != Some(profile_id.as_str())
        || answer.get("profile") != Some(&faces::PROFILE.to_json())
        || geometry.profile != faces::PROFILE
    {
        return Err(faces_mismatch("the answer's size profile is not the admitted one".to_owned()));
    }
    let geometry_digest = crosswalk::exposed_geometry(Some(&geometry.canonical), Some(&profile_id))?;
    if answer.get("geometry").and_then(|g| field(g, "digest")).as_deref() != Some(geometry_digest.as_str()) {
        return Err(faces_mismatch("the answer's geometry digest is not the certified bytes'".to_owned()));
    }

    let table = &geometry.table.faces;
    let (mut vertices, mut vertex_offsets) = (Vec::new(), vec![0u32]);
    let (mut triangles, mut triangle_offsets) = (Vec::new(), vec![0u32]);
    for (b, face) in table.iter().enumerate() {
        let start = (vertices.len() / 4) as u32;
        let region = &engine.regions.rep_vertices[face.orbit as usize];
        for &v in &face.vertices {
            let x = approximate4(&region[v as usize]);
            vertices.extend(x.ok_or_else(|| faces_mismatch(format!("base face {b}: a vertex has no finite value")))?);
        }
        for corner in face.fan().into_iter().flatten() {
            let at = face.vertices.binary_search(&corner);
            let at = at.map_err(|_| faces_mismatch(format!("base face {b}: a triangle corner off the face")))?;
            triangles.push(start + at as u32);
        }
        vertex_offsets.push((vertices.len() / 4) as u32);
        triangle_offsets.push((triangles.len() / 3) as u32);
    }
    let mut transports = Vec::with_capacity(ORDER * 16);
    for k in 0..ORDER {
        for entry in engine.kplus.matrix(k as u16, &engine.model).iter().flatten() {
            let x = approximate(entry);
            transports.push(x.ok_or_else(|| faces_mismatch(format!("K+ element {k} has no finite value")))?);
        }
    }
    let piece_transport: Vec<u32> = engine.regions.transport.iter().map(|&k| u32::from(k)).collect();
    let rows = geometry.profile.full_cut().faces3 as usize;
    let (mut piece, mut base, mut pole) =
        (Vec::with_capacity(rows), Vec::with_capacity(rows), Vec::with_capacity(rows));
    let (mut side, mut cell) = (Vec::with_capacity(rows), Vec::with_capacity(2 * rows));
    for p in 0..NP {
        for face in geometry.piece_faces(engine, p).map_err(faces_mismatch)? {
            piece.push(face.piece);
            base.push(face.base_index);
            pole.push(u32::from(face.pole));
            side.push(u8::from(face.side == faces::Side::In));
            cell.extend(face.cell);
        }
    }
    let (b, f) = (table.len() as u64, piece.len() as u64);
    let exposed = ExposedFaces {
        geometry_digest,
        profile_id,
        state_digest,
        base_vertices: Array::f32s(vec![vertices.len() as u64 / 4, 4], &vertices),
        base_vertex_offsets: Array::u32s(vec![b + 1], &vertex_offsets),
        base_triangles: Array::u32s(vec![triangles.len() as u64 / 3, 3], &triangles),
        base_triangle_offsets: Array::u32s(vec![b + 1], &triangle_offsets),
        transports: Array::f32s(vec![ORDER as u64, 4, 4], &transports),
        piece_transport: Array::u32s(vec![NP as u64], &piece_transport),
        face_piece: Array::u32s(vec![f], &piece),
        face_base: Array::u32s(vec![f], &base),
        face_pole: Array::u32s(vec![f], &pole),
        face_side: Array::u8s(vec![f], &side),
        face_cell: Array::u32s(vec![f, 2], &cell),
    };
    check_exposed_faces(&exposed)?;
    Ok(exposed)
}

/// The host-side check of a J10 payload against the admitted size profile (`c600_jumble::faces::PROFILE`), which it
/// trusts for the counts: the payload names that profile's ID; every array's descriptor, element type and shape (the
/// base cut faces' vertices, triangles and faces, the 7,200 K⁺ elements, the 177,120 pieces and the full table's
/// faces) and their byte total; offsets that start at 0, never decrease and end at their array's rows, base faces of
/// at least four vertices whose triangle corners lie in their own range, finite binary32 values, transports in K⁺,
/// and face rows in piece order that name a piece, base face and pole of the model with side 0 or 1 and a cell whose
/// chamber on the face's side is its piece. The check is structural: it does not recompute a row's pole, base face or
/// other chamber, which the payload carries from the certified table that [`exposed_faces_from`] binds to the answer.
/// Returns the byte total; any failure is `EV3:input.display_bundle_mismatch` naming J10, and nothing is repaired.
pub fn check_exposed_faces(exposed: &ExposedFaces) -> Result<u64, Error> {
    let profile = &c600_jumble::faces::PROFILE;
    if exposed.profile_id != profile.id() {
        return Err(faces_mismatch("the payload names another size profile".to_owned()));
    }
    let (base, full) = (profile.base_cut(), profile.full_cut());
    let (b, v, t, f) = (base.faces3, base.vertices, base.triangles, full.faces3);
    let shapes: [(Elem, Vec<u64>); 11] = [
        (Elem::F32, vec![v, 4]),
        (Elem::U32, vec![b + 1]),
        (Elem::U32, vec![t, 3]),
        (Elem::U32, vec![b + 1]),
        (Elem::F32, vec![ORDER as u64, 4, 4]),
        (Elem::U32, vec![NP as u64]),
        (Elem::U32, vec![f]),
        (Elem::U32, vec![f]),
        (Elem::U32, vec![f]),
        (Elem::U8, vec![f]),
        (Elem::U32, vec![f, 2]),
    ];
    let mut total = 0;
    for ((name, array), (elem, shape)) in exposed.arrays().into_iter().zip(shapes) {
        let want: Vec<Option<u64>> = shape.into_iter().map(Some).collect();
        array.expect(name, elem, &want).map_err(|e| faces_mismatch(e.summary))?;
        total += array.desc.byte_len;
    }
    if total != profile.instanced_bytes() {
        return Err(faces_mismatch(format!("{total} bytes, not the profile's {}", profile.instanced_bytes())));
    }
    for (name, array) in [("base_vertices", &exposed.base_vertices), ("transports", &exposed.transports)] {
        if !array.as_f32().iter().all(|x| x.is_finite()) {
            return Err(faces_mismatch(format!("{name}: a value is not finite")));
        }
    }
    let offsets = |name: &str, array: &Array, end: u64| -> Result<Vec<u32>, Error> {
        let o = array.as_u32();
        let ordered = o.first() == Some(&0) && o.windows(2).all(|w| w[0] <= w[1]) && o.last() == Some(&(end as u32));
        ordered.then_some(o).ok_or_else(|| faces_mismatch(format!("{name}: not offsets from 0 to {end}")))
    };
    let vo = offsets("base_vertex_offsets", &exposed.base_vertex_offsets, v)?;
    let to = offsets("base_triangle_offsets", &exposed.base_triangle_offsets, t)?;
    let corners = exposed.base_triangles.as_u32();
    for face in 0..b as usize {
        let range = vo[face]..vo[face + 1];
        let own = &corners[3 * to[face] as usize..3 * to[face + 1] as usize];
        if range.len() < 4 || !own.iter().all(|c| range.contains(c)) {
            return Err(faces_mismatch(format!("base face {face}: a triangle corner outside its face")));
        }
    }
    if exposed.piece_transport.as_u32().iter().any(|&k| k as usize >= ORDER) {
        return Err(faces_mismatch("piece_transport: an element outside K+".to_owned()));
    }
    let (piece, base_of, pole) = (exposed.face_piece.as_u32(), exposed.face_base.as_u32(), exposed.face_pole.as_u32());
    let (side, cell) = (&exposed.face_side.bytes, exposed.face_cell.as_u32());
    for row in 0..f as usize {
        let p = piece[row];
        let (lower, upper) = (cell[2 * row], cell[2 * row + 1]);
        let held = match side[row] {
            1 => upper == p && lower <= NP as u32,
            0 => lower == p && upper < NP as u32,
            _ => false,
        };
        let ordered = row == 0 || piece[row - 1] <= p;
        if p as usize >= NP || base_of[row] as u64 >= b || pole[row] as usize >= POLES || !held || !ordered {
            return Err(faces_mismatch(format!("face row {row}: not a face of a piece of the model")));
        }
    }
    Ok(total)
}

/// RM.12's fixed admission check of J10's payload against `ceiling`, the componentwise minimum of the host's and the
/// backend's profiles: after [`check_exposed_faces`], the payload's `input_bytes`, the sum of its arrays' descriptor
/// byte lengths (RM.11). At the ceiling it passes and returns the bytes; above it is
/// `EV3:resource_limit.display_plan_ceiling` with the field, the requested count and the ceiling, and nothing is
/// truncated. No numeric ceiling is fixed yet: RM.11 asks for a `LimitProfile` only before a feature is advertised as
/// supported, J10 is not (RM.15), and RM.13 leaves production ceilings to be pinned before enablement, so only declared
/// fixture profiles reach this check.
pub fn admit_exposed_faces(exposed: &ExposedFaces, ceiling: &LimitProfile) -> Result<u64, Error> {
    let bytes = check_exposed_faces(exposed)?;
    if bytes > ceiling.input_bytes {
        return Err(Error::ceiling("input_bytes", bytes, ceiling.input_bytes));
    }
    Ok(bytes)
}

/// The exact projection to slots of one jumbling state (RPI.23, J13). A sticker of a piece at the lattice pose k is
/// in the slot k·s of its home slot s: M8's slot action of K⁺ with A3's frame check (`levels::Tables::piece_slots`),
/// which at the identity is the home slot. A piece off the lattice has no slot, and none is invented: the answer is
/// `undefined` with reason `off_lattice`. A piece, as on 600-cell-Full, has no single slot. It holds the pose level
/// only: a label-level subject gets [`codes::LEVEL_NOT_HELD`] before any digest is compared (KN1.8).
pub struct PoseProjection {
    digest: String,
    /// Whether each piece's pose is on the lattice.
    lattice: Vec<bool>,
    /// The slot of each sticker, by its home slot; `None` for a sticker of a piece off the lattice.
    slots: Vec<Option<u32>>,
}

/// The reason of a slot request for a piece off the lattice.
pub const OFF_LATTICE: &str = "off_lattice";

impl PoseProjection {
    /// The projection for `state`, bound to its canonical pose digest. The slots of a piece at a lattice pose other
    /// than the identity come from M8's level tables; an engine error there, such as a frame disagreement, is
    /// [`Missing::Engine`] and no projection, never an answer.
    pub fn new(engine: &Engine, state: &State) -> Result<PoseProjection, Missing> {
        let poses: Vec<Option<u16>> = (0..NP).map(|piece| state.pose_of(piece).kidx()).collect();
        let mut slots: Vec<Option<u32>> = (0u32..)
            .zip(&engine.model.slot_piece)
            .map(|(home, &piece)| (poses.get(piece as usize) == Some(&Some(0))).then_some(home))
            .collect();
        let turned: Vec<(usize, u16)> =
            poses.iter().enumerate().filter_map(|(piece, &k)| k.filter(|&k| k != 0).map(|k| (piece, k))).collect();
        if !turned.is_empty() {
            let tables = levels::tables(engine).map_err(|error| Missing::Engine(error.0))?;
            for (piece, k) in turned {
                let placed = tables.piece_slots(engine, piece, k).map_err(|error| Missing::Engine(error.0))?;
                for (&home, slot) in tables.slots(piece).iter().zip(placed) {
                    slots[home as usize] = Some(slot);
                }
            }
        }
        Ok(PoseProjection { digest: state.digest(engine), lattice: poses.iter().map(Option::is_some).collect(), slots })
    }

    fn piece(&self, piece: u32) -> Partial<u32> {
        match self.lattice.get(piece as usize) {
            None => Partial::Undefined("no such piece".to_owned()),
            Some(false) => Partial::Undefined(OFF_LATTICE.to_owned()),
            Some(true) => Partial::Undefined("a piece has no single slot".to_owned()),
        }
    }
}

impl Projection for PoseProjection {
    fn slot(&self, subject: &StateRef, object: &ObjectId) -> Partial<u32> {
        if subject.level() != Level::Poses {
            return Partial::Undefined(codes::LEVEL_NOT_HELD.to_owned());
        }
        if subject.digest() != self.digest {
            return Partial::Undefined("the projection is for another subject".to_owned());
        }
        match object {
            ObjectId::Slot(slot) => Partial::Total(*slot),
            ObjectId::Sticker(home) => match self.slots.get(*home as usize) {
                Some(Some(slot)) => Partial::Total(*slot),
                Some(None) => Partial::Undefined(OFF_LATTICE.to_owned()),
                None => Partial::Undefined("no such sticker".to_owned()),
            },
            ObjectId::Piece(piece) | ObjectId::Point { witness: piece, .. } => self.piece(*piece),
            _ => Partial::Undefined("the object has no slot".to_owned()),
        }
    }
}

/// The shrink anchors of J01 (render/1 section 2; `work/experiments/renderer-sb/SPEC.md` section 3; owner decision of
/// 10 October 2026): for each base sticker, the area-weighted centroid of its triangles, as f32 [433, 4]. The
/// triangles of sticker s are the consecutive vertex triples of the mesh vertices in `[offsets[s], offsets[s + 1])`.
/// Every f32 component is widened to binary64 and all arithmetic is binary64 in SPEC's order (each dot product sums
/// components 0 to 3, each sum runs in triangle order), with one rounding to binary32 at the end. The anchors move with
/// the sticker, so a piece's shrunk stickers land on the shrunk stickers of the slots it reaches; the numbering centres
/// (`mesh_centers.f32`) are not anchors. A mesh of the wrong shape, offsets that do not cut it into triangles, or a
/// sticker with no finite positive area or no finite anchor refuse the assets with `EV3:input.display_bundle_mismatch`;
/// nothing is regenerated.
pub fn shrink_anchors(geometry: &GeometryInput) -> Result<Array, Error> {
    let mismatch = |summary: String| Error::new(codes::BUNDLE_MISMATCH, summary).with("feature", "J01");
    geometry.vertices.expect("mesh vertices", Elem::F32, &[None, Some(4)]).map_err(|e| mismatch(e.summary))?;
    geometry.offsets.expect("mesh offsets", Elem::U32, &[None]).map_err(|e| mismatch(e.summary))?;
    let (vertices, offsets) = (geometry.vertices.as_f32(), geometry.offsets.as_u32());
    let count = vertices.len() / 4;
    let vertex = |k: usize| -> [f64; 4] { std::array::from_fn(|i| f64::from(vertices[k * 4 + i])) };
    let dot = |x: &[f64; 4], y: &[f64; 4]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2] + x[3] * y[3];
    let mut anchors = Vec::with_capacity(offsets.len().saturating_sub(1) * 4);
    for (local, range) in offsets.windows(2).enumerate() {
        let (begin, end) = (range[0] as usize, range[1] as usize);
        if begin > end || end > count || (end - begin) % 3 != 0 {
            return Err(mismatch(format!("sticker {local}: offsets {begin}..{end} are not triangles of the mesh")));
        }
        let (mut total, mut weighted) = (0f64, [0f64; 4]);
        for t in (begin..end).step_by(3) {
            let (a, b, c) = (vertex(t), vertex(t + 1), vertex(t + 2));
            let e1: [f64; 4] = std::array::from_fn(|i| b[i] - a[i]);
            let e2: [f64; 4] = std::array::from_fn(|i| c[i] - a[i]);
            let gram = dot(&e1, &e1) * dot(&e2, &e2) - dot(&e1, &e2) * dot(&e1, &e2);
            let area = 0.5 * gram.max(0.0).sqrt();
            total += area;
            for (i, w) in weighted.iter_mut().enumerate() {
                *w += area * ((a[i] + b[i] + c[i]) / 3.0);
            }
        }
        let anchor = weighted.map(|w| w / total);
        if !(total.is_finite() && total > 0.0) || !anchor.iter().all(|x| x.is_finite()) {
            return Err(mismatch(format!("sticker {local}: no finite positive triangle area")));
        }
        anchors.extend(anchor.map(|x| x as f32));
    }
    let stickers = (anchors.len() / 4) as u64;
    Ok(Array::f32s(vec![stickers, 4], &anchors))
}

/// The first-frame readback check of one revision (J06, render/1 section 7, JI.41 "Revision adoption and
/// readback"): the bound arrays a backend read back, compared with the engine bundle's by SHA-256 and element by
/// element. It carries its revision, device and frame stamps. It is the adapter's check record, not RM.26's
/// presentation confirmation: a failed check leaves the display out of date and changes no committed receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Readback {
    pub revision: u64,
    pub device: u64,
    pub frame: u64,
    /// The bundle's arrays whose readback is absent, repeated, of another revision, shape or element type, or differs
    /// by digest or by element, in the bundle's order.
    pub failed: Vec<String>,
}

impl Readback {
    /// Whether all three arrays matched, so the frame shows the revision it claims.
    pub fn passed(&self) -> bool {
        self.failed.is_empty()
    }

    /// Whether this check speaks of the current revision on the current device; a stale check is discarded.
    pub fn current(&self, revision: u64, device: u64) -> bool {
        self.revision == revision && self.device == device
    }
}

/// J06's check of `readback` against `bundle` in frame `frame` on device `device`. Every array the bundle's puzzle
/// binds is compared: exactly one readback of that name at the bundle's revision, with the engine array's element type
/// and shape, the engine array's digest and the same elements. Nothing is adopted, retried or repaired here.
pub fn readback_check(bundle: &EngineBundle, readback: &[BoundArray], device: u64, frame: u64) -> Readback {
    let revision = bundle.revision.state;
    let matches = |name: &str| {
        let engine: Vec<&BoundArray> = bundle.arrays.iter().filter(|a| a.name == name).collect();
        let read: Vec<&BoundArray> = readback.iter().filter(|a| a.name == name).collect();
        let ([engine], [read]) = (engine.as_slice(), read.as_slice()) else {
            return false;
        };
        let (want, have) = (&engine.array, &read.array);
        engine.revision == revision
            && read.revision == revision
            && have.desc.elem == want.desc.elem
            && have.desc.shape == want.desc.shape
            && sha256_hex(&have.bytes) == want.desc.digest
            && have.bytes == want.bytes
    };
    let failed =
        bundle_names(bundle.revision.identity.puzzle).into_iter().filter(|name| !matches(name)).map(str::to_owned);
    Readback { revision, device, frame, failed: failed.collect() }
}

/// The jumbling rows the fixture report supports: posed stickers, certified transforms, certified overlays and the
/// projection chain.
pub const ROWS: [&str; 4] = ["J02", "J03", "J04", "J05"];

/// A fixture support report for the jumbling puzzle: the general rows of [`produce::support_report`], with J02 to
/// J05 supported with `limits` and the 600-cell-Full rows they replace (F01, F03, F04) reported unsupported. Rows it
/// omits (J01, J06 to J12, F25, F27) are unsupported (RM.15); J13 is the jumbling binding of F18.
pub fn support_report(limits: &LimitProfile) -> SupportReport {
    let mut report = produce::support_report(limits);
    for (row, replacement) in [("F01", "J02"), ("F03", "J03"), ("F04", "J05")] {
        let refusal = Error::unsupported(
            row,
            format!("a 600-cell-Full row; the jumbling puzzle uses {replacement} in its place"),
        );
        report.features.insert(row.to_owned(), Support::Unsupported(refusal));
    }
    for row in ROWS {
        let stages = if row == "J05" {
            produce::STAGES.iter().map(|&(name, params)| produce::stage_def(name, params)).collect()
        } else {
            Vec::new()
        };
        report
            .features
            .insert(row.to_owned(), Support::Supported { layout_version: 1, limits: limits.clone(), stages });
    }
    report
}

/// A declared host context for the jumbling puzzle at d = 4 with that identity block. The geometry binding is the
/// unchanged, verified geometry of [`GeometryInput`]; J01's shrink anchors are not among it.
pub fn fixture_context(geometry: &GeometryInput, identity: IdentityBlock, limits: LimitProfile) -> Context {
    Context {
        identity,
        dimension: 4,
        geometry: geometry.digests(),
        support: support_report(&limits),
        limits,
        tokens: produce::token_table(),
        words: produce::word_records(),
        checks: Vec::new(),
    }
}
