//! CT-RPI-19 (source), the Windows leg: one reproducibility key gives equal exact plan fields and approximate values
//! within declared precision, including maths-library-sensitive cases (RPI.56, LX-3). The maths-library-sensitive
//! cases are camera turns built with the platform's `sin`, `cos` and `sqrt`, compared with exact images in Q(√5). The
//! headless Linux leg is these same tests on Linux against the same pinned key; it is not run here, so CT-RPI-19 stays
//! partly run. Bitwise equality of floats across platforms is not claimed (RPI.56).

mod common;

use std::f64::consts::PI;

use c600_display::chart::{CameraPose, Pose, Stereographic};
use c600_display::exact::{self, Q5, V4};
use c600_display::reference::{self, Evaluation, Fixture, Missing, Point, PrecisionRule};
use common::*;

/// The declared rule of these fixtures (RPI.36 is open): 1e-4 per coordinate, outside 1 − û·p̂ ≤ 0.05.
const RULE: PrecisionRule = PrecisionRule { absolute: 1e-4, exclusion: 0.05 };

/// The key of the evaluation at the exact pose of the π/5 turn, pinned so that every platform's leg compares against
/// the same value.
const KEY: &str = "9227ee6f8e5a3134a5ef8ee3e462a9e6d4948e26dff3d2e02aab3781cfb92736";

/// The unit icosians cos θ + sin θ û for θ = π/5 and θ = 2π/5; both are vertices of the fixture, so their left
/// multiplications are exact poses.
fn turns() -> [(f64, V4); 2] {
    let half = Q5::new(1, 0, 2).expect("1/2");
    let small = Q5::new(-1, 1, 4).expect("1/(2φ) = cos 2π/5");
    let large = Q5::new(1, 1, 4).expect("φ/2 = cos π/5");
    [(PI / 5.0, [large, small, half, Q5::ZERO]), (2.0 * PI / 5.0, [small, large, Q5::ZERO, half])]
}

/// The same turn as a binary64 matrix built with the platform's maths library: x ↦ q x with
/// q = (cos θ, sin θ û), û the normalised axis.
fn float_turn(theta: f64, q: &V4, separate: bool) -> [[f64; 4]; 4] {
    let axis = [q[1].approx(), q[2].approx(), q[3].approx()];
    let norm = axis.iter().map(|x| x * x).sum::<f64>().sqrt();
    let (s, c) = if separate { (theta.sin(), theta.cos()) } else { theta.sin_cos() };
    let (a, b, cc, d) = (c, s * axis[0] / norm, s * axis[1] / norm, s * axis[2] / norm);
    [[a, -b, -cc, -d], [b, a, -d, cc], [cc, d, a, -b], [d, -cc, b, a]]
}

fn exact_turn(q: &V4) -> Pose {
    Pose::rotation(exact::left(q).expect("small values")).expect("a unit icosian is an element of K⁺")
}

/// Evaluates the fixture from scratch: a fresh plan at revision 9, the built-in chain, the 120 vertices and pole +w.
fn evaluate(pose: &CameraPose, rule: PrecisionRule) -> Evaluation {
    let base = revision(9);
    let mut publication = sheet(1, &base, 1);
    publication.plans[0].charts.push(built_in(1));
    let plan = &publication.plans[0];
    let vertices = exact::cell600();
    let declaration = Stereographic { pole: W };
    let fixture = Fixture { plan, chart: &plan.charts[0], declaration: &declaration, vertices: &vertices, rule };
    reference::evaluate(&fixture, pose)
}

/// CT-DET-15 (whole: the display plan's exact fields and key equal one pinned digest on every platform that runs the
/// test, a float pose gives the same exact fields whatever maths-library path built its matrix, and the approximate
/// points stay within the declared precision of their exact images; the platforms are Windows on the headless shards
/// and headless Linux in the Linux Rust job, which both run this crate's tests; RT1's display fields are in
/// `c600-run/tests/conformance_det.rs` and `values.rs`).
#[test]
fn ct_rpi_19_windows_leg_one_key_gives_equal_exact_fields() {
    let (theta, q) = turns()[0];
    let at = CameraPose::Exact(exact_turn(&q));
    let (first, second) = (evaluate(&at, RULE), evaluate(&at, RULE));
    assert_eq!(first.exact, second.exact);
    assert_eq!(first.key(), second.key());
    assert_eq!(first.key(), KEY, "the pinned key of every platform's leg");

    // The key is the exact inputs': another pose or another declared rule is another key.
    let other = CameraPose::Exact(exact_turn(&turns()[1].1));
    assert_ne!(evaluate(&other, RULE).key(), first.key());
    assert_ne!(evaluate(&at, PrecisionRule { absolute: 1e-5, exclusion: 0.05 }).key(), first.key());

    // A float pose enters by its declared text, not by the bits its maths library produced: two matrices for the same
    // declared turn, built along different library paths, give the same key and equal exact fields.
    let label = "left turn pi/5 about the axis of the icosian (phi/2, 1/(2 phi), 1/2, 0)".to_owned();
    let paths = [
        CameraPose::Approximate { label: label.clone(), matrix: float_turn(theta, &q, false) },
        CameraPose::Approximate { label, matrix: float_turn(theta, &q, true) },
    ];
    let (one, two) = (evaluate(&paths[0], RULE), evaluate(&paths[1], RULE));
    assert_eq!(one.exact, two.exact);
    assert_eq!(one.key(), two.key());
    assert!(one.exact.exact_missing.is_empty(), "a float pose decides nothing exactly");
    // The plan fields are the same exact fields at the exact pose and at the float one.
    let (a, b) = (&first.exact, &one.exact);
    assert_eq!(
        (a.plan, &a.subject, a.base, &a.chart, &a.declaration, &a.vertices, &a.precision),
        (b.plan, &b.subject, b.base, &b.chart, &b.declaration, &b.vertices, &b.precision)
    );
}

#[test]
fn ct_rpi_19_windows_leg_approximate_values_within_declared_precision() {
    let mut worst = 0.0f64;
    let mut compared = 0;
    for (theta, q) in turns() {
        let matrix = exact::left(&q).expect("small values");
        let exact_pose = CameraPose::Exact(exact_turn(&q));
        let float_pose =
            CameraPose::Approximate { label: format!("turn {theta}"), matrix: float_turn(theta, &q, false) };
        let at_exact = evaluate(&exact_pose, RULE);
        let at_float = evaluate(&float_pose, RULE);
        let missing = |e: &Evaluation| -> Vec<usize> {
            e.points.iter().enumerate().filter(|(_, p)| matches!(p, Point::Missing(_))).map(|(i, _)| i).collect()
        };
        assert_eq!(missing(&at_exact), missing(&at_float), "no vertex lies near the declared exclusion");
        for (i, v) in exact::cell600().iter().enumerate() {
            let posed = exact::apply(&matrix, v).expect("small values");
            match reference::exact_image(&W, &posed) {
                None => {
                    // The vertex the turn carries onto the pole: exactly singular at the exact pose, and excluded, not
                    // drawn, at the float pose.
                    assert_eq!(posed, W);
                    assert_eq!(at_exact.points[i], Point::Missing(Missing::Singular));
                    assert_eq!(at_float.points[i], Point::Missing(Missing::Excluded));
                }
                Some(image) => {
                    let image = image.map(|c| c.approx());
                    for evaluation in [&at_exact, &at_float] {
                        let Point::Drawn(y) = evaluation.points[i] else {
                            panic!("vertex {i}: {:?}", evaluation.points[i])
                        };
                        for k in 0..4 {
                            let error = (y[k] as f64 - image[k]).abs();
                            worst = worst.max(error);
                            assert!(error <= RULE.absolute as f64, "vertex {i}, coordinate {k}: {error}");
                        }
                        compared += 1;
                    }
                }
            }
        }
    }
    assert_eq!(compared, 2 * 2 * 119);
    assert!(worst <= RULE.absolute as f64, "{worst}");
}
