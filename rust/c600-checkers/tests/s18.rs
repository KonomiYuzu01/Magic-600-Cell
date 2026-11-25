//! Stage 18's research-archive sub-checkers (LP5 piece R5) on the archive read in place and on synthetic records
//! built from the jumbling engine.
//!
//! Test names carry the CT ID and part (PHS.62):
//!
//! - CT-LB5-05 (source, part: both `steps.json` forms and the runner and lattice records with their `ia16` words
//!   importing and passing in place, gate 1 rechecked in place on both records, the archive's mutation controls
//!   failing on synthetic records, gates 2 and 3 on synthetic records, T6's step 4 and the rule stages answering
//!   `historical_check_incomplete` for gate 3, and a shortened chain not accepted without rechecking its changed input;
//!   not run: the archive's completed gates 2 and 3 rechecked in place, T6's four steps and the rule stages under
//!   gate 2 and T6's steps 1 to 3 under gate 3, which only the ignored tests in `s18/long.rs` replay).
//! - CT-LB5-09 (source, part: the 672,417, 871,582 and 2,352,984-letter cases streamed through length and admission
//!   checks, an oversized length refused before anything is decoded, and gate 3's table telling the archive's
//!   completed replays from a new check, which gets its own report; with `s18/synthetic.rs`, T6's step 4 and the rule
//!   stages never passed; not run: the import's own reading of gate 3, which `c600-archive`'s binding data keeps
//!   `historical_check_incomplete`).
//! - CT-LB1-04 (source, part: the expansion half, stage 18's 35 expansion groups: every word file's streamed length
//!   equal to its declared length, each step's exact length computed and admitted before any word is read; not
//!   run: the library half on straight-line programs and symbolic bounds, in `c600-library`).
//! - CT-KN6-01 (source, part: the gates of stage 18's five sub-checkers: `s18.expansion` and `s18.lattice_run` on the
//!   archive's records, `s18.composed`, `s18.tail_replay` and `s18.lattice_replay` on synthetic records; not
//!   run: the other kinds and stages).
//! - CT-KN6-03 (source, part: gate 1 on both `steps.json` records and the lattice run record passing in place; not
//!   run: the in-place gate 2 and 3 replays and the lattice phase's replay, which only ignored tests run).
//!
//! The archive is read in place and never written. Inputs outside the Git-blob manifest (the stop table and stage
//! 12's 672,417-letter step) are pinned here by SHA-256.
//!
//! The replays of the archive's own steps and of the lattice phase take far longer than a test run should (about 2 to
//! 3 ms per letter on the exact engine) and are `#[ignore]`d with their reasons; one fast case per sub-checker, a
//! synthetic record where the archive has no short one, runs every time.

#[path = "kn6/common.rs"]
mod common;

mod s18 {
    pub mod archive;
    pub mod fixtures;
    pub mod long;
    pub mod synthetic;
}
