//! EV3.2's view rows in the command layer (RPI.81 to RPI.83; LP2 draft): `view.open` and `view.close` reach the host's
//! view model ([`Views`]) only inside a command the solver invoked, after the layer checked their argument forms, and
//! the host's answers reach the receipt. The host here is a recording stand-in for `c600-view`'s `ViewModel`, with a
//! fixture budget of two further views standing in for RPI.82's ceilings. CT-RPI-36's layer half and CT-EV3-07's view
//! half on the layer: an open or close writes nothing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use c600_command::{
    code, Caller, CommandLayer, Error, Invocation, Kind, MemoryStore, Overlay, Receipt, Secret, Shell, Status, Table,
    ViewSourceRef, Views, VIEW_CLOSE_ROW, VIEW_OPEN_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const IN_USE: &str = "EV3:input.view_id_in_use";
const NOT_CLOSABLE: &str = "EV3:input.view_not_closable";
const CEILING: &str = "EV3:resource_limit.display_plan_ceiling";
const MODULE: &str = "5c5d2b0e6f1d8a7a3e4b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap()
}

/// What the host's view model saw, and the further views it holds open (at most two: the fixture budget).
#[derive(Default)]
struct Model {
    open: BTreeSet<String>,
    calls: Vec<String>,
}

#[derive(Clone, Default)]
struct Host(Arc<Mutex<Model>>);

impl Host {
    fn calls(&self) -> Vec<String> {
        self.0.lock().unwrap().calls.clone()
    }
}

impl Views for Host {
    fn open(&mut self, command: &str, view: &str, source: &ViewSourceRef) -> Result<(), Error> {
        let mut model = self.0.lock().unwrap();
        model.calls.push(format!("open {command} {view} {source:?}"));
        if view == "global" || view == "local" || model.open.contains(view) {
            return Err(Error::new(IN_USE, view.to_owned()));
        }
        if model.open.len() == 2 {
            return Err(Error::new(CEILING, "view_vertex_evals"));
        }
        model.open.insert(view.to_owned());
        Ok(())
    }

    fn close(&mut self, command: &str, view: &str) -> Result<(), Error> {
        let mut model = self.0.lock().unwrap();
        model.calls.push(format!("close {command} {view}"));
        if model.open.remove(view) {
            Ok(())
        } else {
            Err(Error::new(NOT_CLOSABLE, view.to_owned()))
        }
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    host: Host,
}

/// A session at the solved state on H-08's table with an overlay view row `tool.panel {cap: CellId}` and an overlay
/// query row `tool.probe`, and the host's view model when `views`.
fn fixture(views: bool) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let overlay = |entry: &str| Overlay { module: "tools/views".into(), entry: entry.into() };
    layer.register_overlay("tool.panel", Kind::View, &[("cap", "CellId")], overlay("panel")).unwrap();
    layer.register_overlay("tool.probe", Kind::Query, &[], overlay("probe")).unwrap();
    let host = Host::default();
    if views {
        layer.set_views(Box::new(host.clone()));
    }
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), host }
}

fn open_args(view: &str, source: &str) -> Value {
    parse(&format!(r#"{{"view": "{view}", "source": {source}}}"#))
}

fn close_args(view: &str) -> Value {
    parse(&format!(r#"{{"view": "{view}"}}"#))
}

fn built_in(name: &str) -> String {
    format!(r#"{{"built_in": "{name}"}}"#)
}

fn refusal(receipt: &Receipt) -> &'static str {
    assert_eq!(receipt.job, None, "a refusal takes no job: {:?}", receipt.status);
    receipt.status.code().unwrap_or_else(|| panic!("not refused: {:?}", receipt.status))
}

fn accepted(receipt: &Receipt) {
    assert_eq!((receipt.job, &receipt.status), (None, &Status::Accepted));
}

impl Fixture {
    fn invoke(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    fn from_run(&mut self, run: &str, command: &str, row: &str, args: Value) -> Receipt {
        let epoch = self.layer.revision().epoch;
        let served = self.layer.table().row(row).is_some_and(|found| found.overlay.is_some());
        let invocation = Invocation {
            command: command.into(),
            row: row.into(),
            args,
            token: None,
            epoch,
            budget: served.then_some(c600_command::Budget::CEILING),
        };
        self.layer.invoke(&Caller::Runtime { run: run.into() }, invocation)
    }

    /// The layer's revision, records and run jobs: what an open or close must leave as it was.
    fn fixed(&mut self) -> (String, usize, usize) {
        let revision = format!("{:?}", self.layer.revision());
        (revision, self.layer.records().len(), self.layer.run_jobs())
    }
}

/// Without the host's view model, both rows are unavailable and refused with `KN5:input.row_not_built`, as before.
#[test]
fn without_a_view_model_both_rows_are_not_built() {
    let mut f = fixture(false);
    for row in [VIEW_OPEN_ROW, VIEW_CLOSE_ROW] {
        assert_eq!(f.layer.availability(row).map_err(|e| e.code), Err(code::ROW_NOT_BUILT), "{row}");
    }
    assert_eq!(refusal(&f.invoke(VIEW_OPEN_ROW, open_args("chart", &built_in("orbit.chart")))), code::ROW_NOT_BUILT);
    assert_eq!(refusal(&f.invoke(VIEW_CLOSE_ROW, close_args("chart"))), code::ROW_NOT_BUILT);
}

/// The solver opens and closes a further view: each row reaches the host once with its command ID and answers
/// `accepted` with no job, and neither writes (CT-EV3-07's view half): the revision, the records and the jobs stay.
#[test]
fn the_solver_opens_and_closes_a_further_view_and_nothing_is_written() {
    let mut f = fixture(true);
    for row in [VIEW_OPEN_ROW, VIEW_CLOSE_ROW] {
        assert_eq!(f.layer.availability(row), Ok(()), "{row}");
    }
    let before = f.fixed();
    let open = f.invoke(VIEW_OPEN_ROW, open_args("chart", &built_in("orbit.chart")));
    accepted(&open);
    let source = format!(r#"{{"overlay": {{"row": "tool.panel", "module": "{MODULE}", "arguments": {{"cap": 7}}}}}}"#);
    let panel = f.invoke(VIEW_OPEN_ROW, open_args("panel", &source));
    accepted(&panel);
    let close = f.invoke(VIEW_CLOSE_ROW, close_args("chart"));
    accepted(&close);
    assert_eq!(f.fixed(), before, "an open or close writes nothing");
    let arguments = [("cap".to_owned(), Value::Int(7))].into_iter().collect();
    let overlay = ViewSourceRef::Overlay { row: "tool.panel".into(), module: MODULE.into(), arguments };
    assert_eq!(
        f.host.calls(),
        [
            format!("open {} chart {:?}", open.command, ViewSourceRef::BuiltIn { name: "orbit.chart".into() }),
            format!("open {} panel {overlay:?}", panel.command),
            format!("close {} chart", close.command),
        ]
    );
}

/// The host's refusals reach the receipt unchanged: an open view, Global or Local named by `view.open`
/// (`EV3:input.view_id_in_use`), Global, Local or an unknown view named by `view.close`
/// (`EV3:input.view_not_closable`), and an open past the budget (`EV3:resource_limit.display_plan_ceiling`).
#[test]
fn the_view_models_refusals_reach_the_receipt() {
    let mut f = fixture(true);
    accepted(&f.invoke(VIEW_OPEN_ROW, open_args("a", &built_in("orbit.chart"))));
    for view in ["a", "global", "local"] {
        assert_eq!(refusal(&f.invoke(VIEW_OPEN_ROW, open_args(view, &built_in("orbit.chart")))), IN_USE, "{view}");
    }
    for view in ["global", "local", "unknown"] {
        assert_eq!(refusal(&f.invoke(VIEW_CLOSE_ROW, close_args(view))), NOT_CLOSABLE, "{view}");
    }
    accepted(&f.invoke(VIEW_OPEN_ROW, open_args("b", &built_in("orbit.chart"))));
    assert_eq!(refusal(&f.invoke(VIEW_OPEN_ROW, open_args("c", &built_in("orbit.chart")))), CEILING);
    accepted(&f.invoke(VIEW_CLOSE_ROW, close_args("a")));
    accepted(&f.invoke(VIEW_OPEN_ROW, open_args("c", &built_in("orbit.chart"))));
}

/// A retried delivery returns the stored receipt and never reaches the host again (KN5.21).
#[test]
fn a_retry_returns_the_stored_receipt_and_the_host_sees_one_call() {
    let mut f = fixture(true);
    for (row, args) in [(VIEW_OPEN_ROW, open_args("a", &built_in("orbit.chart"))), (VIEW_CLOSE_ROW, close_args("a"))] {
        let invocation = f.shell.prepare(&f.layer, row, args).unwrap();
        let first = f.shell.send(&mut f.layer, invocation.clone());
        accepted(&first);
        assert_eq!(f.shell.send(&mut f.layer, invocation), first, "{row}");
    }
    assert_eq!(f.host.calls().len(), 2);
}

/// Malformed arguments are refused with `KN5:input.argument_form` before the host is called: a `ViewId` that is not a
/// nonempty text, a `ViewSourceRef` of another form, and an overlay source that does not name an overlay row of kind
/// `view` or whose arguments do not fit that row.
#[test]
fn argument_forms_are_checked_before_the_host() {
    let mut f = fixture(true);
    let overlay = |row: &str, module: &str, arguments: &str| {
        format!(r#"{{"overlay": {{"row": "{row}", "module": "{module}", "arguments": {arguments}}}}}"#)
    };
    let refused_open = [
        parse(r#"{"view": "", "source": {"built_in": "orbit.chart"}}"#),
        parse(r#"{"view": 3, "source": {"built_in": "orbit.chart"}}"#),
        open_args("a", "{}"),
        open_args("a", r#"{"built_in": ""}"#),
        open_args("a", r#"{"built_in": 3}"#),
        open_args("a", r#""orbit.chart""#),
        open_args("a", &format!(r#"{{"built_in": "x", "overlay": {}}}"#, overlay("tool.panel", MODULE, "{}"))),
        open_args("a", &overlay("tool.panel", "abc", r#"{"cap": 7}"#)),
        open_args("a", &overlay("tool.panel", &MODULE.to_uppercase(), r#"{"cap": 7}"#)),
        open_args("a", &overlay("tool.panel", MODULE, r#"[7]"#)),
        open_args("a", &overlay("tool.probe", MODULE, "{}")),
        open_args("a", &overlay("tool.missing", MODULE, "{}")),
        open_args("a", &overlay("structure.compare", MODULE, "{}")),
        open_args("a", &overlay("tool.panel", MODULE, "{}")),
        open_args("a", &overlay("tool.panel", MODULE, r#"{"cap": 7, "extra": 1}"#)),
        open_args("a", &overlay("tool.panel", MODULE, r#"{"cap": 600}"#)),
        open_args("a", &overlay("tool.panel", MODULE, r#"{"cap": "7"}"#)),
    ];
    for args in refused_open {
        let shown = format!("{args:?}");
        assert_eq!(refusal(&f.invoke(VIEW_OPEN_ROW, args)), code::ARGUMENT_FORM, "{shown}");
    }
    for args in [parse(r#"{"view": ""}"#), parse(r#"{"view": null}"#)] {
        let shown = format!("{args:?}");
        assert_eq!(refusal(&f.invoke(VIEW_CLOSE_ROW, args)), code::ARGUMENT_FORM, "{shown}");
    }
    assert_eq!(f.host.calls(), Vec::<String>::new(), "the host saw none of them");
    // The hint answers the forms as the invocation does.
    let hint = |args: Value| f.layer.availability_for(VIEW_OPEN_ROW, &args).map_err(|error| error.code);
    assert_eq!(hint(parse(r#"{"view": "", "source": {"built_in": "x"}}"#)), Err(code::ARGUMENT_FORM));
    assert_eq!(hint(open_args("a", "{}")), Err(code::ARGUMENT_FORM));
    assert_eq!(hint(open_args("a", &built_in("orbit.chart"))), Ok(()));
    accepted(&f.invoke(VIEW_OPEN_ROW, open_args("a", &overlay("tool.panel", MODULE, r#"{"cap": 599}"#))));
    // A run's malformed request is refused for its form before its caller is judged.
    assert_eq!(refusal(&f.from_run("recompute/1", "r1", VIEW_CLOSE_ROW, close_args(""))), code::ARGUMENT_FORM);
}

/// Either row runs only inside a command the solver invoked (CT-RPI-36's layer half). A run that is not a program
/// job's run the host holds, as a recomputation started by a revision change is not, is refused with
/// `LG3:capability.not_granted` and never reaches the host, so a view cannot open views. A run of a job the solver
/// invoked may open and close a view while the host holds its job, and not after the job is reported.
#[test]
fn only_a_run_inside_a_solver_command_reaches_the_view_model() {
    let mut f = fixture(true);
    let open = open_args("a", &built_in("orbit.chart"));
    assert_eq!(refusal(&f.from_run("recompute/1", "r1", VIEW_OPEN_ROW, open.clone())), code::NOT_GRANTED);
    assert_eq!(refusal(&f.from_run("recompute/1", "r2", VIEW_CLOSE_ROW, close_args("a"))), code::NOT_GRANTED);
    assert!(f.host.calls().is_empty());

    let probe = f.invoke("tool.probe", json::object([]));
    let Status::Running(job) = probe.status else { panic!("{:?}", probe.status) };
    let taken = f.layer.take_program_job().expect("the probe's job");
    assert_eq!(taken.job, job);
    let run = taken.run();
    accepted(&f.from_run(&run, "p1", VIEW_OPEN_ROW, open.clone()));
    accepted(&f.from_run(&run, "p2", VIEW_CLOSE_ROW, close_args("a")));
    assert_eq!(f.host.calls().len(), 2);

    // A job the solver's job invoked is inside the solver's command too, while both jobs are held.
    f.layer.set_program_capacity(2);
    let nested = f.from_run(&run, "p3", "tool.probe", json::object([]));
    let Status::Running(inner_job) = nested.status else { panic!("{:?}", nested.status) };
    let inner = f.layer.take_program_job().expect("the nested job");
    assert_eq!(inner.job, inner_job);
    accepted(&f.from_run(&inner.run(), "n1", VIEW_OPEN_ROW, open_args("b", &built_in("orbit.chart"))));
    assert_eq!(f.host.calls().len(), 3);

    f.layer.finish_program_job(job, Ok(Value::Null)).expect("reported");
    assert_eq!(refusal(&f.from_run(&run, "p4", VIEW_OPEN_ROW, open.clone())), code::NOT_GRANTED);
    let after = open_args("c", &built_in("orbit.chart"));
    assert_eq!(refusal(&f.from_run(&inner.run(), "n2", VIEW_OPEN_ROW, after)), code::NOT_GRANTED);
    assert_eq!(f.host.calls().len(), 3);
}
