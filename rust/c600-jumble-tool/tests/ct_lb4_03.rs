//! CT-LB4-03, the jumbling half (LB4.4, INV-6, INV-8; RPI.22, RPI.23, J02, J13): several display instances of the
//! jumbled puzzle map to their canonical IDs and levels through chapter 10's source map at pose level, with no lift
//! from labels into poses; a chart inverse value cannot identify a pick; and a layout filter preserves the
//! mechanically analysed set. The state is the one after the first two records of the pinned I_a journal
//! (`research/jumbling/fixtures/wj-I_a.json`): some pieces at the identity, some off the lattice and some at lattice
//! poses other than the identity. The representations are `c600-repr`'s layouts with `c600-display`'s posed row (J02)
//! of that state beside them, installed over the mock backend in the jumbling fixture context, and the slot level is
//! `c600-display`'s pose projection (J13). C6's `c600-repr/tests/ct_lb4_03.rs` runs the 600-cell-Full half.
//!
//! What runs: eight instances of six canonical objects in an atlas of two charts and on a sheet, resolved at the
//! occurrence, sticker, piece, cell and slot levels on the pose-level subject (Jumble on, the identity block with the
//! I_a menu, the pose digest); a slot answer at the identity, off the lattice and at another lattice pose, each with
//! its own reason, and a 600-cell-Full label projection that holds no pose level, so it answers nothing for that
//! subject even under the subject's own digest; the atlas's occurrences
//! drawn at one point, two pieces sharing one pose index, and a planted inverse that nothing calls; and four layout
//! filters, one of them by pose, that leave the analysed set (the engine's certified inside set of the last twist), the
//! posed row, the subject and the base as they were, validate, install and move no state. Not run: a layout evaluated
//! again at a later revision of a jumbled session, since no command-layer row commits a jumbling state yet (LP2's
//! KN5); the slot of a sticker at a lattice pose other than the identity, which needs a K⁺ slot action the public
//! kernel API does not have (LP2); derived pose-level subjects; chart properties at exact poses (LB4.2); and any
//! backend, readback or drawing (J06). A pass is source and fixture evidence over the mock backend, not a backend run.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_digest::sha256_hex;
use c600_display::codes;
use c600_display::host::{Host, MockBackend};
use c600_display::jumble::{self, PoseProjection, OFF_LATTICE};
use c600_display::pick::{LabelProjection, Partial, PickMode, PickReply, Projection, Requested, Resolution};
use c600_display::plan::*;
use c600_display::produce::GeometryInput;
use c600_display::receipt::Status;
use c600_display::validate::{validate, Context, LimitProfile};
use c600_jumble::model::NP;
use c600_jumble::state::{Outcome, State};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context as Engine;
use c600_kernel::json::{self, Value};
use c600_kernel::{Kernel, Puzzle};
use c600_repr::layout::{Header, Occurrence, Representation};

/// The one producing source of the fixture maps.
const SOURCE: &str = "jumbling representations fixture source";

/// The slot reason `c600-display`'s pose projection gives for a piece at a lattice pose other than the identity.
const NO_SLOT_ACTION: &str = "slot: the kernel offers no slot action for a lattice pose";

/// The slot reason of a projection bound to another subject.
const ANOTHER_SUBJECT: &str = "slot: the projection is for another subject";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| Kernel::load(&repository().join("assets")).expect("the 600-cell-Full model loads"))
}

fn geometry() -> &'static GeometryInput {
    static GEOMETRY: OnceLock<GeometryInput> = OnceLock::new();
    GEOMETRY.get_or_init(|| GeometryInput::load(&repository().join("assets")).expect("the shipped geometry loads"))
}

/// The jumbling fixture context of `base`'s identity block, with generous declared ceilings.
fn context(base: &RevisionRef) -> Context {
    jumble::fixture_context(geometry(), base.identity.clone(), LimitProfile::uniform("fixture/1", 1 << 40))
}

/// The state after the first `count` records of the I_a journal from solved, and the outcome of the last.
fn walk(count: usize) -> (State, Outcome) {
    let engine = engine();
    let bytes = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    let doc = json::parse(&bytes).expect("the I_a fixture is JSON");
    let menu = Menu::from_record(engine, doc.get("menu").expect("a menu")).expect("the pinned I_a menu");
    let records = doc.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array).expect("records");
    let mut state = State::new(Some(Arc::new(menu)));
    let mut last = None;
    for (index, record) in records[..count].iter().enumerate() {
        let twist = Twist::from_record(engine, record).expect("a journal record");
        let outcome = state.apply(engine, &twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, c600_jumble::state::Status::Admissible, "record {index}");
        last = Some(outcome);
    }
    (state, last.expect("at least one record"))
}

/// The first home slot of `piece`: the canonical ID of one of its stickers.
fn home_of(piece: u32) -> u32 {
    engine().model.slot_piece.iter().position(|&p| p == piece).expect("every piece has a sticker") as u32
}

/// The piece an occurrence's object belongs to, read from the model.
fn piece_of(occurrence: &Occurrence) -> Option<u32> {
    match &occurrence.object {
        ObjectId::Sticker(home) => Some(engine().model.slot_piece[*home as usize]),
        ObjectId::Piece(piece) => Some(*piece),
        _ => None,
    }
}

fn source_digest() -> String {
    sha256_hex(SOURCE.as_bytes())
}

/// A range of the fixture source.
fn range(start: u64, end: u64) -> SourceLink {
    SourceLink::Range { digest: source_digest(), start, end }
}

/// A chart of fixture stages, version 1, each with its declared number of parameters.
fn chart(id: u64, stages: &[(&str, usize)]) -> Chart {
    Chart {
        chart: ChartId(id),
        stages: stages
            .iter()
            .map(|&(name, params)| Stage {
                name: name.to_owned(),
                version: 1,
                definition: sha256_hex(format!("{name}/1").as_bytes()),
                params: vec![0.5; params],
            })
            .collect(),
        dimension: 4,
        precision: "binary32".to_owned(),
        properties: Vec::new(),
    }
}

/// An occurrence of `object` as instance `id` at the sheet origin.
fn occurrence(id: u64, object: ObjectId, chart: Option<u64>, piece: Option<u32>, source: SourceLink) -> Occurrence {
    Occurrence { instance: InstanceId(id), object, chart: chart.map(ChartId), position: [0.0, 0.0], piece, source }
}

/// A sheet or atlas of `occurrences` at `base` analysed for `analysed`, with the posed row of the base state as
/// primitive 2, after `c600-repr`'s layout and analysed set.
fn representation(
    plan: u64,
    base: &RevisionRef,
    posed: &PosedInput,
    analysed: &[ObjectId],
    charts: Vec<Chart>,
    occurrences: &[Occurrence],
) -> Representation {
    let header = Header {
        plan: PlanId(plan),
        base: base.clone(),
        geometry: GeometryBinding { identity: base.identity.clone(), digests: geometry().digests() },
        sources: vec![source_digest()],
        token: "highlight".to_owned(),
    };
    let mut representation = Representation::layout(&header, analysed, charts, occurrences).expect("a layout");
    representation.plan.primitives.push(Primitive::Posed(posed.clone()));
    representation
}

/// A publication of the representations into view 1.
fn publication(base: &RevisionRef, parts: Vec<Representation>) -> Publication {
    let (plans, maps) = parts.into_iter().map(|r| (r.plan, r.map)).unzip();
    Publication {
        target: ViewId(1),
        base: base.clone(),
        nodes: Vec::new(),
        plans,
        maps,
        sources: BTreeMap::from([(source_digest(), Arc::new(SOURCE.as_bytes().to_vec()))]),
    }
}

/// A host whose engine image has adopted `base`, over a mock backend with the jumbling fixture report, view 1 open.
fn host(base: &RevisionRef) -> Host<MockBackend> {
    let ctx = context(base);
    let mut host = Host::new(MockBackend::new(ctx.support.clone()), ctx, base.clone());
    host.open_view(ViewId(1)).expect("view 1 opens");
    host
}

/// Publishes standalone and installs: the receipt must end `completed(view_published)`.
fn install(host: &mut Host<MockBackend>, command: u64, publication: Publication) {
    let receipt = host.publish(CommandId(command), publication);
    assert_eq!(receipt.status, Status::Accepted, "{:?}", receipt.status);
    let ticket = host.ticket(CommandId(command)).expect("the command was prepared");
    host.backend.finish(ticket);
    host.pump();
    let status = host.receipt(CommandId(command)).expect("a receipt").status;
    assert!(matches!(status, Status::Completed(_)), "{status:?}");
}

/// A reply for `instance` under view 1's current binding.
fn reply(host: &Host<MockBackend>, instance: u64) -> PickReply {
    PickReply { binding: host.pick_binding(ViewId(1)).expect("a current binding"), instance: InstanceId(instance) }
}

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

/// A slot request answered in part: the covered resolution at the occurrence and the missing part.
fn partial_slot(host: &Host<MockBackend>, instance: u64, projection: &dyn Projection) -> (Resolution, Vec<String>) {
    match host.resolve_pick(&reply(host, instance), PickMode::Explicit, Requested::Slot, projection) {
        Ok(Some(Partial::Partial { covered, missing })) => (covered, missing),
        other => panic!("expected a partial slot answer for instance {instance}, got {other:?}"),
    }
}

/// The fixture: the state after the first two I_a records, the certified inside set of the second, four pieces at
/// three kinds of pose, and two representations at that state's pose-level revision. Plan 1 is an atlas of two charts
/// whose six occurrences all sit at one sheet position; plan 2 is a sheet. Each plan draws the posed row.
struct Fixture {
    state: State,
    base: RevisionRef,
    posed: PosedInput,
    /// The certified inside set of the second record's twist.
    inside: Vec<u32>,
    /// J03's certified transform row of that twist.
    transform: TransformInput,
    /// A piece of `inside` off the lattice.
    off: u32,
    /// Another piece of `inside` at the same pose as `off`.
    twin: u32,
    /// A piece at a lattice pose other than the identity.
    turned: u32,
    /// A piece at the identity.
    still: u32,
    atlas: Representation,
    sheet: Representation,
}

fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let engine = engine();
        let (state, outcome) = walk(2);
        assert_eq!((state.moved_count(), state.off_lattice_count()), (5_799, 5_404), "moved and off-lattice pieces");
        let inside = outcome.inside.clone();
        let transform = jumble::transform(&outcome, None, 1.0, (13, 14)).expect("an admissible twist");
        let poses = state.pose_ids();
        let off =
            *inside.iter().find(|&&p| !state.is_lattice_piece(p as usize)).expect("a piece moved off the lattice");
        let twin = *inside
            .iter()
            .find(|&&p| p != off && poses[p as usize] == poses[off as usize])
            .expect("another piece at the same pose");
        let turned = (0..NP as u32)
            .find(|&p| matches!(state.pose_of(p as usize).kidx(), Some(k) if k != 0))
            .expect("a piece at a lattice pose other than the identity");
        let still = (0..NP as u32).find(|&p| state.pose_of(p as usize).kidx() == Some(0)).expect("an unmoved piece");
        let base = jumble::revision(engine, &state, 14);
        let posed = jumble::posed(engine, &state).expect("finite poses");

        let analysed: Vec<ObjectId> = inside.iter().map(|&p| ObjectId::Piece(p)).collect();
        let sticker = |piece: u32| ObjectId::Sticker(home_of(piece));
        let atlas = representation(
            1,
            &base,
            &posed,
            &analysed,
            vec![
                chart(1, &[("rotate", 6), ("stereographic", 0), ("camera3", 9)]),
                chart(2, &[("rotate", 6), ("perspective", 1)]),
            ],
            &[
                occurrence(1, sticker(off), Some(1), None, range(0, 1)),
                occurrence(2, sticker(off), Some(2), Some(off), range(1, 2)),
                occurrence(3, sticker(twin), Some(1), Some(twin), range(0, 1)),
                occurrence(4, sticker(turned), Some(2), None, range(0, 1)),
                occurrence(5, sticker(still), Some(1), None, range(0, 1)),
                occurrence(6, ObjectId::Cell(7), Some(2), None, range(0, 1)),
            ],
        );
        let sheet = representation(
            2,
            &base,
            &posed,
            &analysed,
            Vec::new(),
            &[
                occurrence(7, sticker(off), None, None, range(2, 3)),
                occurrence(8, ObjectId::Piece(off), None, None, range(0, 1)),
            ],
        );
        Fixture { state, base, posed, inside, transform, off, twin, turned, still, atlas, sheet }
    })
}

/// CT-LB4-03 (part, the jumbling half: instances to canonical IDs and levels). Eight display instances in an atlas and
/// on a sheet resolve through the installed source map to their canonical objects, each under its own instance ID,
/// plan and producing source. Every answer is bound to the pose level: the subject is the base revision with Jumble
/// on, the identity block of the jumbled puzzle with the I_a menu, and the pose digest (INV-6). The piece level answers
/// only where the occurrence states its piece, the cell level for a cell, and the slot level through the pose
/// projection (J13): a sticker at the identity is in its home slot, a sticker or piece off the lattice has no slot
/// (`off_lattice`), and a sticker at another lattice pose has no slot the kernel can name, with its own reason. No
/// slot is lifted from labels: the solved label state of 600-cell-Full has every sticker at home, and its projection
/// answers `KN1:input.level_not_held` for the pose-level subject, also when it is bound to that subject's own digest.
#[test]
fn ct_lb4_03_jumbling_half_display_instances_map_to_canonical_ids_at_pose_level() {
    let f = fixture();
    let engine = engine();
    let mut host = host(&f.base);
    install(&mut host, 1, publication(&f.base, vec![f.atlas.clone(), f.sheet.clone()]));

    let (off, twin, turned, still) = (home_of(f.off), home_of(f.twin), home_of(f.turned), home_of(f.still));
    let expected = [
        (1, 1, ObjectId::Sticker(off)),
        (2, 1, ObjectId::Sticker(off)),
        (3, 1, ObjectId::Sticker(twin)),
        (4, 1, ObjectId::Sticker(turned)),
        (5, 1, ObjectId::Sticker(still)),
        (6, 1, ObjectId::Cell(7)),
        (7, 2, ObjectId::Sticker(off)),
        (8, 2, ObjectId::Piece(f.off)),
    ];
    assert_eq!((f.base.mode, f.base.identity.puzzle), (Mode::JumbleOn, Puzzle::Jumbling));
    for (id, plan, object) in &expected {
        let r = total(&host, &reply(&host, *id), Requested::Occurrence, &PlantedInverse);
        assert_eq!((r.instance, r.plan), (InstanceId(*id), PlanId(*plan)));
        assert_eq!((&r.target, &r.occurrence), (object, object));
        // INV-6: the canonical ID is bound to the jumbled puzzle, its menu and the pose level of the base.
        assert_eq!(r.subject, StateRef::Snapshot(f.base.clone()));
        assert_eq!(r.subject.identity(), &engine.identity(f.state.menu()));
        assert_eq!(r.subject.digest(), f.state.digest(engine), "the pose digest of the state");
        assert_eq!(r.lineage, None);
    }

    // The copies share the canonical sticker and keep their own instance IDs and producing sources.
    let copies: Vec<Resolution> =
        [1, 2, 7].iter().map(|id| total(&host, &reply(&host, *id), Requested::Sticker, &PlantedInverse)).collect();
    assert!(copies.iter().all(|r| r.target == ObjectId::Sticker(off)));
    assert_eq!(copies.iter().map(|r| r.instance.0).collect::<Vec<_>>(), [1, 2, 7]);
    assert_eq!(copies.iter().map(|r| r.source.clone()).collect::<Vec<_>>(), [range(0, 1), range(1, 2), range(2, 3)]);

    // Levels. A sticker occurrence has a piece target only where the layout shows it as part of its piece; nothing is
    // inferred from its pose.
    undefined(&host, &reply(&host, 1), Requested::Piece);
    undefined(&host, &reply(&host, 4), Requested::Piece);
    for (id, piece) in [(2, f.off), (3, f.twin), (8, f.off)] {
        assert_eq!(total(&host, &reply(&host, id), Requested::Piece, &PlantedInverse).target, ObjectId::Piece(piece));
    }
    undefined(&host, &reply(&host, 8), Requested::Sticker);
    assert_eq!(total(&host, &reply(&host, 6), Requested::Cell, &PlantedInverse).target, ObjectId::Cell(7));
    undefined(&host, &reply(&host, 6), Requested::Sticker);
    undefined(&host, &reply(&host, 1), Requested::Cell);

    // The slot level is the kernel's exact pose projection of the bound subject (J13).
    let projection = PoseProjection::new(engine, &f.state);
    assert_eq!(total(&host, &reply(&host, 5), Requested::Slot, &projection).target, ObjectId::Slot(still));
    let off_lattice = vec![format!("slot: {OFF_LATTICE}")];
    for (id, target) in [(1, ObjectId::Sticker(off)), (2, ObjectId::Sticker(off)), (3, ObjectId::Sticker(twin))]
        .into_iter()
        .chain([(7, ObjectId::Sticker(off)), (8, ObjectId::Piece(f.off))])
    {
        let (covered, missing) = partial_slot(&host, id, &projection);
        assert_eq!((covered.target, missing), (target, off_lattice.clone()), "instance {id}: no slot is invented");
        assert_eq!(covered.subject.digest(), f.state.digest(engine), "the partial answer keeps its subject");
    }
    let (covered, missing) = partial_slot(&host, 4, &projection);
    assert_eq!((covered.target, missing), (ObjectId::Sticker(turned), vec![NO_SLOT_ACTION.to_owned()]));
    assert_ne!(NO_SLOT_ACTION, off_lattice[0], "another lattice pose is not off the lattice");
    let (covered, missing) = partial_slot(&host, 6, &projection);
    assert_eq!((covered.target, missing.len()), (ObjectId::Cell(7), 1), "a cell has no slot");

    // No lift from labels into poses (KN1.8, KN1.10). In the solved label state of 600-cell-Full every sticker is in
    // its home slot, the old occupant's answer. The label projection holds the label level only, so it answers nothing
    // for the pose-level subject, whether it is bound to its own label digest or to this subject's pose digest: the
    // slot level falls back to the occurrence and names the level, never the digest.
    let solved = kernel().solved();
    assert_eq!(solved.labels()[off as usize], off as i32, "the label state has the sticker at home");
    let not_held = vec![format!("slot: {}", codes::LEVEL_NOT_HELD)];
    for digest in [solved.digest().to_owned(), f.state.digest(engine)] {
        let labels = LabelProjection::new(solved.labels(), &digest);
        for (id, target) in [(1, ObjectId::Sticker(off)), (5, ObjectId::Sticker(still))] {
            let (covered, missing) = partial_slot(&host, id, &labels);
            assert_eq!((covered.target, missing), (target, not_held.clone()), "instance {id}");
        }
    }
    assert_ne!(not_held[0], ANOTHER_SUBJECT, "a level not held is not another subject");
    assert_ne!(not_held[0], off_lattice[0], "nor off the lattice");
}

/// CT-LB4-03 (part, the jumbling half: a chart inverse value cannot identify a pick). All six occurrences of the atlas
/// sit at one sheet position in two charts, and two of them are stickers of two pieces that share one pose index of
/// the posed row, so one pose matrix moves both: no value computed back from a position, a chart or a pose (a
/// pose-table index is never an identity, INV-6) could tell them apart. Each still resolves by its instance ID to its
/// own object and piece. A reply carries a binding and an instance ID and nothing else, a planted inverse is never
/// called, and an instance ID outside the installed map is refused with no positional fallback.
#[test]
fn ct_lb4_03_jumbling_half_a_chart_inverse_value_cannot_identify_a_pick() {
    let f = fixture();
    let occurrences = f.atlas.occurrences().expect("a filterable atlas");
    assert!(occurrences.iter().all(|o| o.position == [0.0, 0.0]), "every occurrence is drawn at one point");
    assert_eq!(occurrences.iter().filter(|o| o.chart == Some(ChartId(1))).count(), 3);
    let index = f.posed.pose_index.as_i32();
    assert!(f.off != f.twin && index[f.off as usize] == index[f.twin as usize], "two pieces, one pose index");

    let mut host = host(&f.base);
    install(&mut host, 1, publication(&f.base, vec![f.atlas.clone()]));
    let objects: Vec<ObjectId> = (1..=6)
        .map(|id| {
            let reply = reply(&host, id);
            for requested in [Requested::Occurrence, Requested::Sticker, Requested::Piece, Requested::Cell] {
                host.resolve_pick(&reply, PickMode::Explicit, requested, &PlantedInverse).expect("a current reply");
            }
            total(&host, &reply, Requested::Occurrence, &PlantedInverse).target
        })
        .collect();
    let sticker = |piece: u32| ObjectId::Sticker(home_of(piece));
    assert_eq!(
        objects,
        [sticker(f.off), sticker(f.off), sticker(f.twin), sticker(f.turned), sticker(f.still), ObjectId::Cell(7)]
    );
    let pieces: Vec<ObjectId> =
        [2, 3].iter().map(|id| total(&host, &reply(&host, *id), Requested::Piece, &PlantedInverse).target).collect();
    assert_eq!(pieces, [ObjectId::Piece(f.off), ObjectId::Piece(f.twin)], "one pose, two pieces");

    let unknown = reply(&host, 99);
    for requested in [Requested::Occurrence, Requested::Slot] {
        let error = host.resolve_pick(&unknown, PickMode::Explicit, requested, &PlantedInverse).expect_err("no 99");
        assert_eq!(error.code, codes::PLAN_INVALID);
    }
}

/// CT-LB4-03 (part, the jumbling half: a layout filter preserves the mechanically analysed set). The analysed set is
/// the engine's certified inside set of the state's last twist, J03's moving set, as pieces: a set the engine
/// certified, wider than what the layout shows. Filtering the atlas to one chart, to the instances of one object, to
/// the occurrences of pieces at the identity (a filter that reads the pose table), or to nothing rebuilds the layout,
/// the instance list and the map entries and leaves the analysed set, its exact-set digest, the posed row, the
/// subject, the base and the charts as they were (INV-6). The filtered plan validates at pose level with the same
/// subject and installs; the engine revision the host adopted and the pose digest do not move (INV-8); a kept instance
/// resolves to the same canonical ID as before, and a filtered-out instance has no entry to resolve through.
#[test]
fn ct_lb4_03_jumbling_half_a_layout_filter_preserves_the_analysed_set() {
    let f = fixture();
    let engine = engine();
    let analysed = f.atlas.analysed().expect("the analysed set").clone();
    let pieces: Vec<ObjectId> = f.inside.iter().map(|&p| ObjectId::Piece(p)).collect();
    assert_eq!(analysed.objects, pieces, "the certified inside set, in canonical order");
    assert_eq!(analysed.set_digest, object_set_digest(&pieces));
    assert_eq!(f.transform.moving.as_u32(), f.inside, "J03's certified moving set, not a set the display computed");
    assert_eq!(f.transform.moving_digest, piece_set_digest(&f.inside));
    assert!(f.inside.len() > 6 && !f.inside.contains(&f.still), "wider than the layout, and not what it shows");

    let at_identity = |o: &Occurrence| piece_of(o).is_some_and(|p| f.state.pose_of(p as usize).kidx() == Some(0));
    let one_chart = f.atlas.filtered(|o| o.chart == Some(ChartId(1))).expect("filtered");
    let one_object = f.atlas.filtered(|o| o.object == ObjectId::Sticker(home_of(f.off))).expect("filtered");
    let by_pose = f.atlas.filtered(at_identity).expect("filtered");
    let nothing = f.atlas.filtered(|_| false).expect("filtered");
    let filters = [(&one_chart, vec![1, 3, 5]), (&one_object, vec![1, 2]), (&by_pose, vec![5]), (&nothing, vec![])];
    for (filtered, kept) in filters {
        assert_eq!(filtered.analysed(), Some(&analysed), "the analysed set is untouched");
        assert_eq!(filtered.plan.subject, f.atlas.plan.subject);
        assert_eq!(filtered.plan.base, f.atlas.plan.base);
        assert_eq!(filtered.plan.charts, f.atlas.plan.charts);
        assert_eq!(filtered.plan.geometry, f.atlas.plan.geometry);
        assert_eq!(filtered.plan.primitives.len(), 3);
        assert_eq!(filtered.plan.primitives[2], Primitive::Posed(f.posed.clone()), "the posed row is untouched");
        let shown: Vec<u64> = filtered.plan.instances.iter().map(|i| i.instance.0).collect();
        assert_eq!(shown, kept);
        assert_eq!(filtered.map.entries.keys().map(|id| id.0).collect::<Vec<_>>(), kept);
        for instance in &filtered.plan.instances {
            let before = f.atlas.plan.instances.iter().find(|i| i.instance == instance.instance).expect("kept");
            assert_eq!(instance, before, "a kept instance is the instance it was");
        }
    }

    // The filtered representation validates at pose level with the same subject, and installs.
    let ctx = context(&f.base);
    let whole = validate(&publication(&f.base, vec![f.atlas.clone()]), &ctx).expect("valid");
    let part = validate(&publication(&f.base, vec![by_pose.clone()]), &ctx).expect("valid");
    assert_eq!(whole.subjects, part.subjects);
    assert!(whole.subjects.iter().all(|subject| subject.digest == f.base.digest), "the pose digest");
    assert_ne!(whole.digest, part.digest, "the display plan changed, and only it");

    let mut host = host(&f.base);
    install(&mut host, 1, publication(&f.base, vec![f.atlas.clone()]));
    let resolve = |host: &Host<MockBackend>, ids: &[u64]| -> Vec<ObjectId> {
        ids.iter().map(|id| total(host, &reply(host, *id), Requested::Occurrence, &PlantedInverse).target).collect()
    };
    let before = resolve(&host, &[1, 3, 5]);
    install(&mut host, 2, publication(&f.base, vec![one_chart]));
    assert_eq!(host.adopted(), &f.base, "the engine revision does not move");
    assert_eq!(host.adopted().digest, f.state.digest(engine), "nor does the pose digest");
    assert_eq!(resolve(&host, &[1, 3, 5]), before);
    let filtered_out = reply(&host, 2);
    let error = host
        .resolve_pick(&filtered_out, PickMode::Explicit, Requested::Occurrence, &PlantedInverse)
        .expect_err("instance 2 was filtered out");
    assert_eq!(error.code, codes::PLAN_INVALID);
}
