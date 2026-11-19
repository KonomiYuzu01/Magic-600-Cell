//! CT-EV3-07 through KN5's command layer over M6's journal (`JournalStore`), with C5's provenance reader installed: the
//! solver's explicit workspace command (`workspace.set`) and state commits share one durable action order, which
//! `session.undo` and `session.redo` walk across both kinds, also after a power loss; and a view cannot write
//! workspace state or make undo stale. Each test names the conformance parts it covers with PHS.62 markers.
//!
//! EV3.2's view rows reach the host's view model, `c600-view`'s, through C5's `ModelViews` (LP2's pull request 271).
//! A view's evaluation is not built: the view is a language overlay row of kind `view` (KN5.3), whose program job the
//! test finishes as the host would.

mod common;

use std::sync::{Arc, Mutex};

use c600_command::{
    code, Completion, CompletionKind, Direction, Error as CommandError, Kind, Overlay, Status, ViewSourceRef,
    RECORD_ROW, VIEW_CLOSE_ROW, VIEW_OPEN_ROW,
};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::json::{self, Value};
use c600_session::disk::MemDisk;
use c600_tools::bridge::Bridge;
use c600_tools::views::{model_revision, ModelViews};
use c600_view as view;
use common::layer::{completed, record_args, refused, Fixture};
use common::{manual, JOURNAL};

/// The view row of the CT-EV3-07 view test.
const PANEL: &str = "tool.c7.panel";
/// The source text that stands in for the view row's checked module; its SHA-256 is the module digest of the view's
/// source.
const PANEL_SOURCE: &[u8] = b"tools/c7 panel";
const BIG: u64 = 1 << 40;

/// One `session.undo` or `session.redo` of the action on top of its stack, which must commit. Returns the hint's
/// order and target, the state digest after the step and the record `sheet`'s letters after it.
fn walk(f: &mut Fixture, direction: Direction) -> (u64, u64, String, Option<Vec<i64>>) {
    let hint = f.hint(direction);
    let receipt = f.step(direction, hint);
    assert!(matches!(receipt.status, Status::Committed(_)), "{direction:?} {hint:?}: {:?}", receipt.status);
    let letters = f.layer.record("sheet").and_then(|value| value.letters());
    (hint.order, hint.target, f.layer.revision().digest.clone(), letters)
}

/// CT-EV3-07 (part: the solver's explicit workspace command is a durable action in the session's one order with the
/// state commits: each action takes the next place, and a hint taken before a later action of either kind is
/// superseded; a power loss keeps the order, the records, both stacks and the receipts; `session.undo` steps back
/// through the three actions in their order and `session.redo` forward again, across a second power loss between
/// them, the record's undo removing its value without moving the puzzle and its redo restoring it; not run: H-08's own
/// workspace rows, which the layer does not build), EV4.15, KN5.20, KN5.23, LI.9, LI.26.
#[test]
fn ct_ev3_07_a_workspace_command_and_commits_share_one_undo_order() {
    let disk = MemDisk::new();
    let mut f = Fixture::create(Box::new(disk.clone()));
    let solved = f.layer.revision().digest.clone();
    let first = f.commit(&[1]);
    let one = f.layer.revision().digest.clone();
    let before_write = f.hint(Direction::Undo);
    let set = f.write("sheet", &[1, 2], &manual());
    assert!(completed(&set), "{:?}", set.status);
    let before_second = f.hint(Direction::Undo);
    let second = f.commit(&[2]);
    let two = f.layer.revision().digest.clone();
    let top = f.hint(Direction::Undo);
    let places: Vec<(u64, u64)> = [before_write, before_second, top].iter().map(|h| (h.order, h.target)).collect();
    assert_eq!(places, [(1, 1), (2, 2), (3, 3)]);
    for stale in [before_write, before_second] {
        let receipt = f.step(Direction::Undo, stale);
        assert_eq!(refused(&receipt), Some(code::UNDO_SUPERSEDED), "{stale:?}");
    }
    assert_eq!(f.hint(Direction::Undo), top, "a refusal takes no place in the order");

    let kept = f.kept();
    drop(f);
    disk.crash();
    let mut f = Fixture::open(Box::new(disk.clone()));
    assert_eq!(f.kept(), kept);
    for receipt in [&first, &set, &second] {
        assert_eq!(f.layer.outcome(&receipt.command, &receipt.request).as_ref(), Some(receipt), "{}", receipt.command);
    }

    let sheet = Some(vec![1, 2]);
    let back: Vec<_> = (0..3).map(|_| walk(&mut f, Direction::Undo)).collect();
    let expected = [(3, 3, one.clone(), sheet.clone()), (4, 2, one.clone(), None), (5, 1, solved.clone(), None)];
    assert_eq!(back, expected);
    assert_eq!(f.layer.undo_hint(Direction::Undo), None);

    let kept = f.kept();
    drop(f);
    disk.crash();
    let mut f = Fixture::open(Box::new(disk.clone()));
    assert_eq!(f.kept(), kept);
    let forward: Vec<_> = (0..3).map(|_| walk(&mut f, Direction::Redo)).collect();
    let expected = [(6, 1, one.clone(), None), (7, 2, one, sheet.clone()), (8, 3, two, sheet)];
    assert_eq!(forward, expected);
    assert_eq!(f.layer.undo_hint(Direction::Redo), None);
}

/// CT-EV3-07 (part: a view cannot write workspace state or make undo stale, on the layer over the journal after a
/// commit and a workspace command, with the host's view model behind EV3.2's view rows: the solver opens a view of
/// the language overlay row `tool.c7.panel`, of kind `view`, through the shell, and the view model holds it; the
/// view's run, which the host starts for the view and not inside a command the solver invoked, is refused
/// `workspace.set`, `view.open` and `view.close` (`LG3:capability.not_granted`) and the commit row `session.undo`,
/// and no refused row reaches the view model; the row, invoked on the view's run and by the shell, publishes at the
/// revision it was admitted at; the solver closes the view; none of this writes the journal or moves the revision,
/// the records or the action order, and the undo hint taken before them still undoes the latest action, the
/// workspace command; not run: a view's evaluation, which the test's finished job stands in for; C6's checker leg is
/// `c600-repr`'s `tests/ct_ev3.rs`), EV3.2, EV4.15, KN5.3, KN5.15, ONE.77, RPI.83.
#[test]
fn ct_ev3_07_a_view_cannot_write_the_workspace_or_make_undo_stale() {
    let disk = MemDisk::new();
    let mut f = Fixture::create(Box::new(disk.clone()));
    let panel = Overlay { module: "tools/c7".into(), entry: "panel".into() };
    f.layer.register_overlay(PANEL, Kind::View, &[], panel).expect("a view row");
    f.commit(&[1]);
    assert!(completed(&f.write("sheet", &[1, 2], &manual())));
    let hint = f.hint(Direction::Undo);
    let before = (f.kept(), disk.durable(JOURNAL).expect("a journal"));
    for row in [VIEW_OPEN_ROW, VIEW_CLOSE_ROW] {
        assert_eq!(f.layer.availability(row).map_err(|error| error.code), Err(code::ROW_NOT_BUILT), "{row}");
    }
    let views = install_views(&mut f);
    for row in [VIEW_OPEN_ROW, VIEW_CLOSE_ROW] {
        assert_eq!(f.layer.availability(row).map_err(|error| error.code), Ok(()), "{row}");
    }
    let opened = f.shell.invoke(&mut f.layer, VIEW_OPEN_ROW, open_args("panel")).expect("prepared");
    assert_eq!((opened.job, &opened.status), (None, &Status::Accepted));
    assert_eq!(open_views(&views), ["global", "local", "panel"]);

    let mut bridge = Bridge::new(&mut f.layer, "view-1");
    let write = bridge.invoke(RECORD_ROW, record_args("sheet", &[2, 1], &manual()), None);
    assert_eq!(refused(&write), Some(code::NOT_GRANTED));
    let undo = bridge.invoke("session.undo", json::object([("hint", hint.to_json())]), None);
    assert_eq!(refused(&undo), Some(code::COMMIT_ROW_REFUSED));
    for (row, args) in [(VIEW_OPEN_ROW, open_args("other")), (VIEW_CLOSE_ROW, close_args("panel"))] {
        assert_eq!(refused(&bridge.invoke(row, args, None)), Some(code::NOT_GRANTED), "a view's {row}");
    }
    let from_program = bridge.invoke(PANEL, json::object([]), None);
    publish(&mut f, from_program.status);
    let from_shell = f.shell.invoke(&mut f.layer, PANEL, json::object([])).expect("prepared");
    publish(&mut f, from_shell.status);
    assert_eq!(open_views(&views), ["global", "local", "panel"]);

    let closed = f.shell.invoke(&mut f.layer, VIEW_CLOSE_ROW, close_args("panel")).expect("prepared");
    assert_eq!((closed.job, &closed.status), (None, &Status::Accepted));
    assert_eq!(open_views(&views), ["global", "local"]);
    let reached: Vec<_> = (1..=3).map(|n| views.command(view::CommandId::Caller(n))).collect();
    assert_eq!(reached, [Some(opened.command), Some(closed.command), None], "only the solver's two commands");

    assert_eq!((f.kept(), disk.durable(JOURNAL).expect("a journal")), before, "nothing was written");
    assert_eq!(f.hint(Direction::Undo), hint);
    let undone = f.step(Direction::Undo, hint);
    assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);
    assert_eq!(f.layer.record("sheet"), None, "the workspace command was the latest action");
}

/// Installs the host's view model behind the layer's view rows, at the layer's current revision with Global and Local
/// open, under a fixture profile whose every ceiling is generous. The labels part of its bundle is the state's label
/// digest; the metadata and styles parts are fixture digests bound to the revision. The host serves one source, the
/// overlay row [`PANEL`], as a solver's own view of the state, with no class (BND.27); any other source is refused
/// with `KN5:input.argument_form`, naming the argument.
fn install_views(f: &mut Fixture) -> ModelViews {
    let head = f.layer.revision();
    let revision = model_revision(head).expect("a label digest");
    let part = |kind: view::PartKind, digest: [u8; 32]| view::BundlePart { kind, revision, digest };
    let fixture = |name: &str| sha256(format!("{name} of {}", head.to_json().canonical()).as_bytes());
    let parts = vec![
        part(view::PartKind::Metadata, fixture("metadata")),
        part(view::PartKind::Labels, sha256(head.digest.as_bytes())),
        part(view::PartKind::Styles, fixture("styles")),
    ];
    let candidate = view::BundleCandidate { revision, mode: view::Mode::JumbleOff, parts };
    let entries: Vec<(&str, u64, &str)> =
        view::Field::ALL.iter().map(|field| (field.name(), BIG, field.unit())).collect();
    let profile = view::LimitProfile::from_entries("generous", &entries).expect("a complete fixture profile");
    let none = view::Workload::default();
    let model =
        view::ViewModel::new(view::Profile::Solving, profile, &candidate, none, none).expect("Global and Local fit");
    let views = ModelViews::new(Arc::new(Mutex::new(model)), declare);
    f.layer.set_views(Box::new(views.clone()));
    views
}

fn declare(source: &ViewSourceRef) -> Result<view::Declared, CommandError> {
    match source {
        ViewSourceRef::Overlay { row, .. } if row == PANEL => {
            let workload = view::Workload { vertex_evals: 1, cpu_bytes: 1, gpu_bytes: 1, ..view::Workload::default() };
            let output = view::OutputAllowance::new(BIG, 16);
            Ok(view::Declared { depends: view::Depends::STATE, workload, output, class: None })
        }
        _ => Err(CommandError::new(code::ARGUMENT_FORM, "source")),
    }
}

/// `view.open` of a view `name` of [`PANEL`], with no arguments.
fn open_args(name: &str) -> Value {
    let module = Value::String(sha256_hex(PANEL_SOURCE));
    let row = Value::String(PANEL.into());
    let overlay = json::object([("row", row), ("module", module), ("arguments", json::object([]))]);
    json::object([("view", Value::String(name.into())), ("source", json::object([("overlay", overlay)]))])
}

fn close_args(name: &str) -> Value {
    json::object([("view", Value::String(name.into()))])
}

/// The view model's open views, in open order.
fn open_views(views: &ModelViews) -> Vec<String> {
    let model = views.model().lock().expect("the view model");
    model.views().iter().map(|id| id.as_str().to_owned()).collect()
}

/// Runs the view's program job that `status` admitted, as the host would, and checks its publication: a transient
/// `completed(view_published)` at the revision the job was admitted at.
fn publish(f: &mut Fixture, status: Status) {
    let Status::Running(id) = status else { panic!("the view was not admitted: {status:?}") };
    let job = f.layer.take_program_job().expect("the view's job");
    assert_eq!((job.job, job.row.as_str()), (id, PANEL));
    let admitted = job.revision.clone();
    let published = f.layer.finish_program_job(job.job, Ok(Value::String("a panel".into()))).expect("a receipt");
    let Status::Completed(Completion { kind: CompletionKind::ViewPublished, revision, .. }) = &published.status else {
        panic!("{:?}", published.status)
    };
    assert_eq!((revision, f.layer.revision()), (&admitted, &admitted));
}
