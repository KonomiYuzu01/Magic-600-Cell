//! LB1.10's independence relation `full-disjoint-support/1` of the loaded 600-cell-Full model (LP2 draft): the
//! complete labelled-slot support of each of the 1,200 generators, read from the kernel, and the relation that
//! [`Relation::full_disjoint_support`] builds from them. A host pins the relation in a run's options,
//! `c600_run::interp::Options::relation` in process or `c600_runtime::program::JobOptions::relation` on the process
//! host, so `trace_key` orders a word's letters by the pinned model and never by a caller's claim of independence.
//! The supports read the model only, never a session's state.

use std::fmt;
use std::sync::{Mutex, MutexGuard};

use c600_kernel::{Kernel, GENERATORS, PUZZLE_NAME};
use c600_run::trace::{Relation, RelationError};

/// Why the relation of the loaded model could not be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupportError {
    /// The kernel refused a one-letter word or could not compute its effect.
    Kernel(c600_kernel::Error),
    /// The supports do not make a relation.
    Relation(RelationError),
}

impl fmt::Display for SupportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SupportError::Kernel(error) => write!(f, "the kernel could not compute a generator's support: {error}"),
            SupportError::Relation(error) => write!(f, "the supports make no relation: {error}"),
        }
    }
}

impl std::error::Error for SupportError {}

/// The complete support of each generator g, at g - 1: the labelled slots that the one-letter word `[g]` moves, in
/// increasing order, from the kernel's effect of that word. A generator's inverse moves the same slots. This reads
/// 1,200 effects; [`full_relation`] reads them once per model in a process.
pub fn supports(kernel: &Kernel) -> Result<Vec<Vec<i32>>, c600_kernel::Error> {
    (1..=GENERATORS as i64).map(|g| Ok(kernel.effect(&kernel.word(&[g])?)?.source().to_vec())).collect()
}

/// The relations this process has built, by model ID and the SHA-256 of the loaded manifest.
static RELATIONS: Mutex<Vec<((String, String), Relation)>> = Mutex::new(Vec::new());

fn relations() -> MutexGuard<'static, Vec<((String, String), Relation)>> {
    RELATIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// LB1.10's relation `full-disjoint-support/1` of the loaded model, under its identity block: the puzzle name and the
/// model ID. The supports depend on the loaded model alone, and the model ID is the digest of the model's verified
/// file list, so a process builds the relation once per model ID and manifest SHA-256; later calls clone it.
pub fn full_relation(kernel: &Kernel) -> Result<Relation, SupportError> {
    let model = (kernel.identity().model_id, kernel.manifest_sha256().to_string());
    if let Some((_, relation)) = relations().iter().find(|(built, _)| *built == model) {
        return Ok(relation.clone());
    }
    let supports = supports(kernel).map_err(SupportError::Kernel)?;
    let relation = Relation::full_disjoint_support(PUZZLE_NAME, &model.0, &supports).map_err(SupportError::Relation)?;
    let mut built = relations();
    // Another thread may have built the same relation meanwhile; both are one relation.
    if !built.iter().any(|(m, _)| *m == model) {
        built.push((model, relation.clone()));
    }
    Ok(relation)
}
