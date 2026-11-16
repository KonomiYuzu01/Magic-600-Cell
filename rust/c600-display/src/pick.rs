//! Frame stamps (RM.24), pick bindings (RPI.24) and the typed results of pick resolution (RPI.22, RPI.23, RPI.25).
//! A pick first obtains an instance ID and then resolves it through the installed source map; pixels, token colours,
//! pose-table indices and chart inverses never identify an object, and nothing here accepts a chart inverse.

use std::collections::BTreeMap;

use crate::error::codes;
use crate::plan::{InstanceId, Level, Lineage, ObjectId, PlanId, PreviewId, SourceLink, StateRef, ViewId};
use crate::validate::PlanSubject;

/// KN0.4's typed partial result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Partial<T> {
    Total(T),
    Partial { covered: T, missing: Vec<String> },
    Undefined(String),
}

/// RM.24's `FrameStamp` for one view's surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameStamp {
    pub device_epoch: u64,
    pub surface: u64,
    pub surface_generation: u64,
    /// Increasing within one surface and device epoch.
    pub sequence: u64,
    /// The adopted engine state revision.
    pub engine_revision: u64,
    pub view: ViewId,
    pub view_generation: u64,
    /// The installed plan and map generation.
    pub publication_generation: u64,
    /// The host's subject labels of the installed plans (RM.8, RM.9).
    pub subjects: Vec<PlanSubject>,
    pub camera: u64,
    pub viewport: (u32, u32),
    pub visibility: u64,
    pub interaction: u64,
}

/// What a frame shows of an engine preview (RM.26).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewShown {
    pub preview: PreviewId,
    pub literal: String,
    /// The preview's base state revision.
    pub base: u64,
}

/// The preview field of a presented frame; absence is explicit (RM.26).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Presented {
    NoPreview,
    Previews(Vec<PreviewShown>),
}

/// RM.26's `PresentedFrame`: emitted only for a frame whose presentation the adapter confirmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentedFrame {
    pub stamp: FrameStamp,
    pub presented: Presented,
}

/// RPI.24's pick binding: what a pick request and its reply bind. The frame sequence is not part of it, so a later
/// frame with the same bindings can still consume the reply; the engine revision enters through the declared state
/// dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickBinding {
    pub device_epoch: u64,
    pub surface: u64,
    pub surface_generation: u64,
    pub view: ViewId,
    pub view_generation: u64,
    pub publication_generation: u64,
    pub camera: u64,
    pub viewport: (u32, u32),
    pub visibility: u64,
    pub interaction: u64,
    /// The declared dependencies of the installed plans and their values.
    pub dependencies: BTreeMap<String, String>,
}

impl PickBinding {
    /// The first field in which `self` differs from `current`, by its RPI.24/RM.24 name.
    pub fn differs(&self, current: &PickBinding) -> Option<&'static str> {
        let fields: [(&'static str, bool); 10] = [
            ("device_epoch", self.device_epoch != current.device_epoch),
            ("surface", self.surface != current.surface),
            ("surface_generation", self.surface_generation != current.surface_generation),
            ("view", self.view != current.view),
            ("view_generation", self.view_generation != current.view_generation),
            ("publication_generation", self.publication_generation != current.publication_generation),
            ("camera", self.camera != current.camera),
            ("viewport", self.viewport != current.viewport),
            ("visibility", self.visibility != current.visibility),
            ("interaction", self.interaction != current.interaction),
        ];
        fields.into_iter().find(|(_, differs)| *differs).map(|(name, _)| name)
    }
}

/// An ID-pass reply: a stamped instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickReply {
    pub binding: PickBinding,
    pub instance: InstanceId,
}

/// Whether the solver explicitly asked for the resolution or it is passive hover or picking (RPI.25).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickMode {
    Explicit,
    Passive,
}

/// The requested object kind and level (RPI.23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requested {
    /// The occurrence's own canonical object.
    Occurrence,
    Sticker,
    Piece,
    Slot,
    Cell,
    Grip,
    Cap,
    Orbit,
}

/// A resolved pick: the canonical target and its producing source linkage, with the occurrence and its subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub target: ObjectId,
    /// The occurrence's object; a certificate point keeps its witness, grip and source role here.
    pub occurrence: ObjectId,
    pub instance: InstanceId,
    pub plan: PlanId,
    pub subject: StateRef,
    /// The lineage of a derived subject, so a derived occurrence is never taken for a live selection.
    pub lineage: Option<Lineage>,
    /// The producing source version, pinned with the installed publication.
    pub source: SourceLink,
}

/// The kernel's exact projection to slots for a bound subject (RPI.23).
pub trait Projection {
    /// The slot of a sticker or piece at `subject`, or why there is none (off the lattice, no slot is invented; a
    /// subject at a level the projection does not hold, [`codes::LEVEL_NOT_HELD`]).
    fn slot(&self, subject: &StateRef, object: &ObjectId) -> Partial<u32>;
}

/// The projection of a 600-cell-Full label state: sticker `h` is in the slot whose label is `h`. It holds the label
/// level only: a pose-level subject gets [`codes::LEVEL_NOT_HELD`] before any digest is compared (KN1.8, KN1.10), so
/// no slot is lifted from labels into poses, even under a digest equal to its own.
pub struct LabelProjection {
    slot_of: Vec<u32>,
    digest: String,
}

impl LabelProjection {
    /// The projection for labels with that state digest.
    pub fn new(labels: &[i32], digest: &str) -> LabelProjection {
        let mut slot_of = vec![u32::MAX; labels.len()];
        for (slot, &label) in labels.iter().enumerate() {
            if let Some(entry) = usize::try_from(label).ok().and_then(|l| slot_of.get_mut(l)) {
                *entry = slot as u32;
            }
        }
        LabelProjection { slot_of, digest: digest.to_owned() }
    }
}

impl Projection for LabelProjection {
    fn slot(&self, subject: &StateRef, object: &ObjectId) -> Partial<u32> {
        if subject.level() != Level::Labels {
            return Partial::Undefined(codes::LEVEL_NOT_HELD.to_owned());
        }
        if subject.digest() != self.digest {
            return Partial::Undefined("the projection is for another subject".to_owned());
        }
        match object {
            ObjectId::Sticker(home) => match self.slot_of.get(*home as usize) {
                Some(&slot) if slot != u32::MAX => Partial::Total(slot),
                _ => Partial::Undefined("no such sticker".to_owned()),
            },
            ObjectId::Slot(slot) => Partial::Total(*slot),
            _ => Partial::Undefined("a piece of 600-cell-Full has no single slot".to_owned()),
        }
    }
}
