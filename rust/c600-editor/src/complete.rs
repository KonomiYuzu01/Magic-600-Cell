//! `complete(source, position, module_context)` (EV1.3). Completion reads the recovered scope at the position, the
//! declared types and effects, and the selected module context. Each candidate carries the source range it would
//! replace, its declared type and effects, its identity and a documentation reference. A suggestion grants nothing and
//! makes no command available, and context the draft lacks stays explicit in [`Completion::missing`] instead of being
//! borrowed from an active tool. Candidates come in a neutral order, by label; nothing ranks them.

use std::collections::BTreeSet;

use c600_syntax::ast::{Decl, TriviaKind, TypeExpr};
use c600_syntax::{
    base_types, builtins, capability_names, effect_names, lexemes, orders, precisions, type_constructors, type_text,
    Lexeme, LexemeKind,
};

use crate::analysis::{identity_difference, Closure, Source};
use crate::context::ModuleContext;
use crate::document::{Document, SourceRange, Subject};
use crate::envelope::{code, Envelope, Form, IdentitySubject, NextAction, Reason, Service};
use crate::index::{Index, NameKind, Owner};

/// What kind of text the position is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionContext {
    /// Inside a comment.
    Comment,
    /// Inside a string.
    Text,
    /// Inside a literal word, menu word or charge: numbers only.
    Literal,
    /// Where a declaration starts.
    Declaration,
    /// In a declaration's head after its parameters.
    Signature,
    /// Where a new name is written: after `let`, a declaration keyword or `import`, a parameter, a generic or a
    /// record field.
    NewName,
    Type,
    /// An `effects` list.
    Effects,
    /// A `requires` list.
    Capabilities,
    /// After `alias.`: the exported declarations of the module the alias imports.
    Member {
        alias: String,
        digest: String,
    },
    /// After `value.`: a record field.
    Field,
    Expression,
}

/// What a candidate is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateKind {
    Keyword,
    Builtin,
    Order,
    Type,
    Effect,
    Capability,
    Local(NameKind),
    Declaration,
    Import,
    /// An exported declaration of a dependency.
    Member,
    Field,
}

/// The identity of a declaration: the digest of the source that declares it, its module and its name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclIdentity {
    pub digest: String,
    pub module: String,
    pub name: String,
}

/// One suggestion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub label: String,
    pub kind: CandidateKind,
    /// The range the candidate would replace.
    pub replace: SourceRange,
    /// The declared type, or a declaration's signature.
    pub ty: Option<String>,
    /// A declaration's declared effects.
    pub effects: Option<Vec<String>>,
    pub identity: Option<DeclIdentity>,
    /// A documentation reference: `keyword:<name>`, `builtin:<name>`, `type:<name>`, `effect:<name>` or
    /// `decl:<digest>:<name>`. The documents themselves are LP4-10's.
    pub doc: Option<String>,
}

/// Context the draft lacks, kept explicit (EV1.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    /// The alias imports bytes the module context does not hold.
    Dependency { alias: String, digest: String },
    /// The value's type is not declared where the draft can read it, so its fields are unknown.
    FieldTypes,
}

/// The answer to `complete`.
#[derive(Clone, Debug)]
pub struct Completion {
    pub subject: Subject,
    pub context: CompletionContext,
    /// The identifier the position touches, or the empty range at the position.
    pub replace: SourceRange,
    /// The part of that identifier before the position, which every candidate's label starts with.
    pub prefix: String,
    pub candidates: Vec<Candidate>,
    pub missing: Vec<Missing>,
}

const DECLARATION_KEYWORDS: [&str; 6] = ["command", "contract", "export", "fn", "import", "view"];
const SIGNATURE_KEYWORDS: [&str; 2] = ["charge", "effects"];
const EXPRESSION_KEYWORDS: [&str; 8] = ["false", "fn", "match", "menu", "q5", "record", "true", "word"];
const STATEMENT_KEYWORDS: [&str; 2] = ["let", "return"];

/// Completes at a position in the document's declared units, for a request naming `digest`.
pub fn complete(
    document: &Document,
    digest: &str,
    position: usize,
    context: &ModuleContext,
) -> Result<Completion, Envelope> {
    let at = document.byte_offset(digest, position, Service::Complete)?;
    let text = document.expect(digest, Service::Complete)?;
    let subject = document.subject(&context.edition, Some(context.digest()));
    let closure = Closure::read(Source::read(text.as_bytes(), digest), context);
    if let (Some(session), Some(supports)) = (&context.session, &context.supports) {
        if let Some((field, expected, held)) = identity_difference(session, supports) {
            return Err(identity_refusal(IdentitySubject::Module, field, expected, held));
        }
    }
    let index = Index::build(&closure.root.file, &closure.root.lexemes);
    let (tokens, trivia) = lexemes(text);
    let range = |s: usize, e: usize| SourceRange::new(digest, s, e);

    let ident = tokens.iter().find(|l| l.kind == LexemeKind::Name && l.span.start <= at && at <= l.span.end);
    let (start, end) = ident.map_or((at, at), |l| (l.span.start, l.span.end));
    let prefix = text[start..at].to_owned();
    let mut completion = Completion {
        subject,
        context: CompletionContext::Expression,
        replace: range(start, end),
        prefix: prefix.clone(),
        candidates: Vec::new(),
        missing: Vec::new(),
    };

    let in_comment = trivia.iter().any(|t| t.kind == TriviaKind::Comment && t.span.start < at && at <= t.span.end);
    if in_comment {
        completion.context = CompletionContext::Comment;
        return Ok(completion);
    }
    if in_text(text, &tokens, at) {
        completion.context = CompletionContext::Text;
        return Ok(completion);
    }
    let before: Vec<&Lexeme> = tokens.iter().filter(|l| l.kind != LexemeKind::End && l.span.end <= start).collect();
    let owner = index.owner_at(at);
    completion.context = context_at(&before, &index, owner);

    let mut found: Vec<Candidate> = Vec::new();
    let replace = range(start, end);
    let simple = |label: &str, kind: CandidateKind, doc: &str| Candidate {
        label: label.to_owned(),
        kind,
        replace: replace.clone(),
        ty: None,
        effects: None,
        identity: None,
        doc: Some(format!("{doc}:{label}")),
    };
    match completion.context.clone() {
        CompletionContext::Comment
        | CompletionContext::Text
        | CompletionContext::Literal
        | CompletionContext::NewName => {}
        CompletionContext::Declaration => {
            let after_export = before.last().is_some_and(|l| l.text == "export");
            for k in DECLARATION_KEYWORDS.iter().filter(|k| !after_export || !matches!(**k, "export" | "import")) {
                found.push(simple(k, CandidateKind::Keyword, "keyword"));
            }
        }
        CompletionContext::Signature => {
            for k in SIGNATURE_KEYWORDS {
                found.push(simple(k, CandidateKind::Keyword, "keyword"));
            }
        }
        CompletionContext::Type => {
            let approx =
                innermost(&before).is_some_and(|(open, i)| open == "<" && i > 0 && before[i - 1].text == "Approx");
            if approx {
                for p in precisions() {
                    found.push(simple(p, CandidateKind::Type, "type"));
                }
            } else {
                for t in base_types().iter().chain(type_constructors()) {
                    found.push(simple(t, CandidateKind::Type, "type"));
                }
                for i in index.visible(at, owner) {
                    if index.bindings[i].kind == NameKind::Generic {
                        found.push(local(&index, i, &replace));
                    }
                }
            }
        }
        CompletionContext::Effects => {
            for e in effect_names() {
                found.push(simple(e, CandidateKind::Effect, "effect"));
            }
            for c in capability_names().iter().filter(|c| !effect_names().contains(c)) {
                found.push(simple(c, CandidateKind::Capability, "capability"));
            }
            for i in index.visible(at, owner) {
                if index.bindings[i].kind == NameKind::Generic {
                    found.push(local(&index, i, &replace));
                }
            }
        }
        CompletionContext::Capabilities => {
            for c in capability_names() {
                found.push(simple(c, CandidateKind::Capability, "capability"));
            }
        }
        CompletionContext::Member { alias, digest: dep } => match closure.deps.get(&dep) {
            None => completion.missing.push(Missing::Dependency { alias, digest: dep }),
            Some(source) => {
                if let (Some(session), Some(supports)) =
                    (&context.session, context.dependency(&dep).and_then(|d| d.supports.as_ref()))
                {
                    if let Some((field, expected, held)) = identity_difference(session, supports) {
                        let subject = IdentitySubject::Dependency { alias, digest: dep };
                        return Err(identity_refusal(subject, field, expected, held));
                    }
                }
                let module = source.module_name().unwrap_or_default().to_owned();
                for d in source.file.declarations().filter(|d| d.exported) {
                    found.push(Candidate {
                        label: d.name.text.clone(),
                        kind: CandidateKind::Member,
                        replace: replace.clone(),
                        ty: Some(signature(d)),
                        effects: Some(d.effects.iter().map(|e| e.text.clone()).collect()),
                        identity: Some(DeclIdentity {
                            digest: dep.clone(),
                            module: module.clone(),
                            name: d.name.text.clone(),
                        }),
                        doc: Some(format!("decl:{dep}:{}", d.name.text)),
                    });
                }
            }
        },
        CompletionContext::Field => {
            let fields = record_fields(&before, &index, owner);
            match fields {
                Some(fields) => {
                    for (name, ty) in fields {
                        let mut c = simple(&name, CandidateKind::Field, "field");
                        c.ty = Some(ty);
                        c.doc = None;
                        found.push(c);
                    }
                }
                None => completion.missing.push(Missing::FieldTypes),
            }
        }
        CompletionContext::Expression => {
            let module = closure.root.module_name().unwrap_or_default().to_owned();
            for i in index.visible(at, owner) {
                let b = &index.bindings[i];
                match b.kind {
                    NameKind::Generic => {}
                    NameKind::Declaration => {
                        let decl = b.decl.map(|n| index.decls[n]);
                        found.push(Candidate {
                            label: b.name.clone(),
                            kind: CandidateKind::Declaration,
                            replace: replace.clone(),
                            ty: decl.map(signature),
                            effects: decl.map(|d| d.effects.iter().map(|e| e.text.clone()).collect()),
                            identity: Some(DeclIdentity {
                                digest: digest.to_owned(),
                                module: module.clone(),
                                name: b.name.clone(),
                            }),
                            doc: Some(format!("decl:{digest}:{}", b.name)),
                        });
                    }
                    NameKind::Import => {
                        let mut c = simple(&b.name, CandidateKind::Import, "import");
                        c.doc = b.digest.as_ref().map(|d| format!("module:{d}"));
                        found.push(c);
                    }
                    _ => found.push(local(&index, i, &replace)),
                }
            }
            for b in builtins() {
                found.push(simple(b, CandidateKind::Builtin, "builtin"));
            }
            for o in orders() {
                found.push(simple(o, CandidateKind::Order, "order"));
            }
            for k in EXPRESSION_KEYWORDS {
                found.push(simple(k, CandidateKind::Keyword, "keyword"));
            }
            if before.last().is_some_and(|l| l.text == "{" || l.text == ";") {
                for k in STATEMENT_KEYWORDS {
                    found.push(simple(k, CandidateKind::Keyword, "keyword"));
                }
            }
        }
    }
    let mut seen = BTreeSet::new();
    found.retain(|c| c.label.starts_with(&prefix) && seen.insert((c.label.clone(), c.kind)));
    found.sort_by(|a, b| (&a.label, a.kind).cmp(&(&b.label, b.kind)));
    completion.candidates = found;
    Ok(completion)
}

fn identity_refusal(
    subject: IdentitySubject,
    field: crate::context::IdentityField,
    expected: Option<String>,
    held: Option<String>,
) -> Envelope {
    Envelope::new(
        code::IDENTITY_MISMATCH,
        Form::Refusal,
        Service::Complete,
        Reason::IdentityMismatch { subject, field, expected, held },
        NextAction::SelectContext,
    )
    .saying("completion reads types under one model identity, and the context's claims differ from the session's")
}

/// A local binding as a candidate, with its declared type where it has one.
fn local(index: &Index, i: usize, replace: &SourceRange) -> Candidate {
    let b = &index.bindings[i];
    Candidate {
        label: b.name.clone(),
        kind: CandidateKind::Local(b.kind),
        replace: replace.clone(),
        ty: b.ty.as_ref().map(type_text),
        effects: None,
        identity: None,
        doc: None,
    }
}

/// A declaration's signature: `fn name(a: T) -> R effects [..]`.
pub(crate) fn signature(d: &Decl) -> String {
    let keyword = match d.kind {
        c600_syntax::ast::DeclKind::Fn => "fn",
        c600_syntax::ast::DeclKind::View => "view",
        c600_syntax::ast::DeclKind::Command => "command",
        c600_syntax::ast::DeclKind::Contract => "contract",
    };
    let generics = if d.generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", d.generics.iter().map(|g| g.text.as_str()).collect::<Vec<_>>().join(","))
    };
    let params: Vec<String> = d.params.iter().map(|p| format!("{}: {}", p.name.text, type_text(&p.ty))).collect();
    let effects: Vec<&str> = d.effects.iter().map(|e| e.text.as_str()).collect();
    format!(
        "{keyword} {}{generics}({}) -> {} effects [{}]",
        d.name.text,
        params.join(", "),
        type_text(&d.result),
        effects.join(", ")
    )
}

/// Whether the position is inside a string: a closed one, or a quote that opens no string on its line, up to the
/// line's end.
fn in_text(text: &str, tokens: &[Lexeme], at: usize) -> bool {
    tokens.iter().any(|l| {
        let closed = l.kind == LexemeKind::Text && l.span.start < at && at < l.span.end;
        let open = l.kind == LexemeKind::Other
            && l.text == "\""
            && l.span.start < at
            && !text[l.span.start..at].contains('\n');
        closed || open
    })
}

/// The innermost unclosed bracket before the position, and its index among the tokens.
fn innermost<'a>(before: &[&'a Lexeme]) -> Option<(&'a str, usize)> {
    let mut stack: Vec<(&str, usize)> = Vec::new();
    for (i, l) in before.iter().enumerate() {
        if l.kind != LexemeKind::Punct {
            continue;
        }
        match l.text.as_str() {
            "(" | "[" | "{" | "<" => stack.push((l.text.as_str(), i)),
            ")" | "]" | "}" | ">" => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.last().copied()
}

fn is_declaration_keyword(text: &str) -> bool {
    matches!(text, "fn" | "view" | "command" | "contract")
}

/// The completion context from the tokens before the position.
fn context_at(before: &[&Lexeme], index: &Index, owner: Owner) -> CompletionContext {
    let previous = before.last().map(|l| l.text.as_str());
    let open = innermost(before);
    let opener = |i: usize| i.checked_sub(1).map(|p| before[p].text.as_str());
    if let Some(("[", i)) = open {
        match opener(i) {
            Some("effects") => return CompletionContext::Effects,
            Some("requires") => return CompletionContext::Capabilities,
            Some("word" | "menu" | "charge" | "inputs") => return CompletionContext::Literal,
            Some("contexts" | "permits") => return CompletionContext::NewName,
            _ => {}
        }
    }
    if previous == Some(".") {
        if let Some(value) = before.len().checked_sub(2).map(|p| before[p]).filter(|l| l.kind == LexemeKind::Name) {
            let alias = index
                .local(&value.text, value.span.start, owner)
                .or_else(|| index.module_binding(&value.text))
                .filter(|&i| index.bindings[i].kind == NameKind::Import);
            if let Some(i) = alias {
                let digest = index.bindings[i].digest.clone().unwrap_or_default();
                return CompletionContext::Member { alias: value.text.clone(), digest };
            }
        }
        return CompletionContext::Field;
    }
    match previous {
        Some("let" | "import" | "module") => return CompletionContext::NewName,
        Some(k) if is_declaration_keyword(k) => return CompletionContext::NewName,
        Some("export") => return CompletionContext::Declaration,
        Some(":" | "->") => return CompletionContext::Type,
        _ => {}
    }
    if let Some((bracket, i)) = open {
        let head = opener(i);
        let head2 = i.checked_sub(2).map(|p| before[p].text.as_str());
        match bracket {
            // A declaration's generic parameters name new types; elsewhere `<` opens type arguments.
            "<" if head2.is_some_and(is_declaration_keyword) => return CompletionContext::NewName,
            "<" => return CompletionContext::Type,
            // A parameter list: a declaration's or a closure's.
            "(" if matches!(previous, Some("(" | ","))
                && (head == Some("fn") || head2.is_some_and(is_declaration_keyword) || head == Some(">")) =>
            {
                return CompletionContext::NewName
            }
            // A record's field names.
            "{" if head == Some("record") && matches!(previous, Some("{" | ",")) => return CompletionContext::NewName,
            _ => {}
        }
        return CompletionContext::Expression;
    }
    match previous {
        None | Some(";" | "}") => CompletionContext::Declaration,
        _ => CompletionContext::Signature,
    }
}

/// The fields of the value before `.`, when it is a parameter declared with a record type.
fn record_fields(before: &[&Lexeme], index: &Index, owner: Owner) -> Option<Vec<(String, String)>> {
    let value = before.len().checked_sub(2).map(|p| before[p]).filter(|l| l.kind == LexemeKind::Name)?;
    let i = index.local(&value.text, value.span.start, owner)?;
    match index.bindings[i].ty.as_ref()? {
        TypeExpr::Record { fields, .. } => Some(fields.iter().map(|(n, t)| (n.text.clone(), type_text(t))).collect()),
        _ => None,
    }
}
