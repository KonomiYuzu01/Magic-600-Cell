//! LP5 piece R4: the research-archive sub-checkers of stages 15 to 17, on the archive read in place (read-only).
//!
//! Test names carry the CT ID and part (PHS.62); each stage's module doc marks its parts: stage 15 in `r4/s15.rs`,
//! stage 16 in `r4/s16.rs` and stage 17 in `r4/s17.rs`. Slow cases are `#[ignore]` with their reason; one fast case of
//! each family runs. An ignored test is not run by the default runner (`cargo test --locked --offline`), so a marker
//! counts it among what is not run.

#[path = "kn6/common.rs"]
mod common;

mod r4 {
    pub mod s15;
    pub mod s16;
    pub mod s17;
    pub mod support;
}
