//! Shared fixtures of the debugger's tests: a checked program, its target, a test engine, and a capturing host that
//! makes a run record the way a session would.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use c600_check::check;
use c600_debug::replay::{Event, ModuleRef, Recorded, ReproKey, RequestKey, RunRecord, SinkKey, TEST_COST_UNITS};
use c600_debug::session::{evaluate, Executor, RunOptions, Target};
use c600_debug::sources::Executed;
use c600_debug::{Identity, Pinned};
use c600_digest::sha256_hex;
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::{code, Amounts};
use c600_run::value::Value;
use c600_syntax::{lower, parse, Lowered};

pub const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// A program with an ordinary call, a host row inside a callee, a workspace write, a view and a preview.
pub const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule m requires [file];\n\n\
    fn add(x: Nat, y: Nat) -> Nat effects [] {\n    let s = exact_add(x, y);\n    return s;\n}\n\n\
    fn probe() -> Nat effects [snapshot] {\n    return read_snapshot<Nat>(\"pieces\");\n}\n\n\
    export fn main(a: Nat) -> Nat effects [snapshot] {\n    let b = add(a, 2);\n    return exact_add(b, probe());\n}\n\n\
    export fn pure(a: Nat) -> Nat effects [] {\n    let b = add(a, 2);\n    return add(b, b);\n}\n\n\
    export fn save(w: Word) -> Unit effects [workspace, file] {\n    \
    let a = write_workspace<Word>(\"saved\", w);\n    let b = file_write(\"saved.6cw\", w);\n    return ();\n}\n\n\
    export fn show(a: Nat) -> Unit effects [snapshot, view] {\n    \
    return publish_view(record { n = exact_add(a, probe()) });\n}\n\n\
    export fn preview(w: Word) -> Text effects [preview] {\n    return receipt_id(propose(w, target(\"t\")));\n}\n";

/// Parses, lowers, checks and loads `sources`; the checker must admit them.
pub fn build(sources: &[&str]) -> (Lowered, Arc<Loaded>) {
    let files: Vec<_> = sources
        .iter()
        .map(|s| {
            let file = parse(s.as_bytes());
            assert!(file.diagnostics.is_empty(), "{s}\n{:#?}", file.diagnostics);
            file
        })
        .collect();
    let lowered = lower(&files).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let loaded = load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"));
    (lowered, Arc::new(loaded))
}

/// 600-cell-Full's identity block as these tests pin it.
pub fn identity() -> Identity {
    Identity { puzzle: "600-cell-Full".into(), model: "model-1".into(), menu: None, menu_order: None, contract: None }
}

pub fn pinned() -> Pinned {
    Pinned { identity: identity(), level: "labels".into(), digest: "a".repeat(64) }
}

/// The target of `m.<function>` of `SOURCE`, holding its bytes.
pub fn target(function: &str) -> Target {
    let bytes: Arc<[u8]> = Arc::from(SOURCE.as_bytes());
    let digest = sha256_hex(SOURCE.as_bytes());
    Target {
        module: "m".into(),
        function: function.into(),
        module_digest: digest.clone(),
        sources: BTreeMap::from([(digest, bytes)]),
        ir_version: c600_ir::IR_VERSION.into(),
        execution_contract: c600_ir::EXEC_CONTRACT.into(),
        input: pinned(),
        executor: Executor::Evaluator,
    }
}

/// Run options with `SOURCE`'s source map.
pub fn mapped(lowered: &Lowered, limits: Amounts) -> RunOptions {
    let executed = Executed::new(lowered, &[SOURCE.as_bytes()]);
    RunOptions { sources: Some(Arc::new(executed)), ..RunOptions::new(limits) }
}

/// What an engine saw: its performed rows and the sinks it was handed.
#[derive(Default)]
pub struct Seen {
    pub performed: AtomicUsize,
    pub sinks: Mutex<Vec<String>>,
}

/// A test engine: `pieces` is 40, a preview answers a receipt, and every sink succeeds. It records what reached it,
/// and when it has a release channel, each performed row waits for one release.
pub struct Engine {
    pub seen: Arc<Seen>,
    pub release: Option<Receiver<()>>,
    pub refuse: bool,
    pub kernel_cost: u128,
}

impl Engine {
    pub fn new() -> (Engine, Arc<Seen>) {
        let seen = Arc::new(Seen::default());
        (Engine { seen: Arc::clone(&seen), release: None, refuse: false, kernel_cost: 9 }, seen)
    }
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: self.kernel_cost, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.seen.performed.fetch_add(1, Ordering::SeqCst);
        if let Some(release) = &self.release {
            let _ = release.recv();
        }
        if self.refuse {
            return Err(code::NOT_GRANTED);
        }
        match request {
            Request::Snapshot { query: "pieces", .. } => Ok(Value::int(40)),
            Request::Propose { .. } => Ok(Value::receipt(vec![
                ("base", Value::text("rev-7")),
                ("effect", Value::text("moves 12 stickers")),
                ("id", Value::text("preview-1")),
                ("protection", Value::tag("clear")),
            ])),
            _ => Err(code::NOT_GRANTED),
        }
    }

    fn sink(&mut self, sink: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        self.seen.sinks.lock().unwrap().push(format!("{sink:?}"));
        Ok(())
    }
}

/// A host that passes every call to `host` and keeps the trace a run record holds (the session's capture, here for
/// the tests).
pub struct Capturing<'h> {
    pub host: &'h mut dyn Host,
    pub events: Vec<Event>,
}

impl Host for Capturing<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        let answer = self.host.tariff(request);
        self.events.push(Event::Tariff { request: RequestKey::of(request), answer });
        answer
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let answer = self.host.perform(request);
        self.events.push(Event::Perform { request: RequestKey::of(request), answer: answer.clone() });
        answer
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        let answer = self.host.sink(sink, value, bytes);
        self.events.push(Event::Sink { sink: SinkKey::of(sink), bytes: bytes.to_vec(), answer });
        answer
    }
}

/// The key of a record of `m.<function>(args)` of `SOURCE` under `limits`.
pub fn key(function: &str, args: Vec<Value>, limits: Amounts) -> ReproKey {
    ReproKey {
        sources: vec![ModuleRef { name: "m".into(), digest: sha256_hex(SOURCE.as_bytes()) }],
        edition: c600_syntax::EDITION.into(),
        ir_version: c600_ir::IR_VERSION.into(),
        execution_contract: c600_ir::EXEC_CONTRACT.into(),
        input: pinned(),
        module: "m".into(),
        function: function.into(),
        args,
        seed: None,
        limits,
        cost_units: TEST_COST_UNITS.into(),
    }
}

/// Runs `m.<function>(args)` of `SOURCE` on `host` with its source map and test cost units, and records it.
pub fn capture(name: &str, function: &str, args: Vec<Value>, limits: Amounts, host: &mut dyn Host) -> (RunRecord, Run) {
    let (lowered, loaded) = build(&[SOURCE]);
    let executed = Executed::new(&lowered, &[SOURCE.as_bytes()]);
    let options = Options { sources: Some(&executed), ..Options::new(limits) };
    let mut capturing = Capturing { host, events: Vec::new() };
    let run = loaded.run("m", function, args.clone(), &mut capturing, &options).unwrap_or_else(|r| panic!("{r:?}"));
    let record = RunRecord {
        name: name.into(),
        key: key(function, args, limits),
        events: capturing.events,
        result: Recorded::of(&run),
    };
    (record, run)
}

/// Opens `run` on `m.<function>(args)` in `debugger`, with `engine` behind the debugger's read-only host.
pub fn open(
    debugger: &mut c600_debug::session::Debugger,
    run: &str,
    function: &str,
    args: Vec<Value>,
    engine: Engine,
    options: RunOptions,
    loaded: &Arc<Loaded>,
) -> c600_debug::session::StepResult {
    let execute = evaluate(Arc::clone(loaded), "m".into(), function.into(), args, Box::new(engine), options);
    debugger.open(run, target(function), execute).expect("a new session")
}
