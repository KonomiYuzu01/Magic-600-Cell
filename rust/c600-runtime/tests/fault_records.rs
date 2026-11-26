//! CT-DET-13 and CT-LG4-04 (parts), the runtime half: every fault the parent reports maps to a `c600_session` run
//! record ending that agrees with it, code for code (DET.17, DET.43, KN7.10, KN7.13). The session crate keeps no edge to
//! this crate: its `run::Fault` is a plain value that the runtime's caller fills, and this test is the caller. An LP2
//! draft until the IR freeze (PHS.49).
//!
//! Evidence kind: source. No runtime process runs here; the faults are built values, and the process-level rehearsals
//! are in `faults.rs`.

use c600_run::meter::Outcome;
use c600_runtime::parent::{RUNTIME_LOST, SAFETY_STOP};
use c600_runtime::{Fault, FaultKind};
use c600_session::persist::run::{self, exact};

const KINDS: [FaultKind; 7] = [
    FaultKind::Spawn,
    FaultKind::Incompatible,
    FaultKind::Lost,
    FaultKind::Malformed,
    FaultKind::Oversized,
    FaultKind::Protocol,
    FaultKind::Deadline,
];

/// The caller's mapping: a passed deadline is a safety stop with the elapsed time, every other fault a lost runtime.
fn reported(fault: &Fault, elapsed_ms: u128) -> run::Fault {
    match fault.kind {
        FaultKind::Deadline => run::Fault::Deadline { elapsed_ms },
        _ => run::Fault::RuntimeLost,
    }
}

/// CT-DET-13 (part: the mapping from each fault kind of the runtime's parent to a run record's ending; not run: a
/// record written from a fault of a real runtime process, which needs the session store's executor, LP3's job) and
/// CT-LG4-04 (part: a passed deadline is recorded as the safety stop, a cut-off with `KN7:resource_limit.safety_stop`
/// and its elapsed time kept apart; not run: the other halves, in `faults.rs`, `jobs.rs` and `conformance_lg4.rs`).
#[test]
fn each_fault_kind_maps_to_the_ending_its_code_and_outcome_name() {
    assert_eq!(run::codes::SAFETY_STOP, SAFETY_STOP);
    assert_eq!(run::codes::RUNTIME_LOST, RUNTIME_LOST);
    for kind in KINDS {
        let fault = Fault { kind, detail: String::new(), exit: None, stderr: String::new() };
        let mapped = reported(&fault, 1_234);
        let payload = exact(&mapped.ending(), &[]);
        let text = |name: &str| payload.get(name).and_then(|v| v.as_text()).map(str::to_owned);
        assert_eq!(text("code").as_deref(), Some(fault.code()), "{kind:?}");
        assert_eq!(text("outcome").as_deref(), Some(fault.outcome().name()), "{kind:?}");
        let deadline = kind == FaultKind::Deadline;
        assert_eq!(mapped.safety_stop(), deadline.then_some(1_234), "{kind:?}");
        let expected = if deadline { (SAFETY_STOP, Outcome::CutOff) } else { (RUNTIME_LOST, Outcome::Error) };
        assert_eq!((fault.code(), fault.outcome()), expected, "{kind:?}");
    }
}
