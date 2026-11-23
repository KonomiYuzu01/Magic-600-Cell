//! LP5 piece R4: the research-archive sub-checkers of stages 15 to 17, on the archive read in place (read-only).
//!
//! Test names carry the CT ID and part (PHS.62): CT-LB5-02 (stage 15), CT-LB5-03 (stage 16), CT-LB5-04 (stage 17), and
//! CT-KN6-03's stage 15, 16 and 17 parts (the conformance corpus passing under the kernel's checkers, every
//! sub-checker's mutation gate). Slow cases are `#[ignore]` with their reason; one fast case of each family runs.

#[path = "kn6/common.rs"]
mod common;

mod r4 {
    pub mod s15;
    pub mod s16;
    pub mod s17;
    pub mod support;
}
