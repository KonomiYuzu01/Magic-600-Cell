//! LB1.10's independence relation `jumble-locality/1` for the words of the pinned menu (CT-LB1-06's jumbling half;
//! LP2's pull request 251 for the run-time key, `c600_run::trace`).
//!
//! [`relation`] gives the relation a jumbling run pins beside its menu alphabet ([`Alphabet::pinned`]), so that
//! `trace_key` on a menu word is LB1.10's key on the model: the bits are the table [`Locality::build`] regenerates from
//! the loaded model and the exact Δ(c, d) (Stage 14's Lemma L1), the grip count is the model's, the item count is the
//! pinned menu's, and the identity block is the alphabet's, so a key carries the menu it was taken under and a key of
//! another menu, model or contract revision refuses comparison (`LB1:input.key_contract_mismatch`). Letters at one grip
//! are dependent; two distinct grips are independent exactly when no piece is ever inside both caps.
//!
//! The relation reads the model only, never a state, so a key is the same in every session of one pinned menu.

use std::fmt;

use c600_jumble::locality::{self, Locality};
use c600_jumble::model::ModelError;
use c600_jumble::Context;
use c600_run::trace::{Relation, RelationError, JUMBLE_LOCALITY};

use crate::word::{menu_identity, Alphabet};

/// Why no relation was built: the model's table does not regenerate, or the run-time relation refuses its parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalityError {
    /// [`Locality::build`] refused the loaded model.
    Model(String),
    /// `c600_run`'s relation refused the parts.
    Relation(RelationError),
}

impl fmt::Display for LocalityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalityError::Model(message) => write!(f, "the locality table: {message}"),
            LocalityError::Relation(error) => write!(f, "the trace relation: {error}"),
        }
    }
}

impl From<ModelError> for LocalityError {
    fn from(error: ModelError) -> LocalityError {
        LocalityError::Model(error.0)
    }
}

/// `jumble-locality/1` of the loaded model, regenerated once: the table every relation of [`relation_with`] shares.
pub fn table(ctx: &Context) -> Result<Locality, LocalityError> {
    debug_assert_eq!(locality::JUMBLE_LOCALITY, JUMBLE_LOCALITY, "one contract name in the model and the run");
    Ok(Locality::build(ctx)?)
}

/// The run-time relation of `alphabet`'s menu words under the table `table` of the same model.
pub fn relation_with(table: &Locality, alphabet: &Alphabet) -> Result<Relation, LocalityError> {
    let grips = u32::try_from(table.grips()).map_err(|_| LocalityError::Model("a grip count beyond u32".into()))?;
    let items = u32::try_from(alphabet.items()).map_err(|_| LocalityError::Model("an item count beyond u32".into()))?;
    let identity = menu_identity(alphabet.identity());
    Relation::jumble_locality(&identity, grips, items, table.bits().to_vec()).map_err(LocalityError::Relation)
}

/// The run-time relation of `alphabet`'s menu words on the loaded model ([`table`], then [`relation_with`]).
pub fn relation(ctx: &Context, alphabet: &Alphabet) -> Result<Relation, LocalityError> {
    relation_with(&table(ctx)?, alphabet)
}
