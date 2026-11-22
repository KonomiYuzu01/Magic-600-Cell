//! CT-RM-07 (source, whole): RM.37's fifteen jumbling rows as fixture payloads, one test per row in the table's order.
//! They are source-contract outcomes over supplied producer data under a synthetic jumbling identity; no jumbling
//! geometry, kernel or backend run is involved here. `jumbling.rs` runs the rows that M5 can check on its real model,
//! and `c600-jumble`'s `faces.rs` row 10's geometry. Bindings are current and budgets fit unless a row says otherwise.

mod common;

use c600_display::array::Array;
use c600_display::codes;
use c600_display::crosswalk::*;
use c600_display::pick::{Partial, PickMode, PickReply, Requested};
use c600_display::plan::*;
use c600_display::produce;
use c600_display::receipt::{is_reserved, CancelReason, JobOutcome, Status};
use c600_display::validate::{self, validate, BoundArray, Context, EngineBundle, LimitProfile, Support};
use c600_display::Form;

use common::*;

const V1: ViewId = ViewId(1);

/// A jumbling context over the retained geometry: the fixture rows plus J03, J04 and J05 (with the fixture's
/// built-in stages and no `hopf`). J04's declared `overlay_items` ceiling is `overlay_items`; every other ceiling fits.
fn jumbling_context(overlay_items: u64) -> Context {
    let mut ctx = context();
    ctx.identity = fixture_identity();
    let supported = |limits: LimitProfile, stages| Support::Supported { layout_version: 1, limits, stages };
    let stages = produce::STAGES.iter().map(|&(name, params)| produce::stage_def(name, params)).collect();
    let mut certificate = limits(1 << 40);
    certificate.overlay_items = overlay_items;
    ctx.support.features.insert("J03".to_owned(), supported(limits(1 << 40), Vec::new()));
    ctx.support.features.insert("J04".to_owned(), supported(certificate, Vec::new()));
    ctx.support.features.insert("J05".to_owned(), supported(limits(1 << 40), stages));
    ctx
}

/// A jumbling session revision with Jumble on.
fn jumbling(state: u64) -> RevisionRef {
    RevisionRef { identity: fixture_identity(), mode: Mode::JumbleOn, ..revision(state) }
}

/// A one-plan jumbling publication in view 1.
fn single(base: &RevisionRef, primitives: Vec<Primitive>, instances: Vec<DisplayInstance>) -> Publication {
    let (mut plan, map) = plan(1, base, primitives, instances);
    plan.geometry.identity = fixture_identity();
    publication(1, base, vec![(plan, map)])
}

fn point(exact: &str, vector: [f32; 4]) -> CertPoint {
    CertPoint { exact: exact.to_owned(), vector }
}

/// RM.37's full named-cap straddler set at grip 4 and cap 0: pieces 7 and 9, each with one verified point pair.
fn straddlers() -> CertOverlay {
    CertOverlay {
        grip: 4,
        cap: 0,
        kind: CertKind::FullNamedCapSet,
        pieces: vec![7, 9],
        set_digest: piece_set_digest(&[7, 9]),
        pairs: vec![
            PointPair {
                piece: 7,
                points: [point("(1/2,0,0,1/2)", [0.5, 0.0, 0.0, 0.5]), point("(1/2,0,1/2,0)", [0.5, 0.0, 0.5, 0.0])],
            },
            PointPair {
                piece: 9,
                points: [point("(0,1/2,0,1/2)", [0.0, 0.5, 0.0, 0.5]), point("(0,1/2,1/2,0)", [0.0, 0.5, 0.5, 0.0])],
            },
        ],
        survey: Vec::new(),
        cut: None,
    }
}

/// The cap-map witness, which names only piece 7.
fn witness() -> CertOverlay {
    CertOverlay {
        kind: CertKind::CapMapWitness,
        pieces: vec![7],
        set_digest: piece_set_digest(&[7]),
        pairs: Vec::new(),
        ..straddlers()
    }
}

fn overlay(publication: &Publication) -> &CertOverlay {
    match &publication.plans[0].primitives[0] {
        Primitive::CertOverlay(overlay) => overlay,
        other => panic!("{other:?}"),
    }
}

fn ceiling_counts(error: &c600_display::Error) -> (&str, &str, &str) {
    (error.context["field"].as_str(), error.context["requested"].as_str(), error.context["ceiling"].as_str())
}

/// The three pose arrays of a jumbling bundle, bound to the revisions `bound`.
fn pose_bundle(revision: &RevisionRef, bound: [u64; 3]) -> EngineBundle {
    let arrays = [
        ("pose_index", Array::u32s(vec![4], &[0, 1, 1, 0])),
        ("poses", Array::f32s(vec![2, 4, 4], &[0.0; 32])),
        ("pose_lattice", Array::i32s(vec![2, 4], &[0; 8])),
    ];
    EngineBundle {
        revision: revision.clone(),
        arrays: arrays
            .into_iter()
            .zip(bound)
            .map(|((name, array), revision)| BoundArray { name: name.to_owned(), revision, array })
            .collect(),
    }
}

#[test]
fn ct_rm_07_row_01_a_mixed_pose_bundle_is_refresh_required() {
    let twelve = jumbling(12);
    let thirteen = jumbling(13);
    let mut host = host_with(&twelve, jumbling_context(1 << 40));
    let error = host.adopt(&pose_bundle(&thirteen, [12, 13, 13])).expect_err("a revision-12 pose index");
    assert_eq!(error.code, codes::BUNDLE_MISMATCH);
    assert_eq!(host.adopted(), &twelve, "no mixed bundle is adopted");
    assert_eq!(host.adopt(&pose_bundle(&thirteen, [13; 3])), Ok(thirteen));
}

#[test]
fn ct_rm_07_row_02_a_half_turn_without_a_plane_has_no_swept_preview() {
    let base = jumbling(12);
    let half_turn = TransformInput {
        grip: 4,
        plane: None,
        angle: std::f32::consts::PI,
        moving: Array::u32s(vec![2], &[7, 9]),
        moving_digest: piece_set_digest(&[7, 9]),
        from: 12,
        to: 13,
    };
    assert!(!validate::swept_preview(&half_turn));
    let mut host = host_with(&base, jumbling_context(1 << 40));
    install(&mut host, 1, single(&base, vec![Primitive::Transform(half_turn.clone())], Vec::new()));
    let installed = &host.installed(V1).expect("installed").publication.plans[0].primitives[0];
    assert_eq!(installed, &Primitive::Transform(half_turn), "no guessed plane, moving set or legality");
}

#[test]
fn ct_rm_07_row_03_the_full_straddler_set_is_kept_whole() {
    let base = jumbling(12);
    let ctx = jumbling_context(1 << 40);
    let full = single(&base, vec![Primitive::CertOverlay(straddlers())], Vec::new());
    assert_eq!(validate(&full, &ctx).expect("the full payload").counts["overlay_items"], 6);
    let mut host = host_with(&base, ctx.clone());
    install(&mut host, 1, full);
    let kept = overlay(&host.installed(V1).expect("installed").publication);
    assert_eq!((kept.pieces.as_slice(), kept.pairs.len()), ([7, 9].as_slice(), 2));
    assert_eq!(kept.pairs.iter().flat_map(|pair| &pair.points).count(), 4, "all four points");
    validate(&single(&base, vec![Primitive::CertOverlay(witness())], Vec::new()), &ctx).expect("the witness alone");

    let mut truncated = straddlers();
    truncated.pieces = vec![7];
    truncated.pairs.pop();
    let error =
        validate(&single(&base, vec![Primitive::CertOverlay(truncated)], Vec::new()), &ctx).expect_err("truncated");
    assert_eq!(error.code, codes::PLAN_INVALID, "the witness cannot replace the full set");
    let mut unchecked = straddlers();
    unchecked.pairs[1].points[1].vector[2] = f32::NAN;
    let error =
        validate(&single(&base, vec![Primitive::CertOverlay(unchecked)], Vec::new()), &ctx).expect_err("one bad point");
    assert_eq!(error.code, codes::PLAN_INVALID, "admission checks the entire payload");
}

#[test]
fn ct_rm_07_row_04_an_over_ceiling_standalone_payload_is_refused() {
    let base = jumbling(12);
    let mut host = host_with(&base, jumbling_context(5));
    host.publish(CommandId(1), single(&base, vec![Primitive::CertOverlay(witness())], Vec::new()));
    let error =
        refused(&host.publish(CommandId(2), single(&base, vec![Primitive::CertOverlay(straddlers())], Vec::new())));
    assert_eq!(error.code, codes::PLAN_CEILING);
    assert_eq!(ceiling_counts(&error), ("overlay_items", "6", "5"));
    assert!(!is_reserved(&error), "no KN7:budget.display_plan_limit substitutes");
    assert_eq!(status(&host, 1), Status::Accepted, "A stays pending");
    finish(&mut host, 1);
    let installed = host.installed(V1).expect("A installs");
    assert_eq!(installed.command, CommandId(1));
    assert_eq!(overlay(&installed.publication), &witness(), "no part of B, and no one-witness truncation of it");
    one_terminal_each(&host);
}

#[test]
fn ct_rm_07_row_05_over_ceiling_scheduled_output_fails_its_invocation() {
    let base = jumbling(12);
    let mut host = host_with(&base, jumbling_context(5));
    host.publish(CommandId(1), single(&base, vec![Primitive::CertOverlay(witness())], Vec::new()));
    let running = host.schedule(CommandId(2), V1, u64::MAX);
    let Status::Running(job) = running.status else { panic!("{:?}", running.status) };
    let superseded = Status::Cancelled(CancelReason::Superseded { by: CommandId(2) });
    assert_eq!(status(&host, 1), superseded);

    let payload = single(&base, vec![Primitive::CertOverlay(straddlers())], Vec::new());
    let receipt = host.deliver(CommandId(2), payload.clone()).expect("a receipt");
    let error = failed(&receipt);
    assert_eq!((error.code, ceiling_counts(&error)), (codes::PLAN_CEILING, ("overlay_items", "6", "5")));
    assert_eq!(
        (receipt.job, receipt.request.as_str()),
        (Some(job), running.request.as_str()),
        "the original invocation"
    );
    assert_eq!(host.job_outcome(CommandId(2)), Some(&JobOutcome::Error(error.clone())));
    assert_eq!(host.deliver(CommandId(2), payload), Some(receipt), "no second admission");
    finish(&mut host, 1);
    assert!(host.installed(V1).is_none(), "no resurrection or partial installation");
    assert_eq!(status(&host, 1), superseded, "A stays cancelled");
    one_terminal_each(&host);
}

#[test]
fn ct_rm_07_row_06_an_uncertain_survey_stays_pending_and_the_cut_keeps_its_plane() {
    let base = jumbling(12);
    let mut survey: Vec<GripStatus> =
        (0..600).map(|grip| if grip % 2 == 0 { GripStatus::A } else { GripStatus::B }).collect();
    survey[321] = GripStatus::Uncertain;
    assert_eq!(survey_state(&survey), SurveyState::Pending { uncertain: vec![321], missing: 0 });
    assert_eq!(survey.iter().filter(|s| **s != GripStatus::Uncertain).count(), 599, "never 600 certified statuses");

    let cut = NamedCut { pole: 4, plane: "<n_4, x> = 121/125".to_owned(), alpha: (121, 125) };
    let mut payload = straddlers();
    payload.survey = survey.clone();
    payload.cut = Some(cut.clone());
    let mut host = host_with(&base, jumbling_context(1 << 40));
    install(&mut host, 1, single(&base, vec![Primitive::CertOverlay(payload.clone())], Vec::new()));
    let kept = overlay(&host.installed(V1).expect("installed").publication);
    assert_eq!((&kept.survey, &kept.cut), (&survey, &Some(cut)), "the supplied exact plane, nothing mesh-derived");

    payload.cut = Some(NamedCut { pole: 4, plane: String::new(), alpha: (121, 125) });
    let error = validate(&single(&base, vec![Primitive::CertOverlay(payload)], Vec::new()), &jumbling_context(1 << 40))
        .expect_err("a cut without its plane");
    assert_eq!(error.code, codes::PLAN_INVALID);
}

#[test]
fn ct_rm_07_row_07_an_unsupported_projection_stage_has_no_substitute() {
    let base = jumbling(12);
    let staged = |stages: &[(&str, usize)]| {
        let mut publication = single(&base, vec![Primitive::ChartStage(StageInput { chart: ChartId(1) })], Vec::new());
        publication.plans[0].charts.push(chart(1, stages));
        publication
    };
    let (refusal, failure) = both_sides(staged(&[("rotate", 6), ("hopf", 0)]), jumbling_context(1 << 40));
    assert_eq!((refusal.code, failure.code), (codes::UNSUPPORTED, codes::UNSUPPORTED));
    assert_eq!(refusal.context["feature"], "J05");
    assert_eq!(failure, refusal.in_form(Form::Stop));
    validate(&staged(&[("rotate", 6), ("stereographic", 0)]), &jumbling_context(1 << 40)).expect("admitted stages");
}

#[test]
fn ct_rm_07_row_08_the_seam_total_counts_every_mismatch() {
    let seams = vec![
        Seam { domain: 1, cell: 10, on_grip: true },
        Seam { domain: 2, cell: 11, on_grip: false },
        Seam { domain: 3, cell: 12, on_grip: false },
    ];
    let complete = SeamTable { seams: seams.clone(), complete: true };
    assert_eq!(complete.total(), Partial::Total(3), "N counts off-grip mismatches too, not only the on-grip one");
    assert_eq!(complete.seams.iter().filter(|s| !s.on_grip).count(), 2, "off-grip entries stay in the payload");
    let interrupted = SeamTable { seams: seams[..2].to_vec(), complete: false };
    assert!(matches!(interrupted.total(), Partial::Undefined(_)), "an interrupted enumeration has no total");
    assert_eq!(interrupted.seams, seams[..2], "its entries keep their domain and cell identities");
}

#[test]
fn ct_rm_07_row_09_the_cover_table_and_a_scoped_empty_repair_list() {
    let meeting: Vec<u32> = (1..=80).collect();
    let cover = CoverTable::new(0, meeting.clone()).expect("a completed cover table");
    assert_eq!((cover.cap, cover.rows()), (0, meeting.as_slice()), "all 80 rows in cap-ID order");
    let mut swapped = meeting;
    swapped.swap(0, 1);
    assert!(CoverTable::new(0, swapped).is_err(), "rows are never reordered");

    let input = "input:fixture-state-12".to_owned();
    let empty = RepairList::Completed { cap: 0, input: input.clone(), repairs: Vec::new() };
    assert_eq!(empty.negative(), Some(format!("no repair at cap 0 for input {input}")), "scoped to its cap and input");
    assert_eq!(
        RepairList::CutOff { cap: 0, input: input.clone() }.negative(),
        None,
        "a cut-off list supplies no negative"
    );
    assert_eq!(RepairList::Completed { cap: 0, input, repairs: vec![(3, 5)] }.negative(), None);
}

#[test]
fn ct_rm_07_row_10_missing_exposed_geometry_is_unavailable() {
    let error = exposed_geometry(None, Some("profile/1")).expect_err("no certified geometry");
    assert_eq!((error.code, error.context["missing"].as_str()), (codes::EXPOSED_GEOMETRY_UNAVAILABLE, "geometry"));
    let faces = b"faces".as_slice();
    let error = exposed_geometry(Some(faces), None).expect_err("no size profile");
    assert_eq!((error.code, error.context["missing"].as_str()), (codes::EXPOSED_GEOMETRY_UNAVAILABLE, "size profile"));
    assert!(exposed_geometry(Some(faces), Some("profile/1")).is_ok());
    let ctx = jumbling_context(1 << 40);
    assert_eq!(ctx.support.feature("J10").expect_err("no supported-feature claim").code, codes::UNSUPPORTED);
}

#[test]
fn ct_rm_07_row_11_release_order_and_revision_clearing() {
    let base = jumbling(12);
    let mut host = host_with(&base, jumbling_context(1 << 40));
    let mut log = ReleaseLog::new(12);
    assert!(log.check(12, "C") && log.check(12, "D"));

    let release = |id: u64| {
        let shown = vec![instance(id, 1, ObjectId::Sticker(id as u32), 0)];
        single(&base, vec![Primitive::Layout(sheet_layout(&[id]))], shown)
    };
    host.publish(CommandId(1), release(1));
    host.publish(CommandId(2), release(2));
    assert_eq!(status(&host, 1), Status::Cancelled(CancelReason::Superseded { by: CommandId(2) }));
    assert!(!log.release(12, "B", false), "B appends only after completion");
    finish(&mut host, 1);
    assert!(host.installed(V1).is_none(), "A cannot publish late");
    finish(&mut host, 2);
    let completed = matches!(status(&host, 2), Status::Completed(_));
    assert!(log.release(12, "B", completed));
    assert_eq!(log.entries(), ["C", "D", "B"], "C then D keep their tried order");

    log.set_revision(13);
    assert!(log.entries().is_empty(), "revision 13 clears the revision-12 checks");
    assert!(!log.check(12, "E"), "a revision-12 check no longer records");
    one_terminal_each(&host);
}

#[test]
fn ct_rm_07_row_12_off_lattice_nu_is_undefined_and_the_catalogue_is_unfiltered() {
    assert_eq!(nu(&NuObservation::OffLattice), Ok(Partial::Undefined("off_lattice".to_owned())), "no zero or old nu");
    let catalogue = Catalogue::fixture(4, 5);
    let ids: Vec<u32> = catalogue.rows().iter().map(|row| row.id).collect();
    assert_eq!(ids, (0..36).collect::<Vec<u32>>(), "36 rows in fixed ID order");
}

#[test]
fn ct_rm_07_row_13_zero_multiplicities_keep_nu_zero() {
    let observed = nu(&NuObservation::Lattice { nu: vec![0; 4], fixed: vec![0, 1] }).expect("a lattice result");
    assert_eq!(observed, Partial::Total(vec![0; 4]));
    let catalogue = Catalogue::fixture(4, 5);
    assert_eq!(catalogue.predict(&[0; CORRECTIONS]), vec![0; 4], "JI.13's finite-group sum of zeros");
    assert_eq!(catalogue.rows().len(), CORRECTIONS);
    assert!(catalogue.rows().windows(2).all(|w| w[0].id < w[1].id), "the rows keep their fixed order");
}

/// Row 13 over coordinates with their own moduli, as ν has (F2 bits, F5 values): each coordinate sums modulo its own
/// modulus, the derived exponent is their least common multiple, and a catalogue that is not 36 rows in ID order with
/// one residue per coordinate is refused, never repaired.
#[test]
fn ct_rm_07_row_13_mixed_moduli_sum_each_coordinate_in_its_own_group() {
    let effect = |id: u32| vec![i64::from(id % 2), i64::from(id % 5), i64::from((id + 1) % 5)];
    let rows = || (0..CORRECTIONS as u32).map(|id| Correction { id, effect: effect(id) }).collect::<Vec<_>>();
    let catalogue = Catalogue::new(rows(), vec![2, 5, 5]).expect("36 rows over moduli 2, 5 and 5");
    assert_eq!((catalogue.moduli(), catalogue.modulus), (&[2, 5, 5][..], 10), "the exponent is derived");
    assert_eq!(catalogue.predict(&[0; CORRECTIONS]), vec![0, 0, 0]);
    let mut multiplicities = [0i64; CORRECTIONS];
    multiplicities[1] = 3; // effect [1, 1, 2]
    multiplicities[3] = -1; // effect [1, 3, 4]
    assert_eq!(catalogue.predict(&multiplicities), vec![0, 0, 2], "3·[1,1,2] − [1,3,4] in F2 × F5 × F5");

    let mut reordered = rows();
    reordered.swap(0, 1);
    let mut wide = rows();
    wide[7].effect.push(0);
    let mut outside = rows();
    outside[4].effect[0] = 2;
    let refused = [
        Catalogue::new(rows()[..35].to_vec(), vec![2, 5, 5]),
        Catalogue::new(reordered, vec![2, 5, 5]),
        Catalogue::new(wide, vec![2, 5, 5]),
        Catalogue::new(outside, vec![2, 5, 5]),
        Catalogue::new(rows(), vec![2, 5, 1]),
    ];
    for (case, result) in refused.into_iter().enumerate() {
        assert_eq!(result.expect_err("refused").code, codes::PLAN_INVALID, "case {case}");
    }
}

#[test]
fn ct_rm_07_row_14_a_nonzero_fixed_nu_coordinate_freezes() {
    let error = nu(&NuObservation::Lattice { nu: vec![0, 2, 0, 0], fixed: vec![1] }).expect_err("a fixed coordinate");
    assert_eq!((error.code, error.form), (codes::FIXED_NU_NONZERO, Form::Freeze));
    assert_eq!(error.context["coordinate"], "1");
    let free = nu(&NuObservation::Lattice { nu: vec![0, 2, 0, 0], fixed: vec![0] }).expect("a free coordinate");
    assert_eq!(free, Partial::Total(vec![0, 2, 0, 0]));
}

#[test]
fn ct_rm_07_row_15_a_certificate_point_hit_resolves_its_witness_piece() {
    let base = jumbling(12);
    let hit = ObjectId::Point { witness: 7, grip: 4, role: "pair 0, point 1".to_owned() };
    let mut host = host_with(&base, jumbling_context(1 << 40));
    install(
        &mut host,
        1,
        single(&base, vec![Primitive::CertOverlay(straddlers())], vec![instance(1, 1, hit.clone(), 0)]),
    );
    let reply = PickReply { binding: host.pick_binding(V1).expect("current"), instance: InstanceId(1) };
    let resolve = |requested| host.resolve_pick(&reply, PickMode::Explicit, requested, &OffLattice).expect("current");

    let Some(Partial::Total(piece)) = resolve(Requested::Piece) else { panic!("the witness piece") };
    assert_eq!((piece.target, &piece.occurrence), (ObjectId::Piece(7), &hit), "grip and source role are retained");
    let Some(Partial::Total(grip)) = resolve(Requested::Grip) else { panic!("the named grip") };
    assert_eq!(grip.target, ObjectId::Grip(4));
    let Some(Partial::Partial { covered, missing }) = resolve(Requested::Slot) else { panic!("off the lattice") };
    assert_eq!((covered.target, missing), (ObjectId::Piece(7), vec!["slot: off_lattice".to_owned()]), "no slot");
    assert!(matches!(resolve(Requested::Sticker), Some(Partial::Undefined(_))), "it is not a sticker hit");

    host.set_camera(V1);
    let error = host.pick_source(&reply, EDITED.as_bytes(), &Unasked).expect_err("stale");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "camera"), "RM.10's order");
    assert_eq!(host.resolve_pick(&reply, PickMode::Passive, Requested::Piece, &Unasked), Ok(None));
}
