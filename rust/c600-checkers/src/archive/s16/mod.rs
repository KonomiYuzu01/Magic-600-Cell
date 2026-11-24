//! The research-archive sub-checkers of stage 16 (six jams) (piece R4).
//!
//! - [`macros`] (`s16.macro`): the jam descents and P** whole descents (`macro-certificate/1`, `macro-check/1`), each
//!   from its exact table with the jam word as its prefix and the jam end as its start.
//! - [`release`] (`s16.release`): the literal traces, SJ-E1 among them, letter by letter (LB5.11).
//! - [`mobility`] (`s16.mobility`): the unit mobility records at the jams, recomputed from the full label table.
//! - [`reductions`] (`s16.reductions`): reduced transport against the recorded unreduced case, never a lemma; the
//!   routing, T1 and leaf or alphabet records `not_applicable` with their bounds.

pub mod macros;
pub mod mobility;
pub mod reductions;
pub mod release;

use super::SubChecker;

/// The stage's registered sub-checkers.
pub static SUBCHECKERS: &[&SubChecker] = &[&macros::SUB, &release::SUB, &mobility::SUB, &reductions::SUB];
