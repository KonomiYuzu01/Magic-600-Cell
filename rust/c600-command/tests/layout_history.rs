//! ADR-D01's layout history and explore state (owner, 22 November 2026, item 4; ADR-D01.14, ADR-D01.39; draft).
//!
//! A completion of a workspace row whose undo class is `layout` is a step of the layout history, which `layout.undo`
//! and `layout.redo` step and `session.undo` and `session.redo` never reach; the move history's steps are out of the
//! layout rows' reach the same way. A step journals as a workspace transaction, and its undo or redo as an undo or redo
//! transaction whose row names the history, so a reopen rebuilds both histories from the journal. While the explore
//! state is on, every commit row but the two layout rows is refused before its token is spent.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::{EXPLORE_ROW, LAYOUT_REDO_ROW, LAYOUT_UNDO_ROW};
use c600_command::{
    code, draft_args, Caller, CommandLayer, Completion, CompletionKind, Direction, Error, HeadlessFrames, Invocation,
    JobOutput, Kind, MemoryStore, Overlay, Provenance, Receipt, RecordValue, Secret, Shell, Status, Table,
    TransactionKind, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const THEME: &str = "theme.select";
const DENSITY: &str = "density.preset.set";

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

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

fn record(payload: &str) -> RecordValue {
    RecordValue { payload: text(payload), provenance: "block".into() }
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
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

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state that protects nothing, on H-08's table with its drafts, the test reader, and
/// `theme.select` and `density.preset.set` served.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.set_provenance(Box::new(Reader));
    serve(&mut layer);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

fn serve(layer: &mut CommandLayer) {
    let overlay = |entry: &str| Overlay { module: "tools/design".into(), entry: entry.into() };
    layer.serve_row(THEME, overlay("theme")).unwrap();
    layer.serve_row(DENSITY, overlay("density")).unwrap();
}

/// A reopen of `layer`'s store; the host serves the rows again.
fn reopen(layer: &CommandLayer) -> CommandLayer {
    let store = layer.store().clone();
    let mut reopened = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    reopened.set_provenance(Box::new(Reader));
    serve(&mut reopened);
    reopened
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

    /// Invokes a served layout row and completes its job with the one record `name` = `payload`.
    fn step(&mut self, row: &str, args: Value, name: &str, payload: &str) -> Receipt {
        let started = self.shell.invoke(&mut self.layer, row, args).unwrap();
        let Status::Running(job) = started.status else { panic!("{row} not admitted: {:?}", started.status) };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job));
        let records = [(name.to_owned(), record(payload))].into_iter().collect();
        let output = JobOutput { value: Value::Null, views: Vec::new(), records };
        let done = self.layer.finish_program_job_with(job, Ok(output)).unwrap();
        assert!(
            matches!(done.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })),
            "{:?}",
            done.status
        );
        done
    }

    fn theme(&mut self, name: &str, payload: &str) -> Receipt {
        self.step(THEME, json::object([("family", text("dawn"))]), name, payload)
    }

    /// `workspace.set` of `name` = `payload`: a step of the move history (undo class `workspace`).
    fn write(&mut self, name: &str, payload: &str) -> Receipt {
        let args = json::object([("payload", text(payload)), ("provenance", text("block")), ("record", text(name))]);
        self.shell.invoke(&mut self.layer, RECORD_ROW, args).unwrap()
    }

    fn press(&mut self, keys: &str) -> Result<Receipt, Error> {
        self.shell.press(&mut self.layer, keys, false).map(|receipt| receipt.expect("a bound key"))
    }

    fn payload(&self, name: &str) -> Option<Value> {
        self.layer.record(name).map(|value| value.payload.clone())
    }

    fn last_order(&self) -> Option<u64> {
        self.layer.store().log.last().map(|t| t.order)
    }

    fn explore(&mut self, on: bool) -> Receipt {
        self.shell.invoke(&mut self.layer, EXPLORE_ROW, json::object([("on", Value::Bool(on))])).unwrap()
    }
}

/// ADR-D01.14: a design-profile step is the layout history's, not the move history's. `Ctrl+Alt+Z` undoes it and
/// `Ctrl+Alt+Y` redoes it, while `Ctrl+Z` passes over it to the commit before it; each history's redo survives a step of
/// the other. The steps journal as a workspace transaction and as undo and redo transactions whose rows name the
/// history, and a reopen rebuilds both histories.
#[test]
fn a_layout_step_is_undone_in_its_own_history() {
    let mut f = fixture();
    assert!(committed(&f.commit(&word(&[1]))));
    let commit = f.last_order().unwrap();
    assert_eq!(f.layer.layout_hint(Direction::Undo), None);
    f.theme("design/theme", "dawn");
    let step = f.last_order().unwrap();
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|h| h.target), Some(commit), "session.undo passes the step");
    assert_eq!(f.layer.layout_hint(Direction::Undo).map(|h| h.target), Some(step));
    assert_eq!(f.layer.availability(LAYOUT_UNDO_ROW), Ok(()));
    assert_eq!(f.layer.availability(LAYOUT_REDO_ROW).unwrap_err().code, code::UNDO_SUPERSEDED);

    let undone = f.press("Ctrl+Alt+Z").unwrap();
    assert!(committed(&undone), "{:?}", undone.status);
    assert_eq!(f.payload("design/theme"), None);
    assert_eq!(f.layer.layout_hint(Direction::Redo).map(|h| h.target), Some(step));
    assert_eq!(f.layer.layout_hint(Direction::Undo), None);
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|h| h.target), Some(commit));

    assert!(committed(&f.press("Ctrl+Z").unwrap()), "the move history undoes the commit");
    assert_eq!(f.layer.undo_hint(Direction::Redo).map(|h| h.target), Some(commit));
    assert_eq!(f.layer.layout_hint(Direction::Redo).map(|h| h.target), Some(step), "a move undo keeps the layout redo");

    assert!(committed(&f.press("Ctrl+Alt+Y").unwrap()));
    assert_eq!(f.payload("design/theme"), Some(text("dawn")));
    assert_eq!(f.layer.undo_hint(Direction::Redo).map(|h| h.target), Some(commit), "a layout redo keeps the move redo");

    let log = &f.layer.store().log;
    let kinds: Vec<(TransactionKind, &str)> = log.iter().map(|t| (t.kind, t.row.as_str())).collect();
    assert_eq!(
        kinds,
        [
            (TransactionKind::Commit, "operation.execute"),
            (TransactionKind::Workspace, THEME),
            (TransactionKind::Undo, LAYOUT_UNDO_ROW),
            (TransactionKind::Undo, "session.undo"),
            (TransactionKind::Redo, LAYOUT_REDO_ROW),
        ]
    );
    assert_eq!(log[2].target, Some(step));
    assert_eq!(log[2].pre, log[2].post, "a layout undo moves no label");

    let g = reopen(&f.layer);
    for direction in [Direction::Undo, Direction::Redo] {
        assert_eq!(g.layout_hint(direction), f.layer.layout_hint(direction), "{direction:?}");
        assert_eq!(g.undo_hint(direction), f.layer.undo_hint(direction), "{direction:?}");
    }
    assert_eq!(g.record("design/theme").map(|v| v.payload.clone()), Some(text("dawn")));
}

/// Neither history's rows reach the other's steps: with only a layout step, `Ctrl+Z` has no target, and a
/// `session.undo` carrying the layout hint is `KN5:stale.undo_superseded`; with only a commit, a `layout.undo` carrying
/// the move hint is refused the same way. Nothing is written for either refusal.
#[test]
fn neither_history_reaches_the_others_steps() {
    let mut f = fixture();
    f.theme("design/theme", "dawn");
    let layout = f.layer.layout_hint(Direction::Undo).expect("the step");
    assert_eq!(f.layer.undo_hint(Direction::Undo), None);
    assert_eq!(f.press("Ctrl+Z").unwrap_err().code, code::UNDO_SUPERSEDED);
    assert_eq!(f.layer.availability("session.undo").unwrap_err().code, code::UNDO_SUPERSEDED);
    let hint = json::object([("hint", layout.to_json())]);
    let forged = f.shell.invoke(&mut f.layer, "session.undo", hint).unwrap();
    assert_eq!(code_of(&forged), code::UNDO_SUPERSEDED);
    assert_eq!(f.layer.store().log.len(), 1);

    let mut g = fixture();
    assert!(committed(&g.commit(&word(&[1]))));
    let moves = g.layer.undo_hint(Direction::Undo).expect("the commit");
    assert_eq!(g.layer.layout_hint(Direction::Undo), None);
    assert_eq!(g.press("Ctrl+Alt+Z").unwrap_err().code, code::UNDO_SUPERSEDED);
    let hint = json::object([("hint", moves.to_json())]);
    let forged = g.shell.invoke(&mut g.layer, LAYOUT_UNDO_ROW, hint).unwrap();
    assert_eq!(code_of(&forged), code::UNDO_SUPERSEDED);
    assert_eq!(g.layer.store().log.len(), 1);
}

/// A step of one history cuts the other history's actions that wrote one of its records, so no undo writes over the
/// newer step: a move-history write of a record a layout step wrote leaves the layout history without that step, and a
/// layout step that writes a record a move-history write wrote leaves the move history without that write. A step that
/// writes another record cuts nothing.
#[test]
fn a_step_of_one_history_cuts_the_others_writes_of_its_records() {
    let mut f = fixture();
    f.theme("shared", "layout");
    f.theme("design/theme", "dawn");
    let kept = f.last_order().unwrap();
    assert!(matches!(f.write("shared", "moves").status, Status::Completed(_)));
    let write = f.last_order().unwrap();
    assert_eq!(f.layer.layout_hint(Direction::Undo).map(|h| h.target), Some(kept), "the older shared step is cut");
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|h| h.target), Some(write));
    assert!(committed(&f.press("Ctrl+Alt+Z").unwrap()));
    assert_eq!(f.layer.layout_hint(Direction::Undo), None, "the cut step is not reached beneath the kept one");
    assert_eq!(f.payload("shared"), Some(text("moves")));

    let mut g = fixture();
    assert!(matches!(g.write("shared", "moves").status, Status::Completed(_)));
    assert!(matches!(g.write("other", "moves").status, Status::Completed(_)));
    let other = g.last_order().unwrap();
    g.step(DENSITY, json::object([("preset", text("compact"))]), "shared", "layout");
    assert_eq!(g.layer.undo_hint(Direction::Undo).map(|h| h.target), Some(other));
    assert!(committed(&g.press("Ctrl+Z").unwrap()));
    assert_eq!(g.layer.undo_hint(Direction::Undo), None, "the shared write left the move history");
    assert_eq!(g.payload("shared"), Some(text("layout")));
    assert_eq!(g.payload("other"), None);

    let h = reopen(&g.layer);
    for direction in [Direction::Undo, Direction::Redo] {
        assert_eq!(h.layout_hint(direction), g.layer.layout_hint(direction), "{direction:?}");
        assert_eq!(h.undo_hint(direction), g.layer.undo_hint(direction), "{direction:?}");
    }
}

/// A new layout step drops the layout history's redo, as a new action drops the move history's; a move-history step
/// keeps it.
#[test]
fn a_new_layout_step_drops_only_the_layout_redo() {
    let mut f = fixture();
    f.theme("design/theme", "dawn");
    assert!(committed(&f.press("Ctrl+Alt+Z").unwrap()));
    assert!(f.layer.layout_hint(Direction::Redo).is_some());
    assert!(committed(&f.commit(&word(&[1]))));
    assert!(f.layer.layout_hint(Direction::Redo).is_some(), "a commit keeps the layout redo");
    f.theme("design/density", "compact");
    assert_eq!(f.layer.layout_hint(Direction::Redo), None);
    assert!(f.layer.undo_hint(Direction::Undo).is_some(), "the commit stays undoable");
}

/// ADR-D01.39: while the explore state is on, every commit row but the two layout rows is unavailable and refused with
/// `KN5:capability.commits_paused` before its token is spent, so the same press commits once the state is off. A
/// layout undo is not paused, a preview may still be asked, and the state writes nothing and answers `accepted`.
#[test]
fn the_explore_state_pauses_commits_before_their_token_is_spent() {
    let mut f = fixture();
    assert!(committed(&f.commit(&word(&[1]))));
    f.theme("design/theme", "dawn");
    let before = (f.layer.store().log.len(), f.layer.revision().clone());
    let on = f.explore(true);
    assert_eq!((on.job, &on.status), (None, &Status::Accepted));
    assert!(f.layer.explore());
    assert_eq!((f.layer.store().log.len(), f.layer.revision().clone()), before);
    for row in ["session.undo", "operation.execute", "session.new"] {
        assert_eq!(f.layer.availability(row).unwrap_err().code, code::COMMITS_PAUSED, "{row}");
    }

    let hint = f.layer.undo_hint(Direction::Undo).expect("the commit");
    let press = f.shell.prepare(&f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    let paused = f.shell.send(&mut f.layer, press.clone());
    assert_eq!(code_of(&paused), code::COMMITS_PAUSED);
    assert_eq!(f.layer.store().log.len(), before.0);

    let asked = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[2]))).unwrap();
    assert!(matches!(asked.status, Status::Running(_)), "a preview is no commit: {:?}", asked.status);
    assert_eq!(f.layer.availability(LAYOUT_UNDO_ROW), Ok(()));
    assert!(committed(&f.press("Ctrl+Alt+Z").unwrap()), "a layout undo is not paused");
    assert!(committed(&f.press("Ctrl+Alt+Y").unwrap()));

    let off = f.explore(false);
    assert_eq!(off.status, Status::Accepted);
    assert!(!f.layer.explore());
    let hint = f.layer.undo_hint(Direction::Undo).expect("the commit");
    let press = f.shell.prepare(&f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    assert!(committed(&f.shell.send(&mut f.layer, press)), "the token was not spent");
}

/// The refusal comes before authentication, so it spends no token and a forged press gets it too. As for the other
/// refusals there (ruling 4; KN5.21), an authentic press's refusal attaches to its command ID, so a retry answers the
/// same refusal, also after the state is turned off. Nothing is written, and the shell's next press commits.
#[test]
fn a_paused_press_is_answered_like_any_refusal_before_authentication() {
    let mut f = fixture();
    assert!(committed(&f.commit(&word(&[1]))));
    let hint = json::object([("hint", f.layer.undo_hint(Direction::Undo).expect("the commit").to_json())]);
    f.explore(true);
    let mut forger = Shell::new(Secret::fixed([9; 32]), "forger");
    let forged = forger.prepare(&f.layer, "session.undo", hint.clone()).unwrap();
    let refused = f.layer.invoke(&Caller::Shell, forged);
    assert_eq!(code_of(&refused), code::COMMITS_PAUSED);

    let press = f.shell.prepare(&f.layer, "session.undo", hint.clone()).unwrap();
    let paused = f.layer.invoke(&Caller::Shell, press.clone());
    assert_eq!(code_of(&paused), code::COMMITS_PAUSED);
    assert_eq!(f.layer.outcome(&press.command, &paused.request), Some(paused.clone()));
    f.explore(false);
    assert_eq!(f.layer.invoke(&Caller::Shell, press), paused, "a retry never changes its answer");
    assert_eq!(f.layer.store().log.len(), 1);
    let again = f.shell.prepare(&f.layer, "session.undo", hint).unwrap();
    assert!(committed(&f.shell.send(&mut f.layer, again)));
}

/// A run turns the explore state only inside a command the solver invoked (ADR-D01.2's first route): any other run is
/// refused with `LG3:capability.not_granted`. A run inside one may turn it on and off. A restart clears it, and a bad
/// `on` is an argument-form refusal.
#[test]
fn only_a_run_inside_a_solver_command_turns_the_explore_state() {
    let mut f = fixture();
    let invocation = |command: &str, on: bool, epoch: u64| Invocation {
        command: command.into(),
        row: EXPLORE_ROW.into(),
        args: json::object([("on", Value::Bool(on))]),
        token: None,
        epoch,
        budget: None,
    };
    let epoch = f.layer.revision().epoch;
    let outside = f.layer.invoke(&Caller::Runtime { run: "recompute/1".into() }, invocation("r1", true, epoch));
    assert_eq!(code_of(&outside), code::NOT_GRANTED);
    assert!(!f.layer.explore());

    f.layer
        .register_overlay("tool.probe", Kind::Query, &[], Overlay { module: "t".into(), entry: "p".into() })
        .unwrap();
    let probe = f.shell.invoke(&mut f.layer, "tool.probe", json::object([])).unwrap();
    assert!(matches!(probe.status, Status::Running(_)));
    let run = f.layer.take_program_job().expect("the probe's job").run();
    let caller = Caller::Runtime { run: run.clone() };
    assert_eq!(f.layer.invoke(&caller, invocation("p1", true, epoch)).status, Status::Accepted);
    assert!(f.layer.explore());
    assert_eq!(f.layer.invoke(&caller, invocation("p2", false, epoch)).status, Status::Accepted);
    assert!(!f.layer.explore());

    let bad = f.shell.invoke(&mut f.layer, EXPLORE_ROW, json::object([("on", text("yes"))])).unwrap();
    assert_eq!(code_of(&bad), code::ARGUMENT_FORM);
    f.explore(true);
    f.layer.restart(Secret::fixed(SECRET));
    assert!(!f.layer.explore(), "a restart clears the explore state");
}
