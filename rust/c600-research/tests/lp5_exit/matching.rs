//! A check report matched with its record's own meaning, scope, original labels and check mode through R0's import:
//! `import_archive` over the Git-blob manifest with import table edition 1 and discriminator edition 1, the members
//! granted from the checkout, read in place.
//!
//! The reading used here (the plan's section 2, item 1; PHS.31's "compare with the format-specific recorded meaning,
//! including scope, original label and check mode"):
//!
//! - *Meaning*: the group's typed statement, whose ID is the digest of its content (family, record path and bytes,
//!   pointers, form), on the very bytes the check read: the record is the report's certificate (or, entering as an input
//!   or a table, one of its bound inputs) with the edition's pinned digest, and R0 imported those bytes with every
//!   integrity check passing and recognised its format, or its absence, as the archive has it.
//! - *Scope*: the group's scope phrase is EVD.36's phrase for one instance, the scope of a KN6 report on these bytes
//!   (the same phrase for every group, since every edition 1 statement covers one instance). The import table's own
//!   scope facts at the group's pointers must be finite ([`FINITE_SCOPES`]): a record stating every reachable state or
//!   an admitted domain there would not match a one-instance check.
//! - *Original labels*: every enumerated occurrence at the group's pointers (the table's facts, recorded with the
//!   table's reason, or unmapped) holds the record's value word for word: the value read again at each of its JSON
//!   pointers is the imported value, byte for byte in canonical form, so the import's pointers name the labels the
//!   record holds. None of them contradicts the pass: no claim fact `refuted`, no job outcome `error`, `cancelled` or
//!   `cut off`, and no verdict boolean with facts that is false. A record with no enumerated label at the group's
//!   pointers (numbers and prose only) has none to match, and is counted.
//! - *Check mode*: the table's check-mode facts at the group's pointers are the record's own (a producer's actual
//!   replay, a fresh audit). They are kept as the record's and never replaced or compared for equality: the KN6 report
//!   is a new, independent check beside them. The sweep counts them exactly, so a change in what the import says shows.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::sync::OnceLock;

use c600_archive::bindings::{Binding, Bindings};
use c600_archive::import::{ImportReport, OccurrenceOutcome, Recognition};
use c600_archive::table::Outcome;
use c600_archive::{import_archive, Discriminator, GrantedFile, GrantedFiles, ImportTable, ImportTableRef};
use c600_archive::{Limits, LimitsRef};
use c600_checkers::CheckReport;

use crate::support::{read, sha};

/// The Git-blob manifest of the stage 14 to 19 archive (PHS.32's import baseline).
pub const CORPUS_MANIFEST: &str = "work/experiments/language-lx/envelope-facts/MANIFEST.json";

pub fn table() -> &'static ImportTable {
    static TABLE: OnceLock<ImportTable> = OnceLock::new();
    TABLE.get_or_init(ImportTable::edition_1)
}

/// R0's import of the archive, once per test binary.
pub fn import() -> &'static ImportReport {
    static REPORT: OnceLock<ImportReport> = OnceLock::new();
    REPORT.get_or_init(|| {
        let manifest = read(CORPUS_MANIFEST).expect("the Git-blob manifest");
        let doc = c600_checkers::claims::document(&manifest).ok().expect("the manifest is JSON");
        let mut members = Vec::new();
        for list in ["records", "auxiliary_corpus"] {
            for entry in doc.get(list).and_then(|l| l.as_array()).expect(list) {
                let name = entry.get("path").and_then(|p| p.as_str()).expect("a path").to_owned();
                if let Some(bytes) = read(&name) {
                    members.push(GrantedFile { name, bytes });
                }
            }
        }
        let discriminator = Discriminator::edition_1();
        let limits = Limits::research_archive();
        let report = import_archive(
            GrantedFiles { manifest: GrantedFile { name: CORPUS_MANIFEST.to_owned(), bytes: manifest }, members },
            ImportTableRef { table: table(), discriminator: &discriminator },
            LimitsRef(&limits),
        )
        .expect("the archive imports");
        assert_eq!(report.refusal(), None, "every member passes its integrity checks");
        report
    })
}

/// One occurrence at a group's pointers, with the record's value word for word.
#[derive(Clone, Debug)]
pub struct Label {
    pub pointer: String,
    pub field: String,
    /// The value, as canonical JSON.
    pub value: String,
    /// `facts`, `recorded` or `unmapped`.
    pub outcome: &'static str,
    /// The table rule's reason word for word (its label rule and note), or the import's reason word.
    pub reason: String,
}

/// What the import says about the record of one group, at the group's pointers.
#[derive(Clone, Debug, Default)]
pub struct Meaning {
    pub recognition: String,
    pub labels: Vec<Label>,
    pub claims: BTreeSet<String>,
    pub scopes: BTreeSet<String>,
    pub checks: BTreeSet<String>,
    pub check_modes: BTreeSet<String>,
    pub job_outcomes: BTreeSet<String>,
    /// Labels that would contradict a pass.
    pub contradictions: Vec<String>,
}

fn in_scope(pointer: &str, scope: &[String]) -> bool {
    scope.iter().any(|p| p.is_empty() || pointer == p || pointer.starts_with(&format!("{p}/")))
}

/// The record's meaning at the group's pointers, through the import. A record the import does not hold with passing
/// integrity, under the edition's pin, is a test failure.
pub fn meaning(bindings: &Bindings, row: &Binding) -> Result<Meaning, String> {
    let report = import();
    let pin = bindings.file(row.source);
    let at = report.member(&pin.path).ok_or_else(|| format!("{}: not in the Git-blob manifest", pin.path))?;
    let member = &report.members[at];
    if member.integrity() != "pass" {
        return Err(format!("{}: import integrity {} ({:?})", pin.path, member.integrity(), report.failed_checks(at)));
    }
    if member.stored_sha256.as_deref() != Some(pin.sha256.as_str()) {
        return Err(format!("{}: imported bytes are not the edition's pin", pin.path));
    }
    let recognition = match &member.recognition {
        Recognition::Declared(format) => format!("format {format}"),
        Recognition::Absent(d) => format!("absent format ({d:?})"),
        Recognition::Referenced { by, encoding, .. } => format!("member of {by} ({encoding})"),
        other => return Err(format!("{}: recognised as {other:?}", pin.path)),
    };
    let stored = report.stored_bytes(at).expect("a granted member");
    let decoded = if pin.path.ends_with(".gz") {
        c600_checkers::archive::s19::common::decode(stored, 1 << 26).map_err(|e| e.to_string())?.into_owned()
    } else {
        stored.to_vec()
    };
    if member.decoded_sha256.as_deref() != Some(sha(&decoded).as_str()) {
        return Err(format!("{}: decoded bytes are not the import's", pin.path));
    }
    let mut out = Meaning { recognition, ..Meaning::default() };
    let doc = c600_checkers::claims::document(&decoded).ok();
    for occurrence in report.occurrences.iter().filter(|o| o.member == at) {
        let pointers: Vec<&String> = occurrence.pointers.iter().filter(|p| in_scope(p, &row.pointers)).collect();
        if pointers.is_empty() {
            continue;
        }
        let value = occurrence.value.canonical();
        // Word for word: the value read again at every pointer is the imported value.
        let doc = doc.as_ref().ok_or_else(|| format!("{}: occurrences in a record that is not JSON", pin.path))?;
        for pointer in &pointers {
            let again = c600_archive::discriminator::pointer_get(doc, pointer)
                .ok_or_else(|| format!("{}: no value at {pointer}", pin.path))?
                .canonical();
            if again != value {
                return Err(format!("{}: {pointer} holds {again}, the import kept {value}", pin.path));
            }
        }
        let (outcome, reason, facts) = match &occurrence.outcome {
            OccurrenceOutcome::Facts { rule } => {
                let rule = &table().rules()[*rule];
                let Outcome::Facts(facts) = &rule.outcome else {
                    return Err(format!("{}: a facts outcome whose rule records", pin.path));
                };
                ("facts", rule.reason.clone(), Some(facts.clone()))
            }
            OccurrenceOutcome::Recorded { reason, .. } => ("recorded", reason.clone(), None),
            OccurrenceOutcome::Unmapped { reason } => ("unmapped", (*reason).to_owned(), None),
        };
        if let Some(f) = &facts {
            out.claims.extend(f.claim.clone());
            out.scopes.extend(f.scope.clone());
            out.checks.extend(f.check.clone());
            out.check_modes.extend(f.check_mode.clone());
            out.job_outcomes.extend(f.job_outcome.clone());
            let contradicts = f.claim.as_deref() == Some("refuted")
                || matches!(f.job_outcome.as_deref(), Some("error" | "cancelled" | "cut off"))
                || (occurrence.class == c600_archive::enumerate::Class::B && value == "false");
            if contradicts {
                out.contradictions.push(format!("{} = {value} ({reason})", occurrence.field));
            }
        }
        for pointer in pointers {
            out.labels.push(Label {
                pointer: pointer.clone(),
                field: occurrence.field.clone(),
                value: value.clone(),
                outcome,
                reason: reason.clone(),
            });
        }
    }
    Ok(out)
}

/// The report's identity against the check that was prepared: the registered sub-checker's ID and source digest, the
/// research-archive kind under that ID, and the certificate's digest as the input digest.
pub fn identity_problems(prepared: &crate::roles::Prepared, report: &CheckReport) -> Vec<String> {
    let mut problems = Vec::new();
    let sub = c600_checkers::archive::find(prepared.id).expect("registered");
    if report.checker != sub.checker.sources.identity() {
        problems.push(format!("checker {:?} is not {}'s registered identity", report.checker, prepared.id));
    }
    if report.kind != prepared.kind() {
        problems.push(format!("kind {:?} is not {:?}", report.kind, prepared.kind()));
    }
    if report.input != prepared.digest || report.input != sha(&prepared.certificate) {
        problems.push(format!("input digest {} is not the certificate's {}", report.input, prepared.digest));
    }
    if report.trace.is_empty() {
        problems.push("an empty trace".into());
    }
    problems
}

/// The scopes a record's own facts may state at the pointers of a group a one-instance check covers: one finite
/// instance, or a declared finite search set (a census the checker enumerates, or a chain's stop the trace leaves out of
/// its scope). Every reachable state or an admitted domain would be a universal claim no finite check matches.
pub const FINITE_SCOPES: [&str; 2] = ["one finite instance", "declared search set"];

/// The report matched with the group's binding and its record's meaning: the problems found, empty when it matches.
pub fn match_report(
    bindings: &Bindings,
    row: &Binding,
    prepared: &crate::roles::Prepared,
    report: &CheckReport,
    meaning: &Meaning,
) -> Vec<String> {
    let mut problems = identity_problems(prepared, report);
    let pin = bindings.file(row.source);
    match &prepared.record_role {
        // The record is the certificate: its pinned bytes, or for a stored gzip member the bytes its stage manifest
        // decodes them to.
        None => {
            if !pin.path.ends_with(".gz") && report.input != pin.sha256 {
                problems.push(format!("the certificate is not the record's pinned bytes {}", pin.sha256));
            }
        }
        // The record is an input: the report binds it, by its role and its pinned digest.
        Some(role) => {
            if !report.inputs.iter().any(|i| i.role == *role && i.sha256 == pin.sha256) {
                problems.push(format!("the report does not bind the record as {role} with {}", pin.sha256));
            }
        }
    }
    if c600_digest::sha256_hex(bindings.statement_content(row).as_bytes()) != row.statement.id
        || row.statement.input != pin.sha256
    {
        problems.push("the statement is not the record's".into());
    }
    if row.scope_phrase != c600_evidence::scope_phrase(&c600_evidence::Covers::OneInstance, &[]) {
        problems.push(format!("scope phrase {:?} is not EVD.36's for one instance", row.scope_phrase));
    }
    for scope in &meaning.scopes {
        if !FINITE_SCOPES.contains(&scope.as_str()) {
            problems.push(format!("the record states the scope {scope:?}, which a one-instance check does not match"));
        }
    }
    for contradiction in &meaning.contradictions {
        problems.push(format!("the record's own label contradicts a pass: {contradiction}"));
    }
    problems
}
