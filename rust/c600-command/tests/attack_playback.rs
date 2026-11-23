//! Attack tests on recording playback (owner-requested attack of the command layer, class B: a recording that commits
//! when played). When these tests were written playback was not built, and the door a playback would come in by was a
//! stored row that an entry serves (`CommandLayer::serve_row`). Since LP4-9 the layer builds the four `recording.*`
//! rows itself (`src/layer/playback.rs`): no entry serves them, and they need the host's recorder. These tests attack
//! both doors and pin what the layer answers; `tests/recording.rs` holds LP4-9's own tests, the unattended table among
//! them.
//!
//! Each test asserts the rule. The one that failed while its bypass existed gives the finding, what was observed on
//! commit 4c20ce30 and the fix.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, Caller, CommandLayer, Invocation, MemoryRecorder, MemoryStore, Overlay, Receipt, Secret, Shell, Status,
    Table, POLICY_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{Kernel, ProtectionMode};

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
///
/// Since LP4-9 the layer builds `recording.play`: an entry's serve of it is refused with
/// `KN5:input.overlay_row_conflict` and changes nothing, and with the host's recorder installed and a recording the
/// solver made, a run's `recording.play` is refused with `LG3:capability.not_granted` (no `recording.*` row is a run's,
/// [`CommandLayer::invoke`]'s `runtime_may`): no job waits and no playback state is written. The solver's own play of
/// the same recording starts.
#[test]
fn a_run_cannot_start_a_served_playback() {
    let mut f = fixture();
    let overlay = Overlay { module: "tools/recording".into(), entry: "play".into() };
    let refused = f.layer.serve_row("recording.play", overlay).unwrap_err();
    assert_eq!(refused.code, code::OVERLAY_ROW_CONFLICT);
    assert!(f.layer.table().row("recording.play").is_some_and(|row| row.overlay.is_none()));

    let recorder = MemoryRecorder::new();
    f.layer.set_recorder(Box::new(recorder.clone())).unwrap();
    for (row, args) in [("recording.start", args("recording.start")), (POLICY_ROW, policy(None))] {
        let _ = f.shell.invoke(&mut f.layer, row, args).unwrap();
    }
    let stopped = f.shell.invoke(&mut f.layer, "recording.stop", args("recording.stop")).unwrap();
    assert!(matches!(stopped.status, Status::Completed(_)), "{:?}", stopped.status);
    assert_eq!(recorder.held().recordings.get("r1").map(|r| r.steps.len()), Some(1));

    for row in ROWS {
        let receipt = f.runtime("stray-run", row, args(row));
        assert_eq!(receipt.status.code(), Some(code::NOT_GRANTED), "{row}: {:?}", receipt.status);
        let job = f.layer.take_program_job().map(|program| program.job);
        assert!(
            matches!(receipt.status, Status::Refused(_)) && job.is_none() && f.layer.run_jobs() == 0,
            "the run's `{row}` was answered {}, and the waiting program job is {job:?}",
            answer(&receipt)
        );
        assert!(recorder.held().playbacks.is_empty(), "{row}");
        assert_eq!(f.layer.capturing(), None, "{row}");
    }
    let played = f.shell.invoke(&mut f.layer, "recording.play", args("recording.play")).unwrap();
    assert!(matches!(played.status, Status::Running(_)), "the solver's play: {:?}", played.status);
}

fn policy(confirm: Option<bool>) -> Value {
    match confirm {
        None => json::object([("policy", text("strict"))]),
        Some(confirm) => json::object([("confirm", Value::Bool(confirm)), ("policy", text("strict"))]),
    }
}

/// Finding LD1 (class B and D, a lead: playback is not built). Rule: EV4.22 [D] lets a recording play view and
/// workspace steps unattended, never a commit. H-08 gives `protection.policy.set`, a workspace row, no `confirm` gate,
/// so a recorded switch from Strict to Net would be an unattended step, and the solver's next commit would be judged
/// under a policy they did not set in this sitting.
///
/// Fixed (the owner's reading of 18 November 2026, `docs/wiki/decisions/owner-decisions-2026-11-18.md`): a change of
/// the policy needs the solver's own confirmed press, as a release of protection does, so no playback or run changes
/// it. A step with no confirmation, or with a confirmation of false, is refused with `KN5:input.confirmation_required`;
/// a run's step is refused with or without one; the policy, the revision and the journal stay as they were. The
/// solver's confirmed press changes the policy.
#[test]
fn ld1_a_policy_change_needs_the_solvers_confirmed_press() {
    let mut f = fixture();
    let before = (f.layer.policy(), f.layer.revision().clone(), f.transactions());
    let strict = |confirm: Option<bool>| match confirm {
        None => json::object([("policy", text("strict"))]),
        Some(confirm) => json::object([("confirm", Value::Bool(confirm)), ("policy", text("strict"))]),
    };
    for (what, confirm) in [("no confirmation", None), ("a confirmation of false", Some(false))] {
        let receipt = f.shell.invoke(&mut f.layer, POLICY_ROW, strict(confirm)).unwrap();
        assert_eq!(receipt.status.code(), Some(code::CONFIRMATION_REQUIRED), "{what}: {:?}", receipt.status);
        assert_eq!((f.layer.policy(), f.layer.revision().clone(), f.transactions()), before, "{what}");
    }
    for confirm in [None, Some(true)] {
        let receipt = f.runtime("stray-run", POLICY_ROW, strict(confirm));
        assert!(matches!(receipt.status, Status::Refused(_)), "a run's step ({confirm:?}): {:?}", receipt.status);
        println!("a run's policy step with confirmation {confirm:?} is answered {}", answer(&receipt));
        assert_eq!((f.layer.policy(), f.layer.revision().clone(), f.transactions()), before, "{confirm:?}");
    }
    assert_eq!(before.0, ProtectionMode::Net);
    let pressed = f.shell.invoke(&mut f.layer, POLICY_ROW, strict(Some(true))).unwrap();
    assert!(matches!(pressed.status, Status::Completed(_)), "the solver's press: {:?}", pressed.status);
    assert_eq!((f.layer.policy(), f.transactions()), (ProtectionMode::Strict, before.2 + 1));
}

/// Guard (class B, refuted): without the host's recorder, the layer does nothing for a recording row. From the shell
/// and from a run, each of the four rows is refused, no job is admitted and nothing durable changes. The codes are
/// printed, not pinned: they are the layer's answer for a row it cannot run (since LP4-9, `KN5:input.row_not_built`
/// from the shell, as before, and `LG3:capability.not_granted` from a run).
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
