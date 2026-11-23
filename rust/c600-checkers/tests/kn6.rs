//! KN6's checkers on fixtures built from the kernels and on the repository's certificates, read in place.
//!
//! Test names carry the CT ID and part: CT-KN6-01's core part (every kind's mutation gate, the report and the
//! registration point) and CT-KN6-03's harness part (the gate harness on repository certificates, and stage 15's
//! `macro-certificate/1` certificates passing in place). LP2's half of CT-KN6-02, the import scan, is
//! `tests/test_language_scans.py`.

mod kn6 {
    pub mod admissibility;
    pub mod checkpoints;
    pub mod common;
    pub mod core;
    pub mod expansion;
    pub mod macros;
}
