//! Attack tests on the caller stamp and the command IDs (owner-requested attack of the command layer, class G: caller
//! confusion). The host stamps the caller from the channel, and the envelope names none, so no field of an envelope
//! claims the shell (`tests/attack_runtime.rs` shows that a run's replay of the solver's own envelope, token included,
//! is refused). What is left is the command ID: the retry lookup keeps receipts under the caller's scope and the
//! command ID. On commit 4c20ce30 the scope was the caller's kind (`shell` or `runtime`); since the owner's reading of
//! finding G1 (18 November 2026) it is the shell's, or the run's own.
//!
//! Each test asserts the rule; its comment gives the finding and what was observed on commit 4c20ce30. A run prepares a
//! preview only inside a command the solver invoked (finding B2), so the runs here are the runs of tool jobs the shell
//! invoked.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use c600_command::{
    draft_args, Caller, CommandLayer, HeadlessFrames, Invocation, Kind, MemoryStore, Overlay, Receipt, Secret, Shell,
    Status, Table,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
/// A tool's query row, which the solver invokes to run the tool.
const TOOL: &str = "tool.propose";

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    Arc::clone(KERNEL.get_or_init(|| {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
        Arc::new(Kernel::load(&assets).unwrap_or_else(|e| panic!("the model in assets/ does not load: {e}")))
    }))
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

/// A session at the solved state that protects nothing, on H-08's table, with a tool's row.
fn layer() -> CommandLayer {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let overlay = Overlay { module: "tools/propose".into(), entry: "run".into() };
    layer.register_overlay(TOOL, Kind::Query, &[], overlay).unwrap();
    layer
}

/// The solver invokes the tool, and the host takes its job: returns the job and its run (`ProgramJob::run`).
fn solver_run(layer: &mut CommandLayer, shell: &mut Shell) -> (u64, String) {
    let receipt = shell.invoke(layer, TOOL, json::object([])).unwrap();
    let Status::Running(job) = receipt.status else { panic!("the tool's row: {:?}", receipt.status) };
    let program = layer.take_program_job().expect("the tool's job");
    assert_eq!((program.job, &program.caller), (job, &Caller::Shell));
    (job, program.run())
}

/// One preview envelope on `run`'s channel under the command ID `command`, with no token.
fn preview(layer: &mut CommandLayer, run: &str, command: &str, letters: &[i64]) -> Receipt {
    let invocation = Invocation {
        command: command.to_owned(),
        row: "operation.preview".to_owned(),
        args: draft_args(&word(letters)),
        token: None,
        epoch: layer.revision().epoch,
        budget: None,
    };
    layer.invoke(&Caller::Runtime { run: run.to_owned() }, invocation)
}

fn answer(receipt: &Receipt) -> String {
    match &receipt.status {
        Status::Refused(error) | Status::Failed(error) => error.code.to_owned(),
        other => format!("{other:?}"),
    }
}

/// Finding G1 (class G, attacker 4: the bridge stamps its own IDs `<run>/<n>`, so a script chooses none; a host that
/// names runs so that two IDs meet, as `a` at its 11th call and `a/1` at its first, reaches it too). Rule: KN5.21 [D]
/// "The same ID with different content is refused ... without changing the original invocation's record", read with
/// the layer's own words that a command ID is recorded for the caller (`invoke`, `src/layer.rs`). All runs share the
/// one scope `runtime`, so a command ID that one run used is used up for every other run.
///
/// Observed on 4c20ce30: after run `run-a` sends a preview under the ID `run-b/1`, the first envelope of run `run-b`,
/// which its bridge stamps `run-b/1`, is refused with `KN5:input.command_id_reused`. Nothing durable changes; one run
/// stops another run's first command.
///
/// Fixed (the owner's reading of 18 November 2026, `docs/wiki/decisions/owner-decisions-2026-11-18.md`: a command ID
/// lives in its caller's namespace): the scope is the run's own, so the squat under the other run's ID is recorded for
/// the squatter only. The two runs here are the runs of two tool jobs the solver invoked, which run at once.
#[test]
fn g1_one_run_cannot_use_up_another_runs_command_id() {
    let mut layer = layer();
    layer.set_program_capacity(2);
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let (_, run_a) = solver_run(&mut layer, &mut shell);
    let (_, run_b) = solver_run(&mut layer, &mut shell);
    let theirs = format!("{run_b}/1");
    let squat = preview(&mut layer, &run_a, &theirs, &[1]);
    assert!(matches!(squat.status, Status::Running(_)), "{:?}", squat.status);
    assert_eq!(layer.run_jobs(), 1, "the squatter's preview is built, so the exclusive lane is free again");
    let own = preview(&mut layer, &run_b, &theirs, &[2]);
    assert!(
        matches!(own.status, Status::Running(_)),
        "{run_b}'s first command, under its own ID `{theirs}`, was answered {}",
        answer(&own)
    );
    // Within one run's namespace the rule of KN5.21 stands: the same ID with different content is refused.
    let again = preview(&mut layer, &run_b, &theirs, &[3]);
    assert_eq!(answer(&again), "KN5:input.command_id_reused");
}

/// Guard (class G, refuted): a run cannot use up the shell's command IDs. A run sends previews under the IDs the
/// shell is about to use; the solver's own preview, presentation and commit press are then answered as if the run had
/// sent nothing, because the shell's scope is its own.
#[test]
fn a_run_cannot_use_up_the_shells_command_ids() {
    let mut layer = layer();
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let (job, run) = solver_run(&mut layer, &mut shell);
    for n in 0..40 {
        for id in [format!("shell/{n}"), format!("shell-{n}"), format!("shell{n}"), n.to_string()] {
            let _ = preview(&mut layer, &run, &id, &[1]);
        }
    }
    layer.run_jobs();
    layer.finish_program_job(job, Ok(Value::Null)).expect("the tool's job is reported");
    let mut frames = HeadlessFrames::new();
    let shown = shell.invoke(&mut layer, "operation.preview", draft_args(&word(&[2]))).unwrap();
    let Status::Running(id) = shown.status else { panic!("the solver's preview: {:?}", shown.status) };
    layer.run_jobs();
    frames.present(&mut shell, &mut layer, id).unwrap();
    let press = shell.prepare(&layer, "operation.execute", json::object([("preview", Value::Int(id as i64))])).unwrap();
    println!("the solver's commit goes under the command ID `{}`", press.command);
    let committed = shell.send(&mut layer, press);
    assert!(matches!(committed.status, Status::Committed(_)), "{:?}", committed.status);
    assert_eq!(layer.store().log.len(), 1);
}
