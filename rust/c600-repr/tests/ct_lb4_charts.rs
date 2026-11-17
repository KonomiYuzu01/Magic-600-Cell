//! CT-LB4-01 (the CPU reference part), CT-LB4-02 (synthetic) and CT-LB4-04 (the admission and CPU half): map property
//! checks at exact camera poses, display decisions between them, and the admission of a user chart (LB4.2, LB4.3).
//! The exact checks are `c600-display`'s drafted repertoire (RPI.34 [O]): the stereographic chart over convex hulls of
//! exact 600-cell vertices in Q(√5). Every result is a source or synthetic fixture outcome; nothing here ran on a
//! device.

mod common;

use c600_digest::sha256_hex;
use c600_display::chart::{
    self, Admission, CameraPose, Method, Pose, Property, PropertyStatus, Region, Stereographic, Verdict, Witness,
};
use c600_display::codes;
use c600_display::exact::{self, Q5, V4};
use c600_display::plan::*;
use c600_display::reference::{self, Fixture, Point, PrecisionRule};
use c600_display::stage::{Output, StageDescription};
use c600_display::validate::validate;
use c600_repr::chart::{admit, admit_user_chart, reference_decision, survey, Subject, Survey, UserChartError};
use c600_repr::layout::Representation;

use common::*;

const RULE: PrecisionRule = PrecisionRule { absolute: 1e-4, exclusion: 0.05 };

fn region(name: &str, vertices: Vec<V4>) -> Region {
    Region::hull(name, vertices)
}

/// x ↦ −x, an element of K⁺.
fn antipodal() -> Pose {
    let mut minus = [[Q5::ZERO; 4]; 4];
    for (i, row) in minus.iter_mut().enumerate() {
        row[i] = Q5::int(-1);
    }
    Pose::rotation(minus).expect("−1 is an element of K⁺")
}

/// Left multiplication by a unit icosian, an element of K⁺.
fn left(q: &V4) -> Pose {
    Pose::rotation(exact::left(q).expect("small values")).expect("a unit icosian is an element of K⁺")
}

/// A binary64 turn by `t` in the x0–x3 plane, with its declared text.
fn float_turn(t: f64) -> CameraPose {
    let mut matrix = [[0.0; 4]; 4];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    matrix[0][0] = t.cos();
    matrix[0][3] = -t.sin();
    matrix[3][0] = t.sin();
    matrix[3][3] = t.cos();
    CameraPose::Approximate { label: format!("turn {t} in x0-x3"), matrix }
}

/// The built-in chain as chart `id`, claiming `properties`.
fn claiming(id: u64, properties: &[&str]) -> Chart {
    let mut claimed = built_in(id);
    claimed.properties = properties.iter().map(|p| (*p).to_owned()).collect();
    claimed
}

/// A representation that shows one cell through `chart`.
fn shown(base: &RevisionRef, chart: Chart) -> Representation {
    let id = chart.chart.0;
    let occurrences = [occurrence(1, ObjectId::Cell(0), Some(id))];
    Representation::layout(&header(1, base), &[ObjectId::Cell(0)], vec![chart], &occurrences).expect("an atlas")
}

fn status<'a>(statuses: &'a [(String, PropertyStatus)], property: &str) -> &'a PropertyStatus {
    &statuses.iter().find(|(name, _)| name == property).expect("a claimed property").1
}

/// What the kernel answers, computed from the model, a word and a protected set: a state digest and whether the
/// protected derivation is refused. Nothing a display decision produces is among its inputs.
fn mechanical() -> (String, bool) {
    let recipe = kernel().word(&[1]).expect("a legal letter");
    let protected = kernel().protected_set(&[0]).expect("an orbit of the model");
    let refused = matches!(
        kernel().derive(&kernel().solved(), &recipe, &protected).expect("derives"),
        c600_kernel::Derivation::Refused(_)
    );
    (derive(&[1]).state.digest().to_owned(), refused)
}

/// CT-LB4-01 (part, the CPU reference: the exact region test of the domain, boundary cases included, and the refusal).
/// A cell whose four vertices all pass the per-vertex test and whose centroid, an interior point, lies on the chart's
/// singular ray: the vertex test establishes nothing, the exact region test refutes the domain with the exact point,
/// and a chart claiming the domain is refused. The boundary cases are decided as closed: a singular ray through an
/// edge's midpoint or through a vertex refutes the domain, and a ray that misses the region establishes it. The region
/// test is `c600-display`'s: KN3.3's `ray_meet` from the origin along the pole over the hull of the posed vertices.
#[test]
fn ct_lb4_01_cpu_reference_domain_by_exact_region_test_and_refusal_at_an_interior_point() {
    let cell = cell();
    let pole = vertex_sum(&cell);
    let declaration = Stereographic { pole };
    let flat = region("cell", cell.clone());
    let at = CameraPose::Exact(Pose::identity());

    // Every vertex passes: the vertex test finds no witness and cannot establish the property from that.
    let vertices = chart::vertex_test(&declaration, &Property::Domain, &flat, &at).expect("an exact pose");
    assert!(matches!(vertices, Verdict::NotEstablished { .. }), "{vertices:?}");
    // The region test finds the interior point: the centroid, with all four barycentric coefficients 1/4.
    let verdict = chart::check(&declaration, &Property::Domain, &flat, &at).expect("an exact pose");
    let Verdict::Refuted { method: Method::RegionTest, witness: Witness::PointOnSingularRay(point), .. } = &verdict
    else {
        panic!("{verdict:?}")
    };
    assert_eq!(Some(*point), exact::scale(Q5::new(1, 0, 4).expect("1/4"), &pole));
    assert!(cell.iter().all(|v| v != point), "the witness is no vertex");

    // The chart is refused, through a survey of the exact pose and `admit_chart`.
    let base = revision(12);
    let claimed = claiming(1, &["domain"]);
    let rep = shown(&base, claimed.clone());
    let properties = [Property::Domain];
    let subject = Subject {
        plan: &rep.plan,
        chart: &claimed,
        declaration: &declaration,
        region: &flat,
        properties: &properties,
        rule: RULE,
    };
    let surveyed = survey(&subject, std::slice::from_ref(&at), &reference_decision).expect("surveyed");
    assert_eq!(surveyed.checks.len(), 1);
    assert_eq!(surveyed.checks[0].verdict, verdict);
    let (admission, statuses) = admit(&claimed, &surveyed);
    let Admission::Refused(error) = admission else { panic!("{admission:?}") };
    assert_eq!(error.code, codes::CHART_CHECK_FAILED);
    assert_eq!(status(&statuses, "domain"), &PropertyStatus::NotEstablished, "withheld, and no negative certified");
    // The reference draws the four vertices all the same: drawing them decided nothing.
    assert_eq!(surveyed.frames[0].drawn.len(), 4);
    assert_eq!(surveyed.frames[0].coverage, Coverage::Complete);

    // Boundary cases of the region test. The ray through an edge's midpoint meets the region on its boundary.
    let through_edge = Stereographic { pole: exact::add(&cell[0], &cell[1]).expect("small values") };
    let verdict = chart::check(&through_edge, &Property::Domain, &flat, &at).expect("an exact pose");
    let Verdict::Refuted { method: Method::RegionTest, witness: Witness::PointOnSingularRay(point), .. } = verdict
    else {
        panic!("{verdict:?}")
    };
    assert_eq!(Some(point), exact::scale(Q5::new(1, 0, 2).expect("1/2"), &through_edge.pole));
    let vertices = chart::vertex_test(&through_edge, &Property::Domain, &flat, &at).expect("an exact pose");
    assert!(matches!(vertices, Verdict::NotEstablished { .. }), "{vertices:?}");
    // The ray through a vertex: both tests refute, each with its own witness.
    let through_vertex = Stereographic { pole: exact::scale(Q5::int(3), &cell[2]).expect("small values") };
    let verdict = chart::check(&through_vertex, &Property::Domain, &flat, &at).expect("an exact pose");
    assert!(
        matches!(&verdict, Verdict::Refuted { witness: Witness::PointOnSingularRay(p), .. } if *p == cell[2]),
        "{verdict:?}"
    );
    let vertices = chart::vertex_test(&through_vertex, &Property::Domain, &flat, &at).expect("an exact pose");
    assert!(matches!(vertices, Verdict::Refuted { witness: Witness::VertexOnSingularRay(2), .. }), "{vertices:?}");
    // A ray that misses the region: established, by the region test. The same cell at the antipodal pose is such a case.
    let away = Stereographic { pole: exact::scale(Q5::int(-1), &pole).expect("small values") };
    for (declaration, pose) in [(&away, at.clone()), (&declaration, CameraPose::Exact(antipodal()))] {
        let verdict = chart::check(declaration, &Property::Domain, &flat, &pose).expect("an exact pose");
        assert!(matches!(verdict, Verdict::Established { method: Method::RegionTest, .. }), "{verdict:?}");
    }
}

/// CT-LB4-01 (part, the CPU reference: section membership on the posed vertices, boundary cases included). The slab is
/// closed: vertices exactly on its lower and upper bounds are inside, decided in exact arithmetic over Q(√5) where one
/// bound is irrational; a bound moved inward by 1/1000 refutes the section with the vertex it leaves outside. The test
/// reads the posed vertices: at the antipodal pose the same slab is refuted and its mirror image holds.
#[test]
fn ct_lb4_01_cpu_reference_section_membership_on_the_posed_vertices_with_boundary_cases() {
    let cell = cell();
    let declaration = Stereographic { pole: vertex_sum(&cell) };
    let flat = region("cell", cell.clone());
    let at = CameraPose::Exact(Pose::identity());
    // E1 · E1 = 1, and E1's product with each of its three neighbours is φ/2.
    let half_phi = Q5::new(1, 1, 4).expect("φ/2");
    let step = Q5::new(1, 0, 1000).expect("1/1000");
    let section = |low: Q5, high: Q5| Property::Section { normal: E1, low, high };

    let on_both_bounds = section(half_phi, Q5::ONE);
    let verdict = chart::check(&declaration, &on_both_bounds, &flat, &at).expect("an exact pose");
    assert!(matches!(verdict, Verdict::Established { method: Method::PosedVertices, .. }), "{verdict:?}");
    // The vertex test alone still establishes nothing.
    let vertices = chart::vertex_test(&declaration, &on_both_bounds, &flat, &at).expect("an exact pose");
    assert!(matches!(vertices, Verdict::NotEstablished { .. }), "{vertices:?}");

    let high_inward = section(half_phi, Q5::ONE.sub(step).expect("small values"));
    let verdict = chart::check(&declaration, &high_inward, &flat, &at).expect("an exact pose");
    assert!(
        matches!(verdict, Verdict::Refuted { method: Method::PosedVertices, witness: Witness::VertexOutside(0), .. }),
        "{verdict:?}"
    );
    let low_inward = section(half_phi.add(step).expect("small values"), Q5::ONE);
    let verdict = chart::check(&declaration, &low_inward, &flat, &at).expect("an exact pose");
    assert!(
        matches!(verdict, Verdict::Refuted { method: Method::PosedVertices, witness: Witness::VertexOutside(1), .. }),
        "{verdict:?}"
    );

    // Posed vertices, not the stored ones: at x ↦ −x the slab holds −1 ≤ E1 · x ≤ −φ/2 and no longer the first one.
    let turned = CameraPose::Exact(antipodal());
    let verdict = chart::check(&declaration, &on_both_bounds, &flat, &turned).expect("an exact pose");
    assert!(matches!(verdict, Verdict::Refuted { method: Method::PosedVertices, .. }), "{verdict:?}");
    let minus = |q: Q5| Q5::ZERO.sub(q).expect("small values");
    let mirrored = section(minus(Q5::ONE), minus(half_phi));
    let verdict = chart::check(&declaration, &mirrored, &flat, &turned).expect("an exact pose");
    assert!(matches!(verdict, Verdict::Established { method: Method::PosedVertices, .. }), "{verdict:?}");
    assert_eq!(verdict.scope().poses, vec![antipodal().id()], "a verdict covers its own pose");
}

/// CT-LB4-01 (part, the CPU reference: each property requires its own check). A chart that claims the domain,
/// injectivity, embedding, isometry and homomorphism is surveyed at an exact pose where its domain holds. An
/// established domain establishes nothing else; injectivity and the embedding are each established only by their own
/// check, through `c600-display`'s reduction to a central projection; isometry and homomorphism have no exact checker
/// for a stereographic chart and stay `not_established`, neither refuted nor passed, so the chart is not admitted as
/// claimed. A full-dimensional region whose five vertices all pass the per-vertex test fails injectivity inside: its
/// own check refutes it with two points on the ray through the region's centroid, and the chart is refused.
#[test]
fn ct_lb4_01_cpu_reference_each_property_requires_its_own_check() {
    let cell = cell();
    let declaration = Stereographic { pole: vertex_sum(&cell) };
    let flat = region("cell", cell.clone());
    let base = revision(12);
    let claimed = claiming(1, &["domain", "injective", "embedding", "isometry", "homomorphism"]);
    let rep = shown(&base, claimed.clone());
    let turned = CameraPose::Exact(antipodal());
    let pose = [turned.clone()];
    let run = |properties: &[Property], region: &Region| {
        let subject =
            Subject { plan: &rep.plan, chart: &claimed, declaration: &declaration, region, properties, rule: RULE };
        admit(&claimed, &survey(&subject, &pose, &reference_decision).expect("surveyed"))
    };
    let established = |s: &PropertyStatus| matches!(s, PropertyStatus::Established(_));

    // The domain check alone: only the domain is established.
    let (admission, statuses) = run(&[Property::Domain], &flat);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
    assert!(established(status(&statuses, "domain")));
    for other in ["injective", "embedding", "isometry", "homomorphism"] {
        assert_eq!(status(&statuses, other), &PropertyStatus::NotEstablished, "{other}");
    }

    // With its own check, injectivity is established, and the other three are not.
    let (admission, statuses) = run(&[Property::Domain, Property::Injective], &flat);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
    assert!(established(status(&statuses, "domain")) && established(status(&statuses, "injective")));
    for other in ["embedding", "isometry", "homomorphism"] {
        assert_eq!(status(&statuses, other), &PropertyStatus::NotEstablished, "{other}");
    }

    // Every property with its own check: the embedding is established too; isometry and homomorphism have no checker
    // for a stereographic chart and are not.
    let unit: Vec<Vec<Q5>> =
        (0..4).map(|i| (0..4).map(|j| if i == j { Q5::ONE } else { Q5::ZERO }).collect()).collect();
    let isometry = Property::Isometry { source: unit.clone(), target: unit };
    let homomorphism = Property::Homomorphism { codomain: Vec::new() };
    let all = [Property::Domain, Property::Injective, Property::Embedding, isometry.clone(), homomorphism.clone()];
    let (admission, statuses) = run(&all, &flat);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
    for property in ["domain", "injective", "embedding"] {
        assert!(established(status(&statuses, property)), "{property}");
    }
    for (name, property) in [("isometry", &isometry), ("homomorphism", &homomorphism)] {
        assert_eq!(status(&statuses, name), &PropertyStatus::NotEstablished, "{name}");
        let verdict = chart::check(&declaration, property, &flat, &turned).expect("an exact pose");
        assert!(matches!(verdict, Verdict::NotEstablished { .. }), "{verdict:?}");
    }

    // A full-dimensional region whose five vertices all pass the per-vertex test, beside a domain that holds. Its own
    // check refutes injectivity with two distinct points on the ray through the centroid, an interior point, so the
    // chart is refused and every status is withheld.
    let mut solid = flat.clone();
    solid.name = "solid".to_owned();
    let off_the_flat =
        exact::add(&exact::scale(Q5::int(2), &E1).expect("small values"), &cell[1]).expect("small values");
    solid.vertices.push(off_the_flat);
    let vertices = chart::vertex_test(&declaration, &Property::Injective, &solid, &turned).expect("an exact pose");
    assert!(matches!(vertices, Verdict::NotEstablished { .. }), "{vertices:?}");
    let domain = chart::check(&declaration, &Property::Domain, &solid, &turned).expect("an exact pose");
    assert!(matches!(domain, Verdict::Established { .. }), "{domain:?}");
    let verdict = chart::check(&declaration, &Property::Injective, &solid, &turned).expect("an exact pose");
    let Verdict::Refuted { method: Method::Reduction(_), witness: Witness::PointsOnOneRay(x, y), .. } = verdict else {
        panic!("{verdict:?}")
    };
    let centroid = exact::scale(Q5::int(-1), &vertex_sum(&solid.vertices)).expect("small values");
    assert!(x != y && exact::same_ray(&x, &y) == Some(true), "{x:?} {y:?}");
    assert_eq!(exact::same_ray(&x, &centroid), Some(true), "the posed centroid's ray");
    let (admission, statuses) = run(&[Property::Domain, Property::Injective], &solid);
    let Admission::Refused(error) = admission else { panic!("{admission:?}") };
    assert_eq!(error.code, codes::CHART_CHECK_FAILED);
    assert_eq!(error.context["property"], "injective");
    assert!(statuses.iter().all(|(_, s)| s == &PropertyStatus::NotEstablished), "{statuses:?}");
}

/// CT-LB4-02 (synthetic, whole definition over this crate's drafted survey). A chart is surveyed over exact poses with
/// float poses between them, once with the CPU reference's display decision and once with each of four replacements.
/// The replaced decisions change the frames of the float poses and nothing else: the exact verdicts and their check
/// IDs, the result of `admit_chart` (an admitted run and a refused run), the validated publication's digest, chart
/// admissions and property statuses, the chart's content identity, and the kernel's state digest and protection
/// verdict are the same. Two of these hold by construction and the test states them rather than discovers them: the
/// kernel takes no display input, and `admit` has no parameter a frame could reach. A decision is never asked at an
/// exact pose, and a float pose has no exact verdict.
#[test]
fn ct_lb4_02_replaced_display_decisions_reach_only_display_data() {
    let cell = cell();
    let declaration = Stereographic { pole: vertex_sum(&cell) };
    let flat = region("cell", cell.clone());
    let base = revision(12);
    let claimed = claiming(1, &["domain", "injective"]);
    let rep = shown(&base, claimed.clone());
    let properties = [Property::Domain, Property::Injective];
    let subject = Subject {
        plan: &rep.plan,
        chart: &claimed,
        declaration: &declaration,
        region: &flat,
        properties: &properties,
        rule: RULE,
    };
    let exact_pose = CameraPose::Exact;
    let runs = [
        // The domain holds at both exact poses: admitted.
        (
            vec![
                exact_pose(antipodal()),
                float_turn(0.3),
                float_turn(0.0),
                float_turn(2.0),
                exact_pose(left(&cell[1])),
            ],
            true,
        ),
        // The domain fails at the identity: refused.
        (vec![exact_pose(Pose::identity()), float_turn(0.3), float_turn(1.0), exact_pose(antipodal())], false),
    ];
    type Decision = Box<dyn Fn(&CameraPose, u32, &Point) -> bool>;
    let replacements: Vec<(&str, Decision)> = vec![
        ("show everything", Box::new(|_, _, _| true)),
        ("show nothing", Box::new(|_, _, _| false)),
        ("every other vertex", Box::new(|_, i, _| i % 2 == 0)),
        ("inverted", Box::new(|pose, i, point| !reference_decision(pose, i, point))),
    ];
    let outcome = |surveyed: &Survey| {
        let mut ctx = context();
        ctx.checks = surveyed.checks.clone();
        let validated = validate(&publication(1, &base, vec![rep.clone()]), &ctx).expect("valid");
        (
            admit(&claimed, surveyed),
            surveyed.checks.iter().map(|c| c.id()).collect::<Vec<_>>(),
            validated.digest,
            validated.charts,
            validated.properties,
            chart::identity(&claimed),
        )
    };
    let before = mechanical();

    for (poses, admitted) in &runs {
        let exact_poses = poses.iter().filter(|p| matches!(p, CameraPose::Exact(_))).count();
        let reference = survey(&subject, poses, &reference_decision).expect("surveyed");
        assert_eq!(reference.checks.len(), exact_poses * properties.len(), "verdicts come from exact poses only");
        assert_eq!(reference.frames.len(), poses.len());
        let expected = outcome(&reference);
        match (&expected.0 .0, admitted) {
            (Admission::Admitted, true) => {}
            (Admission::Refused(error), false) => assert_eq!(error.code, codes::CHART_CHECK_FAILED),
            other => panic!("{other:?}"),
        }
        for pose in poses.iter().filter(|p| matches!(p, CameraPose::Approximate { .. })) {
            let error = chart::check(&declaration, &Property::Domain, &flat, pose).expect_err("no exact verdict");
            assert_eq!(error.code, codes::APPROXIMATE_PROPERTY_INPUT);
        }

        for (name, decide) in &replacements {
            let asked_only_between = |pose: &CameraPose, i: u32, point: &Point| {
                assert!(matches!(pose, CameraPose::Approximate { .. }), "{name}: asked at an exact pose");
                decide(pose, i, point)
            };
            let replaced = survey(&subject, poses, &asked_only_between).expect("surveyed");
            // Nothing exact moved.
            assert_eq!(replaced.checks, reference.checks, "{name}");
            assert_eq!(outcome(&replaced), expected, "{name}");
            // The frames of the exact poses are the reference's; only the float poses' frames may differ.
            for (a, b) in replaced.frames.iter().zip(&reference.frames) {
                assert_eq!(a.pose, b.pose);
                if a.exact_pose {
                    assert_eq!(a, b, "{name}");
                }
            }
            if *name == "show nothing" {
                for frame in replaced.frames.iter().filter(|f| !f.exact_pose) {
                    assert!(frame.drawn.is_empty());
                    assert!(matches!(&frame.coverage, Coverage::Partial { missing } if missing.len() == 4));
                }
                assert_ne!(replaced.frames, reference.frames, "the replaced decision did reach the display data");
            }
        }
    }
    assert_eq!(mechanical(), before, "the kernel's digest and protection verdict are as they were");
}

fn describe(name: &str, kind: StageKind, outputs: &[Output]) -> StageDescription {
    StageDescription { kind, digest: sha256_hex(name.as_bytes()), outputs: outputs.to_vec() }
}

/// A user chart: the built-in rotation followed by a user stage, named by the stage description's digest.
fn user_chart(id: u64, description: &StageDescription, properties: &[&str]) -> Chart {
    let mut user = chart(id, &[("rotate", 6)]);
    user.stages.push(Stage {
        name: "user_projection".to_owned(),
        version: 1,
        definition: description.digest.clone(),
        params: Vec::new(),
    });
    user.properties = properties.iter().map(|p| (*p).to_owned()).collect();
    user
}

/// CT-LB4-04 (part, the admission half as far as `c600-display` reaches it). A user chart whose vertex stage passes the
/// stage rules is bound to that stage by its digest, and that admission alone leaves `admit_chart` at
/// `not_established` for the property the chart claims. A stage outside the stage rules, a fragment stage and a chart
/// that names another stage are refused before `admit_chart` is asked. An exact property claim needs LB4.2's checks:
/// the claim is established only by exact checks of this chart at exact poses, with those scopes; checks of another
/// chart, and a survey of float poses only, establish nothing; a refuted check refuses the chart. Not run: reading the
/// declared outputs from a stage module (no stage-language checker), HLSL compilation and the compile record.
#[test]
fn ct_lb4_04_admission_half_a_user_chart_stays_not_established_without_exact_checks() {
    let description = describe("user chart stage", StageKind::Vertex, &[Output::Position, Output::CoverageCode]);
    let user = user_chart(9, &description, &["injective"]);

    // Admission alone.
    let admitted = admit_user_chart(&description, &user, &[]).expect("the stage rules pass");
    assert_eq!(admitted.stage.digest(), description.digest);
    assert!(matches!(admitted.admission, Admission::NotEstablished(_)), "{:?}", admitted.admission);
    assert_eq!(admitted.statuses, vec![("injective".to_owned(), PropertyStatus::NotEstablished)]);

    // The stage rules come first.
    let missing_coverage = describe("user chart stage", StageKind::Vertex, &[Output::Position]);
    let extra_depth =
        describe("user chart stage", StageKind::Vertex, &[Output::Position, Output::CoverageCode, Output::Depth]);
    for bad in [&missing_coverage, &extra_depth] {
        match admit_user_chart(bad, &user, &[]) {
            Err(UserChartError::Stage(error)) => assert_eq!(error.code, codes::STAGE_RULE_VIOLATION),
            other => panic!("{other:?}"),
        }
    }
    let fragment = describe("user chart stage", StageKind::Fragment, &[Output::Rgb]);
    assert_eq!(admit_user_chart(&fragment, &user, &[]), Err(UserChartError::NotAVertexStage));
    let another = describe("another stage", StageKind::Vertex, &[Output::Position, Output::CoverageCode]);
    assert_eq!(admit_user_chart(&another, &user, &[]), Err(UserChartError::Unbound));
    assert_eq!(admit_user_chart(&description, &built_in(1), &[]), Err(UserChartError::Unbound));

    // LB4.2's checks, for this chart, at exact poses.
    let cell = cell();
    let declaration = Stereographic { pole: vertex_sum(&cell) };
    let flat = region("cell", cell.clone());
    let base = revision(12);
    let rep = shown(&base, built_in(1));
    let properties = [Property::Domain, Property::Injective];
    let surveyed = |chart: &Chart, poses: &[CameraPose]| {
        let subject = Subject {
            plan: &rep.plan,
            chart,
            declaration: &declaration,
            region: &flat,
            properties: &properties,
            rule: RULE,
        };
        survey(&subject, poses, &reference_decision).expect("surveyed")
    };
    let exact_poses = [CameraPose::Exact(antipodal()), CameraPose::Exact(left(&cell[1]))];
    let checks = surveyed(&user, &exact_poses).checks;
    let established = admit_user_chart(&description, &user, &checks).expect("the stage rules pass");
    assert_eq!(established.admission, Admission::Admitted);
    let [(property, PropertyStatus::Established(scopes))] = &established.statuses[..] else {
        panic!("{:?}", established.statuses)
    };
    assert_eq!(property, "injective");
    let covered: Vec<String> = scopes.iter().flat_map(|s| s.poses.clone()).collect();
    assert_eq!(covered, vec![antipodal().id(), left(&cell[1]).id()], "the exact poses checked, and no other");

    // Float poses give no check, and the built-in chart's checks are not this chart's.
    let between = surveyed(&user, &[float_turn(0.3), float_turn(2.0)]);
    assert!(between.checks.is_empty());
    assert_eq!(between.frames.len(), 2);
    for checks in [between.checks, surveyed(&built_in(1), &exact_poses).checks] {
        let open = admit_user_chart(&description, &user, &checks).expect("the stage rules pass");
        assert!(matches!(open.admission, Admission::NotEstablished(_)), "{:?}", open.admission);
    }

    // A refuted check refuses the chart: the domain fails at the identity.
    let claiming_domain = user_chart(9, &description, &["domain"]);
    let checks = surveyed(&claiming_domain, &[CameraPose::Exact(Pose::identity())]).checks;
    let refused = admit_user_chart(&description, &claiming_domain, &checks).expect("the stage rules pass");
    let Admission::Refused(error) = refused.admission else { panic!("{:?}", refused.admission) };
    assert_eq!(error.code, codes::CHART_CHECK_FAILED);
}

/// CT-LB4-04 (part, the CPU half of the definition binding, and built-in charts). The CPU reference's reproducibility
/// key names the user chart's content identity, which names the stage's digest: another stage definition is another
/// key, and a claimed property is not part of it. The host reports user stages unsupported (F05), so a plan with a
/// user stage primitive, or with a chart whose stage is no admitted definition, is refused with no substitute, while
/// the built-in chart validates and installs before and after. Not run: the GPU half of the binding (no backend, no
/// compiled stage), and the reference evaluates the chart's exact declaration, not a user stage's code, because no
/// stage language runs here.
#[test]
fn ct_lb4_04_cpu_half_the_reference_key_binds_the_user_chart_and_built_in_charts_stay_available() {
    let description = describe("user chart stage", StageKind::Vertex, &[Output::Position, Output::CoverageCode]);
    let user = user_chart(9, &description, &["injective"]);
    let cell = cell();
    let declaration = Stereographic { pole: vertex_sum(&cell) };
    let base = revision(12);
    let rep = shown(&base, built_in(1));
    let pose = CameraPose::Exact(antipodal());
    let evaluate = |chart: &Chart| {
        let fixture = Fixture { plan: &rep.plan, chart, declaration: &declaration, vertices: &cell, rule: RULE };
        reference::evaluate(&fixture, &pose)
    };

    let bound = evaluate(&user);
    assert_eq!(bound.exact.chart, chart::identity(&user));
    assert_ne!(bound.key(), evaluate(&built_in(9)).key(), "the user chart is not the built-in chart");
    let other = describe("another stage", StageKind::Vertex, &[Output::Position, Output::CoverageCode]);
    assert_ne!(bound.key(), evaluate(&user_chart(9, &other, &["injective"])).key(), "another definition, another key");
    assert_eq!(bound.key(), evaluate(&user_chart(9, &description, &[])).key(), "a claim is not part of the identity");
    assert_eq!(bound, evaluate(&user), "the same key gives the same evaluation");

    // Built-in charts are available.
    let built = publication(1, &base, vec![rep.clone()]);
    let validated = validate(&built, &context()).expect("the built-in chart validates");
    assert_eq!(validated.charts.len(), 1);
    assert_eq!(validated.charts[0].admission, Admission::Admitted);

    // A user stage is not: the host reports F05 unsupported, and a chart naming a stage that is no admitted
    // definition gets no substitute.
    let mut with_stage = built.clone();
    with_stage.plans[0].primitives.push(Primitive::UserStage(UserStageInput {
        kind: StageKind::Vertex,
        digest: description.digest.clone(),
        compile_record: sha256_hex(b"compile record"),
    }));
    let error = validate(&with_stage, &context()).expect_err("no stage checker");
    assert_eq!(error.code, codes::UNSUPPORTED);
    assert_eq!(error.context["feature"], "F05");
    let through_user = publication(1, &base, vec![shown(&base, user.clone())]);
    let error = validate(&through_user, &context()).expect_err("no admitted definition");
    assert_eq!(error.code, codes::UNSUPPORTED);

    // Neither refusal took the built-in chart away.
    let mut host = host(&base);
    install(&mut host, 1, built.clone());
    assert_eq!(validate(&built, &context()).expect("still valid").digest, validated.digest);
    assert_eq!(host.adopted(), &base);
}
