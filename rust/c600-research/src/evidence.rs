//! A check job's reports in the evidence graph (EVD.46, EVD.64): the only path by which the research host writes check
//! `independent checker`.
//!
//! [`admit`] takes a [`CheckRun`], which only [`crate::job::Admitted::run`] and a matching
//! [`crate::record::reproduce`] make: its reports are the kernel checker's own, computed in this process. Each enters
//! through `c600_evidence::admit_check_report` as one leaf with basis certificate and check independent checker, the
//! checker's source digest and the certificate's digest as its identity, and the job's pinned input as its covered
//! set: a pass adds a leaf, a fail a failed leaf that is never live, a not-applicable verdict none (EVD.31). A report
//! covers the pinned input only when its checker read that start state itself: the pin is the caller's statement, the
//! bound start the checker's. A check that was cut off, cancelled or refused has no report and adds nothing: a stop is
//! no verdict and no negative (KN7.4). A report a caller holds as bytes or as a value, a stored record, an imported
//! label and a user program's assertion have no path here; the `record` sink refuses check independent checker to
//! every writer (EVD.65).

use c600_checkers::Verdict;
use c600_evidence::{
    admit_check_report, CheckReport, Covered, Covers, DigestRef, EvidenceError, Graph, Key, NodeId,
    Verdict as EvidenceVerdict,
};

use crate::job::CheckRun;

/// The covered set of a job's reports: the one instance the job was pinned at.
pub fn covered(run: &CheckRun) -> Covered {
    let input = run.input();
    let digest = match input.level.as_str() {
        "poses" => DigestRef::poses(&input.digest),
        _ => DigestRef::labels(&input.digest),
    };
    Covered::new(Covers::OneInstance, Some(digest))
}

/// EVD.46 through the job: each report of `run`, in order, as one leaf of `target`, or `None` for a check with no
/// report, a not-applicable verdict, or a fail whose checker read no start state. The job's identity block must be the
/// target scope's, and a report covers the pinned input only when its checker read that start state itself
/// ([`crate::job::Pinned::bound_by`]): a pass that read no state, or any report that read another state, is about
/// something else and refuses the admission, as EVD.18's entry check refuses a report about another input. A refusal
/// leaves `g` unchanged.
pub fn admit(g: &mut Graph, target: &Key, run: &CheckRun) -> Result<Vec<Option<NodeId>>, EvidenceError> {
    if target.scope.identity != run.input().identity_text() {
        return Err(EvidenceError::scope_mismatch("the job's input was pinned under another identity block"));
    }
    let covered = covered(run);
    let mut staged = g.clone();
    let mut leaves = Vec::new();
    for child in run.children() {
        let Some(report) = &child.report else {
            leaves.push(None);
            continue;
        };
        let bound = run.input().bound_by(report);
        let verdict = match &report.verdict {
            Verdict::NotApplicable { reason } => EvidenceVerdict::NotApplicable { reason: reason.clone() },
            _ if bound == Some(false) => {
                return Err(EvidenceError::scope_mismatch("a check read a start state other than the pinned input"));
            }
            Verdict::Pass if bound.is_none() => {
                return Err(EvidenceError::scope_mismatch("a pass that read no start state covers no pinned input"));
            }
            Verdict::Pass => EvidenceVerdict::Pass,
            Verdict::Fail { .. } if bound.is_none() => {
                leaves.push(None);
                continue;
            }
            Verdict::Fail { claim, reason, .. } => {
                EvidenceVerdict::Fail { claim: claim.clone(), reason: reason.clone() }
            }
        };
        let mapped = CheckReport {
            verdict,
            checker_id: report.checker.id.to_owned(),
            checker_source: report.checker.source.clone(),
            certificate: report.input.clone(),
            stated_inputs: covered.clone(),
            complete: report.complete(),
        };
        leaves.push(admit_check_report(&mut staged, target, &mapped)?);
    }
    *g = staged;
    Ok(leaves)
}
