//! The research-archive sub-checkers of stage 17 (routing in the driver) (piece R4).
//!
//! - [`macros`] (`s17.macro`): the chosen descents of P*** (PX3) and RR-7304 (`macro-certificate/1`,
//!   `macro-check/1`), each from its exact table with the jam word as its prefix and the jam end as its start.
//! - [`rr`] (`s17.rr`): the five tails of Lemma RR against brute force at the recorded gate inputs H2 and H2b of
//!   `test_rr.py` (no lemma); the RR, P*** and transport stops `not_applicable` with their bounds.

pub mod macros;
pub mod rr;

use super::SubChecker;

/// The stage's registered sub-checkers.
pub static SUBCHECKERS: &[&SubChecker] = &[&macros::SUB, &rr::SUB];
