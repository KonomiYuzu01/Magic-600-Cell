//! `diagnose(source, module_context)` (EV1.4): every diagnostic with a stable code, a source range and its typed
//! context, and the static verdict. Ordinary type errors, effect errors, exactness violations (LG2.22's precedence,
//! forwarded unchanged), unmet evidence obligations and model or menu identity mismatches stay apart by code. A
//! successful verdict is a candidate for activation, never an execution grant (EV1.8, EV1.11), and the service
//! changes no active module.

use c600_check::Report;
use c600_ir::{EXEC_CONTRACT, IR_VERSION};
use c600_syntax::{Lowered, EDITION};

use crate::analysis::{closure_digests, findings, identities, missing, obligations, syntax, Closure, Source};
use crate::context::ModuleContext;
use crate::document::{Document, SourceRange, Subject};
use crate::envelope::{code, Envelope, Form, NextAction, Reason, Service};

/// The stage at which a draft stopped before the checker gave a verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The bytes are not pinned source.
    Encoding,
    /// A syntax diagnostic or a recovery hole, in the draft or a dependency.
    Syntax,
    /// An import names content the context does not hold.
    Dependencies,
    /// Lowering refused the draft.
    Lowering,
}

/// What the check of exactly this source, edition, dependency closure and context covered: the data an activation's
/// check reference names (EV1.11, LP4-2). It grants nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedCandidate {
    pub source: String,
    pub module: String,
    pub edition: String,
    /// The dependency closure's digests, sorted.
    pub closure: Vec<String>,
    pub ir_version: String,
    pub exec_contract: String,
    pub context: String,
}

/// The static verdict. Only a complete, lowered and checked draft with no finding is `Checked`; nothing here invents
/// a successful check (EV1.2, EV1.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Checked(CheckedCandidate),
    /// The checker ran and found something, or the context refuses the module.
    Refused,
    NotChecked(Stage),
}

/// The answer to `diagnose`.
#[derive(Clone, Debug)]
pub struct Diagnosis {
    pub subject: Subject,
    pub diagnostics: Vec<Envelope>,
    pub verdict: Verdict,
}

/// Diagnoses the document `digest` names under a module context.
pub fn diagnose(document: &Document, digest: &str, context: &ModuleContext) -> Result<Diagnosis, Envelope> {
    let subject = document.subject(&context.edition, Some(context.digest()));
    if digest != document.digest() {
        return Err(crate::document::stale(Service::Diagnose, digest, document.digest()));
    }
    if let Some(encoding) = document.encoding() {
        let diagnostics = vec![syntax(encoding, digest, Service::Diagnose)];
        return Ok(Diagnosis { subject, diagnostics, verdict: Verdict::NotChecked(Stage::Encoding) });
    }
    if context.edition != EDITION {
        let reason = Reason::EditionUnsupported { requested: context.edition.clone(), supported: EDITION.to_owned() };
        let envelope = Envelope::new(
            code::EDITION_UNSUPPORTED,
            Form::Diagnostic,
            Service::Diagnose,
            reason,
            NextAction::SelectContext,
        )
        .at(Some(SourceRange::new(digest, 0, 0)))
        .saying("the module context names an edition this parser does not read");
        return Ok(Diagnosis { subject, diagnostics: vec![envelope], verdict: Verdict::NotChecked(Stage::Syntax) });
    }
    let closure = Closure::read(Source::read(document.bytes(), digest), context);
    Ok(run(&closure, context, subject).0)
}

/// The diagnosis of a read closure, with the lowered program and the checker's report when the check ran.
pub(crate) fn run(
    closure: &Closure,
    context: &ModuleContext,
    subject: Subject,
) -> (Diagnosis, Option<(Lowered, Report)>) {
    let mut diagnostics: Vec<Envelope> = Vec::new();
    for source in std::iter::once(&closure.root).chain(closure.deps.values()) {
        diagnostics.extend(source.file.diagnostics.iter().map(|d| syntax(d, &source.digest, Service::Diagnose)));
    }
    let syntax_failed = !diagnostics.is_empty();
    diagnostics.extend(missing(closure));
    diagnostics.extend(identities(closure, context, Service::Diagnose, Form::Diagnostic));
    let stop = |diagnostics, stage| {
        (Diagnosis { subject: subject.clone(), diagnostics, verdict: Verdict::NotChecked(stage) }, None)
    };
    if syntax_failed {
        return stop(diagnostics, Stage::Syntax);
    }
    if !closure.missing.is_empty() {
        return stop(diagnostics, Stage::Dependencies);
    }
    let (lowered, report) = match closure.check() {
        Some(Ok(checked)) => checked,
        Some(Err(refusal)) => {
            diagnostics.push(refusal);
            return stop(diagnostics, Stage::Lowering);
        }
        None => return stop(diagnostics, Stage::Syntax),
    };
    diagnostics.extend(findings(closure, &lowered, &report));
    diagnostics.extend(obligations(closure, &report, &context.obligations));
    let verdict = if diagnostics.is_empty() {
        Verdict::Checked(CheckedCandidate {
            source: closure.root.digest.clone(),
            module: closure.root.module_name().unwrap_or_default().to_owned(),
            edition: context.edition.clone(),
            closure: closure_digests(closure),
            ir_version: IR_VERSION.to_owned(),
            exec_contract: EXEC_CONTRACT.to_owned(),
            context: context.digest(),
        })
    } else {
        Verdict::Refused
    };
    (Diagnosis { subject, diagnostics, verdict }, Some((lowered, report)))
}
