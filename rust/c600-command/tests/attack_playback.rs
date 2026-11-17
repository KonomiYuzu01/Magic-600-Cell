//! Attack tests on recording playback (owner-requested attack of the command layer, class B: a recording that commits
//! when played). Playback is not built: no source file in the workspace plays a recording, and the four `recording.*`
//! rows of H-08 have no implementation in the layer. What exists is the door a playback would come in by: a stored
//! row that an entry serves (`CommandLayer::serve_row`), which `tests/served_rows.rs` already exercises with
//! `recording.play`. These tests attack that door and pin what the layer answers today.
//!
//! Each test asserts the rule. The one that failed while its bypass existed gives the finding, what was observed on
//! commit 4c20ce30 and the fix.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, Caller, CommandLayer, Invocation, MemoryStore, Overlay, Receipt, Secret, Shell, Status, Table,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const ROWS: [&str; 4] = ["recording.start", "recording.stop", "recording.play", "recording.convert"];

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    Arc::clone(KERNEL.get_or_init(|| {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
        Arc::new(Kernel::load(&assets).unwrap_or_else(|e| panic!("the model in assets/ does not load: {e}")))
    }))
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// The arguments H-08 gives each recording row.
fn args(row: &str) -> Value {
    match row {
        "recording.start" => json::object([("name", text("r1"))]),
        "recording.convert" => json::object([("recording", text("r1")), ("target", text("draft"))]),
        _ => json::object([("recording", text("r1"))]),
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    sent: u64,
}

/// A session at the solved state that protects nothing, on H-08's table.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), sent: 0 }
}

impl Fixture {
    /// One envelope on a run's channel, with no token, stamped as the host stamps it.
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

    fn transactions(&self) -> usize {
        self.layer.store().log.len()
    }
}

fn answer(receipt: &Receipt) -> String {
    match &receipt.status {
        Status::Refused(error) | Status::Failed(error) => error.code.to_owned(),
        other => format!("{other:?}"),
    }
}

/// B3, second instance (class B, attacker 4 today: the bridge's envelope carries only a `draft`, so no script can send
/// these arguments over the product's wire). Rule: EV4.22 [D] "RPI.14 stays true: playback is solver-invoked, not a
/// view effect", and EV4.12 [B] a commit never plays unattended. A stored row that an entry serves is admitted for
/// any caller, so once `recording.play` is served a run starts a playback job with no solver action.
///
/// Observed on 4c20ce30: the run's invocation of the served `recording.play` is answered `Running(1)` and a program
/// job for the entry is waiting.
///
/// Fixed with B3: `recording.play` is a workspace row, which a run does not reach, so the invocation is refused with
/// `LG3:capability.not_granted` and no job waits.
#[test]
fn a_run_cannot_start_a_served_playback() {
    let mut f = fixture();
    let overlay = Overlay { module: "tools/recording".into(), entry: "play".into() };
    f.layer.serve_row("recording.play", overlay).unwrap();
    let receipt = f.runtime("stray-run", "recording.play", args("recording.play"));
    let job = f.layer.take_program_job().map(|program| program.job);
    assert_eq!(receipt.status.code(), Some(code::NOT_GRANTED), "{:?}", receipt.status);
    assert!(
        matches!(receipt.status, Status::Refused(_)) && job.is_none(),
        "the run's `recording.play` was answered {}, and the waiting program job is {job:?}",
        answer(&receipt)
    );
}

/// Guard (class B, refuted): playback is not built, and the layer does nothing for a recording row that no entry
/// serves. From the shell and from a run, each of the four rows is refused, no job is admitted and nothing durable
/// changes. The codes are printed, not pinned: they are the layer's answer for a row it does not build.
#[test]
fn an_unserved_recording_row_does_nothing() {
    let mut f = fixture();
    let before = (f.layer.revision().clone(), f.transactions());
    for row in ROWS {
        let from_shell = f.shell.invoke(&mut f.layer, row, args(row));
        let from_shell = match from_shell {
            Ok(receipt) => {
                assert!(matches!(receipt.status, Status::Refused(_)), "{row} from the shell: {:?}", receipt.status);
                answer(&receipt)
            }
            Err(error) => error.code.to_owned(),
        };
        let from_run = f.runtime("stray-run", row, args(row));
        assert!(matches!(from_run.status, Status::Refused(_)), "{row} from a run: {:?}", from_run.status);
        println!("{row}: the shell is answered {from_shell}, a run {}", answer(&from_run));
        assert!(f.layer.take_program_job().is_none(), "{row} admitted a job");
        assert_eq!((f.layer.revision(), f.transactions()), (&before.0, before.1), "{row}");
    }
}
