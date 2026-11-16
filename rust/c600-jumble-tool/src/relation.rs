//! One typed relation of the relationship strip on the jumbled puzzle (EV3.8, EV3.9): the current occupant of a
//! sticker slot, at pose level, on and off the lattice.
//!
//! The occupant of slot t is the piece sitting in the position of t's home piece A (the glossary's "Occupant"). From
//! `c600-jumble`'s public facts:
//!
//! - A at the identity pose sits in its own position, so the occupant is A.
//! - Only a piece of A's K⁺-orbit can sit in A's position on the lattice, and a piece at a lattice pose other than the
//!   identity sits in the position that pose makes of it. The public API does not say which position that is (no slot
//!   action), so while A is not at the identity and some piece of its orbit is at such a pose, the answer is
//!   `unknown(`[`NO_SLOT_ACTION`]`)`.
//! - Otherwise A is off the lattice and no lattice piece sits in its position: "none: off the lattice".
//!
//! The relation reads the pose table, the K⁺-orbit classes and the model's home slots, and no label (EV3.9, KN1.15).
//! It assumes a configuration free of collisions, as every state reached by admitted twists is (state contract,
//! section 3); a raw colliding pose table (KN2.14) is outside it.

use std::collections::BTreeMap;
use std::fmt;

use c600_jumble::model::NS;
use c600_jumble::state::State;
use c600_jumble::{codes, Context};
use c600_kernel::IdentityBlock;

use crate::level::Level;

/// The relation's name (EV3.8's role "current occupant").
pub const RELATION: &str = "current occupant";

/// The reason of an unknown occupant.
pub const NO_SLOT_ACTION: &str =
    "no slot action: the public kernel API does not say which position a lattice pose other than the identity fills";

/// Why a slot has no occupant in a result. "None: off the lattice" and unknown are different values (KN0.4, EV3.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    /// "none: off the lattice": no lattice piece sits in the slot's position. A known absence.
    OffLattice,
    /// `unknown(reason)`: the relation cannot tell which piece sits there (KN0.5).
    Unknown(&'static str),
}

/// KN0.4's typed partial result: `total(T)`, `partial(covered: T, missing: M)` or `undefined(reason)`, where the
/// reason is the missing part, since nothing is covered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Partial<T, M> {
    Total(T),
    Partial { covered: T, missing: M },
    Undefined(M),
}

/// The occupants of named slots: slot to piece for the covered slots, slot to [`Missing`] for the others.
pub type Answer = Partial<BTreeMap<u32, u32>, BTreeMap<u32, Missing>>;

/// The relation's result, with what it read (KN0.6, EV3.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occupants {
    /// [`RELATION`].
    pub relation: &'static str,
    /// The level it read: always [`Level::Poses`].
    pub level: Level,
    /// The identity block of the puzzle, model and menu it read.
    pub identity: IdentityBlock,
    /// `Digest<poses>` of the state it read (`State::digest`).
    pub read: String,
    /// The answer for the named slots, each named once.
    pub answer: Answer,
}

impl Occupants {
    /// The answer for one slot: the occupant, or why there is none. `None` for a slot the call did not name.
    pub fn get(&self, slot: u32) -> Option<Result<u32, Missing>> {
        let (covered, missing) = match &self.answer {
            Partial::Total(covered) => (Some(covered), None),
            Partial::Partial { covered, missing } => (Some(covered), Some(missing)),
            Partial::Undefined(missing) => (None, Some(missing)),
        };
        if let Some(&piece) = covered.and_then(|covered| covered.get(&slot)) {
            return Some(Ok(piece));
        }
        missing.and_then(|missing| missing.get(&slot)).map(|&missing| Err(missing))
    }
}

/// A slot the model does not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoSuchSlot(pub u32);

impl NoSuchSlot {
    /// The chapter 13 code: a named object outside the model.
    pub fn code(&self) -> &'static str {
        codes::NAMED_OBJECT_INVALID
    }
}

impl fmt::Display for NoSuchSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: slot {} is not in 0..{}", self.code(), self.0, NS - 1)
    }
}

impl std::error::Error for NoSuchSlot {}

/// The current occupant of each named slot in `state`, by the rule of the module documentation. A slot outside the
/// model refuses the whole call; repeats are answered once.
pub fn current_occupants(ctx: &Context, state: &State, slots: &[u32]) -> Result<Occupants, NoSuchSlot> {
    if let Some(&slot) = slots.iter().find(|&&slot| slot as usize >= NS) {
        return Err(NoSuchSlot(slot));
    }
    let orbit = |piece: usize| ctx.regions.orbit_of[piece] as usize;
    // The orbits with a piece at a lattice pose other than the identity.
    let mut turned = vec![false; ctx.regions.reps.len()];
    for piece in 0..state.pose_ids().len() {
        if matches!(state.pose_of(piece).kidx(), Some(k) if k != 0) {
            turned[orbit(piece)] = true;
        }
    }
    let mut covered = BTreeMap::new();
    let mut missing = BTreeMap::new();
    for &slot in slots {
        let home = ctx.model.slot_piece[slot as usize];
        if state.pose_of(home as usize).kidx() == Some(0) {
            covered.insert(slot, home);
        } else if turned[orbit(home as usize)] {
            missing.insert(slot, Missing::Unknown(NO_SLOT_ACTION));
        } else {
            missing.insert(slot, Missing::OffLattice);
        }
    }
    let answer = match (covered.is_empty(), missing.is_empty()) {
        (_, true) => Partial::Total(covered),
        (true, false) => Partial::Undefined(missing),
        (false, false) => Partial::Partial { covered, missing },
    };
    Ok(Occupants {
        relation: RELATION,
        level: Level::Poses,
        identity: ctx.identity(state.menu()),
        read: state.digest(ctx),
        answer,
    })
}
