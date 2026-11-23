//! Stage 18's research-archive sub-checkers (LP5 piece R5) on the archive read in place and on synthetic records
//! built from the jumbling engine.
//!
//! Test names carry the CT ID and part (PHS.62): CT-LB5-05 (stage 18: both `steps.json` forms, the lattice and runner
//! records, the archive's completed gates and mutation controls, the missing third gate), CT-LB5-09 (the recorded
//! long words through length and admission checks) and CT-LB1-04's expansion half (stage 18's 35 expansion groups).
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
