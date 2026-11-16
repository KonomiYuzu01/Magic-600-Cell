//! The view-model and publication types of chapter 10: `ViewNode` (RPI.5, RPI.65), `DisplayPlan` (RPI.6 to RPI.8),
//! the primitive rows of RPI.15 with the payloads of RM.6 and RM.7, `DisplayInstance` (RPI.20), `SourceMap` (RPI.21,
//! RM.10) and the publication of RPI.10. KN0.1 to KN0.3 supply the identity, revision and state references.
//! These are data: nothing here is a function for the renderer to execute (RPI.5).

use std::collections::BTreeMap;
use std::sync::Arc;

use c600_digest::sha256_hex;
use c600_kernel::IdentityBlock;

use crate::array::Array;

macro_rules! ids {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$doc])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name(pub u64);
        )*
    };
}

ids!(
    /// A view in LI.22's view namespace.
    ViewId,
    NodeId,
    PlanId,
    MapId,
    /// An occurrence ID; it distinguishes occurrences only within one installed view and plan generation (RPI.20).
    InstanceId,
    ChartId,
    /// A command of KN5.21.
    CommandId,
    JobId,
    /// An engine preview (KN5.9).
    PreviewId,
);

/// A statement key of chapter 06.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(pub String);

/// KN0.2's `mode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    JumbleOff,
    JumbleOn,
}

/// KN0.2's revision reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionRef {
    pub session: String,
    pub epoch: u64,
    pub identity: IdentityBlock,
    pub mode: Mode,
    pub head: u64,
    pub state: u64,
    pub workspace: u64,
    pub protection: u64,
    /// `Digest<level>`: the labels with Jumble off, the poses with Jumble on.
    pub digest: String,
}

/// The level of a state digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Labels,
    Poses,
}

/// KN2.11's derived-state reference: digest, level and identity block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedRef {
    pub digest: String,
    pub level: Level,
    pub identity: IdentityBlock,
}

/// KN0.3: a revision of a session or a derived state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateRef {
    Snapshot(RevisionRef),
    Derived(DerivedRef),
}

impl StateRef {
    pub fn digest(&self) -> &str {
        match self {
            StateRef::Snapshot(revision) => &revision.digest,
            StateRef::Derived(derived) => &derived.digest,
        }
    }

    pub fn identity(&self) -> &IdentityBlock {
        match self {
            StateRef::Snapshot(revision) => &revision.identity,
            StateRef::Derived(derived) => &derived.identity,
        }
    }
}

/// The lineage of a derived subject (ONE.21): KN2.11's derivation, or EV3.12's imported lineage with its explicit
/// comparison base. The optional fields exist so that a malformed program value can be represented and refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lineage {
    Derivation {
        base: Option<StateRef>,
        /// `Digest<literal word>`.
        word: String,
    },
    Imported {
        import: String,
        comparison_base: Option<RevisionRef>,
    },
}

/// The kind of a `ViewNode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Object,
    Relation,
    Container,
    Control,
}

/// A node's content, in RPI.65's three kinds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    /// State data: what a kernel row returned for the subject at `base`, named by the row and its result digest.
    State { row: String, digest: String },
    /// A word of EVD.32, asked for through `word()` on a key; the host supplies the word and its EVD.36 phrase.
    Word(Key),
    /// Program content: the program's own text. The host never reads it for evidence.
    Text(String),
}

/// RPI.5's `ViewNode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewNode {
    pub node: NodeId,
    pub base: RevisionRef,
    pub kind: NodeKind,
    pub content: Vec<Content>,
    pub plans: Vec<PlanId>,
    pub children: Vec<NodeId>,
    pub source: Option<SourceLink>,
    /// Command and binding references of chapters 01 and 04, by name; drawing invokes none of them (RPI.14).
    pub commands: Vec<String>,
    pub claims: Vec<Key>,
}

/// RPI.8's static geometry binding: the model identity and the immutable geometry content digests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryBinding {
    pub identity: IdentityBlock,
    /// Asset name to SHA-256, for the arrays of RM.6's `GeometryInput`.
    pub digests: BTreeMap<String, String>,
}

/// RPI.6's `DisplayPlan`.
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayPlan {
    pub plan: PlanId,
    pub base: RevisionRef,
    pub subject: StateRef,
    /// KN2.11's derivation or EV3.12's lineage, when the subject is derived (RPI.7).
    pub derivation: Option<Lineage>,
    /// The reachability reference of a derived subject (RM.8).
    pub reachability: Option<String>,
    pub dimension: u32,
    pub geometry: GeometryBinding,
    pub primitives: Vec<Primitive>,
    pub charts: Vec<Chart>,
    pub instances: Vec<DisplayInstance>,
    pub source_map: MapId,
    /// The declared display precision of chapter 09, by name; RPI.36's rule is open, so it is not interpreted.
    pub precision: String,
    /// The revisions and immutable input digests actually read: `state`, `workspace`, `protection`, `input:<name>`.
    pub dependencies: BTreeMap<String, String>,
}

/// One primitive row of RPI.15.
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    Retained(RetainedInput),
    Posed(PosedInput),
    Transform(TransformInput),
    ChartStage(StageInput),
    UserStage(UserStageInput),
    Layout(LayoutInput),
    CertOverlay(CertOverlay),
    SetOverlay(OverlayInput),
    Tokens(TokenInput),
    Derived(DerivedDisplay),
    Preview(PreviewDisplay),
}

impl Primitive {
    /// The feature row of RM.27 or RM.34 that admits this primitive.
    pub fn feature(&self) -> &'static str {
        match self {
            Primitive::Retained(_) => "F01",
            Primitive::Posed(_) => "J02",
            Primitive::Transform(_) => "F03",
            Primitive::ChartStage(_) => "F04",
            Primitive::UserStage(stage) => match stage.kind {
                StageKind::Vertex => "F05",
                StageKind::Fragment => "F26",
            },
            Primitive::Layout(layout) => match layout.kind {
                LayoutKind::Sheet => "F06",
                LayoutKind::Atlas { .. } => "F07",
                LayoutKind::Tree { .. } => "F08",
                LayoutKind::Complex(_) => "F09",
                LayoutKind::Graph(_) => "F10",
            },
            Primitive::CertOverlay(_) => "J04",
            Primitive::SetOverlay(overlay) => match overlay.predicate {
                Some(Predicate::Section(_)) => "F11",
                Some(Predicate::Slab(_)) => "F12",
                Some(Predicate::Clip(_)) => "F13",
                None => "F14",
            },
            Primitive::Tokens(_) => "F15",
            Primitive::Derived(_) => "F16",
            Primitive::Preview(_) => "F17",
        }
    }
}

/// RM.6's `RetainedInput`: the full labels and the separate visibility and interaction sets of one revision.
#[derive(Clone, Debug, PartialEq)]
pub struct RetainedInput {
    /// u32 [259800].
    pub labels: Array,
    /// u8 [259800], values 0 or 1.
    pub visibility: Array,
    /// u8 [259800], values 0 or 1.
    pub interaction: Array,
    /// Token names for the object classes, resolved before installation.
    pub tokens: Vec<String>,
}

/// The jumbling posed-sticker row (render/1 section 3); its producer waits for M5.
#[derive(Clone, Debug, PartialEq)]
pub struct PosedInput {
    pub pose_index: Array,
    pub poses: Array,
    pub pose_lattice: Array,
}

/// RM.6's `TransformInput`: an engine-certified retained motion.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformInput {
    pub grip: u32,
    /// f32 [2, d], the engine-named plane; absent when the engine names none.
    pub plane: Option<Array>,
    pub angle: f32,
    /// Sorted moving-piece IDs, u32 [m].
    pub moving: Array,
    /// The exact-set digest of the moving pieces.
    pub moving_digest: String,
    /// The committed endpoints, as state revisions.
    pub from: u64,
    pub to: u64,
}

/// One stage occurrence of a chart's chain.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    pub name: String,
    pub version: u32,
    /// The evaluator definition digest.
    pub definition: String,
    pub params: Vec<f64>,
}

/// A chart (RPI.6, RM.6 `StageInput`): an ordered chain of stages with its dimension and precision.
#[derive(Clone, Debug, PartialEq)]
pub struct Chart {
    pub chart: ChartId,
    pub stages: Vec<Stage>,
    pub dimension: u32,
    pub precision: String,
    /// Exact chart properties the plan asks to rely on; RPI.34 names no checker, so none is established.
    pub properties: Vec<String>,
}

/// The chart-stage row: a chart reference whose stages the backend evaluates per vertex.
#[derive(Clone, Debug, PartialEq)]
pub struct StageInput {
    pub chart: ChartId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageKind {
    Vertex,
    Fragment,
}

/// The user-stage row (RPI.68 to RPI.80). The stage checker is not built here, so it is always unsupported.
#[derive(Clone, Debug, PartialEq)]
pub struct UserStageInput {
    pub kind: StageKind,
    pub digest: String,
    pub compile_record: String,
}

/// The layout kinds of RM.7.
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutKind {
    Sheet,
    /// The chart of each occurrence, in the order of the layout's instances.
    Atlas {
        membership: Vec<ChartId>,
    },
    /// Root indices and parent/child edges u32 [p, 2].
    Tree {
        roots: Vec<u32>,
        parent_child: Array,
    },
    Complex(IncidenceInput),
    Graph(GraphInput),
}

/// RM.6's `LayoutInput`.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutInput {
    /// The typed object/instance table [n].
    pub instances: Vec<InstanceId>,
    /// f32 [n, k], k = 2 or 3.
    pub positions: Array,
    /// u32 [e, 2].
    pub edges: Option<Array>,
    /// u32 [t, 3].
    pub triangles: Option<Array>,
    pub kind: LayoutKind,
}

/// RM.6's `IncidenceInput` additions: a dimension-tagged cell complex over the layout's instances.
#[derive(Clone, Debug, PartialEq)]
pub struct IncidenceInput {
    /// u32 [c].
    pub dims: Array,
    /// u32 [c + 1].
    pub offsets: Array,
    /// u32 [b].
    pub boundary: Array,
    /// i32 [b], values -1 or 1.
    pub orientation: Array,
}

/// RM.6's `GraphInput` additions: each node's subject and supplied transition references per directed edge.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphInput {
    pub subjects: Vec<StateRef>,
    /// u32 [e, 2].
    pub edges: Array,
    pub transitions: Vec<String>,
}

/// A certificate point: its exact reference and its converted float vector.
#[derive(Clone, Debug, PartialEq)]
pub struct CertPoint {
    pub exact: String,
    pub vector: [f32; 4],
}

/// A verified point pair of one straddling piece.
#[derive(Clone, Debug, PartialEq)]
pub struct PointPair {
    pub piece: u32,
    pub points: [CertPoint; 2],
}

/// A grip status of the survey.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripStatus {
    A,
    B,
    Uncertain,
}

/// A named cut: the exact pole and plane and alpha as a rational.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedCut {
    pub pole: u32,
    pub plane: String,
    pub alpha: (i64, i64),
}

/// Whether a certificate payload is the full named-cap set or a cap-map witness (J04).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertKind {
    FullNamedCapSet,
    CapMapWitness,
}

/// The certified geometric overlay row (render contract section 5, J04): a jumbling row.
#[derive(Clone, Debug, PartialEq)]
pub struct CertOverlay {
    pub grip: u32,
    pub cap: u32,
    pub kind: CertKind,
    /// Strictly increasing piece IDs.
    pub pieces: Vec<u32>,
    /// The exact-set digest of the pieces ([`piece_set_digest`]).
    pub set_digest: String,
    pub pairs: Vec<PointPair>,
    /// The grip survey, in grip order; empty when the payload carries none.
    pub survey: Vec<GripStatus>,
    pub cut: Option<NamedCut>,
}

/// The coverage of an exact result (KN0.4): complete, partial with what is missing, or undefined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Coverage {
    Complete,
    Partial { missing: Vec<String> },
    Undefined(String),
}

/// The exact predicate of a section, slab or clip overlay (RM.7); the three stay distinct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Predicate {
    Section(String),
    Slab(String),
    Clip(String),
}

/// An exact set or a relation (EV3.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OverlayKind {
    Set,
    /// A relation carries its claim key under RPI.65; `None` is refused.
    Relation {
        claim: Option<Key>,
    },
}

/// RM.6's `OverlayInput`.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayInput {
    pub kind: OverlayKind,
    /// The canonical objects, in canonical order without repeats.
    pub objects: Vec<ObjectId>,
    /// [`object_set_digest`] of the objects.
    pub set_digest: String,
    pub coverage: Coverage,
    pub predicate: Option<Predicate>,
    /// f32 [v, k].
    pub positions: Option<Array>,
    /// u32 [e, 2].
    pub lines: Option<Array>,
    /// u32 [t, 3].
    pub triangles: Option<Array>,
    pub tokens: Vec<String>,
}

/// RM.6's `TokenInput`: names bound to the host token table by its digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenInput {
    pub table: String,
    /// (object class, token name).
    pub names: Vec<(String, String)>,
}

/// The derived-state display row: a derived subject with its lineage and a nested primitive set.
#[derive(Clone, Debug, PartialEq)]
pub struct DerivedDisplay {
    pub subject: DerivedRef,
    pub lineage: Option<Lineage>,
    pub reachability: String,
    pub primitives: Vec<Primitive>,
}

/// The preview display row: drawn from the engine's preview object only (RM.9).
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewDisplay {
    pub preview: PreviewId,
    pub base: RevisionRef,
    /// The literal word digest.
    pub literal: String,
    /// The certified effect digest.
    pub effect: String,
    pub primitives: Vec<Primitive>,
}

/// A canonical object of KN1. A certificate point targets its witness piece and keeps its grip and source role.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectId {
    /// A sticker by its home slot, which names `(A, f)` as `433 * A + f`.
    Sticker(u32),
    Piece(u32),
    Slot(u32),
    Cell(u32),
    Grip(u32),
    Cap(u32),
    Orbit(u32),
    Point {
        witness: u32,
        grip: u32,
        role: String,
    },
}

/// RPI.20's `DisplayInstance`. The host binds the view and its generation at installation; the base and subject are
/// the plan's, and the object's model and level are the plan identity and subject level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayInstance {
    pub instance: InstanceId,
    pub plan: PlanId,
    pub object: ObjectId,
    /// The index of its primitive in the plan.
    pub primitive: u32,
    /// The declared sticker-to-piece association of a sticker occurrence (RPI.23).
    pub piece: Option<u32>,
    /// Eligibility from the view's declared interaction set.
    pub interaction: bool,
}

/// RPI.21's source linkage of one entry: an EV1.10 range bound to its producing digest, or explicit missing source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceLink {
    /// A half-open UTF-8 byte range `[start, end)` of the source with that digest.
    Range {
        digest: String,
        start: u64,
        end: u64,
    },
    Missing {
        digest: String,
        reason: String,
    },
}

impl SourceLink {
    pub fn digest(&self) -> &str {
        match self {
            SourceLink::Range { digest, .. } | SourceLink::Missing { digest, .. } => digest,
        }
    }
}

/// One entry of a source map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapEntry {
    pub object: ObjectId,
    pub source: SourceLink,
}

/// RPI.21's `SourceMap`. The host binds the plan's installed generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMap {
    pub map: MapId,
    pub plan: PlanId,
    pub base: RevisionRef,
    /// The producing source digests.
    pub sources: Vec<String>,
    pub entries: BTreeMap<InstanceId, MapEntry>,
}

/// RM.6's `PublicationInput`: nodes, plans and maps of one target view, and the pinned source bytes by digest.
#[derive(Clone, Debug, PartialEq)]
pub struct Publication {
    pub target: ViewId,
    pub base: RevisionRef,
    pub nodes: Vec<ViewNode>,
    pub plans: Vec<DisplayPlan>,
    pub maps: Vec<SourceMap>,
    pub sources: BTreeMap<String, Arc<Vec<u8>>>,
}

/// The exact-set digest of piece IDs: the SHA-256 of the sorted IDs as little-endian u32.
pub fn piece_set_digest(pieces: &[u32]) -> String {
    let bytes: Vec<u8> = pieces.iter().flat_map(|p| p.to_le_bytes()).collect();
    sha256_hex(&bytes)
}

/// The exact-set digest of canonical objects: the SHA-256 of their canonical text, one per line.
pub fn object_set_digest(objects: &[ObjectId]) -> String {
    let text: String = objects.iter().map(|object| format!("{}\n", object_text(object))).collect();
    sha256_hex(text.as_bytes())
}

/// The canonical text of an object ID.
pub fn object_text(object: &ObjectId) -> String {
    match object {
        ObjectId::Sticker(id) => format!("sticker:{id}"),
        ObjectId::Piece(id) => format!("piece:{id}"),
        ObjectId::Slot(id) => format!("slot:{id}"),
        ObjectId::Cell(id) => format!("cell:{id}"),
        ObjectId::Grip(id) => format!("grip:{id}"),
        ObjectId::Cap(id) => format!("cap:{id}"),
        ObjectId::Orbit(id) => format!("orbit:{id}"),
        ObjectId::Point { witness, grip, role } => format!("point:{witness}:{grip}:{role}"),
    }
}
