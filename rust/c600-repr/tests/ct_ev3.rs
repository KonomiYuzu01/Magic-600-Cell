//! CT-EV3-06 and CT-EV3-07's view-cannot-write half (EV3.2, EV3.8 to EV3.10, EV4.15), over the public APIs of
//! `c600-view`, `c600-command`, `c600-check`, `c600-display`, `c600-jumble`, `c600-jumble-tool` and this crate.
//! CT-EV3-06 has three parts here: the counter half for a representation view, and the relationship strip's records
//! (`c600_display::strip`) over C8's current occupant on the jumbled puzzle, with the schedule that computes them.
//! Counters are counted, never timed. Every result is a source or synthetic fixture outcome.

mod common;

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use c600_check::{check, EFFECT_UNDECLARED};
use c600_command::{code, Direction, Kind, Overlay, Status};
use c600_display::plan::{InstanceId, Key, Level, Mode, NodeId, NodeKind, ObjectId, RevisionRef, ViewNode};
use c600_display::strip::{self, Extent, Missing, Role, StripRelation};
use c600_ir::Effect;
use c600_jumble::levels;
use c600_jumble::model::{NP, NS};
use c600_jumble::state::{Pose, State};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble_tool::level::Level as RelationLevel;
use c600_jumble_tool::relation::{self as occupant, current_occupants, PIECES_COLLIDE, RELATION};
use c600_kernel::json::{self, Value};
use c600_repr::layout::{linked, Linked, LinkedSelection, Representation};
use c600_repr::tool::{self, ENTRIES, MODULE, ROWS, SOURCE};
use c600_tools::tool::lower_source;
use c600_view as view;

use common::*;

const BIG: u64 = 1_000_000;
const FRAMES: u64 = 60;

/// A complete fixture limit profile: every field generous, `overdraw` 1. Declared values, not measured ones.
fn profile() -> view::LimitProfile {
    let entries: Vec<(&str, u64, &str)> = view::Field::ALL
        .iter()
        .map(|field| (field.name(), if *field == view::Field::Overdraw { 1 } else { BIG }, field.unit()))
        .collect();
    view::LimitProfile::from_entries("fixture-1", &entries).expect("a complete fixture profile")
}

fn rev(state: u64, workspace: u64) -> view::Revision {
    view::Revision::new(1, state, workspace, 1, [state as u8; 32])
}

fn bundle(revision: view::Revision) -> view::BundleCandidate {
    bundle_in(view::Mode::JumbleOff, revision)
}

/// A complete engine bundle of `mode` at `revision`; each part's digest names its kind and the revision's counters.
fn bundle_in(mode: view::Mode, revision: view::Revision) -> view::BundleCandidate {
    let parts = mode
        .required()
        .iter()
        .map(|kind| {
            let mut digest = [0u8; 32];
            digest[0] = *kind as u8;
            digest[1..9].copy_from_slice(&revision.state.to_be_bytes());
            digest[9..17].copy_from_slice(&revision.workspace.to_be_bytes());
            digest[17..25].copy_from_slice(&revision.protection.to_be_bytes());
            view::BundlePart { kind: *kind, revision, digest }
        })
        .collect();
    view::BundleCandidate { revision, mode, parts }
}

/// A view model with one visible representation view of the built-in tool, class `view_aid`, depending on the state.
fn model_with_view(revision: view::Revision) -> (view::ViewModel, view::ViewId) {
    model_in(view::Mode::JumbleOff, revision, "representation", "representations")
}

/// A view model with Jumble `mode` and one visible view `name` of the built-in tool `built_in_name`, class `view_aid`,
/// depending on the state.
fn model_in(
    mode: view::Mode,
    revision: view::Revision,
    name: &str,
    built_in_name: &str,
) -> (view::ViewModel, view::ViewId) {
    let none = view::Workload::default();
    let mut model = view::ViewModel::new(view::Profile::Solving, profile(), &bundle_in(mode, revision), none, none)
        .expect("Global and Local fit");
    let v = view::ViewId::new(name);
    let declared = view::Declared {
        depends: view::Depends::STATE,
        workload: view::Workload { vertex_evals: 1, cpu_bytes: 1, gpu_bytes: 1, ..view::Workload::default() },
        output: view::OutputAllowance::new(BIG, 16),
        class: Some(view::BoundaryClass::ViewAid),
    };
    let source = view::ViewSource::BuiltIn { name: built_in_name.to_owned() };
    model.open_view(view::CommandId::Caller(BIG), view::Origin::Solver, v.clone(), source, declared).expect("it fits");
    (model, v)
}

/// The size descriptor of a representation's publication, as the view model counts it.
fn content(representation: &Representation) -> view::Content {
    let instances = representation.plan.instances.len() as u64;
    view::Content {
        nodes: vec![vec![1, 2, 3]],
        plans: vec![view::PlanDescriptor {
            plan: representation.plan.plan.0,
            source_map: representation.map.map.0,
            instances,
            ..view::PlanDescriptor::default()
        }],
        maps: vec![view::MapDescriptor { map: representation.map.map.0, entries: instances }],
        source_bytes: SOURCE.len() as u64,
    }
}

/// An atlas with two images of sticker 5, one of sticker 6 and one of cell 7, at state revision `state`.
fn atlas(state: u64) -> Representation {
    let base = revision(state);
    let analysed = [ObjectId::Sticker(5), ObjectId::Sticker(6), ObjectId::Cell(7)];
    Representation::layout(
        &header(1, &base),
        &analysed,
        vec![built_in(1)],
        &[
            occurrence(1, ObjectId::Sticker(5), Some(1)),
            occurrence(2, ObjectId::Sticker(5), Some(1)),
            occurrence(3, ObjectId::Sticker(6), Some(1)),
            occurrence(4, ObjectId::Cell(7), Some(1)),
        ],
    )
    .expect("an atlas")
}

/// CT-EV3-06 (part, the counter half: unchanged frame draws perform no relation computation; EV3.10). Counted, never
/// timed. In `c600-view`, a visible representation view runs once at open; sixty frames, each reading the frame input
/// and offered the same bundle again, start no run and move none of the view's work counters, a workspace-only
/// revision moves none either for a view that depends on the state, and one new state revision adds exactly one
/// recomputation. In this crate, a linked selection is computed once for its inputs: sixty frame reads and sixty
/// repeated selections of the same object leave the computation count at one, and each changed input (the selected
/// object, the revision, the target's map entries) adds exactly one. The relation counted here is this crate's linked
/// selection; the strip's own relation, with EV3.8 and EV3.9, runs in the two tests that follow.
#[test]
fn ct_ev3_06_counter_half_unchanged_frame_draws_compute_no_relation() {
    let r11 = rev(11, 1);
    let (mut model, v) = model_with_view(r11);
    let shown = atlas(11);
    assert_eq!(model.pump(|_| view::RunOutcome::Published(content(&shown))), 1);
    let before = model.counters(&v).expect("open");
    assert_eq!((before.recomputations, before.program_runs, before.installs), (1, 1, 1));

    for _ in 0..FRAMES {
        let frame = model.frame_input();
        assert_eq!(frame.revision, r11);
        assert!(frame.views.iter().any(|drawn| drawn.view == v), "the view is drawn");
        assert_eq!(model.adopt(&bundle(r11)), Ok(r11), "the same bundle again");
        assert_eq!(model.pump(|_| panic!("an unchanged frame started a run")), 0);
    }
    assert_eq!(model.counters(&v), Some(before), "no counter moved in {FRAMES} unchanged frames");

    // A workspace-only revision is no changed input of a view that depends on the state.
    let workspace_only = rev(11, 2);
    assert_eq!(model.adopt(&bundle(workspace_only)), Ok(workspace_only));
    assert_eq!(model.pump(|_| panic!("a workspace revision started a run")), 0);
    assert_eq!(model.counters(&v), Some(before));

    // A changed input at a named revision: one recomputation, and then frames reuse its result again.
    let r12 = view::Revision::new(1, 12, 2, 1, [12; 32]);
    assert_eq!(model.adopt(&bundle(r12)), Ok(r12));
    assert_eq!(model.pump(|_| view::RunOutcome::Published(content(&atlas(12)))), 1);
    let after = model.counters(&v).expect("open");
    assert_eq!((after.recomputations, after.program_runs), (before.recomputations + 1, before.program_runs + 1));
    for _ in 0..FRAMES {
        model.frame_input();
        assert_eq!(model.pump(|_| panic!("an unchanged frame started a run")), 0);
    }
    assert_eq!(model.counters(&v), Some(after));

    // The linked selection of this crate.
    let sticker = ObjectId::Sticker(5);
    let mut selection = LinkedSelection::new();
    assert_eq!(selection.frame(), None, "nothing is computed before a selection");
    let images = selection.select(&shown, &sticker).clone();
    assert_eq!(images, Linked::Images(vec![InstanceId(1), InstanceId(2)]));
    assert_eq!(images, linked(&shown, &sticker));
    for _ in 0..FRAMES {
        assert_eq!(selection.frame(), Some(&images));
        assert_eq!(selection.select(&shown, &sticker), &images, "the same inputs are already computed");
    }
    assert_eq!((selection.computations(), selection.frames()), (1, FRAMES + 1));

    // Each changed input computes once.
    assert_eq!(selection.select(&shown, &ObjectId::Cell(7)), &Linked::Images(vec![InstanceId(4)]));
    assert_eq!(selection.computations(), 2, "another object");
    assert_eq!(selection.select(&atlas(12), &ObjectId::Cell(7)), &Linked::Images(vec![InstanceId(4)]));
    assert_eq!(selection.computations(), 3, "another revision");
    let filtered = atlas(12).filtered(|o| o.object != ObjectId::Cell(7)).expect("filtered");
    assert_eq!(selection.select(&filtered, &ObjectId::Cell(7)), &Linked::NoImage);
    assert_eq!(selection.computations(), 4, "other map entries");
    for _ in 0..FRAMES {
        assert_eq!(selection.frame(), Some(&Linked::NoImage));
    }
    assert_eq!(selection.computations(), 4);
}

/// The jumbling model (M5), from the repository's assets.
fn engine() -> &'static c600_jumble::Context {
    static ENGINE: OnceLock<c600_jumble::Context> = OnceLock::new();
    ENGINE.get_or_init(|| c600_jumble::Context::load(&assets()).expect("the jumbling model loads"))
}

/// The pinned I_a fixture of the jumbling research: its menu and its journal.
fn wj_i_a() -> &'static Value {
    static DOC: OnceLock<Value> = OnceLock::new();
    DOC.get_or_init(|| {
        let repository = assets().parent().expect("the repository").to_path_buf();
        let bytes = std::fs::read(repository.join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
        json::parse(&bytes).expect("the I_a fixture is JSON")
    })
}

fn i_a() -> Arc<Menu> {
    Arc::new(Menu::from_record(engine(), wj_i_a().get("menu").expect("a menu")).expect("the pinned I_a menu"))
}

/// The state after the first `count` records of the I_a journal from solved, each an admissible twist.
fn after(menu: &Arc<Menu>, count: usize) -> State {
    let records = wj_i_a().get("journal").and_then(|journal| journal.get("records")).and_then(Value::as_array);
    let mut state = State::new(Some(menu.clone()));
    for record in &records.expect("the journal's records")[..count] {
        let twist = Twist::from_record(engine(), record).expect("a journal record");
        let outcome = state.apply(engine(), &twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, c600_jumble::state::Status::Admissible);
    }
    state
}

/// Solved except that a second piece of piece 4's K⁺-orbit sits at the lattice pose that carries it into piece 4's
/// chamber, which then holds two pieces: a raw pose table (KN2.14) that no twist reaches.
fn crowded_table() -> State {
    let ctx = engine();
    let (kp, regions) = (&ctx.kplus, &ctx.regions);
    let (orbit, to_4) = regions.rep_of(4);
    let b = (0..NP).find(|&p| p != 4 && regions.rep_of(p).0 == orbit).expect("a second piece of the orbit");
    let onto = kp.compose(to_4, kp.inv[regions.rep_of(b).1 as usize]);
    assert_eq!(levels::tables(ctx).expect("the level tables").image(ctx, b, onto), 4);
    let mut poses = vec![Pose::K(0); NP];
    poses[b] = Pose::K(onto);
    levels::state_of(ctx, Some(i_a()), &poses).expect("the raw table reads")
}

/// C8's current occupant of every sticker slot of `state` (`c600_jumble_tool::relation`), carried as the strip record
/// of statement `key` at state revision `number`: each slot an operand, each occupant a piece, each missing value
/// kept with its reason. Its base is the revision whose identity block and pose digest the relation read (KN0.6).
fn strip_record(key: &Key, state: &State, number: u64) -> StripRelation {
    let ctx = engine();
    let slots: Vec<u32> = (0..NS as u32).collect();
    let occupants = current_occupants(ctx, state, &slots).expect("no engine error").expect("slots of the model");
    let base = c600_display::jumble::revision(ctx, state, number);
    assert_eq!((occupants.relation, &occupants.identity, &occupants.read), (RELATION, &base.identity, &base.digest));
    let result = slots
        .iter()
        .map(|&slot| {
            let value = match occupants.get(slot).expect("every slot is named") {
                Ok(piece) => Ok(ObjectId::Piece(piece)),
                Err(occupant::Missing::OffLattice) => Err(Missing::OffLattice),
                Err(occupant::Missing::Unknown(reason)) => Err(Missing::Unknown(reason.to_owned())),
            };
            (ObjectId::Slot(slot), value)
        })
        .collect();
    StripRelation {
        statement: key.clone(),
        relation: occupants.relation.to_owned(),
        operands: Role::Destination,
        values: Role::CurrentOccupant,
        identity: occupants.identity,
        level: match occupants.level {
            RelationLevel::Poses => Level::Poses,
            RelationLevel::Labels => Level::Labels,
        },
        base,
        result,
        evidence: vec!["research/jumbling/fixtures/wj-I_a.json".to_owned()],
        source: None,
    }
}

/// A view node of `kind` at revision `base` whose claims are `claims`, with no content of its own.
fn node(base: &RevisionRef, kind: NodeKind, claims: &[Key]) -> ViewNode {
    ViewNode {
        node: NodeId(1),
        base: base.clone(),
        kind,
        content: Vec::new(),
        plans: Vec::new(),
        children: Vec::new(),
        source: None,
        commands: Vec::new(),
        claims: claims.to_vec(),
    }
}

/// CT-EV3-06, the strip record (EV3.8, EV3.9; RPI.65; KN0.4, KN0.6, KN1.10). C8's current occupant of every sticker
/// slot on the pinned I_a menu, carried as `c600_display::strip` records. Solved, the record is total and each slot
/// answers its home piece. After the first I_a record it is partial: each slot of a piece the twist moved off the
/// lattice answers "none: off the lattice", never the piece that sat there before. On a raw pose table in which a
/// second piece shares piece 4's chamber, the record answers `unknown(reason)` instead, a different value. Every slot
/// keeps its entry, and KN0.4's three forms are told apart. A record is shown only on a relation node of its own
/// revision whose claims carry its statement key; a record that read another model, one that lifts labels into poses
/// and one that answers "none" from labels are refused with `EV3:input.display_plan_invalid`, while a label-level
/// record on the lattice is shown. Not run: EV3.8's other roles, a relation whose value is no canonical object, and a
/// record that a program produces.
#[test]
fn ct_ev3_06_a_strip_record_keeps_none_off_the_lattice_apart_from_unknown() {
    let ctx = engine();
    let menu = i_a();
    let key = Key("ct-ev3-06/current-occupant".to_owned());
    let home = |operand: &ObjectId| match operand {
        ObjectId::Slot(slot) => ObjectId::Piece(ctx.model.slot_piece[*slot as usize]),
        other => panic!("an operand is a slot: {other:?}"),
    };

    let solved = strip_record(&key, &State::new(Some(menu.clone())), 1);
    assert_eq!((solved.extent(), solved.result.len()), (Extent::Total, NS));
    assert!(solved.covered().all(|(slot, piece)| *piece == home(slot)), "solved, each slot holds its home piece");

    let off = strip_record(&key, &after(&menu, 1), 2);
    assert_eq!((off.extent(), off.result.len()), (Extent::Partial, NS), "every slot keeps its entry");
    assert!(off.missing().all(|(_, missing)| *missing == Missing::OffLattice), "nothing is unknown");
    let none: Vec<ObjectId> = off.missing().map(|(slot, _)| slot.clone()).collect();
    assert!(!none.is_empty());
    for slot in &none {
        assert_eq!(solved.result[slot], Ok(home(slot)), "{slot:?}: its occupant at solved");
        assert_eq!(off.result[slot], Err(Missing::OffLattice), "{slot:?}: none, never its old occupant");
    }

    let crowded = strip_record(&key, &crowded_table(), 3);
    let unknown = Missing::Unknown(PIECES_COLLIDE.to_owned());
    assert_eq!(crowded.extent(), Extent::Partial);
    assert!(crowded.missing().all(|(_, missing)| *missing == unknown), "two pieces in one chamber");
    assert_ne!(Missing::OffLattice, unknown, "two different values");

    let mut undefined = off.clone();
    undefined.result.retain(|_, value| value.is_err());
    assert_eq!(undefined.extent(), Extent::Undefined, "nothing covered");
    let mut empty = off.clone();
    empty.result.clear();
    assert_eq!(empty.extent(), Extent::Total, "no operand is a total result with nothing in it");

    // Which node shows which record.
    let shown = node(&off.base, NodeKind::Relation, &[key.clone()]);
    assert_eq!(strip::attach(&shown, &off), Ok(()));
    let mut other_model = off.clone();
    other_model.identity.model_id.push_str("-other");
    other_model.result.clear();
    let mut lifted = off.clone();
    lifted.base.mode = Mode::JumbleOff;
    lifted.result.clear();
    let mut from_labels = off.clone();
    from_labels.level = Level::Labels;
    from_labels.result.retain(|_, value| value.is_err());
    let refusals = [
        ("not a relation node", node(&off.base, NodeKind::Object, &[key.clone()]), &off),
        ("no claim", node(&off.base, NodeKind::Relation, &[]), &off),
        ("another statement", node(&off.base, NodeKind::Relation, &[Key("ct-ev3-06/other".to_owned())]), &off),
        ("the revision of the old occupants", shown.clone(), &solved),
        ("another model", shown.clone(), &other_model),
        ("labels lifted into poses", node(&lifted.base, NodeKind::Relation, &[key.clone()]), &lifted),
        ("none read from labels", shown.clone(), &from_labels),
    ];
    for (what, node, record) in &refusals {
        let error = strip::attach(node, record).expect_err(what);
        assert_eq!(error.code, c600_display::codes::PLAN_INVALID, "{what}");
    }
    let mut labels = solved.clone();
    labels.level = Level::Labels;
    let on_the_lattice = node(&solved.base, NodeKind::Relation, &[key.clone()]);
    assert_eq!(strip::attach(&on_the_lattice, &labels), Ok(()), "on the lattice, labels may be read");
}

/// One run of the strip's view: the strip record of `key` at the state revision the job read, from the state of that
/// number, published as one relation node.
fn strip_run(
    key: &Key,
    states: &BTreeMap<u64, State>,
    computed: &mut Vec<StripRelation>,
    job: &view::Job,
) -> view::RunOutcome {
    let number = job.base.revision.state;
    computed.push(strip_record(key, &states[&number], number));
    let nodes = vec![key.0.as_bytes().to_vec()];
    view::RunOutcome::Published(view::Content { nodes, plans: Vec::new(), maps: Vec::new(), source_bytes: 0 })
}

/// CT-EV3-06, the strip's schedule (EV3.10; KN0.6, KN7.2): a strip relation is computed by its view's run in
/// `c600-view`'s lane, for changed inputs at a named revision, never per frame. A visible strip view of class
/// `view_aid` that depends on the state, on a Jumble-on model, computes C8's current occupant once at open. Sixty
/// frames, each reading the frame input and offered the same bundle again, and a workspace-only revision compute
/// nothing. A burst of two state revisions computes once, at the newer: the record names that revision and equals the
/// record computed there directly, and the superseded revision is never computed. Each revision admits one
/// recomputation, and the newer supersedes the older before a runner takes it, so the burst counts two recomputations
/// and one program run. Sixty more frames compute nothing.
#[test]
fn ct_ev3_06_a_strip_record_is_computed_at_named_revisions_never_per_frame() {
    let menu = i_a();
    let key = Key("ct-ev3-06/current-occupant".to_owned());
    let states = BTreeMap::from([(11, State::new(Some(menu.clone()))), (12, after(&menu, 1)), (13, after(&menu, 2))]);
    let mut computed = Vec::new();
    let r11 = rev(11, 1);
    let (mut model, v) = model_in(view::Mode::JumbleOn, r11, "strip", "strip");
    assert_eq!(model.pump(|job| strip_run(&key, &states, &mut computed, job)), 1);
    assert_eq!(computed.iter().map(|record| record.base.state).collect::<Vec<_>>(), [11]);
    let before = model.counters(&v).expect("open");

    for _ in 0..FRAMES {
        assert_eq!(model.frame_input().revision, r11);
        assert_eq!(model.adopt(&bundle_in(view::Mode::JumbleOn, r11)), Ok(r11), "the same bundle again");
        assert_eq!(model.pump(|_| panic!("an unchanged frame computed the strip relation")), 0);
    }
    let workspace_only = rev(11, 2);
    assert_eq!(model.adopt(&bundle_in(view::Mode::JumbleOn, workspace_only)), Ok(workspace_only));
    assert_eq!(model.pump(|_| panic!("a workspace revision computed the strip relation")), 0);
    assert_eq!((model.counters(&v), computed.len()), (Some(before), 1));

    for state in [12, 13] {
        let revision = rev(state, 2);
        assert_eq!(model.adopt(&bundle_in(view::Mode::JumbleOn, revision)), Ok(revision));
    }
    assert_eq!(model.pump(|job| strip_run(&key, &states, &mut computed, job)), 1, "one run for the burst");
    assert_eq!(computed.iter().map(|record| record.base.state).collect::<Vec<_>>(), [11, 13], "12 is skipped");
    assert_eq!(computed[1], strip_record(&key, &states[&13], 13), "the record of the revision it names");
    let after_burst = model.counters(&v).expect("open");
    assert_eq!(
        (after_burst.recomputations, after_burst.program_runs),
        (before.recomputations + 2, before.program_runs + 1),
        "two admitted, one run"
    );
    for _ in 0..FRAMES {
        model.frame_input();
        assert_eq!(model.pump(|_| panic!("an unchanged frame computed the strip relation")), 0);
    }
    assert_eq!((model.counters(&v), computed.len()), (Some(after_burst), 2));
}

/// CT-EV3-07 (part, the view-cannot-write half; EV3.2, EV4.15). Three layers, each through its public API. The
/// checker: a view that writes a workspace record or prepares a preview is no admitted program, and the shipped view
/// entry has neither effect. The command layer, after a real commit so that an undo target exists: a program's
/// invocation of each of the tool's six rows and of `session.undo` is refused, and a view row registered as an
/// overlay runs to `completed(view_published)` at the revision it was admitted at; through all of it the layer's
/// revision (its workspace number included), the durable log and the undo hint are as they were, and the hint taken
/// before still undoes. The view model: installing the view's publication and drawing frames leaves the adopted
/// revision the one the engine supplied. Not run: that an explicit workspace command joins the durable action order,
/// which is C7's (`c600-durable` is not on this base, and the layer builds none of the tool's workspace rows).
#[test]
fn ct_ev3_07_view_half_a_representation_view_cannot_write_workspace_or_stale_undo() {
    // The checker.
    for (what, tail) in [
        (
            "a view that writes a workspace record",
            "export view writes() -> Unit effects [snapshot, view, workspace] {\n    \
             return write_workspace<Nat>(\"filter.apply\", 1);\n}\n",
        ),
        (
            "a view that prepares a preview",
            "export view proposes(w: Word) -> Text effects [view, preview] {\n    \
             let r = propose(w, target(\"operation.preview\"));\n    return receipt_id(r);\n}\n",
        ),
    ] {
        let program = lower_source(&format!("{SOURCE}\n{tail}")).expect("it lowers");
        let report = check(&program).expect("checked");
        assert_eq!(report.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(), [EFFECT_UNDECLARED], "{what}");
    }
    let shipped = lower_source(SOURCE).expect("the shipped source lowers");
    let report = check(&shipped).expect("checked");
    for entry in ENTRIES.iter().filter(|e| e.kind == c600_ir::FnKind::View) {
        let summary = report.summary(MODULE, entry.name).expect("a summary");
        assert!(!summary.effects.contains(&Effect::ChangeWorkspace), "{}", entry.name);
        assert!(!summary.effects.contains(&Effect::PreparePreview), "{}", entry.name);
        assert!(tool::admit(&shipped, MODULE, entry.name).expect("admitted or refused").is_admitted());
    }

    // The command layer, with one committed action to undo.
    let mut session = session();
    session.commit(&[1, 2]);
    let before = session.fixed();
    let hint = before.2.expect("the commit can be undone");
    assert_eq!(before.0.workspace, 0);

    for (i, row) in ROWS.iter().enumerate() {
        let receipt = session.layer.invoke(&program_channel(), session.runtime_call(&format!("rt/{i}"), row));
        assert_eq!(receipt.status.code(), Some(code::ROW_NOT_BUILT), "{row}: {:?}", receipt.status);
        assert_eq!(session.fixed(), before, "{row}");
    }
    let mut undo = session.runtime_call("rt/undo", "session.undo");
    undo.args = json::object([("hint", hint.to_json())]);
    let receipt = session.layer.invoke(&program_channel(), undo);
    assert_eq!(receipt.status.code(), Some(code::COMMIT_ROW_REFUSED), "{:?}", receipt.status);
    assert_eq!(session.fixed(), before);

    // A view row runs, from the shell and from a program, and writes nothing.
    let overlay = Overlay { module: MODULE.to_owned(), entry: "filter_counts".to_owned() };
    session.layer.register_overlay("tool.representation", Kind::View, &[], overlay).expect("a view overlay row");
    let from_shell =
        session.shell.invoke(&mut session.layer, "tool.representation", json::object([])).expect("prepared");
    let from_program = session.layer.invoke(&program_channel(), session.runtime_call("rt/view", "tool.representation"));
    for receipt in [from_shell, from_program] {
        assert!(matches!(receipt.status, Status::Running(_) | Status::Cancelled(_)), "{:?}", receipt.status);
    }
    let mut finished = 0;
    while let Some(job) = session.layer.take_program_job() {
        let done = session.layer.finish_program_job(job.job, Ok(Value::Int(1))).expect("a receipt");
        let Status::Completed(completion) = &done.status else { panic!("{:?}", done.status) };
        assert_eq!(completion.kind, c600_command::CompletionKind::ViewPublished);
        assert_eq!(completion.revision, before.0, "published at the revision it was admitted at");
        finished += 1;
    }
    assert!(finished >= 1, "a view ran");
    assert_eq!(session.fixed(), before, "a view wrote no workspace state and moved no undo target");

    // The view model: a publication and its frames leave the adopted revision the engine's.
    let r11 = rev(11, 1);
    let (mut model, v) = model_with_view(r11);
    assert_eq!(model.pump(|_| view::RunOutcome::Published(content(&atlas(11)))), 1);
    for _ in 0..FRAMES {
        assert_eq!(model.frame_input().revision, r11);
    }
    assert_eq!(*model.adopted(), r11);
    assert!(model.installed(&v).is_some());

    // Undo is not stale: the hint taken before the views ran still names the action, and undoing it commits.
    assert_eq!(session.layer.undo_hint(Direction::Undo), Some(hint.clone()));
    let receipt = session
        .shell
        .invoke(&mut session.layer, "session.undo", json::object([("hint", hint.to_json())]))
        .expect("prepared");
    assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    assert_eq!(session.layer.snapshot().digest(), kernel().solved().digest());
}
