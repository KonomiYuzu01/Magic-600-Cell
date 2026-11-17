//! CT-RPI-14 (synthetic), the whole definition over this crate's drafted repertoire: at exact camera poses an exact
//! region test or a cited reduction decides declared chart properties, and a vertex test only refutes them;
//! intervening float poses cannot create exact verdicts (RPI.29, RPI.30, RPI.34; LB4.2). The repertoire stays RPI.34
//! [O]: here it is the stereographic chart over convex hulls of exact 600-cell vertices in Q(√5).

mod common;

use std::collections::HashSet;

use c600_display::chart::{
    self, Admission, CameraPose, ChartCheck, Method, Pose, Property, PropertyStatus, Region, Scope, Stereographic,
    Verdict, Witness,
};
use c600_display::exact::{self, Q5, V4};
use c600_display::reference::{self, Fixture, PrecisionRule};
use c600_display::{codes, Form};
use common::*;

fn region(name: &str, vertices: Vec<V4>) -> Region {
    Region::hull(name, vertices)
}

fn diagonal(values: [i128; 4]) -> [[Q5; 4]; 4] {
    let mut m = [[Q5::ZERO; 4]; 4];
    for (i, value) in values.into_iter().enumerate() {
        m[i][i] = Q5::int(value);
    }
    m
}

/// x ↦ −x, an element of K⁺.
fn antipodal() -> Pose {
    Pose::rotation(diagonal([-1, -1, -1, -1])).expect("−1 is an element of K⁺")
}

/// A binary64 turn by `t` in the x0–x3 plane.
fn float_turn(t: f64) -> [[f64; 4]; 4] {
    let mut m = [[0.0; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m[0][0] = t.cos();
    m[0][3] = -t.sin();
    m[3][0] = t.sin();
    m[3][3] = t.cos();
    m
}

fn properties(pole: &V4) -> [Property; 3] {
    [Property::Domain, Property::Injective, Property::Section { normal: *pole, low: Q5::ONE, high: Q5::int(4) }]
}

#[test]
fn ct_rpi_14_fixture_600_cell_and_exact_poses() {
    let all = exact::cell600();
    assert_eq!(all.len(), 120);
    assert_eq!(all.iter().copied().collect::<HashSet<V4>>().len(), 120);
    assert!(all.iter().all(|v| exact::dot(v, v) == Some(Q5::ONE)), "unit vertices");
    // Left multiplication by every unit icosian is an element of K⁺, so each is an exact pose.
    for q in &all {
        let m = exact::left(q).expect("small values");
        assert_eq!(exact::in_k_plus(&m), Some(true), "{q:?}");
        Pose::rotation(m).expect("an exact pose");
    }
    // A reflection and a quarter turn in one coordinate plane are exact matrices outside K⁺: neither is an exact pose.
    let mut quarter = diagonal([0, 0, 1, 1]);
    quarter[0][1] = Q5::int(-1);
    quarter[1][0] = Q5::ONE;
    for matrix in [diagonal([1, 1, 1, -1]), quarter] {
        let error = Pose::rotation(matrix).expect_err("not an element of K⁺");
        assert_eq!(error.code, codes::APPROXIMATE_PROPERTY_INPUT);
    }
    let cell = cell();
    let edge = Q5::new(1, 1, 4).expect("φ/2");
    for (i, u) in cell.iter().enumerate() {
        for v in &cell[i + 1..] {
            assert_eq!(exact::dot(u, v), Some(edge), "a cell's vertices are pairwise adjacent");
        }
    }
}

#[test]
fn ct_rpi_14_region_test_refutes_where_the_vertex_test_cannot() {
    let cell = cell();
    let pole = vertex_sum(&cell);
    let chart = Stereographic { pole };
    let region = region("cell", cell.clone());
    let at = CameraPose::Exact(Pose::identity());

    // LB4.2's shape: every vertex stays well off the pole's axis (binary64 display arithmetic, which decides nothing)
    // while the centroid lies on it.
    let axis = pole.map(|q| q.approx());
    let norm = axis.iter().map(|x| x * x).sum::<f64>().sqrt();
    for v in &cell {
        let v = v.map(|q| q.approx());
        let along = v.iter().zip(&axis).map(|(a, b)| a * b).sum::<f64>() / norm;
        let off = (v.iter().map(|x| x * x).sum::<f64>() - along * along).sqrt();
        assert!(off > 0.2296, "a vertex {off} from the axis");
    }

    // The vertex test finds no vertex on the singular ray, and it cannot establish the property from that.
    let vertices = chart::vertex_test(&chart, &Property::Domain, &region, &at).expect("an exact pose");
    assert!(matches!(vertices, Verdict::NotEstablished { .. }), "{vertices:?}");

    // The exact region test refutes it with an exact point of the region on the ray: the cell's centroid.
    let verdict = chart::check(&chart, &Property::Domain, &region, &at).expect("an exact pose");
    let Verdict::Refuted { method: Method::RegionTest, witness: Witness::PointOnSingularRay(point), scope } = verdict
    else {
        panic!("{verdict:?}")
    };
    assert_eq!(Some(point), exact::scale(Q5::new(1, 0, 4).expect("1/4"), &pole));
    assert_eq!(exact::same_ray(&point, &pole), Some(true));
    let expected = Scope {
        property: "domain".to_owned(),
        regions: vec!["cell".to_owned()],
        vertices: 4,
        poses: vec!["identity".to_owned()],
    };
    assert_eq!(scope, expected);
}

#[test]
fn ct_rpi_14_every_exact_pose_of_k_plus_is_decided() {
    let cell = cell();
    let chart = Stereographic { pole: vertex_sum(&cell) };
    let region = region("cell", cell);
    let mut refuted = Vec::new();
    for q in exact::cell600() {
        let pose = Pose::rotation(exact::left(&q).expect("small values")).expect("an exact pose");
        let id = pose.id();
        let verdict = chart::check(&chart, &Property::Domain, &region, &CameraPose::Exact(pose)).expect("exact");
        assert_eq!(verdict.scope().poses, vec![id], "a verdict covers its own pose only");
        match verdict {
            Verdict::Established { method: Method::RegionTest, .. } => {}
            Verdict::Refuted { method: Method::RegionTest, .. } => refuted.push(q),
            other => panic!("an exact pose is decided: {other:?}"),
        }
    }
    // Left multiplication by q ≠ 1 moves every point of the sphere, so only q = 1 keeps the cell on the pole's axis.
    assert_eq!(refuted, vec![E1]);
    // Between those poses nothing is claimed: the same region at the antipodal pose is decided on its own.
    let away = chart::check(&chart, &Property::Domain, &region, &CameraPose::Exact(antipodal())).expect("exact");
    assert!(matches!(away, Verdict::Established { method: Method::RegionTest, .. }), "{away:?}");
}

#[test]
fn ct_rpi_14_injectivity_by_reduction_and_section_on_posed_vertices() {
    let cell = cell();
    let pole = vertex_sum(&cell);
    let chart = Stereographic { pole };
    let at = CameraPose::Exact(Pose::identity());
    let flat = region("cell", cell.clone());

    let verdict = chart::check(&chart, &Property::Injective, &flat, &at).expect("exact");
    assert!(
        matches!(verdict, Verdict::Established { method: Method::Reduction(chart::CENTRAL_PROJECTION), .. }),
        "{verdict:?}"
    );
    let vertices = chart::vertex_test(&chart, &Property::Injective, &flat, &at).expect("exact");
    assert!(matches!(vertices, Verdict::NotEstablished { .. }), "{vertices:?}");

    // A region through the origin has no positive functional, so the reduction does not apply and no two vertices
    // share a ray: unknown, neither false nor a pass.
    let mut solid = cell.clone();
    solid.push(exact::scale(Q5::int(-1), &E1).expect("small values"));
    let verdict = chart::check(&chart, &Property::Injective, &region("solid", solid), &at).expect("exact");
    assert!(matches!(verdict, Verdict::NotEstablished { .. }), "{verdict:?}");

    // Two vertices on one ray: the exact check refutes with two points of the region on one ray, and the vertex test
    // finds the two vertices.
    let mut shared = cell.clone();
    shared.push(exact::scale(Q5::new(1, 0, 2).expect("1/2"), &E1).expect("small values"));
    let shared = region("shared", shared);
    let verdict = chart::check(&chart, &Property::Injective, &shared, &at).expect("exact");
    let Verdict::Refuted { method: Method::Reduction(_), witness: Witness::PointsOnOneRay(x, y), .. } = verdict else {
        panic!("{verdict:?}")
    };
    assert!(x != y && exact::same_ray(&x, &y) == Some(true), "{x:?} {y:?}");
    let vertices = chart::vertex_test(&chart, &Property::Injective, &shared, &at).expect("exact");
    assert!(matches!(vertices, Verdict::Refuted { witness: Witness::SharedRay(0, 4), .. }), "{vertices:?}");
    // Two vertices on the singular ray are outside the domain: they witness nothing, so nothing is refuted.
    let on_axis = region("on the singular ray", vec![pole, exact::scale(Q5::int(2), &pole).expect("small values")]);
    for verdict in [
        chart::check(&chart, &Property::Injective, &on_axis, &at),
        chart::vertex_test(&chart, &Property::Injective, &on_axis, &at),
    ] {
        let verdict = verdict.expect("exact");
        assert!(matches!(verdict, Verdict::NotEstablished { .. }), "{verdict:?}");
    }
    // A zero pole declares no chart, so no property of it is decided.
    let undefined = Stereographic { pole: [Q5::ZERO; 4] };
    for property in properties(&pole) {
        for verdict in
            [chart::check(&undefined, &property, &flat, &at), chart::vertex_test(&undefined, &property, &flat, &at)]
        {
            let verdict = verdict.expect("exact");
            assert!(matches!(verdict, Verdict::NotEstablished { .. }), "{verdict:?}");
        }
    }

    // Section membership is decided on the posed vertices; the vertex test alone still only refutes.
    let inside = Property::Section { normal: pole, low: Q5::ONE, high: Q5::int(4) };
    let narrow = Property::Section { normal: pole, low: Q5::ONE, high: Q5::int(3) };
    let decided = chart::check(&chart, &inside, &flat, &at).expect("exact");
    assert!(matches!(decided, Verdict::Established { method: Method::PosedVertices, .. }), "{decided:?}");
    let vertices = chart::vertex_test(&chart, &inside, &flat, &at).expect("exact");
    assert!(matches!(vertices, Verdict::NotEstablished { .. }), "{vertices:?}");
    let decided = chart::check(&chart, &narrow, &flat, &at).expect("exact");
    assert!(
        matches!(decided, Verdict::Refuted { method: Method::PosedVertices, witness: Witness::VertexOutside(0), .. }),
        "{decided:?}"
    );
    let vertices = chart::vertex_test(&chart, &narrow, &flat, &at).expect("exact");
    assert!(matches!(vertices, Verdict::Refuted { method: Method::VertexTest, .. }), "{vertices:?}");
    let away = chart::check(&chart, &inside, &flat, &CameraPose::Exact(antipodal())).expect("exact");
    assert!(matches!(away, Verdict::Refuted { method: Method::PosedVertices, .. }), "{away:?}");
}

#[test]
fn ct_rpi_14_float_poses_between_exact_poses_create_no_verdict() {
    let cell = cell();
    let pole = vertex_sum(&cell);
    let chart = Stereographic { pole };
    let region = region("cell", cell.clone());
    let approximate = |label: &str, matrix| CameraPose::Approximate { label: label.to_owned(), matrix };
    let poses = [
        CameraPose::Exact(Pose::identity()),
        approximate("turn 0.3 in x0-x3", float_turn(0.3)),
        // Bitwise the identity matrix, and still approximate: never rounded into an exact pose.
        approximate("float identity", float_turn(0.0)),
        approximate("turn pi in x0-x3", float_turn(std::f64::consts::PI)),
        CameraPose::Exact(antipodal()),
    ];
    let built_in = built_in(1);
    let mut checks = Vec::new();
    for pose in &poses {
        for property in properties(&pole) {
            for result in
                [chart::check(&chart, &property, &region, pose), chart::vertex_test(&chart, &property, &region, pose)]
            {
                match pose {
                    CameraPose::Exact(_) => checks.push(ChartCheck {
                        chart: chart::identity(&built_in),
                        property: property.name().to_owned(),
                        verdict: result.expect("an exact pose has a verdict"),
                    }),
                    CameraPose::Approximate { .. } => {
                        let error = result.expect_err("a float pose has no verdict");
                        assert_eq!(error.code, codes::APPROXIMATE_PROPERTY_INPUT);
                        assert_eq!(error.form, Form::Refusal);
                    }
                }
            }
        }
    }
    // Only the exact poses gave verdicts, each covering exactly one of them.
    let exact_poses = [Pose::identity().id(), antipodal().id()];
    assert_eq!(checks.len(), 2 * 3 * 2);
    for check in &checks {
        let poses = &check.verdict.scope().poses;
        assert_eq!(poses.len(), 1);
        assert!(exact_poses.contains(&poses[0]), "{poses:?}");
    }

    // Between the exact poses, drawing is a display decision: binary32 points and coverage, nothing decided exactly.
    let base = revision(3);
    let publication = sheet(1, &base, 1);
    let rule = PrecisionRule { absolute: 1e-4, exclusion: 0.05 };
    let fixture = Fixture { plan: &publication.plans[0], chart: &built_in, declaration: &chart, vertices: &cell, rule };
    for pose in &poses[1..4] {
        let evaluation = reference::evaluate(&fixture, pose);
        assert!(evaluation.exact.exact_missing.is_empty(), "{:?}", evaluation.exact);
        assert_eq!(evaluation.points.len(), 4);
    }

    // Admission takes the established verdicts with their scopes: injectivity holds at the two exact poses checked,
    // and the status claims nothing about the poses in between.
    let mut claiming = built_in.clone();
    claiming.properties = vec!["injective".to_owned()];
    let (admission, statuses) = chart::admit_chart(&claiming, &checks);
    assert_eq!(admission, Admission::Admitted);
    let [(property, PropertyStatus::Established(scopes))] = &statuses[..] else { panic!("{statuses:?}") };
    assert_eq!(property, "injective");
    let covered: Vec<&String> = scopes.iter().flat_map(|s| &s.poses).collect();
    assert_eq!(covered, exact_poses.iter().collect::<Vec<_>>());
}
