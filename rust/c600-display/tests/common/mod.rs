//! Shared fixtures: the shipped geometry and kernel, a declared host context, the RM.30 sources and builders for
//! 600-cell-Full publications at fixture revisions. Every value here is a source or synthetic fixture.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_digest::sha256_hex;
use c600_display::array::Array;
use c600_display::error::{Error, Form};
use c600_display::exact::{self, Q5, V4};
use c600_display::host::{Host, MockBackend};
use c600_display::pick::{Partial, Projection};
use c600_display::plan::*;
use c600_display::produce::{self, GeometryInput};
use c600_display::receipt::{JobOutcome, Receipt, Status};
use c600_display::validate::{Context, LimitProfile};
use c600_kernel::{IdentityBlock, Kernel};

/// RM.30's source and its edited version.
pub const SOURCE: &str = "a\u{1F600}b";
pub const EDITED: &str = "a\u{1F600}c";
pub const SOURCE_DIGEST: &str = "6fba5b2ea783ded096fc2444d540ffbdf49168df30993b155b7efb683313f110";
pub const EDITED_DIGEST: &str = "f2f6acee05793ab28e37f3562f442906e607a9347977cad09a2d7d682241c97a";

pub fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

pub fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| Kernel::load(&assets()).expect("the shipped model loads"))
}

pub fn geometry() -> &'static GeometryInput {
    static GEOMETRY: OnceLock<GeometryInput> = OnceLock::new();
    GEOMETRY.get_or_init(|| GeometryInput::load(&assets()).expect("the shipped geometry loads"))
}

pub fn identity() -> IdentityBlock {
    geometry().identity.clone()
}

/// Declared fixture ceilings, all at `value`.
pub fn limits(value: u64) -> LimitProfile {
    LimitProfile::uniform("fixture/1", value)
}

/// The fixture context with generous declared ceilings.
pub fn context() -> Context {
    produce::fixture_context(geometry(), limits(1 << 40))
}

/// A revision of the fixture session at state revision `state` with that labels digest.
pub fn revision_with(state: u64, digest: &str) -> RevisionRef {
    RevisionRef {
        session: "fixture".to_owned(),
        epoch: 1,
        identity: identity(),
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

pub fn pinned(text: &str) -> (String, Arc<Vec<u8>>) {
    (sha256_hex(text.as_bytes()), Arc::new(text.as_bytes().to_vec()))
}

/// The range [5, 6) of `a😀b`, which selects `b`.
pub fn link() -> SourceLink {
    SourceLink::Range { digest: SOURCE_DIGEST.to_owned(), start: 5, end: 6 }
}

pub fn instance(id: u64, plan: u64, object: ObjectId, primitive: u32) -> DisplayInstance {
    DisplayInstance { instance: InstanceId(id), plan: PlanId(plan), object, primitive, piece: None, interaction: true }
}

/// A session plan at `base` and its source map; every instance links to `link()`.
pub fn plan(
    id: u64,
    base: &RevisionRef,
    primitives: Vec<Primitive>,
    instances: Vec<DisplayInstance>,
) -> (DisplayPlan, SourceMap) {
    let entries =
        instances.iter().map(|i| (i.instance, MapEntry { object: i.object.clone(), source: link() })).collect();
    let plan = DisplayPlan {
        plan: PlanId(id),
        base: base.clone(),
        subject: StateRef::Snapshot(base.clone()),
        derivation: None,
        reachability: None,
        dimension: 4,
        geometry: GeometryBinding { identity: identity(), digests: geometry().digests() },
        primitives,
        charts: Vec::new(),
        instances,
        source_map: MapId(id),
        precision: "binary32".to_owned(),
        dependencies: BTreeMap::from([("state".to_owned(), base.state.to_string())]),
    };
    let map = SourceMap {
        map: MapId(id),
        plan: PlanId(id),
        base: base.clone(),
        sources: vec![SOURCE_DIGEST.to_owned()],
        entries,
    };
    (plan, map)
}

pub fn publication(view: u64, base: &RevisionRef, parts: Vec<(DisplayPlan, SourceMap)>) -> Publication {
    let (plans, maps) = parts.into_iter().unzip();
    Publication {
        target: ViewId(view),
        base: base.clone(),
        nodes: Vec::new(),
        plans,
        maps,
        sources: BTreeMap::from([pinned(SOURCE)]),
    }
}

/// A sheet layout of the instances `ids`, one row each.
pub fn sheet_layout(ids: &[u64]) -> LayoutInput {
    let positions: Vec<f32> = ids.iter().enumerate().flat_map(|(i, _)| [i as f32, 0.0]).collect();
    LayoutInput {
        instances: ids.iter().map(|&id| InstanceId(id)).collect(),
        positions: Array::f32s(vec![ids.len() as u64, 2], &positions),
        edges: None,
        triangles: None,
        kind: LayoutKind::Sheet,
    }
}

/// A publication with one plan: a sheet of the instances `first..first + n`, each showing sticker `id`.
pub fn sheet_from(view: u64, base: &RevisionRef, first: u64, n: u64) -> Publication {
    let ids: Vec<u64> = (first..first + n).collect();
    let instances = ids.iter().map(|&id| instance(id, 1, ObjectId::Sticker(id as u32), 0)).collect();
    publication(view, base, vec![plan(1, base, vec![Primitive::Layout(sheet_layout(&ids))], instances)])
}

pub fn sheet(view: u64, base: &RevisionRef, n: u64) -> Publication {
    sheet_from(view, base, 1, n)
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
    host_with(base, context())
}

pub fn host_with(base: &RevisionRef, ctx: Context) -> Host<MockBackend> {
    let backend = MockBackend::new(ctx.support.clone());
    let mut host = Host::new(backend, ctx, base.clone());
    host.open_view(ViewId(1)).expect("view 1 opens");
    host
}

/// The backend finishes the command's preparation and the host polls it.
pub fn finish(host: &mut Host<MockBackend>, command: u64) {
    let ticket = host.ticket(CommandId(command)).expect("the command was prepared");
    host.backend.finish(ticket);
    host.pump();
}

/// Publishes standalone and installs: the receipt must end `completed(view_published)`.
pub fn install(host: &mut Host<MockBackend>, command: u64, publication: Publication) {
    let receipt = host.publish(CommandId(command), publication);
    assert_eq!(receipt.status, Status::Accepted, "{:?}", receipt.status);
    finish(host, command);
    assert!(matches!(status(host, command), Status::Completed(_)), "{:?}", status(host, command));
}

pub fn status(host: &Host<MockBackend>, command: u64) -> Status {
    host.receipt(CommandId(command)).expect("a receipt").status
}

/// The refused error of a receipt, with its form checked.
pub fn refused(receipt: &Receipt) -> Error {
    match &receipt.status {
        Status::Refused(error) => {
            assert_eq!(error.form, Form::Refusal);
            error.clone()
        }
        other => panic!("expected refused, got {other:?}"),
    }
}

/// The failed error of a receipt, with its form checked.
pub fn failed(receipt: &Receipt) -> Error {
    match &receipt.status {
        Status::Failed(error) => {
            assert_eq!(error.form, Form::Stop);
            error.clone()
        }
        other => panic!("expected failed, got {other:?}"),
    }
}

/// Every command in the host's terminal log appears once, and its receipt still carries that first terminal (RM.22).
pub fn one_terminal_each(host: &Host<MockBackend>) {
    let mut seen = BTreeSet::new();
    for (command, terminal) in host.events() {
        assert!(terminal.is_terminal(), "{terminal:?}");
        assert!(seen.insert(*command), "command {} has a second terminal", command.0);
        assert_eq!(&status(host, command.0), terminal, "the first terminal stands");
    }
}

/// An engine preview of the fixture model at `base` (KN5.9).
pub fn preview(id: u64, base: &RevisionRef) -> PreviewDisplay {
    PreviewDisplay {
        preview: PreviewId(id),
        base: base.clone(),
        literal: sha256_hex(format!("literal {id}").as_bytes()),
        effect: sha256_hex(format!("effect {id}").as_bytes()),
        primitives: Vec::new(),
    }
}

/// A projection the test never expects to be asked: resolving by instance ID needs none.
pub struct Unasked;

impl Projection for Unasked {
    fn slot(&self, _: &StateRef, object: &ObjectId) -> Partial<u32> {
        panic!("a projection was asked for {object:?}")
    }
}

/// The projection of a subject off the lattice: no slot exists, and none is invented (KN0.4, KN1.7).
pub struct OffLattice;

impl Projection for OffLattice {
    fn slot(&self, _: &StateRef, _: &ObjectId) -> Partial<u32> {
        Partial::Undefined("off_lattice".to_owned())
    }
}

/// RM.20's two sides for one payload: standalone admission and a scheduled invocation's final validation. Returns
/// the standalone refusal and the scheduled failure; neither installs anything, and the scheduled job's outcome is
/// `error(Error)` with the same error.
pub fn both_sides(publication: Publication, ctx: Context) -> (Error, Error) {
    let base = publication.base.clone();
    let mut standalone = host_with(&base, ctx.clone());
    let refusal = refused(&standalone.publish(CommandId(1), publication.clone()));
    standalone.pump();
    assert!(standalone.installed(ViewId(1)).is_none());
    let mut scheduled = host_with(&base, ctx);
    let running = scheduled.schedule(CommandId(1), ViewId(1), u64::MAX);
    assert!(matches!(running.status, Status::Running(_)));
    let failure = failed(&scheduled.deliver(CommandId(1), publication).expect("a receipt"));
    scheduled.pump();
    assert!(scheduled.installed(ViewId(1)).is_none());
    assert_eq!(scheduled.job_outcome(CommandId(1)), Some(&JobOutcome::Error(failure.clone())));
    (refusal, failure)
}

/// The vertex (1, 0, 0, 0) of the 600-cell.
pub const E1: V4 = [Q5::ONE, Q5::ZERO, Q5::ZERO, Q5::ZERO];

/// The vertex (0, 0, 0, 1), on the built-in stereographic pole +w.
pub const W: V4 = [Q5::ZERO, Q5::ZERO, Q5::ZERO, Q5::ONE];

/// A cell of the 600-cell: (1, 0, 0, 0) and three of its neighbours, pairwise adjacent (product φ/2).
pub fn cell() -> Vec<V4> {
    let edge = Q5::new(1, 1, 4).expect("φ/2");
    let adjacent = |u: &V4, v: &V4| exact::dot(u, v) == Some(edge);
    let neighbours: Vec<V4> = exact::cell600().into_iter().filter(|v| adjacent(&E1, v)).collect();
    assert_eq!(neighbours.len(), 12, "a vertex of the 600-cell has 12 neighbours");
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

/// The built-in projection chain of F04 with the stereographic stage.
pub fn built_in(id: u64) -> Chart {
    chart(id, &[("rotate", 6), ("stereographic", 0), ("camera3", 9)])
}
