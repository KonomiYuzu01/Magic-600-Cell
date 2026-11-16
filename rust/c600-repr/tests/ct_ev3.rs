//! CT-EV3-06, its counter half, and CT-EV3-07, its view-cannot-write half, for a representation view (EV3.2, EV3.10,
//! EV4.15), over the public APIs of `c600-view`, `c600-command`, `c600-check` and this crate. Counters are counted,
//! never timed. Every result is a source or synthetic fixture outcome.

mod common;

use c600_check::{check, EFFECT_UNDECLARED};
use c600_command::{code, Direction, Kind, Overlay, Status};
use c600_display::plan::{InstanceId, ObjectId};
use c600_ir::Effect;
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
    let parts = view::Mode::JumbleOff
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
    view::BundleCandidate { revision, mode: view::Mode::JumbleOff, parts }
}

/// A view model with one visible representation view of the built-in tool, class `view_aid`, depending on the state.
fn model_with_view(revision: view::Revision) -> (view::ViewModel, view::ViewId) {
    let none = view::Workload::default();
    let mut model = view::ViewModel::new(view::Profile::Solving, profile(), &bundle(revision), none, none)
        .expect("Global and Local fit");
    let v = view::ViewId::new("representation");
    let declared = view::Declared {
        depends: view::Depends::STATE,
        workload: view::Workload { vertex_evals: 1, cpu_bytes: 1, gpu_bytes: 1, ..view::Workload::default() },
        output: view::OutputAllowance::new(BIG, 16),
        class: Some(view::BoundaryClass::ViewAid),
    };
    let source = view::ViewSource::BuiltIn { name: "representations".to_owned() };
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
/// object, the revision, the target's map entries) adds exactly one. Not run: EV3.8 and EV3.9, that an off-lattice
/// relation is distinct from unknown, which wait for M5; and the relationship strip's typed relations, which no crate
/// builds yet, so the relation counted here is this crate's linked selection.
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
