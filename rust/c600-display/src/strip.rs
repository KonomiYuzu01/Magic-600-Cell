//! The relationship strip's data contract (EV3.8 to EV3.10): the view-model fields of one typed relation, which EN3
//! carries without deciding the strip's design. How the strip looks, the words it shows and its encoding tokens are not
//! set here (EV3.8, EV5.11).
//!
//! A [`StripRelation`] carries what EV3.8 lists: a typed statement reference, the relation and its operands, the
//! canonical IDs of the roles it names, its model and level, its base revision, its result and evidence references,
//! and any producing source location. The statement reference is a chapter 06 [`Key`]. It travels in `claims` of the
//! relation's [`ViewNode`] (RPI.5, RPI.65), and [`attach`] checks that a record can be shown on a node.
//!
//! EV3.9: each operand has a canonical value, "none: off the lattice" ([`Missing::OffLattice`]) or `unknown(reason)`
//! ([`Missing::Unknown`]). The two are different values and no sentinel stands for either. Every operand keeps its
//! entry, so a partial result retains its covered and missing parts (KN0.4). A record names the revision it read and
//! attaches only to a node of that revision, so it never shows an old occupant (KN0.6).
//!
//! EV3.10: a record is data computed at a named revision. Nothing here computes a relation or runs per frame. A view's
//! run computes the record for changed inputs in `c600-view`'s lane, and frames draw what that run published.
//!
//! Drafts, where chapters 04 and 06 leave the form open:
//!
//! - A record states one relation over operands of one role, with values of one role. Each operand and each value is
//!   one canonical object of KN1 ([`ObjectId`]). A relation whose value is no canonical object, such as an element of
//!   the frame group, or whose value names several objects, is not drafted.
//! - A record's level is the stored level it read (KN1.8): labels or poses. Off the lattice, pose is the only stored
//!   level, so a record that answers there from classes or regions (EV3.9) names poses.
//! - Evidence references are opaque strings that chapter 06 records resolve; [`attach`] reads none of them.
//! - [`attach`] refuses with `EV3:input.display_plan_invalid`, the code RPI.65 gives a relation node with no claim key.
//!   A record that read another model than its base revision, that read poses at a revision holding labels (KN1.10),
//!   or that answers "none: off the lattice" from labels (EV3.9, KN1.8) is refused the same way.

use std::collections::BTreeMap;

use c600_kernel::IdentityBlock;

use crate::error::Error;
use crate::plan::{Key, Level, NodeKind, ObjectId, RevisionRef, SourceLink, StateRef, ViewNode};

/// What "frame" names in a role (EV3.8). Unqualified, it is reference stickers; every other use carries its qualifier,
/// so the strip can tell them apart. The word the strip shows for each is a preference and is not set here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Frame {
    /// Unqualified: reference stickers.
    ReferenceStickers,
    /// The frame domain of the jumbling research.
    JumblingDomain,
    /// The frame part of the jumbling research.
    JumblingPart,
    /// Frame bits.
    Bits,
    /// Frame sums.
    Sums,
    /// The C₃ component of restoration theory, which no sticker shows.
    RestorationC3,
    /// The C₂ component of restoration theory, which no sticker shows.
    RestorationC2,
    /// A3's F_f.
    A3,
}

/// A role a relation names (EV3.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    Destination,
    /// The piece currently sitting in a position. It is partial: off the lattice it has no value.
    CurrentOccupant,
    WantedPiece,
    Buffer,
    Frame(Frame),
    Residual,
}

/// Why an operand has no value (EV3.9; KN0.4, KN0.5). The two are different values, and neither is an old value, a
/// zero slot or false.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Missing {
    /// "none: off the lattice": a known absence of a lattice value.
    OffLattice,
    /// `unknown(reason)`: insufficient knowledge.
    Unknown(String),
}

/// How much of a result is defined: KN0.4's `total`, `partial` and `undefined`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    /// Every operand has a value.
    Total,
    /// Some operands have a value and some do not.
    Partial,
    /// No operand has a value.
    Undefined,
}

/// One typed relation as the strip consumes it (EV3.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StripRelation {
    /// The typed statement reference: the chapter 06 key that the relation node's `claims` carries (RPI.65).
    pub statement: Key,
    /// The relation, by its name in the grammar shared by code and the theory book (EV5.11).
    pub relation: String,
    /// The role its operands play.
    pub operands: Role,
    /// The role its values play.
    pub values: Role,
    /// The model it read: the KN0.1 identity block.
    pub identity: IdentityBlock,
    /// The level it read (KN1.8).
    pub level: Level,
    /// The revision it read (KN0.2, KN0.6).
    pub base: RevisionRef,
    /// The result: each operand, in canonical order, with its value or why it has none. Operands and values are the
    /// canonical IDs of the record's two roles.
    pub result: BTreeMap<ObjectId, Result<ObjectId, Missing>>,
    /// The evidence references, as chapter 06 records name them.
    pub evidence: Vec<String>,
    /// The producing source location, when a program produced the record (RPI.21).
    pub source: Option<SourceLink>,
}

impl StripRelation {
    /// KN0.4's form of the result: total when every operand has a value, undefined when none has, partial otherwise.
    pub fn extent(&self) -> Extent {
        match self.covered().count() {
            n if n == self.result.len() => Extent::Total,
            0 => Extent::Undefined,
            _ => Extent::Partial,
        }
    }

    /// The covered part: each operand with its value.
    pub fn covered(&self) -> impl Iterator<Item = (&ObjectId, &ObjectId)> {
        self.result.iter().filter_map(|(operand, value)| value.as_ref().ok().map(|value| (operand, value)))
    }

    /// The missing part: each operand with no value, and why.
    pub fn missing(&self) -> impl Iterator<Item = (&ObjectId, &Missing)> {
        self.result.iter().filter_map(|(operand, value)| value.as_ref().err().map(|missing| (operand, missing)))
    }
}

/// Checks that `relation` can be shown on `node` (EV3.8, EV3.9, RPI.65, KN0.6, KN1.8, KN1.10): the node is a relation
/// node whose `claims` carry the record's statement key, both read the same revision, the record read that revision's
/// model and a level the revision holds, and it answers "none: off the lattice" only from poses. A refusal is
/// `EV3:input.display_plan_invalid`.
pub fn attach(node: &ViewNode, relation: &StripRelation) -> Result<(), Error> {
    let id = node.node.0;
    if node.kind != NodeKind::Relation {
        return Err(Error::invalid(format!("node {id} is not a relation node, so it shows no strip relation")));
    }
    if !node.claims.contains(&relation.statement) {
        return Err(Error::invalid(format!(
            "relation node {id} does not carry statement key {} in its claims (EV3.8, RPI.65)",
            relation.statement.0
        )));
    }
    if relation.base != node.base {
        return Err(Error::invalid(format!("the relation read another revision than node {id} (KN0.6)")));
    }
    if relation.identity != relation.base.identity {
        return Err(Error::invalid("the relation read another model than its base revision (KN0.1)"));
    }
    let held = StateRef::Snapshot(relation.base.clone()).level();
    if relation.level == Level::Poses && held == Level::Labels {
        return Err(Error::invalid("a pose-level relation at a label-level revision lifts labels into poses (KN1.10)"));
    }
    if relation.level == Level::Labels && relation.missing().any(|(_, missing)| *missing == Missing::OffLattice) {
        return Err(Error::invalid("off the lattice a relation reads poses, classes or regions, never labels (EV3.9)"));
    }
    Ok(())
}
