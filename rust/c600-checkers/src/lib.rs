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
//! # The cooperative stop (KN7.6, KN7.10; LP5 plan, piece R8)
//!
//! [`check_until`] runs a check in a stop scope ([`claims::scoped`]): before each letter a checker produces or applies,
//! the scope's hook is asked with the units charged so far ([`claims::COST_RULE`]), and a code it answers stops the
//! check at that letter. A stopped check is [`Checked::Stopped`] with the hook's code and the units that ran: never a
//! verdict, so a long expansion under a job's budget ends as the job's cut-off. Scopes nest, and a nested check is
//! charged to every enclosing scope, so it cannot run past what they have left. A check outside every scope charges
//! nothing and runs as before. [`engines::Loaded`] owns the exact engines for a host that has no edge to the engine
//! crates.
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
pub mod engines;
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
    let checker = checker(kind).ok_or_else(|| no_checker(kind))?;
    run_checker(engines, certificate, kind, checker)
}

fn no_checker(kind: CheckerKind) -> Refusal {
    Refusal {
        code: codes::NO_CHECKER,
        message: match kind {
            CheckerKind::Archive(id) => format!("no research-archive sub-checker is registered as {id}"),
            other => format!("no checker for {}", other.name()),
        },
    }
}

/// What a check under a stop hook ended with ([`check_until`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checked {
    /// The checker reached its verdict after charging `charged` units.
    Report { report: CheckReport, charged: u64 },
    /// The hook, or an enclosing scope's hook, stopped the check before its verdict with `code`, after `charged`
    /// units. There is no verdict and no report: a stop decides no claim.
    Stopped { code: &'static str, charged: u64 },
}

/// KN6.1's check under a cooperative stop (KN7.6, KN7.10): [`check`] run in a stop scope whose hook is asked before
/// each unit of [`claims::COST_RULE`] with the units the check would have charged, and stops it with the code it
/// answers. A kind with no checker, an engine the kind needs that was not supplied, and a call from inside a hook are
/// refusals with `KN6:evidence.no_checker`, as for [`check`], which also refuses a call from inside a hook. A check
/// started inside a stopped scope is stopped with that scope's code, whether or not it reaches a letter.
pub fn check_until(
    engines: &Engines,
    certificate: &CertificateRef,
    kind: CheckerKind,
    hook: impl FnMut(u64) -> Option<&'static str> + 'static,
) -> Result<Checked, Refusal> {
    let checker = checker(kind).ok_or_else(|| no_checker(kind))?;
    let scope = claims::scoped(hook, || run_checker(engines, certificate, kind, checker)).ok_or_else(|| Refusal {
        code: codes::NO_CHECKER,
        message: "a check cannot start inside a stop hook".into(),
    })?;
    match (scope.value, scope.stopped) {
        (_, Some(code)) => Ok(Checked::Stopped { code, charged: scope.charged }),
        (Ok(report), None) => Ok(Checked::Report { report, charged: scope.charged }),
        (Err(refusal), None) => Err(refusal),
    }
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
    // Inside a stop hook a check could be neither charged nor stopped (`claims::in_hook`); outside every hook, as
    // before R8, this never refuses.
    if claims::in_hook() {
        return Err(Refusal { code: codes::NO_CHECKER, message: "a check cannot run inside a stop hook".into() });
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
    // A check whose stop scope was stopped has no verdict: its code is the scope's (KN7.10), never a report.
    if let Some(code) = claims::stopped() {
        return Err(Refusal { code, message: "the check was stopped before its verdict".into() });
    }
    Ok(CheckReport { verdict, checker: checker.sources.identity(), kind, input, inputs: run.inputs, trace: run.trace })
}
