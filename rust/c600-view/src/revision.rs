//! Revisions, bases and declared inputs (KN0.2, ONE.8, ONE.10, ONE.63, KN7.3).
//!
//! A [`Revision`] carries the engine epoch and the three counters of KN0.2's `RevisionRef` and its state digest.
//! Counters never decrease within an engine epoch, and undo advances them, so equal digests can come back with larger
//! counters. A new epoch may start its counters again, so counters name a revision only within their epoch. A
//! [`Base`] is ONE.8's pair of a revision and the parts its output depends on. Output attaches to an adopted revision
//! only when it has the same epoch and the counters of every declared part are equal (ONE.63, ONE.70). An
//! [`InputKey`] names a read result's declared inputs, so a result can come back after an undo (ONE.10).

use crate::Digest;

/// One revision of one session (KN0.2): its engine epoch, its three counters and its state digest.
///
/// The session, identity block, mode and head of KN0.2 are not modelled: one [`crate::ViewModel`] serves one session
/// and one identity, and every move of `head` advances `state`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Revision {
    /// KN0.2's engine epoch, which distinguishes process lifetimes: here a number that grows with each engine start
    /// (draft).
    pub epoch: u64,
    /// State revision.
    pub state: u64,
    /// Workspace revision.
    pub workspace: u64,
    /// Protection revision.
    pub protection: u64,
    /// KN0.2's digest: labels with Jumble off, poses with Jumble on.
    pub digest: Digest,
}

impl Revision {
    /// A revision from its epoch, counters and digest.
    pub fn new(epoch: u64, state: u64, workspace: u64, protection: u64, digest: Digest) -> Revision {
        Revision { epoch, state, workspace, protection, digest }
    }

    /// The counter of one part.
    pub fn counter(&self, part: Part) -> u64 {
        match part {
            Part::State => self.state,
            Part::Workspace => self.workspace,
            Part::Protection => self.protection,
        }
    }

    /// True when the revision belongs to a later epoch than `other`, or to the same epoch with no counter below
    /// `other`'s (KN0.2: counters never decrease within an epoch).
    pub fn not_older_than(&self, other: &Revision) -> bool {
        self.epoch > other.epoch
            || (self.epoch == other.epoch
                && self.state >= other.state
                && self.workspace >= other.workspace
                && self.protection >= other.protection)
    }

    /// True when the revision has `other`'s epoch and the counters of every part in `depends` equal `other`'s. Another
    /// epoch never matches, whatever `depends` names: its counters name other revisions.
    pub fn same_parts(&self, other: &Revision, depends: Depends) -> bool {
        self.epoch == other.epoch && depends.parts().all(|part| self.counter(part) == other.counter(part))
    }

    /// True when the epoch and all three counters equal `other`'s.
    pub fn same_counters(&self, other: &Revision) -> bool {
        self.same_parts(other, Depends::ALL)
    }
}

/// A part of a revision that a result can depend on (ONE.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Part {
    State,
    Workspace,
    Protection,
}

/// ONE.8's `Set<state | workspace | protection>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Depends(u8);

impl Depends {
    /// Depends on nothing.
    pub const NONE: Depends = Depends(0);
    /// Depends on the state only.
    pub const STATE: Depends = Depends(1);
    /// Depends on the workspace only.
    pub const WORKSPACE: Depends = Depends(2);
    /// Depends on protection only.
    pub const PROTECTION: Depends = Depends(4);
    /// Depends on all three parts.
    pub const ALL: Depends = Depends(7);

    /// The set of the given parts.
    pub fn of(parts: &[Part]) -> Depends {
        parts.iter().fold(Depends::NONE, |set, part| set.union(Depends::part(*part)))
    }

    fn part(part: Part) -> Depends {
        match part {
            Part::State => Depends::STATE,
            Part::Workspace => Depends::WORKSPACE,
            Part::Protection => Depends::PROTECTION,
        }
    }

    /// True when `part` is in the set.
    pub fn contains(self, part: Part) -> bool {
        self.0 & Depends::part(part).0 != 0
    }

    /// The union of two sets.
    pub fn union(self, other: Depends) -> Depends {
        Depends(self.0 | other.0)
    }

    /// The parts in the set, in the order state, workspace, protection.
    pub fn parts(self) -> impl Iterator<Item = Part> {
        [Part::State, Part::Workspace, Part::Protection].into_iter().filter(move |part| self.contains(*part))
    }

    /// The set as bits: state 1, workspace 2, protection 4.
    pub fn bits(self) -> u8 {
        self.0
    }
}

/// ONE.8's `Base`: the revision a result read and the parts it depends on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Base {
    pub revision: Revision,
    pub depends: Depends,
}

impl Base {
    /// A base from its revision and declared parts.
    pub fn new(revision: Revision, depends: Depends) -> Base {
        Base { revision, depends }
    }

    /// ONE.63: true when output with this base may attach to `adopted`.
    pub fn matches(&self, adopted: &Revision) -> bool {
        self.revision.same_parts(adopted, self.depends)
    }

    /// The declared inputs of a result with this base (ONE.10).
    pub fn inputs(&self) -> InputKey {
        InputKey::at(&self.revision, self.depends)
    }
}

/// The declared inputs of a read result (ONE.10, KN7.3).
///
/// The state part is identified by KN0.2's digest, so an undo that returns to an equal digest gives an equal key, in
/// any epoch (KN7.3, ONE.17). Workspace and protection have no digest in KN0.2, so they are identified by their epoch
/// and counter (draft).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputKey {
    depends: Depends,
    state: Option<Digest>,
    workspace: Option<(u64, u64)>,
    protection: Option<(u64, u64)>,
}

impl InputKey {
    /// The inputs that `depends` names at `revision`.
    pub fn at(revision: &Revision, depends: Depends) -> InputKey {
        InputKey {
            depends,
            state: depends.contains(Part::State).then_some(revision.digest),
            workspace: depends.contains(Part::Workspace).then_some((revision.epoch, revision.workspace)),
            protection: depends.contains(Part::Protection).then_some((revision.epoch, revision.protection)),
        }
    }
}
