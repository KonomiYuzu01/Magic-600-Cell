//! `parse(source, edition)` (EV1.2): the recovered structure and diagnostics even for an incomplete draft. Recovery
//! marks gaps and gives completion and navigation a structure to read; it supplies no executable meaning, and a draft
//! with a gap never passes the check because its surrounding structure parsed.

use c600_syntax::ast::{DeclKind, Item, ParsedFile, TriviaKind};
use c600_syntax::{Span, EDITION};

use crate::analysis::syntax;
use crate::document::{Document, SourceRange, Subject};
use crate::envelope::{code, Envelope, Form, NextAction, Reason, Service};

/// What an outline entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineKind {
    Import,
    Fn,
    View,
    Command,
    Contract,
    /// A recovery hole: a declaration that did not parse.
    Hole,
    Comment,
}

/// One entry of a file's outline, in source order: plain data a host can show or send over a C interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem {
    pub kind: OutlineKind,
    pub range: SourceRange,
    /// The declared name, an import's alias, or a hole's name as far as it was read.
    pub name: Option<String>,
    pub name_range: Option<SourceRange>,
    pub exported: bool,
    /// For a hole, the declaration kind as far as it was read.
    pub recovered_kind: Option<OutlineKind>,
}

/// The answer to `parse`.
#[derive(Clone, Debug)]
pub struct Parse {
    pub subject: Subject,
    /// The recovered structure: the parsed program, its recovery holes with what was read of them, and its trivia.
    pub file: ParsedFile,
    pub outline: Vec<OutlineItem>,
    pub diagnostics: Vec<Envelope>,
}

impl Parse {
    /// Whether the draft parsed without a diagnostic or a hole. Even then it runs only after a check and an
    /// activation (EV1.8, EV1.11).
    pub fn is_complete(&self) -> bool {
        self.diagnostics.is_empty() && self.file.holes.is_empty()
    }
}

fn kind(k: DeclKind) -> OutlineKind {
    match k {
        DeclKind::Fn => OutlineKind::Fn,
        DeclKind::View => OutlineKind::View,
        DeclKind::Command => OutlineKind::Command,
        DeclKind::Contract => OutlineKind::Contract,
    }
}

/// Parses a document under an edition. An edition this parser does not read gives `LG1:syntax.edition_unsupported`
/// and no structure; no other edition is tried (LG1.2).
pub fn parse(document: &Document, edition: &str) -> Parse {
    let subject = document.subject(edition, None);
    let digest = document.digest();
    if edition != EDITION {
        let reason = Reason::EditionUnsupported { requested: edition.to_owned(), supported: EDITION.to_owned() };
        let envelope =
            Envelope::new(code::EDITION_UNSUPPORTED, Form::Diagnostic, Service::Parse, reason, NextAction::EditSource)
                .at(document.text().map(|_| SourceRange::new(digest, 0, 0)))
                .saying("the request names an edition this parser does not read");
        return Parse { subject, file: ParsedFile::default(), outline: Vec::new(), diagnostics: vec![envelope] };
    }
    // A copy of the shared parse when another service has made it, else a parse of its own: sharing this one would
    // take a copy of it here, on the path of the parse's own answer, which a host asks for first.
    let file = document.parsed_if_made().map_or_else(|| c600_syntax::parse(document.bytes()), ParsedFile::clone);
    let diagnostics = file.diagnostics.iter().map(|d| syntax(d, digest, Service::Parse)).collect();
    let outline = outline(&file, digest);
    Parse { subject, file, outline, diagnostics }
}

/// The outline of a parsed file: imports, declarations, holes and comments, in source order.
pub(crate) fn outline(file: &ParsedFile, digest: &str) -> Vec<OutlineItem> {
    let range = |s: Span| SourceRange::new(digest, s.start, s.end);
    let mut items = Vec::new();
    for item in &file.items {
        items.push(match item {
            Item::Import(i) => OutlineItem {
                kind: OutlineKind::Import,
                range: range(i.loc.0),
                name: Some(i.alias.text.clone()),
                name_range: Some(range(i.alias.loc.0)),
                exported: false,
                recovered_kind: None,
            },
            Item::Decl(d) => OutlineItem {
                kind: kind(d.kind),
                range: range(d.loc.0),
                name: Some(d.name.text.clone()),
                name_range: Some(range(d.name.loc.0)),
                exported: d.exported,
                recovered_kind: None,
            },
        });
    }
    for r in &file.recovered {
        items.push(OutlineItem {
            kind: OutlineKind::Hole,
            range: range(r.hole),
            name: r.name.as_ref().map(|n| n.text.clone()),
            name_range: r.name.as_ref().map(|n| range(n.loc.0)),
            exported: r.exported,
            recovered_kind: r.kind.map(kind),
        });
    }
    for t in file.trivia.iter().filter(|t| t.kind == TriviaKind::Comment) {
        items.push(OutlineItem {
            kind: OutlineKind::Comment,
            range: range(t.span),
            name: None,
            name_range: None,
            exported: false,
            recovered_kind: None,
        });
    }
    items.sort_by_key(|i| (i.range.start, i.range.end));
    items
}
