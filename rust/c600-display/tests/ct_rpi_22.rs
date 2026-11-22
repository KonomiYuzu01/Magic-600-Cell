//! CT-RPI-22 (source, part: the chart half: failed chart checks affect only their display output; a failure changes
//! no session and certifies no negative, RPI.48 to RPI.50). The limit half is in `c600-view`'s `view_rows.rs`.

mod common;

use c600_display::chart::{
    self, Admission, CameraPose, ChartAdmission, ChartCheck, Pose, Property, PropertyStatus, Region, Stereographic,
    Verdict,
};
use c600_display::exact::{self, Q5};
use c600_display::plan::*;
use c600_display::validate::validate;
use c600_display::{codes, Form};
use common::*;

/// A plan drawing the instances `ids` through `chart`.
fn charted(id: u64, base: &RevisionRef, ids: &[u64], chart: Chart) -> (DisplayPlan, SourceMap) {
    let instances = ids.iter().map(|&i| instance(i, id, ObjectId::Sticker(i as u32), 0)).collect();
    let primitives =
        vec![Primitive::Layout(sheet_layout(ids)), Primitive::ChartStage(StageInput { chart: chart.chart })];
    let (mut drawn, map) = plan(id, base, primitives, instances);
    drawn.charts.push(chart);
    (drawn, map)
}

/// The built-in chain with its rotation parameters at `angle`, claiming `property`: another content identity per angle.
fn claiming(id: u64, angle: f64, property: &str) -> Chart {
    let mut claimed = built_in(id);
    claimed.stages[0].params = vec![angle; 6];
    claimed.properties = vec![property.to_owned()];
    claimed
}

/// Exact check results on the cell fixture: the domain refuted at the identity (the pole is on the cell's axis), the
/// domain established at the antipodal pose, and injectivity unknown on a region through the origin.
fn verdicts() -> (Verdict, Verdict, Verdict) {
    let cell = cell();
    let declaration = Stereographic { pole: vertex_sum(&cell) };
    let flat = Region::hull("cell", cell.clone());
    let mut solid = flat.clone();
    solid.name = "solid".to_owned();
    solid.vertices.push(exact::scale(Q5::int(-1), &E1).expect("small values"));
    let identity = CameraPose::Exact(Pose::identity());
    let mut minus = [[Q5::ZERO; 4]; 4];
    for (i, row) in minus.iter_mut().enumerate() {
        row[i] = Q5::int(-1);
    }
    let antipodal = CameraPose::Exact(Pose::rotation(minus).expect("−1 is in K⁺"));
    let refuted = chart::check(&declaration, &Property::Domain, &flat, &identity).expect("exact");
    let established = chart::check(&declaration, &Property::Domain, &flat, &antipodal).expect("exact");
    let unknown = chart::check(&declaration, &Property::Injective, &solid, &identity).expect("exact");
    assert!(matches!(refuted, Verdict::Refuted { .. }), "{refuted:?}");
    assert!(matches!(established, Verdict::Established { .. }), "{established:?}");
    assert!(matches!(unknown, Verdict::NotEstablished { .. }), "{unknown:?}");
    (refuted, established, unknown)
}

#[test]
fn ct_rpi_22_chart_half_admission_consumes_checks_and_keeps_unknowns() {
    let (refuted, established, unknown) = verdicts();
    let failing = claiming(1, 0.25, "domain");
    let passing = claiming(2, 0.5, "domain");
    let open = claiming(3, 0.75, "injective");
    let checks = vec![
        ChartCheck { chart: chart::identity(&failing), property: "domain".to_owned(), verdict: refuted.clone() },
        ChartCheck { chart: chart::identity(&passing), property: "domain".to_owned(), verdict: established.clone() },
        ChartCheck { chart: chart::identity(&open), property: "injective".to_owned(), verdict: unknown },
    ];

    // A refuted check refuses the chart, naming the chart, the property and the check; the status is withheld.
    let (admission, statuses) = chart::admit_chart(&failing, &checks);
    let Admission::Refused(error) = admission else { panic!("{admission:?}") };
    assert_eq!((error.code, error.form), (codes::CHART_CHECK_FAILED, Form::Refusal));
    assert_eq!(error.context["chart"], "1");
    assert_eq!(error.context["property"], "domain");
    assert_eq!(error.context["check"], checks[0].id());
    assert_eq!(statuses, vec![("domain".to_owned(), PropertyStatus::NotEstablished)]);

    // An established check admits its chart, with the scope it covers and nothing wider.
    let (admission, statuses) = chart::admit_chart(&passing, &checks);
    assert_eq!(admission, Admission::Admitted);
    assert_eq!(statuses, vec![("domain".to_owned(), PropertyStatus::Established(vec![established.scope().clone()]))]);

    // An unknown result stays unknown: neither false nor a pass.
    let (admission, statuses) = chart::admit_chart(&open, &checks);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
    assert_eq!(statuses, vec![("injective".to_owned(), PropertyStatus::NotEstablished)]);

    // A check of another chart, even the same property, is not this chart's check; with none, C2's default stands.
    let unchecked = claiming(4, 1.0, "domain");
    let (admission, statuses) = chart::admit_chart(&unchecked, &checks);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
    assert_eq!(statuses, vec![("domain".to_owned(), PropertyStatus::NotEstablished)]);
    let (admission, _) = chart::admit_chart(&failing, &[]);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
}

#[test]
fn ct_rpi_22_chart_half_a_failed_check_affects_only_its_output() {
    let (refuted, established, unknown) = verdicts();
    let base = revision(5);
    let failing = claiming(1, 0.25, "domain");
    let passing = claiming(1, 0.5, "domain");
    let open = claiming(1, 0.75, "injective");
    let checks = vec![
        ChartCheck { chart: chart::identity(&failing), property: "domain".to_owned(), verdict: refuted.clone() },
        ChartCheck { chart: chart::identity(&passing), property: "domain".to_owned(), verdict: established.clone() },
        ChartCheck { chart: chart::identity(&open), property: "injective".to_owned(), verdict: unknown },
    ];
    let mut ctx = context();
    ctx.checks = checks.clone();
    let publication = publication(
        1,
        &base,
        vec![
            charted(1, &base, &[1, 2], failing.clone()),
            charted(2, &base, &[3, 4], passing.clone()),
            charted(3, &base, &[5, 6], open.clone()),
        ],
    );

    // The session has adopted `base` and installed an earlier publication.
    let mut host = host_with(&base, ctx.clone());
    install(&mut host, 1, sheet(1, &base, 1));
    let adopted = host.adopted().clone();
    let earlier = host.receipt(CommandId(1)).expect("a receipt");

    // The publication with the failing chart installs: the other display data continues.
    install(&mut host, 2, publication.clone());
    let validated = host.installed(ViewId(1)).expect("installed").validated.clone();
    let failure = match &validated.charts[0] {
        ChartAdmission { plan: PlanId(1), chart: ChartId(1), admission: Admission::Refused(error), coverage } => {
            assert_eq!(coverage, &Coverage::Undefined(codes::CHART_CHECK_FAILED.to_owned()));
            error.clone()
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(failure.code, codes::CHART_CHECK_FAILED);
    assert_eq!(failure.context["check"], checks[0].id());
    assert_eq!(
        validated.charts[1],
        ChartAdmission {
            plan: PlanId(2),
            chart: ChartId(1),
            admission: Admission::Admitted,
            coverage: Coverage::Complete
        }
    );
    assert!(matches!(
        &validated.charts[2],
        ChartAdmission { plan: PlanId(3), admission: Admission::NotEstablished(_), coverage: Coverage::Complete, .. }
    ));
    // Every plan's subject is shown; the failed chart's plan keeps its subject too.
    assert_eq!(validated.subjects.iter().map(|s| s.plan).collect::<Vec<_>>(), vec![PlanId(1), PlanId(2), PlanId(3)]);

    // No negative is certified: the failed property is withheld, not false, and the unknown one stays unknown.
    assert_eq!(
        validated.properties,
        vec![
            (ChartId(1), "domain".to_owned(), PropertyStatus::NotEstablished),
            (ChartId(1), "domain".to_owned(), PropertyStatus::Established(vec![established.scope().clone()])),
            (ChartId(1), "injective".to_owned(), PropertyStatus::NotEstablished),
        ]
    );
    // The check result itself is unchanged, still scoped to its region and pose.
    assert_eq!(ctx.checks[0].verdict, refuted);

    // No failure changes the session, another receipt or the publication's identity.
    assert_eq!(host.adopted(), &adopted);
    assert_eq!(host.receipt(CommandId(1)).expect("a receipt"), earlier);
    assert_eq!(validated.digest, validate(&publication, &context()).expect("valid without checks").digest);
    one_terminal_each(&host);
}
