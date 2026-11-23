//! EV2's debugger on the host (chapter 04 EV2.1, EV2.8, EV2.10; LP4-5, LP4-6): CT-EV2-01 and CT-EV2-09 whole, over a
//! command layer. The debugger reads the layer through `c600_tools::debug::Rows`, its query rows; a stepped run's
//! engine here is live, so a write that reached it would commit.

mod process_support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use c600_check::check;
use c600_command::{code, draft_args, CommandLayer, Direction, HeadlessFrames, Shell, Status};
use c600_debug::code::{DEBUG_WRITE_FORBIDDEN, REVISION_GONE};
use c600_debug::journal::{freshness, Freshness, JournalRows, Travel};
use c600_debug::replay::{Comparison, ModuleRef, Recorded, Replayer, ReproKey, RunRecord};
use c600_debug::session::{evaluate, Debugger, Reason, RunOptions, State, StepResult, Target};
use c600_debug::sources::Executed;
use c600_debug::{Identity, Pinned};
use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value as Json};
use c600_run::interp::{load, Host, Loaded, Request, RowTariff, Sink};
use c600_run::meter::{code as meter_code, Amounts, Interruption};
use c600_run::value::Value;
use c600_syntax::{lower, parse, Lowered};
use c600_tools::debug::Rows;

use process_support::{fingerprint, session};

const WIDE: Amounts = Amounts { steps: 1 << 30, kernel_cost: 1 << 30, memory: 1 << 30, output: 1 << 30 };

/// A research module: a read of the input, and one entry per effect site.
const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule d requires [file];\n\n\
    export fn study(k: Int) -> Int effects [snapshot] {\n    \
    let n = read_snapshot<Int>(\"probe\");\n    return exact_add(n, k);\n}\n\n\
    export fn keep(w: Word) -> Unit effects [workspace] {\n    return write_workspace<Word>(\"kept\", w);\n}\n\n\
    export fn export(w: Word) -> Unit effects [file] {\n    return file_write(\"kept.6cw\", w);\n}\n\n\
    export fn show(k: Int) -> Unit effects [view] {\n    return publish_view(record { k = k });\n}\n\n\
    export fn try_word(w: Word) -> Text effects [preview] {\n    return receipt_id(propose(w, target(\"t\")));\n}\n";

fn build() -> (Lowered, Arc<Loaded>) {
    let file = parse(SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let loaded = Arc::new(load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}")));
    (lowered, loaded)
}

fn identity() -> Identity {
    Identity { puzzle: "600-cell-Full".into(), model: "model".into(), menu: None, menu_order: None, contract: None }
}

/// A session on the command layer, with its shell and frames.
struct Live {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

impl Live {
    fn new(name: &str) -> Live {
        let (layer, shell) = session(name);
        Live { layer, shell, frames: HeadlessFrames::new() }
    }

    /// Previews and commits `letters` as the solver does; the commit's status.
    fn commit(&mut self, letters: &[i64]) -> Status {
        let recipe = json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap();
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(&recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let args = json::object([("preview", Json::Int(id as i64))]);
        let press = self.shell.prepare(&self.layer, "operation.execute", args).unwrap();
        self.shell.send(&mut self.layer, press).status
    }

    fn undo(&mut self) -> Status {
        let hint = self.layer.undo_hint(Direction::Undo).expect("an undo target");
        self.shell.invoke(&mut self.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap().status
    }

    /// Everything durable a debugger operation must leave alone: the head digest and event, the state revision, the
    /// durable transactions, the workspace revision and records, and the journal's first page.
    fn durable(&self) -> (impl PartialEq + std::fmt::Debug, u64, Vec<(String, u64)>, String) {
        let records = self.layer.records().iter().map(|(name, r)| (name.clone(), r.version)).collect();
        let page = format!("{:?}", self.layer.journal(None, 50, None).map(|p| p.entries.len()));
        (fingerprint(&self.layer), self.layer.revision().workspace, records, page)
    }
}

/// A live engine on a shared session: the probe row reads the state revision, a preview commits its word, and any
/// sink commits a fixed word. Every write it is asked for is counted.
struct Engine {
    live: Arc<Mutex<Live>>,
    writes: Arc<AtomicUsize>,
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 1, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let mut live = self.live.lock().unwrap();
        match request {
            Request::Snapshot { query: "probe", .. } => Ok(Value::int(live.layer.revision().state)),
            Request::Propose { word, .. } => {
                self.writes.fetch_add(1, Ordering::SeqCst);
                let letters: Vec<i64> = word.iter().map(|&l| i64::from(l)).collect();
                live.commit(&letters);
                Ok(Value::receipt(vec![("base", Value::text("e1")), ("id", Value::text("p/1"))]))
            }
            _ => Err(meter_code::NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.live.lock().unwrap().commit(&[1]);
        Ok(())
    }
}

fn target(input: Pinned) -> Target {
    let digest = sha256_hex(SOURCE.as_bytes());
    Target {
        module: "d".into(),
        function: String::new(),
        module_digest: digest.clone(),
        sources: [(digest, Arc::from(SOURCE.as_bytes()))].into(),
        ir_version: c600_ir::IR_VERSION.into(),
        execution_contract: c600_ir::EXEC_CONTRACT.into(),
        input,
        executor: c600_debug::session::Executor::Evaluator,
    }
}

fn pinned(live: &Live) -> Pinned {
    Pinned { identity: identity(), level: "labels".into(), digest: live.layer.revision().digest.clone() }
}

fn to_end(d: &mut Debugger, run: &str, mut at: StepResult) -> StepResult {
    let mut n = 0;
    while let State::Stopped { .. } | State::Waiting { .. } = at.state {
        n += 1;
        at = d.debug_step(run, &at.stop_id, &format!("{run}/{n}")).unwrap();
        if let State::Waiting { .. } = at.state {
            at = d.wait(run).unwrap();
        }
    }
    at
}

/// CT-EV2-01 (whole): every debugger operation, and stepping over each effect site with a live engine behind the
/// debugger, changes no journal, workspace or session digest and emits no commit.
#[test]
fn ct_ev2_01_debugger_operations_change_nothing_durable() {
    let (lowered, loaded) = build();
    let live = Arc::new(Mutex::new(Live::new("ev2-01")));
    assert!(matches!(live.lock().unwrap().commit(&[1, 2]), Status::Committed(_)));
    let before = live.lock().unwrap().durable();
    let writes = Arc::new(AtomicUsize::new(0));
    let engine = || Box::new(Engine { live: Arc::clone(&live), writes: Arc::clone(&writes) });
    let options = || {
        let executed = Executed::new(&lowered, &[SOURCE.as_bytes()]);
        RunOptions { sources: Some(Arc::new(executed)), ..RunOptions::new(WIDE) }
    };
    let input = pinned(&live.lock().unwrap());
    let word = || Value::Word(Arc::from(&[3, -4][..]));
    let mut d = Debugger::new();

    // Stepping over each effect site: refused at its site, and nothing reaches the live engine.
    for (entry, arg) in [("keep", word()), ("export", word()), ("show", Value::int(2)), ("try_word", word())] {
        let execute = evaluate(Arc::clone(&loaded), "d".into(), entry.into(), vec![arg], engine(), options());
        let first = d.open(entry, Target { function: entry.into(), ..target(input.clone()) }, execute).unwrap();
        let end = to_end(&mut d, entry, first);
        let State::Terminal(t) = &end.state else { panic!("{end:?}") };
        assert_eq!((t.reason, t.code), (Reason::Error, Some(DEBUG_WRITE_FORBIDDEN)), "{entry}");
        assert!(matches!(&end.source, c600_debug::session::Source::Mapped(r) if !r.is_empty()), "{entry}");
        let _ = d.profile(entry).unwrap();
        d.close(entry);
    }

    // The reading operations of a stepped run: steps with duplicates and a stale stop, the current stop, inspection,
    // the profile, cancellation, a safety stop and closing.
    let execute = evaluate(Arc::clone(&loaded), "d".into(), "study".into(), vec![Value::int(4)], engine(), options());
    let first = d.open("study", Target { function: "study".into(), ..target(input.clone()) }, execute).unwrap();
    let second = d.debug_step("study", &first.stop_id, "s1").unwrap();
    assert_eq!(d.debug_step("study", &first.stop_id, "s1").unwrap(), second);
    assert!(d.debug_step("study", &first.stop_id, "s2").is_err());
    let _ = d.current("study").unwrap();
    for value in &second.values {
        let _ = d.inspect_value("study", &value.reference).unwrap();
    }
    let end = to_end(&mut d, "study", second);
    let result = end.values.first().expect("the run's value");
    let _ = d.inspect_value("study", &result.reference).unwrap();
    let _ = d.profile("study").unwrap();
    let execute = evaluate(Arc::clone(&loaded), "d".into(), "study".into(), vec![Value::int(4)], engine(), options());
    let first = d.open("cancel", Target { function: "study".into(), ..target(input.clone()) }, execute).unwrap();
    let _ = d.debug_step("cancel", &first.stop_id, "c1").unwrap();
    let _ = d.cancel("cancel", Interruption::ByUser).unwrap();
    let execute = evaluate(Arc::clone(&loaded), "d".into(), "study".into(), vec![Value::int(4)], engine(), options());
    let first = d.open("safety", Target { function: "study".into(), ..target(input.clone()) }, execute).unwrap();
    let _ = d.debug_step("safety", &first.stop_id, "f1").unwrap();
    let _ = d.safety_stop("safety").unwrap();
    for run in d.runs() {
        d.close(&run);
    }

    // Journal travel, freshness and invocation recovery through the query rows.
    {
        let guard = live.lock().unwrap();
        let rows = Rows(&guard.layer);
        let mut travel = Travel::new(&rows);
        let _ = travel.open(0).unwrap();
        let _ = travel.page(None, 10, None).unwrap();
        let _ = travel.head().unwrap();
        let _ = travel.records().unwrap();
        let _ = travel.checkpoints().unwrap_err();
        let _ = travel.recover("shell:9", "unknown").unwrap();
        let _ = freshness(&input, &rows).unwrap();
    }

    // Replay: a safety-stopped record is not run, and a record whose workspace write was not captured is refused
    // rather than written; replay has no live host at all.
    let stopped = RunRecord {
        name: "keep".into(),
        key: ReproKey {
            sources: vec![ModuleRef { name: "d".into(), digest: sha256_hex(SOURCE.as_bytes()) }],
            edition: c600_syntax::EDITION.into(),
            ir_version: c600_ir::IR_VERSION.into(),
            execution_contract: c600_ir::EXEC_CONTRACT.into(),
            input: input.clone(),
            module: "d".into(),
            function: "keep".into(),
            args: vec![word()],
            seed: None,
            limits: WIDE,
            cost_units: c600_debug::replay::TEST_COST_UNITS.into(),
        },
        events: Vec::new(),
        result: Recorded::safety_stopped(Amounts::default()),
    };
    let mut replayer = Replayer::new(identity());
    replayer.hold(SOURCE.as_bytes());
    assert!(matches!(replayer.replay_run(&stopped).unwrap().comparison, Comparison::Incomplete { .. }));
    let mut uncaptured = stopped.clone();
    uncaptured.result.code = None;
    uncaptured.result.outcome = c600_run::meter::Outcome::Completed;
    assert_eq!(replayer.replay_run(&uncaptured).unwrap_err().code, DEBUG_WRITE_FORBIDDEN);

    assert_eq!(writes.load(Ordering::SeqCst), 0, "no write reached the live engine");
    assert_eq!(live.lock().unwrap().durable(), before, "no journal, workspace or session digest changed");
}

/// A research engine pinned to an input: the probe row answers the state revision it was pinned at, after waiting
/// for a release, so the run stays suspended at that call while the solver works.
struct PinnedEngine {
    state: u64,
    release: Receiver<()>,
}

impl Host for PinnedEngine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 1, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let _ = self.release.recv();
        match request {
            Request::Snapshot { query: "probe", .. } => Ok(Value::int(self.state)),
            _ => Err(meter_code::NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(meter_code::NOT_GRANTED)
    }
}

/// CT-EV2-09 (whole): journal travel reads the query rows and leaves the active line unchanged; an unavailable revision
/// stays explicit; a digest-pinned research run suspended at its kernel call finishes on its old input across a commit
/// and across an undo, and moving the travel cursor does not make it current.
#[test]
fn ct_ev2_09_travel_reads_rows_and_a_pinned_run_keeps_its_input() {
    let (lowered, loaded) = build();
    let mut live = Live::new("ev2-09");
    assert!(matches!(live.commit(&[1]), Status::Committed(_)));
    assert!(matches!(live.commit(&[2, 3]), Status::Committed(_)));

    // Travel through the query rows: the cursor moves, the head does not.
    let before = live.durable();
    let head = live.layer.revision().head.clone();
    {
        let rows = Rows(&live.layer);
        let page = rows.journal(None, 10, None).unwrap();
        let orders: Vec<u64> = page.entries.iter().map(|e| e.order).collect();
        assert_eq!(page.entries[0].event, head);
        assert_eq!(orders.last(), Some(&0));
        let mut travel = Travel::new(&rows);
        let start = travel.open(0).unwrap().clone();
        assert_eq!((start.entry.kind.as_str(), start.at_head), ("start", false));
        let opened = travel.open(orders[1]).unwrap().clone();
        assert_eq!(opened.entry.kind, "commit");
        // An event the journal does not hold is explicit, and the cursor stays where it was.
        let gone = travel.open(99).unwrap_err();
        assert_eq!(gone.code, REVISION_GONE);
        assert_eq!(travel.cursor(), Some(&opened), "never silently the head");
        assert_eq!(travel.checkpoints().unwrap_err().code, code::ROW_NOT_BUILT);
        assert_eq!(travel.head().unwrap().head, head);
    }
    assert_eq!(live.durable(), before, "travel changed nothing");

    // A research run pinned at the head, suspended at its probe read while the solver commits, then while the solver
    // undoes: each finishes on the input it was pinned to, and its result is stale afterwards.
    let options = || {
        let executed = Executed::new(&lowered, &[SOURCE.as_bytes()]);
        RunOptions { sources: Some(Arc::new(executed)), ..RunOptions::new(WIDE) }
    };
    let mut d = Debugger::new();
    for (run, change) in [("across-commit", true), ("across-undo", false)] {
        let input = pinned(&live);
        let state = live.layer.revision().state;
        let (release, waiting) = mpsc::channel();
        let engine = Box::new(PinnedEngine { state, release: waiting });
        let execute =
            evaluate(Arc::clone(&loaded), "d".into(), "study".into(), vec![Value::int(1000)], engine, options());
        let mut at = d.open(run, Target { function: "study".into(), ..target(input.clone()) }, execute).unwrap();
        let mut n = 0;
        while !matches!(at.state, State::Waiting { .. }) {
            n += 1;
            at = d.debug_step(run, &at.stop_id, &format!("{run}{n}")).unwrap();
        }
        let status = if change { live.commit(&[5]) } else { live.undo() };
        assert!(matches!(status, Status::Committed(_)), "{run}: {status:?}");
        assert_ne!(live.layer.revision().state, state);
        release.send(()).unwrap();
        let waited = d.wait(run).unwrap();
        let end = to_end(&mut d, run, waited);
        let State::Terminal(t) = &end.state else { panic!("{end:?}") };
        assert_eq!(t.reason, Reason::Completed, "{run}");
        let value = d.inspect_value(run, &end.values[0].reference).unwrap();
        assert_eq!(value.shown.text, format!("{{\"int\":\"{}\"}}", state + 1000), "{run}: the old input");
        assert_eq!(d.target(run).unwrap().input, input, "{run}: the session keeps its input");

        let rows = Rows(&live.layer);
        let stale = freshness(&input, &rows).unwrap();
        assert!(matches!(&stale, Freshness::Stale { pinned, .. } if *pinned == input.digest), "{run}: {stale:?}");
        // Moving the travel cursor to the head does not make the old result current.
        let head_order = rows.journal(None, 1, None).unwrap().entries[0].order;
        let mut travel = Travel::new(&rows);
        assert!(travel.open(head_order).unwrap().at_head);
        assert_eq!(freshness(&input, &rows).unwrap(), stale);
    }
}
