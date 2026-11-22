//! CT-RPI-18 (synthetic, whole definition on the CPU reference evaluator): singular and domain-boundary fixtures
//! produce missing coverage, no non-finite draw or pick coordinates, and unchanged exact subjects (RPI.36, RPI.37).
//! The exclusion neighbourhood is a declared fixture value, since RPI.36 is open. The same rule on a device with a user
//! vertex stage is CT-RPI-33 (windows) and is not run here.

mod common;

use c600_display::chart::{CameraPose, Pose, Stereographic};
use c600_display::exact::{self, Q5, V4};
use c600_display::plan::*;
use c600_display::reference::{self, Evaluation, Fixture, Missing, Point, PrecisionRule};
use common::*;

const RULE: PrecisionRule = PrecisionRule { absolute: 1e-4, exclusion: 0.05 };

/// The 120 vertices and the origin, a synthetic point on the domain's boundary.
fn vertices() -> Vec<V4> {
    let mut vertices = exact::cell600();
    vertices.push([Q5::ZERO; 4]);
    vertices
}

fn index_of(vertices: &[V4], v: &V4) -> u32 {
    vertices.iter().position(|u| u == v).expect("a fixture vertex") as u32
}

/// A one-plan publication drawn through the built-in stereographic chain.
fn charted(base: &RevisionRef) -> Publication {
    let mut publication = sheet(1, base, 2);
    publication.plans[0].charts.push(built_in(1));
    publication.plans[0].primitives.push(Primitive::ChartStage(StageInput { chart: ChartId(1) }));
    publication
}

/// Undefined coordinates enter neither the draw nor hit testing, and nothing finite stands in for them.
fn no_undefined_coordinates(evaluation: &Evaluation) {
    let missing: Vec<u32> = evaluation
        .points
        .iter()
        .enumerate()
        .filter(|(_, p)| matches!(p, Point::Missing(_)))
        .map(|(i, _)| i as u32)
        .collect();
    let (draw, pick) = (evaluation.draw(), evaluation.pick());
    assert_eq!(draw, pick, "the draw and hit testing read the same points");
    assert_eq!(draw.len() + missing.len(), evaluation.points.len());
    for (i, y) in draw.iter().chain(&pick) {
        assert!(y.iter().all(|c| c.is_finite()), "vertex {i}: {y:?}");
        assert!(!missing.contains(i), "a missing vertex {i} was drawn");
    }
}

fn missing_reasons(evaluation: &Evaluation) -> Vec<(u32, Missing)> {
    evaluation
        .points
        .iter()
        .enumerate()
        .filter_map(|(i, p)| match p {
            Point::Missing(m) => Some((i as u32, *m)),
            Point::Drawn(_) => None,
        })
        .collect()
}

#[test]
fn ct_rpi_18_singular_and_boundary_vertices_have_missing_coverage() {
    let base = revision(4);
    let publication = charted(&base);
    let vertices = vertices();
    let declaration = Stereographic { pole: W };
    let chart = &publication.plans[0].charts[0];
    let fixture =
        Fixture { plan: &publication.plans[0], chart, declaration: &declaration, vertices: &vertices, rule: RULE };
    let (pole, origin) = (index_of(&vertices, &W), 120);

    let evaluation = reference::evaluate(&fixture, &CameraPose::Exact(Pose::identity()));
    let expected = vec![(pole, Missing::Singular), (origin, Missing::Origin)];
    let mut sorted = expected.clone();
    sorted.sort_by_key(|(i, _)| *i);
    assert_eq!(missing_reasons(&evaluation), sorted);
    assert_eq!(evaluation.exact.exact_missing, sorted, "both decided exactly at an exact pose");
    assert_eq!(
        evaluation.coverage(),
        Coverage::Partial { missing: sorted.iter().map(|(i, m)| format!("vertex {i}: {}", m.reason())).collect() }
    );
    no_undefined_coordinates(&evaluation);

    // At the exact pose x ↦ k x the vertex (1, 0, 0, 0) moves onto the pole instead.
    let turned = Pose::rotation(exact::left(&W).expect("small values")).expect("k is a unit icosian");
    let evaluation = reference::evaluate(&fixture, &CameraPose::Exact(turned));
    let mut expected = vec![(index_of(&vertices, &E1), Missing::Singular), (origin, Missing::Origin)];
    expected.sort_by_key(|(i, _)| *i);
    assert_eq!(missing_reasons(&evaluation), expected);
    assert_eq!(evaluation.exact.exact_missing, expected);
    no_undefined_coordinates(&evaluation);
}

#[test]
fn ct_rpi_18_near_the_pole_a_float_pose_gives_missing_coverage_not_a_point() {
    let base = revision(4);
    let publication = charted(&base);
    let vertices = vertices();
    let declaration = Stereographic { pole: W };
    let chart = &publication.plans[0].charts[0];
    let (pole, origin) = (index_of(&vertices, &W), 120);
    // A turn by 1e-4 in the x2–x3 plane: the pole vertex is within binary32 rounding of the pole.
    let (s, c) = (1e-4f64.sin(), 1e-4f64.cos());
    let matrix = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, c, -s], [0.0, 0.0, s, c]];
    let pose = CameraPose::Approximate { label: "turn 1e-4 in x2-x3".to_owned(), matrix };

    // With the declared exclusion the vertex is excluded, a display decision; only the origin is decided exactly.
    let fixture =
        Fixture { plan: &publication.plans[0], chart, declaration: &declaration, vertices: &vertices, rule: RULE };
    let evaluation = reference::evaluate(&fixture, &pose);
    assert_eq!(missing_reasons(&evaluation), vec![(pole, Missing::Excluded), (origin, Missing::Origin)]);
    assert_eq!(evaluation.exact.exact_missing, vec![(origin, Missing::Origin)]);
    no_undefined_coordinates(&evaluation);

    // With no exclusion at all, binary32 division by zero still yields missing coverage, never an infinite point.
    let unguarded = PrecisionRule { absolute: 1e-4, exclusion: -1.0 };
    let fixture =
        Fixture { plan: &publication.plans[0], chart, declaration: &declaration, vertices: &vertices, rule: unguarded };
    let evaluation = reference::evaluate(&fixture, &pose);
    assert_eq!(missing_reasons(&evaluation), vec![(pole, Missing::NonFinite), (origin, Missing::Origin)]);
    no_undefined_coordinates(&evaluation);
}

#[test]
fn ct_rpi_18_missing_coverage_leaves_exact_subjects_unchanged() {
    let base = revision(4);
    let publication = charted(&base);
    let mut host = host(&base);
    install(&mut host, 1, publication.clone());
    let installed = host.installed(ViewId(1)).expect("installed").validated.clone();
    let adopted = host.adopted().clone();
    let receipt = host.receipt(CommandId(1)).expect("a receipt");
    let plan = publication.plans[0].clone();

    let vertices = vertices();
    let declaration = Stereographic { pole: W };
    let fixture =
        Fixture { plan: &plan, chart: &plan.charts[0], declaration: &declaration, vertices: &vertices, rule: RULE };
    for pose in [
        CameraPose::Exact(Pose::identity()),
        CameraPose::Exact(Pose::rotation(exact::left(&W).expect("small values")).expect("an exact pose")),
    ] {
        let evaluation = reference::evaluate(&fixture, &pose);
        assert!(matches!(evaluation.coverage(), Coverage::Partial { .. }));
        // The evaluation's exact fields name the plan's own subject and revision.
        assert_eq!(evaluation.exact.plan, plan.plan);
        assert_eq!(evaluation.exact.subject, plan.subject.digest());
        assert_eq!(evaluation.exact.base, plan.base.state);
    }
    // The plan, the installed publication, the engine's revision and the receipt are as they were.
    assert_eq!(plan, publication.plans[0]);
    assert_eq!(host.installed(ViewId(1)).expect("installed").validated, installed);
    assert_eq!(installed.subjects.len(), 1);
    assert_eq!(installed.subjects[0].digest, plan.subject.digest());
    assert_eq!(host.adopted(), &adopted);
    assert_eq!(host.receipt(CommandId(1)).expect("a receipt"), receipt);
}
