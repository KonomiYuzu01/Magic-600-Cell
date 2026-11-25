//! Stepping RT1's evaluator (EV2.12; LP4-5): the gate sees the run's own LG4/DET.20 positions on the caller's
//! thread, adds no step, charge or tariff, and stops an ordinary call at the callee's entry and its return at the
//! caller's continuation. The debugger's stepper and its checks, by contract ID, are in `c600-debug`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use c600_check::check;
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink, SourceRanges};
use c600_run::meter::{code, Amounts, Interruption, Outcome, SourceRange};
use c600_run::step::{Boundary, Gate, Slot};
use c600_run::value::Value;
use c600_syntax::{lower, parse, Lowered};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
    fn add(x: Nat, y: Nat) -> Nat effects [] {\n    let s = exact_add(x, y);\n    return s;\n}\n\n\
    fn probe() -> Nat effects [snapshot] {\n    return read_snapshot<Nat>(\"pieces\");\n}\n\n\
    export fn main(a: Nat) -> Nat effects [snapshot] {\n    let b = add(a, 2);\n    return exact_add(b, probe());\n}\n";

fn build() -> (Lowered, Arc<Loaded>) {
    let file = parse(SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let loaded = load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"));
    (lowered, Arc::new(loaded))
}

struct Ranges(Lowered);

impl SourceRanges for Ranges {
    fn range(&self, module: &str, function: &str, path: &str) -> Option<SourceRange> {
        let map = self.0.source(module)?;
        let span = map.span(function, path)?;
        Some(SourceRange { digest: map.digest.clone(), start: span.start, end: span.end })
    }
}

/// An engine that answers `pieces`, counts its performed rows, and optionally waits for a release before answering.
struct Engine {
    performed: Arc<AtomicUsize>,
    release: Option<Receiver<()>>,
}

impl Engine {
    fn quick() -> Engine {
        Engine { performed: Arc::new(AtomicUsize::new(0)), release: None }
    }
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 64 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.performed.fetch_add(1, Ordering::SeqCst);
        if let Some(release) = &self.release {
            let _ = release.recv();
        }
        match request {
            Request::Snapshot { query: "pieces", .. } => Ok(Value::int(40)),
            _ => Err(code::NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

/// A gate that records every boundary and lets every operation through.
#[derive(Default)]
struct Recorder(Vec<Boundary>);

impl Gate for Recorder {
    fn before(&mut self, boundary: Boundary) -> Option<Interruption> {
        self.0.push(boundary);
        None
    }
}

fn ungated(loaded: &Loaded, options: &Options) -> Run {
    loaded.run("m", "main", vec![Value::int(5)], &mut Engine::quick(), options).unwrap_or_else(|r| panic!("{r:?}"))
}

/// The gate sees one boundary per operation, before it, at the position the run's meter has reached and under the
/// operation's own locator; a gate that lets everything through leaves the run's rows, charges, stop and output as
/// they are without one (EV2.12: no second step counter and no tariff of its own).
#[test]
fn a_gate_consumes_the_runs_own_positions_and_changes_nothing() {
    let (lowered, loaded) = build();
    let ranges = Ranges(lowered);
    let options = Options { sources: Some(&ranges), ..Options::new(WIDE) };
    let plain = ungated(&loaded, &options);
    let mut gate = Recorder::default();
    let gated = loaded
        .run_gated("m", "main", vec![Value::int(5)], &mut Engine::quick(), &options, &mut gate)
        .unwrap_or_else(|r| panic!("{r:?}"));
    assert_eq!(gated.summary, plain.summary);
    assert_eq!(gated.output, plain.output);
    assert_eq!(plain.output.as_deref(), Some(&b"{\"int\":\"47\"}\n"[..]));
    let rows = &plain.summary.rows;
    assert_eq!(gate.0.len(), rows.len());
    let mut position = 0;
    for (boundary, row) in gate.0.iter().zip(rows) {
        assert_eq!(boundary.at, row.at);
        assert_eq!(boundary.step, position, "the boundary is the meter's position before {}", row.at);
        position += row.steps;
    }
    assert_eq!(position, plain.summary.used.steps);

    // Interrupting at a boundary stops the run there, as a cancellation does.
    struct Stop(u128);
    impl Gate for Stop {
        fn before(&mut self, boundary: Boundary) -> Option<Interruption> {
            (boundary.step == self.0).then_some(Interruption::ByUser)
        }
    }
    let cut = loaded
        .run_gated("m", "main", vec![Value::int(5)], &mut Engine::quick(), &options, &mut Stop(4))
        .unwrap_or_else(|r| panic!("{r:?}"));
    assert_eq!((cut.summary.outcome, cut.summary.code), (Outcome::Cancelled, Some(code::CANCELLED_BY_USER)));
    assert_eq!(cut.summary.used.steps, 4);
}

/// An ordinary call's next boundary is the callee's entry, with the call site among the enclosing sites and the
/// callee's parameters as its locals; the return reaches the caller's continuation (EV2.12).
#[test]
fn a_call_stops_at_the_callees_entry_and_returns_to_the_continuation() {
    let (lowered, loaded) = build();
    let ranges = Ranges(lowered);
    let options = Options { sources: Some(&ranges), ..Options::new(WIDE) };
    let mut gate = Recorder::default();
    loaded
        .run_gated("m", "main", vec![Value::int(5)], &mut Engine::quick(), &options, &mut gate)
        .unwrap_or_else(|r| panic!("{r:?}"));
    let stops = &gate.0;
    let call = stops.iter().position(|b| b.op == "call" && b.at.starts_with("m.main:")).expect("the call of add");
    let entry = &stops[call + 1];
    assert_eq!((stops[call].at.as_str(), entry.at.as_str()), ("m.main:0#call", "m.main:0 > m.add:0.0#visit"));
    assert_eq!(entry.operation.as_ref().map(|o| o.function.as_str()), Some("add"));
    let site = entry.sites.last().expect("the call site");
    assert_eq!(site.operation.function, "main");
    let range = site.source.as_ref().expect("the call site's range");
    assert_eq!(&SOURCE[range.start..range.end], "add(a, 2)");
    let names: Vec<_> = entry.locals.iter().map(|l| (l.name.as_str(), l.slot)).collect();
    assert_eq!(names, [("x", Slot::Param), ("y", Slot::Param)]);

    let ret = stops.iter().position(|b| b.op == "return" && b.at.contains("m.add:")).expect("add's return");
    let continuation = &stops[ret + 1];
    assert_eq!(continuation.at, "m.main:root#visit");
    assert!(continuation.sites.is_empty(), "{:?}", continuation.sites);
    // A completed `let` is a local until its scope exits, so `s` is gone at add's return; in main, `b` is bound only
    // after the call returned.
    let names = |b: &Boundary| b.locals.iter().map(|l| l.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&stops[ret - 1]), ["x", "y", "s"]);
    assert_eq!(stops[ret - 1].op, "scope_exit");
    assert_eq!(names(&stops[ret]), ["x", "y"]);
    assert_eq!(names(continuation), ["a"]);
    let later = stops.iter().rev().find(|b| b.at.starts_with("m.main:") && b.op != "return").expect("a later stop");
    assert!(later.locals.iter().any(|l| l.name == "b" && l.slot == Slot::Let));
}
