//! `hover` and `definition` (EV1.5, EV1.6). Hover gives the declared type, effects and required capabilities,
//! exactness, the checker's inferred needs when the check ran, the identity and documentation reference, and keeps
//! the static check, evidence and permission apart. For what it cannot resolve it gives the known fields and why the
//! rest is unknown; it never invents a successful check. Definition resolves a declaration in the digest-pinned
//! dependency graph: a range in a dependency names that dependency's digest.

use c600_check::{separation, Policy, Report, Summary};
use c600_ir::{Capability, Effect};
use c600_syntax::ast::{Decl, TypeExpr};
use c600_syntax::{base_types, type_constructors, type_text};

use crate::analysis::{Closure, Source};
use crate::complete::{signature, DeclIdentity};
use crate::context::ModuleContext;
use crate::diagnose::{run, Stage, Verdict};
use crate::document::{Document, SourceRange, Subject};
use crate::envelope::{Envelope, Service};
use crate::index::{Index, NameKind, Occurrence, Target};

/// What a hovered name resolves to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// A binding inside a declaration or a recovery hole.
    Local(NameKind),
    /// A function of the draft's module.
    Declaration,
    /// An import alias and the digest it pins.
    Import {
        digest: String,
    },
    /// A declaration of a pinned dependency.
    Member {
        digest: String,
    },
    Builtin,
    Order,
    Keyword,
    Type,
    Effect,
    Capability,
    Unresolved,
}

/// A value's exactness as its declared type says (LG2.19).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exactness {
    Exact,
    Approximate,
    /// Display-qualified: it may be shown, never steer exact computation.
    Display,
    Unknown,
}

/// The module's static check, which is neither evidence for a result nor permission to invoke (EV1.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticCheck {
    Checked,
    Refused,
    NotChecked(Stage),
}

/// Permission to invoke: a hover never grants it. It is decided at invocation, by the grants and KN5.7's
/// availability (EV1.3, EV1.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    DecidedAtInvocation,
}

/// What the checker inferred a function needs (LG3.2), with how capability separation sees it under the interim
/// policy (BND.41).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inferred {
    pub effects: Vec<String>,
    pub capabilities: Vec<String>,
    pub reads_state: bool,
    pub prepares_previews: bool,
}

/// A field a hover does not know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverField {
    Type,
    Effects,
    Capabilities,
    ModelAndLevel,
    Definition,
}

/// Why it is not known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Because {
    /// The draft did not reach the checker.
    NotChecked(Stage),
    /// The import pins bytes the module context does not hold.
    DependencyMissing { digest: String },
    /// The name resolves to nothing the draft declares or imports.
    NameUnresolved,
    /// The IR draft has no identity tower or levels yet (LG2.3; `rust/README.md`).
    NotInIrDraft,
    /// The checker reports no inferred type for a local binding.
    NotReportedByChecker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unresolved {
    pub field: HoverField,
    pub because: Because,
}

/// The answer to `hover`.
#[derive(Clone, Debug)]
pub struct Hover {
    pub subject: Subject,
    /// The hovered name's range.
    pub range: SourceRange,
    pub name: String,
    pub resolved: Resolved,
    /// The declared type, or a declaration's signature.
    pub declared: Option<String>,
    pub declared_effects: Option<Vec<String>>,
    /// A module's or import's required capabilities, from its header.
    pub requires: Option<Vec<String>>,
    pub exactness: Exactness,
    pub check: StaticCheck,
    pub inferred: Option<Inferred>,
    pub identity: Option<DeclIdentity>,
    pub definition: Option<SourceRange>,
    /// A documentation reference, in [`crate::Candidate::doc`]'s forms.
    pub docs: Option<String>,
    /// Evidence references. The IR draft has no claims, so a hover shows none.
    pub evidence: Vec<String>,
    pub permission: Permission,
    pub unresolved: Vec<Unresolved>,
}

/// The source name of an effect.
pub(crate) fn effect_name(e: Effect) -> &'static str {
    match e {
        Effect::ReadSnapshot => "snapshot",
        Effect::ChangeView => "view",
        Effect::ChangeWorkspace => "workspace",
        Effect::PreparePreview => "preview",
        Effect::GrantedFile => "file",
        Effect::SeededRandomness => "random",
        Effect::Budgets => "budget",
    }
}

fn capability_name(c: Capability) -> &'static str {
    match c {
        Capability::File => "file",
        Capability::Clock => "clock",
        Capability::Randomness => "random",
        Capability::Network => "network",
        Capability::Process => "process",
        Capability::SessionFile => "session_file",
    }
}

fn inferred(summary: &Summary) -> Inferred {
    let s = separation(summary, &Policy::interim());
    Inferred {
        effects: summary.effects.iter().map(|e| effect_name(*e).to_owned()).collect(),
        capabilities: summary.capabilities.iter().map(|c| capability_name(*c).to_owned()).collect(),
        reads_state: s.reads_state,
        prepares_previews: s.prepares_previews,
    }
}

/// A declared type's exactness.
fn exactness(t: &TypeExpr) -> Exactness {
    match t {
        TypeExpr::Named { name, .. } => match name.text.as_str() {
            "Display" => Exactness::Display,
            "Approx" | "ApproxBool" => Exactness::Approximate,
            // The collections and the identity tower's types (T1 drafts) are exact.
            n if base_types().contains(&n) || type_constructors().contains(&n) => Exactness::Exact,
            _ => Exactness::Unknown,
        },
        TypeExpr::Fn { .. } | TypeExpr::Record { .. } => Exactness::Exact,
    }
}

/// The resolved target of the name a position touches.
enum Found<'i> {
    Binding(usize),
    Member { alias: usize, name: &'i str },
    Plain(&'i Target),
}

struct Read<'a> {
    closure: Closure,
    context: &'a ModuleContext,
}

fn read<'a>(
    document: &Document,
    digest: &str,
    position: usize,
    context: &'a ModuleContext,
    service: Service,
) -> Result<(Read<'a>, usize), Envelope> {
    let at = document.byte_offset(digest, position, service)?;
    let text = document.expect(digest, service)?;
    let closure = Closure::read(Source::read(text.as_bytes(), digest), context);
    Ok((Read { closure, context }, at))
}

fn found<'i>(index: &'i Index, occurrence: Occurrence) -> (Found<'i>, &'i c600_syntax::Span, &'i str) {
    match occurrence {
        Occurrence::Binding(i) => (Found::Binding(i), &index.bindings[i].span, &index.bindings[i].name),
        Occurrence::Ref(r) => {
            let reference = &index.refs[r];
            let target = match &reference.target {
                Target::Binding(i) => Found::Binding(*i),
                Target::Member { alias, name } => Found::Member { alias: *alias, name },
                other => Found::Plain(other),
            };
            (target, &reference.span, &reference.name)
        }
    }
}

/// The exported or local declaration `name` of a source.
fn declaration<'s>(source: &'s Source, name: &str) -> Option<&'s Decl> {
    source.file.declarations().find(|d| d.name.text == name)
}

/// Hovers at a position in the document's declared units; `None` when it touches no name.
pub fn hover(
    document: &Document,
    digest: &str,
    position: usize,
    context: &ModuleContext,
) -> Result<Option<Hover>, Envelope> {
    let (read, at) = read(document, digest, position, context, Service::Hover)?;
    let closure = &read.closure;
    let index = Index::build(&closure.root.file, closure.root.lexemes());
    let Some(occurrence) = index.occurrence(at) else { return Ok(None) };
    let subject = document.subject(&context.edition, Some(context.digest()));
    let (diagnosis, checked) = run(closure, read.context, subject.clone());
    let check = match diagnosis.verdict {
        Verdict::Checked(_) => StaticCheck::Checked,
        Verdict::Refused => StaticCheck::Refused,
        Verdict::NotChecked(stage) => StaticCheck::NotChecked(stage),
    };
    let (target, span, name) = found(&index, occurrence);
    let mut hover = Hover {
        subject,
        range: closure.root.range(*span),
        name: name.to_owned(),
        resolved: Resolved::Unresolved,
        declared: None,
        declared_effects: None,
        requires: None,
        exactness: Exactness::Unknown,
        check,
        inferred: None,
        identity: None,
        definition: None,
        docs: None,
        evidence: Vec::new(),
        permission: Permission::DecidedAtInvocation,
        unresolved: Vec::new(),
    };
    let not_checked = |hover: &Hover| match hover.check {
        StaticCheck::NotChecked(stage) => Because::NotChecked(stage),
        _ => Because::NotReportedByChecker,
    };
    let module = closure.root.module_name().unwrap_or_default().to_owned();
    match target {
        Found::Binding(i) => {
            let b = &index.bindings[i];
            hover.definition = Some(closure.root.range(b.span));
            match b.kind {
                NameKind::Declaration => {
                    hover.resolved = Resolved::Declaration;
                    hover.identity =
                        Some(DeclIdentity { digest: digest.to_owned(), module: module.clone(), name: b.name.clone() });
                    hover.docs = Some(format!("decl:{digest}:{}", b.name));
                    match b.decl.map(|n| index.decls[n]) {
                        Some(d) => describe(&mut hover, d),
                        None => {
                            let because = not_checked(&hover);
                            hover.unresolved.push(Unresolved { field: HoverField::Type, because: because.clone() });
                            hover.unresolved.push(Unresolved { field: HoverField::Effects, because });
                        }
                    }
                    infer(&mut hover, checked.as_ref().map(|(_, r)| r), &module, &b.name);
                }
                NameKind::Import => {
                    let pinned = b.digest.clone().unwrap_or_default();
                    hover.resolved = Resolved::Import { digest: pinned.clone() };
                    hover.docs = Some(format!("module:{pinned}"));
                    match closure.deps.get(&pinned) {
                        Some(dep) => {
                            hover.declared = dep.module_name().map(|m| format!("module {m}"));
                            hover.requires =
                                dep.file.module.as_ref().map(|m| m.requires.iter().map(|r| r.text.clone()).collect());
                        }
                        None => hover.unresolved.push(Unresolved {
                            field: HoverField::Type,
                            because: Because::DependencyMissing { digest: pinned },
                        }),
                    }
                }
                kind => {
                    hover.resolved = Resolved::Local(kind);
                    match &b.ty {
                        Some(t) => {
                            hover.declared = Some(type_text(t));
                            hover.exactness = exactness(t);
                        }
                        None if kind == NameKind::Generic => {}
                        None => {
                            let because = not_checked(&hover);
                            hover.unresolved.push(Unresolved { field: HoverField::Type, because });
                        }
                    }
                    if kind != NameKind::Generic {
                        hover
                            .unresolved
                            .push(Unresolved { field: HoverField::ModelAndLevel, because: Because::NotInIrDraft });
                    }
                }
            }
        }
        Found::Member { alias, name } => {
            let pinned = index.bindings[alias].digest.clone().unwrap_or_default();
            hover.resolved = Resolved::Member { digest: pinned.clone() };
            match closure.deps.get(&pinned) {
                None => {
                    for field in [HoverField::Type, HoverField::Effects, HoverField::Definition] {
                        hover
                            .unresolved
                            .push(Unresolved { field, because: Because::DependencyMissing { digest: pinned.clone() } });
                    }
                }
                Some(dep) => match declaration(dep, name).filter(|d| d.exported) {
                    None => {
                        for field in [HoverField::Type, HoverField::Definition] {
                            hover.unresolved.push(Unresolved { field, because: Because::NameUnresolved });
                        }
                    }
                    Some(d) => {
                        let dep_module = dep.module_name().unwrap_or_default().to_owned();
                        describe(&mut hover, d);
                        hover.definition = Some(dep.range(d.name.loc.0));
                        hover.identity = Some(DeclIdentity {
                            digest: pinned.clone(),
                            module: dep_module.clone(),
                            name: name.to_owned(),
                        });
                        hover.docs = Some(format!("decl:{pinned}:{name}"));
                        let report = match &checked {
                            Some((_, report)) => Some(report.clone()),
                            None => closure.check_dependencies().map(|(_, r)| r),
                        };
                        infer(&mut hover, report.as_ref(), &dep_module, name);
                    }
                },
            }
        }
        Found::Plain(target) => {
            let (resolved, docs) = match target {
                Target::Builtin => (Resolved::Builtin, "builtin"),
                Target::Order => (Resolved::Order, "order"),
                Target::Keyword => (Resolved::Keyword, "keyword"),
                Target::Type => (Resolved::Type, "type"),
                Target::Effect => (Resolved::Effect, "effect"),
                Target::Capability => (Resolved::Capability, "capability"),
                _ => (Resolved::Unresolved, ""),
            };
            if resolved == Resolved::Unresolved {
                hover.unresolved.push(Unresolved { field: HoverField::Type, because: Because::NameUnresolved });
                hover.unresolved.push(Unresolved { field: HoverField::Definition, because: Because::NameUnresolved });
            } else {
                hover.docs = Some(format!("{docs}:{name}"));
            }
            hover.resolved = resolved;
        }
    }
    Ok(Some(hover))
}

/// A declaration's declared signature, effects and result exactness.
fn describe(hover: &mut Hover, d: &Decl) {
    hover.declared = Some(signature(d));
    hover.declared_effects = Some(d.effects.iter().map(|e| e.text.clone()).collect());
    hover.exactness = exactness(&d.result);
    hover.unresolved.push(Unresolved { field: HoverField::ModelAndLevel, because: Because::NotInIrDraft });
}

/// The checker's inferred needs of a function, when the check ran; otherwise why they are unknown.
fn infer(hover: &mut Hover, report: Option<&Report>, module: &str, function: &str) {
    match report.and_then(|r| r.summary(module, function)) {
        Some(summary) => hover.inferred = Some(inferred(summary)),
        None => {
            let because = match hover.check {
                StaticCheck::NotChecked(stage) => Because::NotChecked(stage),
                _ => Because::NameUnresolved,
            };
            hover.unresolved.push(Unresolved { field: HoverField::Capabilities, because });
        }
    }
}

/// The ranges that define the name at a position: a binding's own range, a dependency declaration's range under the
/// dependency's digest, or none for a reserved name or an unresolved one.
pub fn definition(
    document: &Document,
    digest: &str,
    position: usize,
    context: &ModuleContext,
) -> Result<Vec<SourceRange>, Envelope> {
    let (read, at) = read(document, digest, position, context, Service::Definition)?;
    let closure = &read.closure;
    let index = Index::build(&closure.root.file, closure.root.lexemes());
    let Some(occurrence) = index.occurrence(at) else { return Ok(Vec::new()) };
    let (target, _, _) = found(&index, occurrence);
    Ok(match target {
        Found::Binding(i) => vec![closure.root.range(index.bindings[i].span)],
        Found::Member { alias, name } => {
            let pinned = index.bindings[alias].digest.clone().unwrap_or_default();
            closure
                .deps
                .get(&pinned)
                .and_then(|dep| declaration(dep, name).filter(|d| d.exported).map(|d| dep.range(d.name.loc.0)))
                .into_iter()
                .collect()
        }
        Found::Plain(_) => Vec::new(),
    })
}
