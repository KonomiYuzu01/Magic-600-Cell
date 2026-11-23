//! `rename` and applying its edits (EV1.6). Rename resolves the declaration a position names in the digest-pinned
//! graph and proposes edits to it and to the references that resolve to it, each edit under the digest of the bytes
//! it changes. Equal text in an unrelated scope, a comment or a string is never touched, and occurrences inside a
//! recovery hole, which has no resolved scope, are left as they are and reported. A dependency is never rewritten
//! under its old digest: renaming its declaration proposes edits to its bytes, which then have a new digest, and
//! re-pins every import of the changed bytes along the closure up to the draft. The resulting draft is checked again
//! before it can run, and the answer carries that check. Applying an edit to bytes other than its target is refused.

use std::collections::BTreeMap;

use c600_syntax::{sha256_hex, LexemeKind, Span};

use crate::analysis::{Closure, Source};
use crate::context::ModuleContext;
use crate::diagnose::{run, Verdict};
use crate::document::{stale, Document, SourceRange, Subject};
use crate::envelope::{code, Envelope, Form, NextAction, Reason, Service};
use crate::index::{Index, NameKind, Occurrence, Owner, Target};

/// One proposed edit: replace `range` of the bytes its digest names with `text`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edit {
    pub range: SourceRange,
    pub text: String,
}

/// The answer to `rename`.
#[derive(Clone, Debug)]
pub struct Rename {
    pub subject: Subject,
    /// The resolved declaration's range, under the digest of the source that declares it; `None` when the position
    /// names no binding of the draft or its dependencies (a reserved name, an unresolved name, a binding inside a
    /// recovery hole).
    pub declaration: Option<SourceRange>,
    /// The edits, grouped by digest (the draft's first) and in source order.
    pub edits: Vec<Edit>,
    /// Occurrences inside recovery holes, left unchanged.
    pub skipped: Vec<SourceRange>,
    /// Each changed source's digest and the digest of its bytes after the edits.
    pub changed: Vec<(String, String)>,
    /// The draft's diagnostics after the edits, checked again against the edited closure.
    pub diagnostics: Vec<Envelope>,
    pub verdict: Verdict,
}

impl Rename {
    /// The edits of one source.
    pub fn edits_for<'a>(&'a self, digest: &'a str) -> impl Iterator<Item = &'a Edit> {
        self.edits.iter().filter(move |e| e.range.digest == digest)
    }
}

/// Proposes a rename of the declaration a position in the document's declared units names.
pub fn rename(
    document: &Document,
    digest: &str,
    position: usize,
    proposed: &str,
    context: &ModuleContext,
) -> Result<Rename, Envelope> {
    let at = document.byte_offset(digest, position, Service::Rename)?;
    let text = document.expect(digest, Service::Rename)?;
    let subject = document.subject(&context.edition, Some(context.digest()));
    let closure = Closure::read(Source::read(text.as_bytes(), digest), context);
    let index = Index::build(&closure.root.file, &closure.root.lexemes);

    let mut plan = Plan::default();
    let target = index.occurrence(at).map(|o| (o, index.binding_of(o)));
    match target {
        Some((_, Some(i))) if !matches!(index.bindings[i].owner, Owner::Hole(_)) => {
            let b = &index.bindings[i];
            plan.declaration = Some(closure.root.range(b.span));
            plan.rename(&closure.root, &index, i, proposed);
        }
        Some((Occurrence::Ref(r), None)) => {
            if let Target::Member { alias, name } = &index.refs[r].target {
                let pinned = index.bindings[*alias].digest.clone().unwrap_or_default();
                if let Some(dep) = closure.deps.get(&pinned) {
                    let dep_index = Index::build(&dep.file, &dep.lexemes);
                    let decl = dep_index
                        .module_binding(name)
                        .filter(|&i| dep_index.bindings[i].kind == NameKind::Declaration)
                        .filter(|&i| dep_index.bindings[i].decl.is_some_and(|n| dep_index.decls[n].exported));
                    if let Some(i) = decl {
                        plan.declaration = Some(dep.range(dep_index.bindings[i].span));
                        plan.rename(dep, &dep_index, i, proposed);
                        plan.members(&closure, &pinned, name, proposed);
                    }
                }
            }
        }
        _ => {}
    }
    let (edits, changed, edited) = plan.finish(&closure);

    // The resulting draft, checked again against the edited closure.
    let mut after = context.clone();
    for (old, bytes) in &edited {
        if *old != closure.root.digest {
            let supports = context.dependency(old).and_then(|d| d.supports.clone());
            after.add_dependency(bytes.clone(), supports);
        }
    }
    let root_bytes = edited.get(&closure.root.digest).cloned().unwrap_or_else(|| text.as_bytes().to_vec());
    let root_digest = sha256_hex(&root_bytes);
    let rechecked = Closure::read(Source::read(&root_bytes, &root_digest), &after);
    let (diagnosis, _) = run(&rechecked, &after, subject.clone());
    Ok(Rename {
        subject,
        declaration: plan.declaration,
        edits,
        skipped: plan.skipped,
        changed,
        diagnostics: diagnosis.diagnostics,
        verdict: diagnosis.verdict,
    })
}

#[derive(Default)]
struct Plan {
    declaration: Option<SourceRange>,
    /// Rename edits by digest.
    edits: BTreeMap<String, Vec<Edit>>,
    skipped: Vec<SourceRange>,
}

impl Plan {
    fn push(&mut self, source: &Source, span: Span, text: &str) {
        self.edits
            .entry(source.digest.clone())
            .or_default()
            .push(Edit { range: source.range(span), text: text.to_owned() });
    }

    /// The binding's own range and every resolved reference outside holes.
    fn rename(&mut self, source: &Source, index: &Index, binding: usize, proposed: &str) {
        self.push(source, index.bindings[binding].span, proposed);
        for r in index.references(binding) {
            match r.owner {
                Owner::Hole(_) => self.skipped.push(source.range(r.span)),
                _ => self.push(source, r.span, proposed),
            }
        }
    }

    /// Every `alias.name` reference to the dependency `pinned` in the closure's other sources.
    fn members(&mut self, closure: &Closure, pinned: &str, name: &str, proposed: &str) {
        for source in std::iter::once(&closure.root).chain(closure.deps.values()) {
            if source.digest == pinned {
                continue;
            }
            let index = Index::build(&source.file, &source.lexemes);
            for r in &index.refs {
                let Target::Member { alias, name: member } = &r.target else { continue };
                if member != name || index.bindings[*alias].digest.as_deref() != Some(pinned) {
                    continue;
                }
                match r.owner {
                    Owner::Hole(_) => self.skipped.push(source.range(r.span)),
                    _ => self.push(source, r.span, proposed),
                }
            }
        }
    }

    /// The edits with the re-pinned imports, each changed source's new digest, and its edited bytes by old digest.
    /// Sources are visited dependencies first, so every import's new digest is known before its importer's edits.
    #[allow(clippy::type_complexity)]
    fn finish(&mut self, closure: &Closure) -> (Vec<Edit>, Vec<(String, String)>, BTreeMap<String, Vec<u8>>) {
        let mut changed: BTreeMap<String, String> = BTreeMap::new();
        let mut edited: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut all = Vec::new();
        for digest in order(closure) {
            let Some(source) = closure.source(&digest) else { continue };
            let mut edits = self.edits.remove(&digest).unwrap_or_default();
            for item in &source.file.items {
                let c600_syntax::ast::Item::Import(import) = item else { continue };
                let Some(new) = changed.get(&import.digest) else { continue };
                let literal = source.lexemes.iter().find(|l| {
                    l.kind == LexemeKind::Text && l.span.start >= import.loc.0.start && l.span.end <= import.loc.0.end
                });
                if let Some(l) = literal {
                    let inner = Span { start: l.span.start + 1, end: l.span.end - 1, ..l.span };
                    edits.push(Edit { range: source.range(inner), text: new.clone() });
                }
            }
            if edits.is_empty() {
                continue;
            }
            edits.sort();
            edits.dedup();
            let text = std::str::from_utf8(&source.bytes).unwrap_or_default();
            let bytes = splice(text, &edits).into_bytes();
            changed.insert(digest.clone(), sha256_hex(&bytes));
            edited.insert(digest.clone(), bytes);
            all.push((digest == closure.root.digest, edits));
        }
        all.sort_by_key(|(root, _)| !*root);
        let edits = all.into_iter().flat_map(|(_, e)| e).collect();
        (edits, changed.into_iter().collect(), edited)
    }
}

/// The closure's digests, each after every source it imports, the draft last.
fn order(closure: &Closure) -> Vec<String> {
    fn visit(closure: &Closure, digest: &str, seen: &mut Vec<String>) {
        if seen.iter().any(|d| d == digest) {
            return;
        }
        let Some(source) = closure.source(digest) else { return };
        for item in &source.file.items {
            if let c600_syntax::ast::Item::Import(i) = item {
                visit(closure, &i.digest, seen);
            }
        }
        seen.push(digest.to_owned());
    }
    let mut seen = Vec::new();
    visit(closure, &closure.root.digest.clone(), &mut seen);
    seen
}

/// The text with sorted, non-overlapping edits applied.
fn splice(text: &str, edits: &[Edit]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for e in edits {
        out.push_str(&text[at..e.range.start]);
        out.push_str(&e.text);
        at = e.range.end;
    }
    out.push_str(&text[at..]);
    out
}

/// Applies edits to a document's bytes. Every edit must name the document's digest (else `EV1:stale.editor_source`)
/// and a valid range on it, and no two edits may overlap or insert at one point (else
/// `EV1:input.source_range_invalid`). Nothing is applied unless all are valid.
pub fn apply_edits(document: &Document, edits: &[Edit]) -> Result<Vec<u8>, Envelope> {
    for e in edits {
        if e.range.digest != document.digest() {
            return Err(stale(Service::ApplyEdits, &e.range.digest, document.digest()));
        }
    }
    let text = document.expect(document.digest(), Service::ApplyEdits)?;
    let mut sorted = edits.to_vec();
    sorted.sort();
    for e in &sorted {
        document.validate_in(text, &e.range, Service::ApplyEdits)?;
    }
    for pair in sorted.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if b.range.start < a.range.end || a.range == b.range {
            return Err(Envelope::new(
                code::SOURCE_RANGE_INVALID,
                Form::Refusal,
                Service::ApplyEdits,
                Reason::OverlappingEdits { first: a.range.clone(), second: b.range.clone() },
                NextAction::SeparateEdits,
            )
            .saying("two proposed edits overlap"));
        }
    }
    Ok(splice(text, &sorted).into_bytes())
}
