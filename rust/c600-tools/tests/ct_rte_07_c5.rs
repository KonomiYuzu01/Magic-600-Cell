//! CT-RTE-07, the 600-cell-Full subset, its two keys that need a runtime process (RTE.46; LX-3's frozen corpus
//! `lx3-neutral-v2`, `budgets.json`): B10, a wall-clock safety stop, and B11, a lost runtime. The rest of the subset is
//! in `c600-module/tests/ct_rte_07_full.rs`, which lists these two keys as left with a pointer here.
//!
//! The corpus's program is an own tool in tool authoring whose loop over ten items ticks once per item. Here the loop is
//! `repeat(10, 0, …)` and its tick a read of the host's row `tick`, which answers how many ticks the host has served;
//! the job runs in `c600-runtime`'s child under the corpus's limits.
//!
//! - B10: the host blocks inside every tick past a safety stop of 1 ms. The parent notices the deadline after the call
//!   returns, while it waits for the child's next message: the run is cut off with `KN7:resource_limit.safety_stop`.
//! - B11: the host kills the child inside its first tick, after admission. The run ends in an error with
//!   `KN7:resource_limit.runtime_lost`; the host served that one tick and nothing after.
//!
//! As the corpus's repetition rule asks, each key checks its typed reason and truthful partial coverage (fewer than
//! ten ticks), not an identical cut-off position. A fresh child then runs the same job to its end. The deadline is a
//! timeout of the test, not a measurement.
//!
//! Evidence kind: source and synthetic. Nothing here measures time or performance, and no Windows/DirectX claim.
//!
//! CT-RTE-07 (source, part: B10 and B11, with a runtime process; not run here: the other keys, in
//! `c600-module/tests/ct_rte_07_full.rs` and the files it names, B12's cold run, which has no test on this commit,
//! B12's other five cache states, not applicable by the owner's decision of 20 November 2026 item 1 and no pass, and
//! the Linux run).

mod process_support;

use std::time::Duration;

use c600_digest::sha256;
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{self, hex, Json};
use c600_run::value::Value;
use c600_runtime::parent::{RUNTIME_LOST, SAFETY_STOP};
use c600_runtime::{Config, Ended, Fault, FaultKind, Job, JobOptions, Runtime};

const CORPUS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../work/experiments/language-lx/lx3-sandbox-flow/corpus/");

/// The corpus's loop: ten items, one tick each. The loop's value is the host's answer to the last tick.
const TICKS: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
    export fn go() -> Nat effects [snapshot] {\n    \
    return repeat(10, 0, fn(done: Nat) -> Nat {\n        return read_snapshot<Nat>(\"tick\");\n    });\n}\n";

/// A budget key's case in the frozen `budgets.json`: its limits, outcome, code and host event.
struct Case {
    limits: Amounts,
    outcome: String,
    code: String,
    event: String,
}

fn case(id: &str) -> Case {
    let read = |name: &str| std::fs::read(format!("{CORPUS}{name}")).unwrap();
    let bytes = read("budgets.json");
    let freeze = provenance::parse(&read("FREEZE.json")).expect("FREEZE.json reads");
    let frozen = freeze.get("files").and_then(|f| f.get("budgets.json")).and_then(Json::as_text);
    assert_eq!(frozen, Some(hex(&sha256(&bytes)).as_str()), "budgets.json is the frozen one");
    let json = provenance::parse(&bytes).expect("budgets.json reads");
    let Some(Json::List(cases)) = json.get("cases") else { panic!("the cases") };
    let case = cases.iter().find(|c| c.get("id").and_then(Json::as_text) == Some(id)).expect("the case");
    let limits = case.get("limits").expect("its limits");
    let n = |k: &str| match limits.get(k) {
        Some(Json::Int(v)) => *v as u128,
        other => panic!("{k}: {other:?}"),
    };
    let expected = case.get("expected").expect("its expectation");
    let text = |o: &Json, k: &str| o.get(k).and_then(Json::as_text).expect(k).to_string();
    Case {
        limits: Amounts { steps: n("steps"), kernel_cost: n("kernel_cost"), memory: n("memory"), output: n("output") },
        outcome: text(expected, "outcome"),
        code: text(expected, "code"),
        event: text(case.get("host").expect("its host event"), "event"),
    }
}

/// The host side: the row `tick` costs 1 and answers how many ticks it has served; `on_tick` acts inside each one.
struct Ticks {
    served: u32,
    on_tick: Box<dyn FnMut(u32)>,
}

impl Ticks {
    fn new(on_tick: impl FnMut(u32) + 'static) -> Ticks {
        Ticks { served: 0, on_tick: Box::new(on_tick) }
    }
}

impl Host for Ticks {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Snapshot { query: "tick", .. } => Ok(RowTariff { kernel_cost: 1, result_bound: 4096 }),
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Snapshot { query: "tick", .. } => {
                self.served += 1;
                (self.on_tick)(self.served);
                Ok(Value::int(i64::from(self.served)))
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}

fn job(limits: Amounts) -> Job {
    Job {
        sources: vec![TICKS.as_bytes().to_vec()],
        module: "m".into(),
        entry: "go".into(),
        args: Vec::new(),
        options: JobOptions::new(limits),
    }
}

/// A runtime on the `c600-runtime` binary, with its child started.
fn started() -> Runtime {
    let mut rt = Runtime::new(Config::new(process_support::runtime()));
    rt.start().unwrap_or_else(|f| panic!("{f}"));
    rt
}

/// The typed end of a faulted run: the corpus's outcome and code, and fewer than ten ticks served.
fn ended_as(fault: &Fault, kind: FaultKind, case: &Case, served: u32) {
    assert_eq!(fault.kind, kind, "{fault}\n{}", fault.stderr);
    assert_eq!((fault.outcome().name(), fault.code()), (case.outcome.as_str(), case.code.as_str()));
    assert!(served < 10, "the loop did not finish: {served} ticks");
}

/// After a fault, a fresh child runs the same job to its end: ten ticks, the loop's value 10.
fn recovers(mut rt: Runtime, limits: Amounts) {
    let spawns = rt.spawns();
    let mut host = Ticks::new(|_| {});
    let run = match rt.run(&job(limits), &mut host, None).unwrap_or_else(|f| panic!("{f}")) {
        Ended::Ran(run) => run,
        Ended::Refused(r) => panic!("refused: {r:#?}"),
    };
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
    assert_eq!(run.output.as_deref(), Some(&b"{\"int\":\"10\"}\n"[..]));
    assert_eq!((host.served, rt.spawns()), (10, spawns + 1), "ten ticks in a fresh child");
}

/// CT-LG4-04 (part: the key B10's wall-clock safety stop ends the run cut off with `KN7:resource_limit.safety_stop` in
/// a runtime process; not run: the stop's run record, in `c600-session/tests/persist_run.rs`, the cache's charges, in
/// `c600-session/tests/jobs.rs`, and the four resources, in `c600-run/tests/conformance_lg4.rs`).
/// B10: the host blocks in each tick for 20 ms; the run's safety stop of 1 ms cuts it off once a call returns.
#[test]
fn ct_rte_07_full_b10_safety_stop() {
    let case = case("B10");
    assert_eq!((case.outcome.as_str(), case.code.as_str(), case.event.as_str()), ("cut_off", SAFETY_STOP, "deadline"));
    let mut rt = started();
    let mut host = Ticks::new(|_| std::thread::sleep(Duration::from_millis(20)));
    let fault = rt.run(&job(case.limits), &mut host, Some(Duration::from_millis(1))).expect_err("a safety stop");
    ended_as(&fault, FaultKind::Deadline, &case, host.served);
    assert!(rt.child_id().is_none(), "the stopped child is gone");
    recovers(rt, case.limits);
}

/// B11: the host kills the child from inside the first tick; the run ends in an error and the host serves nothing
/// after that tick.
#[test]
fn ct_rte_07_full_b11_runtime_lost() {
    let case = case("B11");
    assert_eq!(
        (case.outcome.as_str(), case.code.as_str(), case.event.as_str()),
        ("error", RUNTIME_LOST, "runtime_loss")
    );
    let mut rt = started();
    let killer = rt.killer().expect("a started child");
    let mut host = Ticks::new(move |served| {
        if served == 1 {
            killer.kill();
        }
    });
    let fault = rt.run(&job(case.limits), &mut host, None).expect_err("a lost runtime");
    ended_as(&fault, FaultKind::Lost, &case, host.served);
    assert_eq!(host.served, 1, "the host served the tick it killed the child in, and nothing after");
    assert!(rt.child_id().is_none(), "the killed child is gone");
    recovers(rt, case.limits);
}
