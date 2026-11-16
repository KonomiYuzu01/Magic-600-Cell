//! The 600 Cell Script interpreter (route RT1) and the `lp1-exec-draft-1` reference meter (LP2, milestone M2).
//!
//! - [`meter`] is the reference meter of chapter 09 (DET.55 to DET.60). `tests/traces.rs` replays the 40 frozen
//!   contract-vector traces through it.
//! - [`rng`] is the counter-addressed generator `sha256-counter-draft-1` (DET.61).

#![forbid(unsafe_code)]

pub mod meter;
pub mod rng;
