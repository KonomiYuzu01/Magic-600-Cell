//! Checker stepping (EV2.6; LP4-6).
//!
//! Checker stepping consumes KN6.4's per-claim trace: the independent checker's own steps and outcomes for each checked
//! claim. Every step names the certificate input digest and the checker source digest it binds, the claim, the checked
//! inputs and the outcome, and links to word positions where they matter ([`crate::words::WordRef`]). The trace is the
//! checker's output, stored or re-derived on demand by whoever holds it ([`Traces`]); this crate imports no producer,
//! search or construction code (KN6.2, KN6.4; `tests/dependencies.rs`).
//!
//! A trace that cannot be resolved is unavailable, `EV2:input.replay_dependency_missing`, and never a passed check; so
//! is a trace that binds other certificate or checker digests than the ones asked for. A trace whose steps disagree with
//! the digests it binds is refused the same way: no step of it is shown.
//!
//! # Drafts
//!
//! KN6.4 fixes the trace's content, not its form. [`Trace`] is the debugger's reading of it; no checker in this
//! workspace writes a per-claim trace yet, so CT-EV2-07 runs here over a trace built by its test.

use crate::code::REPLAY_DEPENDENCY_MISSING;
use crate::words::WordRef;
use crate::DebugError;

/// One claim's step of a per-claim trace (KN6.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimStep {
    /// The certificate input digest and checker source digest the step binds.
    pub certificate: String,
    pub checker: String,
    /// The claim's name in the certificate.
    pub claim: String,
    /// The checked inputs, by digest or name.
    pub inputs: Vec<String>,
    /// `holds`, `fails` or `unestablished`, as the checker wrote it.
    pub outcome: String,
    /// The word positions the step cites.
    pub words: Vec<WordRef>,
}

/// KN6.4's per-claim trace of one checker run over one certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    pub certificate: String,
    pub checker: String,
    pub steps: Vec<ClaimStep>,
}

/// Whoever holds checker traces: a store of the checker's outputs, or a host that re-derives one by running the checker
/// as budgeted work that cannot modify the puzzle. `None` means it holds none for the pair.
pub trait Traces {
    fn trace(&self, certificate: &str, checker: &str) -> Option<Trace>;
}

/// Stepping through one trace.
#[derive(Debug)]
pub struct Stepping {
    trace: Trace,
    next: usize,
}

fn unavailable(detail: String) -> DebugError {
    DebugError::new(REPLAY_DEPENDENCY_MISSING, detail)
}

impl Stepping {
    /// Opens the trace of `checker` over `certificate` from `traces` (see the module's description).
    pub fn open(traces: &dyn Traces, certificate: &str, checker: &str) -> Result<Stepping, DebugError> {
        let trace = traces.trace(certificate, checker).ok_or_else(|| {
            unavailable(format!(
                "no per-claim trace of checker {checker} over certificate {certificate}: unavailable, not a passed check (EV2.6)"
            ))
        })?;
        if trace.certificate != certificate || trace.checker != checker {
            return Err(unavailable(format!(
                "the trace binds certificate {} and checker {}, not the ones asked for (EV2.6)",
                trace.certificate, trace.checker
            )));
        }
        if let Some(i) = trace.steps.iter().position(|s| s.certificate != certificate || s.checker != checker) {
            return Err(unavailable(format!("step {i} of the trace binds other digests than its trace (EV2.6)")));
        }
        Ok(Stepping { trace, next: 0 })
    }

    /// The next claim's step, or `None` after the last.
    pub fn step(&mut self) -> Option<&ClaimStep> {
        let step = self.trace.steps.get(self.next)?;
        self.next += 1;
        Some(step)
    }

    /// The step of claim number `index`, without moving.
    pub fn at(&self, index: usize) -> Option<&ClaimStep> {
        self.trace.steps.get(index)
    }

    pub fn trace(&self) -> &Trace {
        &self.trace
    }
}
