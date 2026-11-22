//! Plan and publication validation (part P), with source-level fixtures only.
//!
//! - CT-RPI-02 (source, whole).
//! - CT-RPI-04 (source, part: a missing source map installs nothing; the late older completion is in `publication.rs`).
//! - CT-RPI-05 (source, whole).
//! - CT-RPI-08 (source, part: the 600-cell-Full primitive rows, with the posed and certificate rows unsupported; the
//!   jumbling rows are in `jumbling.rs` and `jumbling_payloads.rs`, and J10's geometry in `c600-jumble`'s `faces.rs`;
//!   not run: J01 and J06 to J12 as a backend's supported rows, on the owner's machine (C9)).
//! - CT-RPI-27 (source, whole).

mod common;

use c600_digest::sha256_hex;
use c600_display::array::Array;
use c600_display::codes;
use c600_display::plan::*;
use c600_display::produce::{self, full_set, retained, transform};
use c600_display::receipt::Status;
use c600_display::validate::{self, validate, Shown, SubjectKind};
use c600_kernel::{Snapshot, SLOTS};

use common::*;

/// The solved state and a revision of it.
fn solved(state: u64) -> (Snapshot, RevisionRef) {
    let snapshot = kernel().solved();
    let base = revision_with(state, snapshot.digest());
    (snapshot, base)
}

/// The state after the one-letter word `[1]` from solved, as a kernel derivation.
fn derived() -> c600_kernel::Derived {
    derive(&[1])
}

/// A one-plan publication at `base` with the given primitives and instances.
fn single(base: &RevisionRef, primitives: Vec<Primitive>, instances: Vec<DisplayInstance>) -> Publication {
    publication(1, base, vec![plan(1, base, primitives, instances)])
}

fn code(result: Result<validate::Validated, c600_display::Error>) -> &'static str {
    result.expect_err("the publication is refused").code
}

/// Asserts `EV3:input.display_plan_invalid` for the stated reason.
fn invalid(result: Result<validate::Validated, c600_display::Error>, reason: &str) {
    let error = result.expect_err("the publication is refused");
    assert_eq!(error.code, codes::PLAN_INVALID, "{error:?}");
    assert!(error.summary.contains(reason), "{reason:?} not in {:?}", error.summary);
}

#[test]
fn ct_rpi_02_wrong_model_or_geometry_hash_prevents_installation() {
    let base = revision(12);
    let mut wrong_model = sheet(1, &base, 2);
    wrong_model.plans[0].geometry.identity.model_id = sha256_hex(b"another model");
    let (refusal, failure) = both_sides(wrong_model, context());
    assert_eq!((refusal.code, failure.code), (codes::PLAN_INVALID, codes::PLAN_INVALID));
    assert!(refusal.summary.contains("another model identity"), "{}", refusal.summary);

    let mut wrong_hash = sheet(1, &base, 2);
    wrong_hash.plans[0].geometry.digests.insert("mesh_vertices.f32".to_owned(), sha256_hex(b"regenerated mesh"));
    let (refusal, failure) = both_sides(wrong_hash, context());
    assert_eq!((refusal.code, failure.code), (codes::PLAN_INVALID, codes::PLAN_INVALID));
    assert!(refusal.summary.contains("geometry digests do not match"), "{}", refusal.summary);

    let mut missing_asset = sheet(1, &base, 2);
    missing_asset.plans[0].geometry.digests.remove("cell_frames.f32");
    invalid(validate(&missing_asset, &context()), "geometry digests do not match");
}

#[test]
fn ct_rpi_02_unsupported_dimension_prevents_installation() {
    let base = revision(12);
    let mut five = sheet(1, &base, 2);
    five.plans[0].dimension = 5;
    let (refusal, failure) = both_sides(five, context());
    assert_eq!((refusal.code, failure.code), (codes::UNSUPPORTED, codes::UNSUPPORTED));
    assert_eq!(refusal.context["feature"], "dimension");
    assert_eq!(failure, refusal.in_form(c600_display::Form::Stop));
}

#[test]
fn ct_rpi_02_unknown_stage_prevents_installation_without_substitute() {
    let base = revision(12);
    let mut unknown = sheet(1, &base, 2);
    unknown.plans[0].charts.push(chart(1, &[("rotate", 6), ("hopf", 0)]));
    unknown.plans[0].primitives.push(Primitive::ChartStage(StageInput { chart: ChartId(1) }));
    let (refusal, failure) = both_sides(unknown, context());
    assert_eq!((refusal.code, failure.code), (codes::UNSUPPORTED, codes::UNSUPPORTED));
    assert_eq!(refusal.context["feature"], "F04");

    let mut newer = sheet(1, &base, 2);
    let mut stages = chart(1, &[("rotate", 6)]);
    stages.stages[0].version = 2;
    newer.plans[0].charts.push(stages);
    assert_eq!(code(validate(&newer, &context())), codes::UNSUPPORTED);

    let mut admitted = sheet(1, &base, 2);
    admitted.plans[0].charts.push(chart(1, &[("rotate", 6), ("perspective", 1), ("camera3", 9)]));
    admitted.plans[0].primitives.push(Primitive::ChartStage(StageInput { chart: ChartId(1) }));
    let validated = validate(&admitted, &context()).expect("the built-in chain is admitted");
    assert_eq!(validated.counts["stages"], 3);
}

#[test]
fn ct_rpi_04_missing_source_map_installs_nothing() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 2));
    let before = host.installed(ViewId(1)).cloned().expect("installed");

    let mut missing = sheet_from(1, &base, 10, 3);
    missing.plans[0].source_map = MapId(9);
    missing.nodes.push(ViewNode {
        node: NodeId(1),
        base: base.clone(),
        kind: NodeKind::Object,
        content: vec![Content::Text("a node of the refused publication".to_owned())],
        plans: vec![PlanId(1)],
        children: Vec::new(),
        source: None,
        commands: Vec::new(),
        claims: Vec::new(),
    });
    let error = refused(&host.publish(CommandId(2), missing));
    assert_eq!(error.code, codes::PLAN_INVALID);
    assert!(error.summary.contains("names source map 9, which is missing"), "{}", error.summary);
    host.pump();
    let after = host.installed(ViewId(1)).expect("still installed");
    assert_eq!(after.command, before.command);
    assert_eq!(after.generation, before.generation);
    assert!(after.publication.nodes.is_empty());
    assert_eq!(host.instances(ViewId(1)), vec![InstanceId(1), InstanceId(2)]);
}

#[test]
fn ct_rpi_05_derived_plan_needs_derivation_and_base() {
    let base = revision(12);
    let subject = DerivedRef { digest: sha256_hex(b"derived D"), level: Level::Labels, identity: identity() };
    let derived_sheet = |derivation: Option<Lineage>| {
        let mut publication = sheet(1, &base, 2);
        publication.plans[0].subject = StateRef::Derived(subject.clone());
        publication.plans[0].derivation = derivation;
        publication
    };
    let (refusal, failure) = both_sides(derived_sheet(None), context());
    assert_eq!((refusal.code, failure.code), (codes::PLAN_INVALID, codes::PLAN_INVALID));
    assert!(refusal.summary.contains("only with its derivation"), "{}", refusal.summary);
    let no_base = Lineage::Derivation { base: None, word: sha256_hex(b"word") };
    let (refusal, failure) = both_sides(derived_sheet(Some(no_base)), context());
    assert_eq!((refusal.code, failure.code), (codes::PLAN_INVALID, codes::PLAN_INVALID));
    assert!(refusal.summary.contains("names no base"), "{}", refusal.summary);

    let lineage = Lineage::Derivation { base: Some(StateRef::Snapshot(base.clone())), word: sha256_hex(b"word") };
    let mut host = host(&base);
    install(&mut host, 1, derived_sheet(Some(lineage.clone())));
    let installed = host.installed(ViewId(1)).expect("installed");
    let subject_label = &installed.validated.subjects[0];
    assert_eq!(subject_label.kind, SubjectKind::Derived);
    assert_eq!(subject_label.digest, subject.digest);
    assert_eq!(subject_label.lineage, Some(lineage));
    assert_eq!(host.adopted(), &base, "a derived plan never becomes session state");
}

#[test]
fn ct_rpi_05_a_derived_kernel_state_keeps_its_subject() {
    let (_, base) = solved(12);
    let derived = derived();
    let subject = DerivedRef { digest: derived.state.digest().to_owned(), level: Level::Labels, identity: identity() };
    let mut publication = single(&base, vec![], vec![]);
    let set = full_set();
    publication.plans[0].subject = StateRef::Derived(subject.clone());
    publication.plans[0].derivation =
        Some(Lineage::Derivation { base: Some(StateRef::Snapshot(base.clone())), word: derived.word.clone() });
    publication.plans[0].primitives.push(Primitive::Retained(retained(
        &derived.state,
        &set,
        &set,
        vec!["sticker".to_owned()],
    )));
    let validated = validate(&publication, &context()).expect("the derived labels bind the derived subject");
    assert_eq!(validated.subjects[0].kind, SubjectKind::Derived);
    assert_eq!(validated.subjects[0].digest, derived.state.digest());
    assert_ne!(validated.subjects[0].digest, base.digest);
}

#[test]
fn ct_rpi_08_f01_retained_stickers() {
    let (snapshot, base) = solved(12);
    let set = full_set();
    let good =
        single(&base, vec![Primitive::Retained(retained(&snapshot, &set, &set, vec!["sticker".to_owned()]))], vec![]);
    let validated = validate(&good, &context()).expect("all 259800 labels of the subject");
    assert_eq!(validated.subjects[0].kind, SubjectKind::Session);

    let other = derived().state;
    let wrong = single(&base, vec![Primitive::Retained(retained(&other, &set, &set, Vec::new()))], vec![]);
    invalid(validate(&wrong, &context()), "the labels do not bind the plan's subject");
}

#[test]
fn ct_rpi_08_f02_visibility_and_interaction_sets() {
    let (snapshot, base) = solved(12);
    let mut visibility = full_set();
    visibility[..1000].fill(0);
    let filtered =
        single(&base, vec![Primitive::Retained(retained(&snapshot, &visibility, &full_set(), Vec::new()))], vec![]);
    validate(&filtered, &context()).expect("a filter keeps the full subject");

    let mut retained_input = retained(&snapshot, &full_set(), &full_set(), Vec::new());
    retained_input.interaction = Array::u8s(vec![SLOTS as u64 - 1000], &vec![1; SLOTS - 1000]);
    let shrunk = single(&base, vec![Primitive::Retained(retained_input)], vec![]);
    invalid(validate(&shrunk, &context()), "interaction: shape");
}

#[test]
fn ct_rpi_08_f03_certified_transform() {
    let (_, base) = solved(12);
    let effect = kernel().effect(&kernel().word(&[1]).unwrap()).unwrap();
    let plane = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]];
    let swept = transform(geometry(), &effect, 0, Some(plane), 1.2566371, (12, 13));
    assert!(validate::swept_preview(&swept));
    assert_eq!(swept.moving.desc.shape, vec![effect.moved_pieces as u64]);
    validate(&single(&base, vec![Primitive::Transform(swept.clone())], vec![]), &context()).expect("certified");

    let half_turn = transform(geometry(), &effect, 0, None, std::f32::consts::PI, (12, 13));
    assert!(!validate::swept_preview(&half_turn), "no swept preview without the engine's plane");
    validate(&single(&base, vec![Primitive::Transform(half_turn)], vec![]), &context()).expect("no plane is guessed");

    let mut dropped = swept;
    let moving = dropped.moving.as_u32();
    dropped.moving = Array::u32s(vec![moving.len() as u64 - 1], &moving[1..]);
    let publication = single(&base, vec![Primitive::Transform(dropped)], vec![]);
    invalid(validate(&publication, &context()), "does not match its exact-set digest");
}

#[test]
fn ct_rpi_08_f04_built_in_projection_chain() {
    let base = revision(12);
    let mut good = sheet(1, &base, 1);
    good.plans[0].charts.push(chart(1, &[("rotate", 6), ("stereographic", 0), ("camera3", 9)]));
    good.plans[0].primitives.push(Primitive::ChartStage(StageInput { chart: ChartId(1) }));
    validate(&good, &context()).expect("admitted stages");

    let mut wrong_params = good.clone();
    wrong_params.plans[0].charts[0].stages[0].params.pop();
    invalid(validate(&wrong_params, &context()), "stage rotate takes 6 parameters, not 5");
}

#[test]
fn ct_rpi_08_f05_f26_user_stages_are_unsupported() {
    let base = revision(12);
    for (kind, row) in [(StageKind::Vertex, "F05"), (StageKind::Fragment, "F26")] {
        let mut publication = sheet(1, &base, 1);
        publication.plans[0].primitives.push(Primitive::UserStage(UserStageInput {
            kind,
            digest: sha256_hex(b"stage"),
            compile_record: sha256_hex(b"record"),
        }));
        let error = validate(&publication, &context()).expect_err("user stages are unsupported");
        assert_eq!(error.code, codes::UNSUPPORTED);
        assert_eq!(error.context["feature"], row);
    }
}

#[test]
fn ct_rpi_08_f06_object_sheet() {
    let base = revision(12);
    validate(&sheet(1, &base, 3), &context()).expect("a sheet");
    let mut short = sheet(1, &base, 3);
    let Primitive::Layout(layout) = &mut short.plans[0].primitives[0] else { unreachable!() };
    layout.positions = Array::f32s(vec![2, 2], &[0.0, 0.0, 1.0, 0.0]);
    invalid(validate(&short, &context()), "another number of positions than instances");
}

#[test]
fn ct_rpi_08_f07_atlas_copies() {
    let base = revision(12);
    let instances = vec![instance(1, 1, ObjectId::Sticker(5), 0), instance(2, 1, ObjectId::Sticker(5), 0)];
    let mut layout = sheet_layout(&[1, 2]);
    layout.kind = LayoutKind::Atlas { membership: vec![ChartId(1), ChartId(2)] };
    let mut good = single(&base, vec![Primitive::Layout(layout)], instances);
    good.plans[0].charts = vec![chart(1, &[("rotate", 6)]), chart(2, &[("rotate", 6)])];
    let validated = validate(&good, &context()).expect("two copies of one sticker");
    assert_eq!(validated.counts["instances"], 2);
    assert_eq!(validated.counts["charts"], 2);

    let mut missing_chart = good;
    missing_chart.plans[0].charts.pop();
    invalid(validate(&missing_chart, &context()), "an atlas names the chart of every occurrence");
}

#[test]
fn ct_rpi_08_f08_tree_layout() {
    let base = revision(12);
    let instances = (1..=3).map(|id| instance(id, 1, ObjectId::Cell(id as u32), 0)).collect::<Vec<_>>();
    let mut layout = sheet_layout(&[1, 2, 3]);
    layout.kind = LayoutKind::Tree { roots: vec![0], parent_child: Array::u32s(vec![2, 2], &[0, 1, 0, 2]) };
    let good = single(&base, vec![Primitive::Layout(layout.clone())], instances.clone());
    assert_eq!(validate(&good, &context()).expect("a tree").counts["layout_indices"], 4);

    layout.kind = LayoutKind::Tree { roots: vec![3], parent_child: Array::u32s(vec![2, 2], &[0, 1, 0, 2]) };
    let bad = single(&base, vec![Primitive::Layout(layout)], instances);
    invalid(validate(&bad, &context()), "a tree needs root indices inside its layout");
}

#[test]
fn ct_rpi_08_f09_cell_complex() {
    let base = revision(12);
    let instances = (1..=3).map(|id| instance(id, 1, ObjectId::Cell(id as u32), 0)).collect::<Vec<_>>();
    let complex = |dims: &[u32], boundary: &[u32]| {
        let mut layout = sheet_layout(&[1, 2, 3]);
        layout.kind = LayoutKind::Complex(IncidenceInput {
            dims: Array::u32s(vec![3], dims),
            offsets: Array::u32s(vec![4], &[0, 0, 0, boundary.len() as u32]),
            boundary: Array::u32s(vec![boundary.len() as u64], boundary),
            orientation: Array::i32s(vec![boundary.len() as u64], &vec![1; boundary.len()]),
        });
        single(&base, vec![Primitive::Layout(layout)], instances.clone())
    };
    validate(&complex(&[0, 0, 1], &[0, 1]), &context()).expect("an edge on two vertices");
    invalid(validate(&complex(&[0, 1, 1], &[0, 1]), &context()), "does not name a lower-dimensional cell");
    assert_eq!(code(validate(&complex(&[0, 0, 5], &[0, 1]), &context())), codes::UNSUPPORTED, "a cell beyond d");
}

#[test]
fn ct_rpi_08_f10_state_graph() {
    let base = revision(12);
    let instances = vec![instance(1, 1, ObjectId::Orbit(0), 0), instance(2, 1, ObjectId::Orbit(0), 0)];
    let other = DerivedRef { digest: sha256_hex(b"after w"), level: Level::Labels, identity: identity() };
    let graph = |transitions: Vec<String>| {
        let mut layout = sheet_layout(&[1, 2]);
        layout.kind = LayoutKind::Graph(GraphInput {
            subjects: vec![StateRef::Snapshot(base.clone()), StateRef::Derived(other.clone())],
            edges: Array::u32s(vec![1, 2], &[0, 1]),
            transitions,
        });
        single(&base, vec![Primitive::Layout(layout)], instances.clone())
    };
    validate(&graph(vec![sha256_hex(b"w")]), &context()).expect("each node keeps its subject");
    invalid(validate(&graph(Vec::new()), &context()), "every directed edge carries one transition reference");
}

fn overlay(
    objects: Vec<ObjectId>,
    kind: OverlayKind,
    coverage: Coverage,
    predicate: Option<Predicate>,
) -> OverlayInput {
    OverlayInput {
        kind,
        set_digest: object_set_digest(&objects),
        objects,
        coverage,
        predicate,
        positions: None,
        lines: None,
        triangles: None,
        tokens: vec!["highlight".to_owned()],
    }
}

#[test]
fn ct_rpi_08_f11_f12_f13_section_slab_clip() {
    let base = revision(12);
    let objects = vec![ObjectId::Piece(3), ObjectId::Piece(8)];
    let rows = [
        (Predicate::Section("x0 = 0".to_owned()), "F11"),
        (Predicate::Slab("0 <= x0 <= 1/2".to_owned()), "F12"),
        (Predicate::Clip("x3 >= 0".to_owned()), "F13"),
    ];
    for (predicate, row) in rows {
        let good = overlay(objects.clone(), OverlayKind::Set, Coverage::Complete, Some(predicate.clone()));
        assert_eq!(Primitive::SetOverlay(good.clone()).feature(), row);
        validate(&single(&base, vec![Primitive::SetOverlay(good)], vec![]), &context()).expect(row);
        let empty = match predicate {
            Predicate::Section(_) => Predicate::Section(String::new()),
            Predicate::Slab(_) => Predicate::Slab(String::new()),
            Predicate::Clip(_) => Predicate::Clip(String::new()),
        };
        let bad = overlay(objects.clone(), OverlayKind::Set, Coverage::Complete, Some(empty));
        let publication = single(&base, vec![Primitive::SetOverlay(bad)], vec![]);
        invalid(validate(&publication, &context()), "names its exact predicate");
    }
}

#[test]
fn ct_rpi_08_f14_sets_relations_and_missing_data() {
    let base = revision(12);
    let objects = vec![ObjectId::Sticker(4), ObjectId::Piece(2)];
    let mut sorted = objects.clone();
    sorted.sort();
    let relation = OverlayKind::Relation { claim: Some(Key("fixture.proved".to_owned())) };
    for (kind, coverage, objects) in [
        (OverlayKind::Set, Coverage::Complete, sorted.clone()),
        (relation, Coverage::Complete, sorted.clone()),
        (OverlayKind::Set, Coverage::Partial { missing: vec!["orbit 3".to_owned()] }, sorted.clone()),
        (OverlayKind::Set, Coverage::Undefined("off_lattice".to_owned()), Vec::new()),
    ] {
        let publication = single(&base, vec![Primitive::SetOverlay(overlay(objects, kind, coverage, None))], vec![]);
        validate(&publication, &context()).expect("explicit coverage");
    }
    let claimless = overlay(sorted.clone(), OverlayKind::Relation { claim: None }, Coverage::Complete, None);
    let publication = single(&base, vec![Primitive::SetOverlay(claimless)], vec![]);
    invalid(validate(&publication, &context()), "a relation overlay carries its claim key");
    let undefined = overlay(sorted, OverlayKind::Set, Coverage::Undefined("off_lattice".to_owned()), None);
    let publication = single(&base, vec![Primitive::SetOverlay(undefined)], vec![]);
    invalid(validate(&publication, &context()), "an undefined result carries no objects");
}

#[test]
fn ct_rpi_08_f15_token_colours() {
    let base = revision(12);
    let table = produce::token_table();
    let tokens =
        |name: &str| TokenInput { table: table.digest.clone(), names: vec![("sticker".to_owned(), name.to_owned())] };
    validate(&single(&base, vec![Primitive::Tokens(tokens("dim"))], vec![]), &context()).expect("resolves");
    let publication = single(&base, vec![Primitive::Tokens(tokens("ultraviolet"))], vec![]);
    invalid(validate(&publication, &context()), "token name ultraviolet does not resolve");
}

#[test]
fn ct_rpi_08_f16_derived_state_display() {
    let (_, base) = solved(12);
    let derived = derived();
    let subject = DerivedRef { digest: derived.state.digest().to_owned(), level: Level::Labels, identity: identity() };
    let set = full_set();
    let display = |lineage: Option<Lineage>| DerivedDisplay {
        subject: subject.clone(),
        lineage,
        reachability: "witnessed".to_owned(),
        primitives: vec![Primitive::Retained(retained(&derived.state, &set, &set, Vec::new()))],
    };
    let lineage = Lineage::Derivation { base: Some(StateRef::Snapshot(base.clone())), word: derived.word.clone() };
    validate(&single(&base, vec![Primitive::Derived(display(Some(lineage)))], vec![]), &context()).expect("derived");
    let publication = single(&base, vec![Primitive::Derived(display(None))], vec![]);
    invalid(validate(&publication, &context()), "only with its derivation");
}

#[test]
fn ct_rpi_08_f17_preview_only_from_the_engine() {
    let base = revision(12);
    let preview = PreviewDisplay {
        preview: PreviewId(1),
        base: base.clone(),
        literal: sha256_hex(b"literal"),
        effect: sha256_hex(b"effect"),
        primitives: Vec::new(),
    };
    let mut host = host(&base);
    host.show_preview(ViewId(1), Some(preview.clone())).expect("the engine path");
    let publication = single(&base, vec![Primitive::Preview(preview)], vec![]);
    invalid(validate(&publication, &context()), "a program cannot supply or label preview data");
}

#[test]
fn ct_rpi_08_posed_and_certificate_rows_are_unsupported_on_full() {
    let base = revision(12);
    let posed = PosedInput {
        pose_index: Array::i32s(vec![1], &[0]),
        poses: Array::f32s(vec![1], &[0.0]),
        pose_lattice: Array::i32s(vec![1], &[0]),
    };
    let error = validate(&single(&base, vec![Primitive::Posed(posed)], vec![]), &context()).expect_err("M5");
    assert_eq!((error.code, error.context["feature"].as_str()), (codes::UNSUPPORTED, "J02"));
    let cert = CertOverlay {
        grip: 0,
        cap: 0,
        kind: CertKind::CapMapWitness,
        pieces: vec![7],
        set_digest: piece_set_digest(&[7]),
        pairs: Vec::new(),
        survey: Vec::new(),
        cut: None,
    };
    let error = validate(&single(&base, vec![Primitive::CertOverlay(cert)], vec![]), &context()).expect_err("M5");
    assert_eq!((error.code, error.context["feature"].as_str()), (codes::UNSUPPORTED, "J04"));
}

fn node(base: &RevisionRef, kind: NodeKind, content: Vec<Content>, claims: Vec<Key>) -> ViewNode {
    ViewNode {
        node: NodeId(1),
        base: base.clone(),
        kind,
        content,
        plans: Vec::new(),
        children: Vec::new(),
        source: Some(link()),
        commands: Vec::new(),
        claims,
    }
}

fn with_node(base: &RevisionRef, node: ViewNode) -> Publication {
    let mut publication = sheet(1, base, 1);
    publication.nodes.push(node);
    publication
}

#[test]
fn ct_rpi_27_words_only_through_word_with_their_phrase() {
    let base = revision(12);
    let key = Key("fixture.proved".to_owned());
    let content = vec![
        Content::State { row: "progress".to_owned(), digest: sha256_hex(b"rows") },
        Content::Word(key.clone()),
        Content::Text("proved".to_owned()),
    ];
    let good = with_node(&base, node(&base, NodeKind::Relation, content, vec![key.clone()]));
    let validated = validate(&good, &context()).expect("state, a word on a key with a record, and program text");
    let shown = &validated.nodes[0].content;
    assert_eq!(shown[0], Shown::State { row: "progress".to_owned(), digest: sha256_hex(b"rows") });
    assert_eq!(
        shown[1],
        Shown::Word {
            key: key.clone(),
            word: "proved".to_owned(),
            scope: "proved on D[k], for reachable inputs".to_owned()
        }
    );
    assert_eq!(shown[2], Shown::Program("proved".to_owned()), "program text carries no word");

    let unclaimed = with_node(&base, node(&base, NodeKind::Object, vec![Content::Word(key.clone())], Vec::new()));
    invalid(validate(&unclaimed, &context()), "needs that key in the node's claims");
    let unknown = Key("fixture.unrecorded".to_owned());
    let unrecorded =
        with_node(&base, node(&base, NodeKind::Object, vec![Content::Word(unknown.clone())], vec![unknown]));
    invalid(validate(&unrecorded, &context()), "which has no record");
}

#[test]
fn ct_rpi_27_relation_node_with_empty_claims_is_invalid() {
    let base = revision(12);
    let state = vec![Content::State { row: "progress".to_owned(), digest: sha256_hex(b"rows") }];
    let empty = with_node(&base, node(&base, NodeKind::Relation, state.clone(), Vec::new()));
    let (refusal, failure) = both_sides(empty, context());
    assert_eq!((refusal.code, failure.code), (codes::PLAN_INVALID, codes::PLAN_INVALID));
    assert!(refusal.summary.contains("states no claim key"), "{}", refusal.summary);
    let object = with_node(&base, node(&base, NodeKind::Object, state, Vec::new()));
    validate(&object, &context()).expect("state data needs no claim");
    let mut host = host(&base);
    install(&mut host, 1, object);
    assert!(matches!(status(&host, 1), Status::Completed(_)));
}
