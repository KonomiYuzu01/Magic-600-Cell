//! The shared reading of a draft and its pinned dependency graph: parsing every file of the closure, lowering and
//! checking them with the product front end (`c600_syntax::lower`, `c600_check::check`), and turning each finding into
//! an envelope with its source range. Nothing here runs a program (EV1.9).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use c600_check::{check, Report};
use c600_ir::ExactRole;
use c600_syntax::ast::{Item, ParsedFile, Trivia};
use c600_syntax::{lexemes, lower, parse, Diagnostic, Lexeme, Lowered, Span};

use crate::context::{ModelIdentity, ModuleContext, Obligation};
use crate::document::{Document, SourceRange};
use crate::envelope::{code, Envelope, Form, IdentitySubject, NextAction, Reason, Service};

/// A parsed file and, lexed when first asked for, its tokens and trivia (none when the bytes are not UTF-8).
pub(crate) struct Source {
    pub digest: String,
    pub bytes: Vec<u8>,
    pub file: Arc<ParsedFile>,
    lexed: OnceLock<(Vec<Lexeme>, Vec<Trivia>)>,
}

impl Source {
    pub fn read(bytes: &[u8], digest: &str) -> Source {
        let file = Arc::new(parse(bytes));
        Source { digest: digest.to_owned(), bytes: bytes.to_vec(), file, lexed: OnceLock::new() }
    }

    /// A document's bytes read as [`Source::read`] reads them, with the parse the document shares between the
    /// services that ask about it.
    pub fn of(document: &Document, digest: &str) -> Source {
        let bytes = document.bytes().to_vec();
        Source { digest: digest.to_owned(), bytes, file: document.parsed(), lexed: OnceLock::new() }
    }

    fn lexed(&self) -> &(Vec<Lexeme>, Vec<Trivia>) {
        self.lexed.get_or_init(|| std::str::from_utf8(&self.bytes).map(lexemes).unwrap_or_default())
    }

    pub fn lexemes(&self) -> &[Lexeme] {
        &self.lexed().0
    }

    pub fn trivia(&self) -> &[Trivia] {
        &self.lexed().1
    }

    pub fn range(&self, span: Span) -> SourceRange {
        SourceRange::new(self.digest.clone(), span.start, span.end)
    }

    pub fn module_name(&self) -> Option<&str> {
        self.file.module.as_ref().map(|m| m.name.text.as_str())
    }

    /// The module header's range, or the empty range at the file's start when the header did not parse.
    pub fn header(&self) -> SourceRange {
        match &self.file.module {
            Some(m) => self.range(m.loc.0),
            None => SourceRange::new(self.digest.clone(), 0, 0),
        }
    }
}

/// An import of the root that the context cannot satisfy.
pub(crate) struct Missing {
    pub alias: String,
    pub digest: String,
    /// The root's import through which the missing content is reached.
    pub span: Span,
}

/// The root draft and the dependencies its imports reach, transitively, among the context's bytes.
pub(crate) struct Closure {
    pub root: Source,
    /// Dependencies by digest.
    pub deps: BTreeMap<String, Source>,
    pub missing: Vec<Missing>,
}

impl Closure {
    pub fn read(root: Source, context: &ModuleContext) -> Closure {
        let mut closure = Closure { root, deps: BTreeMap::new(), missing: Vec::new() };
        let roots: Vec<(String, String, Span)> = imports(&closure.root.file);
        for (alias, digest, span) in roots {
            let mut pending = vec![digest.clone()];
            while let Some(d) = pending.pop() {
                if closure.deps.contains_key(&d) {
                    continue;
                }
                let Some(dep) = context.dependency(&d) else {
                    if !closure.missing.iter().any(|m| m.digest == d) {
                        closure.missing.push(Missing { alias: alias.clone(), digest: d.clone(), span });
                    }
                    continue;
                };
                let source = Source::read(&dep.bytes, &d);
                pending.extend(imports(&source.file).into_iter().map(|(_, digest, _)| digest));
                closure.deps.insert(d, source);
            }
        }
        closure
    }

    /// The source with this digest: the root or a dependency.
    pub fn source(&self, digest: &str) -> Option<&Source> {
        if self.root.digest == digest {
            Some(&self.root)
        } else {
            self.deps.get(digest)
        }
    }

    /// The source declaring the module `name`.
    pub fn module(&self, name: &str) -> Option<&Source> {
        std::iter::once(&self.root).chain(self.deps.values()).find(|s| s.module_name() == Some(name))
    }

    /// Whether every file parsed cleanly and every import is held, so that lowering can run.
    pub fn complete(&self) -> bool {
        self.missing.is_empty() && std::iter::once(&self.root).chain(self.deps.values()).all(clean)
    }

    /// Lowers and checks the whole closure, root included.
    pub fn check(&self) -> Option<Result<(Lowered, Report), Envelope>> {
        if !self.complete() {
            return None;
        }
        let files: Vec<ParsedFile> =
            std::iter::once(&self.root).chain(self.deps.values()).map(|s| ParsedFile::clone(&s.file)).collect();
        Some(self.lower_and_check(&files))
    }

    /// Lowers and checks the dependencies alone, for what a hover on a dependency's declaration shows while the root
    /// draft is broken.
    pub fn check_dependencies(&self) -> Option<(Lowered, Report)> {
        if self.deps.is_empty() || !self.deps.values().all(clean) {
            return None;
        }
        let files: Vec<ParsedFile> = self.deps.values().map(|s| ParsedFile::clone(&s.file)).collect();
        self.lower_and_check(&files).ok()
    }

    fn lower_and_check(&self, files: &[ParsedFile]) -> Result<(Lowered, Report), Envelope> {
        let lowered = lower(files).map_err(|e| {
            let source = e.module.as_deref().and_then(|m| self.module(m)).unwrap_or(&self.root);
            let span = e.span.map(|s| source.range(s)).unwrap_or_else(|| source.header());
            Envelope::new(
                code::IR_REJECTED,
                Form::Diagnostic,
                Service::Diagnose,
                Reason::Lowering { module: e.module.clone() },
                NextAction::EditSource,
            )
            .at(Some(span))
            .saying(e.message.clone())
        })?;
        let report = check(&lowered.program).map_err(|e| {
            Envelope::new(
                code::IR_REJECTED,
                Form::Diagnostic,
                Service::Diagnose,
                Reason::Lowering { module: None },
                NextAction::EditSource,
            )
            .at(Some(self.root.header()))
            .saying(e.to_string())
        })?;
        Ok((lowered, report))
    }

    /// The source range of a checker finding: the IR node's range from the source map, else its declaration, else
    /// its module's header (LG4.12).
    pub fn finding_range(&self, lowered: &Lowered, module: &str, function: &str, path: &str) -> SourceRange {
        let source = self.module(module).unwrap_or(&self.root);
        let map = lowered.source(module);
        let span = map
            .and_then(|m| m.span(function, path))
            .or_else(|| map.and_then(|m| m.declarations.get(function).copied()));
        span.map(|s| source.range(s)).unwrap_or_else(|| source.header())
    }
}

fn clean(s: &Source) -> bool {
    s.file.diagnostics.is_empty() && s.file.holes.is_empty() && s.file.digest.is_some()
}

/// A file's imports: alias, digest and the import's range.
fn imports(file: &ParsedFile) -> Vec<(String, String, Span)> {
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Import(i) => Some((i.alias.text.clone(), i.digest.clone(), i.loc.0)),
            Item::Decl(_) => None,
        })
        .collect()
}

/// A syntax diagnostic as an envelope, its code forwarded unchanged (ERR.7).
pub(crate) fn syntax(d: &Diagnostic, digest: &str, service: Service) -> Envelope {
    let (reason, next) = if let Some(raw) = &d.raw {
        (Reason::Encoding { raw: raw.clone() }, NextAction::Reencode)
    } else if d.code == code::RECOVERY_GAP {
        (Reason::RecoveryGap, NextAction::EditSource)
    } else {
        (Reason::Syntax, NextAction::EditSource)
    };
    let span = d.span.map(|s| SourceRange::new(digest, s.start, s.end));
    Envelope::new(d.code, Form::Diagnostic, service, reason, next).at(span).saying(d.message.clone())
}

/// The checker's findings as envelopes, each forwarded unchanged with its typed context and range (EV1.4, ERR.7).
pub(crate) fn findings(closure: &Closure, lowered: &Lowered, report: &Report) -> Vec<Envelope> {
    report
        .diagnostics
        .iter()
        .map(|d| {
            let span = closure.finding_range(lowered, &d.module, &d.function, &d.path);
            let reason = Reason::Check {
                module: d.module.clone(),
                function: d.function.clone(),
                path: d.path.clone(),
                value_type: d.value_type.clone(),
                display_control: d.display_control,
                qualifier_erasure: d.qualifier_erasure,
                expected: d.mismatch.as_ref().map(|m| m.expected.clone()),
                supplied: d.mismatch.as_ref().map(|m| m.supplied.clone()),
            };
            Envelope::new(d.code, Form::Diagnostic, Service::Diagnose, reason, NextAction::EditSource)
                .at(Some(span))
                .saying(d.note.clone())
        })
        .collect()
}

/// The missing imports as envelopes: nothing stands in for absent content (LG5.3).
pub(crate) fn missing(closure: &Closure) -> Vec<Envelope> {
    closure
        .missing
        .iter()
        .map(|m| {
            Envelope::new(
                code::DEPENDENCY_MISSING,
                Form::Diagnostic,
                Service::Diagnose,
                Reason::DependencyMissing { alias: m.alias.clone(), digest: m.digest.clone() },
                NextAction::SupplyDependency { digest: m.digest.clone() },
            )
            .at(Some(closure.root.range(m.span)))
            .saying("the module context holds no bytes with the imported digest")
        })
        .collect()
}

/// The first differing field between a session identity and the nearest claimed identity, with both values, or
/// `None` when the claims include the session's identity. The nearest claim agrees on the most leading fields.
pub(crate) fn identity_difference(
    session: &ModelIdentity,
    supports: &[ModelIdentity],
) -> Option<(crate::context::IdentityField, Option<String>, Option<String>)> {
    if supports.contains(session) {
        return None;
    }
    let nearest = supports.iter().max_by_key(|s| session.first_difference(s));
    let Some(claim) = nearest else {
        let field = crate::context::IdentityField::Puzzle;
        return Some((field, session.field(field).map(str::to_owned), None));
    };
    let field = session.first_difference(claim)?;
    Some((field, session.field(field).map(str::to_owned), claim.field(field).map(str::to_owned)))
}

/// Model or menu identity mismatches (KN1.4, LG5.5): the draft's claims at its header, and each pinned dependency's
/// claims at the root's import of it.
pub(crate) fn identities(closure: &Closure, context: &ModuleContext, service: Service, form: Form) -> Vec<Envelope> {
    let Some(session) = &context.session else { return Vec::new() };
    let mut found = Vec::new();
    let mismatch = |subject, field, expected, held| {
        Envelope::new(
            code::IDENTITY_MISMATCH,
            form,
            service,
            Reason::IdentityMismatch { subject, field, expected, held },
            NextAction::SelectContext,
        )
        .saying("the claimed model identity differs from the session's; nothing is reinterpreted")
    };
    if let Some(supports) = &context.supports {
        if let Some((field, expected, held)) = identity_difference(session, supports) {
            found.push(mismatch(IdentitySubject::Module, field, expected, held).at(Some(closure.root.header())));
        }
    }
    for (alias, digest, span) in imports(&closure.root.file) {
        let Some(supports) = context.dependency(&digest).and_then(|d| d.supports.as_ref()) else { continue };
        if let Some((field, expected, held)) = identity_difference(session, supports) {
            let subject = IdentitySubject::Dependency { alias, digest };
            found.push(mismatch(subject, field, expected, held).at(Some(closure.root.range(span))));
        }
    }
    found
}

/// The obligations the root does not meet (EV1.4): a declaration that is absent, or whose checked summary supplies
/// no exact evidence sink.
pub(crate) fn obligations(closure: &Closure, report: &Report, required: &[Obligation]) -> Vec<Envelope> {
    let Some(module) = closure.root.module_name() else { return Vec::new() };
    let mut found = Vec::new();
    for o in required {
        let decl = closure.root.file.declarations().find(|d| d.name.text == o.declaration);
        let supplied = report
            .summary(module, &o.declaration)
            .is_some_and(|s| s.exact_sinks.iter().any(|(role, _)| *role == ExactRole::Evidence));
        if supplied {
            continue;
        }
        let span = decl.map(|d| closure.root.range(d.name.loc.0)).unwrap_or_else(|| closure.root.header());
        found.push(
            Envelope::new(
                code::OBLIGATION_UNMET,
                Form::Diagnostic,
                Service::Diagnose,
                Reason::ObligationUnmet { declaration: o.declaration.clone(), role: o.role, declared: decl.is_some() },
                NextAction::SupplyObligation { declaration: o.declaration.clone() },
            )
            .at(Some(span))
            .saying("the module context requires evidence that the declaration does not supply"),
        );
    }
    found
}

/// The digests of a checked closure's dependencies, sorted.
pub(crate) fn closure_digests(closure: &Closure) -> Vec<String> {
    let set: BTreeSet<&String> = closure.deps.keys().collect();
    set.into_iter().cloned().collect()
}
