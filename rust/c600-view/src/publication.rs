//! Publications: nodes, plan-size descriptors and source maps (RPI.6, RPI.10, RM.11, RM.13, RM.20, LI.16).
//!
//! A publication is the content of one `publish_view` call: view nodes, display plans and source maps, bound to one
//! [`Base`]. This crate keeps only what admission needs: each plan is a minimal plan-size descriptor that states the
//! counts of RM.11's per-publication fields, plus opaque descriptor bytes. Structure is checked first (duplicate IDs
//! and dangling source-map references are `EV3:input.display_plan_invalid`, RM.20), then the fixed ceilings (RM.12).
//!
//! The canonical bytes stand in for LI.16's canonical form, which is not pinned here (draft): a version byte,
//! unsigned LEB128 integers and length-prefixed byte strings, in field order. The output charge and the
//! `canonical_output_bytes` count are their length. Transport framing, compression and cache reuse never change
//! that length (RM.13).

use c600_digest::sha256;

use crate::error::{code, Context, Error, Form, PlanFault};
use crate::limits::{Field, LimitProfile, OutputAllowance};
use crate::provenance::Provenance;
use crate::revision::Base;
use crate::Digest;

/// A minimal plan-size descriptor: one `DisplayPlan` as admission counts it (RPI.6, RM.11).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanDescriptor {
    /// `DisplayPlanId`.
    pub plan: u64,
    /// The `SourceMapId` the plan's instances resolve through (RPI.21).
    pub source_map: u64,
    /// `DisplayInstance` records; duplicates count separately.
    pub instances: u64,
    /// Chart records.
    pub charts: u64,
    /// Stage occurrences after chain expansion.
    pub stages: u64,
    /// Weighted checked scalar operations for one vertex evaluation of the expanded chain (RM.38).
    pub stage_ops: u64,
    /// Weighted checked scalar operations for one fragment invocation (RM.38).
    pub fragment_ops: u64,
    /// Supplied position rows.
    pub layout_vertices: u64,
    /// Index entries, including incidence boundaries and graph edges.
    pub layout_indices: u64,
    /// Exact-set, relation and certificate records.
    pub overlay_items: u64,
    /// Opaque descriptor bytes.
    pub payload: Vec<u8>,
}

/// A source map: its ID and entry count (RPI.21).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapDescriptor {
    pub map: u64,
    pub entries: u64,
}

/// The content of one publication: nodes, plans, maps and the pinned source length.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Content {
    /// View nodes as opaque canonical bytes.
    pub nodes: Vec<Vec<u8>>,
    pub plans: Vec<PlanDescriptor>,
    pub maps: Vec<MapDescriptor>,
    /// Pinned source byte length (RM.11 `source_bytes`).
    pub source_bytes: u64,
}

fn put_uint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    put_uint(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

/// The version byte of this crate's canonical publication bytes.
pub const CANONICAL_VERSION: u8 = 1;

impl Content {
    /// True when the publication has nothing to show: no nodes and no plans.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.plans.is_empty()
    }

    /// The canonical uncompressed bytes of the publication bound to `base` (LI.16 stand-in, draft).
    pub fn canonical_bytes(&self, base: &Base) -> Vec<u8> {
        let mut out = vec![CANONICAL_VERSION];
        put_uint(&mut out, base.revision.state);
        put_uint(&mut out, base.revision.workspace);
        put_uint(&mut out, base.revision.protection);
        out.extend_from_slice(&base.revision.digest);
        out.push(base.depends.bits());
        put_uint(&mut out, self.source_bytes);
        put_uint(&mut out, self.nodes.len() as u64);
        for node in &self.nodes {
            put_bytes(&mut out, node);
        }
        put_uint(&mut out, self.plans.len() as u64);
        for plan in &self.plans {
            for value in [
                plan.plan,
                plan.source_map,
                plan.instances,
                plan.charts,
                plan.stages,
                plan.stage_ops,
                plan.fragment_ops,
                plan.layout_vertices,
                plan.layout_indices,
                plan.overlay_items,
            ] {
                put_uint(&mut out, value);
            }
            put_bytes(&mut out, &plan.payload);
        }
        put_uint(&mut out, self.maps.len() as u64);
        for map in &self.maps {
            put_uint(&mut out, map.map);
            put_uint(&mut out, map.entries);
        }
        out
    }

    /// RM.20: duplicate plan or map IDs and dangling source-map references invalidate the whole publication.
    pub fn validate(&self, form: Form) -> Result<(), Error> {
        let invalid = |fault| Error { code: code::DISPLAY_PLAN_INVALID, form, context: Context::Plan(fault) };
        let mut plans: Vec<u64> = self.plans.iter().map(|plan| plan.plan).collect();
        plans.sort_unstable();
        if let Some(pair) = plans.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(invalid(PlanFault::DuplicatePlan(pair[0])));
        }
        let mut maps: Vec<u64> = self.maps.iter().map(|map| map.map).collect();
        maps.sort_unstable();
        if let Some(pair) = maps.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(invalid(PlanFault::DuplicateMap(pair[0])));
        }
        if let Some(plan) = self.plans.iter().find(|plan| maps.binary_search(&plan.source_map).is_err()) {
            return Err(invalid(PlanFault::DanglingMap { plan: plan.plan, map: plan.source_map }));
        }
        Ok(())
    }

    /// The per-publication counts of RM.11 for this content, given its canonical length.
    ///
    /// `stage_ops` and `fragment_ops` are maxima over the plans; every other count is a sum. The view-level fields
    /// (`overdraw`, `frame_work`, `view_vertex_evals` and the adapter bytes) are charged at open, show and resize
    /// instead (RM.38, RM.39).
    pub fn counts(&self, canonical_len: u64) -> Vec<(Field, u128)> {
        let sum = |f: fn(&PlanDescriptor) -> u64| self.plans.iter().map(|plan| u128::from(f(plan))).sum::<u128>();
        let max = |f: fn(&PlanDescriptor) -> u64| self.plans.iter().map(|plan| u128::from(f(plan))).max().unwrap_or(0);
        let node_bytes: u128 = self.nodes.iter().map(|node| node.len() as u128).sum();
        let payload_bytes: u128 = self.plans.iter().map(|plan| plan.payload.len() as u128).sum();
        vec![
            (Field::Nodes, self.nodes.len() as u128),
            (Field::Plans, self.plans.len() as u128),
            (Field::Instances, sum(|plan| plan.instances)),
            (Field::Maps, self.maps.len() as u128),
            (Field::MapEntries, self.maps.iter().map(|map| u128::from(map.entries)).sum()),
            (Field::Charts, sum(|plan| plan.charts)),
            (Field::Stages, sum(|plan| plan.stages)),
            (Field::StageOps, max(|plan| plan.stage_ops)),
            (Field::FragmentOps, max(|plan| plan.fragment_ops)),
            (Field::LayoutVertices, sum(|plan| plan.layout_vertices)),
            (Field::LayoutIndices, sum(|plan| plan.layout_indices)),
            (Field::OverlayItems, sum(|plan| plan.overlay_items)),
            (Field::InputBytes, node_bytes + payload_bytes),
            (Field::SourceBytes, u128::from(self.source_bytes)),
            (Field::CanonicalOutputBytes, u128::from(canonical_len)),
        ]
    }

    /// Structure first, then RM.12's fixed ceilings. Returns the canonical bytes on success.
    pub fn admit(&self, base: &Base, profile: &LimitProfile, form: Form) -> Result<Vec<u8>, Error> {
        self.validate(form)?;
        let bytes = self.canonical_bytes(base);
        profile.check_all(&self.counts(bytes.len() as u64), form)?;
        Ok(bytes)
    }

    /// The SHA-256 of the canonical bytes bound to `base`.
    pub fn digest(&self, base: &Base) -> Digest {
        sha256(&self.canonical_bytes(base))
    }
}

/// How a standalone publication's bytes travelled to the host. No charge or ceiling reads it (RM.13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    /// The canonical bytes as they are.
    Canonical,
    /// A compressed or framed transport of `frame_bytes` bytes.
    Compressed { frame_bytes: u64 },
}

/// A standalone host publication: its payload exists at admission (RM.20, LI.25).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationInput {
    /// The generation of the target view the producer prepared it for (RM.20).
    pub generation: u64,
    pub base: Base,
    pub content: Content,
    pub transport: Transport,
    /// The producer's declared output allowance (RM.13).
    pub output: OutputAllowance,
    /// The block the host filled for the producer (BND.22). The view model stores it as given and refuses nothing on
    /// it (BND.26).
    pub provenance: Provenance,
}
