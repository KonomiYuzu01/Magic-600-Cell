//! Picking and stamps (part K): CT-RPI-09 to CT-RPI-12, and CT-RM-04 with RM.30's source vectors and RM.31's derived
//! subject. A pick first obtains an instance ID and then resolves it through the installed source map (RPI.22); no
//! pixel, position, token colour or chart inverse enters a reply. Every result is a source or synthetic fixture
//! outcome, not a backend run.

mod common;

use std::collections::BTreeMap;

use c600_digest::sha256_hex;
use c600_display::array::Array;
use c600_display::codes;
use c600_display::host::{Host, MockBackend};
use c600_display::pick::{LabelProjection, Partial, PickMode, PickReply, Presented, Projection, Requested, Resolution};
use c600_display::plan::*;
use c600_display::produce::{self, full_set, retained};
use c600_display::receipt::Status;
use c600_display::source::{self, Followed, HostRange};
use c600_display::validate::{validate, BoundArray, Context, EngineBundle, SubjectKind, Support};

use common::*;

const V1: ViewId = ViewId(1);
const V2: ViewId = ViewId(2);

/// A reply for `instance` under the view's current binding.
fn reply(host: &Host<MockBackend>, view: ViewId, instance: u64) -> PickReply {
    PickReply { binding: host.pick_binding(view).expect("a current binding"), instance: InstanceId(instance) }
}

/// An explicit resolution of a current reply that must be total.
fn total(host: &Host<MockBackend>, reply: &PickReply, requested: Requested) -> Resolution {
    match host.resolve_pick(reply, PickMode::Explicit, requested, &Unasked) {
        Ok(Some(Partial::Total(resolution))) => resolution,
        other => panic!("expected a total resolution, got {other:?}"),
    }
}

/// Asserts that an explicit resolution of a current reply has no target at that level.
fn no_target(host: &Host<MockBackend>, reply: &PickReply, requested: Requested) {
    let result = host.resolve_pick(reply, PickMode::Explicit, requested, &Unasked);
    assert!(matches!(result, Ok(Some(Partial::Undefined(_)))), "{requested:?}: {result:?}");
}

/// The passive pick of a stale reply is dropped quietly; the explicit one names the changed field (RPI.25).
fn stale_field(host: &Host<MockBackend>, reply: &PickReply) -> String {
    assert_eq!(host.resolve_pick(reply, PickMode::Passive, Requested::Occurrence, &Unasked), Ok(None));
    let error = host.resolve_pick(reply, PickMode::Explicit, Requested::Occurrence, &Unasked).expect_err("stale");
    assert_eq!(error.code, codes::STALE_PICK);
    error.context["field"].clone()
}

fn range(start: u64, end: u64) -> SourceLink {
    SourceLink::Range { digest: SOURCE_DIGEST.to_owned(), start, end }
}

/// The piece of sticker `home`: the piece of its home slot.
fn piece_of(home: u32) -> u32 {
    geometry().slot_piece.as_u32()[home as usize]
}

/// A one-plan publication at `base` whose instances `1..` show `objects` on a sheet.
fn objects(base: &RevisionRef, objects: &[ObjectId]) -> Publication {
    let ids: Vec<u64> = (1..=objects.len() as u64).collect();
    let instances = ids.iter().zip(objects).map(|(&id, object)| instance(id, 1, object.clone(), 0)).collect();
    publication(1, base, vec![plan(1, base, vec![Primitive::Layout(sheet_layout(&ids))], instances)])
}

/// An unretained module's source: the entry names its producing digest and why it is missing (RM.10).
fn missing() -> SourceLink {
    SourceLink::Missing { digest: sha256_hex(b"an unretained module"), reason: "not retained".to_owned() }
}

/// Plan 1 is an atlas of two charts with two copies of sticker 5; plan 2 is a sheet with a third copy, the copy's
/// piece, a cell, an orbit and a grip. Each instance has its own range in `a😀b`, and the grip's source is missing.
fn duplicates(base: &RevisionRef) -> Publication {
    let mut copy = instance(2, 1, ObjectId::Sticker(5), 0);
    copy.piece = Some(piece_of(5));
    let mut atlas = sheet_layout(&[1, 2]);
    atlas.kind = LayoutKind::Atlas { membership: vec![ChartId(1), ChartId(2)] };
    let (mut first, mut first_map) =
        plan(1, base, vec![Primitive::Layout(atlas)], vec![instance(1, 1, ObjectId::Sticker(5), 0), copy]);
    first.charts = vec![chart(1, &[("rotate", 6)]), chart(2, &[("rotate", 6), ("perspective", 1)])];
    for (id, link) in [(1, range(0, 1)), (2, range(1, 5))] {
        first_map.entries.get_mut(&InstanceId(id)).expect("an entry").source = link;
    }

    let shown = [
        (3, ObjectId::Sticker(5)),
        (4, ObjectId::Piece(piece_of(5))),
        (5, ObjectId::Cell(7)),
        (6, ObjectId::Orbit(3)),
        (7, ObjectId::Grip(11)),
    ];
    let instances = shown.iter().map(|(id, object)| instance(*id, 2, object.clone(), 0)).collect();
    let (second, mut second_map) = plan(2, base, vec![Primitive::Layout(sheet_layout(&[3, 4, 5, 6, 7]))], instances);
    for (id, link) in [(5, range(0, 6)), (6, range(5, 5)), (7, missing())] {
        second_map.entries.get_mut(&InstanceId(id)).expect("an entry").source = link;
    }
    second_map.sources.push(missing().digest().to_owned());
    publication(1, base, vec![(first, first_map), (second, second_map)])
}

#[test]
fn ct_rpi_09_copies_resolve_to_their_canonical_object_and_producing_source() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, duplicates(&base));

    let piece = piece_of(5);
    let expected = [
        (1, 1, ObjectId::Sticker(5), "a"),
        (2, 1, ObjectId::Sticker(5), "\u{1F600}"),
        (3, 2, ObjectId::Sticker(5), "b"),
        (4, 2, ObjectId::Piece(piece), "b"),
        (5, 2, ObjectId::Cell(7), SOURCE),
        (6, 2, ObjectId::Orbit(3), ""),
    ];
    for (id, plan, object, text) in expected {
        let resolution = total(&host, &reply(&host, V1, id), Requested::Occurrence);
        assert_eq!((resolution.instance, resolution.plan), (InstanceId(id), PlanId(plan)));
        assert_eq!((&resolution.target, &resolution.occurrence), (&object, &object));
        assert_eq!(resolution.subject, StateRef::Snapshot(base.clone()));
        assert_eq!(resolution.lineage, None);
        match host.follow(V1, &resolution.source).expect("the producing version opens") {
            Followed::Range { digest, text: selected, .. } => {
                assert_eq!((digest.as_str(), selected.as_str()), (SOURCE_DIGEST, text))
            }
            other => panic!("instance {id}: {other:?}"),
        }
    }
    let grip = total(&host, &reply(&host, V1, 7), Requested::Grip);
    assert_eq!(grip.target, ObjectId::Grip(11), "missing source does not stop object resolution");
    let Followed::Missing { digest, reason } = host.follow(V1, &grip.source).expect("an explicit missing source")
    else {
        panic!("no range is invented")
    };
    assert_eq!((digest, reason), (missing().digest().to_owned(), "not retained".to_owned()));

    let copies: Vec<Resolution> = (1..=3).map(|id| total(&host, &reply(&host, V1, id), Requested::Sticker)).collect();
    assert!(copies.iter().all(|r| r.target == ObjectId::Sticker(5)), "copies share the canonical target");
    let ids: Vec<InstanceId> = copies.iter().map(|r| r.instance).collect();
    assert_eq!(ids, [InstanceId(1), InstanceId(2), InstanceId(3)], "each copy keeps its own instance ID");

    no_target(&host, &reply(&host, V1, 1), Requested::Piece);
    assert_eq!(total(&host, &reply(&host, V1, 2), Requested::Piece).target, ObjectId::Piece(piece));
    assert_eq!(total(&host, &reply(&host, V1, 4), Requested::Piece).target, ObjectId::Piece(piece));
    no_target(&host, &reply(&host, V1, 4), Requested::Sticker);
    assert_eq!(total(&host, &reply(&host, V1, 5), Requested::Cell).target, ObjectId::Cell(7));
    no_target(&host, &reply(&host, V1, 5), Requested::Orbit);
    assert_eq!(total(&host, &reply(&host, V1, 6), Requested::Orbit).target, ObjectId::Orbit(3));
}

/// The fixture context with one more declared stage, `collapse`: it sends every point to one point, so its chart is
/// many-to-one with no inverse.
fn collapsing() -> Context {
    let mut ctx = context();
    match ctx.support.features.get_mut("F04") {
        Some(Support::Supported { stages, .. }) => stages.push(produce::stage_def("collapse", 0)),
        other => panic!("F04 is supported in the fixture: {other:?}"),
    }
    ctx
}

/// A planted inverse that throws. Resolution accepts no chart inverse at all; the only callback it can reach is the
/// slot projection, so the inverse is planted there.
struct PlantedInverse;

impl Projection for PlantedInverse {
    fn slot(&self, _: &StateRef, object: &ObjectId) -> Partial<u32> {
        panic!("the planted inverse was called for {object:?}")
    }
}

#[test]
fn ct_rpi_10_a_many_to_one_chart_is_picked_by_instance_id() {
    let base = revision(12);
    let shown = [ObjectId::Sticker(3), ObjectId::Sticker(4), ObjectId::Piece(9), ObjectId::Cell(2)];
    let mut collapsed = objects(&base, &shown);
    let plan = &mut collapsed.plans[0];
    plan.charts = vec![chart(1, &[("collapse", 0)])];
    plan.primitives.push(Primitive::ChartStage(StageInput { chart: ChartId(1) }));
    let Primitive::Layout(layout) = &mut plan.primitives[0] else { unreachable!() };
    layout.positions = Array::f32s(vec![4, 2], &[0.0; 8]);
    layout.kind = LayoutKind::Atlas { membership: vec![ChartId(1); 4] };
    let mut host = host_with(&base, collapsing());
    install(&mut host, 1, collapsed);

    let levels = [
        Requested::Occurrence,
        Requested::Sticker,
        Requested::Piece,
        Requested::Cell,
        Requested::Grip,
        Requested::Cap,
        Requested::Orbit,
    ];
    for (id, object) in (1..).zip(&shown) {
        let reply = reply(&host, V1, id);
        for requested in levels {
            host.resolve_pick(&reply, PickMode::Explicit, requested, &PlantedInverse).expect("a current reply");
        }
        let resolution = host.resolve_pick(&reply, PickMode::Explicit, Requested::Occurrence, &PlantedInverse);
        assert!(matches!(&resolution, Ok(Some(Partial::Total(r))) if r.target == *object), "{resolution:?}");
    }
    let unknown = PickReply { binding: host.pick_binding(V1).expect("current"), instance: InstanceId(99) };
    let error =
        host.resolve_pick(&unknown, PickMode::Explicit, Requested::Occurrence, &PlantedInverse).expect_err("no ID");
    assert_eq!(error.code, codes::PLAN_INVALID, "an ID outside the map has no positional fallback");
}

/// The same instance IDs 1..=n on cells: a replaced map that reuses IDs.
fn cells(base: &RevisionRef, n: u32) -> Publication {
    objects(base, &(1..=n).map(ObjectId::Cell).collect::<Vec<_>>())
}

#[test]
fn ct_rpi_11_delayed_picks_are_discarded_and_unrelated_edits_keep_a_query() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 3));
    let first = reply(&host, V1, 2);

    host.open_view(V2).expect("view 2 opens");
    install(&mut host, 2, sheet_from(2, &base, 1, 5));
    install(&mut host, 3, sheet_from(2, &base, 4, 2));
    host.set_dependency("input:unrelated", "edited");
    assert_eq!(total(&host, &first, Requested::Occurrence).target, ObjectId::Sticker(2), "unrelated edits keep it");

    install(&mut host, 4, cells(&base, 3));
    assert_eq!(stale_field(&host, &first), "publication_generation");
    assert_eq!(total(&host, &reply(&host, V1, 2), Requested::Occurrence).target, ObjectId::Cell(2));

    let changes: [(&str, fn(&mut Host<MockBackend>)); 4] = [
        ("camera", |h| h.set_camera(V1)),
        ("viewport", |h| h.set_viewport(V1, (1024, 768))),
        ("visibility", |h| h.set_visibility(V1)),
        ("interaction", |h| h.set_interaction(V1)),
    ];
    for (field, change) in changes {
        let delayed = reply(&host, V1, 1);
        change(&mut host);
        assert_eq!(stale_field(&host, &delayed), field);
        assert!(host.resolve_pick(&reply(&host, V1, 1), PickMode::Explicit, Requested::Occurrence, &Unasked).is_ok());
    }

    let delayed = reply(&host, V1, 1);
    host.close_view(V1);
    host.open_view(V1).expect("view 1 reopens");
    assert_eq!(stale_field(&host, &delayed), "view_generation");
    install(&mut host, 5, cells(&base, 3));
    assert_eq!(stale_field(&host, &delayed), "surface_generation", "RM.24's order names the reopened surface first");

    let delayed = reply(&host, V1, 1);
    host.backend.lose_device();
    host.pump();
    host.recover().expect("recovers");
    assert_eq!(stale_field(&host, &delayed), "device_epoch");

    let delayed = reply(&host, V1, 1);
    host.set_dependency("state", "13");
    assert_eq!(stale_field(&host, &delayed), "dependency state");
}

#[test]
fn ct_rpi_12_explicit_stale_explains_passive_stale_is_quiet_and_off_lattice_is_partial() {
    let solved = kernel().solved();
    let base = revision_with(12, solved.digest());
    let piece = piece_of(1);
    let mut host = host(&base);
    install(&mut host, 1, objects(&base, &[ObjectId::Sticker(1), ObjectId::Piece(piece)]));
    let sticker = reply(&host, V1, 1);
    let whole = reply(&host, V1, 2);
    let labels = LabelProjection::new(solved.labels(), solved.digest());

    let slot = host.resolve_pick(&sticker, PickMode::Explicit, Requested::Slot, &labels).expect("current");
    assert!(matches!(&slot, Some(Partial::Total(r)) if r.target == ObjectId::Slot(1)), "{slot:?}");
    let Some(Partial::Partial { covered, missing }) =
        host.resolve_pick(&whole, PickMode::Explicit, Requested::Slot, &labels).expect("current")
    else {
        panic!("a piece has no single slot")
    };
    assert_eq!(covered.target, ObjectId::Piece(piece));
    assert_eq!(missing, ["slot: a piece of 600-cell-Full has no single slot"]);
    let Some(Partial::Partial { covered, missing }) = host
        .resolve_pick(&sticker, PickMode::Explicit, Requested::Slot, &OffLattice)
        .expect("a partial answer, not stale")
    else {
        panic!("off the lattice the answer is partial")
    };
    assert_eq!((covered.target, missing), (ObjectId::Sticker(1), vec!["slot: off_lattice".to_owned()]));
    let passive = host.resolve_pick(&sticker, PickMode::Passive, Requested::Slot, &OffLattice).expect("current");
    assert!(matches!(passive, Some(Partial::Partial { .. })), "a current passive pick keeps its partial answer");

    host.set_camera(V1);
    for requested in [Requested::Occurrence, Requested::Slot] {
        let error = host.resolve_pick(&sticker, PickMode::Explicit, requested, &Unasked).expect_err("stale");
        assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "camera"));
        assert_eq!(error.summary, "the pick is stale: camera changed", "a reason for a visible explanation");
        assert_eq!(host.resolve_pick(&sticker, PickMode::Passive, requested, &Unasked), Ok(None), "quiet");
    }
}

/// Engine bundles of a Full state at `revision`: the three arrays, each bound to `bound`.
fn bundle(state: &c600_kernel::Snapshot, revision: &RevisionRef, bound: [u64; 3]) -> EngineBundle {
    let set = full_set();
    let input = retained(state, &set, &set, Vec::new());
    let arrays = [("labels", input.labels), ("visibility", input.visibility), ("interaction", input.interaction)];
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
fn ct_rm_04_a_derived_subject_keeps_its_label_and_comparison_base() {
    let solved = kernel().solved();
    let base = revision_with(12, solved.digest());
    let derived = derive(&[1]);
    let labels = derived.state.labels();
    let moved = (0..labels.len()).find(|&slot| labels[slot] as usize != slot).expect("the word moves a sticker");
    let home = labels[moved] as u32;
    let subject = DerivedRef { digest: derived.state.digest().to_owned(), level: Level::Labels, identity: identity() };
    let lineage = Lineage::Derivation { base: Some(StateRef::Snapshot(base.clone())), word: derived.word.clone() };
    let mut publication = objects(&base, &[ObjectId::Sticker(home)]);
    let plan = &mut publication.plans[0];
    plan.subject = StateRef::Derived(subject.clone());
    plan.derivation = Some(lineage.clone());
    plan.primitives.push(Primitive::Retained(retained(&derived.state, &full_set(), &full_set(), Vec::new())));
    let mut host = host(&base);
    install(&mut host, 1, publication.clone());

    let stamp = host.frame(V1).expect("a frame");
    let label = &stamp.subjects[0];
    assert_eq!(
        (label.kind, label.digest.as_str(), label.comparison_base),
        (SubjectKind::Derived, subject.digest.as_str(), 12)
    );
    assert_eq!(label.lineage, Some(lineage.clone()));
    let presented = host.read_presented();
    assert_eq!(presented.len(), 1);
    assert_eq!(presented[0].presented, Presented::NoPreview);
    assert_eq!(host.presented_preview(), None);
    assert_eq!(host.adopted(), &base, "the session subject stays revision 12; D gets no session revision");

    let pick = reply(&host, V1, 1);
    let resolution = total(&host, &pick, Requested::Occurrence);
    assert_eq!((resolution.subject, resolution.lineage), (StateRef::Derived(subject.clone()), Some(lineage.clone())));
    let projection = LabelProjection::new(labels, derived.state.digest());
    let slot = host.resolve_pick(&pick, PickMode::Explicit, Requested::Slot, &projection).expect("current");
    assert!(matches!(&slot, Some(Partial::Total(r)) if r.target == ObjectId::Slot(moved as u32)), "{slot:?}");

    let next = derive(&[2]);
    let thirteen = revision_with(13, next.state.digest());
    let mixed = host.adopt(&bundle(&next.state, &thirteen, [13, 12, 13])).expect_err("a mixed bundle");
    assert_eq!(mixed.code, codes::BUNDLE_MISMATCH);
    assert_eq!(host.adopted(), &base, "no mixed bundle is adopted");
    assert_eq!(host.adopt(&bundle(&next.state, &thirteen, [13; 3])), Ok(thirteen));
    assert!(host.stale(V1), "a state-dependent D is stale at revision 13");
    let stamp = host.frame(V1).expect("a frame");
    assert_eq!((stamp.engine_revision, stamp.subjects[0].comparison_base), (13, 12));
    assert_eq!(stamp.subjects[0].kind, SubjectKind::Derived, "stale output is still labelled derived");
    assert_eq!(stale_field(&host, &pick), "dependency state");
    let error = refused(&host.publish(CommandId(2), publication));
    assert_eq!(error.code, codes::STALE_PLAN, "D cannot overlay revision 13");
    assert!(matches!(status(&host, 1), Status::Completed(_)));
}

#[test]
fn ct_rm_04_an_imported_value_keeps_its_lineage_and_comparison_base() {
    let base = revision(12);
    let subject = DerivedRef { digest: sha256_hex(b"an imported value"), level: Level::Labels, identity: identity() };
    let import = sha256_hex(b"the import record");
    let imported = |comparison_base: Option<RevisionRef>| {
        let mut publication = sheet(1, &base, 1);
        publication.plans[0].subject = StateRef::Derived(subject.clone());
        publication.plans[0].derivation = Some(Lineage::Imported { import: import.clone(), comparison_base });
        publication
    };
    let error = validate(&imported(None), &context()).expect_err("no comparison base");
    assert_eq!(error.code, codes::PLAN_INVALID);
    assert!(error.summary.contains("explicit comparison base"), "{}", error.summary);

    let lineage = Lineage::Imported { import: import.clone(), comparison_base: Some(base.clone()) };
    let mut host = host(&base);
    install(&mut host, 1, imported(Some(base.clone())));
    let label = &host.installed(V1).expect("installed").validated.subjects[0];
    assert_eq!((label.kind, label.comparison_base, &label.lineage), (SubjectKind::Derived, 12, &Some(lineage.clone())));
    let resolution = total(&host, &reply(&host, V1, 1), Requested::Occurrence);
    assert_eq!(resolution.lineage, Some(lineage), "no derivation or session ancestor is fabricated");
    assert_eq!(resolution.subject, StateRef::Derived(subject));
}

#[test]
fn ct_rm_04_source_ranges_survive_editing_and_bindings_are_checked_first() {
    assert_eq!(sha256_hex(SOURCE.as_bytes()), SOURCE_DIGEST);
    assert_eq!(sha256_hex(EDITED.as_bytes()), EDITED_DIGEST);
    let b = source::convert(SOURCE.as_bytes(), 5, 6).expect("[5,6) selects b");
    assert_eq!(b, HostRange { utf8: (5, 6), utf16: (3, 4), scalar: (2, 3) });
    let insertion = source::convert(SOURCE.as_bytes(), 5, 5).expect("an insertion");
    assert_eq!((insertion.utf16, insertion.scalar), ((3, 3), (2, 2)));
    let units = "a\u{1F600}\u{E9}\nz";
    let boundaries: Vec<Option<(u64, u64)>> = (0..=10).map(|utf8| source::boundary(units, utf8)).collect();
    let expected = [Some((0, 0)), Some((1, 1)), None, None, None, Some((3, 2)), None, Some((4, 3)), Some((5, 4))];
    assert_eq!(boundaries[..9], expected);
    assert_eq!(boundaries[9..], [Some((6, 5)), None]);

    let base = revision(12);
    let mut split = sheet(1, &base, 1);
    split.maps[0].entries.get_mut(&InstanceId(1)).expect("an entry").source = range(2, 5);
    let (refusal, failure) = both_sides(split, context());
    assert_eq!((refusal.code, failure.code), (codes::PLAN_INVALID, codes::PLAN_INVALID));
    assert!(refusal.summary.contains("splits a scalar"), "{}", refusal.summary);

    let mut host = host(&base);
    for command in 1..=7 {
        install(&mut host, command, sheet(1, &base, 1));
    }
    let seventh = reply(&host, V1, 1);
    assert_eq!(seventh.binding.publication_generation, 7);
    assert_eq!(host.pick_source(&seventh, SOURCE.as_bytes(), &Unasked), Ok(Some(b)));
    let error = host.pick_source(&seventh, EDITED.as_bytes(), &Unasked).expect_err("the source changed");
    assert_eq!(error.code, codes::STALE_SOURCE);
    assert_eq!(
        (error.context["producing"].as_str(), error.context["current"].as_str()),
        (SOURCE_DIGEST, EDITED_DIGEST)
    );

    install(&mut host, 8, sheet(1, &base, 1));
    for current in [SOURCE, EDITED] {
        let error = host.pick_source(&seventh, current.as_bytes(), &Unasked).expect_err("a replaced map");
        assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "publication_generation"));
    }

    let eighth = reply(&host, V1, 1);
    host.open_view(V2).expect("view 2 opens");
    let mut module = sheet(2, &base, 1);
    module.sources = BTreeMap::from([pinned(EDITED)]);
    module.maps[0].sources = vec![EDITED_DIGEST.to_owned()];
    module.maps[0].entries.get_mut(&InstanceId(1)).expect("an entry").source =
        SourceLink::Range { digest: EDITED_DIGEST.to_owned(), start: 5, end: 6 };
    install(&mut host, 9, module);
    host.set_dependency("input:module", &sha256_hex(b"a newly checked module"));
    let resolution = total(&host, &eighth, Requested::Occurrence);
    assert_eq!(
        host.follow(V1, &resolution.source),
        Ok(Followed::Range { digest: SOURCE_DIGEST.to_owned(), range: b, text: "b".to_owned() }),
        "the pinned a😀b still selects b"
    );
}
