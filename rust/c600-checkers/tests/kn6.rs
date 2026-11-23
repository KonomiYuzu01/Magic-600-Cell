//! KN6's checkers on fixtures built from the kernels and on the repository's certificates, read in place.
//!
//! Test names carry the CT ID and part (PHS.62):
//!
//! - CT-KN6-01 (source, part: the mutation gates of KN6.1's four kinds other than research archives, admissibility
//!   and straddlers, macro certificates, compressed-word expansion and ν and label checkpoints, each fixture failing
//!   under a tampered claim, a wrong start table and a wrong input digest, with the report's identities and the
//!   archive kind's registration point; not run: the gates of the research-archive kind's sub-checkers, which
//!   `kn6_s14.rs`, `kn6_r4.rs`, `s18.rs` and `kn6_s19.rs` run).
//! - CT-KN6-03 (source, part: the gate harness on repository certificates, and stage 15's `macro-certificate/1`
//!   certificates passing in place at the six chain ends; not run: the other records of stages 14 to 19, which
//!   the stage files run, some of them only in ignored tests).
//!
//! LP2's half of CT-KN6-02, the import scan, is `tests/test_language_scans.py`.

mod kn6 {
    pub mod admissibility;
    pub mod checkpoints;
    pub mod common;
    pub mod core;
    pub mod expansion;
    pub mod macros;
}
