//! One typed relation of the relationship strip on the jumbled puzzle (EV3.8, EV3.9): the current occupant of a
//! sticker slot, at pose level, on and off the lattice.
//!
//! The occupant of slot t is the piece sitting in the position of t's home piece A (the glossary's "Occupant"), that
//! is, the lattice piece in A's chamber. A piece B at lattice pose k sits in the chamber of signature k(M(B)), which
//! M8's level tables give (`c600_jumble::levels::Tables::image`, the slot action of K⁺). A piece off the lattice sits
//! in no chamber, as in `levels::positions`. So the answer for t is:
//!
//! - the one lattice piece in A's chamber;
//! - "none: off the lattice" when no lattice piece is in A's chamber. Without a collision, an empty chamber means that
//!   pieces are off the lattice;
//! - `unknown(`[`PIECES_COLLIDE`]`)` when two lattice pieces are in A's chamber, or when A's chamber is empty while two
//!   pieces share another one. Only a raw pose table collides (KN2.14); no twist reaches one.
//!
//! The relation reads the pose table, the chambers of K⁺ and the model's home slots, and no label (EV3.9, KN1.15).
//! Where `levels::labels` covers slot t, the occupant is the piece whose sticker the label names (CT-EV3-06).

use std::collections::BTreeMap;
use std::fmt;

use c600_jumble::levels;
use c600_jumble::model::{NP, NS};
use c600_jumble::state::{EngineError, State};
use c600_jumble::{codes, Context};
use c600_kernel::IdentityBlock;

use crate::level::Level;

/// The relation's name (EV3.8's role "current occupant").
pub const RELATION: &str = "current occupant";

/// The reason of an unknown occupant.
pub const PIECES_COLLIDE: &str =
    "pieces collide: the pose table puts two lattice pieces in one chamber (a raw table, KN2.14), so which piece sits where is not defined";

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
/// model refuses the whole call; repeats are answered once. The outer error is an engine failure of the level tables,
/// never a property of the state.
pub fn current_occupants(
    ctx: &Context,
    state: &State,
    slots: &[u32],
) -> Result<Result<Occupants, NoSuchSlot>, EngineError> {
    if let Some(&slot) = slots.iter().find(|&&slot| slot as usize >= NS) {
        return Ok(Err(NoSuchSlot(slot)));
    }
    let tables = levels::tables(ctx)?;
    // For each chamber, how many lattice pieces sit in it, and the last one counted.
    let mut claims = vec![0u8; NP];
    let mut claimant = vec![0u32; NP];
    for piece in 0..NP {
        if let Some(k) = state.pose_of(piece).kidx() {
            let chamber = tables.image(ctx, piece, k) as usize;
            claims[chamber] = claims[chamber].saturating_add(1);
            claimant[chamber] = piece as u32;
        }
    }
    let collided = claims.iter().any(|&n| n > 1);
    let mut covered = BTreeMap::new();
    let mut missing = BTreeMap::new();
    for &slot in slots {
        let chamber = ctx.model.slot_piece[slot as usize] as usize;
        match claims[chamber] {
            1 => {
                covered.insert(slot, claimant[chamber]);
            }
            0 if !collided => {
                missing.insert(slot, Missing::OffLattice);
            }
            _ => {
                missing.insert(slot, Missing::Unknown(PIECES_COLLIDE));
            }
        }
    }
    let answer = match (covered.is_empty(), missing.is_empty()) {
        (_, true) => Partial::Total(covered),
        (true, false) => Partial::Undefined(missing),
        (false, false) => Partial::Partial { covered, missing },
    };
    Ok(Ok(Occupants {
        relation: RELATION,
        level: Level::Poses,
        identity: ctx.identity(state.menu()),
        read: state.digest(ctx),
        answer,
    }))
}
