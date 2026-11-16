//! The host-side display-plan and publication contract of chapter 10 (RPI) and the renderer feature matrix (RM),
//! for LP3. It has no renderer, no GPU code and no command layer. It has three parts:
//!
//! - **Plans and publications** ([`plan`], [`array`], [`source`], [`canonical`], [`validate`]): the types of RPI.5 to
//!   RPI.8 and RPI.20 to RPI.23 with RM.6/RM.7's payloads, and the host validator of RPI.10, RM.11, RM.12 and RM.20.
//! - **The publication state machine** ([`host`], [`receipt`]): RM.17 to RM.26, RM.32, RM.35 and RM.36, with LI.25's
//!   standalone and scheduled publication, over RM.14's adapter operations. A mock backend drives it. Receipts mirror
//!   KN5.21 locally; nothing depends on `c600-command`.
//! - **Picks and stamps** ([`pick`], [`host`]): RM.10, RM.24, RPI.24 and RPI.25. A pick resolves by instance ID through
//!   the installed source map, never through a chart inverse.
//!
//! [`produce`] builds the 600-cell-Full producers and fixture contexts; [`crosswalk`] holds RM.37's jumbling vectors
//! as fixture types. Evidence here is source and synthetic only (RM.30): no Windows/DirectX, input, long-session or
//! performance result.
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
//! - RPI.15, M5: posed stickers and certificate overlays are unsupported on 600-cell-Full; jumbling payloads are
//!   checked only as fixtures. User stages (F05, F26) are unsupported: the stage checker of RPI.68 to RPI.80 is not
//!   built.
//! - RM.6, F03: a transform without the engine's plane has no swept preview ([`validate::swept_preview`]); its
//!   endpoints are state revisions with `to > from`.
//! - RM.6: exact-set digests are the SHA-256 of sorted piece IDs as little-endian u32, or of canonical object text
//!   one per line ([`plan::piece_set_digest`], [`plan::object_set_digest`]).
//! - RPI.34: every exact chart property a plan names is `not_established`. RPI.36: the precision is an opaque name.
//! - RM.11: the counted fields are [`validate::COUNTED`]; the stage, fragment, frame, view-vertex and adapter fields
//!   need the stage checker, the weight table or a backend. `overlay_items` counts overlay objects, certificate pieces
//!   and two points per certificate pair. RPI.50: every ceiling here is a declared fixture value.
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
//!
//! Not built: a decoder from `c600_run::value::Value` (it has no list type, so it needs an LP2 change), the jumbling
//! rows of CT-RPI-08 and the posed producer (M5), the user-stage checker, and the ceilings named above that need a
//! weight table or a backend.

#![forbid(unsafe_code)]

pub mod array;
pub mod canonical;
pub mod crosswalk;
pub mod error;
pub mod host;
pub mod pick;
pub mod plan;
pub mod produce;
pub mod receipt;
pub mod source;
pub mod validate;

pub use error::{codes, Error, Form};
pub use host::{Backend, Host, MockBackend};
pub use receipt::{CancelReason, Receipt, Status};
pub use validate::{validate, Context, LimitProfile, Validated};
