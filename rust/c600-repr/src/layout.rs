//! Layouts and atlases, layout filters and linked selection (LB4.4), as values of chapter 10's display plans.
//!
//! A [`Representation`] is one display plan with its source map: a sheet or an atlas of occurrences, each a display
//! instance of a canonical object, beside the exact set the representation was analysed for. `c600-display` validates
//! and installs it and resolves a pick by instance ID through the installed source map (RPI.22); nothing here resolves
//! a pick, and no position or chart value is ever read back into an object.

use std::collections::BTreeMap;

use c600_digest::sha256_hex;
use c600_display::array::Array;
use c600_display::plan::{
    object_set_digest, object_text, Chart, ChartId, Coverage, DisplayInstance, DisplayPlan, GeometryBinding,
    InstanceId, LayoutInput, LayoutKind, MapEntry, MapId, ObjectId, OverlayInput, OverlayKind, PlanId, Primitive,
    RevisionRef, SourceLink, SourceMap, StateRef,
};

/// One occurrence in a layout: a display instance of a canonical object at a sheet position. Several occurrences may
/// show one object; each keeps its own instance ID.
#[derive(Clone, Debug, PartialEq)]
pub struct Occurrence {
    pub instance: InstanceId,
    pub object: ObjectId,
    /// The chart of the occurrence in an atlas; `None` on a sheet.
    pub chart: Option<ChartId>,
    pub position: [f32; 2],
    /// The piece a sticker occurrence is shown as part of, when the layout shows it so.
    pub piece: Option<u32>,
    pub source: SourceLink,
}

/// What every plan of a representation states: its ID, its base revision, its geometry binding, the producing sources
/// of its map and the token name of its set overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub plan: PlanId,
    pub base: RevisionRef,
    pub geometry: GeometryBinding,
    pub sources: Vec<String>,
    pub token: String,
}

/// Why a layout cannot be built or filtered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// Some occurrences name a chart and others do not: a layout is a sheet or an atlas.
    MixedCharts,
    /// Two occurrences share an instance ID.
    RepeatedInstance,
    /// The plan has no sheet or atlas layout at primitive 0, or the layout carries edges or triangles: this draft
    /// filters sheets and atlases only.
    NotFilterable,
}

/// A display plan and its source map.
#[derive(Clone, Debug, PartialEq)]
pub struct Representation {
    pub plan: DisplayPlan,
    pub map: SourceMap,
}

/// The index of the layout primitive in a representation's plan.
pub const LAYOUT: u32 = 0;
/// The index of the analysed-set overlay in a representation's plan.
pub const ANALYSED: usize = 1;

fn layout_of(occurrences: &[Occurrence]) -> Result<LayoutInput, LayoutError> {
    let charts: Vec<ChartId> = occurrences.iter().filter_map(|o| o.chart).collect();
    let kind = if charts.is_empty() {
        LayoutKind::Sheet
    } else if charts.len() == occurrences.len() {
        LayoutKind::Atlas { membership: charts }
    } else {
        return Err(LayoutError::MixedCharts);
    };
    let positions: Vec<f32> = occurrences.iter().flat_map(|o| o.position).collect();
    Ok(LayoutInput {
        instances: occurrences.iter().map(|o| o.instance).collect(),
        positions: Array::f32s(vec![occurrences.len() as u64, 2], &positions),
        edges: None,
        triangles: None,
        kind,
    })
}

impl Representation {
    /// A sheet or atlas of `occurrences` at the header's base revision, beside the exact set `analysed`: the objects
    /// the representation was mechanically analysed for, which a later filter of the layout does not touch. `charts`
    /// are the plan's charts; an atlas names one of them for every occurrence.
    pub fn layout(
        header: &Header,
        analysed: &[ObjectId],
        charts: Vec<Chart>,
        occurrences: &[Occurrence],
    ) -> Result<Representation, LayoutError> {
        let mut seen = std::collections::BTreeSet::new();
        if occurrences.iter().any(|o| !seen.insert(o.instance)) {
            return Err(LayoutError::RepeatedInstance);
        }
        let mut objects = analysed.to_vec();
        objects.sort();
        objects.dedup();
        let overlay = OverlayInput {
            kind: OverlayKind::Set,
            set_digest: object_set_digest(&objects),
            objects,
            coverage: Coverage::Complete,
            predicate: None,
            positions: None,
            lines: None,
            triangles: None,
            tokens: vec![header.token.clone()],
        };
        let instances = occurrences
            .iter()
            .map(|o| DisplayInstance {
                instance: o.instance,
                plan: header.plan,
                object: o.object.clone(),
                primitive: LAYOUT,
                piece: o.piece,
                interaction: true,
            })
            .collect();
        let entries = occurrences
            .iter()
            .map(|o| (o.instance, MapEntry { object: o.object.clone(), source: o.source.clone() }))
            .collect();
        let plan = DisplayPlan {
            plan: header.plan,
            base: header.base.clone(),
            subject: StateRef::Snapshot(header.base.clone()),
            derivation: None,
            reachability: None,
            dimension: 4,
            geometry: header.geometry.clone(),
            primitives: vec![Primitive::Layout(layout_of(occurrences)?), Primitive::SetOverlay(overlay)],
            charts,
            instances,
            source_map: MapId(header.plan.0),
            precision: "binary32".to_owned(),
            dependencies: BTreeMap::from([("state".to_owned(), header.base.state.to_string())]),
        };
        let map = SourceMap {
            map: MapId(header.plan.0),
            plan: header.plan,
            base: header.base.clone(),
            sources: header.sources.clone(),
            entries,
        };
        Ok(Representation { plan, map })
    }

    /// The exact set the representation was analysed for.
    pub fn analysed(&self) -> Option<&OverlayInput> {
        match self.plan.primitives.get(ANALYSED) {
            Some(Primitive::SetOverlay(overlay)) if overlay.kind == OverlayKind::Set => Some(overlay),
            _ => None,
        }
    }

    /// The representation's occurrences, read back from its layout, instances and source map.
    pub fn occurrences(&self) -> Result<Vec<Occurrence>, LayoutError> {
        let Some(Primitive::Layout(layout)) = self.plan.primitives.first() else {
            return Err(LayoutError::NotFilterable);
        };
        let membership = match &layout.kind {
            LayoutKind::Sheet => None,
            LayoutKind::Atlas { membership } => Some(membership),
            _ => return Err(LayoutError::NotFilterable),
        };
        if layout.edges.is_some() || layout.triangles.is_some() {
            return Err(LayoutError::NotFilterable);
        }
        let positions = layout.positions.as_f32();
        let mut out = Vec::with_capacity(layout.instances.len());
        for (i, id) in layout.instances.iter().enumerate() {
            let instance = self.plan.instances.iter().find(|x| x.instance == *id);
            let (Some(instance), Some(entry)) = (instance, self.map.entries.get(id)) else {
                return Err(LayoutError::NotFilterable);
            };
            let (Some(x), Some(y)) = (positions.get(2 * i), positions.get(2 * i + 1)) else {
                return Err(LayoutError::NotFilterable);
            };
            out.push(Occurrence {
                instance: *id,
                object: instance.object.clone(),
                chart: membership.and_then(|m| m.get(i).copied()),
                position: [*x, *y],
                piece: instance.piece,
                source: entry.source.clone(),
            });
        }
        Ok(out)
    }

    /// A layout filter (LB4.4): the same representation showing only the occurrences `keep` accepts. It rebuilds the
    /// layout, the instance list and the source map's entries, and nothing else: the analysed set, its digest, the
    /// subject, the base and the charts are the ones it was given. A filter that keeps nothing leaves an empty layout
    /// beside the same analysed set.
    pub fn filtered(&self, keep: impl Fn(&Occurrence) -> bool) -> Result<Representation, LayoutError> {
        let kept: Vec<Occurrence> = self.occurrences()?.into_iter().filter(|o| keep(o)).collect();
        let mut plan = self.plan.clone();
        let mut map = self.map.clone();
        plan.primitives[LAYOUT as usize] = Primitive::Layout(match (&self.plan.primitives[0], layout_of(&kept)?) {
            // An atlas that keeps no occurrence stays an atlas.
            (Primitive::Layout(LayoutInput { kind: LayoutKind::Atlas { .. }, .. }), mut layout) if kept.is_empty() => {
                layout.kind = LayoutKind::Atlas { membership: Vec::new() };
                layout
            }
            (_, layout) => layout,
        });
        plan.instances.retain(|i| i.primitive != LAYOUT || kept.iter().any(|o| o.instance == i.instance));
        map.entries.retain(|id, _| plan.instances.iter().any(|i| i.instance == *id));
        Ok(Representation { plan, map })
    }
}

/// The images of one canonical object in a representation (LB4's linked selection). A representation that shows no
/// occurrence of the object has no image of it; that says nothing about the object's relations, so it is its own case
/// and never an unknown relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Linked {
    Images(Vec<InstanceId>),
    NoImage,
}

/// The display instances of `object` in `target`, in instance order.
pub fn linked(target: &Representation, object: &ObjectId) -> Linked {
    let images: Vec<InstanceId> =
        target.map.entries.iter().filter(|(_, entry)| entry.object == *object).map(|(id, _)| *id).collect();
    if images.is_empty() {
        Linked::NoImage
    } else {
        Linked::Images(images)
    }
}

/// The inputs a linked selection was computed for: the named revision and subject, the selected object, the target
/// plan and the digest of the target map's entries.
#[derive(Clone, Debug, PartialEq, Eq)]
struct LinkKey {
    state: u64,
    subject: String,
    object: ObjectId,
    plan: PlanId,
    entries: String,
}

fn entries_digest(target: &Representation) -> String {
    let text: String =
        target.map.entries.iter().map(|(id, entry)| format!("{} {}\n", id.0, object_text(&entry.object))).collect();
    sha256_hex(text.as_bytes())
}

/// A linked selection kept between frames (EV3.10): it is computed for its inputs at a named revision and read by
/// every frame after that. `computations` counts the computations; a frame draw and a camera change add none.
#[derive(Clone, Debug, Default)]
pub struct LinkedSelection {
    key: Option<LinkKey>,
    images: Option<Linked>,
    computations: u64,
    frames: u64,
}

impl LinkedSelection {
    pub fn new() -> LinkedSelection {
        LinkedSelection::default()
    }

    /// Selects `object` and links it into `target`. The same object in the same plan at the same revision and subject,
    /// with the same map entries, is already computed and is not computed again.
    pub fn select(&mut self, target: &Representation, object: &ObjectId) -> &Linked {
        let key = LinkKey {
            state: target.plan.base.state,
            subject: target.plan.subject.digest().to_owned(),
            object: object.clone(),
            plan: target.plan.plan,
            entries: entries_digest(target),
        };
        if self.key.as_ref() != Some(&key) {
            self.images = Some(linked(target, object));
            self.key = Some(key);
            self.computations += 1;
        }
        self.images.as_ref().expect("computed above")
    }

    /// One frame's read of the selection: what was computed, or `None` before any selection. It computes nothing.
    pub fn frame(&mut self) -> Option<&Linked> {
        self.frames += 1;
        self.images.as_ref()
    }

    /// How many times a linked selection was computed.
    pub fn computations(&self) -> u64 {
        self.computations
    }

    /// How many frames read the selection.
    pub fn frames(&self) -> u64 {
        self.frames
    }
}
