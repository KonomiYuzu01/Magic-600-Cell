//! The host-side display-plan and publication contract of chapter 10 (RPI) and the renderer feature matrix (RM),
//! for LP3. It has no renderer, no GPU code and no command layer. It has four parts:
//!
//! - **Plans and publications** ([`plan`], [`array`], [`source`], [`canonical`], [`validate`]): the types of RPI.5 to
//!   RPI.8 and RPI.20 to RPI.23 with RM.6/RM.7's payloads, and the host validator of RPI.10, RM.11, RM.12 and RM.20.
//! - **The publication state machine** ([`host`], [`receipt`]): RM.17 to RM.26, RM.32, RM.35 and RM.36, with LI.25's
//!   standalone and scheduled publication, over RM.14's adapter operations. A mock backend drives it. Receipts mirror
//!   KN5.21 locally; nothing depends on `c600-command`.
//! - **Picks and stamps** ([`pick`], [`host`]): RM.10, RM.24, RPI.24 and RPI.25. A pick resolves by instance ID through
//!   the installed source map, never through a chart inverse.
//! - **Charts and stages** ([`exact`], [`chart`], [`reference`], [`stage`]): exact chart-property checks at exact poses
//!   and `admit_chart` (RPI.28 to RPI.30, RPI.34, RPI.48, LB4.2), the binary32 CPU reference evaluation with its
//!   coverage and reproducibility key (RPI.37, RPI.56), and the headless part of the user-stage contract: declared
//!   outputs, composition order and presented-preview identity (RPI.68, RPI.69, RPI.75, RPI.76).
//!
//! [`produce`] builds the 600-cell-Full producers and fixture contexts; [`jumble`] builds the jumbling producers
//! (posed stickers, certified transforms, certified overlays, the seam table and the slot projection) from M5's kernel,
//! `c600_jumble`; [`crosswalk`] holds RM.37's jumbling vectors as fixture types. Evidence here is source and
//! synthetic only (RM.30): no Windows/DirectX, input, long-session or performance result.
//!
//! # Drafts
//!
//! H-08 (the command table) and the IR are drafts until their freezes. Where the text leaves a choice open or names no
//! code, this crate takes the smallest reasonable option:
//!
//! - RM.12: validation precedence is invalid, then unsupported, then ceiling. A malformed publication is
//!   `EV3:input.display_plan_invalid` even when it also names an unsupported row; only a publication with neither is
//!   counted against the ceilings.
//! - RPI.4: a plan with an unsupported dimension is `EV3:input.display_unsupported` and the rest of that plan is not
//!   checked, because its d-shaped payloads have no admitted shape.
//! - RPI.6, RPI.10: every node, plan and source map reads the publication's base revision; a snapshot subject is that
//!   base with no derivation; dependency keys are strings (`state`, `workspace`, `protection`, `input:<name>`).
//! - RPI.20, RPI.25: instance IDs are unique within one publication; the host binds the view and its generation at
//!   installation, and an instance's base and subject are its plan's.
//! - RM.10: every link digest of a map entry is listed in that map's `sources`, and a range's bytes are pinned in the
//!   publication.
//! - RPI.65, EVD.32: a `Word` content needs its key among the node's claims and a record for that key; otherwise the
//!   publication is invalid. A relation node with no claim key is invalid.
//! - RM.8, RM.9: a program publication that carries a preview row is invalid; a preview reaches a view only from the
//!   engine ([`host::Host::show_preview`]).
//! - RPI.15, M5: posed stickers and certificate overlays are unsupported on 600-cell-Full. On the jumbling puzzle
//!   the posed row, the certified transform, the certified overlay and the projection chain are produced from M5's
//!   states ([`jumble`]); of the J07 to J12 payloads only J08's seam table is read from the engine, and the rest
//!   are still checked only as fixtures. User stages (F05, F26) stay
//!   unsupported: no stage module reaches the host, and [`stage`] checks fixture descriptions only.
//! - RPI.15, J02 (render/1 section 3): a posed row is validated only for a jumbling identity and a pose-level subject
//!   (a Jumble-on revision or a derived state at level poses): 177120 pose indices inside the table, finite row-major
//!   d x d matrices with the identity at pose 0, and one lattice entry per pose, -1 or a K+ index below 7200
//!   ([`validate::K_PLUS`]), pose 0's being index 0. A malformed row in a program publication is `EV3:input.display_plan_invalid` under
//!   RM.12's precedence; `EV3:input.display_bundle_mismatch` stays the engine bundle's code (RPI.9). The row's array
//!   hashes are its array descriptors. Its exact state digest is the plan subject's digest, which the validator
//!   cannot recompute from binary32 arrays: the producer binds it ([`jumble::revision`], [`jumble::bundle`]).
//! - RPI.15, J02: a pose entry (a + b sqrt 5)/d is evaluated in binary64, each operation rounded, and then rounded
//!   to binary32 ([`jumble::approximate`]). The solved state's three arrays equal the hashes the J1 fixture records;
//!   the arrays of a later J1 stage (`mid`, `end`) are not compared here, so a non-identity pose matrix's orientation
//!   (row-major, acting on column vectors) and its rounding are checked only by m·mᵀ = I, which a transposed table
//!   also passes. A pose with no finite binary32 value is missing data, not a row.
//! - RPI.17, RM.15: the jumbling fixture report ([`jumble::support_report`]) supports J02 to J05, reports F01, F03
//!   and F04 unsupported (the jumbling puzzle uses J02, J03 and J05 in their place) and omits J01 and J06 to J12.
//!   The jumbling context binds the unchanged geometry digests of [`produce::GeometryInput`]; J01's area-weighted
//!   shrink anchors are not an asset and are not bound.
//! - J03: a jumbling transform is built only from an admissible outcome's certified inside set
//!   ([`jumble::transform`]). `c600_jumble` names no rotation plane, so a transform built from it alone has no swept
//!   preview.
//! - J04: a full named-cap set is the engine's whole `straddler_set`, each piece with both exact points as compact
//!   `[a,b,d]` text and their binary32 vectors; a set its budget left incomplete is [`jumble::Missing::Uncertain`],
//!   never a shorter set. The cap-map witness is a classification's one certificate. The named cut's plane is the
//!   pole normal and kappa as exact text, and alpha is the model's 121/125. A survey is carried only when supplied.
//! - J08: a seam table is read from the engine's `seam_facts` answer ([`jumble::seam_table`]): every mismatched cell,
//!   on-grip or off-grip, with a domain and a cell identity local to that table (ranks of the engine's exact domain
//!   key and chamber pair). It is complete only when the engine's batch was, so an interrupted batch has no total N.
//!   It is a payload, not a supported row: no J08 plan is produced or published.
//! - RPI.23, J13: the jumbling slot projection ([`jumble::PoseProjection`]) answers a sticker of a piece at the lattice
//!   pose k with the slot k·s of its home slot s, from M8's slot action of K+ with A3's frame check
//!   (`c600_jumble::levels::Tables::piece_slots`; at the identity, the home slot), and a piece off the lattice with
//!   `undefined(off_lattice)`. An engine error of the level tables is [`jumble::Missing::Engine`] and no projection,
//!   never an answer. A piece, as on 600-cell-Full, has no single slot.
//! - KN1.8, KN1.10: each slot projection holds one level, the label projection ([`pick::LabelProjection`]) labels and
//!   the pose projection poses ([`plan::StateRef::level`]). A subject at the other level is
//!   `undefined(KN1:input.level_not_held)` before any digest is compared, so nothing is lifted from labels into poses.
//! - RM.6, F03: a transform without the engine's plane has no swept preview ([`validate::swept_preview`]); its
//!   endpoints are state revisions with `to > from`.
//! - RM.6: exact-set digests are the SHA-256 of sorted piece IDs as little-endian u32, or of canonical object text
//!   one per line ([`plan::piece_set_digest`], [`plan::object_set_digest`]).
//! - RPI.34, RPI.48: a chart property a plan names is established only by check results in
//!   [`validate::Context::checks`]; with none, it is `not_established`. RPI.36: a plan's precision is an opaque name.
//! - RM.11: the counted fields are [`validate::COUNTED`]; the stage, fragment, frame, view-vertex and adapter fields
//!   need the stage module checker (RPI.71), the weight table or a backend. `overlay_items` counts overlay objects,
//!   certificate pieces and two points per certificate pair. RPI.50: every ceiling here is a declared fixture value.
//! - RM.13, LI.16: the canonical output bytes are the canonical JSON text of the publication plus its array and source
//!   bytes; only a scheduled publication is charged against an output allowance.
//! - KN5.21: a request digest is the SHA-256 of the canonical publication; a scheduled request's digest names its view
//!   and allowance. A publication has no `committed` status.
//! - ERR.8: errors carry code, form, context and summary only.
//! - RM.22: an error after acceptance takes form `stop`, also for a standalone publication with no job.
//! - RPI.10, RPI.12, RM.22: a standalone publication whose declared dependencies are not current is refused with
//!   `EV3:stale.display_plan`; a scheduled one ends `cancelled(replaced_revision)`; a dependency the host does not
//!   track is taken as current.
//! - RPI.10: a publication to a view that is not open is invalid.
//! - RM.24: one surface per view; the frame stamp lists the host's subject label of every installed plan.
//! - RPI.55, RM.33: [`host::MockBackend`] stands for an adapter; it says nothing about any candidate backend.
//! - RPI.34 [O]: the repertoire is the first option, a restricted family with known checkers: the stereographic chart
//!   declared exactly by its pole ([`chart::Stereographic`]) over regions given as convex hulls of exact vertices, with
//!   the properties `domain`, `injective` and `section` ([`chart::Property`]). Anything else has no checker, and a
//!   property without an established check stays `not_established`, never false.
//! - RPI.30, LB4.2: an exact pose is the identity or a matrix verified to be in K⁺, the 600-cell's rotation group
//!   (determinant 1, the 120 unit icosians mapped onto themselves). Every float matrix, and every exact matrix outside
//!   K⁺, is refused with `LB4:type.approximate_property_input` and never rounded. Domain is an exact cone test
//!   (Carathéodory bases with Cramer signs) with a vertex-sum certificate that the origin is outside; injectivity is a
//!   theorem for a flat region (four independent vertices, all in their affine hull), refuted by two vertices in the
//!   domain on one ray; a section is decided on the posed vertices. A verdict's scope names its region, its vertex
//!   count and its pose.
//! - KN3.2, KN3.3: the checks compute in Q(√5) with checked i128 parts, local to this crate, because `c600-kernel` has
//!   neither KN3's polytope operations nor an unbounded Q(√5). An overflow, a zero pole or an unmet precondition
//!   decides nothing (`not_established`).
//! - RPI.28, RPI.48: a check result names its chart by content identity ([`chart::identity`]: dimension, precision and
//!   each stage's name, version, definition and parameter bits, not the claimed properties) and its property by the
//!   name the plan claims; a property's parameters, such as a section's slab, belong in that name when a plan must tell
//!   them apart. A check's identity is the digest of its chart, property and scope.
//! - RPI.48: `admit_chart` admits a chart when each claimed property has an established result for it and none is
//!   refuted. A refuted result refuses the chart with `EV3:evidence.chart_check_failed`, naming the chart, the property
//!   and the check, and withholds every claimed property's status; anything else is `not_established`. A chart that
//!   claims nothing is admitted.
//! - RPI.48, RM.20: a refused chart does not refuse its publication. The publication installs, that chart's output has
//!   coverage [`plan::Coverage::Undefined`] with the error's code ([`chart::ChartAdmission`]), the other plans
//!   continue, and the check results, the session and every receipt are unchanged.
//! - RPI.36 [O], RPI.35: the reference fixtures declare their own rule ([`reference::PrecisionRule`]): an absolute
//!   bound per output coordinate against the exact image in Q(√5), outside the exclusion neighbourhood
//!   1 − u·p ≤ `exclusion` of the pole, for the normalised point u and pole p, where a vertex is missing with reason
//!   `excluded` (RPI.36's omission option, per vertex; triangles that cross the neighbourhood are not handled). The
//!   image is written in R⁴, in the pole's orthogonal complement, as (u − (u·p)p) / (1 − u·p).
//! - RPI.37: the origin is decided exactly at every pose, the singular ray at an exact pose; any other missing point is
//!   a binary32 display decision (`excluded`, `non_finite`). A missing point enters neither the draw nor the pick list.
//! - RPI.56: the reproducibility key is the SHA-256 of the exact fields: plan, subject, base, chart identity,
//!   declaration, pose (an exact pose's identity, or an approximate pose's declared label, never its float bits), the
//!   vertices' digest, the rule's bits and the exactly decided missing vertices. Approximate values are compared with
//!   the exact image under the declared rule, never bitwise.
//! - RPI.68, RPI.69, RPI.75: a stage description ([`stage::StageDescription`]) declares its kind, digest and outputs.
//!   A vertex stage declares exactly a position and a coverage code, a fragment stage exactly opaque RGB. An alpha,
//!   discard, depth, stencil or coverage output, a repeated or missing output and a digest that is not 64 lowercase
//!   hex digits are `LG2:type.stage_rule_violation`, in its registered form `diagnostic`.
//! - RPI.76, RM.26, ONE.41: a frame draws the view's chain, then a preview by the chain that draws it, then each host
//!   status once, in the order preview, stale, protection breakthrough, selection. Only a preview drawn by the built-in
//!   chain is in the presented report and can bind a commit token; drawn through a user stage it is an extra display.
//!
//! Not built: a decoder from `c600_run::value::Value` (it has no list type, so it needs an LP2 change) and the
//! ceilings named above that need a weight table or a backend. For the jumbling puzzle: J01's shrink anchors, J06's
//! backend readback, plans for the J07 to J12 rows and producers for their payloads other than the seam table (domain
//! and rim summaries, covers and repair, release and word checks, nu), J10's exposed-face geometry, an engine-named
//! plane for a swept preview, and a comparison of the posed arrays with a J1 stage after the start.
//! For user stages: the stage module's parser and checker (RPI.69's body rules, RPI.70's operation tiers and RPI.71's
//! costs need the stage language and RM.38's weight table), and HLSL compilation (RPI.78), the raster bound (RPI.72),
//! quarantine (RPI.77), frame reports (RM.41) and composition on a device, which need a backend; no user stage reaches
//! the host, so F05 and F26 stay unsupported and CT-RPI-32 runs only its headless part. For charts: KN3's polytope
//! operations, any chart or property outside the drafted repertoire (RPI.34 stays open), the GPU agreement check
//! (RPI.35) and a production precision rule (RPI.36 stays open). CT-RPI-19's headless Linux leg is not run here.

#![forbid(unsafe_code)]

pub mod array;
pub mod canonical;
pub mod chart;
pub mod crosswalk;
pub mod error;
pub mod exact;
pub mod host;
pub mod jumble;
pub mod pick;
pub mod plan;
pub mod produce;
pub mod receipt;
pub mod reference;
pub mod source;
pub mod stage;
pub mod validate;

pub use error::{codes, Error, Form};
pub use host::{Backend, Host, MockBackend};
pub use receipt::{CancelReason, Receipt, Status};
pub use validate::{validate, Context, LimitProfile, Validated};
