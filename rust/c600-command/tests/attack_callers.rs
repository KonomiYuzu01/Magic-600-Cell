//! Attack tests on the caller stamp and the command IDs (owner-requested attack of the command layer, class G: caller
//! confusion). The host stamps the caller from the channel, and the envelope names none, so no field of an envelope
//! claims the shell (`tests/attack_runtime.rs` shows that a run's replay of the solver's own envelope, token included,
//! is refused). What is left is the command ID: the retry lookup keeps receipts under the caller's scope and the
//! command ID, and the scope is the caller's kind (`shell` or `runtime`), not the run.
//!
//! Each test asserts the rule; a test that fails while the confusion exists is ignored and its comment gives the
//! finding and what was observed on commit 4c20ce30. Run the ignored one with `-- --include-ignored`.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use c600_command::{
    draft_args, Caller, CommandLayer, HeadlessFrames, Invocation, MemoryStore, Receipt, Secret, Shell, Status, Table,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

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

/// A session at the solved state that protects nothing, on H-08's table.
fn layer() -> CommandLayer {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected)
}

/// One preview envelope on `run`'s channel under the command ID `command`, with no token.
fn preview(layer: &mut CommandLayer, run: &str, command: &str, letters: &[i64]) -> Receipt {
    let invocation = Invocation {
        command: command.to_owned(),
        row: "operation.preview".to_owned(),
        args: draft_args(&word(letters)),
        token: None,
        epoch: layer.revision().epoch,
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
#[test]
#[ignore = "finding G1: fails while one run's command ID is used up for every other run"]
fn g1_one_run_cannot_use_up_another_runs_command_id() {
    let mut layer = layer();
    let squat = preview(&mut layer, "run-a", "run-b/1", &[1]);
    assert!(matches!(squat.status, Status::Running(_)), "{:?}", squat.status);
    let own = preview(&mut layer, "run-b", "run-b/1", &[2]);
    assert!(
        matches!(own.status, Status::Running(_)),
        "run-b's first command, under its own ID `run-b/1`, was answered {}",
        answer(&own)
    );
}

/// Guard (class G, refuted): a run cannot use up the shell's command IDs. A run sends previews under the IDs the
/// shell is about to use; the solver's own preview, presentation and commit press are then answered as if the run had
/// sent nothing, because the shell's scope is its own.
#[test]
fn a_run_cannot_use_up_the_shells_command_ids() {
    let mut layer = layer();
    for n in 0..40 {
        for id in [format!("shell/{n}"), format!("shell-{n}"), format!("shell{n}"), n.to_string()] {
            let _ = preview(&mut layer, "run-a", &id, &[1]);
        }
    }
    layer.run_jobs();
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
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
