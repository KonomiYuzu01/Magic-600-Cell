//! The jumbling half of the display contract (LP3 C8) on M5's real model: CT-RPI-08's jumbling rows, CT-RPI-12's
//! off-lattice part, RPI.23's slot projection at lattice poses other than the identity and the RM.37 vectors of
//! CT-RM-07 that M5 can check. The states are the solved jumbling state, the state after the first applied twist of
//! the pinned I_a walk (`research/jumbling/fixtures/wj-I_a.json`), which is off the lattice, and the state a fixed
//! retained word makes from solved, which stays on it. Every result is a source fixture outcome over a mock backend,
//! not a backend run.

mod common;

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use c600_display::array::Array;
use c600_display::codes;
use c600_display::crosswalk::{exposed_geometry, survey_state, SurveyState};
use c600_display::jumble::{self, Missing, PoseProjection};
use c600_display::pick::{LabelProjection, Partial, PickMode, PickReply, Projection, Requested};
use c600_display::plan::*;
use c600_display::validate::{swept_preview, validate, validate_engine_bundle, Context, K_PLUS};
use c600_jumble::levels;
use c600_jumble::model::{CELL_SLOTS, NS};
use c600_jumble::observe::{seam_facts, Batch, Rows, Scope};
use c600_jumble::q5::Q5;
use c600_jumble::regions::near_poles;
use c600_jumble::state::{Outcome, State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context as Engine;
use c600_kernel::json::{self, Value};

use common::*;

const V1: ViewId = ViewId(1);

struct World {
    engine: Engine,
    fixture: Value,
    /// The solved state under the pinned I_a menu.
    solved: State,
    /// The state after the walk's first applied twist, that twist and its outcome.
    twisted: State,
    twist: Twist,
    outcome: Outcome,
    /// A cap the twisted state blocks.
    blocked: usize,
}

fn world() -> &'static World {
    static WORLD: OnceLock<World> = OnceLock::new();
    WORLD.get_or_init(|| {
        let engine = Engine::load(&assets()).expect("the jumbling model loads");
        let fixture = std::fs::read(assets().join("../research/jumbling/fixtures/wj-I_a.json")).expect("the fixture");
        let fixture = json::parse(&fixture).expect("the fixture is JSON");
        let menu = Menu::from_record(&engine, fixture.get("menu").expect("a menu")).expect("the pinned I_a menu");
        let solved = State::new(Some(Arc::new(menu)));
        let record = first_record(&fixture);
        let twist = Twist::from_record(&engine, record).expect("the first record is a menu twist");
        let mut twisted = solved.clone();
        let outcome = twisted.apply(&engine, &twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "the walk keeps only applied twists");
        let blocked = near_poles(&engine.model, twist.grip)
            .into_iter()
            .filter(|&cap| cap != twist.grip)
            .find(|&cap| {
                let classified = twisted.classify(&engine, cap, false, true, None).expect("no engine error");
                classified.status() == Status::Blocked
            })
            .expect("a cap near the twisted grip is blocked");
        World { engine, fixture, solved, twisted, twist, outcome, blocked }
    })
}

/// The plane of a transform row is the engine's plane of `twist` (J03, render/1 section 4), up to binary32
/// rounding: its rows are the engine's exact u and v normalized, in that order and with their signs, so they are unit
/// and orthogonal, and R(θ) = I + (cos θ − 1)(uuᵀ + vvᵀ) + sin θ (vuᵀ − uvᵀ) is the twist's matrix.
fn assert_engine_plane(transform: &TransformInput, twist: &Twist) {
    assert!(swept_preview(transform), "an engine-named plane gives a swept preview");
    let plane = transform.plane.as_ref().expect("a plane").as_f32();
    let named = twist.plane().expect("the twist turns one plane");
    let display = |x: &Q5| f64::from(jumble::approximate(x).expect("a finite value"));
    let (u, v) = (&plane[..4], &plane[4..]);
    for (row, exact) in [(u, &named.u), (v, &named.v)] {
        let engine: Vec<f64> = exact.iter().map(display).collect();
        let norm = engine.iter().map(|x| x * x).sum::<f64>().sqrt();
        for (&x, e) in row.iter().zip(&engine) {
            assert!((f64::from(x) - e / norm).abs() < 1e-6, "the engine's vector, normalized: {row:?}");
        }
    }
    let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(&x, &y)| f64::from(x) * f64::from(y)).sum::<f64>();
    assert!((dot(u, u) - 1.0).abs() < 1e-6 && (dot(v, v) - 1.0).abs() < 1e-6 && dot(u, v).abs() < 1e-6);
    let (c, s) = (f64::from(transform.angle).cos(), f64::from(transform.angle).sin());
    for i in 0..4 {
        for j in 0..4 {
            let (ui, uj, vi, vj) = (f64::from(u[i]), f64::from(u[j]), f64::from(v[i]), f64::from(v[j]));
            let r = f64::from(u8::from(i == j)) + (c - 1.0) * (ui * uj + vi * vj) + s * (vi * uj - ui * vj);
            assert!((r - display(&twist.matrix[i][j])).abs() < 1e-5, "R(θ) is the twist at ({i}, {j})");
        }
    }
}

fn first_record(fixture: &Value) -> &Value {
    let records = fixture.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array).expect("records");
    records.first().expect("the walk has a first twist")
}

fn ctx_for(state: &State) -> Context {
    jumble::fixture_context(geometry(), world().engine.identity(state.menu()), limits(1 << 40))
}

/// A session plan of the jumbling model at `base` and its source map; every instance links to `link()`.
fn jplan(
    base: &RevisionRef,
    primitives: Vec<Primitive>,
    charts: Vec<Chart>,
    instances: Vec<DisplayInstance>,
) -> Publication {
    let entries =
        instances.iter().map(|i| (i.instance, MapEntry { object: i.object.clone(), source: link() })).collect();
    let plan = DisplayPlan {
        plan: PlanId(1),
        base: base.clone(),
        subject: StateRef::Snapshot(base.clone()),
        derivation: None,
        reachability: None,
        dimension: 4,
        geometry: GeometryBinding { identity: base.identity.clone(), digests: geometry().digests() },
        primitives,
        charts,
        instances,
        source_map: MapId(1),
        precision: "binary32".to_owned(),
        dependencies: BTreeMap::from([("state".to_owned(), base.state.to_string())]),
    };
    let map = SourceMap {
        map: MapId(1),
        plan: PlanId(1),
        base: base.clone(),
        sources: vec![SOURCE_DIGEST.to_owned()],
        entries,
    };
    publication(1, base, vec![(plan, map)])
}

fn recorded_hash(stage: &str, array: &str) -> String {
    let arrays = world().fixture.get("stages").and_then(|s| s.get(stage)).and_then(|s| s.get("arrays"));
    arrays.and_then(|a| a.get(array)).and_then(|a| a.get("sha256")).and_then(Value::as_str).expect("a hash").to_owned()
}

/// A home slot of `piece`.
fn home_of(piece: u32) -> u32 {
    world().engine.model.slot_piece.iter().position(|&p| p == piece).expect("every piece has a slot") as u32
}

/// CT-RPI-08, jumbling rows, produced-and-validated part (RPI.15, RPI.19): the posed row (J02), the certified
/// transform row (J03), the projection chain (J05) and piece and sticker instances are produced from M5's states and
/// pass the host validator and installation. The posed arrays of the solved state equal the three array hashes the
/// J1 fixture records for its start stage; after a twist the lattice entries agree with M5's exact K⁺ test. The
/// transform row carries the plane the engine names for the twist, so it has a swept preview whose R(θ) is the twist.
/// Not run: J01's shrink anchors, J06's backend readback, the J07 to J12 payloads and any array hash of a later J1
/// stage.
#[test]
fn ct_rpi_08_jumbling_rows_are_produced_and_validated() {
    let world = world();
    let engine = &world.engine;

    let start = jumble::posed(engine, &world.solved).expect("finite poses");
    for (array, name) in
        [(&start.pose_index, "pose_index.i32"), (&start.poses, "poses.f32"), (&start.pose_lattice, "pose_lattice.i32")]
    {
        assert_eq!(array.desc.digest, recorded_hash("start", name), "{name} at the start stage");
    }

    let state = &world.twisted;
    let posed = jumble::posed(engine, state).expect("finite poses");
    let (index, lattice, matrices) = (posed.pose_index.as_i32(), posed.pose_lattice.as_i32(), posed.poses.as_f32());
    assert_eq!(lattice.len(), state.poses().len());
    assert!(lattice.len() > 1 && lattice.contains(&-1), "the twist leaves the lattice");
    for (id, &k) in lattice.iter().enumerate() {
        let exact = engine.kplus.index_of_matrix(state.pose_matrix(engine, id as u32), &engine.model);
        assert_eq!(k, exact.map_or(-1, i32::from), "pose {id}: the lattice entry is the exact K+ test");
        let m = &matrices[id * 16..id * 16 + 16];
        for r in 0..4 {
            for c in 0..4 {
                let dot: f32 = (0..4).map(|j| m[r * 4 + j] * m[c * 4 + j]).sum();
                assert!((dot - if r == c { 1.0 } else { 0.0 }).abs() < 1e-5, "pose {id} is not a rotation");
            }
        }
    }
    let off = index.iter().filter(|&&i| lattice[i as usize] < 0).count();
    assert_eq!(off, state.off_lattice_count(), "a piece is off the lattice by its pose's lattice entry");
    let moved = first_record(&world.fixture).get("moved").and_then(Value::as_i64).expect("the recorded count");
    assert_eq!(index.iter().filter(|&&i| i != 0).count() as i64, moved, "the pieces J1 recorded as moved");

    let base = jumble::revision(engine, state, 13);
    assert_eq!((base.mode, base.digest.as_str()), (Mode::JumbleOn, state.digest(engine).as_str()));
    validate_engine_bundle(&jumble::bundle(&base, &posed)).expect("one revision, three arrays");

    let transform = jumble::transform(&world.outcome, &world.twist, (12, 13)).expect("a finite plane");
    let transform = transform.expect("admissible");
    assert_eq!(transform.moving.as_u32(), world.outcome.inside, "the certified inside set, not a float test");
    assert_eq!(transform.moving.rows() as i64, moved);
    assert!(!world.twist.plane().expect("a plane").is_half_turn(), "the walk's first twist is no half-turn");
    assert!(0.0 < transform.angle && transform.angle < std::f32::consts::PI, "θ in (0, π)");
    assert_engine_plane(&transform, &world.twist);

    let piece = world.outcome.inside[0];
    let instances = vec![
        instance(1, 1, ObjectId::Piece(piece), 0),
        DisplayInstance { piece: Some(piece), ..instance(2, 1, ObjectId::Sticker(home_of(piece)), 0) },
    ];
    let primitives = vec![
        Primitive::Posed(posed),
        Primitive::Transform(transform),
        Primitive::ChartStage(StageInput { chart: ChartId(1) }),
    ];
    let publication = jplan(&base, primitives, vec![built_in(1)], instances);
    let ctx = ctx_for(state);
    let validated = validate(&publication, &ctx).expect("the jumbling rows validate");
    assert_eq!((validated.counts["instances"], validated.counts["charts"]), (2, 1));
    let mut host = host_with(&base, ctx);
    install(&mut host, 1, publication);
    assert_eq!(host.instances(V1), [InstanceId(1), InstanceId(2)]);
}

/// CT-RPI-08, jumbling rows, unsupported and missing-data part (RPI.15, RPI.17, RPI.19): a backend that reports no
/// J02, a malformed posed payload, a label-level subject, a blocked twist's missing moving set, a twist that is not
/// the outcome's letter, a transform without the engine's plane, the identity, which turns no plane, a named-cap set
/// its budget left incomplete and an absent exposed-face geometry (the refusal path; M5 supplies none) each give their
/// explicit outcome, and nothing is guessed, truncated or installed. Not run: a backend's own refusals.
#[test]
fn ct_rpi_08_jumbling_unsupported_and_missing_data_are_explicit() {
    let world = world();
    let engine = &world.engine;
    let state = &world.twisted;
    let base = jumble::revision(engine, state, 13);
    let posed = jumble::posed(engine, state).expect("finite poses");
    let ctx = ctx_for(state);
    let single = |primitive: Primitive, base: &RevisionRef| jplan(base, vec![primitive], vec![], vec![]);

    // The rows the fixture report omits or replaces stay unsupported (RM.15).
    for row in ["J01", "J06", "J07", "J08", "J09", "J10", "J11", "J12", "F01", "F03", "F04"] {
        let error = ctx.support.feature(row).expect_err("not supported");
        assert_eq!((error.code, error.context["feature"].as_str()), (codes::UNSUPPORTED, row));
    }
    let mut without = ctx.clone();
    without.support.features.remove("J02");
    let error = validate(&single(Primitive::Posed(posed.clone()), &base), &without).expect_err("no J02");
    assert_eq!((error.code, error.context["feature"].as_str()), (codes::UNSUPPORTED, "J02"));
    let mut host = host_with(&base, without);
    let receipt = host.publish(CommandId(1), single(Primitive::Posed(posed.clone()), &base));
    assert_eq!(refused(&receipt).code, codes::UNSUPPORTED);
    assert!(host.installed(V1).is_none(), "no other row or subject is substituted");

    // Malformed posed payloads are invalid, whatever the backend supports.
    let poses = posed.pose_lattice.rows();
    let mut short = posed.pose_index.as_i32();
    short.pop();
    let mut outside = posed.pose_index.as_i32();
    outside[0] = poses as i32;
    let mut turned = posed.poses.as_f32();
    turned.swap(0, 1);
    let mut infinite = posed.poses.as_f32();
    infinite[20] = f32::INFINITY;
    let mut lattice = posed.pose_lattice.as_i32();
    lattice.push(-1);
    let mut negative = posed.pose_index.as_i32();
    negative[0] = -1;
    let lattice_with = |pose: usize, entry: i32| {
        let mut entries = posed.pose_lattice.as_i32();
        entries[pose] = entry;
        PosedInput { pose_lattice: Array::i32s(vec![poses], &entries), ..posed.clone() }
    };
    assert!(poses > 1, "the twisted state has more than one pose");
    let malformed = [
        PosedInput { pose_index: Array::i32s(vec![short.len() as u64], &short), ..posed.clone() },
        PosedInput { pose_index: Array::i32s(vec![outside.len() as u64], &outside), ..posed.clone() },
        PosedInput { poses: Array::f32s(vec![poses, 4, 4], &turned), ..posed.clone() },
        PosedInput { poses: Array::f32s(vec![poses, 4, 4], &infinite), ..posed.clone() },
        PosedInput { pose_lattice: Array::i32s(vec![poses + 1], &lattice), ..posed.clone() },
        PosedInput { pose_index: Array::i32s(vec![negative.len() as u64], &negative), ..posed.clone() },
        lattice_with(1, K_PLUS),
        lattice_with(1, -2),
        lattice_with(0, -1),
        lattice_with(0, 5),
        PosedInput { poses: Array::f32s(vec![0, 4, 4], &[]), pose_lattice: Array::i32s(vec![0], &[]), ..posed.clone() },
    ];
    for (case, payload) in malformed.into_iter().enumerate() {
        let error = validate(&single(Primitive::Posed(payload), &base), &ctx).expect_err("malformed");
        assert_eq!(error.code, codes::PLAN_INVALID, "case {case}");
    }
    let labels = RevisionRef { mode: Mode::JumbleOff, ..base.clone() };
    let error = validate(&single(Primitive::Posed(posed.clone()), &labels), &ctx).expect_err("label level");
    assert_eq!(error.code, codes::PLAN_INVALID, "a posed row draws a pose-level subject");

    // A twist the engine did not certify has no moving set, and no plane is taken from a twist that is not the
    // outcome's letter. A row without the engine's plane has no swept preview, and the identity turns no plane.
    let mut blocked = state.clone();
    let cap = world.blocked;
    let turn = engine.kplus.a4(cap).into_iter().find(|&k| k != 0).expect("a rotation about the cap");
    let twist = c600_jumble::twist::kplus_twist(engine, cap, turn).expect("it fixes the cap's pole");
    let mut free = State::new(None);
    let record = Twist::from_record(engine, first_record(&world.fixture)).expect("the first record");
    free.apply(engine, &record, None, &[]).expect("no engine error");
    let refused_twist = free.apply(engine, &twist, None, &[]).expect("no engine error");
    assert_eq!(refused_twist.status, Status::Blocked);
    assert_eq!(jumble::transform(&refused_twist, &twist, (13, 14)), Ok(None), "no moving set is guessed");
    let other = Missing::OtherGrip { outcome: world.outcome.grip as u32, twist: cap as u32 };
    assert_eq!(jumble::transform(&world.outcome, &twist, (12, 13)), Err(other), "another grip's twist");
    let certified = jumble::transform(&world.outcome, &world.twist, (12, 13)).expect("a finite plane");
    let certified = certified.expect("admissible");
    assert!(swept_preview(&certified), "the engine names the twist's plane");
    assert!(!swept_preview(&TransformInput { plane: None, ..certified }), "no plane, no swept preview");
    let identity = c600_jumble::twist::kplus_twist(engine, 0, 0).expect("the identity fixes pole 0");
    assert_eq!(jumble::swept_plane(&identity), Ok(None), "the identity turns no plane");
    let unmoved = State::new(None).apply(engine, &identity, None, &[]).expect("no engine error");
    let row = jumble::transform(&unmoved, &identity, (0, 1)).expect("no plane to convert").expect("admissible");
    assert!(row.plane.is_none() && row.angle == 0.0 && !swept_preview(&row), "the identity has no swept preview");

    // A named-cap set the budget left incomplete is missing data, never a shorter set.
    match jumble::named_cap_set(engine, &mut blocked, cap, Some(0)) {
        Err(Missing::Uncertain { cap: named, pieces }) => assert!(named == cap as u32 && pieces > 0),
        other => panic!("expected an incomplete set, got {other:?}"),
    }
    assert_eq!(jumble::named_cap_set(engine, &mut blocked, 600, None), Err(Missing::NoSuchCap { cap: 600 }));

    // J10's refusal path: with no certified exposed-face geometry or size profile supplied (M5 supplies none),
    // nothing is drawn.
    let error = exposed_geometry(None, None).expect_err("no exposed-face geometry is supplied");
    assert_eq!(error.code, codes::EXPOSED_GEOMETRY_UNAVAILABLE);
}

fn reply(host: &c600_display::host::Host<c600_display::host::MockBackend>, instance: u64) -> PickReply {
    PickReply { binding: host.pick_binding(V1).expect("a current binding"), instance: InstanceId(instance) }
}

/// CT-RPI-12, off-lattice part (RPI.23, RPI.25), on a real jumbling state off the lattice: a slot request for a
/// sticker or piece off the lattice is a partial answer with reason `off_lattice` and no invented slot; a sticker
/// at the identity pose resolves to its home slot; the partial answer differs from another subject's partial answer,
/// from a projection at the other level (`KN1:input.level_not_held`, both ways, under a planted equal digest), from
/// an unknown one (a level the occurrence has no target at) and from a stale one, which is checked first. The stale
/// half itself is C2's test in `picking.rs`; a sticker of a piece at a lattice pose other than the identity is
/// `rpi_23_a_sticker_at_a_lattice_pose_is_in_the_slot_of_the_slot_action`'s.
#[test]
fn ct_rpi_12_off_lattice_partial_answers_stay_distinct_on_a_jumbling_state() {
    let world = world();
    let engine = &world.engine;
    let state = &world.twisted;
    let off = world.outcome.inside.iter().copied().find(|&p| !state.is_lattice_piece(p as usize)).expect("moved off");
    let home = (0..c600_jumble::model::NP as u32).find(|&p| state.pose_ids()[p as usize] == 0).expect("unmoved");
    let base = jumble::revision(engine, state, 13);
    let posed = jumble::posed(engine, state).expect("finite poses");
    let instances = vec![
        DisplayInstance { piece: Some(off), ..instance(1, 1, ObjectId::Sticker(home_of(off)), 0) },
        instance(2, 1, ObjectId::Piece(off), 0),
        DisplayInstance { piece: Some(home), ..instance(3, 1, ObjectId::Sticker(home_of(home)), 0) },
    ];
    let mut host = host_with(&base, ctx_for(state));
    install(&mut host, 1, jplan(&base, vec![Primitive::Posed(posed)], vec![], instances));
    let projection = PoseProjection::new(engine, state).expect("the level tables place every lattice piece");
    let resolve = |instance: u64, mode, requested, projection: &PoseProjection| {
        host.resolve_pick(&reply(&host, instance), mode, requested, projection)
    };

    for (instance, covered_target) in [(1, ObjectId::Sticker(home_of(off))), (2, ObjectId::Piece(off))] {
        for mode in [PickMode::Explicit, PickMode::Passive] {
            let Ok(Some(Partial::Partial { covered, missing })) = resolve(instance, mode, Requested::Slot, &projection)
            else {
                panic!("off the lattice the answer is partial")
            };
            assert_eq!((covered.target, missing), (covered_target.clone(), vec!["slot: off_lattice".to_owned()]));
            assert_eq!(covered.subject.digest(), state.digest(engine), "the partial answer keeps its subject");
        }
    }
    let Ok(Some(Partial::Total(piece))) = resolve(1, PickMode::Explicit, Requested::Piece, &projection) else {
        panic!("the declared piece")
    };
    assert_eq!(piece.target, ObjectId::Piece(off), "the piece is known off the lattice; only its slot is not");
    let Ok(Some(Partial::Total(slot))) = resolve(3, PickMode::Explicit, Requested::Slot, &projection) else {
        panic!("a sticker at the identity pose is in its home slot")
    };
    assert_eq!(slot.target, ObjectId::Slot(home_of(home)));

    // A projection of another subject is a partial answer with its own reason, not `off_lattice`; a level the
    // occurrence has no target at is unknown.
    let other = PoseProjection::new(engine, &world.solved).expect("the solved state is on the lattice");
    let Ok(Some(Partial::Partial { missing, .. })) = resolve(1, PickMode::Explicit, Requested::Slot, &other) else {
        panic!("another subject's projection gives a partial answer")
    };
    assert_eq!(missing, ["slot: the projection is for another subject"], "not reported as off the lattice");
    let unknown = resolve(1, PickMode::Explicit, Requested::Cell, &projection);
    assert!(matches!(unknown, Ok(Some(Partial::Undefined(_)))), "{unknown:?}");

    // A projection at the other level is neither (KN1.8, KN1.10), and the level is read before any digest. The solved
    // label state of 600-cell-Full has the off-lattice piece's sticker at home, the old occupant's answer; bound to
    // this subject's own pose digest, it still gives that sticker no slot. The pose projection likewise answers
    // nothing for a label-level snapshot whose digest string is its own.
    let solved = kernel().solved();
    assert_eq!(solved.labels()[home_of(off) as usize], home_of(off) as i32, "the label state has it at home");
    let lifted = LabelProjection::new(solved.labels(), &state.digest(engine));
    let Ok(Some(Partial::Partial { covered, missing })) =
        host.resolve_pick(&reply(&host, 1), PickMode::Explicit, Requested::Slot, &lifted)
    else {
        panic!("a label projection gives a pose-level subject no slot")
    };
    assert_eq!(
        (covered.target, missing),
        (ObjectId::Sticker(home_of(off)), vec![format!("slot: {}", codes::LEVEL_NOT_HELD)])
    );
    let labels = StateRef::Snapshot(RevisionRef { mode: Mode::JumbleOff, ..base.clone() });
    assert_eq!((labels.level(), labels.digest()), (Level::Labels, state.digest(engine).as_str()));
    let answer = projection.slot(&labels, &ObjectId::Sticker(home_of(home)));
    assert_eq!(answer, Partial::Undefined(codes::LEVEL_NOT_HELD.to_owned()), "no slot from poses at the label level");
    assert_eq!(
        projection.slot(&StateRef::Snapshot(base.clone()), &ObjectId::Sticker(home_of(home))),
        Partial::Total(home_of(home)),
        "the same request at the pose level has its slot"
    );

    // Stale is checked first and is neither: explicit work gets its reason, a passive reply is dropped.
    let delayed = reply(&host, 1);
    host.set_camera(V1);
    let error = host.resolve_pick(&delayed, PickMode::Explicit, Requested::Slot, &projection).expect_err("stale");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "camera"));
    assert_eq!(host.resolve_pick(&delayed, PickMode::Passive, Requested::Slot, &projection), Ok(None));
}

/// The state the fixed retained word `[1, 3, -5, 2, -7, 11]` (M8's primitive ids) makes from solved under the I_a
/// menu: on the lattice, with many pieces at poses other than the identity.
fn retained() -> State {
    let world = world();
    let mut state = world.solved.clone();
    for mid in [1, 3, -5, 2, -7, 11] {
        let twist = levels::primitive(&world.engine, mid).expect("a retained generator");
        let outcome = state.apply(&world.engine, &twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "letter {mid}");
    }
    state
}

/// RPI.23 and J13 at lattice poses other than the identity, on [`retained`]'s state: every sticker is in the slot
/// whose label it is (M8's `levels::labels`, A3's π(X)), and a sticker of a turned piece resolves through the host to
/// the slot k·s of M8's slot action with its frame check (`levels::Tables::slot_image`), not to its home slot s. A
/// piece has no single slot, on the lattice as off it, and a label-level subject gets `KN1:input.level_not_held`
/// before any digest is compared.
#[test]
fn rpi_23_a_sticker_at_a_lattice_pose_is_in_the_slot_of_the_slot_action() {
    let world = world();
    let engine = &world.engine;
    let state = retained();
    let levels::Partial::Total(labels) = levels::labels(engine, &state).expect("no engine error") else {
        panic!("a retained word from solved stays on the lattice")
    };
    let projection = PoseProjection::new(engine, &state).expect("the level tables place every lattice piece");
    let base = jumble::revision(engine, &state, 21);
    let subject = StateRef::Snapshot(base.clone());
    for (slot, &label) in (0u32..).zip(&labels.labels) {
        let answer = projection.slot(&subject, &ObjectId::Sticker(label as u32));
        assert_eq!(answer, Partial::Total(slot), "sticker {label} is in the slot whose label it is");
    }

    let tables = levels::tables(engine).expect("the level tables");
    let (home, k) = (0..NS as u32)
        .find_map(|s| {
            let k = state.pose_of(engine.model.slot_piece[s as usize] as usize).kidx()?;
            (tables.slot_image(engine, s, k).ok()? != s).then_some((s, k))
        })
        .expect("the word moves a sticker");
    let moved_to = tables.slot_image(engine, home, k).expect("the slot action");
    assert_eq!(labels.labels[moved_to as usize], home as i32, "the slot whose label is the sticker");
    let piece = engine.model.slot_piece[home as usize];
    let posed = jumble::posed(engine, &state).expect("finite poses");
    let instances = vec![
        DisplayInstance { piece: Some(piece), ..instance(1, 1, ObjectId::Sticker(home), 0) },
        instance(2, 1, ObjectId::Piece(piece), 0),
    ];
    let mut host = host_with(&base, ctx_for(&state));
    install(&mut host, 1, jplan(&base, vec![Primitive::Posed(posed)], vec![], instances));
    let resolve =
        |instance| host.resolve_pick(&reply(&host, instance), PickMode::Explicit, Requested::Slot, &projection);
    let Ok(Some(Partial::Total(slot))) = resolve(1) else { panic!("a sticker at a lattice pose has a slot") };
    assert_eq!(slot.target, ObjectId::Slot(moved_to), "the slot of the slot action, not the home slot {home}");
    let Ok(Some(Partial::Partial { covered, missing })) = resolve(2) else { panic!("a piece has no single slot") };
    assert_eq!(
        (covered.target, missing),
        (ObjectId::Piece(piece), vec!["slot: a piece has no single slot".to_owned()])
    );

    let labels_level = StateRef::Snapshot(RevisionRef { mode: Mode::JumbleOff, ..base.clone() });
    assert_eq!(labels_level.digest(), subject.digest(), "the digest string is the pose subject's own");
    let answer = projection.slot(&labels_level, &ObjectId::Sticker(home));
    assert_eq!(answer, Partial::Undefined(codes::LEVEL_NOT_HELD.to_owned()), "no slot from poses at the label level");
}

/// An engine error of the level tables is an error, never an answer (J13): in a second copy of the model with a
/// planted wrong frame for one cell (the identity, where the frame of a cell other than cell 0 carries pole 0 to that
/// cell's pole), A3's frame check refuses a sticker of a turned piece on that cell whose image cell differs, and the
/// projection of [`retained`]'s state is [`Missing::Engine`] with M8's own error, as `levels::labels` reports it, and
/// no projection. The defect is planted in the copy's tables only: its model ID is changed, so the level tables of the
/// shipped model stay as they are.
#[test]
fn rpi_23_an_engine_error_of_the_level_tables_is_no_projection() {
    let state = retained();
    let mut broken = Engine::load(&assets()).expect("the jumbling model loads");
    broken.model.model_id.push_str("+planted-frame");
    let cell = (0..NS)
        .filter(|&s| s / CELL_SLOTS != 0)
        .find_map(|s| {
            let k = state.pose_of(broken.model.slot_piece[s] as usize).kidx().filter(|&k| k != 0)?;
            let f = s / CELL_SLOTS;
            (broken.kplus.perms[k as usize][f] as usize != f).then_some(f)
        })
        .expect("a turned piece has a sticker off cell 0 whose cell moves");
    assert_ne!(broken.kplus.frame_idx[cell], 0, "the frame of a cell other than cell 0 is not the identity");
    broken.kplus.frame_idx[cell] = 0;

    let error = levels::labels(&broken, &state).err().expect("A3's frame check refuses the planted frame");
    let projection = PoseProjection::new(&broken, &state);
    assert!(matches!(&projection, Err(Missing::Engine(reason)) if *reason == error.0), "{:?}", projection.err());
}

/// CT-RM-07, jumbling half on M5's real model (RM.34, RM.37): rows 1 (mixed-revision bundle), 2 (a certified
/// half-turn: with the plane the engine names it has a swept preview at angle π in the engine's direction, and the
/// same row without that plane has none), 3 (full named-cap set against its cap-map witness), 4 and 5 (the full set
/// over a count ceiling, standalone and scheduled), 6 (an incomplete survey and the named cut's alpha), 7 (an
/// unsupported projection stage), 8 (complete and interrupted seam totals) and 15 (a certificate-point hit on an
/// off-lattice piece). Not run on the real model: rows 9 and 11 to 14 (covers and repair, release order, nu and the
/// correction catalogue), which stay C2's fixture vectors in `crosswalk.rs`; row 10 has no real geometry to check.
#[test]
fn ct_rm_07_jumbling_vectors_hold_on_the_real_model() {
    let world = world();
    let engine = &world.engine;
    let mut state = world.twisted.clone();
    let cap = world.blocked;
    let ctx = ctx_for(&state);

    // Row 1: revision 12's pose index with revision 13's tables is refused whole.
    let (before, after) = (jumble::revision(engine, &world.solved, 12), jumble::revision(engine, &state, 13));
    let (old, new) =
        (jumble::posed(engine, &world.solved).expect("finite"), jumble::posed(engine, &state).expect("finite"));
    let mut mixed = jumble::bundle(&after, &new);
    mixed.arrays[0] = jumble::bundle(&before, &old).arrays.remove(0);
    assert_eq!(mixed.arrays[0].name, "pose_index");
    assert_eq!(validate_engine_bundle(&mixed).expect_err("a mixed bundle").code, codes::BUNDLE_MISMATCH);

    // Row 2: a certified half-turn sweeps only in the direction the engine names (render/1 section 4); the same row
    // without that plane, the vector's case, has no swept preview, and no direction is invented.
    let half = engine.kplus.a4(0).into_iter().find(|&k| k != 0 && engine.kplus.compose(k, k) == 0).expect("order 2");
    let twist = c600_jumble::twist::kplus_twist(engine, 0, half).expect("it fixes pole 0");
    assert!(twist.plane().expect("a half-turn turns one plane").is_half_turn());
    let mut free = State::new(None);
    let outcome = free.apply(engine, &twist, None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
    let transform = jumble::transform(&outcome, &twist, (0, 1)).expect("a finite plane").expect("certified");
    assert_eq!(transform.angle, std::f32::consts::PI, "a half-turn turns by π");
    assert_engine_plane(&transform, &twist);
    let without = TransformInput { plane: None, ..transform.clone() };
    assert!(!swept_preview(&without), "a half-turn without the engine's plane has no swept preview");
    assert_eq!(transform.moving.as_u32(), outcome.inside, "the certified set; no guessed plane or legality");

    // Row 3: the full named-cap set keeps every piece and both points of each; the witness names one piece.
    let full = jumble::named_cap_set(engine, &mut state, cap, None).expect("a complete set");
    assert!(full.pieces.len() > 1, "cap {cap} has more than one straddler");
    assert_eq!((full.kind, full.pairs.len()), (CertKind::FullNamedCapSet, full.pieces.len()));
    let classified = state.classify(engine, cap, false, true, None).expect("no engine error");
    let certificate = classified.straddle.as_ref().expect("blocked");
    assert!(state.check_certificate(
        engine,
        cap,
        certificate.piece as usize,
        &certificate.point_below,
        &certificate.point_above
    ));
    let witness = jumble::witness(&classified).expect("a witness");
    assert_eq!((witness.kind, witness.pieces.len(), witness.pairs.len()), (CertKind::CapMapWitness, 1, 1));
    assert!(full.pieces.contains(&witness.pieces[0]) && witness != full, "the witness cannot replace the set");
    let base = jumble::revision(engine, &state, 13);
    let with = |overlay: &CertOverlay| jplan(&base, vec![Primitive::CertOverlay(overlay.clone())], vec![], vec![]);
    let items = 3 * full.pieces.len() as u64;
    assert_eq!(validate(&with(&full), &ctx).expect("the whole payload").counts["overlay_items"], items);
    let mut dropped = full.clone();
    dropped.pieces.truncate(1);
    assert_eq!(validate(&with(&dropped), &ctx).expect_err("a truncated set").code, codes::PLAN_INVALID);

    // Rows 4 and 5: over a count ceiling the whole payload is refused or fails; nothing shorter installs.
    let mut low = limits(1 << 40);
    low.overlay_items = items - 1;
    let tight = jumble::fixture_context(geometry(), base.identity.clone(), low);
    let (refusal, failure) = both_sides(with(&full), tight);
    for error in [refusal, failure] {
        assert_eq!(error.code, codes::PLAN_CEILING);
        assert_eq!(
            (error.context["field"].as_str(), error.context["requested"].clone(), error.context["ceiling"].clone()),
            ("overlay_items", items.to_string(), (items - 1).to_string())
        );
    }

    // Row 6: a survey with an uncertain grip stays pending; the cut carries the model's exact alpha.
    let grips: Vec<usize> = (0..600).collect();
    let statuses = state.survey(engine, &grips, Some(0)).expect("no engine error");
    let survey: Vec<GripStatus> = statuses.iter().map(|&status| jumble::grip_status(status)).collect();
    let uncertain: Vec<u32> =
        (0u32..).zip(&statuses).filter(|(_, s)| **s == Status::Uncertain).map(|(g, _)| g).collect();
    assert!(!uncertain.is_empty(), "a zero budget leaves a grip uncertain");
    assert_eq!(survey_state(&survey), SurveyState::Pending { uncertain, missing: 0 });
    let cut = full.cut.clone().expect("the named cut");
    assert_eq!((cut.pole, cut.alpha), (cap as u32, (121, 125)), "alpha is the model's, not a mesh-derived value");
    let surveyed = CertOverlay { survey, ..full.clone() };
    validate(&with(&surveyed), &ctx).expect("a pending survey is carried as it is");

    // Row 7: an unsupported projection stage on a jumbling publication names J05; no substitute.
    let stage = Primitive::ChartStage(StageInput { chart: ChartId(1) });
    let unknown = jplan(&base, vec![stage], vec![chart(1, &[("rotate", 6), ("fisheye", 0), ("camera3", 9)])], vec![]);
    let error = validate(&unknown, &ctx).expect_err("an unsupported stage");
    assert_eq!((error.code, error.context["feature"].as_str()), (codes::UNSUPPORTED, "J05"));

    // Row 8: a complete all-scope seam table counts its off-grip mismatches in total N; an interrupted one has none.
    let whole = Batch { chunk: 1 << 12, budget: None, cancel_after: None };
    let facts = seam_facts(engine, &mut state, Scope::All, &whole, Rows::Mismatched, None).expect("seam facts");
    let table = jumble::seam_table(&facts).expect("a listed table");
    let totals = facts.get("totals").expect("totals");
    let count = |field: &str| totals.get(field).and_then(Value::as_i64).expect("a complete batch has totals");
    assert!(table.complete && !table.seams.is_empty(), "the twisted state has seams");
    assert_eq!(table.total(), Partial::Total(count("total_N") as u64), "N is every mismatch, on-grip or not");
    let off_grip = table.seams.iter().filter(|seam| !seam.on_grip).count() as i64;
    assert_eq!((off_grip, count("total_N")), (count("off_grip_mismatches"), count("on_grip_mismatches") + off_grip));
    let identities: std::collections::BTreeSet<(u32, u32)> = table.seams.iter().map(|s| (s.domain, s.cell)).collect();
    assert_eq!(identities.len(), table.seams.len(), "every entry keeps its own domain and cell identity");
    let cut = Batch { chunk: 1, budget: Some(1), cancel_after: None };
    let facts = seam_facts(engine, &mut state, Scope::All, &cut, Rows::Mismatched, None).expect("seam facts");
    assert_eq!(facts.get("totals"), Some(&Value::Null), "the engine claims no total for a cut-off batch");
    let interrupted = jumble::seam_table(&facts).expect("a listed table");
    assert!(!interrupted.complete && matches!(interrupted.total(), Partial::Undefined(_)), "no total N");
    let one_cap =
        seam_facts(engine, &mut state, Scope::Cap(cap as i64), &whole, Rows::Mismatched, None).expect("seam facts");
    assert!(jumble::seam_table(&one_cap).is_none(), "a cap's answer counts its N_c, never total N");

    // Row 15: a certificate point targets its witness piece and grip; off the lattice it has no slot.
    let off = certificate.piece;
    assert!(!state.is_lattice_piece(off as usize), "a straddler is off the lattice");
    let point = ObjectId::Point { witness: off, grip: cap as u32, role: "below".to_owned() };
    let mut host = host_with(&base, ctx);
    let overlay = Primitive::CertOverlay(full.clone());
    install(&mut host, 1, jplan(&base, vec![overlay], vec![], vec![instance(1, 1, point.clone(), 0)]));
    let projection = PoseProjection::new(engine, &state).expect("the level tables place every lattice piece");
    let hit = reply(&host, 1);
    let resolve = |requested| host.resolve_pick(&hit, PickMode::Explicit, requested, &projection).expect("current");
    let Some(Partial::Total(piece)) = resolve(Requested::Piece) else { panic!("the witness piece") };
    assert_eq!((piece.target, piece.occurrence), (ObjectId::Piece(off), point), "the role stays on the occurrence");
    let Some(Partial::Total(grip)) = resolve(Requested::Grip) else { panic!("the named grip") };
    assert_eq!(grip.target, ObjectId::Grip(cap as u32));
    let Some(Partial::Partial { covered, missing }) = resolve(Requested::Slot) else { panic!("off the lattice") };
    assert_eq!((covered.target, missing), (ObjectId::Piece(off), vec!["slot: off_lattice".to_owned()]), "no slot");
    assert!(matches!(resolve(Requested::Sticker), Some(Partial::Undefined(_))), "it is not a sticker hit");
}
