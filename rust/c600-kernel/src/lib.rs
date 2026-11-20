//! The 600-cell-Full kernel of 600 Cell Script (LP2, milestone M4).
//!
//! The kernel reads the model straight from `assets/` and answers the label-level rows of chapter 01's KN2.4 for
//! 600-cell-Full: the label array of a snapshot, the exact effect of a recipe, the protection verdict for a set of
//! protected orbits, and the derived state of KN2.11. The oracle adapter (`src/bin/c600-oracle-adapter.rs`) replays
//! the frozen oracle cases O01 to O07 through this API, never through the engine's internals, and its `agree`
//! command runs the KN2.9 agreement test of the star cache against per-primitive replay.
//!
//! # Loading
//!
//! [`Kernel::load`] reads `assets/manifest.json`, checks that each listed name is a plain file name in `assets/` and
//! that the `model_id` is the SHA-256 of the file list as `build_assets.py` writes it, checks the SHA-256 of every file
//! the manifest lists, and reads `model.npz`, `primitives.npz`, `execution_trees.json.gz`, `census.json` and
//! `seed_atlas.json` from the verified bytes with this crate's own ZIP, DEFLATE, gzip, NPY and JSON readers
//! ([`archive`], [`json`]). The model's identity
//! is the manifest's `model_id` together with the SHA-256 of the manifest's bytes. Only the arrays that effects,
//! protection, progress and the star certificates read are kept, and the loader checks the facts its loops rely on
//! (indices in range, every primitive a permutation of its support, execution trees whose walks end).
//!
//! # Mechanics
//!
//! The mechanics follow `core.py` of 0.4. Permutations run from source to destination and words from left to right.
//! A star expands to `invword(path) + invword(relocation) + seed + relocation + path`, inverted whole for sign -1. The
//! net map of a star comes from the star cache: the seed's net map, certified once per orbit and relocated, then
//! conjugated by the star's setup path, with 0.4's guard on the buffer caps and its frame checks (ADR-L12.7, RTE.18).
//! The cache keeps the 96 most recent stars, as 0.4 does. A cache hit and a recomputation give the same map, so the
//! cache never changes a result; a failed certificate is a [`CertificateError`], never a refusal of the recipe.
//!
//! # Decisions of the integrator for M4
//!
//! - (a) The 0.4 oracle is judged with Net protection at label level: a recipe is refused only when its net map
//!   moves a sticker of a protected orbit, so O06's `word([g, -g])` is accepted. Strict protection came after M4
//!   (below) and is asked for by name ([`Kernel::derive_strict`]); [`Kernel::derive`] stays Net.
//! - (b) A recipe is validated whole, under the 0.4 limits and with the 0.4 error texts, before any effect is
//!   computed: 1 to 256 steps, words of 1 to 100,000 letters, star fields and frame nodes in range, and at most
//!   3,000,000 primitives in the expansion. Streaming words in chunks (KN2.10) came after M4 (below).
//! - (c) The trace's `model` block comes from the loaded identity: the `model_id` and the manifest's SHA-256.
//! - (d) `progress_sha256` is the SHA-256 of the canonical JSON of the 35 progress rows, computed from the labels and
//!   the census as the LX-6 port did. No reference changed, and the cases' `optional_fields` are not used.
//! - (e) The frozen, tracked case files are the adapter's inputs, so no step of M4 needs NumPy.
//!
//! # Chapter 01 rows and contract tests
//!
//! Built here: the identity block of KN0.1 for 600-cell-Full ([`IdentityBlock`]); the registry answer of KN1.1 for
//! the one loaded identity ([`Kernel::model_info`]); KN1.4's refusal of a value with another identity
//! ([`Kernel::check_identity`], and every call that takes a snapshot or a recipe); snapshots of KN2.2 as immutable
//! label arrays ([`Snapshot`]); the KN2.4 rows `labels`, `effect` and `protection` for Full at label level; literal
//! letters for effect and digest (KN2.7); the KN2.9 agreement test (adapter `agree`); `derive` of KN2.11
//! ([`Kernel::derive`]). Contract tests: CT-KN1-01, the labels half of CT-KN1-05 (`Digest<labels>` is the oracle's
//! state hash), the 600-cell-Full half of CT-KN2-01 (O01 to O07 through the kernel API) and CT-KN2-04 (a blocked or
//! invalid recipe leaves every digest unchanged).
//!
//! Not built in M4, and since built (LP2 kernel rows, below): the `positions` row, Strict protection and streaming
//! words. Not built: `open_snapshot` by `RevisionRef`, which waits for the session store; the reachability claim of
//! `Derived`; budgets (KN7); every jumbling row and identity (a jumbling identity block is refused). A failed star
//! certificate or asset check has no code in chapter 13's catalogue yet; the kernel reports it as
//! [`CertificateError`] or [`LoadError`].
//!
//! # LP2 kernel rows after M4
//!
//! Built: KN1.4 for protected sets ([`Kernel::checked_protection`]); the KN2.4 rows `positions`
//! ([`Kernel::positions`]), `protection` with Strict ([`Kernel::strict_protection`], [`Kernel::derive_strict`]) and
//! `replay` for Full ([`Kernel::replay`]); the streaming of KN2.10 ([`Kernel::chunks`], [`Replay`]); KN2.11's refusal
//! shape ([`Refused`]); `protection` at label level for named pieces, fixed cell centres included, under Net and Strict
//! ([`Kernel::piece_protection`], [`Kernel::strict_piece_protection`]), which LP3's C8 reads for LB3.4; the structure
//! rows that `structure.compare`, `filter.preview` and `structure.layer` read, for LP3's C6 ([`Kernel::compare_cells`],
//! [`Kernel::cell_layers`], [`Kernel::layer_counts`], [`Kernel::layer_members`]). Draft choices:
//!
//! - A nonempty [`ProtectedSet`] is bound to the model that built it. The empty set names no orbit, so it belongs to no
//!   model, and `ProtectedSet::none()` equals `protected_set(&[])`. Every call that takes a set checks it except the
//!   old [`Kernel::protection`], whose signature is kept. An [`Effect`] names no model, as M4 decided.
//! - Strict at label level judges, for every executed prefix of the literal expansion, whether its net map moves a
//!   sticker of a protected orbit. While the earlier letters fix the protected orbits, a prefix moves exactly the
//!   protected stickers its last letter moves, so the first refused prefix ends at the first letter that moves one,
//!   read from a per-generator orbit mask. On the shipped model every generator moves a sticker of each of the 35
//!   orbits, so Strict refuses every word at prefix 1 once any orbit is protected.
//! - KN2.11's `first_refused_prefix` is a prefix length (1 for the first letter); `outcome` is [`Refused::refusal`];
//!   `certificates` give, per protected orbit the judged map moves, its smallest moved slot and that sticker's
//!   destination. Under Net the refusal reports no prefix, since Net judges only the end of the complete operation
//!   (LB2.3), and its certificates come from the net map.
//! - `positions` names a piece by its home position, with 0.4's text and `KN2:input.named_object_invalid` for a bad
//!   one. A label array read back by `state_from_labels` can split a piece; its row then has no position.
//! - [`Replay`] judges Strict prefix by prefix and has no length bound, since X3's bound is unset;
//!   [`Kernel::replay_under_ceiling`] counts the 3,000,000-primitive ceiling over all chunks with 0.4's text. A refused
//!   chunk changes nothing. A word of no letters derives the base state (LB1: an empty expansion is a zero-letter
//!   word). `c600-digest` has no incremental hash, so the replay keeps the letters' bytes for `Digest<literal word>`.
//! - A [`ProtectedPieces`] names pieces as `positions` does and is bound to its model as a [`ProtectedSet`] is. A
//!   [`ProtectedSet`] names moving orbits only, so a fixed cell centre had no label-level row. A piece's row gives the
//!   slots of its stickers in the base, which are the base's `positions` slots, and after the judged map, so it depends
//!   on the base. A centre's row is computed like any other; no letter moves a centre's sticker, so it comes out clear
//!   (LB3.4: inside Jumble that label-level pass cannot replace the pose-level check). Strict refuses at the first
//!   letter that moves a slot holding a protected sticker. The piece calls judge only: [`Kernel::derive`] still takes a
//!   [`ProtectedSet`], and a refusal for pieces stays the caller's, with no new code.
//! - The structure rows follow 0.4's `cell_distances` and `layer_mask`: face steps between cells, and the layers around
//!   a cell, in which a position with a home slot in a cell of the layer counts whole, so layers overlap. 0.4 takes a
//!   cell's neighbours as the four nearest centres; here two cells share a face when they share three of their four
//!   vertices, read from the 120 vertex pieces, and the load checks the graph. The rows read no state, so they take no
//!   snapshot. A cell outside 0 to 599 and a layer past the cell's last are `KN2:input.named_object_invalid` with
//!   the texts of 0.4's `cell_mask` and `layer_mask`.

#![forbid(unsafe_code)]

pub mod archive;
mod engine;
pub mod exact;
pub mod json;
mod model;
pub mod request;
mod structure;

use std::fmt;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, Mutex};

use json::Value;
use model::Model;
pub use model::{verify_assets, VerifiedAssets};

/// The refusal codes of chapter 13 that this crate uses.
pub mod codes {
    /// A recipe violates the retained word and star step shape or a step range.
    pub const RECIPE_SHAPE: &str = "KN2:input.recipe_shape";
    /// The expansion exceeds the 3,000,000-primitive compatibility ceiling.
    pub const PRIMITIVE_CEILING: &str = "KN2:resource_limit.primitive_ceiling";
    /// A protected orbit would move under Net protection.
    pub const ORBIT_WOULD_MOVE: &str = "KN2:protection.orbit_would_move";
    /// A named object (here a protected orbit) is missing or out of range.
    pub const NAMED_OBJECT_INVALID: &str = "KN2:input.named_object_invalid";
    /// A value names another model, menu, order digest or contract revision.
    pub const IDENTITY_MISMATCH: &str = "KN1:input.identity_mismatch";
    /// The identity is absent from the model registry.
    pub const UNKNOWN_MODEL: &str = "KN1:input.unknown_model";
}

/// The puzzle name in the manifest.
pub const PUZZLE_NAME: &str = model::PUZZLE;
/// Labelled sticker slots of 600-cell-Full.
pub const SLOTS: usize = model::SLOTS;
/// Physical pieces, fixed cell centres included.
pub const PIECES: usize = model::PIECES;
/// Moving orbits.
pub const ORBITS: usize = model::ORBITS;
/// Legal generators (primitives 1 to 1,200 and their inverses).
pub const GENERATORS: usize = model::GENERATORS;

/// The protected-move refusal text of 0.4 (`session.py`).
pub const PROTECTED_MESSAGE: &str =
    "Protected orbit would move. Remove that protection explicitly or choose a different macro.";

/// The sources of the kernel and its adapter, hashed into [`build`].
pub const SOURCES: [(&str, &[u8]); 11] = [
    ("c600-kernel/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-kernel/src/archive.rs", include_bytes!("archive.rs")),
    ("c600-kernel/src/bin/c600-oracle-adapter.rs", include_bytes!("bin/c600-oracle-adapter.rs")),
    ("c600-kernel/src/engine.rs", include_bytes!("engine.rs")),
    ("c600-kernel/src/exact.rs", include_bytes!("exact.rs")),
    ("c600-kernel/src/json.rs", include_bytes!("json.rs")),
    ("c600-kernel/src/lib.rs", include_bytes!("lib.rs")),
    ("c600-kernel/src/model.rs", include_bytes!("model.rs")),
    ("c600-kernel/src/request.rs", include_bytes!("request.rs")),
    ("c600-kernel/src/structure.rs", include_bytes!("structure.rs")),
    ("c600-digest/src/lib.rs", include_bytes!("../../c600-digest/src/lib.rs")),
];

/// The build string: the crate version and the SHA-256 of a `sha256sum`-style listing of [`SOURCES`].
pub fn build() -> String {
    let mut listing = String::new();
    for (name, bytes) in SOURCES {
        listing.push_str(&format!("{}  {name}\n", c600_digest::sha256_hex(bytes)));
    }
    format!("c600-kernel {} sources-sha256:{}", env!("CARGO_PKG_VERSION"), c600_digest::sha256_hex(listing.as_bytes()))
}

/// Why the model could not be loaded. The message never names a path outside `assets/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadError(pub String);

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoadError {}

/// A star certificate of the loaded model failed, with 0.4's message (`Seed definition mismatch`, `Setup moves a
/// guarded buffer position`, `Transported star frame mismatch` and the others of `certify_seed` and `star_net`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertificateError(pub String);

impl fmt::Display for CertificateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CertificateError {}

/// A refusal with its chapter 13 code and the 0.4 text where 0.4 has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
}

impl Refusal {
    fn new(code: &'static str, message: impl Into<String>) -> Refusal {
        Refusal { code, message: message.into() }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Refusal {}

/// The failure of a call that computes an effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Refusal(Refusal),
    Certificate(CertificateError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Refusal(refusal) => refusal.fmt(f),
            Error::Certificate(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<Refusal> for Error {
    fn from(refusal: Refusal) -> Error {
        Error::Refusal(refusal)
    }
}

impl From<CertificateError> for Error {
    fn from(error: CertificateError) -> Error {
        Error::Certificate(error)
    }
}

/// The puzzle an identity block names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Puzzle {
    Full,
    Jumbling,
}

/// The identity block of KN0.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityBlock {
    pub puzzle: Puzzle,
    /// The `model_id` of `assets/manifest.json`.
    pub model_id: String,
    /// Jumbling only: the order-free menu identity.
    pub menu_id: Option<String>,
    /// Jumbling only: the menu-order digest.
    pub menu_order: Option<String>,
    /// Jumbling only: the state contract revision.
    pub contract_rev: Option<String>,
}

impl IdentityBlock {
    /// The block of 600-cell-Full with the given model ID.
    pub fn full(model_id: &str) -> IdentityBlock {
        IdentityBlock {
            puzzle: Puzzle::Full,
            model_id: model_id.to_owned(),
            menu_id: None,
            menu_order: None,
            contract_rev: None,
        }
    }
}

/// What the registry knows about the loaded model (KN1.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub identity: IdentityBlock,
    pub puzzle_name: &'static str,
    /// The SHA-256 of `assets/manifest.json`.
    pub manifest_sha256: String,
    /// How many listed files were checked against the manifest.
    pub verified_files: usize,
    pub slots: usize,
    pub pieces: usize,
    pub orbits: usize,
    pub generators: usize,
}

struct SnapshotData {
    model_id: String,
    labels: Vec<i32>,
    digest: String,
}

/// A read-only label state of one model (KN2.2). Its data never changes; a derived state is a new snapshot.
#[derive(Clone)]
pub struct Snapshot(Arc<SnapshotData>);

impl Snapshot {
    fn new(model_id: &str, labels: Vec<i32>) -> Snapshot {
        let digest = engine::digest_ints(&labels);
        Snapshot(Arc::new(SnapshotData { model_id: model_id.to_owned(), labels, digest }))
    }

    /// The label array: `labels()[slot]` is the home slot of the sticker now in `slot`.
    pub fn labels(&self) -> &[i32] {
        &self.0.labels
    }

    /// `Digest<labels>`: the SHA-256 of the labels as little-endian 32-bit integers, which is 0.4's state hash.
    pub fn digest(&self) -> &str {
        &self.0.digest
    }

    pub fn model_id(&self) -> &str {
        &self.0.model_id
    }
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot").field("model_id", &self.0.model_id).field("digest", &self.0.digest).finish()
    }
}

/// The `positions` row of KN2.4 for one named piece: where its stickers are in a state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiecePosition {
    /// The named piece, by its home position, 0 to 177,119.
    pub piece: usize,
    /// Its orbit; -1 for a fixed cell centre.
    pub orbit: i32,
    /// Its home slots in increasing order, 0.4's `Model.slots(piece)`.
    pub home: Vec<i32>,
    /// `slots[i]` is the slot that now holds the sticker whose home slot is `home[i]`: the slots and orientation.
    pub slots: Vec<i32>,
    /// The position whose slots hold all of the piece's stickers, 0.4's `where[piece]`. `None` when the stickers are
    /// split over positions, which no twist does but a label array read back by `state_from_labels` can hold.
    pub position: Option<usize>,
}

impl PiecePosition {
    /// The piece is in its home position (0.4's `position_correct`).
    pub fn position_correct(&self) -> bool {
        self.position == Some(self.piece)
    }

    /// Every sticker of the piece is in its home slot (0.4's `correct`).
    pub fn solved(&self) -> bool {
        self.slots == self.home
    }
}

/// Two cells compared, as 0.4's structure explorer compares two colours (LP2 draft): the face steps between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellComparison {
    /// The first cell, 0 to 599.
    pub first: usize,
    /// The second cell, 0 to 599.
    pub second: usize,
    /// The fewest face steps from `first` to `second`: 0 for the same cell and 1 for two cells that share a face.
    pub steps: usize,
}

impl CellComparison {
    /// The two cells share a triangular face.
    pub fn shares_face(&self) -> bool {
        self.steps == 1
    }
}

/// One breadth-first layer around a cell (LP2 draft): the positions with a home slot in a cell `layer` face steps from
/// `cell`, which 0.4's `layer(C, L)` filter selects, and their slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerCounts {
    /// The cell the layers are counted from, 0 to 599.
    pub cell: usize,
    /// The layer: its cells' face steps from `cell`.
    pub layer: usize,
    /// The positions in the layer, each counted whole.
    pub pieces: usize,
    /// The home slots of those positions.
    pub slots: usize,
}

/// The members of one breadth-first layer around a cell (LP2 draft): the positions and slots that [`LayerCounts`]
/// counts, each list in increasing order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerMembers {
    /// The cell the layers are counted from, 0 to 599.
    pub cell: usize,
    /// The layer: its cells' face steps from `cell`.
    pub layer: usize,
    /// The positions in the layer, named by their home positions.
    pub pieces: Vec<usize>,
    /// The home slots of those positions.
    pub slots: Vec<usize>,
}

/// A star step: the orbit's seed relocated and set up by the path to a frame node of the orbit's execution tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Star {
    orbit: usize,
    node: usize,
    sign: i32,
    path: Vec<i32>,
}

impl Star {
    pub fn orbit(&self) -> usize {
        self.orbit
    }

    pub fn node(&self) -> usize {
        self.node
    }

    /// 1 or -1.
    pub fn sign(&self) -> i32 {
        self.sign
    }

    /// The setup letters from the tree's root to the node.
    pub fn path(&self) -> &[i32] {
        &self.path
    }
}

/// One step of a normalized recipe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Word(Vec<i32>),
    Star(Star),
}

/// A recipe validated whole under the 0.4 limits, bound to the model that validated it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    model_id: String,
    steps: Vec<Step>,
    length: usize,
}

impl Recipe {
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The exact number of letters in the expansion.
    pub fn primitive_count(&self) -> usize {
        self.length
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// The stickers and pieces of one orbit that a net map moves; orbit -1 holds the fixed cell centres.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportRow {
    pub orbit: i32,
    pub stickers: usize,
    pub pieces: usize,
}

/// The `effect` row of KN2.4: a recipe's source-to-destination map and what it touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    source: Vec<i32>,
    destination: Vec<i32>,
    /// The SHA-256 of the sources and then the destinations, each as little-endian 32-bit integers.
    pub net_sha256: String,
    pub moved_pieces: usize,
    /// One row per orbit the map moves, in increasing orbit order.
    pub support: Vec<SupportRow>,
    /// `Digest<literal word>`: the SHA-256 of the expansion's letters as little-endian 32-bit integers.
    pub expansion_sha256: String,
    pub primitive_count: usize,
}

impl Effect {
    /// The moved slots in increasing order.
    pub fn source(&self) -> &[i32] {
        &self.source
    }

    /// `destination()[i]` is where the sticker in `source()[i]` goes.
    pub fn destination(&self) -> &[i32] {
        &self.destination
    }

    pub fn moved_stickers(&self) -> usize {
        self.source.len()
    }
}

/// The level a protection verdict is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Labels,
}

/// Net protection judges the net map of the whole operation. Strict judges every executed prefix of the literal
/// expansion (KN2.7, LB2.3), so a word with an identity net map can pass Net and fail Strict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectionMode {
    Net,
    Strict,
}

/// A set of protected moving orbits. A nonempty set is bound to the model that built it, and the checked calls refuse
/// it in another model's kernel (KN1.4). The empty set names no orbit, so it belongs to no model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProtectedSet {
    orbits: Vec<usize>,
    model_id: Option<String>,
}

impl ProtectedSet {
    /// The empty set.
    pub fn none() -> ProtectedSet {
        ProtectedSet::default()
    }

    /// The protected orbits in increasing order, without repeats.
    pub fn orbits(&self) -> &[usize] {
        &self.orbits
    }

    /// The model whose orbits the set names; `None` for the empty set.
    pub fn model_id(&self) -> Option<&str> {
        self.model_id.as_deref()
    }

    fn contains(&self, orbit: i32) -> bool {
        usize::try_from(orbit).is_ok_and(|orbit| self.orbits.binary_search(&orbit).is_ok())
    }

    /// The protected orbits as a bit set over orbits 0 to 34.
    fn mask(&self) -> u64 {
        self.orbits.iter().fold(0, |mask, &orbit| mask | 1 << orbit)
    }
}

/// Whether a protected orbit would move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Clear,
    Moves,
}

/// The `protection` row of KN2.4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Protection {
    pub level: Level,
    pub mode: ProtectionMode,
    pub protected: Vec<usize>,
    /// The support rows of protected orbits in the judged map: the net map under Net, the net map of the first
    /// refused prefix under Strict.
    pub conflicts: Vec<SupportRow>,
    pub verdict: Verdict,
    /// Strict only: the length of the shortest executed literal prefix whose net map moves a protected sticker.
    /// `None` for a clear verdict and under Net, which judges only the end of the complete operation.
    pub first_refused_prefix: Option<u64>,
}

/// A set of pieces protected at label level: any of the 177,120 pieces, fixed cell centres included, named by home
/// position as [`Kernel::positions`] names them. A nonempty set is bound to the model that built it, and the checked
/// calls refuse it in another model's kernel (KN1.4). The empty set names no piece, so it belongs to no model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProtectedPieces {
    pieces: Vec<usize>,
    model_id: Option<String>,
}

impl ProtectedPieces {
    /// The empty set.
    pub fn none() -> ProtectedPieces {
        ProtectedPieces::default()
    }

    /// The protected pieces in increasing order, without repeats.
    pub fn pieces(&self) -> &[usize] {
        &self.pieces
    }

    /// The model whose pieces the set names; `None` for the empty set.
    pub fn model_id(&self) -> Option<&str> {
        self.model_id.as_deref()
    }
}

/// One protected piece of a [`PieceProtection`]: where its stickers are in the base state and after the judged map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PieceRow {
    /// The piece, by its home position.
    pub piece: usize,
    /// Its orbit; -1 for a fixed cell centre.
    pub orbit: i32,
    /// `before[i]` is the slot that holds, in the base state, the sticker whose home slot is the piece's i-th home slot
    /// in increasing order: the base's [`PiecePosition::slots`].
    pub before: Vec<i32>,
    /// The slots of the same stickers after the judged map: the net map under Net; under Strict, the first refused
    /// prefix, or the whole word when no prefix is refused.
    pub after: Vec<i32>,
}

impl PieceRow {
    /// The judged map moves a sticker of the piece: its slots or its orientation change.
    pub fn moves(&self) -> bool {
        self.before != self.after
    }
}

/// The `protection` row of KN2.4 for named pieces, at label level as in 600-cell-Full (LB3.4). Unlike an orbit's, a
/// piece's row depends on the base state, which says where its stickers are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PieceProtection {
    pub level: Level,
    pub mode: ProtectionMode,
    /// The protected pieces in increasing order.
    pub protected: Vec<usize>,
    /// One row per protected piece, in the order of `protected`.
    pub rows: Vec<PieceRow>,
    /// `Moves` when the judged map moves a sticker of a protected piece.
    pub verdict: Verdict,
    /// Strict only: the length of the shortest executed literal prefix that moves a sticker of a protected piece.
    /// `None` for a clear verdict and under Net, which judges only the end of the complete operation.
    pub first_refused_prefix: Option<u64>,
}

impl PieceProtection {
    /// The protected pieces the judged map moves, in increasing order.
    pub fn moved(&self) -> Vec<usize> {
        self.rows.iter().filter(|row| row.moves()).map(|row| row.piece).collect()
    }
}

/// A certificate of a protection refusal: the judged map moves the sticker in slot `source`, of the protected orbit
/// `orbit`, to slot `destination`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovedSticker {
    pub orbit: usize,
    pub source: i32,
    pub destination: i32,
}

/// A derived state (KN2.11).
#[derive(Clone, Debug)]
pub struct Derived {
    pub state: Snapshot,
    /// The digest of the base state.
    pub base: String,
    /// `Digest<literal word>` of the recipe.
    pub word: String,
    pub effect: Effect,
    pub protection: Protection,
}

/// A refused derivation, KN2.11's `{first_refused_prefix, outcome, certificates}`: the outcome is `refusal`. The base
/// state is unchanged, and the effect is still reported.
#[derive(Clone, Debug)]
pub struct Refused {
    pub base: String,
    pub word: String,
    /// Under Strict, the length of the shortest executed literal prefix that moves a protected sticker. `None` under
    /// Net, which judges only the end of the complete operation, so the judged operation is the whole word.
    pub first_refused_prefix: Option<u64>,
    pub refusal: Refusal,
    /// Per protected orbit that the judged map moves, its smallest moved slot and that sticker's destination.
    pub certificates: Vec<MovedSticker>,
    pub effect: Effect,
    pub protection: Protection,
}

/// The result of [`Kernel::derive`].
#[derive(Clone, Debug)]
pub enum Derivation {
    Derived(Derived),
    Refused(Refused),
}

/// A cell named 0 to 599, with the refusal text of 0.4's `Model.cell_mask` (its `cell_distances` says `Color must be
/// C1..C600` instead; one text serves every structure row here).
fn named_cell(cell: i64) -> Result<usize, Refusal> {
    usize::try_from(cell)
        .ok()
        .filter(|&cell| cell < model::CELLS)
        .ok_or_else(|| Refusal::new(codes::NAMED_OBJECT_INVALID, format!("Cell must be 0..{}", model::CELLS - 1)))
}

/// The piece protection row from its rows: `Moves` when one of them moves.
fn piece_verdict(
    mode: ProtectionMode,
    protected: &ProtectedPieces,
    rows: Vec<PieceRow>,
    first_refused_prefix: Option<u64>,
) -> PieceProtection {
    let verdict = if rows.iter().any(PieceRow::moves) { Verdict::Moves } else { Verdict::Clear };
    PieceProtection {
        level: Level::Labels,
        mode,
        protected: protected.pieces.clone(),
        rows,
        verdict,
        first_refused_prefix,
    }
}

/// One row of 0.4's `PuzzleState.progress()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressRow {
    pub orbit: usize,
    pub pieces: usize,
    pub solved: usize,
    pub position_wrong: usize,
    pub orientation_wrong: usize,
    pub color_solved: usize,
    /// The census `rank`.
    pub rank: i64,
    /// The census `colors`.
    pub stickers: i64,
    /// The census `orientation_group`.
    pub orientation: String,
}

impl ProgressRow {
    pub fn to_json(&self) -> Value {
        let count = |n: usize| Value::Int(n as i64);
        json::object([
            ("orbit", count(self.orbit)),
            ("pieces", count(self.pieces)),
            ("solved", count(self.solved)),
            ("position_wrong", count(self.position_wrong)),
            ("orientation_wrong", count(self.orientation_wrong)),
            ("color_solved", count(self.color_solved)),
            ("rank", Value::Int(self.rank)),
            ("stickers", Value::Int(self.stickers)),
            ("orientation", Value::String(self.orientation.clone())),
        ])
    }
}

/// The progress rows of a state and the SHA-256 of their canonical JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub rows: Vec<ProgressRow>,
    pub sha256: String,
}

/// The loaded 600-cell-Full model and its star cache.
pub struct Kernel {
    model: Model,
    structure: structure::Structure,
    solved: Snapshot,
    stars: Mutex<engine::Stars>,
}

impl Kernel {
    /// Loads and checks the model in the `assets` folder.
    pub fn load(assets: &Path) -> Result<Kernel, LoadError> {
        let model = Model::load(assets)?;
        let structure = structure::Structure::build(&model)?;
        let solved = Snapshot::new(&model.model_id, (0..SLOTS as i32).collect());
        Ok(Kernel { model, structure, solved, stars: Mutex::new(engine::Stars::new()) })
    }

    /// The identity block of the loaded model.
    pub fn identity(&self) -> IdentityBlock {
        IdentityBlock::full(&self.model.model_id)
    }

    /// The SHA-256 of the loaded `assets/manifest.json`.
    pub fn manifest_sha256(&self) -> &str {
        &self.model.manifest_sha256
    }

    /// Refuses with `KN1:input.identity_mismatch`, naming the first differing field, unless `identity` is the loaded
    /// model's block.
    pub fn check_identity(&self, identity: &IdentityBlock) -> Result<(), Refusal> {
        let field = if identity.puzzle != Puzzle::Full {
            "puzzle"
        } else if identity.model_id != self.model.model_id {
            "model_id"
        } else if identity.menu_id.is_some() {
            "menu_id"
        } else if identity.menu_order.is_some() {
            "menu_order"
        } else if identity.contract_rev.is_some() {
            "contract_rev"
        } else {
            return Ok(());
        };
        Err(Refusal::new(
            codes::IDENTITY_MISMATCH,
            format!("The identity block differs from the loaded model in {field}"),
        ))
    }

    /// The registry's answer for `identity`; `KN1:input.unknown_model` for any identity but the loaded one.
    pub fn model_info(&self, identity: &IdentityBlock) -> Result<ModelInfo, Refusal> {
        if self.check_identity(identity).is_err() {
            return Err(Refusal::new(codes::UNKNOWN_MODEL, "The identity is not in the model registry"));
        }
        Ok(ModelInfo {
            identity: self.identity(),
            puzzle_name: PUZZLE_NAME,
            manifest_sha256: self.model.manifest_sha256.clone(),
            verified_files: self.model.verified_files,
            slots: SLOTS,
            pieces: PIECES,
            orbits: ORBITS,
            generators: GENERATORS,
        })
    }

    fn same_model(&self, model_id: &str) -> Result<(), Refusal> {
        if model_id == self.model.model_id {
            Ok(())
        } else {
            Err(Refusal::new(codes::IDENTITY_MISMATCH, "The value belongs to another model_id"))
        }
    }

    /// A nonempty protected set must name this model's orbits; the empty set names none.
    fn same_set(&self, protected: &ProtectedSet) -> Result<(), Refusal> {
        protected.model_id.as_deref().map_or(Ok(()), |model_id| self.same_model(model_id))
    }

    /// A nonempty set of protected pieces must name this model's pieces; the empty set names none.
    fn same_pieces(&self, protected: &ProtectedPieces) -> Result<(), Refusal> {
        protected.model_id.as_deref().map_or(Ok(()), |model_id| self.same_model(model_id))
    }

    /// The solved state.
    pub fn solved(&self) -> Snapshot {
        self.solved.clone()
    }

    /// The state with these labels, if they are a permutation of the slots. This is how a stored state object is read
    /// back (LP2, M6). It does not decide whether the state is reachable: that stays the caller's evidence, such as
    /// the stored object's digest.
    pub fn state_from_labels(&self, labels: Vec<i32>) -> Option<Snapshot> {
        if labels.len() != SLOTS {
            return None;
        }
        let mut seen = vec![false; SLOTS];
        for &label in &labels {
            let slot = usize::try_from(label).ok().filter(|&slot| slot < SLOTS)?;
            if std::mem::replace(&mut seen[slot], true) {
                return None;
            }
        }
        Some(Snapshot::new(&self.model.model_id, labels))
    }

    /// The `positions` row of KN2.4: where each named piece's stickers are in `state`. A piece is named by its home
    /// position, as 0.4's `PuzzleState.piece` names positions, with 0.4's refusal text; repeats are allowed.
    pub fn positions(&self, state: &Snapshot, pieces: &[i64]) -> Result<Vec<PiecePosition>, Refusal> {
        self.same_model(state.model_id())?;
        let named = |p: &i64| usize::try_from(*p).ok().filter(|&p| p < PIECES);
        let Some(pieces) = pieces.iter().map(named).collect::<Option<Vec<usize>>>() else {
            let text = format!("Position must be an integer in 0..{}", PIECES - 1);
            return Err(Refusal::new(codes::NAMED_OBJECT_INVALID, text));
        };
        let mut now = vec![0i32; SLOTS];
        for (slot, &label) in state.labels().iter().enumerate() {
            now[label as usize] = slot as i32;
        }
        let piece_of = |slot: i32| self.model.slot_piece[slot as usize] as usize;
        let rows = pieces.into_iter().map(|piece| {
            let home = self.model.home_slots(piece).to_vec();
            let slots: Vec<i32> = home.iter().map(|&h| now[h as usize]).collect();
            let place = piece_of(slots[0]);
            let together =
                self.model.home_slots(place).len() == slots.len() && slots.iter().all(|&slot| piece_of(slot) == place);
            let orbit = self.model.piece_orbit[piece];
            PiecePosition { piece, orbit, home, slots, position: together.then_some(place) }
        });
        Ok(rows.collect())
    }

    /// Two cells compared (LP2 draft, the row `structure.compare` reads): the fewest face steps between them, as 0.4's
    /// structure explorer counts them. A cell is named 0 to 599, refused with 0.4's `cell_mask` text. It reads no
    /// state.
    pub fn compare_cells(&self, first: i64, second: i64) -> Result<CellComparison, Refusal> {
        let (first, second) = (named_cell(first)?, named_cell(second)?);
        Ok(CellComparison { first, second, steps: self.structure.steps(first, second) })
    }

    /// The layers around `cell` (LP2 draft, the counts `filter.preview` reads for a layer), from layer 0 to the cell's
    /// last: in each, the positions with a home slot in a cell that many face steps from `cell`, and their slots. A
    /// position counts whole and can be in several layers, as in 0.4's `layer_mask`. The counts read no state: a
    /// position keeps its slots whatever piece it holds, so 0.4's `home_layer(C, L)`, the pieces whose home is in the
    /// layer, counts the same.
    pub fn cell_layers(&self, cell: i64) -> Result<Vec<LayerCounts>, Refusal> {
        let cell = named_cell(cell)?;
        let counts = self.structure.layers(&self.model, cell).into_iter().enumerate();
        Ok(counts.map(|(layer, (pieces, slots))| LayerCounts { cell, layer, pieces, slots }).collect())
    }

    /// One layer around `cell`, as [`Kernel::cell_layers`] counts it, with 0.4's `layer_mask` refusal of a layer past
    /// the cell's last.
    pub fn layer_counts(&self, cell: i64, layer: i64) -> Result<LayerCounts, Refusal> {
        let (_, layer) = self.named_layer(cell, layer)?;
        Ok(self.cell_layers(cell)?[layer])
    }

    /// The members of one layer around `cell` (LP2 draft, the row `structure.layer` reads), with
    /// [`Kernel::layer_counts`]' refusals: the positions it counts, named by their home positions, and their home
    /// slots, each list in increasing order. The members read no state, as the counts do.
    pub fn layer_members(&self, cell: i64, layer: i64) -> Result<LayerMembers, Refusal> {
        let (cell, layer) = self.named_layer(cell, layer)?;
        let (pieces, slots) = self.structure.members(&self.model, cell, layer);
        Ok(LayerMembers { cell, layer, pieces, slots })
    }

    /// A cell 0 to 599 and a layer around it from 0 to the cell's last, refused with 0.4's `cell_mask` and
    /// `layer_mask` texts.
    fn named_layer(&self, cell: i64, layer: i64) -> Result<(usize, usize), Refusal> {
        let cell = named_cell(cell)?;
        let last = self.structure.last_layer(cell);
        let Some(layer) = usize::try_from(layer).ok().filter(|&layer| layer <= last) else {
            let text = format!("Layer must be L0..L{last} for the chosen color");
            return Err(Refusal::new(codes::NAMED_OBJECT_INVALID, text));
        };
        Ok((cell, layer))
    }

    /// The number of frame nodes in `orbit`'s execution tree.
    pub fn star_nodes(&self, orbit: usize) -> Option<usize> {
        self.model.trees.get(orbit).map(|tree| tree.parent.len())
    }

    /// Normalizes a recipe in 0.4's JSON form, a list of `{"kind": "word", "moves": [...]}` and
    /// `{"kind": "star", "orbit": o, "node": n, "sign": s}` steps, with 0.4's checks, order and texts.
    pub fn recipe(&self, value: &Value) -> Result<Recipe, Refusal> {
        let (steps, length) = engine::normalize(&self.model, value)?;
        Ok(Recipe { model_id: self.model.model_id.clone(), steps, length })
    }

    /// A one-word recipe, with the checks of a word step.
    pub fn word(&self, letters: &[i64]) -> Result<Recipe, Refusal> {
        let word = engine::word(letters.iter().map(|&m| Some(m)), letters.len())?;
        let length = word.len();
        Ok(Recipe { model_id: self.model.model_id.clone(), steps: vec![Step::Word(word)], length })
    }

    /// A protected set of moving orbits 0 to 34; repeats are allowed, as in 0.4's preferences.
    pub fn protected_set(&self, orbits: &[i64]) -> Result<ProtectedSet, Refusal> {
        let mut set = Vec::with_capacity(orbits.len());
        for &orbit in orbits {
            match usize::try_from(orbit) {
                Ok(orbit) if orbit < ORBITS => set.push(orbit),
                _ => return Err(Refusal::new(codes::NAMED_OBJECT_INVALID, "Bad protected orbit list")),
            }
        }
        set.sort_unstable();
        set.dedup();
        let model_id = (!set.is_empty()).then(|| self.model.model_id.clone());
        Ok(ProtectedSet { orbits: set, model_id })
    }

    /// The effect of a recipe, with star nets from the star cache.
    pub fn effect(&self, recipe: &Recipe) -> Result<Effect, Error> {
        self.same_model(&recipe.model_id)?;
        let mut map: Vec<i32> = (0..SLOTS as i32).collect();
        let mut scratch = Vec::new();
        {
            let mut stars = self.stars.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            for step in &recipe.steps {
                match step {
                    Step::Word(word) => {
                        for &m in word {
                            let (source, destination) = self.model.pairs(m);
                            engine::apply(&mut map, source, destination, &mut scratch);
                        }
                    }
                    Step::Star(star) => {
                        let net = stars.star(&self.model, star)?;
                        if star.sign > 0 {
                            engine::apply(&mut map, &net.0, &net.1, &mut scratch);
                        } else {
                            engine::apply(&mut map, &net.1, &net.0, &mut scratch);
                        }
                    }
                }
            }
        }
        let (source, destination) = engine::net_from_map(&map);
        let letters = self.expansion(recipe);
        Ok(self.effect_of(source, destination, engine::digest_ints(&letters), letters.len()))
    }

    /// The effect of a recipe by replaying every letter of its expansion: the reference of the KN2.9 agreement test.
    pub fn effect_by_replay(&self, recipe: &Recipe) -> Result<Effect, Error> {
        self.same_model(&recipe.model_id)?;
        let letters = self.expansion(recipe);
        let mut map: Vec<i32> = (0..SLOTS as i32).collect();
        let mut scratch = Vec::new();
        for &m in &letters {
            let (source, destination) = self.model.pairs(m);
            engine::apply(&mut map, source, destination, &mut scratch);
        }
        let (source, destination) = engine::net_from_map(&map);
        Ok(self.effect_of(source, destination, engine::digest_ints(&letters), letters.len()))
    }

    /// The literal letters of a recipe's expansion. A recipe of another model is refused, since its stars would be
    /// expanded with this model's seeds and relocations (KN1.4).
    pub fn letters(&self, recipe: &Recipe) -> Result<Vec<i32>, Refusal> {
        self.same_model(&recipe.model_id)?;
        Ok(self.expansion(recipe))
    }

    /// The letters of a recipe whose model the caller has checked.
    fn expansion(&self, recipe: &Recipe) -> Vec<i32> {
        let mut letters = Vec::with_capacity(recipe.length);
        engine::expand(&self.model, &recipe.steps, &mut letters);
        letters
    }

    /// The effect of a net map whose literal word has the digest `expansion_sha256` and `primitive_count` letters.
    fn effect_of(&self, source: Vec<i32>, destination: Vec<i32>, expansion_sha256: String, count: usize) -> Effect {
        let mut bytes = Vec::with_capacity(8 * source.len());
        for value in source.iter().chain(&destination) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let support = engine::support(&self.model, &source);
        Effect {
            net_sha256: c600_digest::sha256_hex(&bytes),
            moved_pieces: support.iter().map(|row| row.pieces).sum(),
            support,
            expansion_sha256,
            primitive_count: count,
            source,
            destination,
        }
    }

    /// The Net protection verdict of an effect for a protected set. It does not check the set's model; the checked
    /// form is [`Kernel::checked_protection`].
    pub fn protection(&self, effect: &Effect, protected: &ProtectedSet) -> Protection {
        let conflicts: Vec<SupportRow> =
            effect.support.iter().filter(|row| protected.contains(row.orbit)).cloned().collect();
        let verdict = if conflicts.is_empty() { Verdict::Clear } else { Verdict::Moves };
        Protection {
            level: Level::Labels,
            mode: ProtectionMode::Net,
            protected: protected.orbits.clone(),
            conflicts,
            verdict,
            first_refused_prefix: None,
        }
    }

    /// [`Kernel::protection`] with KN1.4's check: a set of another model's orbits is refused with
    /// `KN1:input.identity_mismatch`. An effect names no model (M4 compares the effects of two identities), so a
    /// recipe's model is checked where its effect is computed.
    pub fn checked_protection(&self, effect: &Effect, protected: &ProtectedSet) -> Result<Protection, Refusal> {
        self.same_set(protected)?;
        Ok(self.protection(effect, protected))
    }

    /// The Strict protection verdict of a recipe: the predicate is evaluated on every executed prefix of the literal
    /// expansion (KN2.7, LB2.3), and a refusal names the shortest refused prefix. Both values are checked (KN1.4).
    pub fn strict_protection(&self, recipe: &Recipe, protected: &ProtectedSet) -> Result<Protection, Refusal> {
        self.same_model(&recipe.model_id)?;
        self.same_set(protected)?;
        Ok(self.strict(&self.expansion(recipe), protected).0)
    }

    /// Strict over literal letters, with the certificates of a refusal. Letters run from source to destination, so
    /// while every earlier letter fixes the protected orbits, the next prefix moves exactly the protected stickers its
    /// last letter moves. The first refused prefix therefore ends at the first letter that moves a protected sticker,
    /// and its conflicts and certificates are that letter's.
    fn strict(&self, letters: &[i32], protected: &ProtectedSet) -> (Protection, Vec<MovedSticker>) {
        let mask = protected.mask();
        let first = letters.iter().position(|&m| self.model.letter_orbits(m) & mask != 0);
        self.strict_verdict(protected, first.map(|index| (index as u64 + 1, letters[index])))
    }

    /// The Strict row for the first refused prefix and its last letter, or for none.
    fn strict_verdict(&self, protected: &ProtectedSet, first: Option<(u64, i32)>) -> (Protection, Vec<MovedSticker>) {
        let (conflicts, certificates) = match first {
            None => (Vec::new(), Vec::new()),
            Some((_, letter)) => {
                let (source, destination) = self.model.pairs(letter);
                let moved: Vec<i32> = source.iter().zip(destination).filter(|(s, d)| s != d).map(|(&s, _)| s).collect();
                let rows = engine::support(&self.model, &moved);
                let conflicts = rows.into_iter().filter(|row| protected.contains(row.orbit)).collect();
                (conflicts, self.certificates(source, destination, protected))
            }
        };
        let protection = Protection {
            level: Level::Labels,
            mode: ProtectionMode::Strict,
            protected: protected.orbits.clone(),
            conflicts,
            verdict: if first.is_some() { Verdict::Moves } else { Verdict::Clear },
            first_refused_prefix: first.map(|(prefix, _)| prefix),
        };
        (protection, certificates)
    }

    /// A protected set of pieces, any of 0 to 177,119 with the fixed cell centres; repeats are allowed. A bad piece is
    /// refused as [`Kernel::positions`] refuses it, with `KN2:input.named_object_invalid` and 0.4's text.
    pub fn protected_pieces(&self, pieces: &[i64]) -> Result<ProtectedPieces, Refusal> {
        let named = |p: &i64| usize::try_from(*p).ok().filter(|&p| p < PIECES);
        let Some(mut set) = pieces.iter().map(named).collect::<Option<Vec<usize>>>() else {
            let text = format!("Position must be an integer in 0..{}", PIECES - 1);
            return Err(Refusal::new(codes::NAMED_OBJECT_INVALID, text));
        };
        set.sort_unstable();
        set.dedup();
        let model_id = (!set.is_empty()).then(|| self.model.model_id.clone());
        Ok(ProtectedPieces { pieces: set, model_id })
    }

    /// Net protection of named pieces at label level (KN2.4, LB3.4): each protected piece's stickers in `base` and
    /// after the effect's net map, and `Moves` when the map moves one of them. A fixed cell centre's sticker is moved by
    /// no letter, so its row is always clear; that is computed, not assumed. Another model's base or set is refused
    /// (KN1.4); an effect names no model, so a recipe's model is checked where its effect is computed.
    pub fn piece_protection(
        &self,
        base: &Snapshot,
        effect: &Effect,
        protected: &ProtectedPieces,
    ) -> Result<PieceProtection, Refusal> {
        self.same_model(base.model_id())?;
        self.same_pieces(protected)?;
        let carry = |slot: i32| effect.source.binary_search(&slot).map_or(slot, |at| effect.destination[at]);
        let rows = self.piece_rows(base, protected, carry);
        Ok(piece_verdict(ProtectionMode::Net, protected, rows, None))
    }

    /// Strict protection of named pieces at label level: every executed prefix of the recipe's literal expansion is
    /// judged (KN2.7, LB2.3). Letters run from source to destination, so while every earlier letter fixes the
    /// protected stickers they stay in their base slots, and the first refused prefix ends at the first letter that
    /// moves a slot holding one; the rows give the slots after that prefix. Another model's base, recipe or set is
    /// refused (KN1.4).
    pub fn strict_piece_protection(
        &self,
        base: &Snapshot,
        recipe: &Recipe,
        protected: &ProtectedPieces,
    ) -> Result<PieceProtection, Refusal> {
        self.same_model(base.model_id())?;
        self.same_model(&recipe.model_id)?;
        self.same_pieces(protected)?;
        let unmoved = self.piece_rows(base, protected, |slot| slot);
        let mut held = vec![false; SLOTS];
        for row in &unmoved {
            for &slot in &row.before {
                held[slot as usize] = true;
            }
        }
        // Whether primitive |m| moves a held slot; a letter and its inverse move the same slots.
        let mut touches: Vec<Option<bool>> = vec![None; GENERATORS];
        let mut first = None;
        for (index, &m) in self.expansion(recipe).iter().enumerate() {
            let seen = &mut touches[(m.unsigned_abs() - 1) as usize];
            let hit = *seen.get_or_insert_with(|| {
                let (source, destination) = self.model.pairs(m);
                source.iter().zip(destination).any(|(&s, &d)| s != d && held[s as usize])
            });
            if hit {
                first = Some((index as u64 + 1, m));
                break;
            }
        }
        let Some((prefix, letter)) = first else {
            return Ok(piece_verdict(ProtectionMode::Strict, protected, unmoved, None));
        };
        let (source, destination) = self.model.pairs(letter);
        let mut image = std::collections::HashMap::new();
        for (&s, &d) in source.iter().zip(destination) {
            if held[s as usize] {
                image.insert(s, d);
            }
        }
        let rows = self.piece_rows(base, protected, |slot| image.get(&slot).copied().unwrap_or(slot));
        Ok(piece_verdict(ProtectionMode::Strict, protected, rows, Some(prefix)))
    }

    /// The rows of the protected pieces: their stickers' slots in `base`, and where `carry` takes each slot.
    fn piece_rows(&self, base: &Snapshot, protected: &ProtectedPieces, carry: impl Fn(i32) -> i32) -> Vec<PieceRow> {
        if protected.pieces.is_empty() {
            return Vec::new();
        }
        let mut now = vec![0i32; SLOTS];
        for (slot, &label) in base.labels().iter().enumerate() {
            now[label as usize] = slot as i32;
        }
        let rows = protected.pieces.iter().map(|&piece| {
            let before: Vec<i32> = self.model.home_slots(piece).iter().map(|&home| now[home as usize]).collect();
            let after = before.iter().map(|&slot| carry(slot)).collect();
            PieceRow { piece, orbit: self.model.piece_orbit[piece], before, after }
        });
        rows.collect()
    }

    /// Per protected orbit that a map moves, its smallest moved slot and where that sticker goes.
    fn certificates(&self, source: &[i32], destination: &[i32], protected: &ProtectedSet) -> Vec<MovedSticker> {
        let mut first: Vec<Option<MovedSticker>> = vec![None; ORBITS];
        for (&s, &d) in source.iter().zip(destination) {
            let orbit = self.model.slot_orbit[s as usize];
            if s == d || !protected.contains(orbit) {
                continue;
            }
            let entry = &mut first[orbit as usize];
            if entry.is_none_or(|certificate| s < certificate.source) {
                *entry = Some(MovedSticker { orbit: orbit as usize, source: s, destination: d });
            }
        }
        first.into_iter().flatten().collect()
    }

    /// Applies a recipe's net map to `base` unless a protected orbit would move under Net protection (KN2.11). A
    /// refusal leaves `base` and every digest unchanged and still reports the effect. Another model's base, recipe
    /// or protected set is refused (KN1.4).
    pub fn derive(&self, base: &Snapshot, recipe: &Recipe, protected: &ProtectedSet) -> Result<Derivation, Error> {
        self.derive_in(ProtectionMode::Net, base, recipe, protected)
    }

    /// [`Kernel::derive`] under Strict protection: every executed literal prefix is judged, and a refusal reports the
    /// first refused prefix and its certificates (KN2.7, KN2.11).
    pub fn derive_strict(&self, base: &Snapshot, recipe: &Recipe, set: &ProtectedSet) -> Result<Derivation, Error> {
        self.derive_in(ProtectionMode::Strict, base, recipe, set)
    }

    fn derive_in(
        &self,
        mode: ProtectionMode,
        base: &Snapshot,
        recipe: &Recipe,
        protected: &ProtectedSet,
    ) -> Result<Derivation, Error> {
        self.same_model(base.model_id())?;
        self.same_set(protected)?;
        let effect = self.effect(recipe)?;
        let (protection, certificates) = match mode {
            ProtectionMode::Net => {
                let certificates = self.certificates(&effect.source, &effect.destination, protected);
                (self.protection(&effect, protected), certificates)
            }
            ProtectionMode::Strict => self.strict(&self.expansion(recipe), protected),
        };
        Ok(self.conclude(base.digest(), base.labels(), effect, protection, certificates))
    }

    /// The derivation from a judged effect: the refusal with its prefix and certificates, or the derived state.
    fn conclude(
        &self,
        base_digest: &str,
        base: &[i32],
        effect: Effect,
        protection: Protection,
        certificates: Vec<MovedSticker>,
    ) -> Derivation {
        let (base_digest, word) = (base_digest.to_owned(), effect.expansion_sha256.clone());
        if protection.verdict == Verdict::Moves {
            let refusal = Refusal::new(codes::ORBIT_WOULD_MOVE, PROTECTED_MESSAGE);
            return Derivation::Refused(Refused {
                base: base_digest,
                word,
                first_refused_prefix: protection.first_refused_prefix,
                refusal,
                certificates,
                effect,
                protection,
            });
        }
        let mut labels = base.to_vec();
        engine::apply(&mut labels, &effect.source, &effect.destination, &mut Vec::new());
        let state = Snapshot::new(&self.model.model_id, labels);
        Derivation::Derived(Derived { state, base: base_digest, word, effect, protection })
    }

    /// The literal letters of a recipe in chunks of at most `size` letters, each with the position of its first
    /// letter (KN2.10). Only one step's expansion is held at a time.
    pub fn chunks<'a>(&'a self, recipe: &'a Recipe, size: NonZeroUsize) -> Result<Chunks<'a>, Refusal> {
        self.same_model(&recipe.model_id)?;
        let (model, steps, size) = (&self.model, &recipe.steps[..], size.get());
        Ok(Chunks { model, steps, pending: Vec::new(), used: 0, position: 0, size })
    }

    /// A whole-word replay of literal letters pushed in chunks of any size, judged under Strict protection on every
    /// executed prefix (KN2.4 `replay`, KN2.10). No length bound applies, since the bound on what the kernel replays
    /// comes from DT-J1's experiment X3 and is unset. Another model's base or protected set is refused (KN1.4).
    pub fn replay(&self, base: &Snapshot, protected: &ProtectedSet) -> Result<Replay<'_>, Refusal> {
        self.replay_with(base, protected, None)
    }

    /// [`Kernel::replay`] under the 3,000,000-primitive compatibility ceiling, counted over all chunks (LB2.4): a push
    /// that would pass it is refused with `KN2:resource_limit.primitive_ceiling` and changes nothing.
    pub fn replay_under_ceiling(&self, base: &Snapshot, protected: &ProtectedSet) -> Result<Replay<'_>, Refusal> {
        self.replay_with(base, protected, Some(engine::MAX_PRIMITIVES as u64))
    }

    fn replay_with(&self, base: &Snapshot, set: &ProtectedSet, ceiling: Option<u64>) -> Result<Replay<'_>, Refusal> {
        self.same_model(base.model_id())?;
        self.same_set(set)?;
        Ok(Replay {
            kernel: self,
            base: base.digest().to_owned(),
            labels: base.labels().to_vec(),
            map: (0..SLOTS as i32).collect(),
            scratch: Vec::new(),
            protected: set.clone(),
            mask: set.mask(),
            ceiling,
            length: 0,
            first: None,
            digest: c600_digest::Sha256::new(),
        })
    }

    /// The progress rows of a state, as 0.4's `PuzzleState.progress()`.
    pub fn progress(&self, state: &Snapshot) -> Result<Progress, Refusal> {
        self.same_model(state.model_id())?;
        let rows = engine::progress(&self.model, state.labels());
        let text = Value::Array(rows.iter().map(ProgressRow::to_json).collect()).canonical();
        Ok(Progress { sha256: c600_digest::sha256_hex(text.as_bytes()), rows })
    }
}

/// A chunk of a recipe's literal letters: `letters[i]` is the letter at position `start + i` of the expansion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub start: u64,
    pub letters: Vec<i32>,
}

/// The literal letters of a recipe in chunks ([`Kernel::chunks`]).
pub struct Chunks<'a> {
    model: &'a Model,
    steps: &'a [Step],
    pending: Vec<i32>,
    used: usize,
    position: u64,
    size: usize,
}

impl Iterator for Chunks<'_> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Chunk> {
        let mut letters = Vec::new();
        while letters.len() < self.size {
            if self.used == self.pending.len() {
                let Some((step, rest)) = self.steps.split_first() else { break };
                self.steps = rest;
                self.pending.clear();
                self.used = 0;
                engine::expand(self.model, std::slice::from_ref(step), &mut self.pending);
                continue;
            }
            let take = (self.size - letters.len()).min(self.pending.len() - self.used);
            letters.extend_from_slice(&self.pending[self.used..self.used + take]);
            self.used += take;
        }
        if letters.is_empty() {
            return None;
        }
        let start = self.position;
        self.position += letters.len() as u64;
        Some(Chunk { start, letters })
    }
}

/// A whole-word replay fed in chunks ([`Kernel::replay`]). It gives no verdict before [`Replay::finish`], so a part
/// not yet replayed is never judged safe; a refusal is known as soon as its prefix has been pushed.
pub struct Replay<'k> {
    kernel: &'k Kernel,
    base: String,
    labels: Vec<i32>,
    map: Vec<i32>,
    scratch: Vec<i32>,
    protected: ProtectedSet,
    mask: u64,
    ceiling: Option<u64>,
    length: u64,
    /// The first refused prefix and its last letter.
    first: Option<(u64, i32)>,
    /// `Digest<literal word>` so far: the letters as little-endian 32-bit integers, hashed as they are pushed, so a
    /// streamed word keeps no copy of its letters (KN2.10, LG4.6).
    digest: c600_digest::Sha256,
}

impl Replay<'_> {
    /// Replays the next letters. A chunk with a letter outside ±1..±1,200, or one that would pass the ceiling of
    /// [`Kernel::replay_under_ceiling`], is refused whole and changes nothing.
    pub fn push(&mut self, letters: &[i64]) -> Result<(), Refusal> {
        if !letters.iter().all(|&m| model::valid_primitive(m)) {
            return Err(Refusal::new(codes::RECIPE_SHAPE, engine::BAD_PRIMITIVE));
        }
        if self.ceiling.is_some_and(|ceiling| self.length + letters.len() as u64 > ceiling) {
            return Err(Refusal::new(codes::PRIMITIVE_CEILING, engine::CEILING));
        }
        let model = &self.kernel.model;
        for (at, &m) in (self.length + 1..).zip(letters) {
            let m = m as i32;
            if self.first.is_none() && model.letter_orbits(m) & self.mask != 0 {
                self.first = Some((at, m));
            }
            let (source, destination) = model.pairs(m);
            engine::apply(&mut self.map, source, destination, &mut self.scratch);
            self.digest.update(&m.to_le_bytes());
        }
        self.length += letters.len() as u64;
        Ok(())
    }

    /// The number of letters replayed so far.
    pub fn position(&self) -> u64 {
        self.length
    }

    /// The first refused prefix so far. `None` says only that no prefix replayed so far is refused.
    pub fn first_refused_prefix(&self) -> Option<u64> {
        self.first.map(|(prefix, _)| prefix)
    }

    /// The end of the word: the derived state, or the refusal at the first refused prefix with its certificates. The
    /// effect is the whole word's, as in [`Kernel::derive_strict`]. A word of no letters derives the base state.
    pub fn finish(self) -> Derivation {
        let kernel = self.kernel;
        let (source, destination) = engine::net_from_map(&self.map);
        let word = c600_digest::hex(&self.digest.finish());
        let effect = kernel.effect_of(source, destination, word, self.length as usize);
        let (protection, certificates) = kernel.strict_verdict(&self.protected, self.first);
        kernel.conclude(&self.base, &self.labels, effect, protection, certificates)
    }
}
