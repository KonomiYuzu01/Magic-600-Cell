//! The host validator of a program publication (RPI.10, RM.20): every reference resolves inside the publication or
//! its pinned inputs, every binding matches (RPI.4, RPI.7, RPI.8, RPI.21, RPI.65), every payload meets RM.6/RM.7,
//! every row and stage is in the support report (RPI.17, RM.15) and every count is within the admission ceiling
//! (RM.11, RM.12). A failure invalidates the whole publication; nothing is truncated or repaired (RM.27).
//!
//! Precedence (a draft, RM.12): a malformed publication is `EV3:input.display_plan_invalid` even when it also names
//! an unsupported row; a well-formed one with an unsupported row or dimension is `EV3:input.display_unsupported`;
//! only a publication with neither is counted against the ceilings.

use std::collections::{BTreeMap, BTreeSet};

use c600_kernel::{IdentityBlock, Puzzle, PIECES, SLOTS};

use crate::array::{Array, Elem};
use crate::canonical;
use crate::chart::{self, Admission, ChartAdmission, ChartCheck};
use crate::error::{codes, Error};
use crate::plan::*;
use crate::source;

/// The cells, and so the grips, of the 600-cell.
pub const CELLS: u32 = 600;

/// The order of K⁺, the retained rotation group a `pose_lattice` entry indexes (render/1 section 3).
pub const K_PLUS: i32 = 7200;

macro_rules! limits {
    ($($field:ident),* $(,)?) => {
        /// RM.11's versioned `LimitProfile`: a finite ceiling for every field; a missing field is not unlimited.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct LimitProfile {
            pub version: String,
            $(pub $field: u64,)*
        }

        impl LimitProfile {
            /// A fixture profile with every ceiling at `value`; declared limits, not measured ones (RPI.50).
            pub fn uniform(version: &str, value: u64) -> LimitProfile {
                LimitProfile { version: version.to_owned(), $($field: value,)* }
            }

            /// RM.12's componentwise minimum.
            pub fn min(&self, other: &LimitProfile) -> LimitProfile {
                LimitProfile { version: self.version.clone(), $($field: self.$field.min(other.$field),)* }
            }

            /// The ceiling of a field, by its RM.11 name.
            pub fn get(&self, field: &str) -> Option<u64> {
                match field {
                    $(stringify!($field) => Some(self.$field),)*
                    _ => None,
                }
            }
        }
    };
}

limits!(
    nodes,
    plans,
    instances,
    maps,
    map_entries,
    charts,
    stages,
    stage_ops,
    fragment_ops,
    overdraw,
    frame_work,
    view_vertex_evals,
    layout_vertices,
    layout_indices,
    overlay_items,
    input_bytes,
    source_bytes,
    canonical_output_bytes,
    adapter_cpu_bytes,
    adapter_gpu_bytes,
);

/// The RM.11 fields a publication is counted against here, in the table's order. The stage, fragment, frame, view
/// and adapter fields need user stages in publications, RM.38's weight table or a backend, which this crate does
/// not have.
pub const COUNTED: [&str; 13] = [
    "nodes",
    "plans",
    "instances",
    "maps",
    "map_entries",
    "charts",
    "stages",
    "layout_vertices",
    "layout_indices",
    "overlay_items",
    "input_bytes",
    "source_bytes",
    "canonical_output_bytes",
];

/// An admitted stage definition and version, with the number of parameters of its typed signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageDef {
    pub name: String,
    pub version: u32,
    pub definition: String,
    pub params: usize,
}

/// One feature row of a support report.
#[derive(Clone, Debug, PartialEq)]
pub enum Support {
    Supported { layout_version: u32, limits: LimitProfile, stages: Vec<StageDef> },
    Unsupported(Error),
}

/// RM.14's `SupportReport`. A row it omits is unsupported (RM.15).
#[derive(Clone, Debug, PartialEq)]
pub struct SupportReport {
    pub interface_version: u32,
    pub backend_build: String,
    pub device_epoch: u64,
    pub features: BTreeMap<String, Support>,
    pub evidence: Vec<String>,
}

impl SupportReport {
    /// The limits of a supported row, or its typed refusal.
    pub fn feature(&self, id: &str) -> Result<(&LimitProfile, &[StageDef]), Error> {
        match self.features.get(id) {
            Some(Support::Supported { limits, stages, .. }) => Ok((limits, stages)),
            Some(Support::Unsupported(error)) => Err(error.clone()),
            None => Err(Error::unsupported(id, format!("the backend reports no support for {id}"))),
        }
    }
}

/// The host token table, bound by its digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenTable {
    pub digest: String,
    pub names: BTreeSet<String>,
}

/// The record `word()` reads for a key: the EVD.32 word and its EVD.36 scope phrase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordRecord {
    pub word: String,
    pub scope: String,
}

/// What the host validates against.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub identity: IdentityBlock,
    /// The admitted dimension; only 4 in 1.0 (RPI.4).
    pub dimension: u32,
    /// The verified geometry digests of `GeometryInput`, by asset name.
    pub geometry: BTreeMap<String, String>,
    pub support: SupportReport,
    /// The host/profile ceilings.
    pub limits: LimitProfile,
    pub tokens: TokenTable,
    /// The records `word()` reads, by key.
    pub words: BTreeMap<Key, WordRecord>,
    /// The chart check results admission consumes (RPI.48); empty, every claimed property stays `not_established`.
    pub checks: Vec<ChartCheck>,
}

/// Who owns a plan's subject (RM.8, RM.9). A program cannot label its own arrays `session` or `preview`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubjectKind {
    Session,
    Derived,
    Preview,
}

/// The host-owned identification of one validated plan (RM.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanSubject {
    pub plan: PlanId,
    pub kind: SubjectKind,
    pub digest: String,
    /// The display comparison base: the state revision of the plan's base.
    pub comparison_base: u64,
    pub lineage: Option<Lineage>,
}

/// Content as the host shows it (RPI.65).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    /// State data, supported by the revision and row that produced it.
    State { row: String, digest: String },
    /// A word computed by `word()` on a key with a record, with its scope phrase.
    Word { key: Key, word: String, scope: String },
    /// The program's own text, shown as the output of its producing module and carrying no word.
    Program(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownNode {
    pub node: NodeId,
    pub kind: NodeKind,
    pub content: Vec<Shown>,
}

pub use crate::chart::PropertyStatus;

/// A validated publication.
#[derive(Clone, Debug, PartialEq)]
pub struct Validated {
    /// The digest that identifies the canonical publication (RM.20's ResultRef).
    pub digest: String,
    /// The canonical uncompressed bytes charged under RM.13.
    pub output_bytes: u64,
    /// The counts of RM.11's counted fields.
    pub counts: BTreeMap<&'static str, u64>,
    pub subjects: Vec<PlanSubject>,
    pub nodes: Vec<ShownNode>,
    /// The union of the plans' declared dependencies.
    pub dependencies: BTreeMap<String, String>,
    /// Every claimed chart property with its status (RPI.34, RPI.48).
    pub properties: Vec<(ChartId, String, PropertyStatus)>,
    /// Every chart's admission and the coverage of its output (RPI.48).
    pub charts: Vec<ChartAdmission>,
}

/// Whether a certified transform may be drawn as swept motion: only with the engine's named plane (RM.6, F03).
pub fn swept_preview(transform: &TransformInput) -> bool {
    transform.plane.is_some()
}

/// The feature row that admits a primitive for a puzzle.
pub fn feature(primitive: &Primitive, puzzle: Puzzle) -> &'static str {
    match (primitive, puzzle) {
        (Primitive::ChartStage(_), Puzzle::Jumbling) => "J05",
        (Primitive::Transform(_), Puzzle::Jumbling) => "J03",
        _ => primitive.feature(),
    }
}

fn chart_feature(puzzle: Puzzle) -> &'static str {
    if puzzle == Puzzle::Jumbling {
        "J05"
    } else {
        "F04"
    }
}

fn invalid(summary: impl Into<String>) -> Error {
    Error::invalid(summary)
}

fn finite(array: &Array, what: &str) -> Result<(), Error> {
    if array.as_f32().iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(invalid(format!("{what}: a coordinate is not finite")))
    }
}

/// Checks an index array u32 [e, width] against `bound`. Returns the number of index entries.
fn indices(array: &Array, what: &str, width: u64, bound: u64) -> Result<u64, Error> {
    let entries = array.expect(what, Elem::U32, &[None, Some(width)])?;
    if array.as_u32().iter().any(|&i| u64::from(i) >= bound) {
        return Err(invalid(format!("{what}: an index is out of range")));
    }
    Ok(entries)
}

/// Checks positions f32 [n, k] with k = 2 or 3, all finite. Returns n.
fn positions(array: &Array, what: &str) -> Result<u64, Error> {
    array.expect(what, Elem::F32, &[None, None])?;
    let k = array.desc.shape[1];
    if k != 2 && k != 3 {
        return Err(invalid(format!("{what}: positions have {k} coordinates, not 2 or 3")));
    }
    finite(array, what)?;
    Ok(array.rows())
}

fn object_in_range(object: &ObjectId) -> Result<(), Error> {
    let (value, bound, what) = match object {
        ObjectId::Sticker(id) => (*id, SLOTS as u32, "sticker"),
        ObjectId::Slot(id) => (*id, SLOTS as u32, "slot"),
        ObjectId::Piece(id) => (*id, PIECES as u32, "piece"),
        ObjectId::Cell(id) | ObjectId::Grip(id) => (*id, CELLS, "cell or grip"),
        ObjectId::Point { witness, grip, role } => {
            if *grip >= CELLS || role.is_empty() {
                return Err(invalid("a certificate point needs a grip and a source role"));
            }
            (*witness, PIECES as u32, "witness piece")
        }
        ObjectId::Cap(_) | ObjectId::Orbit(_) => return Ok(()),
    };
    if value < bound {
        Ok(())
    } else {
        Err(invalid(format!("{what} {value} is out of range")))
    }
}

fn lineage(lineage: Option<&Lineage>, identity: &IdentityBlock) -> Result<(), Error> {
    match lineage {
        None => Err(invalid("a derived subject is displayed only with its derivation or imported lineage (RPI.7)")),
        Some(Lineage::Derivation { base: None, .. }) => Err(invalid("the derivation names no base (RPI.7)")),
        Some(Lineage::Derivation { base: Some(base), word }) => {
            if word.is_empty() {
                Err(invalid("the derivation names no literal word digest"))
            } else if base.identity() != identity {
                Err(invalid("the derivation's base names another model"))
            } else {
                Ok(())
            }
        }
        Some(Lineage::Imported { comparison_base: None, .. }) => {
            Err(invalid("an imported value needs its explicit comparison base (EV3.12)"))
        }
        Some(Lineage::Imported { import, .. }) if import.is_empty() => {
            Err(invalid("an imported value names no import"))
        }
        Some(Lineage::Imported { .. }) => Ok(()),
    }
}

struct Check<'a> {
    ctx: &'a Context,
    publication: &'a Publication,
    invalid: Vec<Error>,
    unsupported: Vec<Error>,
    features: BTreeSet<&'static str>,
    shown: Vec<ShownNode>,
    subjects: Vec<PlanSubject>,
    properties: Vec<(ChartId, String, PropertyStatus)>,
    charts: Vec<ChartAdmission>,
    vertices: u64,
    index_entries: u64,
    overlay_items: u64,
}

impl Check<'_> {
    fn note(&mut self, result: Result<(), Error>) {
        if let Err(error) = result {
            if error.code == codes::UNSUPPORTED {
                self.unsupported.push(error);
            } else {
                self.invalid.push(error);
            }
        }
    }

    fn tokens(&self, names: &[String]) -> Result<(), Error> {
        match names.iter().find(|name| !self.ctx.tokens.names.contains(*name)) {
            Some(name) => Err(invalid(format!("token name {name} does not resolve in the host token table"))),
            None => Ok(()),
        }
    }

    fn nodes(&mut self) {
        let publication = self.publication;
        let ids: BTreeSet<NodeId> = publication.nodes.iter().map(|n| n.node).collect();
        if ids.len() != publication.nodes.len() {
            self.note(Err(invalid("duplicate node IDs")));
        }
        let plans: BTreeSet<PlanId> = publication.plans.iter().map(|p| p.plan).collect();
        for node in &publication.nodes {
            let result = self.node(node, &ids, &plans);
            self.note(result);
        }
    }

    fn node(&mut self, node: &ViewNode, ids: &BTreeSet<NodeId>, plans: &BTreeSet<PlanId>) -> Result<(), Error> {
        if node.base != self.publication.base {
            return Err(invalid(format!("node {} reads another revision than the publication", node.node.0)));
        }
        if let Some(plan) = node.plans.iter().find(|p| !plans.contains(p)) {
            return Err(invalid(format!(
                "node {} names plan {}, which is not in the publication",
                node.node.0, plan.0
            )));
        }
        if node.children.iter().any(|c| !ids.contains(c) || *c == node.node) {
            return Err(invalid(format!("node {} has a dangling or self child", node.node.0)));
        }
        if let Some(link) = &node.source {
            source::admit(link, &self.publication.sources)?;
        }
        if node.kind == NodeKind::Relation && node.claims.is_empty() {
            return Err(invalid(format!("relation node {} states no claim key (RPI.65)", node.node.0)));
        }
        let mut content = Vec::new();
        for item in &node.content {
            content.push(match item {
                Content::State { row, digest } => Shown::State { row: row.clone(), digest: digest.clone() },
                Content::Text(text) => Shown::Program(text.clone()),
                Content::Word(key) => {
                    if !node.claims.contains(key) {
                        return Err(invalid(format!("a word on key {} needs that key in the node's claims", key.0)));
                    }
                    let record = self.ctx.words.get(key).ok_or_else(|| {
                        invalid(format!("word() is asked on key {}, which has no record (RPI.65)", key.0))
                    })?;
                    Shown::Word { key: key.clone(), word: record.word.clone(), scope: record.scope.clone() }
                }
            });
        }
        self.shown.push(ShownNode { node: node.node, kind: node.kind, content });
        Ok(())
    }

    fn plans(&mut self) {
        let publication = self.publication;
        let ids: BTreeSet<PlanId> = publication.plans.iter().map(|p| p.plan).collect();
        if ids.len() != publication.plans.len() {
            self.note(Err(invalid("duplicate plan IDs")));
        }
        let maps: BTreeSet<MapId> = publication.maps.iter().map(|m| m.map).collect();
        if maps.len() != publication.maps.len() {
            self.note(Err(invalid("duplicate source map IDs")));
        }
        let mut instances = BTreeSet::new();
        for plan in &publication.plans {
            for instance in &plan.instances {
                if !instances.insert(instance.instance) {
                    self.note(Err(invalid(format!("instance ID {} is used twice", instance.instance.0))));
                }
            }
        }
        for map in &publication.maps {
            if !ids.contains(&map.plan) {
                self.note(Err(invalid(format!("source map {} names a plan outside the publication", map.map.0))));
            }
        }
        for (digest, bytes) in &publication.sources {
            if c600_digest::sha256_hex(bytes) != *digest {
                self.note(Err(invalid("pinned source bytes do not match their digest")));
            }
        }
        for plan in &publication.plans {
            let result = self.plan(plan);
            self.note(result);
        }
    }

    fn plan(&mut self, plan: &DisplayPlan) -> Result<(), Error> {
        let ctx = self.ctx;
        if plan.base != self.publication.base {
            return Err(invalid(format!("plan {} reads another revision than the publication", plan.plan.0)));
        }
        if plan.geometry.identity != ctx.identity || plan.base.identity != ctx.identity {
            return Err(invalid(format!("plan {} is bound to another model identity (RPI.8)", plan.plan.0)));
        }
        if plan.geometry.digests != ctx.geometry {
            return Err(invalid(format!("plan {}'s geometry digests do not match the verified assets", plan.plan.0)));
        }
        if plan.precision.is_empty() {
            return Err(invalid(format!("plan {} declares no display precision", plan.plan.0)));
        }
        let kind = match &plan.subject {
            StateRef::Snapshot(revision) => {
                if *revision != plan.base || plan.derivation.is_some() {
                    return Err(invalid("a session subject is the plan's base revision, with no derivation"));
                }
                SubjectKind::Session
            }
            StateRef::Derived(derived) => {
                if derived.identity != ctx.identity {
                    return Err(invalid("the derived subject names another model"));
                }
                lineage(plan.derivation.as_ref(), &ctx.identity)?;
                SubjectKind::Derived
            }
        };
        let map = self.publication.maps.iter().find(|m| m.map == plan.source_map).ok_or_else(|| {
            invalid(format!("plan {} names source map {}, which is missing", plan.plan.0, plan.source_map.0))
        })?;
        if map.plan != plan.plan || map.base != plan.base {
            return Err(invalid(format!("source map {} is bound to another plan or revision", map.map.0)));
        }
        if plan.dimension != ctx.dimension {
            return Err(Error::unsupported(
                "dimension",
                format!(
                    "dimension {} is not supported; only d = {} is accepted (RPI.4)",
                    plan.dimension, ctx.dimension
                ),
            ));
        }
        self.charts(plan)?;
        self.instances(plan, map)?;
        for primitive in &plan.primitives {
            let result = self.primitive(primitive, plan, &plan.subject);
            self.note(result);
        }
        self.subjects.push(PlanSubject {
            plan: plan.plan,
            kind,
            digest: plan.subject.digest().to_owned(),
            comparison_base: plan.base.state,
            lineage: plan.derivation.clone(),
        });
        Ok(())
    }

    fn charts(&mut self, plan: &DisplayPlan) -> Result<(), Error> {
        let ids: BTreeSet<ChartId> = plan.charts.iter().map(|c| c.chart).collect();
        if ids.len() != plan.charts.len() {
            return Err(invalid("duplicate chart IDs"));
        }
        let feature = chart_feature(self.ctx.identity.puzzle);
        for chart in &plan.charts {
            if chart.dimension != plan.dimension || chart.precision.is_empty() || chart.stages.is_empty() {
                return Err(invalid(format!(
                    "chart {} has another dimension, no precision or no stages",
                    chart.chart.0
                )));
            }
            for stage in &chart.stages {
                if stage.params.iter().any(|p| !p.is_finite()) {
                    return Err(invalid(format!("stage {} has a non-finite parameter", stage.name)));
                }
            }
            let (admission, statuses) = chart::admit_chart(chart, &self.ctx.checks);
            for (property, status) in statuses {
                self.properties.push((chart.chart, property, status));
            }
            let coverage = match &admission {
                Admission::Refused(error) => Coverage::Undefined(error.code.to_owned()),
                Admission::Admitted | Admission::NotEstablished(_) => Coverage::Complete,
            };
            self.charts.push(ChartAdmission { plan: plan.plan, chart: chart.chart, admission, coverage });
            self.features.insert(feature);
            let admitted = match self.ctx.support.feature(feature) {
                Ok((_, stages)) => stages,
                Err(error) => {
                    self.note(Err(error));
                    continue;
                }
            };
            for stage in &chart.stages {
                let def = admitted
                    .iter()
                    .find(|d| d.name == stage.name && d.version == stage.version && d.definition == stage.definition);
                match def {
                    None => self.note(Err(Error::unsupported(
                        feature,
                        format!(
                            "stage {} version {} is not an admitted definition; no substitute",
                            stage.name, stage.version
                        ),
                    ))),
                    Some(def) if def.params != stage.params.len() => {
                        return Err(invalid(format!(
                            "stage {} takes {} parameters, not {}",
                            stage.name,
                            def.params,
                            stage.params.len()
                        )))
                    }
                    Some(_) => {}
                }
            }
        }
        Ok(())
    }

    fn instances(&mut self, plan: &DisplayPlan, map: &SourceMap) -> Result<(), Error> {
        let ids: BTreeSet<InstanceId> = plan.instances.iter().map(|i| i.instance).collect();
        if let Some(extra) = map.entries.keys().find(|id| !ids.contains(id)) {
            return Err(invalid(format!("source map entry {} names no instance of the plan", extra.0)));
        }
        for instance in &plan.instances {
            if instance.plan != plan.plan || instance.primitive as usize >= plan.primitives.len() {
                return Err(invalid(format!(
                    "instance {} names another plan or a missing primitive",
                    instance.instance.0
                )));
            }
            object_in_range(&instance.object)?;
            if instance.piece.is_some_and(|p| p as usize >= PIECES) {
                return Err(invalid(format!("instance {}'s piece association is out of range", instance.instance.0)));
            }
            let entry = map.entries.get(&instance.instance).ok_or_else(|| {
                invalid(format!("instance {} has no entry in source map {}", instance.instance.0, map.map.0))
            })?;
            if entry.object != instance.object {
                return Err(invalid(format!("instance {}'s map entry names another object", instance.instance.0)));
            }
            if !map.sources.iter().any(|d| d == entry.source.digest()) {
                return Err(invalid(format!(
                    "instance {}'s source is not a producing source of the map",
                    instance.instance.0
                )));
            }
            source::admit(&entry.source, &self.publication.sources)?;
        }
        Ok(())
    }

    fn primitive(&mut self, primitive: &Primitive, plan: &DisplayPlan, subject: &StateRef) -> Result<(), Error> {
        let ctx = self.ctx;
        let feature = feature(primitive, ctx.identity.puzzle);
        self.features.insert(feature);
        let supported = ctx.support.feature(feature);
        match primitive {
            Primitive::Retained(retained) => {
                retained.labels.expect("labels", Elem::U32, &[Some(SLOTS as u64)])?;
                for (array, what) in [(&retained.visibility, "visibility"), (&retained.interaction, "interaction")] {
                    array.expect(what, Elem::U8, &[Some(SLOTS as u64)])?;
                    if array.bytes.iter().any(|&v| v > 1) {
                        return Err(invalid(format!("{what} holds a value other than 0 or 1")));
                    }
                }
                if retained.labels.desc.digest != subject.digest() {
                    return Err(invalid("the labels do not bind the plan's subject (RPI.8)"));
                }
                if retained.labels.as_u32().iter().any(|&l| l as usize >= SLOTS) {
                    return Err(invalid("a label is out of range"));
                }
                self.tokens(&retained.tokens)?;
            }
            Primitive::Posed(posed) => {
                if ctx.identity.puzzle == Puzzle::Full {
                    return Err(Error::unsupported(
                        feature,
                        "posed sticker objects are jumbling rows; their producer waits for M5",
                    ));
                }
                self.posed(posed, plan, subject)?;
            }
            Primitive::Transform(transform) => {
                if transform.grip >= CELLS || !transform.angle.is_finite() || transform.to <= transform.from {
                    return Err(invalid("a transform needs a grip, a finite angle and increasing committed endpoints"));
                }
                if let Some(plane) = &transform.plane {
                    plane.expect("plane", Elem::F32, &[Some(2), Some(u64::from(plan.dimension))])?;
                    finite(plane, "plane")?;
                }
                transform.moving.expect("moving", Elem::U32, &[None])?;
                let moving = transform.moving.as_u32();
                if moving.windows(2).any(|w| w[0] >= w[1]) || moving.iter().any(|&p| p as usize >= PIECES) {
                    return Err(invalid("the moving pieces are not sorted, distinct piece IDs"));
                }
                if piece_set_digest(&moving) != transform.moving_digest {
                    return Err(invalid("the moving set does not match its exact-set digest"));
                }
            }
            Primitive::ChartStage(stage) => {
                if !plan.charts.iter().any(|c| c.chart == stage.chart) {
                    return Err(invalid(format!("chart {} is not in the plan", stage.chart.0)));
                }
            }
            Primitive::UserStage(_) => {
                return Err(Error::unsupported(
                    feature,
                    "user stages need a backend that compiles and bounds them (RPI.72, RPI.78), not built",
                ))
            }
            Primitive::Layout(layout) => self.layout(layout, plan)?,
            Primitive::CertOverlay(cert) => {
                if ctx.identity.puzzle == Puzzle::Full {
                    return Err(Error::unsupported(feature, "certified geometric overlays are jumbling rows"));
                }
                self.cert(cert)?;
            }
            Primitive::SetOverlay(overlay) => self.overlay(overlay)?,
            Primitive::Tokens(tokens) => {
                if tokens.table != ctx.tokens.digest {
                    return Err(invalid("the token names are bound to another token table"));
                }
                let names: Vec<String> = tokens.names.iter().map(|(_, name)| name.clone()).collect();
                self.tokens(&names)?;
            }
            Primitive::Derived(derived) => {
                if derived.subject.identity != ctx.identity {
                    return Err(invalid("the derived subject names another model"));
                }
                lineage(derived.lineage.as_ref(), &ctx.identity)?;
                let inner = StateRef::Derived(derived.subject.clone());
                for primitive in &derived.primitives {
                    self.primitive(primitive, plan, &inner)?;
                }
            }
            Primitive::Preview(_) => {
                return Err(invalid("a program cannot supply or label preview data (RM.8, RM.9)"));
            }
        }
        supported.map(|_| ())
    }

    fn layout(&mut self, layout: &LayoutInput, plan: &DisplayPlan) -> Result<(), Error> {
        let n = positions(&layout.positions, "layout positions")?;
        if n != layout.instances.len() as u64 {
            return Err(invalid("the layout has another number of positions than instances"));
        }
        let distinct: BTreeSet<InstanceId> = layout.instances.iter().copied().collect();
        if distinct.len() != layout.instances.len() {
            return Err(invalid("a layout repeats an instance; repeated objects need distinct instances"));
        }
        if layout.instances.iter().any(|id| !plan.instances.iter().any(|i| i.instance == *id)) {
            return Err(invalid("a layout names an instance outside its plan"));
        }
        self.vertices += n;
        if let Some(edges) = &layout.edges {
            self.index_entries += indices(edges, "layout edges", 2, n)?;
        }
        if let Some(triangles) = &layout.triangles {
            self.index_entries += indices(triangles, "layout triangles", 3, n)?;
        }
        match &layout.kind {
            LayoutKind::Sheet => {}
            LayoutKind::Atlas { membership } => {
                if membership.len() as u64 != n || membership.iter().any(|c| !plan.charts.iter().any(|d| d.chart == *c))
                {
                    return Err(invalid("an atlas names the chart of every occurrence, from the plan's charts"));
                }
            }
            LayoutKind::Tree { roots, parent_child } => {
                if roots.is_empty() || roots.iter().any(|&r| u64::from(r) >= n) {
                    return Err(invalid("a tree needs root indices inside its layout"));
                }
                self.index_entries += indices(parent_child, "tree edges", 2, n)?;
            }
            LayoutKind::Complex(incidence) => {
                incidence.dims.expect("cell dimensions", Elem::U32, &[Some(n)])?;
                incidence.offsets.expect("boundary offsets", Elem::U32, &[Some(n + 1)])?;
                let b = incidence.boundary.expect("boundary indices", Elem::U32, &[None])?;
                incidence.orientation.expect("orientation", Elem::I32, &[Some(b)])?;
                let dims = incidence.dims.as_u32();
                let offsets = incidence.offsets.as_u32();
                let boundary = incidence.boundary.as_u32();
                if offsets[0] != 0 || offsets.windows(2).any(|w| w[0] > w[1]) || u64::from(offsets[n as usize]) != b {
                    return Err(invalid("boundary offsets must start at 0, not decrease and end at b"));
                }
                if incidence.orientation.as_i32().iter().any(|&o| o != 1 && o != -1) {
                    return Err(invalid("an orientation is not -1 or 1"));
                }
                for cell in 0..n as usize {
                    for &face in &boundary[offsets[cell] as usize..offsets[cell + 1] as usize] {
                        if face as usize >= dims.len() || dims[face as usize] >= dims[cell] {
                            return Err(invalid("a boundary reference does not name a lower-dimensional cell"));
                        }
                    }
                }
                self.index_entries += b;
                if dims.iter().any(|&d| d > plan.dimension) {
                    return Err(Error::unsupported("F09", "a cell dimension beyond the admitted d"));
                }
            }
            LayoutKind::Graph(graph) => {
                if graph.subjects.len() as u64 != n || graph.subjects.iter().any(|s| *s.identity() != self.ctx.identity)
                {
                    return Err(invalid("every graph node keeps its own subject of this model"));
                }
                let e = indices(&graph.edges, "graph edges", 2, n)?;
                if graph.transitions.len() as u64 * 2 != e {
                    return Err(invalid("every directed edge carries one transition reference"));
                }
                self.index_entries += e;
            }
        }
        Ok(())
    }

    /// The posed-sticker row of a jumbling plan (J02, render/1 section 3): one pose index per piece, a pose table of
    /// finite row-major matrices with the identity at pose 0, and one lattice entry per pose. The subject is at pose
    /// level; its exact digest cannot be recomputed from binary32 arrays, so the producer binds it.
    fn posed(&mut self, posed: &PosedInput, plan: &DisplayPlan, subject: &StateRef) -> Result<(), Error> {
        if subject.level() != Level::Poses {
            return Err(invalid("a posed row draws a subject at pose level"));
        }
        let d = u64::from(plan.dimension);
        posed.pose_index.expect("pose_index", Elem::I32, &[Some(PIECES as u64)])?;
        let p = posed.pose_lattice.expect("pose_lattice", Elem::I32, &[None])?;
        posed.poses.expect("poses", Elem::F32, &[Some(p), Some(d), Some(d)])?;
        if p == 0 {
            return Err(invalid("the pose table is empty; pose 0 is the identity"));
        }
        finite(&posed.poses, "poses")?;
        if posed.pose_index.as_i32().iter().any(|&i| i < 0 || i as u64 >= p) {
            return Err(invalid("a pose index is outside the pose table"));
        }
        let lattice = posed.pose_lattice.as_i32();
        if lattice.iter().any(|&k| !(-1..K_PLUS).contains(&k)) || lattice[0] != 0 {
            return Err(invalid("a lattice entry is not -1 or a rotation-group index, or pose 0 is not index 0"));
        }
        let first = &posed.poses.as_f32()[..(d * d) as usize];
        let identity =
            first.iter().enumerate().all(|(i, &x)| x == if (i as u64).is_multiple_of(d + 1) { 1.0 } else { 0.0 });
        if !identity {
            return Err(invalid("pose 0 is not the identity"));
        }
        Ok(())
    }

    fn cert(&mut self, cert: &CertOverlay) -> Result<(), Error> {
        if cert.grip >= CELLS || cert.pieces.windows(2).any(|w| w[0] >= w[1]) {
            return Err(invalid("a certificate overlay needs a grip and sorted, distinct pieces"));
        }
        if cert.pieces.iter().any(|&p| p as usize >= PIECES) || piece_set_digest(&cert.pieces) != cert.set_digest {
            return Err(invalid("the certificate pieces do not match their exact-set digest; no truncation"));
        }
        if cert.kind == CertKind::CapMapWitness && cert.pieces.len() != 1 {
            return Err(invalid("a cap-map witness names exactly one piece"));
        }
        for pair in &cert.pairs {
            let points_ok = pair.points.iter().all(|p| !p.exact.is_empty() && p.vector.iter().all(|x| x.is_finite()));
            if !cert.pieces.contains(&pair.piece) || !points_ok {
                return Err(invalid("a point pair names a piece outside the set or an unchecked point"));
            }
        }
        if !cert.survey.is_empty() && cert.survey.len() != CELLS as usize {
            return Err(invalid("a grip survey covers all 600 grips"));
        }
        if let Some(cut) = &cert.cut {
            if cut.plane.is_empty() || cut.alpha.1 <= 0 {
                return Err(invalid("a named cut carries its exact plane and alpha"));
            }
        }
        self.overlay_items += cert.pieces.len() as u64 + 2 * cert.pairs.len() as u64;
        Ok(())
    }

    fn overlay(&mut self, overlay: &OverlayInput) -> Result<(), Error> {
        if overlay.objects.windows(2).any(|w| w[0] >= w[1]) {
            return Err(invalid("overlay objects are not in canonical order without repeats"));
        }
        for object in &overlay.objects {
            object_in_range(object)?;
        }
        if object_set_digest(&overlay.objects) != overlay.set_digest {
            return Err(invalid("the overlay objects do not match their exact-set digest"));
        }
        if overlay.kind == (OverlayKind::Relation { claim: None }) {
            return Err(invalid("a relation overlay carries its claim key (RPI.65)"));
        }
        if matches!(overlay.coverage, Coverage::Undefined(_)) && !overlay.objects.is_empty() {
            return Err(invalid("an undefined result carries no objects"));
        }
        if let Some(Predicate::Section(d) | Predicate::Slab(d) | Predicate::Clip(d)) = &overlay.predicate {
            if d.is_empty() {
                return Err(invalid("a section, slab or clip names its exact predicate"));
            }
        }
        let v = match &overlay.positions {
            Some(p) => positions(p, "overlay positions")?,
            None => 0,
        };
        self.vertices += v;
        if let Some(lines) = &overlay.lines {
            self.index_entries += indices(lines, "overlay lines", 2, v)?;
        }
        if let Some(triangles) = &overlay.triangles {
            self.index_entries += indices(triangles, "overlay triangles", 3, v)?;
        }
        self.overlay_items += overlay.objects.len() as u64;
        self.tokens(&overlay.tokens)
    }
}

/// Validates a program publication against the host context.
pub fn validate(publication: &Publication, ctx: &Context) -> Result<Validated, Error> {
    let mut check = Check {
        ctx,
        publication,
        invalid: Vec::new(),
        unsupported: Vec::new(),
        features: BTreeSet::new(),
        shown: Vec::new(),
        subjects: Vec::new(),
        properties: Vec::new(),
        charts: Vec::new(),
        vertices: 0,
        index_entries: 0,
        overlay_items: 0,
    };
    if publication.base.identity != ctx.identity {
        return Err(invalid("the publication's base names another model identity"));
    }
    check.nodes();
    check.plans();
    if let Some(error) = check.invalid.into_iter().next() {
        return Err(error);
    }
    check.features.insert("F20");
    let mut ceiling = ctx.limits.clone();
    for feature in &check.features {
        match ctx.support.feature(feature) {
            Ok((limits, _)) => ceiling = ceiling.min(limits),
            Err(error) => check.unsupported.push(error),
        }
    }
    if let Some(error) = check.unsupported.into_iter().next() {
        return Err(error);
    }
    let (output_bytes, digest) = canonical::measure(publication);
    let plans = &publication.plans;
    let counts: BTreeMap<&'static str, u64> = [
        ("nodes", publication.nodes.len() as u64),
        ("plans", plans.len() as u64),
        ("instances", plans.iter().map(|p| p.instances.len() as u64).sum()),
        ("maps", publication.maps.len() as u64),
        ("map_entries", publication.maps.iter().map(|m| m.entries.len() as u64).sum()),
        ("charts", plans.iter().map(|p| p.charts.len() as u64).sum()),
        ("stages", plans.iter().flat_map(|p| &p.charts).map(|c| c.stages.len() as u64).sum()),
        ("layout_vertices", check.vertices),
        ("layout_indices", check.index_entries),
        ("overlay_items", check.overlay_items),
        ("input_bytes", canonical::input_bytes(publication)),
        ("source_bytes", publication.sources.values().map(|b| b.len() as u64).sum()),
        ("canonical_output_bytes", output_bytes),
    ]
    .into_iter()
    .collect();
    for field in COUNTED {
        let (requested, limit) = (counts[field], ceiling.get(field).unwrap_or(0));
        if requested > limit {
            return Err(Error::ceiling(field, requested, limit));
        }
    }
    let mut dependencies = BTreeMap::new();
    for plan in plans {
        dependencies.extend(plan.dependencies.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    Ok(Validated {
        digest,
        output_bytes,
        counts,
        subjects: check.subjects,
        nodes: check.shown,
        dependencies,
        properties: check.properties,
        charts: check.charts,
    })
}

/// One array of an engine bundle with the state revision it belongs to.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundArray {
    pub name: String,
    pub revision: u64,
    pub array: Array,
}

/// A revision-bound engine bundle (RPI.9).
#[derive(Clone, Debug, PartialEq)]
pub struct EngineBundle {
    pub revision: RevisionRef,
    pub arrays: Vec<BoundArray>,
}

/// The arrays of a complete bundle: Full's labels and sets, or jumbling's three pose arrays (render/1 section 7).
pub fn bundle_names(puzzle: Puzzle) -> [&'static str; 3] {
    match puzzle {
        Puzzle::Full => ["labels", "visibility", "interaction"],
        Puzzle::Jumbling => ["pose_index", "poses", "pose_lattice"],
    }
}

/// RPI.9's `validate_engine_bundle`: every array once, well described and bound to the same revision; anything
/// else is `refresh_required(EV3:input.display_bundle_mismatch)`, and no mixed bundle is adopted.
pub fn validate_engine_bundle(bundle: &EngineBundle) -> Result<(), Error> {
    let mismatch = |summary: String| Err(Error::new(codes::BUNDLE_MISMATCH, summary));
    for name in bundle_names(bundle.revision.identity.puzzle) {
        let found: Vec<&BoundArray> = bundle.arrays.iter().filter(|a| a.name == name).collect();
        let [array] = found.as_slice() else {
            return mismatch(format!("the bundle needs exactly one {name} array"));
        };
        if array.revision != bundle.revision.state {
            return mismatch(format!("{name} belongs to revision {}, not {}", array.revision, bundle.revision.state));
        }
        if array.array.check(name).is_err() {
            return mismatch(format!("{name} is not described by its descriptor"));
        }
    }
    if bundle.revision.identity.puzzle == Puzzle::Full {
        let labels = bundle.arrays.iter().find(|a| a.name == "labels").map(|a| &a.array.desc.digest);
        if labels != Some(&bundle.revision.digest) {
            return mismatch("the labels are not the revision's state".to_owned());
        }
    }
    Ok(())
}

/// An engine bundle that [`validate_engine_bundle`] accepted. Nothing can change it after the check (it gives no
/// mutable access, and its arrays' bytes are shared read-only), so a host may adopt it without checking it again
/// (`Host::adopt_checked`). A host that builds bundles off its frame thread checks them there (C9, 26 Nov 2026).
#[derive(Clone, Debug)]
pub struct CheckedBundle(EngineBundle);

impl CheckedBundle {
    /// The bundle, once [`validate_engine_bundle`] accepts it; its refusal otherwise.
    pub fn check(bundle: EngineBundle) -> Result<CheckedBundle, Error> {
        validate_engine_bundle(&bundle)?;
        Ok(CheckedBundle(bundle))
    }

    pub fn bundle(&self) -> &EngineBundle {
        &self.0
    }
}

/// A publication that [`validate`] accepted against a context, with the context and the result. Nothing can change
/// either after the check, so `Host::publish_checked` takes the result in place of validating again while the host's
/// context equals this one.
#[derive(Clone, Debug)]
pub struct CheckedPublication {
    pub(crate) publication: Publication,
    pub(crate) ctx: Context,
    pub(crate) validated: Validated,
}

impl CheckedPublication {
    /// The publication, once [`validate`] accepts it against `ctx`; its refusal otherwise.
    pub fn check(publication: Publication, ctx: &Context) -> Result<CheckedPublication, Error> {
        let validated = validate(&publication, ctx)?;
        Ok(CheckedPublication { publication, ctx: ctx.clone(), validated })
    }

    pub fn publication(&self) -> &Publication {
        &self.publication
    }
}

/// The engine path of a preview display (RM.9): the host takes it from the engine's preview object.
pub fn validate_preview(preview: &PreviewDisplay, ctx: &Context) -> Result<(), Error> {
    if preview.base.identity != ctx.identity || preview.literal.is_empty() || preview.effect.is_empty() {
        return Err(invalid("a preview names its base, literal digest and certified effect of this model"));
    }
    if preview.primitives.iter().any(|p| matches!(p, Primitive::Preview(_))) {
        return Err(invalid("a preview does not nest another preview"));
    }
    Ok(())
}
