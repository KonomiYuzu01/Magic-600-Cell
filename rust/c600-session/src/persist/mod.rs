//! The session store's record layer (LP2, M6's remaining rows): chapter 05's PS1 and PS2 over the frozen format
//! packet (`docs/architecture/1.0/language/format-packet/README.md`, FP.4 to FP.13, FP.19, FP.20 and FP.29).
//!
//! - [`codec`]: FP.5's canonical read, FP.7's deterministic gzip and its strict reader, FP.8's digest and blob
//!   references, and FP.9's limits table with its charge (PS1.3 to PS1.11).
//! - [`record`]: FP.4's catalogue, FP.10's dispatch and unknown-field rules, FP.11's references, and chapter 05's
//!   `read_record` and `write_record` (PS1.11 to PS1.13, PS2.2).
//! - [`word`]: FP.13's flat menu words: ia16 letters in mandatory chunks, local and external annotations, and the X3
//!   expansion bound (PS1.18 to PS1.22); a stored word's admission into the current session, under the session's whole
//!   identity block and with Jumble off for retained letters only ([`word::admit_in`]).
//! - [`facts`]: FP.19 and FP.20's facts envelope, admitted and read into `c600-evidence`'s graph (PS1.35 to PS1.37);
//!   the display word per recorded function edition and the edition note (FP.19, PS2.6).
//! - [`migrate`]: FP.29's migration record, the function-edition change, and FP.28's publication, which validates
//!   everything, writes a new generation and only then switches the active pointer (PS2.1 to PS2.9).
//! - [`legacy`]: 0.4's preview certificate (PS1.39) and its import states (PS3.7).
//! - [`pick`]: FP.38's `c600-pick` record and the resolution of FP.17's `pick` field in FP.10's order (BND.54,
//!   BND.85).
//! - [`run`]: the run record (PS1.31) with chapter 09's `ReproKey` (DET.1), its reading (DET.30, KN7.4) and the store's
//!   half of reproduction (DET.7).
//!
//! Each reader refuses with the code of PS1.42; nothing here guesses a meaning, relinks a reference or truncates a
//! word. The vectors these modules answer are read in place from the frozen packet by the crate's tests.
//!
//! # Draft choices
//!
//! The packet leaves these open; each is an M6 draft, not a frozen decision.
//!
//! - A [`Refusal`] carries its code and, for `PS1:resource_limit.persistence_size`, FP.9's row, the row's limit and
//!   the attempted amount (PS1.11); for `PS1:resource_limit.word_expansion`, the word's exact length in decimal
//!   (PS1.21).
//! - `read_record` answers `Recorded` for an unknown or newer format, version or required extension: the bytes stay as
//!   they were and never activate (PS2.2, FP.10). Every other refusal is an error.
//! - A scope's revision-bound context (EVD.10): a scope field named `revision`, `epoch`, `role`, `goal`, `protection`
//!   or `orbit`, or an `input` that is not a digest reference or null, is refused with
//!   `PS1:evidence.scope_not_immutable` at write and at read. Any other scope field outside FP.19 is
//!   `PS1:input.persistence_format`.
//! - The publication store keeps every object once under `objects/<sha256>`, a generation as an archive object that
//!   lists its records and originals, and an append-only pointer log `active` of metadata digests. A torn tail is not
//!   a switch and is cut durably before the next append; a damaged last line, or a generation that does not validate,
//!   is refused, never replaced by an older one ([`migrate`]).
//! - The X3 expansion bound is a parameter of [`word::expand`]: chapter 05 leaves its value to the owner of X3.
//! - Facts admission keeps no second evidence order: mutual negation, a leaf's identity against its scope and
//!   coverage are left to `c600-evidence`'s word function, which reads them as I2 and I7. Two OR roots with one key,
//!   an AND that reaches itself through ANDs alone, and a `complete` payload with `missing` entries are format faults.
//! - A migration has a target exactly when some item converted. A conversion that declares no semantic change and
//!   changes a preserved word field is refused: identity `KN1:input.identity_mismatch`, provenance
//!   `PS1:capability.provenance_incompatible`, any other field `PS1:input.digest_mismatch`. A converter input that
//!   changed during the conversion is `PS1:input.digest_mismatch` (R-08).
//! - An imported 0.4 certificate is state B, a recorded claim with check mode "stated: not replayed"; `conflicts`
//!   (state C) and `token` (state D) are dropped, and the other later fields stay as context.
//! - The run record is a draft of the payload FP.4 reserves as `c600-run-draft`, and `read_record` keeps it recorded.
//!   The key's parameters hold the entry and its arguments, the captured inputs, the grants, the profile and the policy
//!   identity. The captured inputs, the grants and the checkers are sets: written sorted, each once (one capture per
//!   kind and name), and compared as sets at admission. An input whose bytes change while it is captured is
//!   `PS1:input.digest_mismatch` (R-08).
//! - A draft policy identity is the pin `c600-policy-draft-1` over the sorted declassifier names. The profile and the
//!   policy identity are the run's and apply to every grant.
//! - Only a completed run or a `KN7:budget.*` cut-off is re-run: a safety stop, a cancellation, an error and a run
//!   under a clock grant give `not run`. A cut-off covers the outputs it published before its stop, and its stop
//!   boundary. A safety stop's wall-clock time is kept apart from the key and the exact payload. Evidence references
//!   are not in the payload yet. A key's identity block may be a jumbling block in free mode: no menu, a contract
//!   revision.
//! - [`word::admit_in`] takes whether a letter is retained from its caller, which computes it from the pinned menu, and
//!   refuses a jumble letter with Jumble off with the command layer's code.

pub mod codec;
pub mod facts;
pub mod legacy;
pub mod migrate;
pub mod pick;
pub mod record;
pub mod run;
pub mod word;

use std::fmt;

/// The codes this layer refuses with (PS1.42; chapter 13).
pub mod codes {
    pub const FORMAT: &str = "PS1:input.persistence_format";
    pub const VERSION: &str = "PS2:input.persistence_version";
    pub const CANON: &str = "PS1:input.noncanonical_value";
    pub const SIZE: &str = "PS1:resource_limit.persistence_size";
    pub const EXPANSION: &str = "PS1:resource_limit.word_expansion";
    pub const DIGEST: &str = "PS1:input.digest_mismatch";
    pub const SCOPE: &str = "PS1:evidence.scope_not_immutable";
    pub const PROVENANCE: &str = "PS1:capability.provenance_incompatible";
    pub const IDENTITY: &str = "KN1:input.identity_mismatch";
    /// A stored word holding a jumble letter while Jumble is off (PS1.21; the command layer's code for the same rule).
    pub const NEEDS_JUMBLE_ON: &str = "KN5:input.menu_word_needs_jumble_on";
}

/// FP.9's row, its largest admitted amount and the amount that was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limit {
    pub row: &'static str,
    pub max: u64,
    pub attempted: String,
}

/// A refusal with its PS1.42 code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: &'static str,
    /// Set for `PS1:resource_limit.persistence_size` (PS1.11).
    pub limit: Option<Limit>,
    /// Set for `PS1:resource_limit.word_expansion`: the exact expanded length, in decimal (PS1.21).
    pub length: Option<String>,
}

impl Refusal {
    pub fn new(code: &'static str) -> Refusal {
        Refusal { code, limit: None, length: None }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)?;
        if let Some(limit) = &self.limit {
            write!(f, " ({} above {} at {})", limit.attempted, limit.max, limit.row)?;
        }
        if let Some(length) = &self.length {
            write!(f, " (length {length})")?;
        }
        Ok(())
    }
}

impl std::error::Error for Refusal {}

/// Refuses with `code` unless `condition` holds.
pub(crate) fn require(condition: bool, code: &'static str) -> Result<(), Refusal> {
    if condition {
        Ok(())
    } else {
        Err(Refusal::new(code))
    }
}
