//! The 600-cell-Full tool's views on the headless view model and the display contract (LP3, C5 next, part 2). Each
//! test is named after its conformance ID and its doc comment names the part it covers. The tool runs on `c600-run`
//! through the bridge onto `c600-command`; its views are built by `c600_tools::views` and installed in `c600-view`'s
//! view model and in `c600-display`'s host over the mock backend. Every result is a source or synthetic fixture
//! outcome: no device, renderer or timing is involved, and every ceiling is a declared fixture value.
//! CT-EV3-01 (synthetic, whole: `ct_ev3_01_*` runs source to node to instance to canonical ID and back to the
//! producing digest, with one canonical identity for the duplicated instances).
//! CT-EV3-02 (synthetic, whole: `ct_ev3_02_*` runs a burst of commits, the newest bundle at once, stale output and
//! skipped evaluations).
//! CT-EV3-03 (source, whole: `ct_ev3_03_*` runs the derived display's derivation and base and the imported plan's
//! lineage `imported` with its comparison base; the import is a fixture record because no import row exists).
//! CT-EV3-04 (synthetic, whole: `ct_ev3_04_*` runs `EV3:stale.display_pick` before navigation, `EV3:stale.source_link`
//! and the producing version staying open).
//! CT-BND-19 (source, part: the tool's views hold a word's values only for the word the solver named, in
//! `ct_bnd_19_the_tools_view_*`; not run here: the live twist through the input router, in
//! `c600-input/tests/twist.rs`, which with this file is the whole by union).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code as layer_code, CommandLayer, Error as CommandError, FrameChain, HeadlessFrames, PreviewState, Secret, Shell,
    Status as LayerStatus, Table,
};
use c600_digest::sha256_hex;
use c600_display::codes;
use c600_display::host::{Host, MockBackend};
use c600_display::pick::{Partial, PickMode, PickReply, Presented, Projection, Requested, Resolution};
use c600_display::plan::{
    CommandId, Content as NodeContent, InstanceId, Lineage, NodeKind, ObjectId, Publication, RevisionRef, SourceLink,
    StageKind, StateRef, ViewId,
};
use c600_display::produce::{self, GeometryInput};
use c600_display::receipt::Status as DisplayStatus;
use c600_display::source::Followed;
use c600_display::stage::{check_outputs, compose, Chain, HostStatus, Layer, Output, StageDescription};
use c600_display::validate::{validate, Context, LimitProfile, SubjectKind};
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot};
use c600_run::boundary::{Answer, Store};
use c600_tools::bridge::{base as read_base, Bridge};
use c600_tools::expand::literal_digest;
use c600_tools::process::invoke_tool;
use c600_tools::tool::{manifest, BoundaryClass, Manifest, Ran, Tool, ENTRY, MODULE, ROW, SOURCE};
use c600_tools::views::{
    self, content, declared, derivation, engine_bundle, frame_stamp, imported_view, model_revision, presented_frame,
    preview_display, preview_shown, proposal_view, report_presented, revision_ref, word_text, Links, DERIVED_PLAN,
    ENTRY_NODE, PROPOSAL_NODE, SUPPORT_PLAN,
};
use c600_view as view;

const V1: ViewId = ViewId(1);
const BIG: u64 = 1 << 40;
const COMMIT: &str = "Ctrl+Shift+Enter";

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL.get_or_init(|| Arc::new(Kernel::load(&assets()).expect("the model in assets/ loads"))).clone()
}

fn geometry() -> &'static GeometryInput {
    static GEOMETRY: OnceLock<GeometryInput> = OnceLock::new();
    GEOMETRY.get_or_init(|| GeometryInput::load(&assets()).expect("the shipped geometry loads"))
}

/// The display host's fixture context with generous declared ceilings.
fn context() -> Context {
    produce::fixture_context(geometry(), LimitProfile::uniform("fixture/1", BIG))
}

/// A layer at the solved state with no protected orbit, and its shell, sharing one fixed secret.
fn session() -> (CommandLayer, Shell) {
    let kernel = kernel();
    let secret = Secret::fixed([5; 32]);
    let solved = kernel.solved();
    let none = ProtectedSet::none();
    let layer = CommandLayer::new(kernel, Table::h08(), secret.clone(), Default::default(), "views", solved, none);
    (layer, Shell::new(secret, "shell"))
}

fn tool() -> Tool {
    Tool::full_600_cell().expect("the shipped tool loads")
}

fn links() -> Links {
    Links::of(&tool(), SOURCE).expect("the tool's source links")
}

/// The solver names `word`: the shell invokes the tool's row with it, and the tool runs on the word inside that
/// command, as its job's run, through the bridge (ONE.34; finding B2 of the KN5 boundary attack). The layer builds the
/// preview, and the host then reports the command.
fn name_word(layer: &mut CommandLayer, shell: &mut Shell, word: &[i32]) -> Ran {
    let tool = tool();
    let command = invoke_tool(shell, layer, &tool, word).unwrap_or_else(|error| panic!("the tool's row: {error}"));
    let run = command.run();
    let context = tool.context(&run, &read_base(&command.job.revision));
    let mut bridge = Bridge::new(layer, &run);
    let ran = tool.run_in(word, context, &mut bridge, &mut Store::default()).expect("the run starts");
    layer.run_jobs();
    let reported = command.acknowledge(layer).expect("the layer holds the tool's job");
    assert!(matches!(reported.status, LayerStatus::Completed(_)), "the tool's command: {reported:?}");
    ran
}

/// The display reference of the layer's current revision.
fn base_of(layer: &CommandLayer) -> RevisionRef {
    revision_ref(layer.revision(), &geometry().identity).expect("a Full revision")
}

/// The view of the run's one proposal at the layer's current revision, with the layer's pending preview if it has one.
fn view_of(layer: &CommandLayer, ran: &Ran) -> Publication {
    let [shown] = &ran.shell[..] else { panic!("one proposal") };
    proposal_view(V1, &base_of(layer), geometry(), &links(), 0, shown, layer.preview()).expect("the proposal's view")
}

/// A layer on which the solver named `word`, its shell, and the view of the proposal.
fn proposed(word: &[i32]) -> (CommandLayer, Shell, Publication) {
    let (mut layer, mut shell) = session();
    let ran = name_word(&mut layer, &mut shell, word);
    let publication = view_of(&layer, &ran);
    (layer, shell, publication)
}

/// The head digest, the head event, the state revision and the number of durable transactions.
fn fingerprint(layer: &CommandLayer) -> (String, String, u64, usize) {
    let revision = layer.revision();
    (layer.snapshot().digest().to_owned(), revision.head.clone(), revision.state, layer.store().log.len())
}

/// A display host whose engine image has adopted `base`, over the mock backend, with view 1 open.
fn host_at(base: &RevisionRef) -> Host<MockBackend> {
    let ctx = context();
    let mut host = Host::new(MockBackend::new(ctx.support.clone()), ctx, base.clone());
    host.open_view(V1).expect("view 1 opens");
    host
}

/// Publishes standalone and installs: the receipt must end completed.
fn install(host: &mut Host<MockBackend>, command: u64, publication: Publication) {
    let receipt = host.publish(CommandId(command), publication);
    assert_eq!(receipt.status, DisplayStatus::Accepted, "{:?}", receipt.status);
    let ticket = host.ticket(CommandId(command)).expect("the command was prepared");
    host.backend.finish(ticket);
    host.pump();
    let status = host.receipt(CommandId(command)).expect("a receipt").status;
    assert!(matches!(status, DisplayStatus::Completed(_)), "{status:?}");
}

/// A projection the test never expects to be asked: resolving by instance ID needs none.
struct Unasked;

impl Projection for Unasked {
    fn slot(&self, _: &StateRef, object: &ObjectId) -> Partial<u32> {
        panic!("a projection was asked for {object:?}")
    }
}

/// A reply for `instance` under view 1's current binding.
fn reply(host: &Host<MockBackend>, instance: u64) -> PickReply {
    PickReply { binding: host.pick_binding(V1).expect("a current binding"), instance: InstanceId(instance) }
}

/// An explicit resolution of a current reply that must be total.
fn total(host: &Host<MockBackend>, reply: &PickReply, requested: Requested) -> Resolution {
    match host.resolve_pick(reply, PickMode::Explicit, requested, &Unasked) {
        Ok(Some(Partial::Total(resolution))) => resolution,
        other => panic!("expected a total resolution, got {other:?}"),
    }
}

/// The text a link selects in the tool's source.
fn text(link: &SourceLink) -> &'static str {
    match link {
        SourceLink::Range { digest, start, end } => {
            assert_eq!(digest, &sha256_hex(SOURCE.as_bytes()));
            &SOURCE[*start as usize..*end as usize]
        }
        SourceLink::Missing { .. } => panic!("a missing source"),
    }
}

const CALL: &str = "propose(w, target(\"operation.preview\"))";

/// The orbits a publication's support plan shows, in instance order.
fn orbits(publication: &Publication) -> Vec<ObjectId> {
    publication.plans[0].instances.iter().map(|instance| instance.object.clone()).collect()
}

// ----- the view model's side

/// A complete fixture profile of the view model: every field generous, `overdraw` 1.
fn profile() -> view::LimitProfile {
    let entries: Vec<(&str, u64, &str)> = view::Field::ALL
        .iter()
        .map(|field| (field.name(), if *field == view::Field::Overdraw { 1 } else { BIG }, field.unit()))
        .collect();
    view::LimitProfile::from_entries("fixture-1", &entries).expect("a complete fixture profile")
}

/// The view model's candidate bundle of the layer's current revision. The labels part is the state's label digest;
/// the metadata and styles parts are fixture digests bound to the revision.
fn candidate(layer: &CommandLayer) -> view::BundleCandidate {
    let revision = model_revision(layer.revision()).expect("a label digest");
    let part = |kind: view::PartKind, digest: [u8; 32]| view::BundlePart { kind, revision, digest };
    let fixture = |name: &str| {
        let text = format!("{name} of {}", layer.revision().to_json().canonical());
        c600_digest::sha256(text.as_bytes())
    };
    let labels = c600_digest::sha256(layer.revision().digest.as_bytes());
    let parts = vec![
        part(view::PartKind::Metadata, fixture("metadata")),
        part(view::PartKind::Labels, labels),
        part(view::PartKind::Styles, fixture("styles")),
    ];
    view::BundleCandidate { revision, mode: view::Mode::JumbleOff, parts }
}

/// The command ID the command layer gives the solver's `view.open` in these tests.
const OPEN: view::CommandId = view::CommandId::Caller(1 << 20);

/// A view model in the solving profile at the layer's revision, with the tool's view open under its declared class.
fn model_with_view(layer: &CommandLayer) -> (view::ViewModel, view::ViewId) {
    let workload = view::Workload::default();
    let mut model = view::ViewModel::new(view::Profile::Solving, profile(), &candidate(layer), workload, workload)
        .expect("Global and Local fit");
    let id = view::ViewId::new("full_600_cell.preview");
    let workload = view::Workload { vertex_evals: 1, cpu_bytes: 1, gpu_bytes: 1, ..view::Workload::default() };
    let declared = declared(&manifest(), BoundaryClass::Checking, workload, view::OutputAllowance::new(BIG, 16))
        .expect("the manifest declares the class");
    model
        .open_view(OPEN, view::Origin::Solver, id.clone(), views::source(&manifest()), declared)
        .expect("the view opens");
    (model, id)
}

// ----- CT-EV3-01

/// CT-EV3-01 (EV3.1, EV3.5, EV3.6), whole for the tool's view, synthetic: the linkage chain runs from the tool's
/// source digest and ranges, through the checked program's `propose` node, the view's nodes, its plans and its
/// instances, to canonical object IDs and back to the source through the installed source maps, and every step back
/// returns the producing digest, the SHA-256 of the tool's source bytes that ran. The two instances of one orbit, one
/// in the support plan and one in the derived plan, keep one canonical identity and their own instance IDs. The view
/// model installs the same publication's content under the tool's built-in source and declared class.
#[test]
fn ct_ev3_01_the_tools_view_links_back_to_the_source_that_ran() {
    let (tool, links) = (tool(), links());
    let producing = sha256_hex(SOURCE.as_bytes());
    assert_eq!((links.digest.as_str(), manifest().source_sha256.as_str()), (producing.as_str(), producing.as_str()));
    assert_eq!(
        tool.producer().module,
        c600_digest::sha256(SOURCE.as_bytes()),
        "the run's producer names the same bytes"
    );
    assert_eq!(links.name, format!("{MODULE}.{ENTRY}"));

    // Source digest and range to the checked program: the admitted entry's one `propose` node, of the parameter `w`.
    assert!(tool.admission.is_admitted());
    let [proposal] = &links.proposals[..] else { panic!("one proposal: {:?}", links.proposals) };
    assert_eq!((proposal.path.as_str(), text(&proposal.call), text(&proposal.word)), ("0", CALL, "w"));
    let declaration = text(&links.entry);
    assert!(declaration.starts_with("export command preview_word(w: Word)"), "{declaration}");
    assert!(declaration.contains(CALL));
    let other = Manifest { source_sha256: sha256_hex(b"another module"), ..manifest() };
    assert!(Tool::new(SOURCE, other).is_err(), "a tool never loads under a digest of other bytes");
    assert!(Links::of(&tool, &SOURCE.replace("preview_word", "preview_words")).is_err(), "nor do its links");

    // The checked program to the view's nodes and plans.
    let (layer, _, publication) = proposed(&[1, -2]);
    let base = base_of(&layer);
    let [entry, shown] = &publication.nodes[..] else { panic!("two nodes") };
    assert_eq!((entry.node, entry.kind, &entry.source), (ENTRY_NODE, NodeKind::Container, &Some(links.entry.clone())));
    assert_eq!(
        (entry.children.as_slice(), entry.content.as_slice()),
        (&[PROPOSAL_NODE][..], &[NodeContent::Text(links.name.clone())][..])
    );
    assert_eq!(
        (shown.node, shown.kind, &shown.source),
        (PROPOSAL_NODE, NodeKind::Control, &Some(proposal.call.clone()))
    );
    assert_eq!(
        (shown.plans.as_slice(), shown.commands.as_slice()),
        (&[SUPPORT_PLAN, DERIVED_PLAN][..], &[ROW.to_owned()][..])
    );
    assert_eq!(publication.sources.keys().collect::<Vec<_>>(), [&producing], "the source that ran is pinned");
    assert_eq!(publication.sources[&producing].as_slice(), SOURCE.as_bytes());

    // Plans to instances to canonical IDs, and back to the source through the installed maps.
    let mut host = host_at(&base);
    install(&mut host, 1, publication.clone());
    let moved = orbits(&publication);
    let n = moved.len() as u64;
    assert!(n >= 1, "the word moves an orbit");
    let preview = layer.preview().expect("the pending preview");
    let support: Vec<ObjectId> = preview.effect.support.iter().map(|row| ObjectId::Orbit(row.orbit as u32)).collect();
    assert_eq!(moved, support);
    for (index, orbit) in (1..).zip(&moved) {
        let copies = [(index, SUPPORT_PLAN, CALL), (n + index, DERIVED_PLAN, "w")];
        let resolved = copies.map(|(id, plan, selected)| {
            let resolution = total(&host, &reply(&host, id), Requested::Occurrence);
            assert_eq!((resolution.instance, resolution.plan), (InstanceId(id), plan));
            let node = publication.nodes.iter().find(|node| node.plans.contains(&resolution.plan)).expect("a node");
            assert_eq!(node.node, PROPOSAL_NODE, "the plan belongs to the proposal's node");
            match host.follow(V1, &resolution.source).expect("the producing version opens") {
                Followed::Range { digest, text, .. } => {
                    assert_eq!((digest, text.as_str()), (producing.clone(), selected))
                }
                other => panic!("instance {id}: {other:?}"),
            }
            assert_eq!(total(&host, &reply(&host, id), Requested::Orbit).target, *orbit);
            resolution
        });
        assert_eq!((&resolved[0].target, &resolved[1].target), (orbit, orbit), "copies share one canonical identity");
        assert_ne!(resolved[0].instance, resolved[1].instance, "each copy keeps its own instance ID");
        assert_eq!(resolved[0].subject, StateRef::Snapshot(base.clone()));
        assert!(matches!(resolved[1].subject, StateRef::Derived(_)));
    }
    let validated = &host.installed(V1).expect("installed").validated;
    assert_eq!(validated.nodes.len(), 2);

    // The view model installs the same publication's content under the tool's source and class.
    let (mut model, id) = model_with_view(&layer);
    let described = content(&publication);
    assert_eq!((described.nodes.len(), described.plans.len(), described.maps.len()), (2, 2, 2));
    assert_eq!((described.plans[0].instances, described.plans[1].instances, described.maps[0].entries), (n, n, n));
    assert_eq!(described.source_bytes, SOURCE.len() as u64);
    assert_eq!(model.pump(|_| view::RunOutcome::Published(described.clone())), 1);
    let installed = model.installed(&id).expect("installed");
    assert_eq!(installed.content, described);
    assert_eq!(installed.provenance.producer, view::ViewSource::BuiltIn { name: MODULE.to_owned() });
    assert_eq!(installed.provenance.class, Some(view::BoundaryClass::Checking));
    assert_eq!(installed.provenance.profile, view::Profile::Solving);
    assert!(declared(&manifest(), BoundaryClass::ViewAid, view::Workload::default(), view::OutputAllowance::new(0, 0))
        .is_err());
}

// ----- CT-EV3-02

/// CT-EV3-02 (EV3.3, EV3.10), whole for the tool's view, synthetic: during a burst of four commits the view model
/// adopts each newest engine bundle at once, without waiting for the view's evaluation, and the display host adopts
/// the newest bundle directly. The tool's view installed at the first revision is marked stale and names its base; it
/// is never drawn over a newer revision, its picks are stale and its publication cannot attach again. One evaluation
/// was running when the burst continued; the revisions that arrived behind it keep one pending request, so the
/// evaluations of the two middle revisions are skipped and the view runs once more, at the newest revision.
#[test]
fn ct_ev3_02_a_burst_adopts_the_newest_bundle_and_skips_obsolete_evaluations() {
    let (mut layer, mut shell) = session();
    let ran = name_word(&mut layer, &mut shell, &[1, -2]);
    let first = view_of(&layer, &ran);
    let base = base_of(&layer);
    let (mut model, id) = model_with_view(&layer);
    let start = *model.adopted();
    let mut evaluated = Vec::new();
    let mut evaluate = |model: &mut view::ViewModel, publication: &Publication| {
        model.pump(|job| {
            evaluated.push(job.base.revision.state);
            view::RunOutcome::Published(content(publication))
        })
    };
    assert_eq!(evaluate(&mut model, &first), 1);
    let old = model.installed(&id).expect("installed").clone();
    assert!(matches!(model.status(&id), Some(view::ViewStatus::Current { base, .. }) if base == old.base));
    let mut host = host_at(&base);
    install(&mut host, 1, first.clone());
    let delayed = reply(&host, 1);

    // The burst: the solver commits the pending preview and names the next word, four times.
    let mut held = None;
    let mut pending = Vec::new();
    let mut revisions = Vec::new();
    let mut newest = first.clone();
    let mut frames = HeadlessFrames::new();
    for (round, word) in [[3, -4], [5, 6], [-7, 8], [9, -10]].iter().enumerate() {
        let preview = layer.preview().expect("the pending preview").id;
        frames.present(&mut shell, &mut layer, preview).expect("presented by the built-in chain");
        let receipt = shell.press(&mut layer, COMMIT, false).expect("the commit chord").expect("not a repeat");
        assert!(matches!(receipt.status, LayerStatus::Committed(_)), "{receipt:?}");
        let ran = name_word(&mut layer, &mut shell, word);
        newest = view_of(&layer, &ran);
        let adopted = model.adopt(&candidate(&layer)).expect("a complete bundle of one revision");
        assert_eq!((model.adopted(), adopted.state), (&adopted, layer.revision().state), "adopted at once");
        revisions.push(adopted);
        if round == 0 {
            let jobs = model.take_jobs();
            let [job] = &jobs[..] else { panic!("one run: {jobs:?}") };
            assert_eq!(job.base.revision, adopted);
            held = Some((job.job, content(&newest)));
        } else {
            pending.push(model.pending(&id).expect("one pending request"));
            assert!(model.take_jobs().is_empty(), "nothing runs behind the running evaluation");
        }
        // Old program output is stale and names its base; it is not drawn over the new revision.
        assert!(matches!(
            model.status(&id),
            Some(view::ViewStatus::Stale { base, adopted: now, .. }) if base == old.base && now == adopted
        ));
        let frame = model.frame_input();
        let drawn = frame.views.iter().find(|drawn| drawn.view == id).expect("visible");
        assert_eq!((frame.revision, &drawn.draw), (adopted, &view::Draw::Stale { base: old.base }));
    }
    assert_eq!(old.base.revision, start);
    pending.dedup();
    assert_eq!(pending.len(), 1, "one pending request, never a queue per revision");

    let (job, late) = held.expect("the held run");
    model.finish(job, view::RunOutcome::Published(late));
    assert_eq!(evaluate(&mut model, &newest), 1, "one more run");
    let states: Vec<u64> = revisions.iter().map(|revision| revision.state).collect();
    assert_eq!(evaluated, [start.state, states[3]], "the two middle revisions are never evaluated");
    assert_eq!(
        model.counters(&id).map(|counters| counters.program_runs),
        Some(3),
        "the first, the held and the newest"
    );
    let last = revisions[3];
    assert!(matches!(model.status(&id), Some(view::ViewStatus::Current { base, .. }) if base.revision == last));
    assert_eq!(model.installed(&id).map(|installed| &installed.content), Some(&content(&newest)));

    // The display host adopts the newest engine bundle directly; the old output is stale there too.
    let now = base_of(&layer);
    assert_eq!(now.state, last.state);
    assert_eq!(host.adopt(&engine_bundle(layer.snapshot(), &now)), Ok(now.clone()));
    assert!(host.stale(V1));
    assert_eq!(host.frame(V1).expect("a frame").engine_revision, now.state);
    let error = host.resolve_pick(&delayed, PickMode::Explicit, Requested::Occurrence, &Unasked).expect_err("stale");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "dependency state"));
    let DisplayStatus::Refused(error) = host.publish(CommandId(2), first).status else { panic!("attached") };
    assert_eq!(error.code, codes::STALE_PLAN);
    install(&mut host, 3, newest);
    assert!(!host.stale(V1));
}

// ----- CT-EV3-03

/// The state after `word` from solved, derived by the kernel outside any session.
fn outside(word: &[i64]) -> Snapshot {
    let kernel = kernel();
    let recipe = kernel.word(word).expect("a word of legal letters");
    match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("derives") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused: {:?}", refused.refusal),
    }
}

/// CT-EV3-03 (EV3.4, EV3.12), source. The derived half runs whole for the tool's view: the state the tool's preview
/// ends in is displayed only through a plan that names its kernel derivation (the base and the digest of the word the
/// solver named) and its base revision, and neither the command layer, the display host nor the view model acquires a
/// revision for it. The imported half runs on a fixture import record, because no import row exists in the Rust
/// layers: the plan states lineage `imported` with the import digest and an explicit comparison base, a plan without
/// the comparison base is refused, and nothing else (no derivation, session ancestor, node or claim) is stated.
#[test]
fn ct_ev3_03_a_derived_display_names_its_derivation_and_base_and_an_import_its_lineage() {
    let word = [1, -2];
    let (layer, _, publication) = proposed(&word);
    let before = (fingerprint(&layer), layer.revision().clone());
    let base = base_of(&layer);
    let preview = layer.preview().expect("the pending preview").clone();
    let end = preview.end.as_ref().expect("no orbit is protected");
    let lineage = Lineage::Derivation { base: Some(StateRef::Snapshot(base.clone())), word: literal_digest(&word) };
    assert_eq!(derivation(&preview, &base), lineage);
    let derived = &publication.plans[1];
    assert_eq!((derived.plan, &derived.base, &derived.derivation), (DERIVED_PLAN, &base, &Some(lineage.clone())));
    assert_eq!(derived.subject.digest(), end.digest());
    assert_eq!(end.digest(), outside(&[1, -2]).digest(), "the state after the word the solver named");
    assert_ne!(end.digest(), base.digest);

    let mut host = host_at(&base);
    install(&mut host, 1, publication.clone());
    let stamp = host.frame(V1).expect("a frame");
    let [session, label] = &stamp.subjects[..] else { panic!("two subjects: {:?}", stamp.subjects) };
    assert_eq!(
        (session.kind, session.digest.as_str(), &session.lineage),
        (SubjectKind::Session, base.digest.as_str(), &None)
    );
    assert_eq!(
        (label.kind, label.digest.as_str(), label.comparison_base),
        (SubjectKind::Derived, end.digest(), base.state)
    );
    assert_eq!(label.lineage, Some(lineage.clone()));
    assert_eq!(stamp.engine_revision, base.state);
    let n = orbits(&publication).len() as u64;
    let picked = total(&host, &reply(&host, n + 1), Requested::Occurrence);
    assert_eq!((picked.subject.digest(), picked.lineage), (end.digest(), Some(lineage)));
    let presented = host.read_presented();
    assert_eq!(presented.len(), 1);
    assert_eq!(presented[0].presented, Presented::NoPreview, "a derived display is no presented preview");

    // No revision is acquired anywhere.
    assert_eq!(host.adopted(), &base);
    assert_eq!((fingerprint(&layer), layer.revision().clone()), before);
    assert_eq!(layer.preview_state(preview.id), Some(PreviewState::Ready));
    let (mut model, id) = model_with_view(&layer);
    let adopted = *model.adopted();
    model.pump(|_| view::RunOutcome::Published(content(&publication)));
    assert_eq!((model.adopted(), model.shown_revision(&id)), (&adopted, Some(adopted)));
    assert_eq!(model.installed(&id).map(|installed| installed.base.revision), Some(adopted));

    // An imported value: lineage `imported`, an explicit comparison base, and nothing else.
    let links = links();
    let import = sha256_hex(b"a fixture import record");
    let value = outside(&[11, 12]);
    let bare = imported_view(V1, &base, geometry(), &links, &import, &value, None);
    let error = validate(&bare, &context()).expect_err("no comparison base");
    assert_eq!(error.code, codes::PLAN_INVALID);
    let imported = imported_view(V1, &base, geometry(), &links, &import, &value, Some(&base));
    let stated = Lineage::Imported { import: import.clone(), comparison_base: Some(base.clone()) };
    assert_eq!(imported.plans[0].derivation, Some(stated.clone()));
    assert!(imported.nodes.is_empty() && imported.plans[0].instances.is_empty(), "it claims and links nothing");
    assert_eq!(imported.plans[0].reachability, None);
    let mut host = host_at(&base);
    install(&mut host, 1, imported);
    let label = &host.installed(V1).expect("installed").validated.subjects[0];
    assert_eq!(
        (label.kind, label.digest.as_str(), label.comparison_base),
        (SubjectKind::Derived, value.digest(), base.state)
    );
    assert_eq!(label.lineage, Some(stated), "no derivation or session ancestor is made up");
    assert_eq!(host.adopted(), &base);
}

// ----- CT-EV3-04

/// CT-EV3-04 (EV3.5), whole for the tool's view, synthetic, with EV3.5's generation vector (7 against 8) on the tool's
/// source and an edited draft of it. A reply under generation 7 is answered with `EV3:stale.display_pick` once
/// generation 8 is installed, before source navigation and whether or not the current source also differs; so is a
/// reply whose camera binding moved. A current reply against an edited draft is answered with `EV3:stale.source_link`
/// naming the producing and the current digest. A newer editable draft and a newly checked module do not invalidate a
/// current pick, and navigation still opens the pinned producing version.
#[test]
fn ct_ev3_04_bindings_are_checked_before_the_source_and_the_producing_version_stays_open() {
    let (layer, _, publication) = proposed(&[1, -2]);
    let (links, base) = (links(), base_of(&layer));
    let producing = links.digest.clone();
    let edited = SOURCE.replace("let r = ", "let receipt = ").replace("(r)", "(receipt)");
    let edited_digest = sha256_hex(edited.as_bytes());
    assert_ne!(edited_digest, producing);
    let SourceLink::Range { start, end, .. } = links.proposals[0].call else { panic!("a range") };

    let mut host = host_at(&base);
    for command in 1..=7 {
        install(&mut host, command, publication.clone());
    }
    let seventh = reply(&host, 1);
    assert_eq!(seventh.binding.publication_generation, 7);
    let range = host.pick_source(&seventh, SOURCE.as_bytes(), &Unasked).expect("current").expect("a range");
    assert_eq!(range.utf8, (start, end));
    let error = host.pick_source(&seventh, edited.as_bytes(), &Unasked).expect_err("the source changed");
    assert_eq!(error.code, codes::STALE_SOURCE);
    assert_eq!(
        (error.context["producing"].as_str(), error.context["current"].as_str()),
        (producing.as_str(), edited_digest.as_str())
    );

    install(&mut host, 8, publication.clone());
    for current in [SOURCE, edited.as_str()] {
        let error = host.pick_source(&seventh, current.as_bytes(), &Unasked).expect_err("a replaced map");
        assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "publication_generation"));
    }

    let eighth = reply(&host, 1);
    assert_eq!(eighth.binding.publication_generation, 8);
    host.set_dependency("input:module", &edited_digest);
    let resolution = total(&host, &eighth, Requested::Occurrence);
    let Followed::Range { digest, text, range } = host.follow(V1, &resolution.source).expect("the producing version")
    else {
        panic!("a range")
    };
    assert_eq!((digest, text.as_str(), range.utf8), (producing, CALL, (start, end)), "the pinned source still opens");
    assert_eq!(host.pick_source(&eighth, edited.as_bytes(), &Unasked).expect_err("edited").code, codes::STALE_SOURCE);

    host.set_camera(V1);
    for current in [SOURCE, edited.as_str()] {
        let error = host.pick_source(&eighth, current.as_bytes(), &Unasked).expect_err("a moved camera");
        assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "camera"));
    }
}

// ----- CT-BND-19

/// Every word literal in a text, `word[...]`, as letters.
fn words_in(text: &str) -> Vec<Vec<i32>> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("word[") {
        let tail = &rest[at + 5..];
        let close = tail.find(']').expect("a closed word literal");
        let letters = &tail[..close];
        let parsed = letters.split(", ").filter(|_| !letters.is_empty()).map(|l| l.parse().expect("a letter"));
        found.push(parsed.collect());
        rest = &tail[close..];
    }
    found
}

/// CT-BND-19 (BND.4), part, source: the tool's views. Every field of the tool's view that holds a word or a value of
/// a word holds it for the word the solver named for the run and for no other: the proposal node's word text, the
/// derivation's word digest, the derived subject, the preview display's literal digest, the frame's preview report and
/// the view model's installed bytes. Before the solver names a word the view holds nothing; a refused word leaves its
/// own text and no plan; and a view cannot be built from a preview of another word or of another command. The part of
/// BND.4 about a live twist that is not released runs in `c600-input`'s `tests/twist.rs`, which joins the input router
/// to this view model and this view.
#[test]
fn ct_bnd_19_the_tools_view_holds_values_only_for_the_word_the_solver_named() {
    // Before the solver names a word, the open view has no output.
    let (mut layer, mut shell) = session();
    let (mut model, id) = model_with_view(&layer);
    model.pump(|_| view::RunOutcome::Published(view::Content::default()));
    assert!(model.installed(&id).is_none_or(|installed| installed.content.is_empty()));
    assert!(matches!(model.status(&id), Some(view::ViewStatus::Empty { .. })), "{:?}", model.status(&id));
    assert!(layer.preview().is_none());

    // The layer refuses the empty word: a word step with no move has the wrong recipe shape.
    for (case, word) in [("admitted", &[1, -2][..]), ("repeated letters", &[5, 5, -3]), ("refused", &[])] {
        let (mut layer, mut shell) = session();
        let ran = name_word(&mut layer, &mut shell, word);
        let [shown] = &ran.shell[..] else { panic!("{case}: one proposal") };
        assert_eq!(shown.word, word, "{case}: the proposal is of the word the solver named");
        let publication = view_of(&layer, &ran);
        let base = base_of(&layer);

        // Node text: one word literal, the solver's.
        let texts: Vec<&str> = publication
            .nodes
            .iter()
            .flat_map(|node| &node.content)
            .filter_map(|content| if let NodeContent::Text(text) = content { Some(text.as_str()) } else { None })
            .collect();
        let literals: Vec<Vec<i32>> = texts.iter().flat_map(|text| words_in(text)).collect();
        assert_eq!(literals, [word.to_vec()], "{case}");
        assert!(texts.contains(&word_text(word).as_str()), "{case}");
        let commands: Vec<&String> = publication.nodes.iter().flat_map(|node| &node.commands).collect();
        assert_eq!(commands, [ROW], "{case}: the one row the proposal targets");
        // The view model's installed bytes hold the same one literal.
        let described = content(&publication);
        let bytes: Vec<&Vec<u8>> =
            described.nodes.iter().chain(described.plans.iter().map(|plan| &plan.payload)).collect();
        let held: Vec<Vec<i32>> = bytes.iter().flat_map(|bytes| words_in(&String::from_utf8_lossy(bytes))).collect();
        assert_eq!(held, [word.to_vec()], "{case}");

        if case == "refused" {
            assert!(matches!(shown.answer, Answer::Refused(_)));
            assert!(layer.preview().is_none() && publication.plans.is_empty(), "a refused word has no plan");
            continue;
        }
        // Digests of a word: the derivation, the preview display and the frame's preview report.
        let preview = layer.preview().expect("the pending preview").clone();
        let named = literal_digest(word);
        let letters: Vec<i64> = word.iter().map(|&letter| letter as i64).collect();
        let after = outside(&letters);
        assert_eq!(preview.literal, named, "{case}");
        let [support, derived] = &publication.plans[..] else { panic!("{case}: two plans") };
        assert_eq!((&support.subject, &support.derivation), (&StateRef::Snapshot(base.clone()), &None), "{case}");
        assert_eq!(derived.subject.digest(), after.digest(), "{case}: the state after the named word");
        let Some(Lineage::Derivation { word: derived_word, .. }) = &derived.derivation else { panic!("{case}") };
        assert_eq!(derived_word, &named, "{case}");
        let display = preview_display(&preview, &base);
        assert_eq!(
            (display.literal.as_str(), preview_shown(&preview).literal.as_str()),
            (named.as_str(), named.as_str())
        );
        let summary = sha256_hex(preview.summary().canonical().as_bytes());
        let state: Vec<&NodeContent> =
            publication.nodes[1].content.iter().filter(|c| matches!(c, NodeContent::State { .. })).collect();
        assert_eq!(state, [&NodeContent::State { row: ROW.to_owned(), digest: summary }], "{case}");

        // No view from a preview of another word, or of another command.
        let mut other = shown.clone();
        other.word = vec![7, 8];
        assert!(proposal_view(V1, &base, geometry(), &links(), 0, &other, Some(&preview)).is_err(), "{case}");
        let mut other = shown.clone();
        other.id = "another/1".to_owned();
        assert!(proposal_view(V1, &base, geometry(), &links(), 0, &other, Some(&preview)).is_err(), "{case}");
    }

    // A preview the shell built for its own draft is not the tool's proposal: no view of the proposal holds it.
    let ran = name_word(&mut layer, &mut shell, &[1, -2]);
    let recipe = Json::Array(vec![json::object([
        ("kind", Json::String("word".into())),
        ("moves", Json::Array(vec![Json::Int(3), Json::Int(4)])),
    ])]);
    let receipt = shell.invoke(&mut layer, ROW, c600_command::draft_args(&recipe)).expect("the shell previews");
    assert!(matches!(receipt.status, LayerStatus::Running(_)), "{receipt:?}");
    layer.run_jobs();
    let replaced = layer.preview().expect("the shell's preview");
    assert_eq!(replaced.literal, literal_digest(&[3, 4]));
    let stale = proposal_view(V1, &base_of(&layer), geometry(), &links(), 0, &ran.shell[0], Some(replaced));
    assert!(stale.is_err(), "the tool's view takes no word from a preview the proposal did not build");
}

// ----- CT-ONE-29

/// CT-ONE-29 (source, whole; ONE.41, RPI.76), synthetic, at the host's adapter through the layer's `present`: a frame
/// that draws the tool's preview through a user stage composes it as an extra display, and its presented report names
/// no preview. The adapter reports each frame with its stamp; the layer refuses a frame that draws the preview through
/// a user stage, one that shows no preview and one that shows a preview the layer does not hold, so the shell holds no
/// presented preview, mints no commit token and the commit chord stops at its gate; nothing changes. A frame in which
/// the built-in chain draws the preview, here over a view drawn by a user stage, presents it, and the shell's token
/// binds exactly that preview and that frame's stamp. A later user-stage frame ends the presentation, a delayed report
/// older than it changes nothing, and the next built-in frame presents the preview again; the chord commits it.
#[test]
fn ct_one_29_a_preview_drawn_through_a_user_stage_binds_no_commit_token() {
    let (mut layer, mut shell, _) = proposed(&[1, -2]);
    let mut frames = HeadlessFrames::new();
    let refused = |report: Result<Option<u64>, CommandError>| report.err().map(|error| error.code);
    let preview = layer.preview().expect("the pending preview").clone();
    let shown = preview_shown(&preview);
    assert_eq!(
        (shown.preview.0, shown.literal.as_str(), shown.base),
        (preview.id, preview.literal.as_str(), layer.revision().state)
    );
    let before = fingerprint(&layer);
    let args = json::object([("preview", Json::Int(preview.id as i64))]);

    let stage =
        StageDescription { kind: StageKind::Fragment, digest: sha256_hex(b"a user stage"), outputs: vec![Output::Rgb] };
    let user = Chain::User(vec![check_outputs(&stage).expect("an opaque RGB fragment stage")]);

    // The preview drawn through the user stage: an extra display with no presented-preview identity.
    let extra = compose(&Chain::BuiltIn, Some((&shown, &user)), &[HostStatus::Preview]);
    assert!(extra.layers.contains(&Layer::ExtraPreview) && !extra.layers.contains(&Layer::Preview(shown.clone())));
    assert_eq!((&extra.presented, extra.commit_binding()), (&Presented::NoPreview, None));
    let presented = presented_frame(&extra, frames.next());
    assert_eq!((&presented.preview, presented.chain), (&None, FrameChain::UserStage));
    let report = report_presented(&extra, frames.next(), &mut shell, &mut layer);
    assert_eq!(refused(report), Some(layer_code::NO_FRESH_PREVIEW));
    assert_eq!((shell.presented(), layer.preview_state(preview.id)), (None, Some(PreviewState::Ready)));
    let gate = shell.prepare(&layer, "operation.execute", args.clone()).err().map(|error| error.code);
    assert_eq!(gate, Some(layer_code::NO_FRESH_PREVIEW), "no token is minted");
    let chord = shell.press(&mut layer, COMMIT, false).err().map(|error| error.code);
    assert_eq!(chord, Some(layer_code::NO_FRESH_PREVIEW));
    assert_eq!(fingerprint(&layer), before);

    // A frame with no preview, and one naming a preview the layer does not hold, are refused as well.
    let none = compose(&user, None, &[]);
    assert_eq!(presented_frame(&none, frames.next()).chain, FrameChain::BuiltIn);
    assert_eq!(
        refused(report_presented(&none, frames.next(), &mut shell, &mut layer)),
        Some(layer_code::NO_FRESH_PREVIEW)
    );
    let mut another = shown.clone();
    another.literal = literal_digest(&[7, 8]);
    let foreign = compose(&Chain::BuiltIn, Some((&another, &Chain::BuiltIn)), &[]);
    let report = report_presented(&foreign, frames.next(), &mut shell, &mut layer);
    assert_eq!(refused(report), Some(layer_code::NO_FRESH_PREVIEW));
    assert_eq!((shell.presented(), layer.preview_state(preview.id)), (None, Some(PreviewState::Ready)));
    assert_eq!(fingerprint(&layer), before);

    // The display host's own frame report of the preview, drawn by the built-in chain.
    let base = base_of(&layer);
    let mut host = host_at(&base);
    host.show_preview(V1, Some(preview_display(&preview, &base))).expect("the engine's preview");
    host.frame(V1).expect("a frame");
    let reported = host.read_presented();
    assert_eq!(reported.last().map(|frame| &frame.presented), Some(&Presented::Previews(vec![shown.clone()])));
    assert_eq!(host.presented_preview(), Some(shown.clone()));
    let display = &reported.last().expect("the host's report").stamp;
    let stamp = frame_stamp(display);
    let fields = (stamp.device_epoch, stamp.surface, stamp.surface_generation, stamp.sequence);
    assert_eq!(fields, (display.device_epoch, display.surface, display.surface_generation, display.sequence));

    // The built-in chain draws the preview over a view drawn by the user stage: the token binds to that preview and to
    // the frame that presented it.
    let built_in = compose(&user, Some((&shown, &Chain::BuiltIn)), &[HostStatus::Preview]);
    assert_eq!(built_in.commit_binding(), Some(&shown));
    let first = frames.next();
    assert_eq!(report_presented(&built_in, first.clone(), &mut shell, &mut layer), Ok(Some(preview.id)));
    assert_eq!(layer.preview_state(preview.id), Some(PreviewState::Presented));
    let bound = shell.presented().cloned().expect("a presented preview");
    let binding = (bound.id, bound.literal.as_str(), &bound.deps, &bound.frame);
    assert_eq!(binding, (preview.id, preview.literal.as_str(), &preview.deps, &first));

    // A later user-stage frame ends the presentation; a delayed built-in frame from before it changes nothing.
    let delayed = frames.next();
    assert_eq!(
        refused(report_presented(&extra, frames.next(), &mut shell, &mut layer)),
        Some(layer_code::NO_FRESH_PREVIEW)
    );
    assert_eq!((shell.presented(), layer.preview_state(preview.id)), (None, Some(PreviewState::Ready)));
    let report = report_presented(&built_in, delayed, &mut shell, &mut layer);
    assert_eq!(refused(report), Some(layer_code::NO_FRESH_PREVIEW));
    assert_eq!((shell.presented(), layer.preview_state(preview.id)), (None, Some(PreviewState::Ready)));
    let chord = shell.press(&mut layer, COMMIT, false).err().map(|error| error.code);
    assert_eq!(chord, Some(layer_code::NO_FRESH_PREVIEW));
    assert_eq!(fingerprint(&layer), before);

    // The next built-in frame presents it again, and the token binds that frame.
    let again = frames.next();
    assert_eq!(report_presented(&built_in, again.clone(), &mut shell, &mut layer), Ok(Some(preview.id)));
    let bound = shell.presented().cloned().expect("a presented preview");
    assert_eq!((bound.id, &bound.frame), (preview.id, &again));
    let prepared = shell.prepare(&layer, "operation.execute", args).expect("a token is minted");
    assert_eq!(prepared.token.as_ref().and_then(|token| token.binding.preview.clone()), Some(bound));
    let receipt = shell.send(&mut layer, prepared);
    let LayerStatus::Committed(record) = receipt.status else { panic!("{receipt:?}") };
    assert_eq!(record.pre, preview.base.digest());
    assert_eq!(Some(record.post.as_str()), preview.end.as_ref().map(|end| end.digest()));
    assert_eq!(layer.preview_state(preview.id), Some(PreviewState::Committed));
}
