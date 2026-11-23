//! Stored rows that a built-in tool's entry serves (draft; LP3's C6, its first common need): `CommandLayer::serve_row`.
//! A served row keeps H-08's shape, and an invocation of it admits a program job for the entry, as for an overlay row.
//! The frozen text is silent on serving, so these tests follow the draft rulings recorded in `src/lib.rs`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, digest, draft_args, Caller, CancelReason, CommandLayer, Completion, CompletionKind, Direction, Error,
    HeadlessFrames, Invocation, JobOutput, Kind, MemoryStore, Overlay, PreviewState, Provenance, Receipt, RecordValue,
    ResultRef, Row, SavedSet, Secret, Shell, Status, Table, TransactionKind, GRIP_REBIND_ROW, MACRO_SAVE_ROW,
    POLICY_ROW, RECORD_ROW, WORKSHEET_COMPARE_ROW, WORKSHEET_LOAD_ROW, WORKSHEET_SAVE_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

const COMPARE: &str = "structure.compare";
const SAVE: &str = "set.save";
const SCOPE: &str = "view.effect.scope";
/// A language-defined workspace row, which takes the `program` lane: H-08's one stored row on that lane,
/// `recording.play`, is the layer's own since LP4-9.
const PLAY: &str = "tool.play";

/// The stored rows the command layer builds itself.
const LAYER_ROWS: [&str; 17] = [
    "operation.preview",
    "operation.execute",
    "operation.preview.cancel",
    "session.undo",
    "session.redo",
    "protection.orbit.set",
    POLICY_ROW,
    RECORD_ROW,
    WORKSHEET_SAVE_ROW,
    WORKSHEET_LOAD_ROW,
    WORKSHEET_COMPARE_ROW,
    GRIP_REBIND_ROW,
    MACRO_SAVE_ROW,
    // LP4-9: capture, playback and conversion are the layer's own.
    "recording.start",
    "recording.stop",
    "recording.play",
    "recording.convert",
];

/// One row per shape condition, each alone on a row the layer could otherwise serve; the first six are served.
const SHAPES: &str = r#"{"commands": [
    {"id": "t.query", "kind": "query", "lane": "none", "undo": "none", "preview": "none"},
    {"id": "t.view", "kind": "view", "lane": "view", "undo": "none", "preview": "none"},
    {"id": "t.file", "kind": "file", "lane": "none", "undo": "none", "preview": "none"},
    {"id": "t.program", "kind": "workspace", "lane": "program", "undo": "none", "preview": "none"},
    {"id": "t.undo.workspace", "kind": "workspace", "lane": "none", "undo": "workspace", "preview": "none"},
    {"id": "t.filter", "kind": "workspace", "lane": "none", "undo": "workspace", "preview": "filter",
     "gates": ["fresh_filter_preview"]},
    {"id": "t.commit", "kind": "commit", "lane": "none", "undo": "none", "preview": "none"},
    {"id": "t.app", "kind": "app", "lane": "none", "undo": "none", "preview": "none"},
    {"id": "t.lane.commit", "kind": "query", "lane": "commit", "undo": "none", "preview": "none"},
    {"id": "t.lane.exclusive", "kind": "query", "lane": "exclusive", "undo": "none", "preview": "none"},
    {"id": "t.lane.latest", "kind": "query", "lane": "latest_wins", "undo": "none", "preview": "none"},
    {"id": "t.undo.journal", "kind": "workspace", "lane": "none", "undo": "journal", "preview": "none"},
    {"id": "t.undo.query", "kind": "query", "lane": "none", "undo": "workspace", "preview": "none"},
    {"id": "t.preview.required", "kind": "query", "lane": "none", "undo": "none", "preview": "required"},
    {"id": "t.preview.filter", "kind": "query", "lane": "none", "undo": "none", "preview": "filter"},
    {"id": "t.filter.query", "kind": "query", "lane": "none", "undo": "none", "preview": "filter",
     "gates": ["fresh_filter_preview"]},
    {"id": "t.filter.gate", "kind": "workspace", "lane": "none", "undo": "workspace", "preview": "none",
     "gates": ["fresh_filter_preview"]},
    {"id": "t.filter.rule", "kind": "workspace", "lane": "none", "undo": "workspace", "preview": "filter"},
    {"id": "t.filter.extra", "kind": "workspace", "lane": "none", "undo": "workspace", "preview": "filter",
     "gates": ["fresh_filter_preview", "confirm"]},
    {"id": "t.filter.noundo", "kind": "workspace", "lane": "none", "undo": "none", "preview": "filter",
     "gates": ["fresh_filter_preview"]},
    {"id": "t.gate", "kind": "query", "lane": "none", "undo": "none", "preview": "none", "gates": ["confirm"]},
    {"id": "t.guard", "kind": "query", "lane": "none", "undo": "none", "preview": "none", "action_guard": "action_hint"},
    {"id": "t.token", "kind": "query", "lane": "none", "undo": "none", "preview": "none",
     "arguments": {"token": "UserActionToken"}}
]}"#;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/representations".into(), entry: entry.into() }
}

fn compare_args(a: i64, b: i64) -> Value {
    json::object([("cells", Value::Array(vec![Value::Int(a), Value::Int(b)]))])
}

fn save_args(name: &str) -> Value {
    json::object([
        ("kind", text("identity")),
        ("members", Value::Array(vec![Value::Int(3), Value::Int(5)])),
        ("name", text(name)),
    ])
}

/// The record `set.save`'s entry writes for `save_args(name)`, in the runtime's value form.
fn entry_record(name: &str) -> RecordValue {
    let int = |n: &str| json::object([("int", text(n))]);
    let members = Value::Array(vec![int("3"), int("5")]);
    let payload = json::object([("kind", text("identity")), ("members", members), ("name", text(name))]);
    RecordValue { payload, provenance: "block".into() }
}

/// The host's provenance reader as the tests stand it in: it accepts every block.
struct Reader;

impl Provenance for Reader {
    fn check(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

fn scope_args(scope: &str) -> Value {
    json::object([("scope", text(scope))])
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn running(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Running(_))
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn rows(table: &Table) -> Vec<Row> {
    table.rows().cloned().collect()
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state that protects nothing, on H-08's table.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    /// Previews, presents and commits `recipe` through the command route.
    fn commit(&mut self, recipe: &Value) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let press = self.shell.prepare(&self.layer, "operation.execute", preview_args(id)).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// Invokes `row` through the command route and returns the receipt.
    fn invoke(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }
}

/// A served query row: before it is served the layer refuses it as not built; served, it keeps H-08's shape, an
/// invocation admits a program job for the entry at the current revision, and the entry's result completes it as a
/// query with a result reference, moving no revision and writing nothing. A retry answers the stored receipt. The
/// layer checks the argument names, as for an overlay row, and the runtime may invoke the row as it may an overlay row
/// (draft).
#[test]
fn a_served_query_row_runs_its_entry_as_a_program_job() {
    let mut f = fixture();
    assert_eq!(code_of(&f.invoke(COMPARE, compare_args(1, 2))), code::ROW_NOT_BUILT);
    assert_eq!(f.layer.availability(COMPARE).unwrap_err().code, code::ROW_NOT_BUILT);

    f.layer.serve_row(COMPARE, overlay("compare_cells")).unwrap();
    let mut stored = Table::h08().row(COMPARE).unwrap().clone();
    stored.overlay = Some(overlay("compare_cells"));
    assert_eq!(f.layer.table().row(COMPARE), Some(&stored));
    assert_eq!(f.layer.availability(COMPARE), Ok(()));

    let before = f.layer.revision().clone();
    let asked = f.shell.prepare(&f.layer, COMPARE, compare_args(1, 2)).unwrap();
    let started = f.shell.send(&mut f.layer, asked.clone());
    let Status::Running(job) = started.status else { panic!("{:?}", started.status) };
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!((taken.job, taken.row.as_str(), taken.kind), (job, COMPARE, Kind::Query));
    assert_eq!((&taken.args, &taken.caller), (&compare_args(1, 2), &Caller::Shell));
    assert_eq!((&taken.overlay, &taken.revision), (&overlay("compare_cells"), &before));
    assert!(f.layer.take_program_job().is_none());

    let result = json::object([("relation", text("adjacent"))]);
    let done = f.layer.finish_program_job(job, Ok(result.clone())).unwrap();
    let reference = ResultRef { job: Some(job), digest: digest(&result) };
    let expected = Completion { kind: CompletionKind::Query, revision: before.clone(), result: Some(reference) };
    assert_eq!(done.status, Status::Completed(expected));
    assert_eq!(f.layer.revision(), &before);
    assert_eq!(f.layer.store().log.len(), 0);
    assert_eq!(f.layer.invoke(&Caller::Shell, asked), done);

    let other = json::object([("cell", Value::Int(1))]);
    assert_eq!(code_of(&f.invoke(COMPARE, other)), code::ARGUMENT_FORM);

    let runtime = Caller::Runtime { run: "run-1".into() };
    let theirs = Invocation {
        command: "run-1/1".into(),
        row: COMPARE.into(),
        args: compare_args(3, 4),
        token: None,
        epoch: before.epoch,
        budget: Some(c600_command::Budget::CEILING),
    };
    let receipt = f.layer.invoke(&runtime, theirs);
    let Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!((taken.job, &taken.caller), (job, &runtime));
}

/// A served workspace row: the entry's output completes it as one durable workspace action, with a result reference,
/// at a revision whose workspace number is one higher and whose other fields are unchanged. For `set.save` the action
/// writes the saved set's record `set/pair` and the reference carries the exact set's digest. H-08 gives `set.save` the
/// undo class `workspace` (LB7.2, since the command table's freeze), so the action is undoable and the next undo
/// targets it, not the commit before it. Serving is not journaled: a reopen keeps the receipt and the workspace
/// revision, and the reopened layer refuses the row as not built until the host serves it again (draft).
#[test]
fn a_served_workspace_row_completes_as_one_durable_action() {
    let mut f = fixture();
    assert!(committed(&f.commit(&word(&[1]))));
    let hint = f.layer.undo_hint(Direction::Undo).expect("the commit");
    f.layer.serve_row(SAVE, overlay("save_set")).unwrap();
    let asked = f.shell.prepare(&f.layer, SAVE, save_args("pair")).unwrap();
    let started = f.shell.send(&mut f.layer, asked.clone());
    let Status::Running(job) = started.status else { panic!("{:?}", started.status) };
    assert_eq!(f.layer.take_program_job().unwrap().job, job);

    let before = f.layer.revision().clone();
    f.layer.set_provenance(Box::new(Reader));
    let records = [(SAVE.to_owned(), entry_record("pair"))].into_iter().collect();
    let output = JobOutput { value: Value::Null, views: Vec::new(), records };
    let done = f.layer.finish_program_job_with(job, Ok(output)).unwrap();
    let Status::Completed(Completion { kind: CompletionKind::Workspace, revision, result }) = &done.status else {
        panic!("{:?}", done.status)
    };
    let mut moved = before.clone();
    moved.workspace += 1;
    assert_eq!((revision, f.layer.revision()), (&moved, &moved));
    let set = SavedSet { name: "pair".into(), kind: "identity", members: vec![3, 5] };
    assert_eq!(result, &Some(ResultRef { job: Some(job), digest: set.digest(&before.identity) }));
    let saved = set.payload(&before.identity, &before);
    assert_eq!(f.layer.record("set/pair").map(|value| &value.payload), Some(&saved));
    let kinds: Vec<TransactionKind> = f.layer.store().log.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [TransactionKind::Commit, TransactionKind::Workspace]);
    let action = f.layer.store().log.last().map(|t| t.order);
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|h| h.target), action, "the save is the next undo's target");
    assert_ne!(action, Some(hint.target));
    assert_eq!(f.layer.invoke(&Caller::Shell, asked.clone()), done);

    let store = f.layer.store().clone();
    let mut g = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), store).unwrap();
    assert_eq!(g.outcome(&asked.command, &done.request), Some(done.clone()));
    assert_eq!(g.invoke(&Caller::Shell, asked), done);
    assert_eq!((g.revision().workspace, g.store().log.len()), (moved.workspace, 2));
    assert_eq!(g.record("set/pair").map(|value| &value.payload), Some(&saved));
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell-2");
    let again = shell.invoke(&mut g, SAVE, save_args("again")).unwrap();
    assert_eq!(code_of(&again), code::ROW_NOT_BUILT);
    g.serve_row(SAVE, overlay("save_set")).unwrap();
    assert!(running(&shell.invoke(&mut g, SAVE, save_args("again")).unwrap()));
}

/// A commit ends a served row's job as it ends an overlay row's (KN5.18): a job taken and a job still queued are both
/// cancelled with `replaced_revision`, a late result is dropped, and nothing is written for it.
#[test]
fn a_commit_ends_a_served_rows_jobs() {
    let mut f = fixture();
    f.layer.serve_row(SAVE, overlay("save_set")).unwrap();
    f.layer.serve_row(COMPARE, overlay("compare_cells")).unwrap();
    let save = f.invoke(SAVE, save_args("pair"));
    let compare = f.invoke(COMPARE, compare_args(1, 2));
    assert!(running(&save) && running(&compare));
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!(taken.row, SAVE);
    assert!(committed(&f.commit(&word(&[1]))));
    for receipt in [&save, &compare] {
        let ended = f.layer.outcome(&receipt.command, &receipt.request).unwrap();
        assert_eq!(ended.status, Status::Cancelled(CancelReason::ReplacedRevision));
    }
    assert!(f.layer.finish_program_job(taken.job, Ok(Value::Null)).is_none());
    assert!(f.layer.take_program_job().is_none());
    assert_eq!((f.layer.store().log.len(), f.layer.revision().workspace), (1, 0));
}

/// CT-LI-12 (part: a newer request of a view row supersedes its waiting one, where the target is the row itself; not
/// run: a target resolved from an argument, a fixed view or a control name, and requests across rows, which the layer
/// does not build).
/// A served view row runs on H-08's `view` lane, keyed by the row: a newer invocation supersedes a waiting one, and
/// the entry's result completes it as published with the revision unchanged and nothing written.
#[test]
fn a_served_view_row_supersedes_its_waiting_invocation() {
    let mut f = fixture();
    f.layer.serve_row(SCOPE, overlay("effect_scope")).unwrap();
    let before = f.layer.revision().clone();
    let first = f.invoke(SCOPE, scope_args("body"));
    let second = f.invoke(SCOPE, scope_args("complete"));
    assert!(running(&first) && running(&second));
    let superseded = f.layer.outcome(&first.command, &first.request).unwrap();
    assert_eq!(superseded.status, Status::Cancelled(CancelReason::Superseded(second.command.clone())));
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!(Some(taken.job), second.job);
    assert!(f.layer.take_program_job().is_none());
    let done = f.layer.finish_program_job(taken.job, Ok(Value::Null)).unwrap();
    let reference = ResultRef { job: Some(taken.job), digest: digest(&Value::Null) };
    let expected =
        Completion { kind: CompletionKind::ViewPublished, revision: before.clone(), result: Some(reference) };
    assert_eq!(done.status, Status::Completed(expected));
    assert_eq!((f.layer.revision(), f.layer.store().log.len()), (&before, 0));
}

/// A served row keeps H-08's lane (KN7.2; draft): a job on the `program` lane, here a language-defined workspace row's,
/// holds the lane to its capacity, while `set.save`, on the `none` lane, has no occupancy limit and is admitted when the
/// program lane is full.
#[test]
fn a_served_row_keeps_its_lane() {
    let mut f = fixture();
    f.layer.register_overlay(PLAY, Kind::Workspace, &[], overlay("play")).unwrap();
    f.layer.serve_row(SAVE, overlay("save_set")).unwrap();
    f.layer.set_program_capacity(1);
    let play = json::object([]);
    assert!(running(&f.invoke(PLAY, play.clone())));
    assert_eq!(f.layer.availability(PLAY).unwrap_err().code, code::LANE_FULL);
    assert_eq!(code_of(&f.invoke(PLAY, play)), code::LANE_FULL);
    assert_eq!(f.layer.availability(SAVE), Ok(()));
    for name in ["a", "b"] {
        assert!(running(&f.invoke(SAVE, save_args(name))), "{name}");
    }
}

/// CT-LG3-02 (part: serving a stored row refuses the commit and app kinds, a token argument and an action guard; the
/// effects of a program are in `c600-check`, `c600-syntax` and `c600-run`).
/// Serving refuses a row whose admission or completion the layer cannot check without the entry, with
/// `KN5:input.overlay_row_kind`: each shape condition alone on a synthetic table, and stored examples. It refuses a row
/// the layer builds itself, a row an entry already serves and an overlay row with
/// `KN5:input.overlay_row_conflict`, and an ID that names no row (a service) with `KN5:input.unknown_row`. Since the
/// command table's freeze H-08 holds `workspace.set` as the layer's drafts give it (CM.17), so it is a row the layer
/// builds itself without the drafts too. Each refusal leaves the table as it was.
#[test]
fn serving_refuses_a_row_the_layer_cannot_check_and_changes_nothing() {
    let mut table = Table::parse(SHAPES.as_bytes()).unwrap();
    for id in ["t.query", "t.view", "t.file", "t.program", "t.undo.workspace", "t.filter"] {
        assert_eq!(table.serve_row(id, overlay(id)).unwrap().overlay, Some(overlay(id)), "{id}");
    }
    let shapes = [
        "t.commit",
        "t.app",
        "t.lane.commit",
        "t.lane.exclusive",
        "t.lane.latest",
        "t.undo.journal",
        "t.undo.query",
        "t.preview.required",
        "t.preview.filter",
        "t.filter.query",
        "t.filter.gate",
        "t.filter.rule",
        "t.filter.extra",
        "t.filter.noundo",
        "t.gate",
        "t.guard",
        "t.token",
    ];
    let stored = [
        "twist.apply",
        "app.exit",
        "session.checkpoint.save",
        "operation.check",
        "jumble.release.check",
        "jumble.scramble",
        "keymap.save",
        "file.keymap.check",
        "file.keymap.apply",
    ];
    let mut h08 = Table::h08().with_drafts();
    let service = h08.services()[0].clone();
    let cases = shapes
        .iter()
        .map(|id| (0, *id, code::OVERLAY_ROW_KIND))
        .chain(stored.iter().map(|id| (1, *id, code::OVERLAY_ROW_KIND)))
        .chain(LAYER_ROWS.into_iter().map(|id| (1, id, code::OVERLAY_ROW_CONFLICT)))
        .chain([(1, "no.such.row", code::UNKNOWN_ROW), (1, service.as_str(), code::UNKNOWN_ROW)]);
    for (which, id, expected) in cases {
        let table = if which == 0 { &mut table } else { &mut h08 };
        let before = rows(table);
        assert_eq!(table.serve_row(id, overlay(id)).unwrap_err().code, expected, "{id}");
        assert_eq!(rows(table), before, "{id}");
    }

    h08.serve_row(COMPARE, overlay("compare_cells")).unwrap();
    h08.register_overlay("tool.full.inspect", Kind::Query, &[], overlay("inspect")).unwrap();
    let before = rows(&h08);
    for id in [COMPARE, "tool.full.inspect"] {
        assert_eq!(h08.serve_row(id, overlay("other")).unwrap_err().code, code::OVERLAY_ROW_CONFLICT, "{id}");
    }
    let replaced = h08.register_overlay(COMPARE, Kind::Query, &[], overlay("other"));
    assert_eq!(replaced.unwrap_err().code, code::OVERLAY_ROW_CONFLICT);
    assert_eq!(rows(&h08), before);

    let mut plain = Table::h08();
    let draft = Table::parse(SHAPES.as_bytes()).unwrap().with_drafts();
    assert!(draft.row(RECORD_ROW).is_some());
    assert_eq!(plain.row(RECORD_ROW), draft.row(RECORD_ROW), "H-08 holds the draft row as it is");
    assert_eq!(plain.serve_row(RECORD_ROW, overlay("record")).unwrap_err().code, code::OVERLAY_ROW_CONFLICT);

    let mut f = fixture();
    let refused = f.layer.serve_row("twist.apply", overlay("twist")).unwrap_err();
    assert_eq!(refused.code, code::OVERLAY_ROW_KIND);
    assert_eq!(f.layer.table().row("twist.apply"), Table::h08().row("twist.apply"));
}

/// The representations tool's six rows (`c600_repr::rows::NEEDS`) can all be served, `filter.apply` with its gate
/// `fresh_filter_preview`, its filter preview rule and its undo class `workspace`; a stored workspace row whose undo
/// class is `workspace` and that has no gate is served too.
#[test]
fn the_representations_rows_can_be_served() {
    let mut table = Table::h08();
    for id in ["filter.preview", SCOPE, SAVE, COMPARE, "structure.layer", "filter.apply", "operation.phase.edit"] {
        let kept = table.row(id).unwrap().clone();
        let served = table.serve_row(id, overlay(id)).unwrap();
        assert_eq!(Row { overlay: None, ..served.clone() }, kept, "{id}");
    }
}

// ----- ONE.49's job rows and LI.22 to LI.24's lanes in the command layer

/// The undo of the action on top of the stack, through the command route.
fn undo(f: &mut Fixture) -> Receipt {
    let hint = f.layer.undo_hint(Direction::Undo).expect("an eligible target");
    f.invoke("session.undo", json::object([("hint", hint.to_json())]))
}

/// `operation.preview` of `recipe` through the command route; returns its receipt and the job building the preview.
fn ask_preview(f: &mut Fixture, recipe: &Value) -> (Receipt, u64) {
    let receipt = f.invoke("operation.preview", draft_args(recipe));
    let Status::Running(job) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
    (receipt, job)
}

fn status_of(f: &Fixture, receipt: &Receipt) -> Status {
    f.layer.outcome(&receipt.command, &receipt.request).expect("a receipt").status
}

/// CT-ONE-12 (part: ONE.49's row for a job building a preview, at a commit and at an undo: the queued job is
/// cancelled with `replaced_revision`, its preview is discarded and never built, and no job is left; with the view
/// job's row: a view row's taken runner and its queued request are cancelled the same way and the runner's late result
/// is dropped; not run: latest-wins jobs, whose one H-08 row, `jumble.release.check`, the layer does not build (LP3's
/// host)), KN5.18, LI.25.
#[test]
fn a_commit_and_an_undo_cancel_a_job_building_a_preview_and_a_view_rows_jobs() {
    for change in ["commit", "undo"] {
        let mut f = fixture();
        f.layer.serve_row(SCOPE, overlay("effect_scope")).unwrap();
        assert!(committed(&f.commit(&word(&[1]))), "{change}");
        let press = if change == "commit" {
            let (_, a) = ask_preview(&mut f, &word(&[2]));
            assert_eq!(f.layer.run_jobs(), 1);
            f.frames.present(&mut f.shell, &mut f.layer, a).unwrap();
            Some(f.shell.prepare(&f.layer, "operation.execute", preview_args(a)).unwrap())
        } else {
            None
        };
        let (building, b) = ask_preview(&mut f, &word(&[3]));
        let first = f.invoke(SCOPE, scope_args("body"));
        let runner = f.layer.take_program_job().expect("the view row's job");
        let waiting = f.invoke(SCOPE, scope_args("complete"));
        assert_eq!(status_of(&f, &first), Status::Cancelled(CancelReason::Superseded(waiting.command.clone())));
        assert!(f.layer.finish_program_job(runner.job, Ok(Value::Null)).is_none(), "{change}");

        let moved = match press {
            Some(press) => f.shell.send(&mut f.layer, press),
            None => undo(&mut f),
        };
        assert!(committed(&moved), "{change}: {:?}", moved.status);
        assert_eq!(status_of(&f, &building), Status::Cancelled(CancelReason::ReplacedRevision), "{change}");
        assert_eq!(f.layer.preview_state(b), Some(PreviewState::Discarded), "{change}");
        assert_eq!(status_of(&f, &waiting), Status::Cancelled(CancelReason::ReplacedRevision), "{change}");
        assert_eq!((f.layer.run_jobs(), f.layer.take_program_job().map(|job| job.job)), (0, None), "{change}");
        assert!(f.layer.preview().is_none(), "{change}");
    }
}

/// CT-LI-03 (part: concurrent requests at once against the five lanes the layer builds: the commit lane refuses a
/// press with `KN5:busy.commit_in_flight` while it is held, the exclusive lane a second preview with
/// `KN7:busy.lane_full` while a preview job is queued, the program lane a request past its capacity, the `none` lane
/// admits a workspace edit meanwhile, and on the view lane a newer request fences the row's taken runner and replaces
/// its pending request, each settled as `superseded` by the newer command; the replacement waits until the fenced
/// runner is reported, the runner's result is dropped, and the replaced request never runs; once released, the commit
/// ends what read the replaced revision; not run: the `latest_wins` lane, whose one H-08 row the layer does not build,
/// and lane targets resolved from an argument, a fixed view or a control name across rows (LP3's host)), KN7.2, LI.22
/// to LI.24.
#[test]
fn concurrent_requests_obey_the_lanes_the_layer_builds() {
    let mut f = fixture();
    f.layer.serve_row(SCOPE, overlay("effect_scope")).unwrap();
    f.layer.register_overlay(PLAY, Kind::Workspace, &[], overlay("play")).unwrap();
    f.layer.serve_row(SAVE, overlay("save_set")).unwrap();
    f.layer.set_program_capacity(1);
    assert_eq!(f.layer.availability("jumble.release.check").unwrap_err().code, code::ROW_NOT_BUILT);

    let (_, a) = ask_preview(&mut f, &word(&[1]));
    assert_eq!(f.layer.run_jobs(), 1);
    f.frames.present(&mut f.shell, &mut f.layer, a).unwrap();
    let play = json::object([]);
    let played = f.invoke(PLAY, play.clone());
    let first = f.invoke(SCOPE, scope_args("body"));
    assert!(running(&played) && running(&first));
    let player = f.layer.take_program_job().expect("the program job");
    let runner = f.layer.take_program_job().expect("the view runner");
    assert_eq!((player.row.as_str(), runner.row.as_str()), (PLAY, SCOPE));

    // Every lane at once.
    let second = f.invoke(SCOPE, scope_args("complete"));
    let third = f.invoke(SCOPE, scope_args("body"));
    let (building, b) = ask_preview(&mut f, &word(&[2]));
    assert_eq!(code_of(&f.invoke("operation.preview", draft_args(&word(&[3])))), code::LANE_FULL);
    assert_eq!(code_of(&f.invoke(PLAY, play)), code::LANE_FULL);
    let saved = f.invoke(SAVE, save_args("a"));
    assert!(running(&second) && running(&third) && running(&saved));
    f.layer.hold_commit_lane(true);
    let press = f.shell.prepare(&f.layer, "operation.execute", preview_args(a)).unwrap();
    assert_eq!(code_of(&f.shell.send(&mut f.layer, press)), code::COMMIT_IN_FLIGHT);

    // The view lane: one runner and one newest pending request; older work is superseded by the newer command.
    assert_eq!(status_of(&f, &first), Status::Cancelled(CancelReason::Superseded(second.command.clone())));
    assert_eq!(status_of(&f, &second), Status::Cancelled(CancelReason::Superseded(third.command.clone())));
    let next = f.layer.take_program_job().expect("the edit's job");
    assert_eq!(next.row, SAVE, "the replacement waits for the fenced runner");
    assert!(f.layer.take_program_job().is_none());
    assert!(f.layer.finish_program_job(runner.job, Ok(Value::Null)).is_none(), "a superseded runner cannot publish");
    assert_eq!(status_of(&f, &first), Status::Cancelled(CancelReason::Superseded(second.command.clone())));
    let replacement = f.layer.take_program_job().expect("the replacement");
    assert_eq!(Some(replacement.job), third.job);
    assert!(f.layer.take_program_job().is_none(), "the replaced request never runs");
    let shown = f.layer.finish_program_job(replacement.job, Ok(Value::Null)).unwrap();
    assert!(matches!(&shown.status, Status::Completed(c) if c.kind == CompletionKind::ViewPublished), "{shown:?}");

    // Released, the press commits, and the jobs that read the replaced revision end.
    f.layer.hold_commit_lane(false);
    let press = f.shell.prepare(&f.layer, "operation.execute", preview_args(a)).unwrap();
    assert!(committed(&f.shell.send(&mut f.layer, press)));
    for receipt in [&building, &played, &saved] {
        assert_eq!(status_of(&f, receipt), Status::Cancelled(CancelReason::ReplacedRevision));
    }
    assert_eq!(f.layer.preview_state(b), Some(PreviewState::Discarded));
    assert!(f.layer.finish_program_job(player.job, Ok(Value::Null)).is_none());
    assert!(f.layer.finish_program_job(next.job, Ok(Value::Null)).is_none());
}
