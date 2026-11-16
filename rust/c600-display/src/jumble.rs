//! Producers for the jumbling puzzle (RPI.15, RM.34 rows J02 to J05 and J13), built from M5's kernel
//! (`c600_jumble`). They copy what the engine certified and convert exact values to binary32 for display; they
//! classify nothing, infer no inside set, plane or slot, and truncate no set. What the engine does not supply is an
//! explicit [`Missing`] value or an `undefined` projection, never a guessed payload.
//!
//! - [`posed`] builds the posed-sticker row (J02, render/1 section 3) from a state's canonical pose table;
//!   [`revision`] and [`bundle`] bind it to its pose-level revision (RPI.9, render/1 section 7).
//! - [`transform`] builds the certified transform row (J03) from an applied twist's certified inside set.
//! - [`named_cap_set`], [`witness`] and [`named_cut`] build the certified overlay row (J04).
//! - [`seam_table`] reads the seam table of J08 from the engine's `seam_facts` answer.
//! - [`PoseProjection`] is the kernel's exact projection to slots for a jumbling subject (RPI.23, J13).
//! - [`support_report`] and [`fixture_context`] are declared fixture values, as in [`crate::produce`] (RPI.50).

use c600_jumble::model::NP;
use c600_jumble::q5::{Vec4, Q5};
use c600_jumble::state::{Certificate, Classification, Outcome, State, Status};
use c600_jumble::Context as Engine;
use c600_kernel::json::Value;
use c600_kernel::{IdentityBlock, Puzzle};

use crate::array::Array;
use crate::crosswalk::{Seam, SeamTable};
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
}

/// The binary32 display value of an exact number: (a + b√5)/d evaluated in binary64, each operation rounded, then
/// rounded to binary32, so it may differ from the correctly rounded value in the last place. `None` when a
/// coefficient does not fit binary64 or the value is not finite. It is display data only; nothing exact is decided
/// from it.
pub fn approximate(x: &Q5) -> Option<f32> {
    let part = |n: String| n.parse::<f64>().ok();
    let (a, b, d) = (part(x.a().to_string())?, part(x.b().to_string())?, part(x.d().to_string())?);
    let value = ((a + b * 5f64.sqrt()) / d) as f32;
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

/// The certified transform row of an applied twist (J03): the grip and the engine's certified inside set, sorted.
/// Only an admissible outcome has one; a blocked, uncertain, refused or protected outcome has no moving set, and none
/// is guessed. The plane is the one the engine names; `c600_jumble` names none, so without it there is no swept
/// preview ([`crate::validate::swept_preview`]).
pub fn transform(
    outcome: &Outcome,
    plane: Option<[[f32; 4]; 2]>,
    angle: f32,
    endpoints: (u64, u64),
) -> Option<TransformInput> {
    if outcome.status != Status::Admissible {
        return None;
    }
    Some(TransformInput {
        grip: outcome.grip as u32,
        plane: plane.map(|p| Array::f32s(vec![2, 4], &p.concat())),
        angle,
        moving_digest: piece_set_digest(&outcome.inside),
        moving: Array::u32s(vec![outcome.inside.len() as u64], &outcome.inside),
        from: endpoints.0,
        to: endpoints.1,
    })
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

/// The exact projection to slots of one jumbling state (RPI.23, J13). A sticker of a piece at the identity pose is
/// in its home slot. A piece off the lattice has no slot, and none is invented: the answer is `undefined` with reason
/// `off_lattice`. A piece at another lattice pose has a slot, but `c600_jumble` offers no slot action of K⁺, so the
/// answer is `undefined` with its own reason, distinct from `off_lattice`. It holds the pose level only: a label-level
/// subject gets [`codes::LEVEL_NOT_HELD`] before any digest is compared (KN1.8).
pub struct PoseProjection {
    digest: String,
    slot_piece: Vec<u32>,
    /// The K⁺ index of each piece's pose, or -1 off the lattice.
    lattice: Vec<i32>,
}

/// The reason of a slot request for a piece off the lattice.
pub const OFF_LATTICE: &str = "off_lattice";

impl PoseProjection {
    /// The projection for `state`, bound to its canonical pose digest.
    pub fn new(engine: &Engine, state: &State) -> PoseProjection {
        PoseProjection {
            digest: state.digest(engine),
            slot_piece: engine.model.slot_piece.clone(),
            lattice: (0..NP).map(|piece| state.pose_of(piece).kidx().map_or(-1, i32::from)).collect(),
        }
    }

    fn piece(&self, piece: u32, home: Option<u32>) -> Partial<u32> {
        match (self.lattice.get(piece as usize), home) {
            (None, _) => Partial::Undefined("no such piece".to_owned()),
            (Some(-1), _) => Partial::Undefined(OFF_LATTICE.to_owned()),
            (Some(0), Some(home)) => Partial::Total(home),
            (Some(_), Some(_)) => Partial::Undefined("the kernel offers no slot action for a lattice pose".to_owned()),
            (Some(_), None) => Partial::Undefined("a piece has no single slot".to_owned()),
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
            ObjectId::Sticker(home) => match self.slot_piece.get(*home as usize) {
                Some(&piece) => self.piece(piece, Some(*home)),
                None => Partial::Undefined("no such sticker".to_owned()),
            },
            ObjectId::Piece(piece) | ObjectId::Point { witness: piece, .. } => self.piece(*piece, None),
            _ => Partial::Undefined("the object has no slot".to_owned()),
        }
    }
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
