//! CT-LB4-03, the 600-cell-Full half (LB4.4, INV-6, INV-8): several display instances map to their canonical IDs and
//! levels through chapter 10's source map, a chart inverse value cannot identify a pick, and a layout filter preserves
//! the mechanically analysed set. The pose levels wait for M5, so the whole definition does not run. Every result is a
//! source or synthetic fixture outcome over `c600-display`'s mock backend, not a backend run.

mod common;

use c600_display::codes;
use c600_display::host::{Host, MockBackend};
use c600_display::pick::{LabelProjection, Partial, PickMode, PickReply, Projection, Requested, Resolution};
use c600_display::plan::*;
use c600_display::validate::validate;
use c600_repr::layout::{linked, Linked, Representation};

use common::*;

/// A projection that must never be asked: resolving an occurrence, a sticker, a piece or a cell needs none, so a chart
/// inverse planted here would be called by nothing.
struct PlantedInverse;

impl Projection for PlantedInverse {
    fn slot(&self, _: &StateRef, object: &ObjectId) -> Partial<u32> {
        panic!("the planted inverse was called for {object:?}")
    }
}

fn total(host: &Host<MockBackend>, reply: &PickReply, requested: Requested, projection: &dyn Projection) -> Resolution {
    match host.resolve_pick(reply, PickMode::Explicit, requested, projection) {
        Ok(Some(Partial::Total(resolution))) => resolution,
        other => panic!("expected a total resolution of {requested:?}, got {other:?}"),
    }
}

fn undefined(host: &Host<MockBackend>, reply: &PickReply, requested: Requested) {
    let result = host.resolve_pick(reply, PickMode::Explicit, requested, &PlantedInverse);
    assert!(matches!(result, Ok(Some(Partial::Undefined(_)))), "{requested:?}: {result:?}");
}

/// The fixture: a real state of 600-cell-Full, one sticker the word moved, and two representations of it. Plan 1 is
/// an atlas of two charts whose four occurrences all sit at one sheet position; plan 2 is a sheet.
struct Fixture {
    base: RevisionRef,
    labels: Vec<i32>,
    digest: String,
    home: u32,
    slot: u32,
    piece: u32,
    other: u32,
    atlas: Representation,
    sheet: Representation,
}

fn fixture() -> Fixture {
    let derived = derive(&[1]);
    let labels = derived.state.labels().to_vec();
    let slot = (0..labels.len()).find(|&slot| labels[slot] as usize != slot).expect("the word moves a sticker");
    let home = labels[slot] as u32;
    let other = (0..labels.len()).find(|&slot| labels[slot] as usize == slot).expect("the word fixes a sticker") as u32;
    let piece = geometry().slot_piece.as_u32()[home as usize];
    let digest = derived.state.digest().to_owned();
    let base = revision_with(12, &digest);

    let mut as_piece = occurrence(2, ObjectId::Sticker(home), Some(2));
    as_piece.piece = Some(piece);
    as_piece.source = range(1, 2);
    let analysed = [
        ObjectId::Sticker(home),
        ObjectId::Sticker(other),
        ObjectId::Cell(7),
        ObjectId::Piece(piece),
        ObjectId::Cell(3),
    ];
    let atlas = Representation::layout(
        &header(1, &base),
        &analysed,
        vec![built_in(1), chart(2, &[("rotate", 6), ("perspective", 1)])],
        &[
            occurrence(1, ObjectId::Sticker(home), Some(1)),
            as_piece,
            occurrence(3, ObjectId::Sticker(other), Some(1)),
            occurrence(4, ObjectId::Cell(7), Some(2)),
        ],
    )
    .expect("an atlas");
    let sheet = Representation::layout(
        &header(2, &base),
        &analysed,
        Vec::new(),
        &[occurrence(5, ObjectId::Sticker(home), None), occurrence(6, ObjectId::Piece(piece), None)],
    )
    .expect("a sheet");
    Fixture { base, labels, digest, home, slot: slot as u32, piece, other, atlas, sheet }
}

/// CT-LB4-03 (part, the 600-cell-Full half: instances to canonical IDs and levels). Three display instances of one
/// sticker, in two charts of an atlas and on a sheet, resolve through the installed source map to the same canonical
/// sticker, each under its own instance ID, plan and producing source. The levels are explicit: the sticker level
/// answers for a sticker occurrence, the piece level only where the occurrence states its piece, the cell level for a
/// cell, and the slot level through the kernel's exact label projection of the bound subject, which is the label
/// state of 600-cell-Full. Not run: the pose level of a jumbling subject (M5).
#[test]
fn ct_lb4_03_full_half_display_instances_map_to_canonical_ids_and_levels() {
    let f = fixture();
    let mut host = host(&f.base);
    install(&mut host, 1, publication(1, &f.base, vec![f.atlas.clone(), f.sheet.clone()]));

    let expected = [
        (1, 1, ObjectId::Sticker(f.home)),
        (2, 1, ObjectId::Sticker(f.home)),
        (3, 1, ObjectId::Sticker(f.other)),
        (4, 1, ObjectId::Cell(7)),
        (5, 2, ObjectId::Sticker(f.home)),
        (6, 2, ObjectId::Piece(f.piece)),
    ];
    for (id, plan, object) in &expected {
        let r = total(&host, &reply(&host, *id), Requested::Occurrence, &PlantedInverse);
        assert_eq!((r.instance, r.plan), (InstanceId(*id), PlanId(*plan)));
        assert_eq!((&r.target, &r.occurrence), (object, object));
        // INV-6: the canonical ID is bound to its model and level, the label state of 600-cell-Full at the base.
        assert_eq!(r.subject, StateRef::Snapshot(f.base.clone()));
        assert_eq!(r.subject.identity(), &geometry().identity);
        assert_eq!(r.lineage, None);
    }

    // The copies share the canonical sticker and keep their own instance IDs and producing sources.
    let copies: Vec<Resolution> =
        [1, 2, 5].iter().map(|id| total(&host, &reply(&host, *id), Requested::Sticker, &PlantedInverse)).collect();
    assert!(copies.iter().all(|r| r.target == ObjectId::Sticker(f.home)));
    assert_eq!(copies.iter().map(|r| r.instance.0).collect::<Vec<_>>(), [1, 2, 5]);
    assert_eq!(copies[0].source, range(0, 1));
    assert_eq!(copies[1].source, range(1, 2));

    // Levels. A sticker occurrence has a piece target only where the layout shows it as part of its piece.
    undefined(&host, &reply(&host, 1), Requested::Piece);
    assert_eq!(total(&host, &reply(&host, 2), Requested::Piece, &PlantedInverse).target, ObjectId::Piece(f.piece));
    assert_eq!(total(&host, &reply(&host, 6), Requested::Piece, &PlantedInverse).target, ObjectId::Piece(f.piece));
    undefined(&host, &reply(&host, 6), Requested::Sticker);
    assert_eq!(total(&host, &reply(&host, 4), Requested::Cell, &PlantedInverse).target, ObjectId::Cell(7));
    undefined(&host, &reply(&host, 4), Requested::Sticker);
    undefined(&host, &reply(&host, 1), Requested::Cell);

    // The slot level is the kernel's exact projection of the bound label state: the moved sticker is in the slot the
    // word put it in, for every copy, and a fixed sticker is at home.
    let projection = LabelProjection::new(&f.labels, &f.digest);
    for id in [1, 2, 5] {
        let slot = total(&host, &reply(&host, id), Requested::Slot, &projection);
        assert_eq!(slot.target, ObjectId::Slot(f.slot));
        assert_ne!(f.slot, f.home, "the word moved the sticker");
    }
    assert_eq!(total(&host, &reply(&host, 3), Requested::Slot, &projection).target, ObjectId::Slot(f.other));
    // A projection for another subject gives no slot: the answer falls back to the occurrence and says the slot is
    // missing. None is invented from the display.
    let foreign = LabelProjection::new(&f.labels, "another subject");
    match host.resolve_pick(&reply(&host, 1), PickMode::Explicit, Requested::Slot, &foreign) {
        Ok(Some(Partial::Partial { covered, missing })) => {
            assert_eq!(covered.target, ObjectId::Sticker(f.home));
            assert_eq!(missing.len(), 1);
        }
        other => panic!("expected a partial answer without a slot, got {other:?}"),
    }

    // Linked selection reads the same map entries: the selected sticker has two images in the atlas and one on the
    // sheet, and an object the sheet does not show has no image there, which is not a relation of the object.
    assert_eq!(linked(&f.atlas, &ObjectId::Sticker(f.home)), Linked::Images(vec![InstanceId(1), InstanceId(2)]));
    assert_eq!(linked(&f.sheet, &ObjectId::Sticker(f.home)), Linked::Images(vec![InstanceId(5)]));
    assert_eq!(linked(&f.sheet, &ObjectId::Cell(7)), Linked::NoImage);
}

/// CT-LB4-03 (part, the 600-cell-Full half: a chart inverse value cannot identify a pick). All four occurrences of the
/// atlas sit at one sheet position in two charts, so no value computed back from a position or a chart could tell them
/// apart; each still resolves by its instance ID to its own object. A reply carries a binding and an instance ID and
/// nothing else, a planted inverse is never called, and an instance ID outside the installed map is refused with no
/// positional fallback.
#[test]
fn ct_lb4_03_full_half_a_chart_inverse_value_cannot_identify_a_pick() {
    let f = fixture();
    let occurrences = f.atlas.occurrences().expect("a filterable atlas");
    assert!(occurrences.iter().all(|o| o.position == [0.0, 0.0]), "every occurrence is drawn at one point");
    assert_eq!(occurrences.iter().filter(|o| o.chart == Some(ChartId(1))).count(), 2);

    let mut host = host(&f.base);
    install(&mut host, 1, publication(1, &f.base, vec![f.atlas.clone()]));
    let objects: Vec<ObjectId> = (1..=4)
        .map(|id| {
            let reply = reply(&host, id);
            for requested in [Requested::Occurrence, Requested::Sticker, Requested::Piece, Requested::Cell] {
                host.resolve_pick(&reply, PickMode::Explicit, requested, &PlantedInverse).expect("a current reply");
            }
            total(&host, &reply, Requested::Occurrence, &PlantedInverse).target
        })
        .collect();
    assert_eq!(
        objects,
        [ObjectId::Sticker(f.home), ObjectId::Sticker(f.home), ObjectId::Sticker(f.other), ObjectId::Cell(7)]
    );

    let unknown = reply(&host, 99);
    let error = host
        .resolve_pick(&unknown, PickMode::Explicit, Requested::Occurrence, &PlantedInverse)
        .expect_err("no instance 99");
    assert_eq!(error.code, codes::PLAN_INVALID);
}

/// CT-LB4-03 (part, the 600-cell-Full half: a layout filter preserves the mechanically analysed set). Filtering the
/// atlas to one chart, to the instances of one object, or to nothing rebuilds the layout, the instance list and the
/// map entries and leaves the analysed set, its exact-set digest, the subject, the base and the charts as they were
/// (INV-6). The filtered plan validates and installs; the engine revision the host adopted and the state digest do
/// not move (INV-8); a kept instance resolves to the same canonical ID as before, and a filtered-out instance has no
/// entry to resolve through.
#[test]
fn ct_lb4_03_full_half_a_layout_filter_preserves_the_analysed_set() {
    let f = fixture();
    let analysed = f.atlas.analysed().expect("the analysed set").clone();
    let mut sorted = vec![
        ObjectId::Sticker(f.home),
        ObjectId::Sticker(f.other),
        ObjectId::Cell(7),
        ObjectId::Piece(f.piece),
        ObjectId::Cell(3),
    ];
    sorted.sort();
    assert_eq!(analysed.objects, sorted, "canonical order, and wider than what the layout shows");
    assert_eq!(analysed.set_digest, object_set_digest(&sorted));

    let one_chart = f.atlas.filtered(|o| o.chart == Some(ChartId(1))).expect("filtered");
    let one_object = f.atlas.filtered(|o| o.object == ObjectId::Sticker(f.home)).expect("filtered");
    let nothing = f.atlas.filtered(|_| false).expect("filtered");
    for (filtered, kept) in [(&one_chart, vec![1, 3]), (&one_object, vec![1, 2]), (&nothing, vec![])] {
        assert_eq!(filtered.analysed(), Some(&analysed), "the analysed set is untouched");
        assert_eq!(filtered.plan.subject, f.atlas.plan.subject);
        assert_eq!(filtered.plan.base, f.atlas.plan.base);
        assert_eq!(filtered.plan.charts, f.atlas.plan.charts);
        assert_eq!(filtered.plan.geometry, f.atlas.plan.geometry);
        assert_eq!(filtered.plan.primitives.len(), f.atlas.plan.primitives.len());
        let shown: Vec<u64> = filtered.plan.instances.iter().map(|i| i.instance.0).collect();
        assert_eq!(shown, kept);
        assert_eq!(filtered.map.entries.keys().map(|id| id.0).collect::<Vec<_>>(), kept);
        for instance in &filtered.plan.instances {
            let before = f.atlas.plan.instances.iter().find(|i| i.instance == instance.instance).expect("kept");
            assert_eq!(instance, before, "a kept instance is the instance it was");
        }
    }

    // The filtered representation validates with the same subject, and installs.
    let whole = validate(&publication(1, &f.base, vec![f.atlas.clone()]), &context()).expect("valid");
    let part = validate(&publication(1, &f.base, vec![one_chart.clone()]), &context()).expect("valid");
    assert_eq!(whole.subjects, part.subjects);
    assert_ne!(whole.digest, part.digest, "the display plan changed, and only it");

    let mut host = host(&f.base);
    install(&mut host, 1, publication(1, &f.base, vec![f.atlas.clone()]));
    let before: Vec<ObjectId> = [1, 3]
        .iter()
        .map(|id| total(&host, &reply(&host, *id), Requested::Occurrence, &PlantedInverse).target)
        .collect();
    install(&mut host, 2, publication(1, &f.base, vec![one_chart]));
    assert_eq!(host.adopted(), &f.base, "the engine revision does not move");
    assert_eq!(host.adopted().digest, f.digest, "nor does the state digest");
    let after: Vec<ObjectId> = [1, 3]
        .iter()
        .map(|id| total(&host, &reply(&host, *id), Requested::Occurrence, &PlantedInverse).target)
        .collect();
    assert_eq!(before, after);
    let filtered_out = reply(&host, 2);
    let error = host
        .resolve_pick(&filtered_out, PickMode::Explicit, Requested::Occurrence, &PlantedInverse)
        .expect_err("instance 2 was filtered out");
    assert_eq!(error.code, codes::PLAN_INVALID);
    // The state the kernel holds is the one the fixture derived: no filter reached it.
    assert_eq!(derive(&[1]).state.digest(), f.digest);
}
