//! The debugger's stepper (EV2.12; LP4-5): one advance at a time over `c600-run`'s gate, on a thread of the
//! debugger's; a dispatched host row is a suspended kernel call that a duplicate request neither dispatches nor charges
//! again. The contract cases, by ID, are in `tests/stepping.rs`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;

use c600_check::check;
use c600_debug::stepper::{Stale, Stepped, Stepper};
use c600_run::interp::{load, Host, Loaded, NoHost, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::{code, Amounts, Interruption, Outcome};
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

fn ungated(loaded: &Loaded, options: &Options) -> Run {
    loaded.run("m", "main", vec![Value::int(5)], &mut Engine::quick(), options).unwrap_or_else(|r| panic!("{r:?}"))
}

/// One stepped advance at a time; a host row is a suspended call: the advance that dispatched it answers `Waiting`,
/// a duplicate request returns that without dispatching or charging again, a new request naming the awaiting stop
/// joins it, and completion stops at the continuation with the call's result. The stepped run's charges equal the
/// unstepped run's.
#[test]
fn a_suspended_call_is_dispatched_and_charged_once() {
    let (_, loaded) = build();
    let plain = ungated(&loaded, &Options::new(WIDE));
    let performed = Arc::new(AtomicUsize::new(0));
    let (release, waiting) = mpsc::channel();
    let engine = Engine { performed: Arc::clone(&performed), release: Some(waiting) };
    let run = Arc::clone(&loaded);
    let mut stepper = Stepper::spawn(move |line| {
        let mut engine = engine;
        let mut host = line.dispatching(&mut engine);
        run.run_gated("m", "main", vec![Value::int(5)], &mut host, &Options::new(WIDE), line)
            .unwrap_or_else(|r| panic!("{r:?}"))
    });

    let mut n = 0;
    let (call, expected) = loop {
        let current = stepper.state().id();
        n += 1;
        match stepper.step(&current, &format!("r{n}")).expect("a current stop") {
            Stepped::Stopped { .. } => continue,
            Stepped::Waiting { call, site } => {
                assert_eq!(site.op, "kernel");
                assert_eq!(call, site.step + 1);
                break (call, current);
            }
            Stepped::Ended { .. } => panic!("the run ended before its host row"),
        }
    };
    let dispatcher = format!("r{n}");
    let awaiting = format!("{call}/await");
    assert_eq!(performed.load(Ordering::SeqCst), 1);
    // A duplicate request: the same answer, no second dispatch.
    for _ in 0..3 {
        let again = stepper.step(&expected, &dispatcher);
        assert!(matches!(again, Ok(Stepped::Waiting { call: c, .. }) if c == call));
    }
    // A new request naming the awaiting stop joins the call.
    assert!(matches!(stepper.step(&awaiting, "join"), Ok(Stepped::Waiting { .. })));
    // An ID reused with another stop, and an obsolete stop, are stale.
    assert_eq!(stepper.step("0/entry#read", &dispatcher).err(), Some(Stale::Reused));
    assert_eq!(stepper.step("0/entry#read", "fresh").err(), Some(Stale::Obsolete));
    assert_eq!(performed.load(Ordering::SeqCst), 1);

    release.send(()).unwrap();
    let done = stepper.wait().clone();
    let Stepped::Stopped { stop, completed: Some(completed) } = &done else { panic!("{done:?}") };
    assert_eq!(completed.call, call);
    assert!(matches!(completed.result, Ok(Value::Int(_))));
    assert_eq!(stop.step, call, "the continuation follows the call's own row");
    // The dispatching request and the one that joined now answer the continuation, and a new request naming the
    // completed call's awaiting stop joins it too; nothing is dispatched again.
    assert_eq!(stepper.step(&expected, &dispatcher).expect("the completed call").id(), done.id());
    for id in ["join", "late"] {
        assert_eq!(stepper.step(&awaiting, id).expect("the completed call").id(), done.id());
    }
    assert_eq!(performed.load(Ordering::SeqCst), 1);

    loop {
        let current = stepper.state().id();
        n += 1;
        if let Stepped::Ended { .. } = stepper.step(&current, &format!("r{n}")).expect("a current stop") {
            break;
        }
    }
    let stepped = stepper.outcome().expect("the run's end");
    assert_eq!(stepped.summary, plain.summary, "stepping and waiting add no charge");
    assert_eq!(performed.load(Ordering::SeqCst), 1);
}

/// A pure run under the stepper ends with its value, and interrupting a stopped run ends it as cancelled at that
/// boundary.
#[test]
fn interrupting_a_stopped_run_cancels_it_there() {
    let (_, loaded) = build();
    let run = Arc::clone(&loaded);
    let mut stepper = Stepper::spawn(move |line| {
        let mut host = NoHost;
        let mut host = line.dispatching(&mut host);
        run.run_gated("m", "main", vec![Value::int(5)], &mut host, &Options::new(WIDE), line).unwrap()
    });
    let first = stepper.state().id();
    assert!(matches!(stepper.step(&first, "a"), Ok(Stepped::Stopped { .. })));
    stepper.interrupt(Interruption::ByUser);
    assert!(matches!(stepper.state(), Stepped::Ended { .. }));
    let ended = &stepper.outcome().expect("the end").summary;
    assert_eq!((ended.outcome, ended.code), (Outcome::Cancelled, Some(code::CANCELLED_BY_USER)));
    assert_eq!(ended.used.steps, 1);
}
