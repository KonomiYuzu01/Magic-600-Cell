//! What a check, a compile or an interchange reports: a chapter 13 code, the gate that raised it, where, and a text for
//! display only. Handling reads the code and the gate, never the text (ERR.6).

use std::fmt;

/// The codes this crate raises. Every one is an active chapter 13 row; the kernel's and the editor's codes are
/// forwarded unchanged (ERR.7).
pub mod code {
    pub use c600_kernel::codes::{IDENTITY_MISMATCH, PRIMITIVE_CEILING, RECIPE_SHAPE};

    /// `PS1:input.persistence_format`: the carrier or a required field's structure is invalid (PS1.12, PS1.13).
    pub const PERSISTENCE_FORMAT: &str = "PS1:input.persistence_format";
    /// `PS2:input.persistence_version`: an unknown format and version pair, or an unknown required extension
    /// (PS2.1, FP.10).
    pub const PERSISTENCE_VERSION: &str = "PS2:input.persistence_version";
    /// `PS1:input.noncanonical_value`: the bytes are not the one canonical encoding of their value (PS1.3, FP.5, FP.7).
    pub const NONCANONICAL: &str = "PS1:input.noncanonical_value";
    /// `PS1:resource_limit.persistence_size`: a stored, decoded, nesting or count limit would be passed (PS1.10).
    pub const PERSISTENCE_SIZE: &str = "PS1:resource_limit.persistence_size";
    /// `PS1:input.digest_mismatch`: bytes or recorded data disagree with their claimed digest (PS1.2, PS1.37).
    pub const DIGEST_MISMATCH: &str = "PS1:input.digest_mismatch";
    /// `PS3:input.checkpoint_replay`: a replay path, a pre or post state, or the saved state cannot be verified by the
    /// replay of its legal witnesses (PS1.29, PS3.5).
    pub const CHECKPOINT_REPLAY: &str = "PS3:input.checkpoint_replay";
}

/// A step of the compile (ADR-D01.81; revision 6, R-2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gate {
    /// The bytes, the encoding, the record's fields, its seal and its structure.
    Format,
    /// The log's model identity against the loaded model's.
    ModelMatch,
    /// Every move replayed with its legal witness.
    Moves,
    /// The saved state's full-state hash against the replayed head.
    FinalHash,
    /// Every embedded macro's witness.
    Macros,
    /// Every embedded script, compiled by the editor services and never run.
    Scripts,
    /// A 0.4 or MPUlt interchange file (LP4-13).
    Interchange,
}

impl Gate {
    pub const COMPILE: [Gate; 6] =
        [Gate::Format, Gate::ModelMatch, Gate::Moves, Gate::FinalHash, Gate::Macros, Gate::Scripts];

    pub fn name(self) -> &'static str {
        match self {
            Gate::Format => "format",
            Gate::ModelMatch => "model_match",
            Gate::Moves => "moves",
            Gate::FinalHash => "final_hash",
            Gate::Macros => "macros",
            Gate::Scripts => "scripts",
            Gate::Interchange => "interchange",
        }
    }

    /// Whether the saved state may be bound only after this gate passed (revision 6, R-2).
    pub fn binds_state(self) -> bool {
        matches!(self, Gate::Format | Gate::ModelMatch | Gate::Moves | Gate::FinalHash)
    }
}

/// One problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub code: &'static str,
    pub gate: Gate,
    /// Where: a field path such as `payload.log.history.moves.entries[3].post`, an entry or script name, or a line.
    pub at: String,
    /// For display only.
    pub message: String,
}

impl Problem {
    pub fn new(code: &'static str, gate: Gate, at: impl Into<String>, message: impl Into<String>) -> Problem {
        Problem { code, gate, at: at.into(), message: message.into() }
    }

    pub(crate) fn format(at: impl Into<String>, message: impl Into<String>) -> Problem {
        Problem::new(code::PERSISTENCE_FORMAT, Gate::Format, at, message)
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {} ({}): {}", self.code, self.at, self.gate.name(), self.message)
    }
}

impl std::error::Error for Problem {}
