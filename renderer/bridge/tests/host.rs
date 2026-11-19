//! The display host through the C function table, on the shipped model: c600-render's host scenarios
//! (`rust/c600-render/tests/support`) over [`common::Shimmed`], so every call crosses [`c600_render_ffi::DllCore`] and
//! the C shim. Every result is a synthetic fixture outcome, not a GPU run.

mod common;

use c600_render::FakeCore;

use common::support::{frames_and_batches, route_b, scenario, SIZE};
use common::Shimmed;

#[test]
fn the_host_runs_through_the_c_function_table() {
    // JSON reports start in a buffer 5000 bytes short, so every report is read twice.
    scenario(Shimmed::new(FakeCore::new(SIZE), 5000));
}

#[test]
fn frames_and_report_batches_cross_the_c_function_table() {
    frames_and_batches(Shimmed::new(FakeCore::new(SIZE), 0));
}

/// Route B (accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks) through the C function table; the recovery JSON is read
/// twice.
#[test]
fn route_b_crosses_the_c_function_table() {
    route_b(Shimmed::new(FakeCore::on_surface(SIZE), 5000));
}
