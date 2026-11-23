//! PS3's research archive import and exchange (chapter 05 section 11; `docs/progress/1.0/language-lp5-plan.md`,
//! pieces R0 and R1).
//!
//! Import reads granted files only, keeps recognition, integrity, interpretation and recheck as separate results, keeps
//! an unmapped value word for word (PS3.10) and never executes a producer (PS3.17). Product code never walks a checkout
//! or runs Git; the Git-blob manifest is read by the test driver, which grants the extracted members as files.
//!
//! This crate is a scaffold until piece R0 lands the import table.

#![forbid(unsafe_code)]
