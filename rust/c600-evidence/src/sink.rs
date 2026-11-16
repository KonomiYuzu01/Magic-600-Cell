//! The checked entry paths: EVD.18's entry check, EVD.64 and EVD.65's writer check at the `record` sink, EVD.46's
//! `CheckReport` admission and EVD.4's reused leaf. A refused call leaves the graph unchanged.

use crate::error::EvidenceError;
use crate::graph::{Graph, NodeId};
use crate::model::{Basis, Check, CheckFact, CheckMode, Covered, Covers, Form, Grade, Key, Leaf, LeafRole, Quantifier};

/// EVD.64's writers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Writer {
    /// The kernel (KN6). It writes `independent checker` only through `admit_check_report`.
    Kernel,
    /// The host.
    Host,
    /// An import of records on file (section 8). Draft: it writes as the host does, under the same conditions.
    Import,
    /// A user program, through the `record` sink.
    UserProgram,
}

/// What a write cites (EVD.64's conditions).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Citation {
    /// The cited record, for basis `theorem`.
    pub record: Option<String>,
    /// The commit the cited record is pinned to.
    pub commit: Option<String>,
    /// The digest of the cited review record, for check `model review` or `human review`.
    pub review_digest: Option<String>,
}

impl Citation {
    pub fn none() -> Citation {
        Citation::default()
    }

    pub fn theorem(record: &str, commit: &str) -> Citation {
        Citation { record: Some(record.to_string()), commit: Some(commit.to_string()), review_digest: None }
    }

    pub fn review(digest: &str) -> Citation {
        Citation { record: None, commit: None, review_digest: Some(digest.to_string()) }
    }

    pub fn with_review(mut self, digest: &str) -> Citation {
        self.review_digest = Some(digest.to_string());
        self
    }
}

fn lower_hex(s: &str, lens: &[usize]) -> bool {
    lens.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Draft: a commit pin is a Git object name, 40 or 64 lowercase hexadecimal digits.
fn is_commit(s: &str) -> bool {
    lower_hex(s, &[40, 64])
}

/// Draft: a digested review record is named by its SHA-256, 64 lowercase hexadecimal digits.
fn is_review_digest(s: &str) -> bool {
    lower_hex(s, &[64])
}

/// EVD.64 and EVD.65: whether `writer` may set `grade` at the `record` sink. Refusal is
/// `EVD:capability.field_not_writable`.
pub fn check_write(writer: Writer, grade: &Grade, citation: &Citation) -> Result<(), EvidenceError> {
    let refuse = |why: &str| Err(EvidenceError::field_not_writable(why));
    if !grade.is_known() {
        return refuse("a value outside the enumeration has no writer");
    }
    if grade.check == Check::FormalProof {
        return refuse("check formal proof has no writer in 1.0");
    }
    if grade.check == Check::IndependentChecker {
        return refuse("check independent checker comes only from a kernel checker's own report (EVD.46)");
    }
    if writer == Writer::UserProgram {
        let basis_ok = matches!(grade.basis, Basis::Sample | Basis::ExactComputation);
        let check_ok = matches!(grade.check, Check::None | Check::Producer);
        if !(basis_ok && check_ok) {
            return refuse("a user program writes at most basis exact computation with check producer");
        }
        return Ok(());
    }
    if grade.basis == Basis::Theorem {
        if !matches!(writer, Writer::Host | Writer::Import) {
            return refuse("basis theorem is written by the host");
        }
        let pinned = citation.record.as_deref().is_some_and(|r| !r.is_empty())
            && citation.commit.as_deref().is_some_and(is_commit);
        if !pinned {
            return refuse("basis theorem cites a record pinned to a commit");
        }
    }
    if matches!(grade.check, Check::ModelReview | Check::HumanReview) {
        if !matches!(writer, Writer::Host | Writer::Import) {
            return refuse("a model or human review is written by the host");
        }
        if !citation.review_digest.as_deref().is_some_and(is_review_digest) {
            return refuse("a model or human review cites a digested review record");
        }
    }
    Ok(())
}

/// EVD.18: the entry check of a leaf offered as a new argument of `target`. A leaf for another key, with other
/// identity fields or about another input is offered outside its immutable scope, and so is a leaf that would meet I7
/// (a) or (b). Draft: a target with no `form` fact counts as universal here, as it does for I7 (a) (EVD.41).
pub fn entry_check(g: &Graph, target: &Key, leaf: &Leaf) -> Result<(), EvidenceError> {
    let refuse = |why: &str| Err(EvidenceError::scope_mismatch(why));
    if leaf.key != *target {
        return refuse("the leaf was recorded for another key");
    }
    if leaf.identity != target.scope.identity {
        return refuse("the leaf's identity fields differ from the scope");
    }
    if let Some(c) = &leaf.covered {
        if c.input.is_some() && c.input != target.scope.input {
            return refuse("the leaf is about another input; transport needs a lemma (EVD.18)");
        }
        let computed =
            matches!(leaf.grade.basis, Basis::Certificate | Basis::ExhaustiveComputation | Basis::ExactComputation);
        if matches!(leaf.grade.basis, Basis::ExhaustiveComputation | Basis::ExactComputation)
            && !c.covers.is_finite_kind()
        {
            return refuse("the leaf would meet I7 (b)");
        }
        let form = g.statement(&target.statement).and_then(|s| s.form.clone());
        let read_by_i7a = matches!(form, None | Some(Form::Universal) | Some(Form::Mixed(Quantifier::Forall)));
        if computed && read_by_i7a && *c != Covered::of_scope(&target.scope) {
            return refuse("the leaf would meet I7 (a): its covered set is not the statement's scope");
        }
    }
    Ok(())
}

/// The checked path for a leaf with no writer check (a kernel-side or test caller): EVD.18's entry check, then a new OR
/// child.
pub fn offer(g: &mut Graph, target: &Key, leaf: Leaf) -> Result<NodeId, EvidenceError> {
    entry_check(g, target, &leaf)?;
    Ok(g.support(target, leaf))
}

/// The `record` sink: EVD.65's writer check, then EVD.18's entry check, then a new OR child. A refused write adds no
/// leaf.
pub fn record(
    g: &mut Graph,
    writer: Writer,
    target: &Key,
    leaf: Leaf,
    citation: &Citation,
) -> Result<NodeId, EvidenceError> {
    check_write(writer, &leaf.grade, citation)?;
    for m in &leaf.membership {
        check_write(writer, &m.check, citation)?;
    }
    offer(g, target, leaf)
}

/// KN6.1's verdicts, as EVD.46 reads them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail { claim: String, reason: String },
    NotApplicable { reason: String },
}

/// The parts of a KN6.1 `CheckReport` that EVD.46 reads. Produced by a kernel checker, never by a user program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckReport {
    pub verdict: Verdict,
    pub checker_id: String,
    /// The checker's source digest.
    pub checker_source: String,
    /// The certificate's byte digest.
    pub certificate: String,
    /// The certificate's stated inputs (KN6.6, EVD.59).
    pub stated_inputs: Covered,
    /// Draft: the checker states the certificate complete for its stated inputs (EVD.42's "complete certificate").
    pub complete: bool,
}

/// EVD.46: a `CheckReport` enters the graph as one leaf of basis certificate and check independent checker, with the
/// checker's source digest and the certificate's byte digest as its identity. `fail` keeps a failed leaf, which is
/// never live (EVD.31), and `not_applicable` adds none.
pub fn admit_check_report(g: &mut Graph, target: &Key, report: &CheckReport) -> Result<Option<NodeId>, EvidenceError> {
    let mut leaf = Leaf::new(target, Grade::new(Basis::Certificate, Check::IndependentChecker))
        .covered(report.stated_inputs.clone())
        .support(&report.checker_source)
        .support(&report.certificate)
        .check_fact(CheckFact {
            tool: report.checker_source.clone(),
            common_base: Vec::new(),
            kind: report.checker_id.clone(),
            mode: CheckMode::FreshExactAudit,
            transport: None,
        });
    leaf.complete_certificate = report.complete;
    match &report.verdict {
        Verdict::NotApplicable { .. } => Ok(None),
        Verdict::Pass => Ok(Some(offer(g, target, leaf)?)),
        Verdict::Fail { .. } => {
            leaf.failed = true;
            Ok(Some(offer(g, target, leaf)?))
        }
    }
}

/// EVD.4 and EVD.18: reuse a leaf recorded about input X for `target` about input Z through a transport lemma leaf
/// that names the checked relation Z = X.gamma. The result is AND(source leaf, transport leaf) as a new OR child.
pub fn reuse(g: &mut Graph, target: &Key, source: Leaf, transport: Leaf) -> Result<NodeId, EvidenceError> {
    let LeafRole::Transport(rel) = &transport.role else {
        return Err(EvidenceError::scope_mismatch("a reused leaf needs a transport lemma leaf with its relation"));
    };
    let from = source.covered.as_ref().and_then(|c| c.input.clone()).or_else(|| source.key.scope.input.clone());
    if from.as_ref() != Some(&rel.from) || target.scope.input.as_ref() != Some(&rel.to) {
        return Err(EvidenceError::scope_mismatch("the checked relation does not map the source input to the target"));
    }
    if source.identity != target.scope.identity || transport.identity != target.scope.identity {
        return Err(EvidenceError::scope_mismatch("the identity fields differ from the scope"));
    }
    if let Some(c) = &source.covered {
        let moved = Covered::new(c.covers.clone(), Some(rel.to.clone()));
        let form = g.statement(&target.statement).and_then(|s| s.form.clone());
        let computed =
            matches!(source.grade.basis, Basis::Certificate | Basis::ExhaustiveComputation | Basis::ExactComputation);
        let read_by_i7a = matches!(form, None | Some(Form::Universal) | Some(Form::Mixed(Quantifier::Forall)));
        if computed && read_by_i7a && moved != Covered::of_scope(&target.scope) {
            return Err(EvidenceError::scope_mismatch("the transported leaf would meet I7 (a)"));
        }
        if matches!(source.grade.basis, Basis::ExhaustiveComputation | Basis::ExactComputation)
            && !matches!(c.covers, Covers::OneInstance | Covers::SearchSet { .. })
        {
            return Err(EvidenceError::scope_mismatch("the transported leaf would meet I7 (b)"));
        }
    }
    let s = g.add_leaf(source);
    let t = g.add_leaf(transport);
    let and = g.add_and(&[s, t])?;
    g.add_or_child(target, and)?;
    Ok(and)
}
