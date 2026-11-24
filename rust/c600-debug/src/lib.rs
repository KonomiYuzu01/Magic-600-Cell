//! EV2's debugger, replay and profiler for 600 Cell Script (chapter 04 EV2.1 to EV2.12; LP4-5 and LP4-6).
//!
//! The debugger reads. It steps a checked program's evaluation or inspects a recorded run, and it has no path to a
//! write: no dependency on the command layer, the session store or the runtime (EV2.1, EV2.8; `tests/dependencies.rs`).
//! A run it steps gets its host through [`host::ReadOnly`], which refuses every preview and every sink with
//! `EV2:capability.debug_write_forbidden` before the host behind it sees them, so stepping over an effect site writes
//! nothing and emits no commit.
//!
//! - [`session`]: `debug_step(run, expected_stop, request_id) -> StepResult` over `c600-run`'s gate at the run's
//!   LG4/DET.20 positions, which [`stepper`] drives from a thread of its own with a dispatched host row as a suspended
//!   kernel call (EV2.1, EV2.12), and `inspect_value` (EV2.2, [`inspect`]).
//! - [`words`]: references to letter positions of a literal or compressed word, resolved lazily (EV2.5, EV2.11).
//! - [`replay`]: `replay_run(record)`, which preflights the record's key, re-evaluates its computation against its
//!   recorded inputs, inspects its recorded effects without executing them, and compares bytes, outcome, coverage and
//!   charges (EV2.3, EV2.4, EV2.9), with the cost profile (EV2.7, [`profile`]).
//! - [`journal`]: journal travel through the host's query rows (EV2.8) and a digest-pinned run's freshness (EV2.10).
//! - [`checker`]: stepping a checker's per-claim trace (EV2.6).
//! - [`sources`]: the source maps and bytes of an executed lowering (LG4.12, EV1.10).
//!
//! # Drafts
//!
//! Every form here is a draft that changes freely until the editor protocol and the IR freeze (PHS.49): the stop and
//! value reference texts, [`session::StepResult`]'s fields, the run record view [`replay::RunRecord`], the cost
//! profile, the journal row forms and the checker trace form. EV2.9's replay contract follows the owner's reading of
//! 23 November 2026 (replay inspects recorded effects and never re-executes them); chapters 05 and 09 do not state it
//! yet.

#![forbid(unsafe_code)]

pub mod checker;
pub mod host;
pub mod inspect;
pub mod journal;
pub mod profile;
pub mod replay;
pub mod session;
pub mod sources;
pub mod stepper;
pub mod words;

/// The codes the debugger raises (chapter 04 section 7; chapter 13).
pub mod code {
    /// A required digest, edition, IR version, run, stop, value or trace input cannot be resolved (EV2.1, EV2.3).
    pub const REPLAY_DEPENDENCY_MISSING: &str = "EV2:input.replay_dependency_missing";
    /// A literal interval or compressed occurrence path disagrees with the word's exact expansion (EV2.11).
    pub const WORD_RANGE_INVALID: &str = "EV2:input.word_range_invalid";
    /// A step request names an obsolete stop or reuses its request ID with other inputs (EV2.12).
    pub const STALE_DEBUG_STOP: &str = "EV2:stale.debug_stop";
    /// Replay under the record's key differs from the record (EV2.4).
    pub const REPLAY_MISMATCH: &str = "EV2:evidence.replay_mismatch";
    /// Debugging or replay would need a live write or an unsupported effect (EV2.1, EV2.9).
    pub const DEBUG_WRITE_FORBIDDEN: &str = "EV2:capability.debug_write_forbidden";
    /// Another model, menu identity, menu order or contract revision (EV2.3, KN1.4).
    pub const IDENTITY_MISMATCH: &str = "KN1:input.identity_mismatch";
    /// The journal event or revision is no longer held (EV2.8, KN2.2).
    pub const REVISION_GONE: &str = "KN2:stale.revision_gone";
    /// A wall-clock safety stop: the run is incomplete and not reproducible (KN7.10).
    pub const SAFETY_STOP: &str = "KN7:resource_limit.safety_stop";
}

/// A refusal of a debugger operation: its code, which decides handling, and detail text for people (EV1.12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugError {
    pub code: &'static str,
    pub detail: String,
}

impl DebugError {
    pub fn new(code: &'static str, detail: impl Into<String>) -> DebugError {
        DebugError { code, detail: detail.into() }
    }
}

impl std::fmt::Display for DebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for DebugError {}

/// KN1's identity block as debugging and replay compare it (KN0.1, KN1.4): the puzzle and model, and for the jumbling
/// puzzle its menu identity, menu-order digest and contract revision.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity {
    pub puzzle: String,
    pub model: String,
    pub menu: Option<String>,
    pub menu_order: Option<String>,
    pub contract: Option<String>,
}

impl Identity {
    /// The fields in KN1.4's order, an absent menu field as the empty text.
    pub fn fields(&self) -> [(&'static str, &str); 5] {
        fn field(f: &Option<String>) -> &str {
            f.as_deref().unwrap_or("")
        }
        [
            ("puzzle", self.puzzle.as_str()),
            ("model_id", self.model.as_str()),
            ("menu_id", field(&self.menu)),
            ("menu_order", field(&self.menu_order)),
            ("contract_rev", field(&self.contract)),
        ]
    }

    /// The first field in which `self` and `other` differ, by its name, if any.
    pub fn differs(&self, other: &Identity) -> Option<&'static str> {
        self.fields().into_iter().zip(other.fields()).find(|(a, b)| a.1 != b.1).map(|(a, _)| a.0)
    }
}

/// The pinned input a session or a record names (EV2.1, DET.1's `input`): its identity block and its level-qualified
/// digest, `Digest<level>` with the level `labels` or `poses`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pinned {
    pub identity: Identity,
    pub level: String,
    pub digest: String,
}
