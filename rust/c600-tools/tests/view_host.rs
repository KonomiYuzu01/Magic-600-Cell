//! The command layer's view rows on `c600-view`'s view model (LP3; LP2's pull request 271): `view.open` and
//! `view.close`, through the shell and through a program's run, reach the host's view model through
//! [`ModelViews`], and the model's answers reach the receipt. Each test is named after its conformance ID and its doc
//! comment names the part it covers. Every ceiling is a declared fixture value: no device, renderer or timing is
//! involved.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use c600_command::{
    code, Caller, CommandLayer, Error as CommandError, Invocation, Kind, MemoryStore, Overlay, Receipt, Secret, Shell,
    Status, Table, ViewSourceRef, Views, VIEW_CLOSE_ROW, VIEW_OPEN_ROW,
};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;
use c600_tools::tool::{manifest, BoundaryClass, MODULE};
use c600_tools::views::{declared, model_revision, ModelViews};
use c600_view as view;

const SECRET: [u8; 32] = [7; 32];
const BIG: u64 = 1 << 40;
const PANEL: &str = "tool.panel";
/// The source text that stands in for the panel's checked module; its SHA-256 is the overlay's module digest.
const PANEL_SOURCE: &[u8] = b"tools/views panel";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| Arc::new(Kernel::load(&repository().join("assets")).expect("the model in assets/ loads")))
        .clone()
}

fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).expect("a JSON document")
}

/// A complete fixture profile of the view model: RPI.82's `view_vertex_evals` and `adapter_cpu_bytes` at two further
/// views, `overdraw` 1 and every other field generous.
fn profile() -> view::LimitProfile {
    let value = |field: view::Field| match field {
        view::Field::Overdraw => 1,
        view::Field::ViewVertexEvals | view::Field::AdapterCpuBytes => 2,
        _ => BIG,
    };
    let entries: Vec<(&str, u64, &str)> =
        view::Field::ALL.iter().map(|field| (field.name(), value(*field), field.unit())).collect();
    view::LimitProfile::from_entries("fixture-2", &entries).expect("a complete fixture profile")
}

/// The view model's candidate bundle of the layer's current revision. The labels part is the state's label digest;
/// the metadata and styles parts are fixture digests bound to the revision.
fn candidate(layer: &CommandLayer) -> view::BundleCandidate {
    let revision = model_revision(layer.revision()).expect("a label digest");
    let part = |kind: view::PartKind, digest: [u8; 32]| view::BundlePart { kind, revision, digest };
    let fixture = |name: &str| sha256(format!("{name} of {}", layer.revision().to_json().canonical()).as_bytes());
    let parts = vec![
        part(view::PartKind::Metadata, fixture("metadata")),
        part(view::PartKind::Labels, sha256(layer.revision().digest.as_bytes())),
        part(view::PartKind::Styles, fixture("styles")),
    ];
    view::BundleCandidate { revision, mode: view::Mode::JumbleOff, parts }
}

/// A further view's declared workload: one vertex evaluation per frame and one resident byte on each adapter.
const WORKLOAD: view::Workload =
    view::Workload { vertex_evals: 1, stage_ops: 0, fragment_ops: 0, cpu_bytes: 1, gpu_bytes: 1, pixels: 0 };

/// The host's declarations: the tool's built-in source under the manifest's `checking` class, and the overlay row
/// `tool.panel` as a solver's own tool, with no class (BND.27). Any other source is refused with
/// `KN5:input.argument_form`, naming the argument.
fn declare(source: &ViewSourceRef) -> Result<view::Declared, CommandError> {
    let output = view::OutputAllowance::new(BIG, 16);
    match source {
        ViewSourceRef::BuiltIn { name } if name == MODULE => {
            Ok(declared(&manifest(), BoundaryClass::Checking, WORKLOAD, output).expect("the manifest declares it"))
        }
        ViewSourceRef::Overlay { row, .. } if row == PANEL => {
            Ok(view::Declared { depends: view::Depends::STATE, workload: WORKLOAD, output, class: None })
        }
        _ => Err(CommandError::new(code::ARGUMENT_FORM, "source")),
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    views: ModelViews,
}

/// A session at the solved state on H-08's table with an overlay view row `tool.panel {cap: CellId}` and an overlay
/// query row `tool.probe`, and the host's view model at the layer's revision, Global and Local open, behind the
/// layer's view rows.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).expect("no protected piece"));
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let overlay = |entry: &str| Overlay { module: "tools/views".into(), entry: entry.into() };
    layer.register_overlay(PANEL, Kind::View, &[("cap", "CellId")], overlay("panel")).expect("a view row");
    layer.register_overlay("tool.probe", Kind::Query, &[], overlay("probe")).expect("a query row");
    let none = view::Workload::default();
    let model = view::ViewModel::new(view::Profile::Solving, profile(), &candidate(&layer), none, none)
        .expect("Global and Local fit");
    let views = ModelViews::new(Arc::new(Mutex::new(model)), declare);
    layer.set_views(Box::new(views.clone()));
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), views }
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

fn panel(cap: i64) -> String {
    let module = sha256_hex(PANEL_SOURCE);
    format!(r#"{{"overlay": {{"row": "{PANEL}", "module": "{module}", "arguments": {{"cap": {cap}}}}}}}"#)
}

fn accepted(receipt: &Receipt) {
    assert_eq!((receipt.job, &receipt.status), (None, &Status::Accepted));
}

/// The code and detail of a refusal, which takes no job.
fn refused(receipt: &Receipt) -> (&'static str, String) {
    assert_eq!(receipt.job, None, "a refusal takes no job: {:?}", receipt.status);
    match &receipt.status {
        Status::Refused(error) => (error.code, error.detail.clone()),
        other => panic!("not refused: {other:?}"),
    }
}

impl Fixture {
    fn invoke(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).expect("delivered")
    }

    fn open(&mut self, view: &str, source: &str) -> Receipt {
        self.invoke(VIEW_OPEN_ROW, open_args(view, source))
    }

    fn close(&mut self, view: &str) -> Receipt {
        self.invoke(VIEW_CLOSE_ROW, close_args(view))
    }

    fn invoke_on_run(&mut self, run: &str, command: &str, row: &str, args: Value) -> Receipt {
        let epoch = self.layer.revision().epoch;
        let served = self.layer.table().row(row).is_some_and(|found| found.overlay.is_some());
        let invocation = Invocation {
            command: command.into(),
            row: row.into(),
            args,
            token: None,
            epoch,
            budget: served.then_some(c600_command::Budget::DRAFT_CEILING),
        };
        self.layer.invoke(&Caller::Runtime { run: run.into() }, invocation)
    }

    fn model(&self) -> MutexGuard<'_, view::ViewModel> {
        self.views.model().lock().expect("the view model")
    }

    /// The model's open views, in open order.
    fn open_views(&self) -> Vec<String> {
        self.model().views().iter().map(|id| id.as_str().to_owned()).collect()
    }

    /// The layer's revision, records and run jobs: what an open or close must leave as it was.
    fn fixed(&mut self) -> (String, usize, usize) {
        let revision = format!("{:?}", self.layer.revision());
        (revision, self.layer.records().len(), self.layer.run_jobs())
    }
}

/// CT-EV3-08 (part: through the command layer onto the view model, the solver opens a further view from the tool's
/// built-in source and one from an overlay row of kind `view`, each answered `accepted` with no job; the model holds
/// both, in open order after Global and Local, with their sources, an overlay's module digest as its 32 bytes and its
/// arguments as canonical JSON, and schedules each view's first run under a command ID of its own; a close supersedes
/// the closed view's first run, whose cancel reason names the layer's command that closed it, and its resources stay
/// charged until the host completes their retirement; nothing is written: the layer's revision, records and run jobs
/// stay; not run: RM.39's measured ceilings on the selected renderer, C9's), EV3.2, RPI.81, RPI.83, RM.19, LI.23,
/// LI.25.
#[test]
fn ct_ev3_08_the_view_rows_open_and_close_views_on_the_view_model() {
    let mut f = fixture();
    for row in [VIEW_OPEN_ROW, VIEW_CLOSE_ROW] {
        assert_eq!(f.layer.availability(row), Ok(()), "{row}");
    }
    let before = f.fixed();
    let chart = f.open("chart", &built_in(MODULE));
    accepted(&chart);
    let shown = f.open("panel", &panel(7));
    accepted(&shown);
    assert_eq!(f.open_views(), ["global", "local", "chart", "panel"]);
    let first = {
        let model = f.model();
        let (chart_id, panel_id) = (view::ViewId::new("chart"), view::ViewId::new("panel"));
        assert_eq!(model.source(&chart_id), Some(&view::ViewSource::BuiltIn { name: MODULE.into() }));
        let overlay = view::ViewSource::Overlay {
            row: PANEL.into(),
            module: sha256(PANEL_SOURCE),
            arguments: br#"{"cap":7}"#.to_vec(),
        };
        assert_eq!(model.source(&panel_id), Some(&overlay));
        let run = |id: &view::ViewId| model.runner(id).map(|(_, command)| command).or_else(|| model.pending(id));
        let (first, other) = (run(&chart_id), run(&panel_id));
        for scheduled in [first, other] {
            assert!(matches!(scheduled, Some(view::CommandId::Scheduled(_))), "the model's own run: {scheduled:?}");
        }
        assert_ne!(first, other, "each view's first run has a command ID of its own");
        first.expect("the chart's first run")
    };
    assert_eq!(f.views.command(view::CommandId::Caller(1)), Some(chart.command.clone()));
    assert_eq!(f.views.command(view::CommandId::Caller(2)), Some(shown.command.clone()));

    let close = f.close("chart");
    accepted(&close);
    assert_eq!(f.open_views(), ["global", "local", "panel"]);
    let receipt = f.model().receipt(first).cloned();
    let Some(view::Status::Cancelled(view::CancelReason::Superseded { by })) = receipt else {
        panic!("the chart's first run was not superseded: {receipt:?}")
    };
    assert_eq!(f.views.command(by), Some(close.command.clone()), "the cancel reason names the closing command");
    assert_eq!(f.model().retiring(), 1, "the closed view's resources stay charged");
    let tokens = f.views.take_retiring();
    assert_eq!(tokens.len(), 1);
    assert!(f.views.take_retiring().is_empty(), "each token is taken once");
    for token in tokens {
        assert!(f.model().complete_retirement(token));
    }
    assert_eq!(f.model().retiring(), 0);
    assert_eq!(f.fixed(), before, "an open or close writes nothing");
}

/// CT-EV3-08 (part: through the command layer, an open past RPI.82's budget is refused with the model's
/// `EV3:resource_limit.display_plan_ceiling`, naming the field, and nothing is queued; a closed view's resources stay
/// charged under `adapter_cpu_bytes` until the host completes their retirement, and the same open fits after it; not
/// run: RM.39's measured ceilings on the selected renderer, C9's), RPI.82, RM.19, RM.39.
#[test]
fn ct_ev3_08_an_open_past_the_budget_is_refused_until_retirement_completes() {
    let mut f = fixture();
    accepted(&f.open("a", &built_in(MODULE)));
    accepted(&f.open("b", &panel(1)));
    let ceiling = view::code::DISPLAY_PLAN_CEILING;
    assert_eq!(refused(&f.open("c", &built_in(MODULE))), (ceiling, "view_vertex_evals".into()));
    assert_eq!(f.open_views(), ["global", "local", "a", "b"], "nothing is queued");
    accepted(&f.close("a"));
    assert_eq!(refused(&f.open("c", &built_in(MODULE))), (ceiling, "adapter_cpu_bytes".into()));
    assert_eq!(f.open_views(), ["global", "local", "b"]);
    for token in f.views.take_retiring() {
        assert!(f.model().complete_retirement(token));
    }
    accepted(&f.open("c", &built_in(MODULE)));
    assert_eq!(f.open_views(), ["global", "local", "b", "c"]);
}

/// CT-RPI-36 (part: through the command layer onto the view model, the model's refusals reach the receipt with the
/// view as detail: `view.open` of an open view, Global or Local is `EV3:input.view_id_in_use`, and `view.close` of
/// Global, Local or an unknown view is `EV3:input.view_not_closable`; a source the host does not serve is refused with
/// `KN5:input.argument_form` before the model; no refusal changes the open views), RPI.81, RPI.83.
#[test]
fn ct_rpi_36_the_view_models_refusals_reach_the_receipt() {
    let mut f = fixture();
    accepted(&f.open("a", &built_in(MODULE)));
    for name in ["a", "global", "local"] {
        assert_eq!(refused(&f.open(name, &built_in(MODULE))), (view::code::VIEW_ID_IN_USE, name.into()));
    }
    for name in ["global", "local", "unknown"] {
        assert_eq!(refused(&f.close(name)), (view::code::VIEW_NOT_CLOSABLE, name.into()));
    }
    assert_eq!(refused(&f.open("b", &built_in("orbit.chart"))), (code::ARGUMENT_FORM, "source".into()));
    assert_eq!(f.open_views(), ["global", "local", "a"]);
    assert_eq!(f.views.command(view::CommandId::Caller(8)), None, "the unserved source reached no model command");
}

/// A retried delivery returns the stored receipt and never reaches the view model again (KN5.21): each retry's receipt
/// equals the first, which a second call would change (the model refuses a second open of the view with
/// `EV3:input.view_id_in_use` and a second close with `EV3:input.view_not_closable`); the host numbered two commands,
/// and the view is closed.
#[test]
fn a_retry_returns_the_stored_receipt_and_the_view_model_sees_one_call() {
    let mut f = fixture();
    let mut receipts = Vec::new();
    for (row, args) in [(VIEW_OPEN_ROW, open_args("a", &built_in(MODULE))), (VIEW_CLOSE_ROW, close_args("a"))] {
        let invocation = f.shell.prepare(&f.layer, row, args).expect("prepared");
        let first = f.shell.send(&mut f.layer, invocation.clone());
        accepted(&first);
        assert_eq!(f.shell.send(&mut f.layer, invocation), first, "{row}");
        receipts.push(first);
    }
    assert_eq!(f.views.command(view::CommandId::Caller(1)), Some(receipts[0].command.clone()));
    assert_eq!(f.views.command(view::CommandId::Caller(2)), Some(receipts[1].command.clone()));
    assert_eq!(f.views.command(view::CommandId::Caller(3)), None);
    assert_eq!(f.open_views(), ["global", "local"]);
}

/// CT-RPI-36 (part: through the command layer onto the view model, either row reaches the model only inside a command
/// the solver invoked: a run that is not a program job's run the host holds, as a recomputation started by a revision
/// change is not, is refused with `LG3:capability.not_granted` and the model is unchanged; a run of a job the solver
/// invoked opens and closes a view while the host holds its job, and not after the job is reported), RPI.83, BND.30.
#[test]
fn ct_rpi_36_only_a_run_inside_a_solver_command_reaches_the_view_model() {
    let mut f = fixture();
    let open = open_args("a", &built_in(MODULE));
    assert_eq!(refused(&f.invoke_on_run("recompute/1", "r1", VIEW_OPEN_ROW, open.clone())).0, code::NOT_GRANTED);
    assert_eq!(refused(&f.invoke_on_run("recompute/1", "r2", VIEW_CLOSE_ROW, close_args("a"))).0, code::NOT_GRANTED);
    assert_eq!(f.open_views(), ["global", "local"]);

    let probe = f.invoke("tool.probe", json::object([]));
    let Status::Running(job) = probe.status else { panic!("{:?}", probe.status) };
    let taken = f.layer.take_program_job().expect("the probe's job");
    assert_eq!(taken.job, job);
    let run = taken.run();
    accepted(&f.invoke_on_run(&run, "p1", VIEW_OPEN_ROW, open.clone()));
    assert_eq!(f.open_views(), ["global", "local", "a"]);
    assert_eq!(f.views.command(view::CommandId::Caller(1)).as_deref(), Some("p1"));
    accepted(&f.invoke_on_run(&run, "p2", VIEW_CLOSE_ROW, close_args("a")));
    assert_eq!(f.open_views(), ["global", "local"]);

    f.layer.finish_program_job(job, Ok(Value::Null)).expect("reported");
    assert_eq!(refused(&f.invoke_on_run(&run, "p3", VIEW_OPEN_ROW, open)).0, code::NOT_GRANTED);
    assert_eq!(f.open_views(), ["global", "local"]);
}

/// The host takes only 64 hex digits as a digest, a sign excluded, even from a caller that bypasses the layer's form
/// check: an overlay source whose module digest spells `+0` 32 times is refused with `KN5:input.argument_form`, naming
/// `source`, before the model, and `model_revision` refuses a revision whose label digest spells it.
#[test]
fn the_host_takes_only_hex_digests() {
    let f = fixture();
    let signed = "+0".repeat(32);
    let source = ViewSourceRef::Overlay { row: PANEL.into(), module: signed.clone(), arguments: Default::default() };
    let mut views = f.views.clone();
    let refusal = views.open("direct", "a", &source).map_err(|error| (error.code, error.detail));
    assert_eq!(refusal, Err((code::ARGUMENT_FORM, "source".to_owned())));
    assert_eq!(f.open_views(), ["global", "local"]);
    assert_eq!(f.views.command(view::CommandId::Caller(1)), None, "the refused source reached no model command");
    let mut head = f.layer.revision().clone();
    head.digest = signed;
    assert!(model_revision(&head).is_err());
}
