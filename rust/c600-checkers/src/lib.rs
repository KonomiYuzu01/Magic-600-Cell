//! KN6's independent checkers (chapter 01 section 7; `docs/progress/1.0/language-lp5-plan.md`, pieces R2 to R6).
//!
//! Each checker is kernel code identified by its source digest (KN6.3). It recomputes letter by letter on the exact
//! engines without caches and shares no search or construction code with a producer (KN6.2). The research-archive kind
//! is a family of sub-checkers, one per claim family, each with its own identity and mutation gate (the plan's reading
//! A, adopted on 23 November 2026).
//!
//! # The check
//!
//! [`check`] is KN6.1's `check(certificate, kind) -> CheckReport`, with the exact engines it reads passed beside the
//! certificate. It first compares the SHA-256 of the certificate bytes with the digest the certificate is stated to
//! have, then runs the kind's checker, which stops at the first claim that does not hold. The [`CheckReport`] carries
//! the verdict, the checker's ID and source digest, the input digest, the stated inputs with the identities read from
//! them, and the checker's own per-claim trace. A kind with no checker, or one whose engine was not supplied, is a
//! [`Refusal`] with `KN6:evidence.no_checker`, never a report.
//!
//! # Kinds
//!
//! - [`kinds::admissibility`]: admissibility and straddlers on the jumbling puzzle (`kn6-admissibility/1`).
//! - [`kinds::macros`]: macro certificates, the archive's `macro-certificate/1` (LB5.11's scope).
//! - [`kinds::expansion`]: compressed-word expansion, streamed and length-checked before any letter is produced
//!   (`kn6-expansion/1`).
//! - [`kinds::checkpoints`]: ν and label checkpoints along a word on either puzzle (`kn6-checkpoints/1`).
//! - [`archive`]: the research-archive family, one registration point per stage (s14 to s19).
//!
//! [`gate`] is the reusable mutation gate of KN6.5: for any checker and fixture, a tampered claim, a wrong start table
//! and a wrong input digest must fail.
//!
//! # Separation (KN6.2, CT-KN6-02)
//!
//! The crate depends on `c600-kernel`, `c600-jumble` and `c600-digest` only. It reaches no interpreter, library,
//! archive reader, research host, session store or command layer, and inside the engines it calls no chooser: not
//! `c600_jumble::rotation` (exact rotation requests and their nearest-point choice), not `c600_jumble::corrections` (the
//! shipped correction constructions) and not `c600_kernel::request`. `tests/test_language_scans.py` checks this.
//!
//! # Human-solve boundary
//!
//! A checker reads a certificate's words and says whether its claims hold. Nothing here chooses, ranks or outputs a
//! solving word.

#![forbid(unsafe_code)]

pub mod archive;
pub mod claims;
pub mod gate;
pub mod kinds;
pub mod replay;
pub mod report;

use c600_jumble::Context;
use c600_kernel::Kernel;

pub use claims::{Run, Step, Stop};
pub use report::{
    codes, BoundInput, CertificateRef, CheckReport, CheckerId, CheckerKind, ClaimId, Input, Refusal, Sources, Span,
    TraceStep, Verdict,
};

use kinds::{Checker, Needs};

/// The exact engines a check may read: the 600-cell-Full kernel and the jumbling engine, each loaded from `assets/`.
#[derive(Clone, Copy, Default)]
pub struct Engines<'e> {
    pub kernel: Option<&'e Kernel>,
    pub jumble: Option<&'e Context>,
}

/// The checker of a non-archive kind.
pub fn checker(kind: CheckerKind) -> Option<&'static Checker> {
    match kind {
        CheckerKind::Admissibility => Some(&kinds::admissibility::CHECKER),
        CheckerKind::Macro => Some(&kinds::macros::CHECKER),
        CheckerKind::Expansion => Some(&kinds::expansion::CHECKER),
        CheckerKind::Checkpoints => Some(&kinds::checkpoints::CHECKER),
        CheckerKind::Archive(id) => archive::find(id).map(|sub| &sub.checker),
    }
}

/// KN6.1: checks `certificate` as a certificate of `kind` on the exact engines.
pub fn check(engines: &Engines, certificate: &CertificateRef, kind: CheckerKind) -> Result<CheckReport, Refusal> {
    let checker = checker(kind).ok_or_else(|| Refusal {
        code: codes::NO_CHECKER,
        message: match kind {
            CheckerKind::Archive(id) => format!("no research-archive sub-checker is registered as {id}"),
            other => format!("no checker for {}", other.name()),
        },
    })?;
    run_checker(engines, certificate, kind, checker)
}

/// Runs one checker on a certificate. [`check`] resolves the checker from the kind; the archive family's tests call
/// this for a sub-checker that is not registered.
pub fn run_checker(
    engines: &Engines,
    certificate: &CertificateRef,
    kind: CheckerKind,
    checker: &Checker,
) -> Result<CheckReport, Refusal> {
    let Needs { kernel, jumble } = checker.needs;
    if (kernel && engines.kernel.is_none()) || (jumble && engines.jumble.is_none()) {
        return Err(Refusal {
            code: codes::NO_CHECKER,
            message: format!("the checker for {} needs an engine that was not supplied", kind.name()),
        });
    }
    let input = c600_digest::sha256_hex(certificate.bytes);
    let mut run = Run::new(certificate);
    let verdict = if input != certificate.digest {
        let stop =
            claims::reject("input", format!("the certificate bytes have SHA-256 {input}, not the stated digest"));
        match run.failed(stop, &input) {
            Stop::Fail { claim, code, reason } => Verdict::Fail { claim, code, reason },
            Stop::NotApplicable { reason } => Verdict::NotApplicable { reason },
        }
    } else {
        run.held("input", &input);
        match (checker.run)(engines, &mut run) {
            Ok(()) => Verdict::Pass,
            Err(Stop::Fail { claim, code, reason }) => Verdict::Fail { claim, code, reason },
            Err(Stop::NotApplicable { reason }) => Verdict::NotApplicable { reason },
        }
    };
    Ok(CheckReport { verdict, checker: checker.sources.identity(), kind, input, inputs: run.inputs, trace: run.trace })
}
