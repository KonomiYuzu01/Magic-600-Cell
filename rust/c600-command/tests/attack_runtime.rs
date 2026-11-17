//! Attack tests on the human-solve boundary, attacker 1: an envelope on a run's channel (`Caller::Runtime`), with no
//! action of the solver behind it. Each test asserts the frozen rule, so it fails while the bypass exists; a test that
//! fails on the attacked tree is `#[ignore]`d with what was observed. Findings file:
//! `kn5-boundary-attack-findings.md` in the integrator's notes folder; the IDs below are its IDs.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, word_draft, Caller, CommandLayer, Direction, Error, HeadlessFrames, Invocation, JobOutput, Kind,
    MemoryStore, Overlay, PreviewState, Provenance, Receipt, RecordValue, Reopen, Secret, Shell, Status, Table,
    RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

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
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), sent: 0 }
}

impl Fixture {
    /// One envelope on the channel of run `run`: no token, a fresh command ID, the current epoch. No command of the
    /// solver started the run: the layer holds no job for it.
    fn runtime(&mut self, run: &str, row: &str, args: Value) -> Receipt {
        self.sent += 1;
        let invocation = Invocation {
            command: format!("{run}/{}", self.sent),
            row: row.to_owned(),
            args,
            token: None,
            epoch: self.layer.revision().epoch,
        };
        self.layer.invoke(&Caller::Runtime { run: run.to_owned() }, invocation)
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
#[test]
#[ignore = "B2: fails on 4c20ce30, the runtime's preview is admitted with no solver command behind it"]
fn b2_a_preview_outside_a_command_the_solver_invoked_is_refused() {
    let mut f = fixture();
    let receipt = f.runtime("stray-run", "operation.preview", draft_args(&word(&[1])));
    assert!(refused(&receipt), "the stray run's preview was answered {:?}", receipt.status);
    assert_eq!(f.layer.run_jobs(), 0);
    assert!(f.layer.preview().is_none());
}

/// B1 (class C, attacker 1). ONE.41 [D], ONE.42 [O], KN5.12 [O], KN5.14 [B]: the commit takes one action on the preview
/// the solver was shown. A run replaces the presented preview of the solver's word with a preview of its own word. One
/// frame later the commit chord, which the solver pressed for their own word, commits the run's word. Both options
/// ONE.42 leaves open refuse this press: under (a) only a solver action ends a shown preview, so the replacement is
/// refused; under (b) the commit key stays dead for a stated time after a new preview is presented, and here it is
/// pressed on the first frame.
///
/// Observed on 4c20ce30: the press answers `committed`, and the head is the end state of the run's word `[2]`.
#[test]
#[ignore = "B1: fails on 4c20ce30, the chord commits the run's word one frame after the swap"]
fn b1_the_commit_chord_does_not_commit_a_word_a_run_swapped_in_one_frame_before() {
    let mut f = fixture();
    let k = kernel();
    let (solvers, runs) = (word(&[1]), word(&[2]));
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
    let swap = f.runtime("tool-run", "operation.preview", draft_args(&runs));
    let Status::Running(theirs) = swap.status else {
        // Option (a) of ONE.42: the replacement is refused, and the solver's preview stays.
        assert!(refused(&swap));
        assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Presented));
        return;
    };
    f.layer.run_jobs();
    // The display's next frame shows the pending preview, now the run's.
    let _ = f.frames.present(&mut f.shell, &mut f.layer, theirs);

    // The solver's chord, pressed for the preview they previewed and read.
    let press = f.shell.press(&mut f.layer, "Ctrl+Shift+Enter", false);
    let committed = matches!(&press, Ok(Some(receipt)) if matches!(receipt.status, Status::Committed(_)));
    assert!(!committed, "the chord committed on the first frame after the swap: {press:?}");
    assert_ne!(f.layer.revision().digest, end_of(&runs), "the head is the end state of the run's word");
}

/// B6 (class G, attacker 4 or a wire that forwards the row; the bridge's proposals cannot name these arguments).
/// ONE.38's table: a preview is cancelled when "the solver cancelled it, or the job that built it was cancelled". The
/// layer lets a run cancel the solver's presented preview: `operation.preview.cancel` is a workspace row with no
/// token, and `runtime_may` lets it through.
///
/// Observed on 4c20ce30: the receipt is `completed(workspace)` and the solver's preview is `Cancelled`.
#[test]
#[ignore = "B6: fails on 4c20ce30, a run cancels the solver's presented preview"]
fn b6_a_run_cannot_cancel_the_solvers_presented_preview() {
    let mut f = fixture();
    let mine = f.solver_preview(&word(&[1]));
    let receipt = f.runtime("tool-run", "operation.preview.cancel", json::object([("preview", Value::Int(mine as i64))]));
    assert!(refused(&receipt), "the run's cancel was answered {:?}", receipt.status);
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Presented));
}

fn output(records: &[(&str, Value)]) -> JobOutput {
    let records = records
        .iter()
        .map(|(name, payload)| ((*name).to_owned(), RecordValue { payload: payload.clone(), provenance: "run".into() }));
    JobOutput { value: Value::Null, views: Vec::new(), records: records.collect() }
}

/// B3 (classes A and F, attacker 1 where a language-defined workspace row is registered). ONE.52 [B]: "A workspace
/// change by a program happens inside a command the solver invokes, as one undoable action"; ONE.36 [D] says the same
/// of a draft write. A run invokes a language-defined workspace row on its own channel. The layer admits the job with
/// no solver action behind it, and its completion is a durable action: a journal transaction, a new workspace
/// revision and a new action order.
///
/// Observed on 4c20ce30: the invocation is `running(1)`, the completion is `completed(workspace)`, the journal gains
/// one transaction whose caller is the run, and the workspace revision and the action order advance by one.
#[test]
#[ignore = "B3: fails on 4c20ce30, a run's own invocation of a workspace row ends in a durable action"]
fn b3_a_run_alone_causes_no_durable_action() {
    let mut f = fixture();
    let overlay = Overlay { module: "notes".into(), entry: "save".into() };
    f.layer.register_overlay("tool.notes.save", Kind::Workspace, &[], overlay).unwrap();
    let (before, journal) = (f.layer.revision().clone(), f.transactions());

    let receipt = f.runtime("stray-run", "tool.notes.save", json::object([]));
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
#[test]
#[ignore = "B4: fails on 4c20ce30, a run's write removes the solver's undo of their own record write"]
fn b4_a_run_cannot_take_the_solvers_record_write_out_of_the_undo_order() {
    let mut f = fixture();
    let overlay = Overlay { module: "notes".into(), entry: "save".into() };
    f.layer.register_overlay("tool.notes.save", Kind::Workspace, &[], overlay).unwrap();
    let solvers = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let args =
        json::object([("payload", solvers.clone()), ("provenance", text("solver")), ("record", text("macro"))]);
    let written = f.shell.invoke(&mut f.layer, RECORD_ROW, args).unwrap();
    assert!(matches!(written.status, Status::Completed(_)), "{:?}", written.status);
    let hint = f.layer.undo_hint(Direction::Undo).expect("the solver's write is undoable");

    let receipt = f.runtime("stray-run", "tool.notes.save", json::object([]));
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
#[test]
#[ignore = "B4: fails on 4c20ce30 with KN5:stale.undo_superseded"]
fn b4_the_undo_key_still_undoes_the_solvers_write_after_a_runs_write() {
    let mut f = fixture();
    let overlay = Overlay { module: "notes".into(), entry: "save".into() };
    f.layer.register_overlay("tool.notes.save", Kind::Workspace, &[], overlay).unwrap();
    let solvers = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let args = json::object([("payload", solvers), ("provenance", text("solver")), ("record", text("macro"))]);
    f.shell.invoke(&mut f.layer, RECORD_ROW, args).unwrap();

    let receipt = f.runtime("stray-run", "tool.notes.save", json::object([]));
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
