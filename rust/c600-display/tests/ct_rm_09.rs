//! CT-RM-09 (source, part: rows 3 to 6, 8 and 10 without a device; not run: their device parts, listed below, C9),
//! the rows of RM.40's table that need a user stage: rows 3 to 6 (the raster bound U against a frame's allowance),
//! row 8 (a later frame's stop keeps the publication's receipt) and row 10 (a chain's stages quarantine together).
//! The stages are admitted by LP2's stage checker and taken up by the display (`common::stages`);
//! U is RPI.72's formula over placed triangles (`raster`); the stop, the quarantine and the frame reports are
//! `stage::Quarantine`'s (RPI.72 step 4, RPI.77, RM.41). Every limit and surface is a declared fixture value (RM.40).
//! Rows 1, 2, 7 and 9 run in `c600-view`'s `view_rows.rs`.
//!
//! Not run, since they need a device (C9): evaluating the vertex stage and projecting its positions into the triangles
//! given here, computing U on the device and skipping a stopped frame's work (CT-RM-10, CT-RPI-31), and injecting a
//! device fault (CT-RPI-34). F05 and F26 stay unsupported, so no host frame draws a user stage: row 8's chain is stated
//! beside the host, whose frame stamp and receipt the report is checked against.

mod common;

use c600_display::pick::FrameStamp;
use c600_display::plan::*;
use c600_display::raster::{self, Pass, Surface, Triangle};
use c600_display::receipt::Status;
use c600_display::stage::{Chain, CheckedStage, Fallback, FrameOutcome, Quarantine};
use c600_display::{codes, Form};
use c600_ir::build::program;
use common::stages::*;
use common::*;

/// The fixture surface, 16 × 16 pixels, and its pixel count.
const SURFACE: Surface = Surface { width: 16, height: 16 };
const P: u128 = 256;

/// Row 3's six vertices: the surface's corners and the midpoints of its right and bottom edges.
const V: [[f32; 2]; 6] = [[0.0, 0.0], [16.0, 0.0], [16.0, 8.0], [16.0, 16.0], [8.0, 16.0], [0.0, 16.0]];

fn triangles(indices: &[[usize; 3]]) -> Vec<Triangle> {
    indices.iter().map(|&[a, b, c]| Triangle::Placed([V[a], V[b], V[c]])).collect()
}

/// Five triangles over the six vertices that spread: a fan from one corner tiles the surface (128, 256, 256 and 128
/// pixels) and a sliver on the right edge covers no pixel, so U = 3 × pixels at one sample and one pass.
fn spread() -> Vec<Triangle> {
    triangles(&[[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 5], [1, 2, 3]])
}

/// Five triangles over the same six vertices that overlap: each one's rectangle is the whole surface, so U = 5 ×
/// pixels at one sample and one pass.
fn overlap() -> Vec<Triangle> {
    triangles(&[[0, 1, 3], [0, 3, 5], [1, 5, 2], [0, 2, 4], [1, 4, 0]])
}

/// U at one sample and one pass.
fn one_pass(triangles: &[Triangle]) -> u128 {
    raster::bound(SURFACE, &[Pass { triangles, samples: 1 }])
}

/// A frame stamp of view 1 from a host with an installed publication.
fn stamp() -> FrameStamp {
    let base = revision(5);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 3));
    host.frame(ViewId(1)).expect("a frame")
}

const STOPPED: FrameOutcome = FrameOutcome::Stopped(Fallback::BuiltInChain);

/// CT-RM-09, RM.40 row 3, the headless part: U against the allowance, the stop, the fallback and the quarantine. Two
/// plans with equal vertex counts, the same admitted chain and surface and `overdraw` 4: the spread plan (U = 3 ×
/// pixels) runs; the overlapping plan (U = 5 × pixels) stops with `EV3:resource_limit.stage_work_exceeded`, falls back
/// and quarantines every user-stage digest of its chain (RPI.72, RPI.77). Not run: the device's evaluation, bound and
/// skipped fragment work (C9).
#[test]
fn ct_rm_09_row_3_spread_runs_and_overlap_stops() {
    let chain = user_chain();
    let digests: Vec<String> = chain.digests().iter().map(|d| d.to_string()).collect();
    assert_eq!(digests.len(), 2);
    let allowance = raster::allowance(SURFACE, 4);
    let (spread, overlap) = (one_pass(&spread()), one_pass(&overlap()));
    assert_eq!((spread, overlap, allowance), (3 * P, 5 * P, 4 * P));

    let stamp = stamp();
    let mut quarantine = Quarantine::new();
    assert_eq!(quarantine.frame(&stamp, &chain, true, spread, allowance), FrameOutcome::Runs);
    assert!(quarantine.read_frame_reports().is_empty());
    assert!(digests.iter().all(|d| !quarantine.is_quarantined(d)));

    assert_eq!(quarantine.frame(&stamp, &chain, true, overlap, allowance), STOPPED);
    assert!(digests.iter().all(|d| quarantine.is_quarantined(d)));
    let reports = quarantine.read_frame_reports();
    assert_eq!(reports.len(), 1);
    let error = &reports[0].error;
    assert_eq!((error.code, error.form), (codes::STAGE_WORK_EXCEEDED, Form::Status));
    assert_eq!(error.context["digests"], digests.join(","));
    assert_eq!((error.context["bound"].as_str(), error.context["allowance"].as_str()), ("1280", "1024"));

    // With no built-in chain for the plan, the view shows RPI.13's error status instead (RPI.77).
    let mut quarantine = Quarantine::new();
    let outcome = quarantine.frame(&stamp, &chain, false, overlap, allowance);
    assert_eq!(outcome, FrameOutcome::Stopped(Fallback::ErrorStatus));
    assert_eq!(quarantine.read_frame_reports()[0].fallback, Fallback::ErrorStatus);
    // The bound applies to frames with a user stage: the built-in chain runs whatever the count.
    assert_eq!(quarantine.frame(&stamp, &Chain::BuiltIn, true, overlap, allowance), FrameOutcome::Runs);
}

/// CT-RM-09, RM.40 row 4, the headless part: U over shading samples and passes. The spread plan passes at one sample
/// and one pass (U = 3 × pixels), `overdraw` 4; at two samples with the ID pass, which uses the same vertex stage and
/// so the same triangles (RPI.75), U = 12 × pixels and the frame stops (RPI.72). Not run: the device's passes (C9).
#[test]
fn ct_rm_09_row_4_samples_and_the_id_pass_count() {
    let chain = user_chain();
    let allowance = raster::allowance(SURFACE, 4);
    let triangles = spread();
    let one = raster::bound(SURFACE, &[Pass { triangles: &triangles, samples: 1 }]);
    let draw = Pass { triangles: &triangles, samples: 2 };
    let id = Pass { triangles: &triangles, samples: 2 };
    let two = raster::bound(SURFACE, &[draw, id]);
    assert_eq!((one, two), (3 * P, 12 * P));

    let stamp = stamp();
    let mut quarantine = Quarantine::new();
    assert_eq!(quarantine.frame(&stamp, &chain, true, one, allowance), FrameOutcome::Runs);
    assert_eq!(quarantine.frame(&stamp, &chain, true, two, allowance), STOPPED);
    assert_eq!(quarantine.read_frame_reports()[0].error.context["bound"], (12 * P).to_string());
}

/// CT-RM-09, RM.40 row 5, the headless part: the near-plane rule of U. A triangle that crosses the near plane counts
/// the whole surface; with `overdraw` 1 the frame runs at equality and stops on any other coverage, while a triangle
/// wholly outside the clip volume or with an undefined vertex adds nothing (RPI.72, RPI.37). Not run: classifying
/// triangles against the clip volume, which is the adapter's (C9).
#[test]
fn ct_rm_09_row_5_a_near_plane_crossing_counts_the_whole_surface() {
    assert_eq!(raster::count(SURFACE, &Triangle::CrossesNear), 256);
    let allowance = raster::allowance(SURFACE, 1);
    let chain = user_chain();
    let stamp = stamp();
    let mut quarantine = Quarantine::new();
    let near = [Triangle::CrossesNear, Triangle::Outside, Triangle::Undefined];
    assert_eq!(one_pass(&near), P);
    assert_eq!(quarantine.frame(&stamp, &chain, true, one_pass(&near), allowance), FrameOutcome::Runs);

    // Any other coverage: a triangle inside one pixel counts its whole 2 × 2 quad.
    let speck = Triangle::Placed([[5.2, 5.2], [5.8, 5.2], [5.2, 5.8]]);
    let covered = [Triangle::CrossesNear, speck];
    assert_eq!(one_pass(&covered), P + 4);
    assert_eq!(quarantine.frame(&stamp, &chain, true, one_pass(&covered), allowance), STOPPED);
}

/// CT-RM-09, RM.40 row 6, the headless part: lines expand to counted triangles. Row 6's layout is 1,000 repeats of one
/// diagonal edge across the surface, `overdraw` 4, a user stage and zero triangles in the plan. The test builds no
/// plan: it starts from the edge's expansion, the two triangles of a quad of its drawn width, so U is about 2,000 ×
/// pixels (here exactly, since each triangle's rectangle clips to the whole surface) and the frame stops; U is never 0
/// (RPI.72 step 2). Not run: a plan of lines with zero triangles, which waits for the host's support for F05 and F26,
/// the adapter's expansion into the charged buffer and the device's bound (C9).
#[test]
fn ct_rm_09_row_6_a_line_only_layout_is_never_zero() {
    let edge = raster::line([[0.0, 0.0], [16.0, 16.0]], 1.0).expect("a line of positive length and width");
    assert!(edge.iter().all(|t| raster::count(SURFACE, t) == 256), "{edge:?}");
    let expanded: Vec<Triangle> = std::iter::repeat_n(edge, 1000).flatten().collect();
    let u = one_pass(&expanded);
    assert_eq!(u, 2000 * P);

    let stamp = stamp();
    let mut quarantine = Quarantine::new();
    assert_eq!(quarantine.frame(&stamp, &user_chain(), true, u, raster::allowance(SURFACE, 4)), STOPPED);
    // A short line of the same width counts too: a line never escapes the bound.
    let short = raster::line([[2.0, 2.0], [3.0, 2.0]], 1.0).expect("a line");
    assert_eq!(one_pass(&short), 16);
}

/// CT-RM-09, RM.40 row 8, the headless part: a stop after a completed publication, its frame report and the receipt.
/// A publication ends `completed(view_published)`; a later frame's raster bound is over the allowance. The view falls
/// back, the chain's digests quarantine, `read_frame_reports` gives `EV3:resource_limit.stage_work_exceeded` with form
/// `status` for that frame, and the receipt stays (RPI.72, RM.41). Not run: a host frame that draws a user stage
/// (F05 and F26 are unsupported, so the chain is stated beside the host's view) and the device's skipped work (C9).
#[test]
fn ct_rm_09_row_8_a_later_frame_stop_keeps_its_receipt() {
    let base = revision(5);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 3));
    let receipt = host.receipt(CommandId(1)).expect("a receipt");
    assert!(matches!(receipt.status, Status::Completed(_)), "{:?}", receipt.status);
    let generation = host.installed(ViewId(1)).expect("installed").generation;

    let chain = user_chain();
    let allowance = raster::allowance(SURFACE, 4);
    let frame = host.frame(ViewId(1)).expect("a frame");
    let mut quarantine = Quarantine::new();
    assert_eq!(quarantine.frame(&frame, &chain, true, one_pass(&overlap()), allowance), STOPPED);
    assert!(chain.digests().iter().all(|d| quarantine.is_quarantined(d)));

    let reports = quarantine.read_frame_reports();
    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!((&report.stamp, report.view, report.publication_generation), (&frame, ViewId(1), generation));
    assert_eq!((report.error.code, report.error.form), (codes::STAGE_WORK_EXCEEDED, Form::Status));
    assert_eq!(report.fallback, Fallback::BuiltInChain);
    // The report arrives after the publication is terminal and changes no receipt (RM.41, RM.22).
    assert_eq!(host.receipt(CommandId(1)), Some(receipt.clone()));
    assert!(quarantine.read_frame_reports().is_empty(), "each stopped frame is reported once");

    // A later frame of the chain runs no user-stage work and stays on the fallback, with no further report (draft).
    let later = host.frame(ViewId(1)).expect("a frame");
    assert_eq!(quarantine.frame(&later, &chain, true, one_pass(&spread()), allowance), STOPPED);
    assert!(quarantine.read_frame_reports().is_empty());
    assert_eq!(host.receipt(CommandId(1)), Some(receipt));
}

/// CT-RM-09, RM.40 row 10, the headless part: the quarantine of a faulted chain and the refusals after it. A chain
/// with vertex stage V and fragment stage F has a device fault: both digests quarantine, and later publications naming
/// V alone or F alone end with `EV3:input.stage_quarantined`, a refusal before acceptance and a stop after it; a
/// publication naming neither is not refused for quarantine (RPI.77, RM.20). Not run: injecting and detecting the
/// device fault (CT-RPI-34, C9).
#[test]
fn ct_rm_09_row_10_a_chains_stages_quarantine_together() {
    let p = program(vec![vertex(), fragment("f"), fragment("g")]);
    let (v, f, g) = (admitted(&p, "v"), admitted(&p, "f"), admitted(&p, "g"));
    let chain = Chain::User(vec![v.clone(), f.clone()]);
    let mut quarantine = Quarantine::new();
    assert_eq!(quarantine.device_fault(&chain, true), STOPPED);
    assert!(quarantine.is_quarantined(v.digest()) && quarantine.is_quarantined(f.digest()));
    assert!(!quarantine.is_quarantined(g.digest()));

    let base = revision(5);
    let naming = |stage: &CheckedStage| {
        let digest = stage.digest().to_owned();
        let row = Primitive::UserStage(UserStageInput { kind: stage.kind(), digest, compile_record: String::new() });
        publication(1, &base, vec![plan(1, &base, vec![row], Vec::new())])
    };
    for stage in [&v, &f] {
        for form in [Form::Refusal, Form::Stop] {
            let error = quarantine.admit(&naming(stage), form).expect_err("a quarantined digest");
            assert_eq!((error.code, error.form), (codes::STAGE_QUARANTINED, form));
            assert_eq!(error.context["digest"], stage.digest());
        }
    }
    assert_eq!(quarantine.admit(&naming(&g), Form::Refusal), Ok(()));
    assert_eq!(quarantine.admit(&sheet(1, &base, 3), Form::Refusal), Ok(()));
}

/// RPI.72 step 3's formula and `raster`'s drafts: a rectangle is rounded outward to whole pixels, clipped to the
/// surface and widened to whole 2 × 2 quads, and on a surface of odd size the widened quad may pass the last column;
/// a rectangle with no width or height, around a triangle of no area at an interior integer coordinate, counts zero;
/// a non-finite coordinate is an undefined vertex; a line with an undefined end is undefined, and a line of zero
/// length or width has no expansion here.
#[test]
fn raster_bound_rounds_outward_clips_and_widens_to_quads() {
    let count = |corners: [[f32; 2]; 3]| raster::count(SURFACE, &Triangle::Placed(corners));
    assert_eq!(count([[2.5, 2.5], [5.5, 2.5], [2.5, 5.5]]), 16, "2.5..5.5 rounds to 2..6");
    assert_eq!(count([[3.0, 3.0], [5.0, 3.0], [3.0, 5.0]]), 16, "3..5 widens to 2..6");
    assert_eq!(count([[-10.0, -10.0], [3.0, -10.0], [-10.0, 3.0]]), 16, "-10..3 clips to 0..3 and widens to 0..4");
    assert_eq!(count([[20.0, 20.0], [30.0, 20.0], [20.0, 30.0]]), 0, "off the surface");
    assert_eq!(count([[8.0, 2.0], [8.0, 6.0], [8.0, 10.0]]), 0, "no area on x = 8: a rectangle with no width");
    assert_eq!(count([[2.0, 8.0], [6.0, 8.0], [10.0, 8.0]]), 0, "no area on y = 8: a rectangle with no height");
    assert_eq!(count([[f32::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]]), 0, "an undefined vertex");
    let odd = Surface { width: 15, height: 15 };
    assert_eq!(raster::count(odd, &Triangle::Placed([[14.2, 0.2], [14.8, 0.2], [14.2, 0.8]])), 4, "14..15 to 14..16");
    assert_eq!(raster::count(odd, &Triangle::CrossesNear), 225);

    assert_eq!(raster::line([[f32::INFINITY, 0.0], [1.0, 1.0]], 1.0), Some([Triangle::Undefined; 2]));
    assert_eq!(raster::line([[1.0, 1.0], [1.0, 1.0]], 1.0), None);
    assert_eq!(raster::line([[0.0, 0.0], [1.0, 1.0]], 0.0), None);
}
