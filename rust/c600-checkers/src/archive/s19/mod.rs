//! The research-archive sub-checkers of stage 19 (height-depth framework): pose tables, Γ(Y) matching, frontier
//! exits, R3, S and R_L ranks (piece R6).
//!
//! Each sub-checker is a file of this folder with its own [`Sources`](crate::report::Sources) and one line in
//! [`SUBCHECKERS`]. They cover the claim matrix's self-contained stage 19 families (CX.15 to CX.17):
//!
//! - [`pose_table`] (`s19.pose-table`): a stored pose table, its stored and decoded digests and its full validity;
//! - [`matching`] (`s19.gamma-matching`): Γ(Y)'s finite bijection and gauge certificate, brute-forced over its rows;
//! - [`witness`] (`s19.gamma-witness`): the nine-letter forward and inverse witness with prefix transport;
//! - [`frontier`] (`s19.frontier`): the compact and cleanup frontier exits, which fail the sticker goal;
//! - [`ranks`] (`s19.ranks`): R3, S and R_L on the stored blocks, and the fixed-rank arithmetic derived anew.
//!
//! Stage 19 has no executable producer or checker of its own (LB5.16), and its "independent" reviews are model reviews
//! (EVD.52), never these checks. Everything outside these families answers `not_applicable` with its scope and never
//! passes: q11's audit, the low tails, D[k] admission, the remaining obligations and the synthetic state's
//! reachability ([`common::OUTSIDE`]). [`common`] holds what the sub-checkers share; each names it in its sources.

pub mod common;
pub mod frontier;
pub mod matching;
pub mod pose_table;
pub mod ranks;
pub mod witness;

use super::SubChecker;

/// The stage's registered sub-checkers.
pub static SUBCHECKERS: &[&SubChecker] =
    &[&pose_table::SUBCHECKER, &matching::SUBCHECKER, &witness::SUBCHECKER, &frontier::SUBCHECKER, &ranks::SUBCHECKER];
