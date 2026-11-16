//! The headless view model, view lane and view budget of 600 Cell Script (LP3 draft). There is no renderer here.
//!
//! The crate is the host side of chapter 10's view interfaces without a device: which views are open, which engine
//! revision they show, which program output may attach to it, what the view lane runs next and what the views cost.
//! A renderer, a display-plan validator and a user-stage checker sit outside it.
//!
//! # Modules
//!
//! - [`revision`]: KN0.2's counters and digest, ONE.8's [`Base`], ONE.63's dependency match and ONE.10's input key.
//! - [`bundle`]: RPI.9's [`validate_engine_bundle`]. A complete bundle is adopted whole or not at all (ONE.60, ONE.61).
//! - [`limits`]: RM.11's [`LimitProfile`], RM.12's admission ceiling and checks, RM.13's [`OutputAllowance`].
//! - [`publication`]: the content of one `publish_view` call as admission counts it, with its canonical bytes (RPI.10,
//!   RM.11, RM.13, RM.20).
//! - [`receipt`]: KN5.21's receipt statuses and LI.28's job-outcome mapping.
//! - [`model`]: the [`ViewModel`]: Global, Local and further views (RPI.42, RPI.81 to RPI.83), the view lane (KN7.2,
//!   LI.22 to LI.25, ONE.64, RPI.12), revision binding and statuses (ONE.12, ONE.63, ONE.65, RPI.13), visibility
//!   (RPI.11, RM.25) and the view budget (RM.19, RM.38, RM.39, RPI.82).
//!
//! # Seams
//!
//! Engine revisions arrive as [`BundleCandidate`]s over this crate's own small [`Revision`] type; no command-layer
//! crate is used. Program work leaves the model as [`Job`]s ([`ViewModel::take_jobs`]) and returns as a
//! [`RunOutcome`] ([`ViewModel::finish`]), so a closure, a test double or a runtime can run it. The command layer
//! supplies command IDs and the [`Origin`] of `view.open` and `view.close`.
//!
//! # Evidence
//!
//! Source and synthetic evidence only. Every ceiling in the tests is a fixture value: nothing is measured, and
//! nothing here advertises support or a profile (RPI.82, RM.40). No timing is measured or claimed.
//!
//! # Contract tests
//!
//! `tests/view_rows.rs` holds CT-RPI-03, CT-RPI-06, CT-RPI-07, CT-RPI-22 (its limit half), CT-RPI-36, CT-RPI-37,
//! CT-RM-02, the accounting rows 1, 2, 7 and 9 of CT-RM-09 and CT-ONE-30, each test named after its ID.
//!
//! # Drafts
//!
//! Where a clause leaves a choice open or names no code, this crate takes the smallest reasonable option:
//!
//! - RPI.83 names no code for a recomputation that names `view.open` or `view.close`. It is refused with the
//!   existing `LG3:capability.not_granted`. The command layer states the caller as an [`Origin`].
//! - LI.22, RPI.42: Global and Local have the fixed `ViewId`s `global` and `local`. They are always open and always
//!   visible; hide and show leave them unchanged.
//! - RPI.83, KN5.21: `view.open` and `view.close` return a `Result` at admission. KN5.21 has no completion kind for
//!   them, and they never hold the runner slot. A close is a newer request on the view's lane, so its unfinished work
//!   ends `cancelled(superseded(by: close))` (LI.23).
//! - RPI.83: the host declares a view's dependencies, per-frame workload and per-run output allowance when it opens
//!   ([`Declared`]). The view's first run is a host-scheduled recomputation.
//! - LI.25: host-scheduled recomputations get command IDs in their own namespace, [`CommandId::Scheduled`].
//! - LI.25, RPI.12: a pending run binds its base when it starts, so it always runs at the newest adopted revision.
//! - RPI.11, RM.25: runs and publications start only while their view is visible. Hiding cancels the running request
//!   as well as the pending one, `cancelled(by_user)`, and keeps an invalidation, so showing runs once.
//! - RPI.13, charter D2: status precedence. Installed output decides first (`current`, `empty` or `stale`); without
//!   it, a failed newest request is `error`, a newest request waiting for a slot held by fenced work is `busy`, any
//!   other request is `loading`, and nothing is `empty`.
//! - RPI.42, RPI.81: Global and Local run no program of their own; a further view runs its source, built-in or
//!   overlay, through the runner. Standalone host publications may target any open view.
//! - RM.20: a standalone publication naming an unknown view or another generation, and a solver-invoked run naming
//!   Global, Local or no open view, are refused with `EV3:stale.display_plan`.
//! - RM.17, LI.23: a standalone publication is prepared synchronously once the runner slot is free.
//! - RM.12: several excesses report the first one in RM.11's table order.
//! - RM.11: unit tokens (`records`, `weighted_ops`, `invocations_per_pixel`, `vertex_evals_per_frame`, `bytes`,
//!   `resident_bytes`); a profile entry with another unit is invalid.
//! - RM.11, RPI.82: a view's physical reservation (`adapter_cpu_bytes`, `adapter_gpu_bytes`) is the workload declared
//!   at open, including its pending generation; its publications reuse it.
//! - RM.38, RPI.71: `frame_work` uses the declared workload. A built-in chain declares zero stage cost; each view
//!   has its own surface pixels; `overdraw` is the profile's whole-number ceiling. A resize that goes over falls the
//!   resized view back; fallen-back stages return in open order once a recharge fits.
//! - RM.39, KN7.8: eviction drops a hidden view's output and retained read results and moves its resources to safe
//!   retirement, so showing it again runs its program.
//! - KN0.2, RPI.9, ONE.60: one view model serves one session in one engine epoch. A bundle older than the adopted one,
//!   or reusing its counters with other content, is `refresh_required(EV3:input.display_bundle_mismatch)`;
//!   re-delivering the identical bundle changes nothing.
//! - ONE.10, KN7.3, KN7.8: a read result can reappear only for a host-scheduled run of the same view. Its key is the
//!   state digest and the workspace and protection counters of its declared parts. Reuse is charged as a miss,
//!   output included. A view keeps at most [`model::CACHE_LIMIT`] results.
//! - LI.16, RM.13: the canonical bytes stand in for H8's canonical uncompressed output bytes (a version byte,
//!   unsigned LEB128 integers and length-prefixed byte strings). The [`Transport`] of a publication is never charged.
//!
//! # Not built
//!
//! - CT-RM-09 rows 3 to 6, 8 and 10 (the raster bound `U`, `stage_work_exceeded` frame reports, device faults and
//!   quarantine) need a user-stage checker or a device.
//! - CT-RPI-22's chart half (chart admission) needs the display-plan validator.
//! - RM.38's weight table, device loss (`busy.renderer_unavailable`), `display_unsupported` and command-ID reuse
//!   belong to the renderer, the backend and the command layer.

#![forbid(unsafe_code)]

pub mod bundle;
pub mod error;
pub mod limits;
pub mod model;
pub mod publication;
pub mod receipt;
pub mod revision;

/// A SHA-256 digest.
pub type Digest = [u8; 32];

pub use bundle::{validate_engine_bundle, BundleCandidate, BundlePart, CompleteBundle, Mode, PartKind};
pub use error::{code, BundleFault, Context, Error, Form, PlanFault};
pub use limits::{Field, LimitProfile, OutputAllowance, ProfileError};
pub use model::{
    Counters, Declared, Draw, FrameInput, FrameView, Installed, Job, Origin, RetireToken, RunOutcome, StageUse, Usage,
    ViewId, ViewModel, ViewSource, ViewStatus, Workload,
};
pub use publication::{Content, MapDescriptor, PlanDescriptor, PublicationInput, Transport};
pub use receipt::{
    CancelReason, CommandId, Completion, CompletionKind, JobId, JobOutcome, JobResult, ResultRef, Status,
};
pub use revision::{Base, Depends, InputKey, Part, Revision};
