//! Engine bundles (RPI.2, RPI.9, ONE.60, ONE.61).
//!
//! A bundle is one revision's snapshot metadata, mutable labels and styles, plus pose indices, pose table and lattice
//! when Jumble is on. [`validate_engine_bundle`] is RPI.9's pure check: it returns the complete bundle or
//! `refresh_required(EV3:input.display_bundle_mismatch)`. A complete bundle is a read-only image of one revision,
//! replaced whole (ONE.61). No arrays are held here, only their digests: the view model never reads the labels.

use c600_digest::sha256;

use crate::error::{code, BundleFault, Context, Error};
use crate::revision::Revision;
use crate::Digest;

/// The kinds of array an engine bundle carries (RPI.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PartKind {
    Metadata,
    Labels,
    Styles,
    PoseIndices,
    PoseTable,
    Lattice,
}

impl PartKind {
    fn tag(self) -> u8 {
        match self {
            PartKind::Metadata => 0,
            PartKind::Labels => 1,
            PartKind::Styles => 2,
            PartKind::PoseIndices => 3,
            PartKind::PoseTable => 4,
            PartKind::Lattice => 5,
        }
    }
}

/// KN0.2's mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    JumbleOff,
    JumbleOn,
}

impl Mode {
    /// The part kinds a complete bundle of this mode has, each exactly once (RPI.9).
    pub fn required(self) -> &'static [PartKind] {
        match self {
            Mode::JumbleOff => &[PartKind::Metadata, PartKind::Labels, PartKind::Styles],
            Mode::JumbleOn => &[
                PartKind::Metadata,
                PartKind::Labels,
                PartKind::Styles,
                PartKind::PoseIndices,
                PartKind::PoseTable,
                PartKind::Lattice,
            ],
        }
    }
}

/// One received array: its kind, the revision that produced it and the digest of its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BundlePart {
    pub kind: PartKind,
    pub revision: Revision,
    pub digest: Digest,
}

/// A revision-bound candidate: what the host has received for `revision` (RPI.9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleCandidate {
    pub revision: Revision,
    pub mode: Mode,
    pub parts: Vec<BundlePart>,
}

/// A complete engine bundle of one revision (RPI.9). Its parts are sorted by kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteBundle {
    revision: Revision,
    mode: Mode,
    parts: Vec<BundlePart>,
    digest: Digest,
}

impl CompleteBundle {
    /// The revision every part belongs to.
    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    /// The bundle's mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The parts, sorted by kind.
    pub fn parts(&self) -> &[BundlePart] {
        &self.parts
    }

    /// The SHA-256 of the revision counters and every part's kind and digest, in kind order.
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
}

fn mismatch(fault: BundleFault) -> Error {
    Error::refusal(code::DISPLAY_BUNDLE_MISMATCH, Context::Bundle(fault))
}

/// RPI.9's `validate_engine_bundle`: pure, and `Err` is `refresh_required(Error)`.
///
/// The candidate is complete when every part its mode requires occurs exactly once, no other part occurs, and every
/// part belongs to the candidate's revision. Missing or mismatched data is never patched from another revision.
pub fn validate_engine_bundle(candidate: &BundleCandidate) -> Result<CompleteBundle, Error> {
    let required = candidate.mode.required();
    let mut parts = candidate.parts.clone();
    parts.sort_by_key(|part| part.kind);
    for pair in parts.windows(2) {
        if pair[0].kind == pair[1].kind {
            return Err(mismatch(BundleFault::Duplicate(pair[0].kind)));
        }
    }
    if let Some(part) = parts.iter().find(|part| !required.contains(&part.kind)) {
        return Err(mismatch(BundleFault::Unexpected(part.kind)));
    }
    if let Some(kind) = required.iter().find(|kind| !parts.iter().any(|part| part.kind == **kind)) {
        return Err(mismatch(BundleFault::Missing(*kind)));
    }
    if let Some(part) = parts.iter().find(|part| part.revision != candidate.revision) {
        return Err(mismatch(BundleFault::Mismatched {
            kind: part.kind,
            part: part.revision,
            bundle: candidate.revision,
        }));
    }
    let mut bytes = Vec::with_capacity(32 + parts.len() * 33);
    for counter in [candidate.revision.state, candidate.revision.workspace, candidate.revision.protection] {
        bytes.extend_from_slice(&counter.to_be_bytes());
    }
    bytes.extend_from_slice(&candidate.revision.digest);
    for part in &parts {
        bytes.push(part.kind.tag());
        bytes.extend_from_slice(&part.digest);
    }
    Ok(CompleteBundle { revision: candidate.revision, mode: candidate.mode, parts, digest: sha256(&bytes) })
}
