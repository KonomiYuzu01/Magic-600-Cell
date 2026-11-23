//! Runs of the shipped modules on the real geometry, for the records' fixtures: a pure entry on `c600-run` with no
//! host; a state-reading entry through `Boundary` and the tools' host bridge onto a 600-cell-Full command layer, in
//! the solving profile, author `built_in` (as `c600-tools/tests/exact_completion.rs` runs library code); a tool's
//! entry inside the command the solver invokes (`c600_tools::process::invoke_tool`); and the jumbling tool's entries
//! onto the jumbling engine's whole-word check (as `c600-jumble-tool/tests/lb3.rs` runs them).

#![allow(dead_code)]

use std::sync::Arc;

use c600_command::{CommandLayer, PreviewState, Secret, Shell, Status, Table};
use c600_digest::sha256;
use c600_jumble::state::State;
use c600_kernel::json::Value as KernelJson;
use c600_kernel::{Derivation, ProtectedSet, Snapshot};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Host, NoHost, Options, Request, RowTariff, Sink};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::trace::Relation;
use c600_run::value::Value;
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::SetupTable;
use c600_tools::process::invoke_tool;
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins, Tool};

use super::{engine, kernel, menu, repository};

/// Run limits wide enough for every instance.
pub const WIDE: Amounts = Amounts { steps: 1 << 30, kernel_cost: 1 << 40, memory: 1 << 34, output: 1 << 28 };

const SECRET: [u8; 32] = [5; 32];

/// A run's value, with the run completed.
fn completed(run: c600_run::interp::Run, what: &str) -> Value {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{what}: {:?}", run.summary.stop);
    run.value.unwrap_or_else(|| panic!("{what}: a value"))
}

/// `module.entry` of `sources` on `args` with no host, so no state exists for the run to read.
pub fn pure(sources: &[&str], module: &str, entry: &str, args: Vec<Value>, relation: Option<&Relation>) -> Value {
    let program = c600_library::lower_sources(sources).expect("the sources lower");
    let loaded = load(&program).expect("the program loads");
    let options = Options { relation, ..Options::new(WIDE) };
    let run = loaded.run(module, entry, args, &mut NoHost, &options).expect("the run starts");
    completed(run, &format!("{module}.{entry}"))
}

/// The model's trace relation (`full-disjoint-support/1`), which `word_keys` keys words under.
pub fn relation() -> Relation {
    c600_tools::supports::full_relation(&kernel()).expect("the model's relation")
}

/// A command layer whose session starts at `start`, with no protected orbit.
pub fn layer_at(start: Snapshot, name: &str) -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed(SECRET), Default::default(), name, start, none)
}

/// The state `letters` derive from solved with no protected orbit.
pub fn after(letters: &[i64]) -> Snapshot {
    let kernel = kernel();
    let recipe = kernel.word(letters).expect("a well-formed word");
    match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

/// The certified guarded setup tables of `assets/`.
pub fn setups() -> Arc<SetupTable> {
    Arc::new(SetupTable::load(&repository().join("assets")).expect("the setup tables load"))
}

/// The context of one run of `entry` of the module `source` in the solving profile, built in.
fn context(source: &str, entry: &str, layer: &CommandLayer, reads_state: bool, inputs: usize) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(source.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let inputs = vec![named_word_block(); inputs];
    Context { run: "fixture".into(), base: base(layer.revision()), producer, reads_state, inputs, pins: pins() }
}

/// A host that serves the bridge's reads and keeps the views a run publishes: the representations tool's engine side
/// (as `c600-repr/tests/served_rows.rs` serves it).
struct Viewing<'b, 'l> {
    bridge: &'b mut Bridge<'l>,
    views: Vec<Vec<u8>>,
    records: Vec<(String, Vec<u8>)>,
}

impl Host for Viewing<'_, '_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.bridge.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.bridge.perform(request)
    }

    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        match sink {
            Sink::View => self.views.push(bytes.to_vec()),
            Sink::Workspace { record, .. } => self.records.push((record.to_string(), bytes.to_vec())),
            _ => return Err(c600_command::code::NOT_GRANTED),
        }
        Ok(())
    }
}

/// What a run on a layer gave: its value, the views it published and the workspace records it wrote.
pub struct OnLayer {
    pub value: Value,
    pub views: Vec<Vec<u8>>,
    pub records: Vec<(String, Vec<u8>)>,
}

/// `module.entry` of the module `source` on `args` through a boundary and the tools' host bridge onto `layer`, with the
/// setup tables when `table` is given. The run must complete and change nothing on the layer.
pub fn on_layer(
    source: &str,
    module: &str,
    entry: &str,
    args: Vec<Value>,
    layer: &mut CommandLayer,
    table: Option<Arc<SetupTable>>,
) -> OnLayer {
    let program = lower_source(source).expect("the module lowers");
    let report = c600_check::check(&program).expect("well-formed IR");
    let summary = report.summary(module, entry).expect("a summary");
    let reads_state = c600_check::separation(summary, &c600_check::Policy::interim()).reads_state;
    let loaded = load(&program).expect("the module loads");
    let before = (layer.revision().clone(), layer.store().log.len());
    let context = context(source, entry, layer, reads_state, args.len());
    let (views, mut records, run) = {
        let mut bridge = Bridge::new(layer, "fixture").with_setups(table);
        let mut host = Viewing { bridge: &mut bridge, views: Vec::new(), records: Vec::new() };
        let mut store = Store::default();
        let mut boundary = Boundary::new(&mut host, &mut store, context);
        let run = loaded.run(module, entry, args, &mut boundary, &Options::new(WIDE));
        drop(boundary);
        let mut records: Vec<(String, Vec<u8>)> =
            store.records.into_iter().map(|(place, stored)| (format!("{place:?}"), stored.bytes)).collect();
        records.append(&mut host.records);
        let views = std::mem::take(&mut host.views);
        assert!(bridge.sent.is_empty(), "{module}.{entry}: a read sends no invocation");
        (views, records, run)
    };
    records.sort();
    let value = completed(run.expect("the run starts"), &format!("{module}.{entry}"));
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before, "{module}.{entry}: a run changes nothing");
    OnLayer { value, views, records }
}

/// The 600-cell-Full tool's preview of `word` at solved: the tool runs inside the command the solver invokes, its
/// proposal becomes the runtime's `operation.preview`, and the layer builds the preview. Returns the layer, which holds
/// the preview, and the program's output.
pub fn full_tool_preview(word: &[i32]) -> (CommandLayer, Vec<u8>) {
    let mut layer = layer_at(kernel().solved(), "fixture");
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let tool = Tool::full_600_cell().expect("the shipped tool loads");
    let command = invoke_tool(&mut shell, &mut layer, &tool, word).expect("the tool's row");
    let run = command.run();
    let context = tool.context(&run, &base(&command.job.revision));
    let ran = {
        let mut bridge = Bridge::new(&mut layer, &run);
        tool.run_in(word, context, &mut bridge, &mut Store::default()).expect("the run starts")
    };
    assert_eq!(ran.run.summary.outcome, Outcome::Completed);
    assert_eq!(layer.run_jobs(), 1, "the preview is built while the host holds the run's job");
    let receipt = command.acknowledge(&mut layer).expect("the layer holds the tool's job");
    assert!(matches!(receipt.status, Status::Completed(_)), "{receipt:?}");
    let preview = layer.preview().expect("a preview");
    assert_eq!(preview.state, PreviewState::Ready, "the preview is built");
    (layer, ran.run.output.expect("the acknowledgement"))
}

/// The engine side of the jumbling tool's run: each proposal is checked as a whole word by the jumbling engine at
/// `state` (`c600_jumble::observe::word_check`), which changes nothing.
pub struct Checker {
    pub state: State,
    pub proposed: Vec<Vec<i32>>,
    pub checks: Vec<KernelJson>,
}

impl Host for Checker {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 1, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let Request::Propose { word, .. } = request else { return Err("the jumbling tool reads no state") };
        let word = word.to_vec();
        let letters = c600_jumble_tool::word::letters(engine(), &menu(), &word).map_err(|_| "letters of the menu")?;
        let cap = word.first().map(|&cap| cap as i64);
        let check = c600_jumble::observe::word_check(engine(), &self.state, &letters, None, cap, None)
            .map_err(|_| "a whole-word check")?;
        self.proposed.push(word);
        self.checks.push(check);
        Ok(Value::receipt(vec![("id", Value::text("preview-1"))]))
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err("the jumbling tool writes nothing")
    }
}

/// Runs the jumbling tool's `entry` on `word` at solved under the pinned menu; returns the engine side.
pub fn jumble_check(entry: &str, word: &[i32]) -> Checker {
    let mut host = Checker { state: State::new(Some(menu())), proposed: Vec::new(), checks: Vec::new() };
    let tool = c600_jumble_tool::tool::tool(entry).expect("the entry loads");
    let ran = tool.run(word, "fixture", "base", &mut host, &mut Store::default()).expect("the run starts");
    assert_eq!(ran.shell.len(), 1, "{entry}: one proposal");
    host
}
