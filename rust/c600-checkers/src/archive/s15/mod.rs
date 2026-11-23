//! The research-archive sub-checkers of stage 15 (lowering machine) (piece R4).
//!
//! - [`macros`] (`s15.macro`): `macro-certificate/1` and `macro-check/1` from the chain ends, with the jam catalogue's
//!   chain binding; whole descents end at the jams.
//! - [`pstar`] (`s15.pstar`): the P* clean-pair census checked in full; P* and two-level P* stop and run records,
//!   beamed and unbeamed, `not_applicable` with their own bounds; `pstar2/2` recognized.
//!
//! [`jam`], [`grips`] and [`labels`] are shared with stages 16 and 17 and register nothing.

pub mod grips;
pub mod jam;
pub mod labels;
pub mod macros;
pub mod pstar;

use super::SubChecker;

/// The stage's registered sub-checkers.
pub static SUBCHECKERS: &[&SubChecker] = &[&macros::SUB, &pstar::SUB];
