//! The 600 Cell Script interpreter (route RT1) and the `lp1-exec-draft-1` reference meter (LP2, milestone M2).
//!
//! - [`meter`] is the reference meter of chapter 09 (DET.55 to DET.60). `tests/traces.rs` replays the 40 frozen
//!   contract-vector traces through it.
//! - [`rng`] is the counter-addressed generator `sha256-counter-draft-1` (DET.61).
//! - [`value`] holds run-time values with DET.57's sizes, exact literals, exact arithmetic and equality; [`approx`]
//!   holds `Approx<p>` on exact integers; [`encode`] writes canonical output bytes (DET.59, FP.5, FP.6).
//! - [`interp`] runs a loaded IR program under the meter, one charge row per operation (DET.55, DET.56).
//! - [`provenance`] reads, checks and builds FP.17's provenance blocks (milestone M3).
//! - [`boundary`] is the run-time boundary under capability separation: stored words keep their blocks, preview
//!   admission reads them, and a solving run gets one fixed acknowledgement (chapter 07, milestone M3).
//! - [`menu`] is the menu alphabet a run pins for LB1.8's literal words of one menu, and its identity block (LP2
//!   draft).
//! - [`menu_key`] is LG2.18's word key `word-key/1` of a menu word under the item matrices of the run's pinned menu
//!   (LP2 draft).
//! - [`trace`] is LB1.10's trace key `trace-key/1` of a Full word under the independence relation a run pins (LP2
//!   draft).
//! - [`contract`] runs a whole-state contract once per revision (LG4.10; LP2 draft).
//! - [`step`] stops a run at each of its LG4/DET.20 boundaries and shows a gate where it is, on the caller's thread
//!   (EV2.12; LP4-5).
//! - [`tower`] holds the identity tower's run-time values: canonical IDs, digests, costs, certificates, sum values and
//!   the handles of the values a host makes (T1c; T1 drafts).

#![forbid(unsafe_code)]

pub mod approx;
pub mod boundary;
pub mod contract;
pub mod encode;
pub mod interp;
pub mod menu;
pub mod menu_key;
pub mod meter;
pub mod provenance;
pub mod rng;
pub mod step;
pub mod tower;
pub mod trace;
pub mod value;
