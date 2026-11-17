//! The source route's recipe expansion: a case step's recipe to its literal word, through the kernel and outside the
//! program. The expansion reads the model only, never the session's state, so the word the tool proposes is fixed
//! before the run starts.

use c600_kernel::json::Value;
use c600_kernel::{Kernel, Refusal};

/// What a step's recipe expands to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expansion {
    /// The kernel accepted the recipe: the letters of its expansion, in order.
    Literal(Vec<i32>),
    /// The kernel refused the recipe, which is exactly one word step whose moves are integers that fit `i32`. The
    /// moves are the literal word, unchanged: the tool proposes it and the command layer refuses it with the same
    /// answer it gives the same draft on the command route.
    Unchecked { letters: Vec<i32>, refusal: Refusal },
    /// The kernel refused the recipe, and the recipe names no literal word. Nothing runs and nothing is invoked.
    Refused(Refusal),
}

impl Expansion {
    /// The literal word the tool runs with, if there is one.
    pub fn letters(&self) -> Option<&[i32]> {
        match self {
            Expansion::Literal(letters) | Expansion::Unchecked { letters, .. } => Some(letters),
            Expansion::Refused(_) => None,
        }
    }
}

/// Expands a recipe in the 0.4 JSON form through the kernel, with the kernel's limits (KN2).
pub fn expand(kernel: &Kernel, recipe: &Value) -> Expansion {
    match kernel.recipe(recipe) {
        // The recipe is this kernel's own parse, so its model always matches; a refusal would still run nothing.
        Ok(recipe) => match kernel.letters(&recipe) {
            Ok(letters) => Expansion::Literal(letters),
            Err(refusal) => Expansion::Refused(refusal),
        },
        Err(refusal) => match single_word(recipe) {
            Some(letters) => Expansion::Unchecked { letters, refusal },
            None => Expansion::Refused(refusal),
        },
    }
}

/// The moves of a recipe that is exactly one `{"kind": "word", "moves": [...]}` step whose moves are integers that
/// fit `i32`.
fn single_word(recipe: &Value) -> Option<Vec<i32>> {
    let [step] = recipe.as_array()? else {
        return None;
    };
    if step.as_object()?.len() != 2 || step.get("kind")?.as_str()? != "word" {
        return None;
    }
    step.get("moves")?.as_array()?.iter().map(|m| m.as_i64().and_then(|m| i32::try_from(m).ok())).collect()
}

/// The number of moves in each step of a recipe made of plain word steps only, in order: every step is exactly
/// `{"kind": "word", "moves": [...]}`, as `single_word` reads one. None for a recipe that is not a list, is empty, or
/// has any other step: a star step, or a word step with another field, which the bridge's steps would not reproduce.
/// A word step expands to its moves unchanged, so for a recipe the kernel expanded these are the lengths of the
/// solver's word steps in its literal word ([`crate::bridge::Steps`]).
pub fn word_steps(recipe: &Value) -> Option<Vec<usize>> {
    let steps = recipe.as_array().filter(|steps| !steps.is_empty())?;
    let length = |step: &Value| {
        if step.as_object()?.len() != 2 || step.get("kind")?.as_str()? != "word" {
            return None;
        }
        Some(step.get("moves")?.as_array()?.len())
    };
    steps.iter().map(length).collect()
}

/// `Digest<literal word>`: the SHA-256, in hex, of the letters as little-endian 32-bit integers. It is the oracle's
/// `expansion_sha256` and the command layer's literal digest.
pub fn literal_digest(letters: &[i32]) -> String {
    let mut bytes = Vec::with_capacity(4 * letters.len());
    for letter in letters {
        bytes.extend_from_slice(&letter.to_le_bytes());
    }
    c600_digest::sha256_hex(&bytes)
}
