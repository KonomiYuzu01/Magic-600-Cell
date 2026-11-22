//! Attack tests on the human-solve boundary, attacker 1: an envelope on a run's channel (`Caller::Runtime`), with no
//! action of the solver behind it. Each test asserts the frozen rule, so it fails while the bypass exists; a test that
//! fails on the attacked tree is `#[ignore]`d with what was observed. Findings file:
//! `kn5-boundary-attack-findings.md` in the integrator's notes folder; the IDs below are its IDs.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use c600_command::{
    code, draft_args, word_draft, Caller, Clock, CommandLayer, Direction, Error, HeadlessFrames, Invocation, JobOutput,
    Kind, MemoryStore, Overlay, PreviewState, Provenance, Receipt, RecordValue, Reopen, Secret, Shell, Status, Table,
    RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
/// A tool's query row, which the solver invokes to run the tool.
const TOOL: &str = "tool.propose";

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

struct Reader;

impl Provenance for Reader {
    fn check(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

/// A clock that never moves: ADR-D01.9's settle interval never ends on it.
struct Frozen;

impl Clock for Frozen {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    sent: u64,
}

fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.set_provenance(Box::new(Reader));
    let overlay = Overlay { module: "tools/propose".into(), entry: "run".into() };
    layer.register_overlay(TOOL, Kind::Query, &[], overlay).unwrap();
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), sent: 0 }
}

impl Fixture {
    /// One envelope on the channel of run `run`: no token, a fresh command ID, the current epoch. No command of the
    /// solver started the run: the layer holds no job for it.
    fn runtime(&mut self, run: &str, row: &str, args: Value) -> Receipt {
        self.sent += 1;
        let served = self.layer.table().row(row).is_some_and(|found| found.overlay.is_some());
        let invocation = Invocation {
            command: format!("{run}/{}", self.sent),
            row: row.to_owned(),
            args,
            token: None,
            epoch: self.layer.revision().epoch,
            budget: served.then_some(c600_command::Budget::CEILING),
        };
        self.layer.invoke(&Caller::Runtime { run: run.to_owned() }, invocation)
    }

    /// A run inside a command the solver invoked: the shell invokes the tool's row, and the host takes the job and runs
    /// it as the job's run (`ProgramJob::run`). The run lasts until the host reports the job. Returns the job and the run.
    fn solver_run(&mut self) -> (u64, String) {
        let receipt = self.shell.invoke(&mut self.layer, TOOL, json::object([])).unwrap();
        let Status::Running(job) = receipt.status else { panic!("the tool's row: {:?}", receipt.status) };
        let program = self.layer.take_program_job().expect("the tool's job");
        assert_eq!((program.job, &program.caller), (job, &Caller::Shell));
        (job, program.run())
    }

    /// The solver previews `recipe` from the editor's draft with the preview key and sees it presented.
    fn solver_preview(&mut self, recipe: &Value) -> u64 {
        self.shell.draft = Some(word_draft(recipe));
        let receipt = self.shell.press(&mut self.layer, "Ctrl+Enter", false).unwrap().unwrap();
        let Status::Running(job) = receipt.status else { panic!("not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, job).unwrap();
        job
    }

    fn transactions(&self) -> usize {
        self.layer.store().clone().history().unwrap().transactions.len()
    }
}

fn refused(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Refused(_))
}

/// B2 (class C, attacker 1). ONE.34 [D]: "A preview is prepared only inside a command the solver invokes". The layer
/// takes `operation.preview` from any run's channel at any time: it holds no job for the run and never asks which
/// command of the solver the run belongs to.
///
/// Observed on 4c20ce30: the receipt is `running(1)`, the job builds preview 1 and the slot holds it.
///
/// The owner's reading of 18 November 2026: one check in the command layer, not in each host. A run prepares a preview
/// only while the host holds the job of a command the solver invoked, and the run is that job's run. A stray run, a run
/// that names a job it is not, and the job's run after the host reported the job are refused with
/// `KN5:capability.outside_solver_command`; the run of the solver's command is admitted. The check is on
/// (`SOLVER_COMMAND_CHECK` in `src/layer.rs`), so a host must start its runs through a tool row the shell invokes.
#[test]
fn b2_a_preview_outside_a_command_the_solver_invoked_is_refused() {
    let mut f = fixture();
    let receipt = f.runtime("stray-run", "operation.preview", draft_args(&word(&[1])));
    assert!(refused(&receipt), "the stray run's preview was answered {:?}", receipt.status);
    assert_eq!(receipt.status.code(), Some(code::OUTSIDE_SOLVER_COMMAND));
    assert_eq!(f.layer.run_jobs(), 0);
    assert!(f.layer.preview().is_none());

    let (job, run) = f.solver_run();
    for other in ["job-0", "job-99", "job-+1", "job-01"] {
        let receipt = f.runtime(other, "operation.preview", draft_args(&word(&[1])));
        assert_eq!(receipt.status.code(), Some(code::OUTSIDE_SOLVER_COMMAND), "{other}");
    }
    let inside = f.runtime(&run, "operation.preview", draft_args(&word(&[1])));
    assert!(matches!(inside.status, Status::Running(_)), "the solver's run: {:?}", inside.status);
    assert_eq!(f.layer.run_jobs(), 1);
    assert!(f.layer.preview().is_some());

    f.layer.finish_program_job(job, Ok(Value::Null)).expect("the tool's job is reported");
    let after = f.runtime(&run, "operation.preview", draft_args(&word(&[2])));
    assert_eq!(after.status.code(), Some(code::OUTSIDE_SOLVER_COMMAND), "{:?}", after.status);
}

/// B1 (class C, attacker 1). ONE.41 [D], ONE.42 [O], KN5.12 [O], KN5.14 [B]: the commit takes one action on the preview
/// the solver was shown. A run replaces the presented preview of the solver's word with a preview of its own word. One
/// frame later the commit chord, which the solver pressed for their own word, commits the run's word. Both options
/// ONE.42 leaves open refuse this press: under (a) only a solver action ends a shown preview, so the replacement is
/// refused; under (b) the commit key stays dead for a stated time after a new preview is presented, and here it is
/// pressed on the first frame.
///
/// Observed on 4c20ce30: the press answers `committed`, and the head is the end state of the run's word `[2]`.
///
/// Fixed under option (a) on 18 November 2026, when the run's preview was refused. ADR-D01.9 (owner, 22 November 2026,
/// item 4) answers KN5.12 with a version of option (b): the run of a command the solver invoked replaces the shown
/// preview, the replacement names the preview it replaced, and the commit key ignores a press on it for a settle
/// interval from the first frame that shows it. The chord on that first frame is `KN5:busy.preview_settling`, journals
/// nothing, and the head stays at the solved state. The clock here never moves, so the interval never ends.
#[test]
fn b1_the_commit_chord_does_not_commit_a_word_a_run_swapped_in_one_frame_before() {
    let mut f = fixture();
    f.layer.set_clock(Box::new(Frozen));
    let k = kernel();
    let (solvers, runs) = (word(&[1]), word(&[2]));
    let (_, run) = f.solver_run();
    let mine = f.solver_preview(&solvers);
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Presented));
    let end_of = |recipe: &Value| {
        let recipe = k.recipe(recipe).unwrap();
        match k.derive(&k.solved(), &recipe, &k.protected_set(&[]).unwrap()).unwrap() {
            c600_kernel::Derivation::Derived(d) => d.state.digest().to_owned(),
            c600_kernel::Derivation::Refused(_) => panic!("the word derives"),
        }
    };

    // The run's proposal, on its own channel, while the solver looks at their preview.
    let swap = f.runtime(&run, "operation.preview", draft_args(&runs));
    let Status::Running(theirs) = swap.status else { panic!("the run's replacement: {:?}", swap.status) };
    f.layer.run_jobs();
    assert_eq!(f.layer.preview().map(|p| (p.id, p.replaced)), Some((theirs, Some(mine))), "the mark");
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Replaced));
    // The display's next frame shows the pending preview, now the run's.
    f.frames.present(&mut f.shell, &mut f.layer, theirs).unwrap();

    // The solver's chord, pressed for the preview they previewed and read.
    let journal = f.transactions();
    let press = f.shell.press(&mut f.layer, "Ctrl+Shift+Enter", false);
    let committed = matches!(&press, Ok(Some(receipt)) if matches!(receipt.status, Status::Committed(_)));
    assert!(!committed, "the chord committed on the first frame after the swap: {press:?}");
    assert!(matches!(&press, Ok(Some(r)) if r.status.code() == Some(code::PREVIEW_SETTLING)), "{press:?}");
    assert_eq!(f.transactions(), journal);
    assert_ne!(f.layer.revision().digest, end_of(&runs), "the head is the end state of the run's word");
}

/// B1 when the timing differs: the run's preview is admitted while the solver's preview is ready and not yet shown, and a
/// frame presents the solver's preview before the run's job builds. Under ADR-D01.9 the job still builds and replaces
/// the shown preview: the replacement takes its new ID when it builds and names the preview the solver saw, the
/// solver's preview ends, and the chord pressed for it commits nothing. Once a frame shows the replacement, the chord
/// on it waits out the settle interval.
#[test]
fn b1_a_runs_preview_admitted_before_the_frame_replaces_the_shown_preview_with_the_mark() {
    let mut f = fixture();
    f.layer.set_clock(Box::new(Frozen));
    let (_, run) = f.solver_run();
    f.shell.draft = Some(word_draft(&word(&[1])));
    let receipt = f.shell.press(&mut f.layer, "Ctrl+Enter", false).unwrap().unwrap();
    let Status::Running(mine) = receipt.status else { panic!("not admitted: {:?}", receipt.status) };
    assert_eq!(f.layer.run_jobs(), 1);
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Ready));

    // The run's preview is admitted: the solver's preview is not shown yet.
    let swap = f.runtime(&run, "operation.preview", draft_args(&word(&[2])));
    let Status::Running(theirs) = swap.status else { panic!("the run's preview: {:?}", swap.status) };
    // The display's next frame shows the solver's preview; then the run's job builds.
    f.frames.present(&mut f.shell, &mut f.layer, mine).unwrap();
    assert_eq!(f.layer.run_jobs(), 1);

    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Replaced));
    let pending = f.layer.preview().expect("the run's preview is pending");
    assert_eq!((pending.id, pending.state, pending.replaced), (theirs, PreviewState::Ready, Some(mine)));
    let (journal, digest) = (f.transactions(), f.layer.revision().digest.clone());
    let press = f.shell.press(&mut f.layer, "Ctrl+Shift+Enter", false);
    assert!(matches!(&press, Ok(Some(r)) if refused(r)), "the chord for the solver's ended preview: {press:?}");
    f.frames.present(&mut f.shell, &mut f.layer, theirs).unwrap();
    let press = f.shell.press(&mut f.layer, "Ctrl+Shift+Enter", false);
    assert!(matches!(&press, Ok(Some(r)) if r.status.code() == Some(code::PREVIEW_SETTLING)), "{press:?}");
    assert_eq!((f.transactions(), &f.layer.revision().digest), (journal, &digest));
}

/// B6 (class G, attacker 4 or a wire that forwards the row; the bridge's proposals cannot name these arguments).
/// ONE.38's table: a preview is cancelled when "the solver cancelled it, or the job that built it was cancelled". The
/// layer lets a run cancel the solver's presented preview: `operation.preview.cancel` is a workspace row with no
/// token, and `runtime_may` lets it through.
///
/// Observed on 4c20ce30: the receipt is `completed(workspace)` and the solver's preview is `Cancelled`.
///
/// Fixed (finding L17's reading of 18 November 2026: only the solver's action ends a shown preview): a run's cancel of
/// the shown preview is refused with `KN5:capability.preview_shown`, from a stray run and from the run of a command the
/// solver invoked, and the solver's own cancel still ends it.
#[test]
fn b6_a_run_cannot_cancel_the_solvers_presented_preview() {
    let mut f = fixture();
    let (_, run) = f.solver_run();
    let mine = f.solver_preview(&word(&[1]));
    let args = json::object([("preview", Value::Int(mine as i64))]);
    for caller in ["tool-run", run.as_str()] {
        let receipt = f.runtime(caller, "operation.preview.cancel", args.clone());
        assert!(refused(&receipt), "{caller}'s cancel was answered {:?}", receipt.status);
        assert_eq!(receipt.status.code(), Some(code::PREVIEW_SHOWN), "{caller}");
        assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Presented));
    }
    let own = f.shell.invoke(&mut f.layer, "operation.preview.cancel", args).unwrap();
    assert!(matches!(own.status, Status::Completed(_)), "the solver's cancel: {:?}", own.status);
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Cancelled));
}

fn output(records: &[(&str, Value)]) -> JobOutput {
    let records = records.iter().map(|(name, payload)| {
        ((*name).to_owned(), RecordValue { payload: payload.clone(), provenance: "run".into() })
    });
    JobOutput { value: Value::Null, views: Vec::new(), records: records.collect() }
}

/// The envelope below is the one `c600_tools::bridge::Bridge` sends for a program's `propose(word, target(row))`: the
/// row the program names, the arguments `{"draft": {"recipe": [...]}}`, no token, the run's channel. The bridge
/// filters no row, and the row here declares a `draft` argument, so the layer's argument check passes.
///
/// B3 (classes A and F, attacker 1 where a language-defined workspace row is registered). ONE.52 [B]: "A workspace
/// change by a program happens inside a command the solver invokes, as one undoable action"; ONE.36 [D] says the same
/// of a draft write. A run invokes a language-defined workspace row on its own channel. The layer admits the job with
/// no solver action behind it, and its completion is a durable action: a journal transaction, a new workspace
/// revision and a new action order.
///
/// Observed on 4c20ce30: the invocation is `running(1)`, the completion is `completed(workspace)`, the journal gains
/// one transaction whose caller is the run, and the workspace revision and the action order advance by one.
///
/// Fixed: a run reaches no workspace-kind row, so the layer refuses the invocation with `LG3:capability.not_granted`,
/// as it refuses `workspace.set`, and admits no job.
#[test]
fn b3_a_run_alone_causes_no_durable_action() {
    let mut f = fixture();
    let overlay = Overlay { module: "notes".into(), entry: "save".into() };
    f.layer.register_overlay("tool.notes.save", Kind::Workspace, &[("draft", "Draft")], overlay).unwrap();
    let (before, journal) = (f.layer.revision().clone(), f.transactions());

    let receipt = f.runtime("stray-run", "tool.notes.save", draft_args(&word(&[1])));
    assert_eq!(receipt.status.code(), Some(code::NOT_GRANTED), "{:?}", receipt.status);
    if let Status::Running(job) = receipt.status {
        let program = f.layer.take_program_job().expect("the job the run started");
        assert_eq!(program.job, job);
        f.layer.finish_program_job_with(job, Ok(output(&[("note", text("written by the run"))])));
    }
    assert_eq!(f.transactions(), journal, "the journal gained a transaction with no solver action");
    assert_eq!(f.layer.revision(), &before, "the revision moved with no solver action");
    assert!(f.layer.record("note").is_none(), "the run's record is in the workspace");
}

/// B4 (class F, attacker 1 where a language-defined workspace row is registered). KN5.19 [B], ONE.50 [B]: one undo key
/// and one durable order across the journal and the workspace. ONE.52 [B]: a program's workspace change is one
/// undoable action inside a command the solver invokes. The solver writes record `macro` and can undo the write. A
/// run then invokes a workspace row whose job writes `macro`. The solver's write leaves the undo stack (the layer's
/// "history cut"), so the solver's undo key has no target and the solver's letters cannot come back; the run's letters
/// stay, and no undo reaches them.
///
/// Observed on 4c20ce30: after the run's job `undo_hint(Undo)` is `None`, the record holds the run's letters at
/// version 2, and `Ctrl+Z` answers `KN5:stale.undo_superseded` ("no eligible target").
///
/// Fixed with B3: the run's invocation is refused, so no job writes `macro` and the history is not cut.
#[test]
fn b4_a_run_cannot_take_the_solvers_record_write_out_of_the_undo_order() {
    let mut f = fixture();
    let overlay = Overlay { module: "notes".into(), entry: "save".into() };
    f.layer.register_overlay("tool.notes.save", Kind::Workspace, &[("draft", "Draft")], overlay).unwrap();
    let solvers = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let args = json::object([("payload", solvers.clone()), ("provenance", text("solver")), ("record", text("macro"))]);
    let written = f.shell.invoke(&mut f.layer, RECORD_ROW, args).unwrap();
    assert!(matches!(written.status, Status::Completed(_)), "{:?}", written.status);
    let hint = f.layer.undo_hint(Direction::Undo).expect("the solver's write is undoable");

    let receipt = f.runtime("stray-run", "tool.notes.save", draft_args(&word(&[1])));
    if let Status::Running(job) = receipt.status {
        f.layer.take_program_job().expect("the job the run started");
        let theirs = Value::Array(vec![Value::Int(3)]);
        f.layer.finish_program_job_with(job, Ok(output(&[("macro", theirs)])));
    }

    let now = f.layer.undo_hint(Direction::Undo);
    assert_eq!(now.map(|hint| hint.target), Some(hint.target), "the solver's write left the undo stack");
    assert_eq!(f.layer.record("macro").map(|value| &value.payload), Some(&solvers), "the run's letters replaced it");
}

/// B4, the solver's side of it: after the run's write the undo key finds nothing to undo.
///
/// Fixed with B3: the undo key undoes the solver's write.
#[test]
fn b4_the_undo_key_still_undoes_the_solvers_write_after_a_runs_write() {
    let mut f = fixture();
    let overlay = Overlay { module: "notes".into(), entry: "save".into() };
    f.layer.register_overlay("tool.notes.save", Kind::Workspace, &[("draft", "Draft")], overlay).unwrap();
    let solvers = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let args = json::object([("payload", solvers), ("provenance", text("solver")), ("record", text("macro"))]);
    f.shell.invoke(&mut f.layer, RECORD_ROW, args).unwrap();

    let receipt = f.runtime("stray-run", "tool.notes.save", draft_args(&word(&[1])));
    if let Status::Running(job) = receipt.status {
        f.layer.take_program_job().expect("the job the run started");
        f.layer.finish_program_job_with(job, Ok(output(&[("macro", Value::Array(vec![Value::Int(3)]))])));
    }
    let undo = f.shell.press(&mut f.layer, "Ctrl+Z", false);
    let undone = matches!(&undo, Ok(Some(receipt)) if matches!(receipt.status, Status::Committed(_)));
    assert!(undone, "the undo key answered {undo:?}");
}

/// Guard (refuted, class A, attacker 1). KN5.15 [D], KN5.3 [D]: a run reaches no commit row, with or without a token it
/// took from the solver's own press, and its refusal never spends the solver's token or blocks the solver's command ID
/// (ruling 4).
#[test]
fn a_run_reaches_no_commit_row_and_spends_no_press() {
    let mut f = fixture();
    let mine = f.solver_preview(&word(&[1]));
    let press = f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(mine as i64))]));
    let press = press.unwrap();
    let journal = f.transactions();

    // The run replays the solver's whole envelope, token included, on its own channel, and then every commit row.
    let caller = Caller::Runtime { run: "tool-run".into() };
    let stolen = f.layer.invoke(&caller, press.clone());
    assert_eq!(stolen.status.code(), Some(code::COMMIT_ROW_REFUSED));
    for row in ["operation.execute", "twist.apply", "session.undo", "session.redo", "jumble.on", "jumble.off"] {
        let receipt = f.runtime("tool-run", row, json::object([]));
        assert_eq!(receipt.status.code(), Some(code::COMMIT_ROW_REFUSED), "{row}");
    }
    for row in ["protection.orbit.set", "protection.policy.set", RECORD_ROW, "worksheet.save", "worksheet.load"] {
        let receipt = f.runtime("tool-run", row, json::object([]));
        assert_eq!(receipt.status.code(), Some(code::NOT_GRANTED), "{row}");
    }
    assert_eq!(f.transactions(), journal);

    // The solver's press still commits: the run consumed neither its token nor its command ID.
    let receipt = f.shell.send(&mut f.layer, press);
    assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    assert_eq!(f.transactions(), journal + 1);
}
