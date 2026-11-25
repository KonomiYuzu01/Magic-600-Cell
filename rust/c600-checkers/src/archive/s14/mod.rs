//! The research-archive sub-checkers of stage 14 (blocking structure): T3P rounds, P0 to P4 and chain effects, locality and fixed-point levels, literal replay rows (piece R3).
//!
//! Each sub-checker is a file of this folder with its own [`Sources`](crate::report::Sources) and one line in
//! [`SUBCHECKERS`]; [`common`] holds the exact computations they share, and each names it in its sources.
//!
//! - [`t3p`] (`s14.t3p`): `t3p-certificate/1`, `t3p-check/1` and the rounds of `t3p-chain/1`.
//! - [`literal`] (`s14.literal`): the formatless replay records' chronological rows.
//! - [`locality`] (`s14.locality`): `t3p-locality/1`, recomputed from the model.
//! - [`levels`] (`s14.levels`): `t3p-levels/1`, the greatest fixed point rebuilt from the table.
//! - [`blocks`] (`s14.blocks`): the formatless co-held, per-grip and anatomy records at the 7304 stop.
//!
//! Every sub-checker first reads the record's form ([`common::own`]): the finite searches (`t3p-scan/1`, `t3p-pairs/1`,
//! `t3p-plus/1`, the beam and peel records) and the input catalogues are `not_applicable` under every one of them,
//! with their scope, as is a well-formed record of another sub-checker's form; any other record, one with a format no
//! stage 14 form has included, is checked to its last field, so a malformed one fails at the field that departs from
//! its form.

pub mod blocks;
pub mod common;
pub mod levels;
pub mod literal;
pub mod locality;
pub mod t3p;

use super::SubChecker;

/// The stage's registered sub-checkers.
pub static SUBCHECKERS: &[&SubChecker] =
    &[&t3p::SUBCHECKER, &literal::SUBCHECKER, &locality::SUBCHECKER, &levels::SUBCHECKER, &blocks::SUBCHECKER];
