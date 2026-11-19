//! Producers and fixtures for 600-cell-Full (RM.6, RM.27). `GeometryInput` loads the unchanged geometry assets and
//! checks each digest and dimension against `assets/manifest.json`; [`retained`] and [`transform`] build RM.6's
//! `RetainedInput` and `TransformInput` from kernel values; [`fixture_context`] is a declared host context for
//! source-level tests. Its support report and ceilings are declared fixture values, not measured ones (RPI.50), and
//! its stage signatures are fixtures, not the checked library's definitions.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value};
use c600_kernel::{Effect, IdentityBlock, LoadError, Snapshot, SLOTS};

use crate::array::{Array, Elem};
use crate::error::Error;
use crate::plan::{piece_set_digest, Key, Mode, RetainedInput, RevisionRef, TransformInput};
use crate::validate::{Context, LimitProfile, StageDef, Support, SupportReport, TokenTable, WordRecord};

/// The base mesh vertices of `mesh_vertices.f32`.
pub const VERTICES: u64 = 30480;
/// The entries of the manifest's `mesh.offsets`.
pub const OFFSETS: u64 = 434;

/// RM.6's `GeometryInput`: the model identity, the manifest digest and the unchanged arrays, each checked.
#[derive(Clone, Debug)]
pub struct GeometryInput {
    pub identity: IdentityBlock,
    /// The SHA-256 of `assets/manifest.json`.
    pub manifest: String,
    /// f32 [30480, 4].
    pub vertices: Array,
    /// u32 [30480].
    pub sticker: Array,
    /// f32 [600, 4, 4], column-major.
    pub frames: Array,
    /// u32 [434], from the manifest's `mesh.offsets`.
    pub offsets: Array,
    /// u32 [259800]: the piece of each slot.
    pub slot_piece: Array,
}

impl GeometryInput {
    /// Loads and checks the geometry in the `assets` folder. A digest or size mismatch is refused; nothing is
    /// regenerated.
    pub fn load(assets: &Path) -> Result<GeometryInput, LoadError> {
        let text = fs::read(assets.join("manifest.json")).map_err(|e| LoadError(format!("manifest.json: {e}")))?;
        let manifest = json::parse(&text).map_err(|_| LoadError("manifest.json is not JSON".to_owned()))?;
        let model_id = manifest
            .get("model_id")
            .and_then(Value::as_str)
            .ok_or_else(|| LoadError("manifest.json names no model_id".to_owned()))?;
        let files = manifest
            .get("files")
            .and_then(Value::as_object)
            .ok_or_else(|| LoadError("manifest.json lists no files".to_owned()))?;
        let read = |name: &str, elem: Elem, shape: Vec<u64>| -> Result<Array, LoadError> {
            let listed = files
                .get(&format!("assets/{name}"))
                .and_then(Value::as_str)
                .ok_or_else(|| LoadError(format!("{name} is not listed in manifest.json")))?;
            let bytes = fs::read(assets.join(name)).map_err(|e| LoadError(format!("{name}: {e}")))?;
            let array = Array::from_bytes(elem, shape, bytes);
            if array.desc.digest != listed {
                return Err(LoadError(format!("{name}: the digest does not match manifest.json")));
            }
            array.check(name).map_err(|_| LoadError(format!("{name}: the size does not match its dimensions")))?;
            Ok(array)
        };
        let offsets: Vec<u32> = manifest
            .get("mesh")
            .and_then(|mesh| mesh.get("offsets"))
            .and_then(Value::as_array)
            .ok_or_else(|| LoadError("manifest.json has no mesh offsets".to_owned()))?
            .iter()
            .map(|v| v.as_i64().and_then(|v| u32::try_from(v).ok()))
            .collect::<Option<_>>()
            .ok_or_else(|| LoadError("a mesh offset is not a u32".to_owned()))?;
        let ordered = offsets.first() == Some(&0) && offsets.windows(2).all(|w| w[0] <= w[1]);
        if offsets.len() as u64 != OFFSETS || !ordered || offsets.last().map(|&o| u64::from(o)) != Some(VERTICES) {
            return Err(LoadError("the mesh offsets are not 434 nondecreasing entries from 0 to 30480".to_owned()));
        }
        Ok(GeometryInput {
            identity: IdentityBlock::full(model_id),
            manifest: sha256_hex(&text),
            vertices: read("mesh_vertices.f32", Elem::F32, vec![VERTICES, 4])?,
            sticker: read("mesh_sticker.u32", Elem::U32, vec![VERTICES])?,
            frames: read("cell_frames.f32", Elem::F32, vec![600, 4, 4])?,
            offsets: Array::u32s(vec![OFFSETS], &offsets),
            slot_piece: read("slot_piece.u32", Elem::U32, vec![SLOTS as u64])?,
        })
    }

    /// The digests a plan's geometry binding names (RPI.8), by asset name.
    pub fn digests(&self) -> BTreeMap<String, String> {
        [
            ("manifest.json", &self.manifest),
            ("mesh_vertices.f32", &self.vertices.desc.digest),
            ("mesh_sticker.u32", &self.sticker.desc.digest),
            ("cell_frames.f32", &self.frames.desc.digest),
            ("mesh.offsets", &self.offsets.desc.digest),
            ("slot_piece.u32", &self.slot_piece.desc.digest),
        ]
        .into_iter()
        .map(|(name, digest)| (name.to_owned(), digest.clone()))
        .collect()
    }
}

/// A set that holds every slot: the full visibility or interaction set.
pub fn full_set() -> Vec<u8> {
    vec![1; SLOTS]
}

/// RM.6's `RetainedInput` for a kernel state: its full labels and the two separate sets.
pub fn retained(state: &Snapshot, visibility: &[u8], interaction: &[u8], tokens: Vec<String>) -> RetainedInput {
    let labels: Vec<u32> = state.labels().iter().map(|&label| label as u32).collect();
    RetainedInput {
        labels: Array::u32s(vec![SLOTS as u64], &labels),
        visibility: Array::u8s(vec![SLOTS as u64], visibility),
        interaction: Array::u8s(vec![SLOTS as u64], interaction),
        tokens,
    }
}

/// RM.6's `TransformInput` for a certified effect. The moving pieces are exactly the pieces of the moved slots; the
/// plane is the one the engine names, and without it there is no swept preview (F03). `endpoints` are the committed
/// state revisions.
pub fn transform(
    geometry: &GeometryInput,
    effect: &Effect,
    grip: u32,
    plane: Option<[[f32; 4]; 2]>,
    angle: f32,
    endpoints: (u64, u64),
) -> TransformInput {
    let pieces = geometry.slot_piece.as_u32();
    let moving: BTreeSet<u32> = effect.source().iter().map(|&slot| pieces[slot as usize]).collect();
    let moving: Vec<u32> = moving.into_iter().collect();
    TransformInput {
        grip,
        plane: plane.map(|p| Array::f32s(vec![2, 4], &p.concat())),
        angle,
        moving_digest: piece_set_digest(&moving),
        moving: Array::u32s(vec![moving.len() as u64], &moving),
        from: endpoints.0,
        to: endpoints.1,
    }
}

/// A KN0.2 revision of a fixture session with Jumble off, at state revision `state`.
pub fn revision(identity: &IdentityBlock, state: u64, labels: &Snapshot) -> RevisionRef {
    RevisionRef {
        session: "fixture".to_owned(),
        epoch: 1,
        identity: identity.clone(),
        mode: Mode::JumbleOff,
        head: state,
        state,
        workspace: 1,
        protection: 0,
        digest: labels.digest().to_owned(),
    }
}

/// The fixture stage signatures of the built-in projection chain (F04): name and parameter count.
pub const STAGES: [(&str, usize); 4] = [("rotate", 6), ("perspective", 1), ("stereographic", 0), ("camera3", 9)];

/// A fixture stage definition, version 1, whose definition digest is the digest of `name/1`.
pub fn stage_def(name: &str, params: usize) -> StageDef {
    StageDef { name: name.to_owned(), version: 1, definition: sha256_hex(format!("{name}/1").as_bytes()), params }
}

/// A fixture support report for 600-cell-Full: F01 to F24 are supported with `limits`, except the user stages F05
/// and F26, which are reported unsupported. Rows it omits (F25, F27 and the jumbling rows) are unsupported (RM.15).
pub fn support_report(limits: &LimitProfile) -> SupportReport {
    let mut features = BTreeMap::new();
    for row in 1..=24 {
        let id = format!("F{row:02}");
        let support = if row == 5 {
            Support::Unsupported(Error::unsupported(
                &id,
                "user vertex stages need a backend that compiles and bounds them",
            ))
        } else {
            let stages = if row == 4 {
                STAGES.iter().map(|&(name, params)| stage_def(name, params)).collect()
            } else {
                Vec::new()
            };
            Support::Supported { layout_version: 1, limits: limits.clone(), stages }
        };
        features.insert(id, support);
    }
    features.insert(
        "F26".to_owned(),
        Support::Unsupported(Error::unsupported(
            "F26",
            "user fragment stages need a backend that compiles and bounds them",
        )),
    );
    SupportReport {
        interface_version: 1,
        backend_build: "mock".to_owned(),
        device_epoch: 1,
        features,
        evidence: vec!["source fixture; no backend run".to_owned()],
    }
}

/// The fixture host token table.
pub fn token_table() -> TokenTable {
    let names: BTreeSet<String> = ["sticker", "highlight", "dim", "missing", "relation"].map(str::to_owned).into();
    let text: String = names.iter().map(|name| format!("{name}\n")).collect();
    TokenTable { digest: sha256_hex(text.as_bytes()), names }
}

/// Fixture records for `word()`: one key per EVD.32 word the tests show, each with its EVD.36 scope phrase.
pub fn word_records() -> BTreeMap<Key, WordRecord> {
    [
        ("fixture.proved", "proved", "proved on D[k], for reachable inputs"),
        ("fixture.finite", "finite instance", "one finite instance: the base revision"),
        ("fixture.open", "open", "recorded only"),
    ]
    .into_iter()
    .map(|(key, word, scope)| (Key(key.to_owned()), WordRecord { word: word.to_owned(), scope: scope.to_owned() }))
    .collect()
}

/// A declared host context for 600-cell-Full at d = 4, with `limits` as both the host ceilings and every supported
/// row's ceilings.
pub fn fixture_context(geometry: &GeometryInput, limits: LimitProfile) -> Context {
    Context {
        identity: geometry.identity.clone(),
        dimension: 4,
        geometry: geometry.digests(),
        support: support_report(&limits),
        limits,
        tokens: token_table(),
        words: word_records(),
        checks: Vec::new(),
    }
}
