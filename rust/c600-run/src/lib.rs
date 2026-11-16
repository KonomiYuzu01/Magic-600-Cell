//! The 600 Cell Script interpreter (route RT1) and the `lp1-exec-draft-1` reference meter (LP2, milestone M2).
//!
//! - [`meter`] is the reference meter of chapter 09 (DET.55 to DET.60). `tests/traces.rs` replays the 40 frozen
//!   contract-vector traces through it.
//! - [`rng`] is the counter-addressed generator `sha256-counter-draft-1` (DET.61).
//! - [`value`] holds run-time values with DET.57's sizes, exact literals, exact arithmetic and equality; [`approx`]
//!   holds `Approx<p>` on exact integers; [`encode`] writes canonical output bytes (DET.59, FP.5, FP.6).
//! - [`interp`] runs a loaded IR program under the meter, one charge row per operation (DET.55, DET.56).

#![forbid(unsafe_code)]

pub mod approx;
pub mod encode;
pub mod interp;
pub mod meter;
pub mod rng;
pub mod value;
