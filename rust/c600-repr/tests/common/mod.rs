//! Shared fixtures: the shipped geometry and kernel, a declared host context over `c600-display`'s mock backend, and
//! builders for the representations these tests publish. Every value here is a source or synthetic fixture.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    draft_args, ActionHint, Caller, CommandLayer, Direction, HeadlessFrames, Invocation, MemoryStore, Receipt, Secret,
    Shell, Table,
};
use c600_digest::sha256_hex;
use c600_display::exact::{self, Q5, V4};
use c600_display::host::{Host, MockBackend};
use c600_display::pick::PickReply;
use c600_display::plan::*;
use c600_display::produce::{self, GeometryInput};
use c600_display::receipt::Status;
use c600_display::validate::{Context, LimitProfile};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;
use c600_repr::layout::{Header, Occurrence, Representation};

/// The one producing source of the fixture maps.
pub const SOURCE: &str = "representations fixture source";

pub fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

pub fn kernel_arc() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL.get_or_init(|| Arc::new(Kernel::load(&assets()).expect("the shipped model loads"))).clone()
}

pub fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL.get_or_init(kernel_arc)
}

/// The fixture secret of the command layer and its shell.
pub const SECRET: [u8; 32] = [7; 32];

/// A command layer over the stored H-08 table at the solved state, with the shell that mints its tokens.
pub struct Session {
    pub layer: CommandLayer,
    pub shell: Shell,
    /// The headless display's frames, which present the pending preview through the built-in chain.
    pub frames: HeadlessFrames,
}

pub fn session() -> Session {
    let kernel = kernel_arc();
    let start = kernel.solved();
    let layer = CommandLayer::new(
        kernel,
        Table::h08(),
        Secret::fixed(SECRET),
        MemoryStore::default(),
        "s1",
        start,
        c600_kernel::ProtectedSet::none(),
    );
    Session { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Session {
    /// Commits the word through the command route: a preview, its presentation and the commit chord.
    pub fn commit(&mut self, letters: &[i64]) -> Receipt {
        let recipe =
            json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).expect("a recipe");
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(&recipe)).expect("prepared");
        let c600_command::Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, job).expect("presented");
        let args = json::object([("preview", Value::Int(job as i64))]);
        let receipt = self.shell.invoke(&mut self.layer, "operation.execute", args).expect("prepared");
        assert!(matches!(receipt.status, c600_command::Status::Committed(_)), "{:?}", receipt.status);
        receipt
    }

    /// An invocation of `row` by a running program, with one placeholder value for each argument the row names. It is
    /// sent on [`program_channel`]. On a row an entry serves it declares the draft ceiling as its budget, and on
    /// another row none, as `Bridge::invoke` does.
    pub fn runtime_call(&self, command: &str, row: &str) -> Invocation {
        let served = self.layer.table().row(row).is_some_and(|found| found.overlay.is_some());
        Invocation {
            command: command.to_owned(),
            row: row.to_owned(),
            args: placeholder_args(self.layer.table(), row),
            token: None,
            epoch: self.layer.revision().epoch,
            budget: served.then_some(c600_command::Budget::DRAFT_CEILING),
        }
    }

    /// What no refused or view invocation may change: the whole revision, the durable log's length and the undo hint.
    pub fn fixed(&self) -> (c600_command::Revision, usize, Option<ActionHint>) {
        (self.layer.revision().clone(), self.layer.store().log.len(), self.layer.undo_hint(Direction::Undo))
    }
}

/// The channel of the running program: the host stamps this caller on what the program sends (ruling 4).
pub fn program_channel() -> Caller {
    Caller::Runtime { run: "run-1".to_owned() }
}

/// Arguments with the names the row requires, each a placeholder: the layer checks the names before it dispatches.
/// An optional argument, whose type H-08 writes with a trailing `?` (`set.save`'s `replace`), is left out.
pub fn placeholder_args(table: &Table, row: &str) -> Value {
    let row = table.row(row).unwrap_or_else(|| panic!("{row} is a stored row"));
    let required = |name: &&str| row.arguments.get(*name).is_some_and(|ty| !ty.ends_with('?'));
    Value::Object(row.plain_arguments().filter(required).map(|name| (name.to_owned(), Value::Int(1))).collect())
}

pub fn geometry() -> &'static GeometryInput {
    static GEOMETRY: OnceLock<GeometryInput> = OnceLock::new();
    GEOMETRY.get_or_init(|| GeometryInput::load(&assets()).expect("the shipped geometry loads"))
}

/// The fixture context with generous declared ceilings.
pub fn context() -> Context {
    produce::fixture_context(geometry(), LimitProfile::uniform("fixture/1", 1 << 40))
}

/// A revision of the fixture session at state revision `state` with that labels digest.
pub fn revision_with(state: u64, digest: &str) -> RevisionRef {
    RevisionRef {
        session: "fixture".to_owned(),
        epoch: 1,
        identity: geometry().identity.clone(),
        mode: Mode::JumbleOff,
        head: state,
        state,
        workspace: 1,
        protection: 0,
        digest: digest.to_owned(),
    }
}

/// A revision whose digest stands for a state the test never draws.
pub fn revision(state: u64) -> RevisionRef {
    revision_with(state, &sha256_hex(format!("fixture state {state}").as_bytes()))
}

pub fn source_digest() -> String {
    sha256_hex(SOURCE.as_bytes())
}

/// A range of the fixture source.
pub fn range(start: u64, end: u64) -> SourceLink {
    SourceLink::Range { digest: source_digest(), start, end }
}

/// The header of plan `plan` at `base`.
pub fn header(plan: u64, base: &RevisionRef) -> Header {
    Header {
        plan: PlanId(plan),
        base: base.clone(),
        geometry: GeometryBinding { identity: geometry().identity.clone(), digests: geometry().digests() },
        sources: vec![source_digest()],
        token: "highlight".to_owned(),
    }
}

/// An occurrence of `object` as instance `id` at the sheet origin.
pub fn occurrence(id: u64, object: ObjectId, chart: Option<u64>) -> Occurrence {
    Occurrence {
        instance: InstanceId(id),
        object,
        chart: chart.map(ChartId),
        position: [0.0, 0.0],
        piece: None,
        source: range(0, 1),
    }
}

/// A publication of the representations into view `view`.
pub fn publication(view: u64, base: &RevisionRef, parts: Vec<Representation>) -> Publication {
    let (plans, maps) = parts.into_iter().map(|r| (r.plan, r.map)).unzip();
    Publication {
        target: ViewId(view),
        base: base.clone(),
        nodes: Vec::new(),
        plans,
        maps,
        sources: BTreeMap::from([(source_digest(), Arc::new(SOURCE.as_bytes().to_vec()))]),
    }
}

/// A chart of fixture stages, version 1, each with its declared number of parameters.
pub fn chart(id: u64, stages: &[(&str, usize)]) -> Chart {
    Chart {
        chart: ChartId(id),
        stages: stages
            .iter()
            .map(|&(name, params)| Stage {
                name: name.to_owned(),
                version: 1,
                definition: sha256_hex(format!("{name}/1").as_bytes()),
                params: vec![0.5; params],
            })
            .collect(),
        dimension: 4,
        precision: "binary32".to_owned(),
        properties: Vec::new(),
    }
}

/// The built-in projection chain of F04 with the stereographic stage.
pub fn built_in(id: u64) -> Chart {
    chart(id, &[("rotate", 6), ("stereographic", 0), ("camera3", 9)])
}

/// The state after `word` from solved, as a kernel derivation.
pub fn derive(word: &[i64]) -> c600_kernel::Derived {
    let recipe = kernel().word(word).expect("a word of legal letters");
    match kernel().derive(&kernel().solved(), &recipe, &c600_kernel::ProtectedSet::none()).expect("derives") {
        c600_kernel::Derivation::Derived(derived) => derived,
        c600_kernel::Derivation::Refused(refused) => panic!("refused: {:?}", refused.refusal),
    }
}

/// A host whose engine image has adopted `base`, over a mock backend with the fixture report, with view 1 open.
pub fn host(base: &RevisionRef) -> Host<MockBackend> {
    let ctx = context();
    let backend = MockBackend::new(ctx.support.clone());
    let mut host = Host::new(backend, ctx, base.clone());
    host.open_view(ViewId(1)).expect("view 1 opens");
    host
}

/// Publishes standalone and installs: the receipt must end `completed(view_published)`.
pub fn install(host: &mut Host<MockBackend>, command: u64, publication: Publication) {
    let receipt = host.publish(CommandId(command), publication);
    assert_eq!(receipt.status, Status::Accepted, "{:?}", receipt.status);
    let ticket = host.ticket(CommandId(command)).expect("the command was prepared");
    host.backend.finish(ticket);
    host.pump();
    let status = host.receipt(CommandId(command)).expect("a receipt").status;
    assert!(matches!(status, Status::Completed(_)), "{status:?}");
}

/// A reply for `instance` under view 1's current binding.
pub fn reply(host: &Host<MockBackend>, instance: u64) -> PickReply {
    PickReply { binding: host.pick_binding(ViewId(1)).expect("a current binding"), instance: InstanceId(instance) }
}

/// The vertex (1, 0, 0, 0) of the 600-cell.
pub const E1: V4 = [Q5::ONE, Q5::ZERO, Q5::ZERO, Q5::ZERO];

/// A cell of the 600-cell: (1, 0, 0, 0) and three of its neighbours, pairwise adjacent (product φ/2).
pub fn cell() -> Vec<V4> {
    let edge = Q5::new(1, 1, 4).expect("φ/2");
    let adjacent = |u: &V4, v: &V4| exact::dot(u, v) == Some(edge);
    let neighbours: Vec<V4> = exact::cell600().into_iter().filter(|v| adjacent(&E1, v)).collect();
    for (i, a) in neighbours.iter().enumerate() {
        for (j, b) in neighbours.iter().enumerate().skip(i + 1) {
            for c in neighbours.iter().skip(j + 1) {
                if adjacent(a, b) && adjacent(a, c) && adjacent(b, c) {
                    return vec![E1, *a, *b, *c];
                }
            }
        }
    }
    panic!("the 600-cell has a cell at (1, 0, 0, 0)")
}

/// The sum of exact vectors; for a cell, a direction on its axis.
pub fn vertex_sum(vertices: &[V4]) -> V4 {
    vertices.iter().fold([Q5::ZERO; 4], |sum, v| exact::add(&sum, v).expect("small values"))
}
