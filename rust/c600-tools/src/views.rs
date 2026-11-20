//! The 600-cell-Full tool's views (LP3, C5 next, part 2): what the shell shows of the tool's proposals, as a
//! publication of the display contract (`c600-display`) and as the content the headless view model (`c600-view`)
//! installs, both through their public APIs only.
//!
//! The tool prepares a preview of the word the solver named and reads no state (LQ21, LG3.13), so it publishes no view
//! itself. The host builds the view of a proposal from what the run left for the shell ([`Shown`]) and from the
//! engine's preview object ([`Preview`]), which only the shell sees in a solving session (charter E6). Every part of
//! the view links back to the tool's source: the digest is the SHA-256 of the bytes that ran, the manifest's
//! `source_sha256`, and each range is one the parser's source map gives for the checked program (EV3.1, LG4.12).
//!
//! A view holds one word: the word of the proposal, which the built-in column admits only as the word the solver named
//! (BND.4, chapter 07 section 8). No function here takes a word from anywhere else.
//!
//! The command layer's view rows, `view.open` and `view.close`, reach the same view model through [`ModelViews`], the
//! host's `c600_command::Views` (EV3.2, RPI.81 to RPI.83; LP2's pull request 271).
//!
//! The draft choices of this module are listed under the crate documentation's drafts.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use c600_command::{
    code as layer_code, CommandLayer, Error as CommandError, FrameChain, FrameStamp, PresentedFrame, Preview,
    PreviewState, Revision as Head, Shell, ShownPreview, Store, ViewSourceRef, Views,
};
use c600_digest::sha256_hex;
use c600_display::array::Array;
use c600_display::canonical;
use c600_display::pick::{FrameStamp as DisplayStamp, PreviewShown};
use c600_display::plan::{
    Content as NodeContent, DerivedRef, DisplayInstance, DisplayPlan, GeometryBinding, InstanceId, LayoutInput,
    LayoutKind, Level, Lineage, MapEntry, MapId, Mode, NodeId, NodeKind, ObjectId, PlanId, PreviewDisplay, PreviewId,
    Primitive, Publication, RevisionRef, SourceLink, SourceMap, StateRef, ViewId, ViewNode,
};
use c600_display::produce::{full_set, retained, GeometryInput};
use c600_display::stage::{Composition, Layer};
use c600_display::validate::{BoundArray, EngineBundle};
use c600_ir::Expr;
use c600_kernel::json::Value;
use c600_kernel::{IdentityBlock, Snapshot};
use c600_run::boundary::{Answer, Shown};
use c600_syntax::{lower, parse, Span};
use c600_view as view;

use crate::expand::literal_digest;
use crate::tool::{BoundaryClass, Manifest, Tool};

/// The node of the tool's entry.
pub const ENTRY_NODE: NodeId = NodeId(1);
/// The node of the proposal.
pub const PROPOSAL_NODE: NodeId = NodeId(2);
/// The plan of the preview's support over the session state at the base.
pub const SUPPORT_PLAN: PlanId = PlanId(1);
/// The plan of the derived state the preview ends in.
pub const DERIVED_PLAN: PlanId = PlanId(2);

/// Where the tool's views link back to: the tool's source bytes, their digest and the ranges of the checked program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Links {
    /// The SHA-256 of the source bytes that ran: the manifest's `source_sha256`.
    pub digest: String,
    /// The source bytes, pinned with every publication (RM.10).
    pub bytes: Arc<Vec<u8>>,
    /// The entry's name, `<module>.<entry>`.
    pub name: String,
    /// The entry's declaration.
    pub entry: SourceLink,
    /// The entry's `propose` nodes, in IR path order.
    pub proposals: Vec<Proposal>,
}

/// One `propose` node of the entry's body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    /// The node's IR path, as the checker's diagnostics and the parser's source map name it.
    pub path: String,
    /// The whole call.
    pub call: SourceLink,
    /// The word argument.
    pub word: SourceLink,
}

fn paths<'e>(expr: &'e Expr, path: &str, out: &mut Vec<(String, &'e Expr)>) {
    out.push((if path.is_empty() { "root".to_owned() } else { path.to_owned() }, expr));
    for (index, child) in expr.children().into_iter().enumerate() {
        let below = if path.is_empty() { index.to_string() } else { format!("{path}.{index}") };
        paths(child, &below, out);
    }
}

impl Links {
    /// The links of `tool`, whose source is `source`. It fails if `source` is not the bytes the tool's manifest names,
    /// or if the parser's source map has no range for the entry or for one of its proposals.
    pub fn of(tool: &Tool, source: &str) -> Result<Links, String> {
        let digest = sha256_hex(source.as_bytes());
        if digest != tool.manifest.source_sha256 {
            return Err("the source is not the one the tool's manifest names".into());
        }
        let file = parse(source.as_bytes());
        if !file.diagnostics.is_empty() || !file.holes.is_empty() {
            return Err("the source does not parse".into());
        }
        let lowered = lower(&[file]).map_err(|error| format!("the source does not lower: {error}"))?;
        let (module, entry) = (tool.manifest.module, tool.manifest.entry);
        let map = lowered.source(module).filter(|map| map.digest == digest).ok_or("no source map of the module")?;
        let link =
            |span: Span| SourceLink::Range { digest: digest.clone(), start: span.start as u64, end: span.end as u64 };
        let declared = map.declarations.get(entry).copied().ok_or("no declaration range of the entry")?;
        let function = tool.program.modules.iter().filter(|m| m.name == module).flat_map(|m| &m.functions);
        let function = function.into_iter().find(|f| f.name == entry).ok_or("no entry in the checked program")?;
        let mut nodes = Vec::new();
        paths(&function.body, "", &mut nodes);
        let mut proposals = Vec::new();
        for (path, node) in nodes {
            if !matches!(node, Expr::Propose { .. }) {
                continue;
            }
            let call = map.span(entry, &path).ok_or("no range of a proposal")?;
            let word = if path == "root" { "0".to_owned() } else { format!("{path}.0") };
            let word = map.span(entry, &word).ok_or("no range of a proposal's word")?;
            proposals.push(Proposal { path, call: link(call), word: link(word) });
        }
        let (bytes, name) = (Arc::new(source.as_bytes().to_vec()), format!("{module}.{entry}"));
        let entry = link(declared);
        Ok(Links { digest, bytes, name, entry, proposals })
    }

    fn pinned(&self) -> BTreeMap<String, Arc<Vec<u8>>> {
        BTreeMap::from([(self.digest.clone(), self.bytes.clone())])
    }
}

/// The display contract's reference to a command-layer revision. The identity block must be the layer's model, the
/// mode is Jumble off, and the head is the journal event's number.
pub fn revision_ref(head: &Head, identity: &IdentityBlock) -> Result<RevisionRef, String> {
    if identity.model_id != head.identity {
        return Err("the identity block is of another model".into());
    }
    if head.mode != "jumble_off" {
        return Err("the tool's views are of 600-cell-Full with Jumble off".into());
    }
    let event = head.head.strip_prefix('e').and_then(|n| n.parse::<u64>().ok());
    Ok(RevisionRef {
        session: head.session.clone(),
        epoch: head.epoch,
        identity: identity.clone(),
        mode: Mode::JumbleOff,
        head: event.ok_or("the head is not a journal event")?,
        state: head.state,
        workspace: head.workspace,
        protection: head.protection,
        digest: head.digest.clone(),
    })
}

/// The view model's revision of a command-layer revision: its counters and the label digest's 32 bytes.
pub fn model_revision(head: &Head) -> Result<view::Revision, String> {
    let digest = digest_bytes(&head.digest).ok_or("the label digest is not a SHA-256 in hex")?;
    Ok(view::Revision::new(head.epoch, head.state, head.workspace, head.protection, digest))
}

/// The 32 bytes that 64 hex digits spell, or `None` for any other text (a sign, which `from_str_radix` would take,
/// included).
fn digest_bytes(hex: &str) -> Option<view::Digest> {
    let hex = hex.as_bytes();
    if hex.len() != 64 || !hex.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut digest = [0u8; 32];
    for (byte, pair) in digest.iter_mut().zip(hex.chunks(2)) {
        *byte = std::str::from_utf8(pair).ok().and_then(|pair| u8::from_str_radix(pair, 16).ok())?;
    }
    Some(digest)
}

/// The engine bundle of a Full state at `revision` for the display host: the labels and the full visibility and
/// interaction sets, each bound to the state revision.
pub fn engine_bundle(state: &Snapshot, revision: &RevisionRef) -> EngineBundle {
    let set = full_set();
    let input = retained(state, &set, &set, Vec::new());
    let arrays = [("labels", input.labels), ("visibility", input.visibility), ("interaction", input.interaction)];
    let bound = |(name, array): (&str, Array)| BoundArray { name: name.to_owned(), revision: revision.state, array };
    EngineBundle { revision: revision.clone(), arrays: arrays.into_iter().map(bound).collect() }
}

/// The view model's source reference of the tool's views: the built-in source named after the tool's module (RPI.83).
pub fn source(manifest: &Manifest) -> view::ViewSource {
    view::ViewSource::BuiltIn { name: manifest.module.to_owned() }
}

/// The view model's class of a manifest class (BND.27): the same class by name.
pub fn class(class: BoundaryClass) -> view::BoundaryClass {
    match class {
        BoundaryClass::Tracking => view::BoundaryClass::Tracking,
        BoundaryClass::Analysis => view::BoundaryClass::Analysis,
        BoundaryClass::Checking => view::BoundaryClass::Checking,
        BoundaryClass::Protection => view::BoundaryClass::Protection,
        BoundaryClass::ViewAid => view::BoundaryClass::ViewAid,
        BoundaryClass::SetupSearchNamedTarget => view::BoundaryClass::SetupSearchNamedTarget,
        BoundaryClass::RepairList => view::BoundaryClass::RepairList,
    }
}

/// What the host declares when it opens the view of the tool's proposals. A preview is fresh for one state and one
/// protection revision (KN0.8), so the view depends on both. Its class is `stated`, which the manifest must declare.
pub fn declared(
    manifest: &Manifest,
    stated: BoundaryClass,
    workload: view::Workload,
    output: view::OutputAllowance,
) -> Result<view::Declared, String> {
    if !manifest.classes.contains(&stated) {
        return Err(format!("the manifest does not declare the class `{}`", stated.name()));
    }
    let depends = view::Depends::STATE.union(view::Depends::PROTECTION);
    Ok(view::Declared { depends, workload, output, class: Some(class(stated)) })
}

/// The words of a view's letters, in the language's literal form: `word[+1, -2]`.
pub fn word_text(letters: &[i32]) -> String {
    let letters: Vec<String> = letters.iter().map(|letter| format!("{letter:+}")).collect();
    format!("word[{}]", letters.join(", "))
}

fn sheet(ids: &[InstanceId]) -> LayoutInput {
    let positions: Vec<f32> = (0..ids.len()).flat_map(|row| [row as f32, 0.0]).collect();
    LayoutInput {
        instances: ids.to_vec(),
        positions: Array::f32s(vec![ids.len() as u64, 2], &positions),
        edges: None,
        triangles: None,
        kind: LayoutKind::Sheet,
    }
}

fn binding(geometry: &GeometryInput) -> GeometryBinding {
    GeometryBinding { identity: geometry.identity.clone(), digests: geometry.digests() }
}

/// A plan at `base` over `subject`, with its source map: the instances `first..` show `objects` on a sheet after the
/// primitives in `before`, and every instance links to `link`.
#[allow(clippy::too_many_arguments)]
fn plan(
    id: PlanId,
    base: &RevisionRef,
    geometry: &GeometryInput,
    subject: StateRef,
    derivation: Option<Lineage>,
    before: Vec<Primitive>,
    first: u64,
    objects: &[ObjectId],
    link: &SourceLink,
) -> (DisplayPlan, SourceMap) {
    let ids: Vec<InstanceId> = (first..first + objects.len() as u64).map(InstanceId).collect();
    let mut primitives = before;
    let layout = primitives.len() as u32;
    if !ids.is_empty() {
        primitives.push(Primitive::Layout(sheet(&ids)));
    }
    let instances: Vec<DisplayInstance> = ids
        .iter()
        .zip(objects)
        .map(|(&instance, object)| DisplayInstance {
            instance,
            plan: id,
            object: object.clone(),
            primitive: layout,
            piece: None,
            interaction: true,
        })
        .collect();
    let entries =
        instances.iter().map(|i| (i.instance, MapEntry { object: i.object.clone(), source: link.clone() })).collect();
    let plan = DisplayPlan {
        plan: id,
        base: base.clone(),
        subject,
        derivation,
        reachability: None,
        dimension: 4,
        geometry: binding(geometry),
        primitives,
        charts: Vec::new(),
        instances,
        source_map: MapId(id.0),
        precision: "binary32".to_owned(),
        dependencies: BTreeMap::from([
            ("state".to_owned(), base.state.to_string()),
            ("protection".to_owned(), base.protection.to_string()),
        ]),
    };
    let sources = vec![link.digest().to_owned()];
    (plan, SourceMap { map: MapId(id.0), plan: id, base: base.clone(), sources, entries })
}

/// Checks that `preview` is the engine's preview of this proposal at `base`: the proposal's command built it, its
/// literal digest is the digest of the proposal's word, and it reads the state and protection of `base`.
fn of_proposal(shown: &Shown, preview: &Preview, base: &RevisionRef) -> Result<(), String> {
    if preview.command != shown.id {
        return Err("the preview was built by another command".into());
    }
    if preview.literal != literal_digest(&shown.word) {
        return Err("the preview is of another word than the proposal's".into());
    }
    if preview.base.digest() != base.digest
        || (preview.deps.state, preview.deps.protection) != (base.state, base.protection)
    {
        return Err("the preview reads another revision than the base".into());
    }
    if !matches!(preview.state, PreviewState::Ready | PreviewState::Presented) {
        return Err("the preview is not the pending built preview".into());
    }
    Ok(())
}

/// The lineage of the derived state a preview ends in: the kernel derivation from the session state at `base` by the
/// preview's literal word (KN2.11, EV3.4).
pub fn derivation(preview: &Preview, base: &RevisionRef) -> Lineage {
    Lineage::Derivation { base: Some(StateRef::Snapshot(base.clone())), word: preview.literal.clone() }
}

/// The view of one proposal of the tool, for the view `target` at `base`.
///
/// It always has two nodes: the entry, linked to its declaration, and the proposal, linked to its `propose` call,
/// naming the row it targets and holding the proposal's word as text. `proposal` is the index of the `propose` node
/// among [`Links::proposals`].
///
/// With the engine's preview of the proposal it also has the preview's result digest as state content and two plans.
/// [`SUPPORT_PLAN`] is over the session state at `base`: one instance per orbit the word moves (the effect's support
/// rows), each linked to the `propose` call. [`DERIVED_PLAN`] is over the derived state the preview ends in, named by
/// its kernel derivation and `base`: the state's retained input and one more instance per moved orbit, each linked to
/// the call's word argument. Without an end state (a protected orbit would move) there is no derived plan. A refused
/// proposal has the nodes only, with the refusal's code as text.
pub fn proposal_view(
    target: ViewId,
    base: &RevisionRef,
    geometry: &GeometryInput,
    links: &Links,
    proposal: usize,
    shown: &Shown,
    preview: Option<&Preview>,
) -> Result<Publication, String> {
    let link = links.proposals.get(proposal).ok_or("the entry has no such proposal")?;
    let mut content = vec![NodeContent::Text(word_text(&shown.word))];
    if let Answer::Refused(refusal) = &shown.answer {
        content.push(NodeContent::Text(refusal.code.to_owned()));
    }
    let (mut plans, mut maps) = (Vec::new(), Vec::new());
    if let Some(preview) = preview {
        of_proposal(shown, preview, base)?;
        let digest = sha256_hex(preview.summary().canonical().as_bytes());
        content.push(NodeContent::State { row: shown.target.clone(), digest });
        let orbits: Vec<ObjectId> =
            preview.effect.support.iter().map(|row| ObjectId::Orbit(row.orbit as u32)).collect();
        let session = StateRef::Snapshot(base.clone());
        let (support, map) = plan(SUPPORT_PLAN, base, geometry, session, None, Vec::new(), 1, &orbits, &link.call);
        plans.push(support);
        maps.push(map);
        if let Some(end) = &preview.end {
            let subject = DerivedRef {
                digest: end.digest().to_owned(),
                level: Level::Labels,
                identity: geometry.identity.clone(),
            };
            let state = vec![Primitive::Retained(retained(end, &full_set(), &full_set(), Vec::new()))];
            let (derived, map) = plan(
                DERIVED_PLAN,
                base,
                geometry,
                StateRef::Derived(subject),
                Some(derivation(preview, base)),
                state,
                1 + orbits.len() as u64,
                &orbits,
                &link.word,
            );
            plans.push(derived);
            maps.push(map);
        }
    }
    let node = |node, kind, content, children, source: &SourceLink, commands| ViewNode {
        node,
        base: base.clone(),
        kind,
        content,
        plans: Vec::new(),
        children,
        source: Some(source.clone()),
        commands,
        claims: Vec::new(),
    };
    let entry = vec![NodeContent::Text(links.name.clone())];
    let mut shown_node =
        node(PROPOSAL_NODE, NodeKind::Control, content, Vec::new(), &link.call, vec![shown.target.clone()]);
    shown_node.plans = plans.iter().map(|plan| plan.plan).collect();
    let nodes =
        vec![node(ENTRY_NODE, NodeKind::Container, entry, vec![PROPOSAL_NODE], &links.entry, Vec::new()), shown_node];
    Ok(Publication { target, base: base.clone(), nodes, plans, maps, sources: links.pinned() })
}

/// The display of a value imported under KN2.14, for the view `target` at `base`: one plan over the imported state
/// whose lineage is `imported` with the import record's digest and the comparison base the caller states. No
/// derivation, session ancestor or revision is made up for it (EV3.12), and it has no node and no instance. The tool
/// imports nothing: the caller names the import.
pub fn imported_view(
    target: ViewId,
    base: &RevisionRef,
    geometry: &GeometryInput,
    links: &Links,
    import: &str,
    state: &Snapshot,
    comparison_base: Option<&RevisionRef>,
) -> Publication {
    let subject =
        DerivedRef { digest: state.digest().to_owned(), level: Level::Labels, identity: geometry.identity.clone() };
    let lineage = Lineage::Imported { import: import.to_owned(), comparison_base: comparison_base.cloned() };
    let retained = vec![Primitive::Retained(retained(state, &full_set(), &full_set(), Vec::new()))];
    let subject = StateRef::Derived(subject);
    let (plan, map) = plan(SUPPORT_PLAN, base, geometry, subject, Some(lineage), retained, 1, &[], &links.entry);
    Publication {
        target,
        base: base.clone(),
        nodes: Vec::new(),
        plans: vec![plan],
        maps: vec![map],
        sources: links.pinned(),
    }
}

/// The view model's content of a display publication: what `c600-view` installs and digests for it.
///
/// Each node is the display contract's canonical metadata of a publication holding that node alone; each plan's
/// descriptor carries the counts of the plan and, as its payload, the canonical metadata of a publication holding the
/// plan and its map alone; `source_bytes` is the pinned source length.
pub fn content(publication: &Publication) -> view::Content {
    let alone = |nodes: Vec<ViewNode>, plans: Vec<DisplayPlan>, maps: Vec<SourceMap>| {
        let part = Publication {
            target: publication.target,
            base: publication.base.clone(),
            nodes,
            plans,
            maps,
            sources: BTreeMap::new(),
        };
        canonical::publication(&part).canonical().into_bytes()
    };
    let rows = |plan: &DisplayPlan| -> u64 {
        let rows = plan.primitives.iter().map(|primitive| match primitive {
            Primitive::Layout(layout) => layout.instances.len() as u64,
            _ => 0,
        });
        rows.sum()
    };
    let nodes = publication.nodes.iter().map(|node| alone(vec![node.clone()], Vec::new(), Vec::new())).collect();
    let plans = publication
        .plans
        .iter()
        .map(|plan| {
            let maps: Vec<SourceMap> = publication.maps.iter().filter(|m| m.map == plan.source_map).cloned().collect();
            view::PlanDescriptor {
                plan: plan.plan.0,
                source_map: plan.source_map.0,
                instances: plan.instances.len() as u64,
                charts: plan.charts.len() as u64,
                layout_vertices: rows(plan),
                payload: alone(Vec::new(), vec![plan.clone()], maps),
                ..view::PlanDescriptor::default()
            }
        })
        .collect();
    let maps = publication
        .maps
        .iter()
        .map(|map| view::MapDescriptor { map: map.map.0, entries: map.entries.len() as u64 })
        .collect();
    let source_bytes = publication.sources.values().map(|bytes| bytes.len() as u64).sum();
    view::Content { nodes, plans, maps, source_bytes }
}

/// The preview display row of the engine's preview (RM.9): the host takes it from the preview object, never from a
/// program.
pub fn preview_display(preview: &Preview, base: &RevisionRef) -> PreviewDisplay {
    PreviewDisplay {
        preview: PreviewId(preview.id),
        base: base.clone(),
        literal: preview.literal.clone(),
        effect: preview.effect.net_sha256.clone(),
        primitives: Vec::new(),
    }
}

/// What a frame shows of the engine's preview (RM.26).
pub fn preview_shown(preview: &Preview) -> PreviewShown {
    PreviewShown { preview: PreviewId(preview.id), literal: preview.literal.clone(), base: preview.deps.state }
}

/// The command layer's stamp of a display frame (RM.24): its device epoch, surface, surface generation and sequence.
/// The display host keeps one sequence for all its frames, as the layer's frame order needs.
pub fn frame_stamp(stamp: &DisplayStamp) -> FrameStamp {
    FrameStamp {
        device_epoch: stamp.device_epoch,
        surface: stamp.surface,
        surface_generation: stamp.surface_generation,
        sequence: stamp.sequence,
    }
}

/// The command layer's report of one composed frame (RM.26): the preview its presented report names, which
/// `c600-display` gives only for a preview its built-in chain drew, and the chain, a user stage when the frame holds a
/// preview drawn through user stages, an extra display with no presented-preview identity (RPI.76).
pub fn presented_frame(composition: &Composition, stamp: FrameStamp) -> PresentedFrame {
    let preview = composition.commit_binding().map(|shown| ShownPreview {
        id: shown.preview.0,
        literal: shown.literal.clone(),
        base: shown.base,
    });
    let extra = composition.layers.iter().any(|layer| matches!(layer, Layer::ExtraPreview));
    let chain = if extra { FrameChain::UserStage } else { FrameChain::BuiltIn };
    PresentedFrame { preview, stamp, chain }
}

/// The adapter's report of one composed frame to the command layer through the shell (RM.26, ONE.41, RPI.76), with the
/// frame's stamp. The layer decides: a frame that presents the pending preview through the built-in chain binds it with
/// the frame's stamp; any other frame (one that shows no preview, another preview, or a preview drawn through a user
/// stage) ends the pending preview's presentation and is refused with `KN5:stale.no_fresh_preview`, and the shell's
/// binding ends with it, so the shell holds no presented preview to bind a commit token to. A report older than the
/// newest one is refused and changes nothing. Returns the preview presented. With no preview shown, pending or bound,
/// there is nothing to present or end, and nothing is reported (`Ok(None)`).
///
/// Minting and verifying tokens stay in `c600-command`; this function only maps the display's report to the layer's.
pub fn report_presented<S: Store>(
    composition: &Composition,
    stamp: FrameStamp,
    shell: &mut Shell,
    layer: &mut CommandLayer<S>,
) -> Result<Option<u64>, CommandError> {
    let frame = presented_frame(composition, stamp);
    let shown = frame.preview.as_ref().map(|shown| shown.id);
    let pending = layer.preview().map(|preview| preview.id);
    let bound = shell.presented().map(|binding| binding.id);
    match shown.or(pending).or(bound) {
        Some(id) => shell.present(layer, id, &frame).map(|()| Some(id)),
        None => Ok(None),
    }
}

// ----- the command layer's view rows on the view model

/// What the host declares for a source when the layer admits `view.open` (RPI.83, BND.27): the view's dependencies,
/// its workload, the output allowance of each revision-driven run and, for a built-in source, the class its tool's
/// manifest declares. A source the host does not serve is refused here, and the receipt carries the refusal.
pub type Declare = dyn Fn(&ViewSourceRef) -> Result<view::Declared, CommandError> + Send + Sync;

/// The command layer's view rows, `view.open` and `view.close`, served by `c600-view`'s view model (EV3.2, RPI.81 to
/// RPI.83; LP2's pull request 271).
///
/// The layer admits either row (the argument forms, a source's overlay row of kind `view`, RPI.83's solver command) and
/// calls this host once per command ID. The view model then applies its own rules: the view namespace, RPI.82's budget
/// and the view lane. The host's frame loop shares the model.
///
/// Each layer command ID becomes the model's [`view::CommandId::Caller`], numbered from 1 in the order this host first
/// sees it, so the cancel reason of a run that a close supersedes names the command that closed its view
/// ([`ModelViews::command`]). Every call states [`view::Origin::Solver`], since the layer calls the host only inside a
/// command the solver invoked. A closed view's resources stay charged until the host, at its frame boundary, takes the
/// retirement tokens ([`ModelViews::take_retiring`]) and completes each one once the backend proves the resources
/// unused (RM.19).
#[derive(Clone)]
pub struct ModelViews {
    model: Arc<Mutex<view::ViewModel>>,
    declare: Arc<Declare>,
    host: Arc<Mutex<Numbering>>,
}

/// The layer's command IDs this host numbered, and the retirement tokens of closed views not yet taken.
#[derive(Default)]
struct Numbering {
    numbers: BTreeMap<String, u64>,
    commands: Vec<String>,
    retiring: Vec<view::RetireToken>,
}

impl ModelViews {
    /// The layer's view rows on `model`, which declares each source it opens with `declare`.
    pub fn new(
        model: Arc<Mutex<view::ViewModel>>,
        declare: impl Fn(&ViewSourceRef) -> Result<view::Declared, CommandError> + Send + Sync + 'static,
    ) -> ModelViews {
        ModelViews { model, declare: Arc::new(declare), host: Arc::default() }
    }

    /// The view model this host serves.
    pub fn model(&self) -> &Arc<Mutex<view::ViewModel>> {
        &self.model
    }

    /// The layer's command ID behind a [`view::CommandId::Caller`] this host numbered.
    pub fn command(&self, id: view::CommandId) -> Option<String> {
        let view::CommandId::Caller(number) = id else { return None };
        let index = usize::try_from(number.checked_sub(1)?).ok()?;
        self.numbering().commands.get(index).cloned()
    }

    /// The retirement tokens of the views closed since the last call, for `ViewModel::complete_retirement` (RM.19).
    pub fn take_retiring(&self) -> Vec<view::RetireToken> {
        std::mem::take(&mut self.numbering().retiring)
    }

    fn numbering(&self) -> MutexGuard<'_, Numbering> {
        self.host.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn view_model(&self) -> MutexGuard<'_, view::ViewModel> {
        self.model.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The model's command ID of the layer's command `command`.
    fn caller(&self, command: &str) -> view::CommandId {
        let mut numbering = self.numbering();
        if let Some(number) = numbering.numbers.get(command) {
            return view::CommandId::Caller(*number);
        }
        numbering.commands.push(command.to_owned());
        let number = numbering.commands.len() as u64;
        numbering.numbers.insert(command.to_owned(), number);
        view::CommandId::Caller(number)
    }
}

impl Views for ModelViews {
    fn open(&mut self, command: &str, name: &str, source: &ViewSourceRef) -> Result<(), CommandError> {
        let declared = (self.declare)(source)?;
        let source = model_source(source)?;
        let caller = self.caller(command);
        let opened =
            self.view_model().open_view(caller, view::Origin::Solver, view::ViewId::new(name), source, declared);
        opened.map(|_generation| ()).map_err(refusal)
    }

    fn close(&mut self, command: &str, name: &str) -> Result<(), CommandError> {
        let caller = self.caller(command);
        let closed = self.view_model().close_view(caller, view::Origin::Solver, &view::ViewId::new(name));
        if let Some(token) = closed.map_err(refusal)? {
            self.numbering().retiring.push(token);
        }
        Ok(())
    }
}

/// The view model's form of a source: an overlay's module digest as its 32 bytes and its typed arguments as their
/// canonical JSON bytes (JS.2). The layer admits only 64 lowercase hex digits as the digest.
fn model_source(source: &ViewSourceRef) -> Result<view::ViewSource, CommandError> {
    Ok(match source {
        ViewSourceRef::BuiltIn { name } => view::ViewSource::BuiltIn { name: name.clone() },
        ViewSourceRef::Overlay { row, module, arguments } => view::ViewSource::Overlay {
            row: row.clone(),
            module: digest_bytes(module).ok_or_else(|| CommandError::new(layer_code::ARGUMENT_FORM, "source"))?,
            arguments: Value::Object(arguments.clone()).canonical().into_bytes(),
        },
    })
}

/// A refusal of the view model as the layer's receipt carries it: the same code, with the view or the ceiling's field
/// as the detail, as LP2's stand-in host in `c600-command`'s `tests/view_rows.rs` gives them.
fn refusal(error: view::Error) -> CommandError {
    let detail = match error.context {
        view::Context::View(name) => name,
        view::Context::Ceiling { field, .. } => field.to_owned(),
        other => format!("{other:?}"),
    };
    CommandError::new(error.code, detail)
}
