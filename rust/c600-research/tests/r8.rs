//! LP5 piece R8: the KN6 check job, its run record and reproduction, its reports in the evidence graph, and the
//! archive's runs as recorded. One test binary, one module per conformance test it runs a half of.
//!
//! Test names carry the CT ID and part (PHS.62). Each module's doc names the half that runs here and what the LP2
//! half ran. Everything here is source evidence: nothing is Windows, DirectX, input, long-session or performance
//! evidence, no producer runs and no search runs.

#[path = "r8/common.rs"]
mod common;
#[path = "r8/ct_det_01.rs"]
mod ct_det_01;
#[path = "r8/ct_det_02.rs"]
mod ct_det_02;
#[path = "r8/ct_det_11.rs"]
mod ct_det_11;
#[path = "r8/ct_det_24.rs"]
mod ct_det_24;
#[path = "r8/ct_evd_30.rs"]
mod ct_evd_30;
#[path = "r8/ct_lg3_05.rs"]
mod ct_lg3_05;
#[path = "r8/ct_lg5_02.rs"]
mod ct_lg5_02;
#[path = "r8/stop.rs"]
mod stop;
