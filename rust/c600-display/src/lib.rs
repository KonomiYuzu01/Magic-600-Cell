//! The host-side display-plan and publication contract of chapter 10 (RPI) and the renderer feature matrix (RM),
//! for LP3. It has no renderer, no GPU code and no command layer. It has five parts:
//!
//! - **Plans and publications** ([`plan`], [`array`], [`source`], [`canonical`], [`validate`]): the types of RPI.5 to
//!   RPI.8 and RPI.20 to RPI.23 with RM.6/RM.7's payloads, and the host validator of RPI.10, RM.11, RM.12 and RM.20.
//! - **The publication state machine** ([`host`], [`receipt`]): RM.17 to RM.26, RM.32, RM.35 and RM.36, with LI.25's
//!   standalone and scheduled publication, over RM.14's adapter operations, among them engine adoption (RM.16), frame
//!   reports (RM.41) and picks. A mock backend drives it. Receipts mirror KN5.21 locally; nothing depends on
//!   `c600-command`.
//! - **Picks and stamps** ([`pick`], [`host`]): RM.10, RM.24, RPI.24 and RPI.25. A pick resolves by instance ID through
//!   the installed source map, never through a chart inverse.
//! - **Charts and stages** ([`exact`], [`chart`], [`reference`], [`stage`], [`raster`]): exact chart-property checks at
//!   exact poses and `admit_chart` (RPI.28 to RPI.30, RPI.34, RPI.48, LB4.2), the binary32 CPU reference evaluation
//!   with its coverage and reproducibility key (RPI.37, RPI.56), and the headless part of the user-stage contract: the
//!   take-up of stages that LP2's stage checker admits, declared outputs, composition order and presented-preview
//!   identity, and RPI.72's raster bound with the frame stop, the quarantine and the frame reports (RPI.68, RPI.69,
//!   RPI.72, RPI.75 to RPI.77, RM.41).
//! - **The relationship strip's data contract** ([`strip`]): the fields of one typed relation that EV3.8 lets EN3's
//!   view model carry without deciding the strip's design, EV3.9's two missing values, and the check that a record can
//!   be shown on a relation node whose claims carry its statement key (RPI.65).
//!
//! [`produce`] builds the 600-cell-Full producers and fixture contexts; [`jumble`] builds the jumbling producers
//! (posed stickers, certified transforms, certified overlays, the seam table and the slot projection) from M5's kernel,
//! `c600_jumble`; [`crosswalk`] holds RM.37's jumbling vectors as fixture types. Evidence here is source and
//! synthetic only (RM.30): no Windows/DirectX, input, long-session or performance result.
//!
//! # Drafts
//!
//! The IR is a draft until its freeze, and H-08 (the command table) is frozen at LP3 (PHS.53). Where the text leaves a
//! choice open or names no code, this crate takes the smallest reasonable option:
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
//! - EV3.8, EV3.9: a strip record ([`strip::StripRelation`]) states one relation over operands of one role, each
//!   operand and value one canonical object ID, and is shown only on a relation node of its base revision whose claims
//!   carry its statement key ([`strip::attach`]). The strip module's documentation lists its other drafts.
//! - RM.8, RM.9: a program publication that carries a preview row is invalid; a preview reaches a view only from the
//!   engine ([`host::Host::show_preview`]).
//! - RPI.15, M5: posed stickers and certificate overlays are unsupported on 600-cell-Full. On the jumbling puzzle
//!   the posed row, the certified transform, the certified overlay and the projection chain are produced from M5's
//!   states ([`jumble`]); the J07 to J09, J11 and J12 payloads are read from the engine ([`jumble::seam_table`],
//!   [`jumble::observations`]), and J10's payload is built from M5's certified exposed-face geometry and checked
//!   ([`jumble::exposed_faces`], [`jumble::check_exposed_faces`]). User stages (F05, F26) stay unsupported: no user
//!   stage reaches the host's publications or frames, and [`stage`] takes up the stages that LP2's checker admits
//!   beside the host.
//! - RPI.15, J02 (render/1 section 3): a posed row is validated only for a jumbling identity and a pose-level subject
//!   (a Jumble-on revision or a derived state at level poses): 177120 pose indices inside the table, finite row-major
//!   d x d matrices with the identity at pose 0, and one lattice entry per pose, -1 or a K+ index below 7200
//!   ([`validate::K_PLUS`]), pose 0's being index 0. A malformed row in a program publication is
//!   `EV3:input.display_plan_invalid` under RM.12's precedence; `EV3:input.display_bundle_mismatch` stays the engine
//!   bundle's code (RPI.9). The row's array hashes are its array descriptors. Its exact state digest is the plan
//!   subject's digest, which the validator cannot recompute from binary32 arrays: the producer binds it
//!   ([`jumble::revision`], [`jumble::bundle`]).
//! - RPI.15, J02: a pose entry (a + b sqrt 5)/d is evaluated in binary64, each operation rounded, and then rounded
//!   to binary32 ([`jumble::approximate`]). The solved state's three arrays equal the hashes the J1 fixture records;
//!   the arrays of a later J1 stage (`mid`, `end`) are not compared here, so a non-identity pose matrix's orientation
//!   (row-major, acting on column vectors) and its rounding are checked only by m·mᵀ = I, which a transposed table
//!   also passes. A pose with no finite binary32 value is missing data, not a row.
//! - RPI.17, RM.15: the jumbling fixture report ([`jumble::support_report`]) supports J02 to J05, reports F01, F03
//!   and F04 unsupported (the jumbling puzzle uses J02, J03 and J05 in their place) and omits J01 and J06 to J12.
//!   The jumbling context binds the unchanged geometry digests of [`produce::GeometryInput`]; J01's area-weighted
//!   shrink anchors are computed from it and are not an asset, so the context does not bind them.
//! - J03: a jumbling transform is built only from an admissible outcome's certified inside set and the plane the engine
//!   names for the outcome's twist ([`jumble::transform`]; `c600_jumble::twist::Twist::plane`). The engine's u, v and
//!   cos θ are exact and u and v unnormalized; the display normalizes them in binary64, rounds to binary32 and takes
//!   θ = acos(cos θ) ([`jumble::swept_plane`]), so R(θ) of render/1 section 4 is the twist up to rounding. A half-turn
//!   sweeps in the direction the engine names, and the identity has no plane and no swept preview. A twist of another
//!   grip than the outcome's is [`jumble::Missing::OtherGrip`]. Only the grip is checked, since an outcome holds no
//!   matrix. Render/1 section 4, which the J03 row cites, is a draft and RM.6 sets no tolerance, so the rounded plane
//!   is not checked for unit length or orthogonality. A twist other than the identity with no engine-named plane is
//!   [`jumble::Missing::NoPlane`], never shown as the identity.
//! - J04: a full named-cap set is the engine's whole `straddler_set`, each piece with both exact points as compact
//!   `[a,b,d]` text and their binary32 vectors; a set its budget left incomplete is [`jumble::Missing::Uncertain`],
//!   never a shorter set. The cap-map witness is a classification's one certificate. The named cut's plane is the
//!   pole normal and kappa as exact text, and alpha is the model's 121/125. A survey is carried only when supplied.
//! - J08: a seam table is read from the engine's `seam_facts` answer ([`jumble::seam_table`]): every mismatched cell,
//!   on-grip or off-grip, with a domain and a cell identity local to that table (ranks of the engine's exact domain
//!   key and chamber pair). It is complete only when the engine's batch was, so an interrupted batch has no total N.
//!   It is a payload, not a supported row: no J08 plan is produced or published.
//! - J01: the shrink anchors are the area-weighted triangle centroids of each base sticker, in binary64 with one
//!   rounding ([`jumble::shrink_anchors`]); they equal render/1's pinned digest. J06: [`jumble::readback_check`]
//!   compares a backend's readback of the three bound arrays with the engine bundle by digest and element, with its
//!   revision, device and frame stamps; no backend runs it here.
//! - J07, J09, J11, J12 ([`jumble::observations`]): the domain and lattice masks, the rim map and counts, cover
//!   tables and repair lists, word and release checks, and nu with the shipped correction catalogue are the engine's
//!   answers copied and bound to the state's pose digest. An answer that did not complete is
//!   [`jumble::Missing::Incomplete`], never a shorter table; a cut-off repair list supplies no negative. Like J08 they
//!   are payloads, not supported rows.
//! - RPI.23, J13: the jumbling slot projection ([`jumble::PoseProjection`]) answers a sticker of a piece at the lattice
//!   pose k with the slot k·s of its home slot s, from M8's slot action of K+ with A3's frame check
//!   (`c600_jumble::levels::Tables::piece_slots`; at the identity, the home slot), and a piece off the lattice with
//!   `undefined(off_lattice)`. An engine error of the level tables is [`jumble::Missing::Engine`] and no projection,
//!   never an answer. A piece, as on 600-cell-Full, has no single slot.
//! - KN1.8, KN1.10: each slot projection holds one level, the label projection ([`pick::LabelProjection`]) labels and
//!   the pose projection poses ([`plan::StateRef::level`]). A subject at the other level is
//!   `undefined(KN1:input.level_not_held)` before any digest is compared, so nothing is lifted from labels into poses.
//! - RM.6, F03: a transform without the engine's plane has no swept preview ([`validate::swept_preview`]); its
//!   endpoints are state revisions with `to > from`. On 600-cell-Full no engine names a twist's plane yet: the retained
//!   kernel names none, and which engine names it is not settled, so F03 has no swept preview there.
//! - RM.6: exact-set digests are the SHA-256 of sorted piece IDs as little-endian u32, or of canonical object text
//!   one per line ([`plan::piece_set_digest`], [`plan::object_set_digest`]).
//! - RPI.34, RPI.48: a chart property a plan names is established only by its own check results in
//!   [`validate::Context::checks`] at every surveyed exact pose and region; otherwise it is `not_established`. RPI.36:
//!   a plan's precision is an opaque name.
//! - RM.11: the counted fields are [`validate::COUNTED`]; the stage, fragment, frame, view-vertex and adapter fields
//!   need user stages in publications (F05, F26), RM.38's weight table or a backend. `overlay_items` counts overlay
//!   objects, certificate pieces and two points per certificate pair. RPI.50: every ceiling here is a declared fixture
//!   value.
//! - RM.13, LI.16: the canonical output bytes are the canonical JSON text of the publication plus its array and source
//!   bytes; only a scheduled publication is charged against an output allowance.
//! - KN5.21: a request digest is the SHA-256 of the canonical publication; a scheduled request's digest names its view
//!   and allowance. A publication has no `committed` status.
//! - ERR.8: errors carry code, form, context and summary only.
//! - RM.22: an error after acceptance takes form `stop`, also for a standalone publication with no job.
//! - RPI.10, RPI.12, RM.22: a standalone publication whose declared dependencies are not current is refused with
//!   `EV3:stale.display_plan`; a scheduled one ends `cancelled(replaced_revision)`; a dependency the host does not
//!   track is taken as current.
//! - RM.23: after a recovery, a retained publication that is shown in its open view, revalidates and is not stale is
//!   prepared again under its own command and generations once its view has no pending work; a newer publication that
//!   installs first replaces it. Until the new resources are active the view draws none of it (a frame is refused as
//!   renderer unavailable and takes no sequence number) and no pick binds to it. A stale or invalid one, or one whose
//!   preparation is refused, fails or is cancelled, waits for a new publication. No receipt changes.
//! - RPI.10: a publication to a view that is not open is invalid.
//! - RM.24: one surface per view; the frame stamp lists the host's subject label of every installed plan.
//! - RPI.55, RM.33: [`host::MockBackend`] stands for an adapter; it says nothing about any candidate backend.
//! - RM.14, RM.16: the host validates an engine bundle and adopts it through the backend's `adopt_engine`; the host's
//!   revision and its `state` dependency move to the revision the backend answers, and a bundle that either of them
//!   refuses changes nothing.
//! - RM.41: a frame report of the current device epoch whose publication generation and view generation are still
//!   installed marks that publication's fallback ([`host::Installed::fallback`]), which stays for the session as the
//!   quarantine does. Any other report is returned and marks nothing, and a report of an earlier epoch is dropped.
//! - RM.14, RPI.25: a pick names a frame the backend was given; [`host::Host::poll_pick`] resolves a stamped instance
//!   with [`host::Host::resolve_pick`]. A pick asked before a device loss is stale at `device_epoch`, a passive pick
//!   that the backend refuses as stale is dropped quietly, and the host forgets a pick once it is over.
//! - RPI.34 [O]: the repertoire is the first option, a restricted family with known checkers, as a draft that settles
//!   nothing in the chapters: the stereographic chart declared exactly by its pole ([`chart::Stereographic`]) and the
//!   projective charts x ↦ (M·x + t)/(c·x + d), affine maps and central projections among them
//!   ([`chart::Projective`]), over regions that are unions of cells, each the convex hull of exact vertices
//!   ([`chart::Region`]), with the properties `domain`, `injective`, `embedding`, `isometry`, `homomorphism` and
//!   `section` ([`chart::Property`]). Injectivity and isometry are decided on each cell, an embedding is injectivity on
//!   the union of the cells, and a homomorphism maps each cell onto a codomain cell, compared by vertex set. A
//!   stereographic chart is not a projective map, so its isometry and homomorphism have no checker and stay
//!   `not_established`, never false; a projective chart has all four checks, and a denominator with a constant
//!   negative sign is read as the map with every entry negated. Anything else has no checker.
//! - RPI.30, LB4.1, LB4.2: an exact pose is the identity or a matrix verified to be in K⁺, the 600-cell's rotation
//!   group (determinant 1, the 120 unit icosians mapped onto themselves). Every float matrix, and every exact matrix
//!   outside K⁺, is refused with `LB4:type.approximate_property_input` and never rounded. Each property is decided by
//!   its own check over the whole region, never inferred from another property or established from the vertices; a
//!   per-vertex test ([`chart::vertex_test`]) only refutes. A stereographic chart's domain is the region test
//!   `ray_meet` from the origin along the pole, cell by cell; a projective chart's domain and every section are
//!   decided on the posed vertices, to which convexity reduces them. A stereographic chart's injectivity and embedding
//!   reduce to a central projection with a functional positive on the region, one for all cells of an embedding
//!   ([`chart::CENTRAL_PROJECTION`]). The reduction is used one way: an injective central projection establishes the
//!   property on the part of the region in the domain, also when a cell meets the singular ray, and its pair refutes
//!   unless the pair lies on the singular ray. Where the reduction decides nothing, as for a region through the
//!   origin, two vertices on one ray off the singular ray may still refute. A verdict's scope names its property,
//!   region, vertex count and pose.
//! - KN3.2, KN3.3: the region checks are `c600_jumble::polytope`'s, in its unbounded Q(√5); poses, vertices and
//!   witnesses here keep checked i128 parts and convert at that boundary. An overflow, a zero pole, a polytope
//!   operation that refuses its input or an unmet precondition decides nothing (`not_established`).
//! - RPI.28, RPI.48: a check result names its chart by content identity ([`chart::identity`]: dimension, precision and
//!   each stage's name, version, definition and parameter bits, not the claimed properties) and its property by the
//!   name the plan claims; a property's parameters, such as a section's slab, belong in that name when a plan must tell
//!   them apart, after the property's name and a colon (`section:low`). Admission matches the property a check's
//!   scope names against the part of the claimed name before its first colon. A check's identity is the digest of its
//!   chart, the claimed property and its scope, which names the property whose check made it.
//! - RPI.48: `admit_chart` admits a chart when each claimed property has established results of its own check whose
//!   scopes cover every region and exact pose at which any result for that chart was checked, and none is refuted.
//!   A scope names its regions by name only, so a survey gives each region a name of its own. A
//!   refuted result refuses the chart with `EV3:evidence.chart_check_failed`, naming the chart, the property and the
//!   check, and withholds every claimed property's status; anything else is `not_established`. A chart that claims
//!   nothing is admitted.
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
//! - RPI.68, RPI.69: the display takes a stage up through LP2's stage checker. [`stage::take_up`] turns a stage that
//!   `c600_check::admit_stage` admitted with no finding into a [`stage::CheckedStage`]; otherwise it returns every
//!   finding in the checker's order with the checker's code, in that code's registered form (`refusal` for
//!   `LG4:input.ir_version_unsupported` and the checker's two LG3 capability codes, which `admit_stage` does not give
//!   today, `diagnostic` for the others), naming its module, function, path and types. [`stage::check_outputs`] stays
//!   public: `c600-repr`'s chart path still makes a checked stage from a description of its own, until it reads
//!   admitted stages.
//! - RPI.78: a stage's digest is the SHA-256 of its module name, its entry name and the checked definition's Rust
//!   debug text ([`stage::describe`]). It stands in for a digest of the IR's canonical encoding, which waits for the IR
//!   freeze, and it never depends on the host's limits.
//! - RPI.68, RPI.69, RPI.75: a stage description ([`stage::StageDescription`]) declares its kind, digest and outputs.
//!   A vertex stage declares exactly a position and a coverage code, a fragment stage exactly opaque RGB. An alpha,
//!   discard, depth, stencil or coverage output, a repeated or missing output and a digest that is not 64 lowercase
//!   hex digits are `LG2:type.stage_rule_violation`, in its registered form `diagnostic`.
//! - RPI.76, RM.26, ONE.41: a frame draws the view's chain, then a preview by the chain that draws it, then each host
//!   status once, in the order preview, stale, protection breakthrough, selection. Only a preview drawn by the built-in
//!   chain is in the presented report and can bind a commit token; drawn through a user stage it is an extra display.
//! - RPI.72: [`raster`] is step 3's formula on the CPU, over triangles already placed on the surface in pixels and
//!   classified against the clip volume. A rectangle is rounded outward, clipped and then widened to whole quads, which
//!   are not clipped again; a rectangle with no width or height counts zero, since it bounds a triangle of no area,
//!   which covers no pixel; a non-finite coordinate is an undefined vertex; a line's quad has no caps, and a point or a
//!   line of zero length has no expansion here.
//! - RPI.72, RPI.77, RM.41: [`stage::Quarantine`] keeps one session's quarantined digests and frame reports, and the
//!   host does not call it yet. RM.41 reports "each frame that stopped a user stage". A frame over its allowance stops,
//!   quarantines its chain's digests and gives one report, whose `EV3:resource_limit.stage_work_exceeded` names the
//!   digests in one entry, comma-separated in chain order. A later frame of a quarantined chain runs the fallback, so
//!   it runs no user stage and stops none, and gets no report. A device fault quarantines with no report, because
//!   RM.41's report needs an error and the registry names no code for a device fault. A publication naming a
//!   quarantined digest ends with `EV3:input.stage_quarantined` in the form its caller names: a refusal before
//!   acceptance, a stop after it.
//!
//! Not built: a decoder from `c600_run::value::Value` (it has no list type, so it needs an LP2 change) and the
//! ceilings named above that need a weight table or a backend. For the jumbling puzzle: support and plans for the rows
//! J01 and J06 to J12 (a backend's own rows), J06's readback on a device, and a comparison of the posed arrays with a
//! J1 stage after the start.
//! For user stages, waiting for C9's backend: HLSL compilation and its compile record (RPI.78); the vertex evaluation
//! into a charged buffer, the projection and clip classification that place [`raster`]'s triangles, the expansion of
//! points and of lines of zero length, U on a device and the skipping of a stopped frame's work (RPI.72; CT-RM-10,
//! CT-RPI-31); injecting and detecting a device fault and, with CT-RPI-34, whether a frame with a device fault is
//! reported (RPI.77, RM.41); composition on a device; and the host's support for F05 and F26, with
//! [`stage::Quarantine`] wired into its publications and frames behind the backend's `read_frame_reports`. Until
//! then CT-RPI-32 and CT-RM-09's rows 3 to 6, 8 and 10 run only their headless parts, under declared fixture weights in
//! place of RM.38's measured table (RPI.79). For charts: any chart or property outside the drafted repertoire (RPI.34
//! stays open), the GPU agreement check (RPI.35) and a production precision rule (RPI.36 stays open). For the
//! relationship strip: its design, words and encoding tokens (EV3.8, EV5.11), and a strip record for a relation whose
//! value is no canonical object.

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
pub mod raster;
pub mod receipt;
pub mod reference;
pub mod source;
pub mod stage;
pub mod strip;
pub mod validate;

pub use error::{codes, Error, Form};
pub use host::{Backend, Host, MockBackend};
pub use receipt::{CancelReason, Receipt, Status};
pub use validate::{validate, Context, LimitProfile, Validated};
