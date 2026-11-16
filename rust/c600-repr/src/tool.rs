//! The representations tool: its source, its module manifest with the class `view_aid` (BND.27, BND.37), the review of
//! the class against the exact entry definitions and the row set (CT-LB6-03, CT-BND-15), and the form of a review
//! record that names the entry digests. The admission of each entry is `c600_tools::tool::admit_built_in`, the
//! built-in column of chapter 07 section 8; nothing of it is copied here.

use std::collections::{BTreeMap, BTreeSet};

use c600_check::{check, Admission, Grant, IrError, Policy};
use c600_command::{Kind, PreviewRule, Row, Table};
use c600_digest::sha256_hex;
use c600_ir::{Effect, FnKind, Function, Program};
use c600_tools::tool::{admit_built_in, lower_source, targets, BoundaryClass};

/// The tool's source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const SOURCE: &str = include_str!("representations.6cs");
/// The tool's module.
pub const MODULE: &str = "representations";
/// The tool's rows: LB6.5's row "Representation, work-set and filter tools".
pub const ROWS: [&str; 6] =
    ["filter.apply", "filter.preview", "set.save", "structure.compare", "structure.layer", "view.effect.scope"];

/// One entry of the tool: an exported function of the module and the command-layer row it serves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: &'static str,
    pub row: &'static str,
    pub kind: FnKind,
}

/// The tool's entries. `set.save` has none: see [`UNEXPRESSED`].
pub const ENTRIES: [Entry; 5] = [
    Entry { name: "filter_counts", row: "filter.preview", kind: FnKind::View },
    Entry { name: "apply_filter", row: "filter.apply", kind: FnKind::Command },
    Entry { name: "effect_scope", row: "view.effect.scope", kind: FnKind::Command },
    Entry { name: "compare_cells", row: "structure.compare", kind: FnKind::Command },
    Entry { name: "layer_predicate", row: "structure.layer", kind: FnKind::Command },
];

/// The rows with no entry, each with what the IR draft `c600-ir-draft-1` lacks for it.
pub const UNEXPRESSED: [(&str, &str); 1] = [(
    "set.save",
    "its `members` argument is a list of canonical object references, and the IR draft has no list type",
)];

/// The fields of the tool's module manifest that the review reads (BND.27; LG5.2 and PS1.15 define the manifest). The
/// IR draft and the syntax carry no manifest yet, so it is data here, as in `c600-tools`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub tool: &'static str,
    /// The LB6.5 candidate the tool is.
    pub candidate: &'static str,
    pub module: &'static str,
    /// The declared class (BND.37).
    pub class: BoundaryClass,
    /// The command-layer rows the tool serves.
    pub rows: Vec<&'static str>,
    pub entries: Vec<Entry>,
    /// The SHA-256 of the module's source, in hex.
    pub source_sha256: String,
    /// Whether a `PrincipleRecord` exists (LB6.3). None does yet (LB6.5).
    pub principle_record: bool,
}

/// The representations tool's manifest.
pub fn manifest() -> Manifest {
    Manifest {
        tool: "representations tool",
        candidate: "Representation, work-set and filter tools",
        module: MODULE,
        class: BoundaryClass::ViewAid,
        rows: ROWS.to_vec(),
        entries: ENTRIES.to_vec(),
        source_sha256: sha256_hex(SOURCE.as_bytes()),
        principle_record: false,
    }
}

/// The digest a review record names for one entry: the SHA-256 of a line naming this draft, the module, the entry and
/// the SHA-256 of the module's source bytes. It binds the entry to the whole source, so any edit of the module changes
/// every entry digest.
pub fn entry_digest(source_sha256: &str, module: &str, entry: &str) -> String {
    sha256_hex(format!("c600-repr entry digest draft 1\n{module}\n{entry}\n{source_sha256}\n").as_bytes())
}

impl Manifest {
    /// The entry digests of the manifest's entries, by entry name.
    pub fn entry_digests(&self) -> BTreeMap<String, String> {
        self.entries
            .iter()
            .map(|e| (e.name.to_owned(), entry_digest(&self.source_sha256, self.module, e.name)))
            .collect()
    }
}

/// Admits one entry for a run in the solving profile under the built-in column, with no capability granted and the
/// interim policy (BND.54).
pub fn admit(program: &Program, module: &str, entry: &str) -> Result<Admission, IrError> {
    admit_built_in(program, module, entry, &Grant::none(), &Policy::interim())
}

/// What a class review found outside the class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// The declared class is not `view_aid`: this crate reviews that class only.
    ClassNotReviewed,
    /// The checker reports a diagnostic for the program.
    CheckerDiagnostic,
    /// The manifest names an entry the module does not define.
    EntryMissing,
    /// The entry is not exported, or its kind differs from the manifest's.
    EntryDefinition,
    /// The module exports a function the manifest does not list: the review covers every exported function, so no
    /// entry escapes it by being left out of the manifest.
    EntryUnlisted,
    /// The built-in column refuses the entry.
    ColumnRefusal,
    /// The entry's row is not one of the tool's rows, or the tool's row is not in the stored command table.
    RowOutsideTool,
    /// The row lies outside the class: a commit, application or file row, a row that takes a user-action token, or a
    /// row that needs a commit preview.
    RowOutsideClass,
    /// The entry prepares a preview: a view aid chooses no target (BND.63).
    ChoosesTarget,
    /// The entry requests a capability.
    Capability,
    /// The entry changes the workspace and is no command, or serves a row that is no workspace row.
    WorkspaceOutsideRow,
    /// The entry changes the workspace and reads state-derived data: what it stores is not what the solver named.
    StoresStateDerived,
    /// The entry has an effect a view aid has no use for: a file, randomness or budget effect.
    EffectOutsideClass,
    /// The source bytes are not the ones whose SHA-256 the manifest names ([`review_source`]).
    SourceDigest,
    /// The source does not parse or lower ([`review_source`]).
    SourceUnlowered,
}

/// One finding of a class review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule: Rule,
    pub entry: Option<String>,
    pub row: Option<String>,
    pub note: String,
}

fn finding(rule: Rule, entry: Option<&str>, row: Option<&str>, note: impl Into<String>) -> Finding {
    Finding { rule, entry: entry.map(str::to_owned), row: row.map(str::to_owned), note: note.into() }
}

/// Whether a row of the stored command table lies within the class `view_aid`: it changes what is shown or the
/// workspace, or answers a query, and it is no commit path.
pub fn row_within_view_aid(row: &Row) -> bool {
    matches!(row.kind, Kind::Query | Kind::View | Kind::Workspace)
        && !row.takes_token()
        && row.preview != PreviewRule::Required
}

fn function<'p>(program: &'p Program, module: &str, entry: &str) -> Option<&'p Function> {
    program.modules.iter().filter(|m| m.name == module).flat_map(|m| &m.functions).find(|f| f.name == entry)
}

/// Reviews the manifest's class against the exact entry definitions in `program` and against the row set in `table`
/// (BND.37). Every exported function of the tool's module is reviewed, and one the manifest does not list is a
/// finding. An empty result means that nothing lies outside the class as this draft reads it; it is the mechanical
/// part of a review and no review record. A program carries no source digest: [`review_source`] reviews the source
/// bytes the manifest names.
pub fn review_class(program: &Program, manifest: &Manifest, table: &Table) -> Result<Vec<Finding>, IrError> {
    let mut out = Vec::new();
    if manifest.class != BoundaryClass::ViewAid {
        out.push(finding(Rule::ClassNotReviewed, None, None, manifest.class.name()));
        return Ok(out);
    }
    let report = check(program)?;
    for d in &report.diagnostics {
        out.push(finding(Rule::CheckerDiagnostic, Some(d.function.as_str()), None, d.code));
    }
    for row in &manifest.rows {
        match table.row(row) {
            None => out.push(finding(Rule::RowOutsideTool, None, Some(row), "no such row in the command table")),
            Some(found) if !row_within_view_aid(found) => {
                out.push(finding(Rule::RowOutsideClass, None, Some(row), format!("a {:?} row", found.kind)))
            }
            Some(_) => {}
        }
    }
    for entry in &manifest.entries {
        let (name, row) = (Some(entry.name), Some(entry.row));
        if !manifest.rows.contains(&entry.row) {
            out.push(finding(Rule::RowOutsideTool, name, row, "the entry's row is not one of the tool's"));
        }
        let Some(f) = function(program, manifest.module, entry.name) else {
            out.push(finding(Rule::EntryMissing, name, row, "the module defines no such function"));
            continue;
        };
        if !f.exported || f.kind != entry.kind {
            out.push(finding(Rule::EntryDefinition, name, row, format!("exported {}, kind {:?}", f.exported, f.kind)));
        }
        let Some(summary) = report.summary(manifest.module, entry.name) else {
            out.push(finding(Rule::EntryMissing, name, row, "the checker has no summary for it"));
            continue;
        };
        let admission = admit(program, manifest.module, entry.name)?;
        let refusals = admission.diagnostics.len() - report.diagnostics.len();
        if refusals > 0 {
            out.push(finding(Rule::ColumnRefusal, name, row, format!("{refusals} refusal(s) of the built-in column")));
        }
        let proposes = targets(program, manifest.module, entry.name).map_or(true, |found| !found.is_empty());
        if proposes || summary.effects.contains(&Effect::PreparePreview) || !summary.unresolved.is_empty() {
            out.push(finding(
                Rule::ChoosesTarget,
                name,
                row,
                "the entry prepares a preview or calls what it cannot name",
            ));
        }
        if !summary.capabilities.is_empty() || !summary.vars.is_empty() {
            out.push(finding(Rule::Capability, name, row, format!("{:?}", summary.capabilities)));
        }
        let outside = [Effect::GrantedFile, Effect::SeededRandomness, Effect::Budgets];
        if outside.iter().any(|e| summary.effects.contains(e)) {
            out.push(finding(Rule::EffectOutsideClass, name, row, format!("{:?}", summary.effects)));
        }
        if summary.effects.contains(&Effect::ChangeWorkspace) {
            let workspace_row = table.row(entry.row).is_some_and(|r| r.kind == Kind::Workspace);
            if f.kind != FnKind::Command || !workspace_row {
                out.push(finding(
                    Rule::WorkspaceOutsideRow,
                    name,
                    row,
                    "a workspace change outside a workspace command",
                ));
            }
            if !summary.state_reads.is_empty() {
                out.push(finding(Rule::StoresStateDerived, name, row, format!("{:?}", summary.state_reads)));
            }
        }
    }
    let listed: BTreeSet<&str> = manifest.entries.iter().map(|e| e.name).collect();
    let exported = program.modules.iter().filter(|m| m.name == manifest.module).flat_map(|m| &m.functions);
    for f in exported.filter(|f| f.exported && !listed.contains(f.name.as_str())) {
        let note = "an exported function the manifest does not list";
        out.push(finding(Rule::EntryUnlisted, Some(f.name.as_str()), None, note));
    }
    Ok(out)
}

/// Reviews the tool's source bytes: they must be the bytes whose SHA-256 the manifest names, and they must parse and
/// lower; the program they lower to then gets [`review_class`]. Other bytes are found even when they lower to an
/// equal program, because a review record names the source digest.
pub fn review_source(source: &str, manifest: &Manifest, table: &Table) -> Result<Vec<Finding>, IrError> {
    let mut out = Vec::new();
    if sha256_hex(source.as_bytes()) != manifest.source_sha256 {
        out.push(finding(Rule::SourceDigest, None, None, "the source is not the one the manifest names"));
    }
    match lower_source(source) {
        Ok(program) => out.extend(review_class(&program, manifest, table)?),
        Err(error) => out.push(finding(Rule::SourceUnlowered, None, None, error)),
    }
    Ok(out)
}

/// The form of a review record for the tool (CT-BND-15): who reviewed, the class, the source digest, the digest of
/// every entry and the row set. No review of the shipped tool exists yet (BM5); this type fixes what one must name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewRecord {
    pub tool: String,
    pub reviewer: String,
    pub class: String,
    pub source_sha256: String,
    /// The entry digests, by entry name.
    pub entries: BTreeMap<String, String>,
    pub rows: BTreeSet<String>,
}

impl ReviewRecord {
    /// What a record for `manifest` names, with no reviewer: a form to fill in, not a review. [`check_record`]
    /// refuses a record whose reviewer is empty or blank.
    pub fn form(manifest: &Manifest) -> ReviewRecord {
        ReviewRecord {
            tool: manifest.tool.to_owned(),
            reviewer: String::new(),
            class: manifest.class.name().to_owned(),
            source_sha256: manifest.source_sha256.clone(),
            entries: manifest.entry_digests(),
            rows: manifest.rows.iter().map(|r| (*r).to_owned()).collect(),
        }
    }
}

/// Why a review record does not cover a manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordFault {
    NoReviewer,
    Tool,
    Class,
    Source,
    /// The record names no digest for this entry.
    EntryNotNamed(String),
    /// The record names another digest for this entry.
    EntryDigest(String),
    /// The record names an entry the manifest does not have.
    EntryUnknown(String),
    Rows,
}

/// Whether `record` names `manifest`: its tool, class, source digest, every entry's digest and its row set. An empty
/// result means it does. A record for other source bytes names other entry digests and is refused.
pub fn check_record(record: &ReviewRecord, manifest: &Manifest) -> Vec<RecordFault> {
    let mut faults = Vec::new();
    if record.reviewer.trim().is_empty() {
        faults.push(RecordFault::NoReviewer);
    }
    if record.tool != manifest.tool {
        faults.push(RecordFault::Tool);
    }
    if record.class != manifest.class.name() {
        faults.push(RecordFault::Class);
    }
    if record.source_sha256 != manifest.source_sha256 {
        faults.push(RecordFault::Source);
    }
    let expected = manifest.entry_digests();
    for (entry, digest) in &expected {
        match record.entries.get(entry) {
            None => faults.push(RecordFault::EntryNotNamed(entry.clone())),
            Some(named) if named != digest => faults.push(RecordFault::EntryDigest(entry.clone())),
            Some(_) => {}
        }
    }
    for entry in record.entries.keys() {
        if !expected.contains_key(entry) {
            faults.push(RecordFault::EntryUnknown(entry.clone()));
        }
    }
    let rows: BTreeSet<String> = manifest.rows.iter().map(|r| (*r).to_owned()).collect();
    if record.rows != rows {
        faults.push(RecordFault::Rows);
    }
    faults
}
