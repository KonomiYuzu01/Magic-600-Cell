//! Reference documentation generated from source declarations and their checked metadata (EV5.2): for each module,
//! its source and closure digests, edition, IR version and contract, declared capabilities and supported identities;
//! for each function, its signature, its declared and inferred effects, capabilities and grant rows, and a link to its
//! declaration. Documentation comments directly above a declaration are carried as written. The principle record, when
//! one exists, is linked, never copied. A broken draft's output stays [`Status::Broken`] and is never shown as the
//! active tool's documentation (EV1.7).

use c600_check::type_name;
use c600_ir::{FnKind, Function};
use c600_module::{resolve, Files, Identity, Manifest};
use c600_syntax::{parse, Span};

use crate::json::{self, Json};
use crate::{Broken, Checked, DocsError};

/// Which documentation this is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The active version's documentation, named by its closure digest.
    Active,
    /// A checked candidate that is not the active version.
    Candidate,
    /// A draft that does not check: its findings, and no checked metadata.
    Broken { findings: Vec<(String, String)> },
}

impl Status {
    pub fn label(&self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Candidate => "checked candidate, not active",
            Status::Broken { .. } => "broken draft, not the active tool's documentation",
        }
    }
}

/// A link to a declaration: the source digest and the declaration's range in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub source_sha256: String,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// One function's reference entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: &'static str,
    pub exported: bool,
    /// `name: Type` per parameter, in order.
    pub params: Vec<String>,
    pub result: String,
    /// The effect list the source declares, or `None` when it declares none.
    pub declared_effects: Option<Vec<String>>,
    /// The checker's inferred effects, capabilities, grant rows and state reads (LG3.2).
    pub effects: Vec<String>,
    pub capabilities: Vec<String>,
    pub rows: Vec<String>,
    pub reads_state: bool,
    pub declaration: Declaration,
    /// The comment lines directly above the declaration, as written.
    pub doc: Vec<String>,
}

/// One module's reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub source_sha256: String,
    /// The closure digest of the module's manifest and lock (LG5.3), with the supported identities.
    pub closure_sha256: String,
    pub edition: String,
    pub ir_version: String,
    pub exec_contract: String,
    pub capabilities: Vec<String>,
    /// The supported identity blocks as the manifest's canonical bytes hold them.
    pub supports: Vec<String>,
    /// The principle record's data file and digest, when one exists.
    pub principle: Option<(String, String)>,
    pub entries: Vec<Entry>,
}

/// A reference document: a module set's entries and its status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub status: Status,
    pub modules: Vec<Module>,
}

fn names<T: std::fmt::Debug>(items: impl IntoIterator<Item = T>) -> Vec<String> {
    let mut out: Vec<String> = items.into_iter().map(|x| format!("{x:?}")).collect();
    out.sort();
    out
}

/// The manifest and lock of one module with no further module in its closure, and the closure digest (LG5.3). The
/// manifest names no principle record: the record binds the closure, so the closure cannot name the record.
pub fn closure(source: &[u8], supports: &[Identity], boundary: Option<&str>) -> Result<(Vec<u8>, String), DocsError> {
    let refuse = |detail: String| DocsError::new(crate::code::DECOMPOSITION_MISMATCH, detail);
    let file = parse(source);
    let header = file.module.as_ref().ok_or_else(|| refuse("the source has no module header".into()))?;
    let manifest = Manifest {
        module: header.name.text.clone(),
        edition: file.edition.clone().unwrap_or_default(),
        source: c600_digest::sha256(source),
        dependencies: Vec::new(),
        capabilities: header.requires.iter().map(|n| n.text.clone()).collect(),
        boundary: boundary.map(str::to_string),
        principle: None,
        supports: supports.to_vec(),
    };
    let bytes = manifest.bytes();
    let lock = resolve(&bytes, &Files::new().with("module.6cs", source)).map_err(|r| refuse(format!("{r:?}")))?;
    Ok((bytes, c600_digest::hex(&lock.closure_digest())))
}

/// The supported identity blocks of a manifest, read back from its canonical bytes.
fn supports_of(manifest: &[u8]) -> Vec<String> {
    let Ok(doc) = json::parse(manifest) else { return Vec::new() };
    doc.get("payload")
        .and_then(|p| p.get("supports"))
        .and_then(Json::as_list)
        .map(|items| items.iter().map(Json::compact).collect())
        .unwrap_or_default()
}

fn kind(kind: FnKind) -> &'static str {
    match kind {
        FnKind::Helper => "fn",
        FnKind::View => "view",
        FnKind::Command => "command",
        FnKind::Contract => "contract",
    }
}

/// The comment lines directly above `decl`: comments separated from it by whitespace only.
fn doc_above(source: &str, trivia: &[c600_syntax::ast::Trivia], decl: &Span) -> Vec<String> {
    let mut out = Vec::new();
    let mut end = decl.start;
    for t in trivia.iter().rev().filter(|t| t.span.end <= decl.start) {
        let between = &source[t.span.end..end];
        if !between.trim().is_empty() || between.matches('\n').count() > 1 {
            break;
        }
        match t.kind {
            c600_syntax::ast::TriviaKind::Comment => {
                out.push(source[t.span.start..t.span.end].trim_start_matches('#').trim().to_string());
                end = t.span.start;
            }
            // A blank line ends the comment block that belongs to the declaration.
            c600_syntax::ast::TriviaKind::Whitespace if source[t.span.start..t.span.end].matches('\n').count() > 1 => {
                break
            }
            c600_syntax::ast::TriviaKind::Whitespace => end = t.span.start,
        }
    }
    out.reverse();
    out
}

fn entry(checked: &Checked, module: &str, f: &Function, source: &str, file: &c600_syntax::ParsedFile) -> Entry {
    let map = checked.lowered.source(module).expect("a lowered module has a source map");
    let span = map.declarations.get(&f.name).copied().unwrap_or(Span { start: 0, end: 0, line: 0, column: 0 });
    let summary = checked.report.summary(module, &f.name).cloned().unwrap_or_default();
    let separation = c600_check::separation(&summary, &c600_check::Policy::interim());
    Entry {
        name: f.name.clone(),
        kind: kind(f.kind),
        exported: f.exported,
        params: f.params.iter().map(|p| format!("{}: {}", p.name, type_name(&p.ty.ty))).collect(),
        result: type_name(&f.result.ty),
        declared_effects: f.effects.as_ref().map(|b| names(b.effects.iter())),
        effects: names(summary.effects.iter()),
        capabilities: names(summary.capabilities.iter()),
        rows: names(summary.rows.iter()),
        reads_state: separation.reads_state,
        declaration: Declaration {
            source_sha256: map.digest.clone(),
            start: span.start,
            end: span.end,
            line: span.line,
            column: span.column,
        },
        doc: doc_above(source, &file.trivia, &span),
    }
}

/// The reference of every module in `checked`, each with its closure under `supports`. `principles` names the
/// principle record's data file and digest by module. `active` says whether this is the active version.
pub fn reference(
    checked: &Checked,
    supports: &[Identity],
    principles: &[(&str, &str, &str)],
    active: bool,
) -> Result<Reference, DocsError> {
    let mut modules = Vec::new();
    for m in &checked.lowered.program.modules {
        let bytes = checked.source(&m.name).ok_or_else(|| {
            DocsError::new(crate::code::DECOMPOSITION_MISMATCH, format!("no source bytes for `{}`", m.name))
        })?;
        let source = std::str::from_utf8(bytes).unwrap_or("");
        let file = parse(bytes);
        let (manifest, closure_sha256) = closure(bytes, supports, None)?;
        let entries = m.functions.iter().map(|f| entry(checked, &m.name, f, source, &file)).collect();
        modules.push(Module {
            name: m.name.clone(),
            source_sha256: c600_digest::sha256_hex(bytes),
            closure_sha256,
            edition: file.edition.clone().unwrap_or_default(),
            ir_version: checked.lowered.program.ir_version.clone(),
            exec_contract: checked.lowered.program.exec_contract.clone(),
            capabilities: names(m.capabilities.iter()),
            supports: supports_of(&manifest),
            principle: principles
                .iter()
                .find(|(name, _, _)| *name == m.name)
                .map(|(_, path, digest)| (path.to_string(), digest.to_string())),
            entries,
        });
    }
    let status = if active { Status::Active } else { Status::Candidate };
    Ok(Reference { status, modules })
}

/// The reference of a draft that does not check: its findings and no entry (EV1.7).
pub fn broken(draft: &Broken) -> Reference {
    Reference { status: Status::Broken { findings: draft.findings.clone() }, modules: Vec::new() }
}

/// The reference as Markdown: a generated page, never edited by hand.
pub fn render(reference: &Reference) -> String {
    let mut out = format!(
        "# Reference ({})\n\nGenerated from checked metadata (EV5.2); edit the source, not this page.\n",
        reference.status.label()
    );
    if let Status::Broken { findings } = &reference.status {
        out.push_str("\nThis draft does not check. Its findings:\n\n");
        for (code, text) in findings {
            out.push_str(&format!("- `{code}`: {text}\n"));
        }
        return out;
    }
    for m in &reference.modules {
        out.push_str(&format!("\n## Module `{}`\n\n", m.name));
        out.push_str(&format!("- Source: `{}`\n- Closure: `{}`\n", m.source_sha256, m.closure_sha256));
        out.push_str(&format!("- Edition `{}`, IR `{}`, contract `{}`\n", m.edition, m.ir_version, m.exec_contract));
        out.push_str(&format!("- Capabilities requested: [{}]\n", m.capabilities.join(", ")));
        for s in &m.supports {
            out.push_str(&format!("- Supports: `{s}`\n"));
        }
        match &m.principle {
            Some((path, digest)) => out.push_str(&format!("- Principle record: `{path}` (`{digest}`)\n")),
            None => out.push_str("- Principle record: none\n"),
        }
        for e in &m.entries {
            let export = if e.exported { "export " } else { "" };
            out.push_str(&format!(
                "\n### `{export}{} {}({}) -> {}`\n\n",
                e.kind,
                e.name,
                e.params.join(", "),
                e.result
            ));
            for line in &e.doc {
                out.push_str(&format!("{line}\n"));
            }
            if !e.doc.is_empty() {
                out.push('\n');
            }
            let declared =
                e.declared_effects.as_ref().map_or("none declared".to_string(), |d| format!("[{}]", d.join(", ")));
            out.push_str(&format!("- Declared effects: {declared}\n"));
            out.push_str(&format!("- Inferred effects: [{}]\n", e.effects.join(", ")));
            out.push_str(&format!("- Capabilities: [{}]\n", e.capabilities.join(", ")));
            out.push_str(&format!("- Grant rows: [{}]\n", e.rows.join(", ")));
            out.push_str(&format!("- Reads state: {}\n", e.reads_state));
            out.push_str(&format!(
                "- Declaration: `{}` line {}, column {} (bytes {}..{})\n",
                e.declaration.source_sha256,
                e.declaration.line,
                e.declaration.column,
                e.declaration.start,
                e.declaration.end
            ));
        }
    }
    out
}
