//! The research-archive sub-checkers of stage 18 (tail completion): streamed expansion and hash, composed strict′ contract, literal replay with chunk chaining, runner records (piece R5).
//!
//! Each sub-checker is a file of this folder with its own [`Sources`](crate::report::Sources) and one line in
//! [`SUBCHECKERS`]. They read the archive's records as their certificates and its stored members as inputs:
//!
//! | Sub-checker | Certificate | Claim family (matrix) | Gate (LB5.12) |
//! | --- | --- | --- | --- |
//! | [`expansion`] | a `steps.json` record, either form | 18/expansion | gate 1: exact length before expansion, stored words' lengths and hashes |
//! | [`composed`] | a `steps.json` record, either form | 18/composed | gate 2: the composed strict′ contract from the actual chained starts |
//! | [`tail_replay`] | a `steps.json` record, either form | 18/tail_replay | gate 3: complete twist-by-twist replay in chunks whose digests chain |
//! | [`lattice_run`] | `results/lattice-d1/run/run.json` | 18/lattice_run | the run record, report, tables, trees and flat words |
//! | [`lattice_replay`] | `results/lattice-d1/run/run.json` | 18/lattice_run | a fresh literal replay of the 13,231,266 letters |
//!
//! Shared code: [`words`] (stream readers), [`record`] (the `steps.json` model and each step's files), [`play`]
//! (letter-by-letter play and strict′) and [`lattice`] (the run record). A start table the archive does not store is
//! rebuilt by exact replay of stored words from a stored table, under the checking sub-checker's identity, and marked
//! `derived` in its trace (the owner's LP5-3). Gate 3 stays `historical_check_incomplete` for T6's step 4 and the rule's
//! three stages and is never passed for them. Static whole-cap plans, whole-cap negatives and plan tables (matrix
//! family 18/static_tail) have no sub-checker: every sub-checker answers `not_applicable` for them with that scope.
//! Every other certificate is read in the sub-checker's exact form, and a departure from it (a field missing, renamed
//! or unknown, a list emptied or shortened, a record of another sub-checker) fails, naming the field.

pub mod composed;
pub mod expansion;
pub mod lattice;
pub mod lattice_replay;
pub mod lattice_run;
pub mod play;
pub mod record;
pub mod tail_replay;
pub mod words;

use super::SubChecker;

/// The stage's registered sub-checkers.
pub static SUBCHECKERS: &[&SubChecker] = &[
    &expansion::SUBCHECKER,
    &composed::SUBCHECKER,
    &tail_replay::SUBCHECKER,
    &lattice_run::SUBCHECKER,
    &lattice_replay::SUBCHECKER,
];
