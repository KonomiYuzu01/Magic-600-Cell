//! The host's routes of a user module's runs (chapter 04 EV1.7, EV1.8, EV1.11; LP4-2): a program job of the module's
//! overlay row, invoked from the palette through the shell or from a key through the input router, binds the active
//! version when the host takes it and runs in process or in the runtime's own process; a view source that names a
//! module digest opens only on the active version. Every key event here is synthetic; none of this is Windows input
//! evidence.
//!
//! CT-EV1-04 (source, whole by union with `c600-module/tests/ct_ev1_04.rs`: the host's routes, the program job from
//! the palette and from a key on both hosts, the view source, the debugger's route (LP4-5) and a recording's route
//! (LP4-9), while a draft is broken, unchecked or checked and not active, and on the recording route a step that names
//! an unchecked or an inactive digest; the broken drafts and the module set's own routes are in that file. Every route
//! that runs a user module's version is here: the palette, a key, a view source, the debugger and a recording).
//!
//! CT-EV1-08 (source, part: a program job's run suspended at a host call on either host, through the binding's
//! boundary, while B activates; the service, the refusals and the races are in `c600-module/tests/ct_ev1_08.rs` and
//! `c600-runtime/tests/ct_ev1_08_process.rs`, which with this part are the whole by union).

mod process_support;

use std::sync::{Arc, Mutex};

use c600_check::{Grant, Policy};
use c600_command::{
    CommandLayer, Error as LayerError, Kind, MemoryRecorder, Overlay, PlayStatus, ProgramJob, Receipt, Shell, Status,
    ViewSourceRef, RECORDING_PLAY_ROW, RECORDING_START_ROW, RECORDING_STOP_ROW, VIEW_OPEN_ROW,
};
use c600_digest::{sha256, sha256_hex};
use c600_input::{default_specs, BindingSpec, Context, KeyEvent, KeyMap, Layout, Router, RoutingClass, Slot};
use c600_kernel::json::{self, Value as Json};
use c600_module::code::{MODULE_NOT_CHECKED, STALE_ACTIVATION_BASE};
use c600_module::{resolve, Candidate, Dependency, Entry, Files, Identity, Manifest, Modules};
use c600_run::boundary::Store;
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::provenance::{hex, Author, Pin, Profile};
use c600_run::value::Value;
use c600_tools::modules::{bind_job, bind_view_source, run_bound_in_process, ActiveVersions, ModuleRun};
use c600_tools::process::{ProcessHost, IN_PROCESS, PROCESS};
use c600_tools::views::{model_revision, ModelViews};
use c600_view as view;

use process_support::{process_host, session, HOSTS};

const EDITION: &str = "6cs-draft-1";
const PROBE: i64 = 5;
/// The overlay rows `m`'s entry serves, and the key bound to the query row.
const GO: &str = "tool.m.go";
const PANEL: &str = "tool.m.panel";
const KEY: &str = "Ctrl+Shift+KeyT";

// ----- versions of the user module `m`

fn module(name: &str, imports: &[(&str, &str)], body: &str) -> String {
    let mut text = format!("edition \"{EDITION}\";\nmodule {name} requires [];\n\n");
    for (alias, source) in imports {
        text.push_str(&format!("import {alias} \"{}\";\n", hex(&sha256(source.as_bytes()))));
    }
    if !imports.is_empty() {
        text.push('\n');
    }
    text + body
}

const BODY: &str = "export fn go() -> Int effects [snapshot] {\n    \
                    let n = read_snapshot<Int>(\"probe\");\n    \
                    return exact_add(n, helper.value());\n}\n";

/// Version `n` of `m`: the same root text over a dependency whose `value` returns `n`; `go` returns the probe row's
/// value plus `n`.
struct Version {
    n: i64,
    root: String,
    manifest: Vec<u8>,
    lock: Vec<u8>,
    files: Files,
}

fn version_of(n: i64, body: &str) -> Version {
    let dep = module("helper", &[], &format!("export fn value() -> Int effects [] {{\n    return {n};\n}}\n"));
    let root = module("m", &[("helper", &dep)], body);
    let manifest = Manifest {
        module: "m".into(),
        edition: EDITION.into(),
        source: sha256(root.as_bytes()),
        dependencies: vec![Dependency { alias: "helper".into(), source: sha256(dep.as_bytes()) }],
        capabilities: vec![],
        boundary: None,
        principle: None,
        supports: vec![Identity::full()],
    }
    .bytes();
    let files = Files::new().with("src/root.6cs", root.as_bytes()).with("src/dep.6cs", dep.as_bytes());
    let lock = resolve(&manifest, &files).unwrap_or_else(|r| panic!("{r:?}")).bytes();
    Version { n, root, manifest, lock, files }
}

fn version(n: i64) -> Version {
    version_of(n, BODY)
}

impl Version {
    fn closure(&self) -> [u8; 32] {
        c600_module::verify(&self.manifest, &self.lock, &self.files, &Identity::full()).unwrap().digest
    }

    fn source(&self) -> String {
        sha256_hex(self.root.as_bytes())
    }

    /// `go`'s output: its canonical encoding.
    fn output(&self) -> Vec<u8> {
        format!("{{\"int\":\"{}\"}}\n", PROBE + self.n).into_bytes()
    }
}

fn modules() -> Modules {
    let contract = Pin::of("fixture-1", b"p3 fixture capability contract 1\n");
    Modules::new(Identity::full(), contract, Pin::of("m1-draft", b"c600-check fixture\n"), Grant::none())
}

fn check(modules: &Modules, v: &Version) -> Result<Candidate, c600_module::Refusal> {
    let entry = Entry {
        module: "m".into(),
        function: "go".into(),
        author: Author::Own,
        profile: Profile::ToolAuthoring,
        grant: Grant::none(),
        policy: Policy::interim(),
    };
    modules.check(&v.manifest, &v.lock, &v.files, &[entry])
}

fn activate(modules: &Modules, c: &Candidate) {
    let active = modules.active("m");
    modules.activate_module("m", active, c.digest, c.reference).unwrap_or_else(|r| panic!("{r:?}"));
}

// ----- the engine side and the session

/// An engine that serves the probe row and runs a hook once inside it: while the run is suspended at that call.
#[derive(Default)]
struct Probe {
    hook: Option<Box<dyn FnOnce()>>,
}

impl Host for Probe {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Snapshot { query: "probe", .. } => Ok(RowTariff { kernel_cost: 1, result_bound: 64 }),
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Snapshot { query: "probe", .. } => {
                if let Some(hook) = self.hook.take() {
                    hook();
                }
                Ok(Value::int(PROBE))
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}

/// A session with `m`'s query row, the row of a module never activated, and a router whose default map binds [`KEY`]
/// to the query row.
fn layer() -> (CommandLayer, Shell, Router) {
    let (mut layer, shell) = session("ev1");
    let overlay = |module: &str| Overlay { module: module.into(), entry: "go".into() };
    layer.register_overlay(GO, Kind::Query, &[], overlay("m")).unwrap();
    layer.register_overlay("tool.n.go", Kind::Query, &[], overlay("n")).unwrap();
    let mut specs = default_specs();
    specs.push(BindingSpec::new(&[KEY], RoutingClass::Global, GO, vec![]));
    let keymap = KeyMap::apply(&specs, &[Layout::US, Layout::FRENCH], "us-qwerty").unwrap();
    let text = |s: String| Json::String(s);
    let slots = (0..4).map(|n| Slot { cap: text(format!("c{n}")), frame: text(format!("f{n}")) }).collect();
    (layer, shell, Router::new(keymap, Context::new(slots)))
}

/// The solver invokes `row` from the palette (`key` false) or `m`'s query row by its key; the host takes the job.
fn invoke(layer: &mut CommandLayer, shell: &mut Shell, router: &mut Router, row: &str, key: bool) -> ProgramJob {
    let receipt: Receipt = if key {
        let routed = router.key(shell, layer, KeyEvent::down(KEY));
        router.key(shell, layer, KeyEvent::up(KEY));
        routed.receipt().cloned().expect("the key reached the layer")
    } else {
        shell.invoke(layer, row, json::object([])).expect("delivered")
    };
    let Status::Running(id) = receipt.status else { panic!("{row}: {:?}", receipt.status) };
    let job = layer.take_program_job().expect("a program job");
    assert_eq!((job.job, job.row.as_str()), (id, row));
    job
}

/// Binds the job, runs it on `host` (in process or on the process host) and reports it; the run's closure and output.
fn serve(
    modules: &Modules,
    layer: &mut CommandLayer,
    job: ProgramJob,
    host: &str,
    process: &mut ProcessHost,
    engine: &mut Probe,
) -> Result<([u8; 32], Vec<u8>), LayerError> {
    let binding = match bind_job(modules, &job) {
        Ok(binding) => binding,
        Err(error) => {
            let receipt = layer.finish_program_job(job.job, Err(error.clone())).unwrap();
            assert!(matches!(&receipt.status, Status::Failed(e) if e.code == error.code), "{:?}", receipt.status);
            return Err(error);
        }
    };
    let run = ModuleRun::of_job(&job, vec![]);
    let mut store = Store::default();
    let ran = match host {
        IN_PROCESS => run_bound_in_process(&binding, &run, engine, &mut store),
        _ => process.run_bound(&binding, &run, engine, &mut store),
    }
    .unwrap_or_else(|stopped| panic!("{:?}", stopped.cause));
    assert_eq!(ran.closure, binding.digest());
    let output = ran.outcome.run.output.clone().expect("a completed run");
    let result = Json::String(String::from_utf8(output.clone()).unwrap());
    let receipt = layer.finish_program_job(job.job, Ok(result)).unwrap();
    assert!(matches!(receipt.status, Status::Completed(_)), "{:?}", receipt.status);
    Ok((ran.closure, output))
}

/// EV1.7, EV1.8, EV1.11 on the host: while a draft is broken, while B is unchecked and while B is checked and not
/// active, `m`'s query row invoked from the palette and from its key runs A on both hosts, its run names A, and its
/// job completes with A's result. A module that was never activated is refused when the host takes its job, and the
/// job fails with `EV1:input.module_not_checked`. After B activates, both routes run B on both hosts.
#[test]
fn ct_ev1_04_the_palette_and_a_key_run_only_the_active_version() {
    let (a, b) = (version(10), version(20));
    let modules = modules();
    activate(&modules, &check(&modules, &a).unwrap());
    let (mut layer, mut shell, mut router) = layer();
    let mut process = process_host();
    let mut engine = Probe::default();
    let mut all = |layer: &mut CommandLayer, shell: &mut Shell, router: &mut Router, want: &Version, why: &str| {
        for host in HOSTS {
            for key in [false, true] {
                let job = invoke(layer, shell, router, GO, key);
                let ran = serve(&modules, layer, job, host, &mut process, &mut engine).unwrap();
                assert_eq!(ran, (want.closure(), want.output()), "{why}: {host}, key {key}");
            }
        }
    };

    let broken = version_of(20, "export fn go() -> Int effects [] {\n    return true;\n}\n");
    assert_eq!(check(&modules, &broken).unwrap_err().code, "LG2:type.mismatch");
    all(&mut layer, &mut shell, &mut router, &a, "a broken draft");
    all(&mut layer, &mut shell, &mut router, &a, "B unchecked");
    let cb = check(&modules, &b).unwrap();
    all(&mut layer, &mut shell, &mut router, &a, "B checked, not active");

    let job = invoke(&mut layer, &mut shell, &mut router, "tool.n.go", false);
    let refused = serve(&modules, &mut layer, job, PROCESS, &mut process_host(), &mut Probe::default()).unwrap_err();
    assert_eq!(refused.code, MODULE_NOT_CHECKED, "a module never activated");

    activate(&modules, &cb);
    all(&mut layer, &mut shell, &mut router, &b, "B active");
}

/// The view model's fixture profile, candidate and a further view's workload, as `tests/view_host.rs` declares them.
fn view_model(layer: &CommandLayer) -> view::ViewModel {
    const BIG: u64 = 1 << 40;
    let value = |field: view::Field| match field {
        view::Field::Overdraw => 1,
        view::Field::ViewVertexEvals | view::Field::AdapterCpuBytes => 4,
        _ => BIG,
    };
    let entries: Vec<(&str, u64, &str)> =
        view::Field::ALL.iter().map(|field| (field.name(), value(*field), field.unit())).collect();
    let profile = view::LimitProfile::from_entries("fixture-2", &entries).unwrap();
    let revision = model_revision(layer.revision()).unwrap();
    let part = |kind: view::PartKind, digest: [u8; 32]| view::BundlePart { kind, revision, digest };
    let fixture = |name: &str| sha256(format!("{name} of {}", layer.revision().to_json().canonical()).as_bytes());
    let parts = vec![
        part(view::PartKind::Metadata, fixture("metadata")),
        part(view::PartKind::Labels, sha256(layer.revision().digest.as_bytes())),
        part(view::PartKind::Styles, fixture("styles")),
    ];
    let candidate = view::BundleCandidate { revision, mode: view::Mode::JumbleOff, parts };
    let none = view::Workload::default();
    view::ViewModel::new(view::Profile::Solving, profile, &candidate, none, none).unwrap()
}

/// EV1.7 (a view): `m`'s view row opens from a source that names A's source digest. A source naming B's digest is
/// refused while B is unchecked (`EV1:input.module_not_checked`) and while B is checked and not active
/// (`EV1:stale.activation_base`); the receipt carries the refusal, takes no job and opens nothing. Once B activates,
/// B's digest opens and A's is stale.
#[test]
fn ct_ev1_04_a_view_source_opens_only_on_the_active_version() {
    let (a, b) = (version(10), version(20));
    let modules = Arc::new(modules());
    activate(&modules, &check(&modules, &a).unwrap());
    let (mut layer, mut shell, _) = layer();
    let overlay = Overlay { module: "m".into(), entry: "go".into() };
    layer.register_overlay(PANEL, Kind::View, &[], overlay.clone()).unwrap();
    let model = Arc::new(Mutex::new(view_model(&layer)));
    let declared = Arc::clone(&modules);
    let views = ModelViews::new(model, move |source: &ViewSourceRef| {
        bind_view_source(&declared, &overlay, source)?;
        let workload =
            view::Workload { vertex_evals: 1, stage_ops: 0, fragment_ops: 0, cpu_bytes: 1, gpu_bytes: 1, pixels: 0 };
        let output = view::OutputAllowance::new(1 << 40, 16);
        Ok(view::Declared { depends: view::Depends::STATE, workload, output, class: None })
    });
    layer.set_views(Box::new(views.clone()));
    let mut open = |name: &str, digest: &str| {
        let source = format!(r#"{{"overlay": {{"row": "{PANEL}", "module": "{digest}", "arguments": {{}}}}}}"#);
        let args = json::parse(format!(r#"{{"view": "{name}", "source": {source}}}"#).as_bytes()).unwrap();
        let receipt = shell.invoke(&mut layer, VIEW_OPEN_ROW, args).expect("delivered");
        assert_eq!(receipt.job, None, "a view open takes no job: {:?}", receipt.status);
        match receipt.status {
            Status::Accepted => None,
            Status::Refused(error) => Some(error.code),
            other => panic!("{other:?}"),
        }
    };
    let opened = |views: &ModelViews| -> Vec<String> {
        views.model().lock().unwrap().views().iter().map(|id| id.as_str().to_owned()).collect()
    };

    assert_eq!(open("a1", &a.source()), None);
    assert_eq!(open("b1", &b.source()), Some(MODULE_NOT_CHECKED), "B unchecked");
    let cb = check(&modules, &b).unwrap();
    assert_eq!(open("b2", &b.source()), Some(STALE_ACTIVATION_BASE), "B checked, not active");
    assert_eq!(opened(&views), ["global", "local", "a1"]);
    activate(&modules, &cb);
    assert_eq!(open("b3", &b.source()), None);
    assert_eq!(open("a2", &a.source()), Some(STALE_ACTIVATION_BASE), "A after B activated");
    assert_eq!(opened(&views), ["global", "local", "a1", "b3"]);
}

/// The debugger's engine: it serves the probe row, and a stepped run's thread may own it.
struct Served;

impl Host for Served {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        Probe::default().tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        Probe::default().perform(request)
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}

/// Steps `m.go` of `binding` to its end on RT1 in this process, or opens it on the runtime's own process; the last
/// result, the session's target and the shown text of the run's value, if it has one.
fn debug(
    binding: c600_module::Binding,
    process: bool,
) -> (c600_debug::session::StepResult, c600_debug::session::Target, Option<String>) {
    use c600_debug::session::{Debugger, Executor, State};
    let input = c600_debug::Pinned {
        identity: c600_debug::Identity {
            puzzle: "600-cell-Full".into(),
            model: "model".into(),
            menu: None,
            menu_order: None,
            contract: None,
        },
        level: "labels".into(),
        digest: "0".repeat(64),
    };
    let executor = if process { Executor::Unmapped("c600-runtime process".into()) } else { Executor::Evaluator };
    let target = c600_tools::debug::target(&binding, input, executor);
    let wide = c600_run::meter::Amounts { steps: 1 << 30, kernel_cost: 1 << 30, memory: 1 << 30, output: 1 << 30 };
    let run = ModuleRun { run: "debug-1".into(), base: "e0".into(), args: vec![], inputs: vec![], limits: wide };
    let launch = c600_tools::debug::launch(binding, run, Box::new(Served), Store::default());
    let mut debugger = Debugger::new();
    let mut at = debugger.open("d", target.clone(), launch).unwrap();
    let mut n = 0;
    while let State::Stopped { .. } | State::Waiting { .. } = at.state {
        if let c600_debug::session::Source::Mapped(ranges) = &at.source {
            assert!(ranges.iter().all(|r| target.sources.contains_key(&r.digest)), "a stop maps into the bound bytes");
        }
        n += 1;
        at = debugger.debug_step("d", &at.stop_id, &format!("q{n}")).unwrap();
    }
    let shown = at.values.first().map(|value| debugger.inspect_value("d", &value.reference).unwrap().shown.text);
    (at, target, shown)
}

/// EV1.7 on the debugger's route (LP4-5): the debugger names the version it steps by digest and binds through
/// `Modules::bind_pinned`. B's digest is refused while B is unchecked (`EV1:input.module_not_checked`) and while B is
/// checked and not active (`EV1:stale.activation_base`); A's digest steps A's bytes to A's result, and every stop maps
/// into them. On the runtime's own process the session is `unavailable(step_mapping_missing)`. Once B activates, B's
/// digest steps B and A's is stale.
#[test]
fn ct_ev1_04_the_debugger_steps_only_the_active_version() {
    use c600_debug::session::{Reason, State, Unavailable};
    use c600_tools::debug::bind_debug;
    let (a, b) = (version(10), version(20));
    let modules = modules();
    activate(&modules, &check(&modules, &a).unwrap());
    let root = |v: &Version| sha256(v.root.as_bytes());

    assert_eq!(bind_debug(&modules, "m", "go", root(&b)).unwrap_err().code, MODULE_NOT_CHECKED, "B unchecked");
    let cb = check(&modules, &b).unwrap();
    for digest in [root(&b), b.closure()] {
        assert_eq!(bind_debug(&modules, "m", "go", digest).unwrap_err().code, STALE_ACTIVATION_BASE, "B not active");
    }
    let stepped = |v: &Version, digest: [u8; 32]| {
        let (end, target, shown) = debug(bind_debug(&modules, "m", "go", digest).unwrap(), false);
        assert_eq!(target.module_digest, hex(&v.closure()));
        let State::Terminal(t) = &end.state else { panic!("{end:?}") };
        assert_eq!(t.reason, Reason::Completed);
        assert_eq!(shown.as_deref().map(str::as_bytes), Some(v.output().trim_ascii_end()));
    };
    stepped(&a, root(&a));
    stepped(&a, a.closure());
    let (none, _, _) = debug(bind_debug(&modules, "m", "go", root(&a)).unwrap(), true);
    assert_eq!(none.state, State::Unavailable(Unavailable::StepMappingMissing));

    activate(&modules, &cb);
    stepped(&b, root(&b));
    assert_eq!(bind_debug(&modules, "m", "go", root(&a)).unwrap_err().code, STALE_ACTIVATION_BASE, "A after B");
}

/// Plays recording `name`, which waits at its job row, takes the step action and the host takes the step's job.
fn play_step(layer: &mut CommandLayer, shell: &mut Shell, name: &str) -> ProgramJob {
    let played = shell.invoke(layer, RECORDING_PLAY_ROW, json::object([("recording", Json::String(name.into()))]));
    assert!(matches!(played.unwrap().status, Status::Running(_)), "{name}: the play");
    layer.run_jobs();
    assert_eq!(layer.playback(name).map(|s| s.status.clone()), Some(PlayStatus::Waiting(0)), "{name}: a job row waits");
    let step = shell.step(layer, name, false).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    assert!(matches!(step.status, Status::Running(_)), "{name}: {:?}", step.status);
    layer.take_program_job().expect("the step's job")
}

/// EV1.7, EV1.11, EV4.10, EV4.14 on the host's recording route (LP4-9): the solver records `m`'s query row while A is
/// active, and the step names A's closure digest (the layer reads it through the host's `ActiveVersions`). Played, the
/// step waits for the solver's step action, a job row, and its job names the recorded digest, so the host binds it
/// pinned: A's digest runs A on both hosts. On a layer without the versions reader, which checks nothing itself, a
/// step that names a digest no check produced is refused when the host takes its job with
/// `EV1:input.module_not_checked`, and one that names B, checked and not active, with `EV1:stale.activation_base`;
/// both jobs fail with that code and the playback stops on the failure. After B activates, A's step is stale at the
/// host too, and with the reader the layer refuses the play before it starts (`EV4:input.recording_identity_mismatch`).
#[test]
fn ct_ev1_04_a_recording_runs_only_the_version_it_names() {
    let (a, b) = (version(10), version(20));
    let modules = Arc::new(modules());
    activate(&modules, &check(&modules, &a).unwrap());
    let (mut layer, mut shell, mut router) = layer();
    let recorder = MemoryRecorder::new();
    layer.set_recorder(Box::new(recorder.clone())).unwrap();
    layer.set_module_versions(Box::new(ActiveVersions(Arc::clone(&modules))));
    let mut process = process_host();
    let mut engine = Probe::default();

    let named = |name: &str| json::object([("name", Json::String(name.into()))]);
    let stop = |name: &str| json::object([("recording", Json::String(name.into()))]);
    assert!(matches!(shell.invoke(&mut layer, RECORDING_START_ROW, named("r")).unwrap().status, Status::Completed(_)));
    let job = invoke(&mut layer, &mut shell, &mut router, GO, false);
    assert_eq!(job.module, None, "the solver's own press binds the active version");
    serve(&modules, &mut layer, job, IN_PROCESS, &mut process, &mut engine).unwrap();
    assert!(matches!(shell.invoke(&mut layer, RECORDING_STOP_ROW, stop("r")).unwrap().status, Status::Completed(_)));
    let recorded = layer.recording("r").unwrap().clone();
    assert_eq!(recorded.steps[0].module, Some(hex(&a.closure())), "the step names A");

    for host in HOSTS {
        let job = play_step(&mut layer, &mut shell, "r");
        assert_eq!(job.module, Some(hex(&a.closure())));
        let ran = serve(&modules, &mut layer, job, host, &mut process, &mut engine).unwrap();
        assert_eq!(ran, (a.closure(), a.output()), "{host}");
        assert_eq!(layer.playback("r").map(|s| s.status.clone()), Some(PlayStatus::Finished));
        recorder.put(c600_command::Recorded { recordings: recorder.held().recordings, playbacks: Default::default() });
        layer.set_recorder(Box::new(recorder.clone())).unwrap();
    }

    // The host's binding is the check on a layer that reads no versions.
    let cb = check(&modules, &b).unwrap();
    let unchecked = hex(&sha256(b"a digest no check produced"));
    let (mut bare, mut bare_shell, _) = crate::layer();
    let stored = MemoryRecorder::new();
    let mut recordings = std::collections::BTreeMap::new();
    for (name, digest) in [("unchecked", unchecked), ("b", hex(&b.closure()))] {
        let mut copy = recorded.clone();
        copy.name = name.into();
        copy.steps[0].module = Some(digest);
        recordings.insert(name.to_owned(), copy);
    }
    stored.put(c600_command::Recorded { recordings, playbacks: Default::default() });
    bare.set_recorder(Box::new(stored)).unwrap();
    for (name, code) in [("unchecked", MODULE_NOT_CHECKED), ("b", STALE_ACTIVATION_BASE)] {
        let job = play_step(&mut bare, &mut bare_shell, name);
        let refused = serve(&modules, &mut bare, job, PROCESS, &mut process, &mut Probe::default()).unwrap_err();
        assert_eq!(refused.code, code, "{name}");
        let stopped = Some(PlayStatus::Stopped(c600_command::Stop::Outcome(c600_command::ChildOutcome::Failed)));
        assert_eq!(bare.playback(name).map(|s| s.status.clone()), stopped, "{name}");
    }

    activate(&modules, &cb);
    let refused = shell.invoke(&mut layer, RECORDING_PLAY_ROW, stop("r")).unwrap();
    assert_eq!(refused.status.code(), Some(c600_command::code::RECORDING_IDENTITY_MISMATCH), "A after B");
    let mut copy = recorded.clone();
    copy.name = "a".into();
    let mut recordings = std::collections::BTreeMap::new();
    recordings.insert("a".to_owned(), copy);
    let stale = MemoryRecorder::new();
    stale.put(c600_command::Recorded { recordings, playbacks: Default::default() });
    bare.set_recorder(Box::new(stale)).unwrap();
    let job = play_step(&mut bare, &mut bare_shell, "a");
    let refused = serve(&modules, &mut bare, job, IN_PROCESS, &mut process, &mut Probe::default()).unwrap_err();
    assert_eq!(refused.code, STALE_ACTIVATION_BASE, "A's step after B activates");
}

/// EV1.11 on the host: the job of `m`'s query row is bound to A when the host takes it; its run is suspended at its
/// probe read, on either host, while B activates with expected A; it resumes on A through the binding's boundary and
/// its job completes with A's result. The next job binds B.
#[test]
fn ct_ev1_08_a_job_suspended_on_either_host_finishes_on_a_after_b_activates() {
    let (a, b) = (version(10), version(20));
    let mut process = process_host();
    for host in HOSTS {
        let modules = Arc::new(modules());
        activate(&modules, &check(&modules, &a).unwrap());
        let cb = check(&modules, &b).unwrap();
        let (mut layer, mut shell, mut router) = layer();
        let job = invoke(&mut layer, &mut shell, &mut router, GO, false);
        let switched = Arc::new(Mutex::new(None));
        let (m, out, expected) = (Arc::clone(&modules), Arc::clone(&switched), Some(a.closure()));
        let mut engine = Probe {
            hook: Some(Box::new(move || {
                *out.lock().unwrap() = Some(m.activate_module("m", expected, cb.digest, cb.reference));
            })),
        };
        let ran = serve(&modules, &mut layer, job, host, &mut process, &mut engine).unwrap();
        let activation = switched.lock().unwrap().take().expect("the run was suspended at its probe read").unwrap();
        assert_eq!((activation.previous, activation.new), (Some(a.closure()), b.closure()), "{host}");
        assert_eq!(ran, (a.closure(), a.output()), "{host}: the suspended run finished on A");
        let job = invoke(&mut layer, &mut shell, &mut router, GO, false);
        let next = serve(&modules, &mut layer, job, host, &mut process, &mut Probe::default()).unwrap();
        assert_eq!(next, (b.closure(), b.output()), "{host}: the next job binds B");
    }
}
