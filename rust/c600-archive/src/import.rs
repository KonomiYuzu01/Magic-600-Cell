//! PS3.17's `import_archive`: granted files in, an import report out, with recognition, integrity, interpretation and
//! recheck kept as separate results (PS3.10).
//!
//! - *Inventory* comes from the granted corpus manifest (`ps1.38-git-manifest/1`, the Git-blob manifest the test driver
//!   reads). Every listed record and auxiliary member is counted; a listed member that was not granted is reported
//!   missing and stays counted (PS3.15). A granted file the manifest does not list is reported unlisted and not read.
//! - *Identity* is the stored-byte digest. A name is metadata that pairs a manifest entry with a granted file by exact
//!   text; names that differ only in case stay distinct, and nothing is extracted onto a path (PS3.15, PS3.16).
//! - *Integrity* checks stored bytes, the gzip stream, decoded bytes and the canonical digest against the corpus
//!   manifest, PS3.14's stage 19 manifest rows against the members they name, and the word digests a run record
//!   states for its `ia16` members (PS3.13). A source-edition digest is kept as metadata and is never compared with
//!   public bytes.
//! - *Decoding* follows the member's declared form: a name ending in `.gz` declares a gzip stream, and the stream is
//!   checked whatever its first bytes are, so a damaged signature is a failed `gzip_stream` check. The stage 19
//!   manifest's `transformation` texts declare no compression, so the name is the only declaration in this corpus.
//! - *Recognition* is the record's own top-level `format`, or its absence with the discriminator's answer (PS3.11,
//!   PS3.12). A `format` key whose value is not a non-empty string is not an absence: the record is recognised as
//!   having an invalid format, with the value kept word for word. An auxiliary member is recognised only when a run
//!   record names it with an encoding.
//! - *Interpretation* enumerates every (format, field, value) occurrence (EVD.48) and gives each exactly one outcome:
//!   the table's facts, recorded with a reason, or unmapped with its value kept word for word.
//! - *Recheck* is never run here: KN6's checkers own mathematical rechecks, and no archived producer runs.

use std::collections::{BTreeMap, BTreeSet};

use c600_digest::sha256_hex;
use c600_kernel::archive::gunzip;
use c600_kernel::json::{parse, Value};

use crate::discriminator::Discrimination;
use crate::enumerate::{last_key, leaves, Class};
use crate::table::{ImportTableRef, Outcome};
use crate::ArchiveError;

/// The corpus manifest format this importer reads.
pub const CORPUS_MANIFEST_FORMAT: &str = "ps1.38-git-manifest/1";
/// Stage 19's manifest format (PS3.14).
pub const STAGE_MANIFEST_FORMAT: &str = "sanitized-research-archive/1";
/// The run record format whose steps name `ia16` word members.
pub const RUN_RECORD_FORMAT: &str = "runner-run/3";

/// Why an occurrence is recorded or unmapped; reason words, not error codes.
pub const ABSENT_FORMAT: &str =
    "absent format: no reviewed discriminator rule maps this occurrence (PS3.12, FP.22, EVD.53); kept word for word";
pub const NO_RULE: &str = "no rule in this edition for this (format, field, value); kept word for word";
pub const OVER_LIMIT: &str = "the (format, field) has more distinct values than EVD.48's limit; kept word for word";
pub const CLASS_DIFFERS: &str = "the table's class for this triple differs from the enumeration's; kept word for word";
pub const PREREQUISITE_UNMET: &str = "a rule prerequisite is unmet; kept word for word";
pub const INVALID_FORMAT: &str =
    "the record's top-level format is not a non-empty string, so no rule applies (PS3.11); kept word for word";

/// The recheck result of every member: R0 runs no mathematical check.
pub const RECHECK_NOT_RUN: &str =
    "not run: import runs no mathematical recheck and executes no producer (PS3.10, PS3.17)";

/// One granted file: its source name (metadata) and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// PS3.17's `GrantedFiles`: the corpus manifest and the members it may name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantedFiles {
    pub manifest: GrantedFile,
    pub members: Vec<GrantedFile>,
}

/// The caller's limits; every output and every decoded byte is bounded by them (FP.9's discipline).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_members: usize,
    pub max_manifest_bytes: usize,
    pub max_member_stored_bytes: usize,
    pub max_total_stored_bytes: usize,
    pub max_member_decoded_bytes: usize,
    pub max_total_decoded_bytes: usize,
    pub max_occurrences: usize,
}

impl Limits {
    /// Limits that admit the stage 14 to 19 corpus (632 members, 13.4 MB stored, 59 MB of decoded records) with room.
    pub fn research_archive() -> Limits {
        Limits {
            max_members: 4096,
            max_manifest_bytes: 4 << 20,
            max_member_stored_bytes: 64 << 20,
            max_total_stored_bytes: 256 << 20,
            max_member_decoded_bytes: 64 << 20,
            max_total_decoded_bytes: 256 << 20,
            max_occurrences: 200_000,
        }
    }
}

/// PS3.17's `LimitsRef`.
#[derive(Clone, Copy, Debug)]
pub struct LimitsRef<'a>(pub &'a Limits);

/// A listed member's role in the corpus manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Record,
    Auxiliary,
}

/// The result of one integrity check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckResult {
    Pass,
    Fail,
}

/// One integrity check: what was compared, against which record, and the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityCheck {
    /// `stored_sha256`, `stored_bytes`, `gzip_stream`, `decoded_sha256`, `decoded_bytes`, `canonical_sha256`.
    pub check: &'static str,
    /// The record that states the expected value, with its field.
    pub against: String,
    pub expected: String,
    pub actual: String,
    pub result: CheckResult,
}

/// Recognition of one member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recognition {
    /// The member was not granted.
    NotAvailable,
    /// The record's own top-level `format`.
    Declared(String),
    /// No top-level `format` (PS3.11), with the discriminator's answer kept separately.
    Absent(Discrimination),
    /// A top-level `format` whose value is not a non-empty string, kept word for word: malformed, not absent.
    InvalidFormat(Value),
    /// The bytes are not a strict JSON record; the reason never quotes them.
    Unparsed(String),
    /// An auxiliary member that a run record names, with the encoding that record states.
    Referenced { by: String, pointer: String, encoding: String },
    /// An auxiliary member nothing recognises; its bytes are kept and counted.
    Unrecognised,
}

/// One occurrence group's outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OccurrenceOutcome {
    /// The table's facts, by rule index.
    Facts { rule: usize },
    /// Recorded with its reason: a rule's own reason, or one of this module's reason words.
    Recorded { rule: Option<usize>, reason: String },
    /// No rule: the value stays word for word with this reason.
    Unmapped { reason: &'static str },
}

/// The occurrences of one (format, field, value) in one member, with every exact pointer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrences {
    pub member: usize,
    pub format: Option<String>,
    pub field: String,
    pub value: Value,
    pub class: Class,
    pub outcome: OccurrenceOutcome,
    pub pointers: Vec<String>,
}

/// One member's interpretation counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Interpretation {
    pub occurrences: usize,
    pub facts: usize,
    pub recorded: usize,
    pub unmapped: usize,
    pub absent_format: usize,
    pub prose: usize,
    pub payload_booleans: usize,
}

/// The manifest's statement about one member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub git_blob: String,
    pub stored_sha256: String,
    pub stored_bytes: u64,
    pub decoded_sha256: Option<String>,
    pub decoded_bytes: Option<u64>,
    pub canonical_sha256: Option<String>,
    pub canonical_bytes: Option<u64>,
}

/// One listed member's report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberReport {
    pub name: String,
    pub role: Role,
    pub listed: Listed,
    pub present: bool,
    pub stored_sha256: Option<String>,
    pub decoded_sha256: Option<String>,
    pub checks: Vec<IntegrityCheck>,
    pub recognition: Recognition,
    pub interpretation: Option<Interpretation>,
    pub recheck: &'static str,
}

impl MemberReport {
    /// `not_available` for a missing member, else `fail` if any check failed, else `pass`.
    pub fn integrity(&self) -> &'static str {
        if !self.present {
            "not_available"
        } else if self.checks.iter().any(|c| c.result == CheckResult::Fail) {
            "fail"
        } else {
            "pass"
        }
    }
}

/// One row of a stage manifest (PS3.14).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestRow {
    pub path: String,
    pub member: Option<usize>,
    pub original_name: Option<String>,
    /// The captured private-source edition's digest: metadata, never compared with public bytes.
    pub original_sha256: Option<String>,
    pub transformation: Option<String>,
    /// `checked`, `missing` (the named member is not granted) or `unlisted` (the corpus manifest does not list it).
    pub status: &'static str,
}

/// PS3.14's check of one `sanitized-research-archive/1` manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageManifest {
    pub member: usize,
    /// The manifest's top-level fields other than `files`, word for word.
    pub fields: BTreeMap<String, Value>,
    pub rows: Vec<ManifestRow>,
    /// Members in the manifest's directory tree that it does not list.
    pub not_listed: Vec<String>,
}

/// Names that differ only in case, with each name's stored digest (granted bytes, or the listed digest when missing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseGroup {
    pub names: Vec<(String, String)>,
}

impl CaseGroup {
    pub fn distinct_digests(&self) -> bool {
        let digests: BTreeSet<&String> = self.names.iter().map(|(_, d)| d).collect();
        digests.len() == self.names.len()
    }
}

/// PS3.17's `ImportReport`.
#[derive(Clone, Debug)]
pub struct ImportReport {
    pub table_edition: String,
    pub table_sha256: String,
    pub discriminator_edition: String,
    pub discriminator_sha256: String,
    pub discriminator_review: String,
    pub manifest_name: String,
    pub manifest_sha256: String,
    pub manifest_checks: Vec<IntegrityCheck>,
    pub members: Vec<MemberReport>,
    pub occurrences: Vec<Occurrences>,
    /// Prose fields kept word for word: (member, format, field) and their count.
    pub prose: BTreeMap<(usize, Option<String>, String), usize>,
    /// Payload booleans by last key and value.
    pub payload_booleans: BTreeMap<(String, bool), usize>,
    pub stage_manifests: Vec<StageManifest>,
    pub case_only_names: Vec<CaseGroup>,
    pub unlisted: Vec<(String, String)>,
    pub limits: Limits,
    stored: Vec<Option<Vec<u8>>>,
}

/// Imports a granted archive. Integrity failures are results in the report, never repairs; the operation itself
/// refuses only a corpus manifest it cannot read and a limit it would exceed.
pub fn import_archive(
    input: GrantedFiles,
    table: ImportTableRef<'_>,
    limits: LimitsRef<'_>,
) -> Result<ImportReport, ArchiveError> {
    let limits = limits.0;
    if input.manifest.bytes.len() > limits.max_manifest_bytes {
        return Err(ArchiveError::limit("manifest_stored", limits.max_manifest_bytes, input.manifest.bytes.len()));
    }
    if input.members.len() > limits.max_members {
        return Err(ArchiveError::limit("members", limits.max_members, input.members.len()));
    }
    let mut granted: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut stored_total = 0usize;
    for file in input.members {
        if file.bytes.len() > limits.max_member_stored_bytes {
            return Err(ArchiveError::limit("member_stored", limits.max_member_stored_bytes, file.bytes.len()));
        }
        stored_total = stored_total.saturating_add(file.bytes.len());
        if stored_total > limits.max_total_stored_bytes {
            return Err(ArchiveError::limit("total_stored", limits.max_total_stored_bytes, stored_total));
        }
        if granted.insert(file.name, file.bytes).is_some() {
            return Err(ArchiveError::format("two granted files have the same name"));
        }
    }

    let manifest_sha256 = sha256_hex(&input.manifest.bytes);
    let manifest =
        parse(&input.manifest.bytes).map_err(|_| ArchiveError::format("the corpus manifest is not strict JSON"))?;
    if manifest.get("format").and_then(Value::as_str) != Some(CORPUS_MANIFEST_FORMAT) {
        return Err(ArchiveError::format("the corpus manifest is not a ps1.38 Git manifest"));
    }
    let records = manifest.get("records").and_then(Value::as_array).ok_or(ArchiveError::format("no records"))?;
    let auxiliary = manifest
        .get("auxiliary_corpus")
        .and_then(Value::as_array)
        .ok_or(ArchiveError::format("no auxiliary corpus"))?;
    let mut entries: Vec<(String, Role, Listed)> = Vec::new();
    for (list, role) in [(records, Role::Record), (auxiliary, Role::Auxiliary)] {
        for e in list {
            entries.push((need_text(e, "path")?, role, listed(e, role)?));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    if entries.windows(2).any(|w| w[0].0 == w[1].0) {
        return Err(ArchiveError::format("the corpus manifest lists a path twice"));
    }
    if entries.len() > limits.max_members {
        return Err(ArchiveError::limit("members", limits.max_members, entries.len()));
    }
    let manifest_checks = corpus_manifest_checks(&manifest, records, &entries);

    let mut report = ImportReport {
        table_edition: table.table.edition.clone(),
        table_sha256: table.table.sha256.clone(),
        discriminator_edition: table.discriminator.edition.clone(),
        discriminator_sha256: table.discriminator.sha256.clone(),
        discriminator_review: table.discriminator.review.clone(),
        manifest_name: input.manifest.name.clone(),
        manifest_sha256: manifest_sha256.clone(),
        manifest_checks,
        members: Vec::new(),
        occurrences: Vec::new(),
        prose: BTreeMap::new(),
        payload_booleans: BTreeMap::new(),
        stage_manifests: Vec::new(),
        case_only_names: Vec::new(),
        unlisted: Vec::new(),
        limits: limits.clone(),
        stored: Vec::new(),
    };
    let listed_names: BTreeSet<String> = entries.iter().map(|e| e.0.clone()).collect();
    report.unlisted =
        granted.iter().filter(|(n, _)| !listed_names.contains(*n)).map(|(n, b)| (n.clone(), sha256_hex(b))).collect();

    let mut decoded_total = 0usize;
    let mut values: Vec<Option<Value>> = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();
    for (name, role, listed) in entries {
        let index = report.members.len();
        let bytes = granted.remove(&name);
        let mut m = MemberReport {
            name: name.clone(),
            role,
            listed: listed.clone(),
            present: bytes.is_some(),
            stored_sha256: bytes.as_deref().map(sha256_hex),
            decoded_sha256: None,
            checks: Vec::new(),
            recognition: if bytes.is_some() { Recognition::Unrecognised } else { Recognition::NotAvailable },
            interpretation: None,
            recheck: RECHECK_NOT_RUN,
        };
        let mut value = None;
        if let Some(stored) = &bytes {
            let against = format!("{} entry {name}", report.manifest_name);
            let stored_digest = m.stored_sha256.clone().unwrap_or_default();
            m.checks.push(compare("stored_sha256", &against, &listed.stored_sha256, &stored_digest));
            m.checks.push(compare(
                "stored_bytes",
                &against,
                &listed.stored_bytes.to_string(),
                &stored.len().to_string(),
            ));
            if role == Role::Record {
                // The declared form decides: a `.gz` member is a gzip stream and is checked as one, even when its
                // signature bytes are damaged.
                let decoded = if declared_gzip(&name) {
                    let (d, check) = decode(stored, limits, &mut decoded_total, &against)?;
                    m.checks.push(check);
                    d
                } else {
                    count_decoded(stored.len(), limits, &mut decoded_total)?;
                    Some(stored.clone())
                };
                if let Some(decoded) = decoded {
                    let digest = sha256_hex(&decoded);
                    m.decoded_sha256 = Some(digest.clone());
                    m.checks.push(compare("decoded_sha256", &against, opt(&listed.decoded_sha256), &digest));
                    m.checks.push(compare(
                        "decoded_bytes",
                        &against,
                        &listed.decoded_bytes.map(|n| n.to_string()).unwrap_or_default(),
                        &decoded.len().to_string(),
                    ));
                    match parse(&decoded) {
                        Ok(v) => {
                            let canonical = v.canonical();
                            m.checks.push(compare(
                                "canonical_sha256",
                                &against,
                                opt(&listed.canonical_sha256),
                                &sha256_hex(canonical.as_bytes()),
                            ));
                            value = Some(v);
                        }
                        Err(e) => m.recognition = Recognition::Unparsed(e.0.to_string()),
                    }
                }
            }
        }
        if let Some(v) = &value {
            m.recognition = match v.get("format") {
                Some(Value::String(f)) if !f.is_empty() => Recognition::Declared(f.clone()),
                Some(other) => Recognition::InvalidFormat(other.clone()),
                None => Recognition::Absent(table.discriminator.discriminate(
                    &manifest_sha256,
                    m.stored_sha256.as_deref().unwrap_or(""),
                    v,
                )),
            };
            let invalid = matches!(m.recognition, Recognition::InvalidFormat(_));
            pending.push(Pending { member: index, leaves: leaves(v), invalid_format: invalid });
        }
        report.members.push(m);
        values.push(value);
        report.stored.push(bytes);
    }

    interpret(&mut report, table, pending, limits)?;
    stage_manifests(&mut report, &values, limits, &mut decoded_total)?;
    run_records(&mut report, &values, limits, &mut decoded_total)?;
    report.case_only_names = case_groups(&report);
    Ok(report)
}

/// One parsed record's leaves, waiting for interpretation.
struct Pending {
    member: usize,
    leaves: Vec<crate::enumerate::Leaf>,
    /// The record's top-level `format` is present but not a non-empty string.
    invalid_format: bool,
}

fn interpret(
    report: &mut ImportReport,
    table: ImportTableRef<'_>,
    pending: Vec<Pending>,
    limits: &Limits,
) -> Result<(), ArchiveError> {
    // EVD.48's distinct-value limit per declared (format, field), counted over the whole import.
    let mut distinct: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for p in &pending {
        for l in p.leaves.iter().filter(|l| l.class == Class::S) {
            if let Some(f) = &l.format {
                distinct.entry((f.clone(), l.field.clone())).or_default().insert(l.value.canonical());
            }
        }
    }
    let over: BTreeSet<(String, String)> =
        distinct.into_iter().filter(|(_, v)| v.len() > table.table.distinct_value_limit).map(|(k, _)| k).collect();

    let mut total = 0usize;
    for Pending { member, leaves: ls, invalid_format } in pending {
        let mut counts = Interpretation::default();
        let mut groups: BTreeMap<(Option<String>, String, String), usize> = BTreeMap::new();
        for l in ls {
            match l.class {
                Class::T => {
                    counts.prose += 1;
                    *report.prose.entry((member, l.format.clone(), l.field.clone())).or_default() += 1;
                    continue;
                }
                Class::P => {
                    counts.payload_booleans += 1;
                    let b = matches!(l.value, Value::Bool(true));
                    *report.payload_booleans.entry((last_key(&l.field), b)).or_default() += 1;
                    continue;
                }
                Class::S | Class::B => {}
            }
            total += 1;
            if total > limits.max_occurrences {
                return Err(ArchiveError::limit("occurrences", limits.max_occurrences, total));
            }
            counts.occurrences += 1;
            let key = (l.format.clone(), l.field.clone(), l.value.canonical());
            // A leaf with no enclosing format counts as absent only in a record whose `format` key is missing.
            let absent = l.format.is_none() && !invalid_format;
            if let Some(&at) = groups.get(&key) {
                report.occurrences[at].pointers.push(l.pointer);
                bump(&mut counts, &report.occurrences[at].outcome, absent);
                continue;
            }
            let outcome = match &l.format {
                None if invalid_format => {
                    OccurrenceOutcome::Recorded { rule: None, reason: INVALID_FORMAT.to_string() }
                }
                None => OccurrenceOutcome::Recorded { rule: None, reason: ABSENT_FORMAT.to_string() },
                Some(f) if over.contains(&(f.clone(), l.field.clone())) => {
                    OccurrenceOutcome::Recorded { rule: None, reason: OVER_LIMIT.to_string() }
                }
                Some(f) => match table.table.lookup(f, &l.field, &l.value) {
                    None => OccurrenceOutcome::Unmapped { reason: NO_RULE },
                    Some((_, rule)) if rule.class != l.class => {
                        OccurrenceOutcome::Recorded { rule: None, reason: CLASS_DIFFERS.to_string() }
                    }
                    Some((i, rule)) if !rule.prerequisites.is_empty() => {
                        OccurrenceOutcome::Recorded { rule: Some(i), reason: PREREQUISITE_UNMET.to_string() }
                    }
                    Some((i, rule)) => match &rule.outcome {
                        Outcome::Facts(_) => OccurrenceOutcome::Facts { rule: i },
                        Outcome::Recorded => OccurrenceOutcome::Recorded { rule: Some(i), reason: rule.reason.clone() },
                    },
                },
            };
            bump(&mut counts, &outcome, absent);
            groups.insert(key, report.occurrences.len());
            report.occurrences.push(Occurrences {
                member,
                format: l.format,
                field: l.field,
                value: l.value,
                class: l.class,
                outcome,
                pointers: vec![l.pointer],
            });
        }
        report.members[member].interpretation = Some(counts);
    }
    Ok(())
}

fn bump(counts: &mut Interpretation, outcome: &OccurrenceOutcome, absent: bool) {
    match outcome {
        OccurrenceOutcome::Facts { .. } => counts.facts += 1,
        OccurrenceOutcome::Recorded { .. } => counts.recorded += 1,
        OccurrenceOutcome::Unmapped { .. } => counts.unmapped += 1,
    }
    if absent {
        counts.absent_format += 1;
    }
}

/// The corpus manifest's own digests: its record list's canonical digest and its full-corpus path/blob digest.
fn corpus_manifest_checks(
    manifest: &Value,
    records: &[Value],
    entries: &[(String, Role, Listed)],
) -> Vec<IntegrityCheck> {
    let mut checks = Vec::new();
    let records_canonical = Value::Array(records.to_vec()).canonical();
    checks.push(compare(
        "record_manifest_sha256",
        "corpus manifest record_manifest_sha256",
        manifest.get("record_manifest_sha256").and_then(Value::as_str).unwrap_or(""),
        &sha256_hex(records_canonical.as_bytes()),
    ));
    let mut full = String::new();
    for (path, _, l) in entries {
        full.push_str(path);
        full.push('\0');
        full.push_str(&l.git_blob);
        full.push('\n');
    }
    checks.push(compare(
        "full_corpus_manifest_sha256",
        "corpus manifest full_corpus_manifest_sha256",
        manifest.get("full_corpus_manifest_sha256").and_then(Value::as_str).unwrap_or(""),
        &sha256_hex(full.as_bytes()),
    ));
    checks.push(compare(
        "full_corpus_entries",
        "corpus manifest full_corpus_entries",
        &manifest.get("full_corpus_entries").and_then(Value::as_i64).map(|n| n.to_string()).unwrap_or_default(),
        &entries.len().to_string(),
    ));
    checks
}

/// PS3.14: each `sanitized-research-archive/1` manifest's rows against the members they name.
fn stage_manifests(
    report: &mut ImportReport,
    values: &[Option<Value>],
    limits: &Limits,
    decoded_total: &mut usize,
) -> Result<(), ArchiveError> {
    let by_name: BTreeMap<String, usize> =
        report.members.iter().enumerate().map(|(i, m)| (m.name.clone(), i)).collect();
    for (at, value) in values.iter().enumerate() {
        let Some(v) = value else { continue };
        if v.get("format").and_then(Value::as_str) != Some(STAGE_MANIFEST_FORMAT) {
            continue;
        }
        let base = match report.members[at].name.rsplit_once('/') {
            Some((dir, _)) => format!("{dir}/"),
            None => String::new(),
        };
        let fields: BTreeMap<String, Value> = v
            .as_object()
            .map(|o| o.iter().filter(|(k, _)| k.as_str() != "files").map(|(k, x)| (k.clone(), x.clone())).collect())
            .unwrap_or_default();
        let manifest_name = report.members[at].name.clone();
        let mut rows = Vec::new();
        let mut listed_here = BTreeSet::new();
        for (n, row) in v.get("files").and_then(Value::as_array).unwrap_or(&[]).iter().enumerate() {
            let path = row.get("path").and_then(Value::as_str).unwrap_or("").to_string();
            let name = format!("{base}{path}");
            listed_here.insert(name.clone());
            let text = |k: &str| row.get(k).and_then(Value::as_str).map(str::to_string);
            let mut r = ManifestRow {
                path: path.clone(),
                member: by_name.get(&name).copied(),
                original_name: text("original_name"),
                original_sha256: text("original_sha256"),
                transformation: text("transformation"),
                status: "checked",
            };
            let Some(i) = r.member else {
                r.status = "unlisted";
                rows.push(r);
                continue;
            };
            let Some(stored) = report.stored[i].clone() else {
                r.status = "missing";
                rows.push(r);
                continue;
            };
            let against = format!("{manifest_name} files[{n}]");
            let num = |k: &str| row.get(k).and_then(Value::as_i64).map(|x| x.to_string()).unwrap_or_default();
            let checks = &mut report.members[i].checks;
            checks.push(compare("stored_sha256", &against, &text("sha256").unwrap_or_default(), &sha256_hex(&stored)));
            checks.push(compare("stored_bytes", &against, &num("stored_bytes"), &stored.len().to_string()));
            let expected_decoded = text("uncompressed_sha256").unwrap_or_default();
            let decoded = if !declared_gzip(&name) {
                Some(stored)
            } else {
                let (d, check) = decode(&stored, limits, decoded_total, &against)?;
                report.members[i].checks.push(check);
                d
            };
            if let Some(decoded) = decoded {
                let checks = &mut report.members[i].checks;
                checks.push(compare("decoded_sha256", &against, &expected_decoded, &sha256_hex(&decoded)));
                checks.push(compare("decoded_bytes", &against, &num("uncompressed_bytes"), &decoded.len().to_string()));
            }
            rows.push(r);
        }
        let not_listed = report
            .members
            .iter()
            .filter(|m| m.name.starts_with(&base) && m.name != manifest_name && !listed_here.contains(&m.name))
            .map(|m| m.name.clone())
            .collect();
        report.stage_manifests.push(StageManifest { member: at, fields, rows, not_listed });
    }
    Ok(())
}

/// PS3.13's `ia16` members: a `runner-run/3` record's steps name each word member with its encoding, letter count
/// and the SHA-256 of its uncompressed bytes.
fn run_records(
    report: &mut ImportReport,
    values: &[Option<Value>],
    limits: &Limits,
    decoded_total: &mut usize,
) -> Result<(), ArchiveError> {
    let by_name: BTreeMap<String, usize> =
        report.members.iter().enumerate().map(|(i, m)| (m.name.clone(), i)).collect();
    for (at, value) in values.iter().enumerate() {
        let Some(v) = value else { continue };
        if v.get("format").and_then(Value::as_str) != Some(RUN_RECORD_FORMAT) {
            continue;
        }
        let run_name = report.members[at].name.clone();
        let base = run_name.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();
        for (n, step) in v.get("steps").and_then(Value::as_array).unwrap_or(&[]).iter().enumerate() {
            let (Some(word), Some(encoding)) =
                (step.get("word").and_then(Value::as_str), step.get("encoding").and_then(Value::as_str))
            else {
                continue;
            };
            let Some(&i) = by_name.get(&format!("{base}{word}")) else { continue };
            report.members[i].recognition = Recognition::Referenced {
                by: run_name.clone(),
                pointer: format!("/steps/{n}/word"),
                encoding: encoding.to_string(),
            };
            let Some(stored) = report.stored[i].clone() else { continue };
            if encoding != "ia16" {
                continue;
            }
            let against = format!("{run_name} steps[{n}]");
            let decoded = if declared_gzip(&report.members[i].name) {
                let (decoded, check) = decode(&stored, limits, decoded_total, &against)?;
                report.members[i].checks.push(check);
                decoded
            } else {
                count_decoded(stored.len(), limits, decoded_total)?;
                Some(stored)
            };
            if let Some(decoded) = decoded {
                let digest = sha256_hex(&decoded);
                report.members[i].decoded_sha256 = Some(digest.clone());
                let expected = step.get("word_sha256").and_then(Value::as_str).unwrap_or("");
                // Two bytes per ia16 letter; a count from the record whose double overflows refuses the import.
                let letters = match step.get("letters").and_then(Value::as_i64) {
                    Some(l) => l
                        .checked_mul(2)
                        .ok_or(ArchiveError::format("a run record's letter count overflows its byte length"))?
                        .to_string(),
                    None => String::new(),
                };
                let checks = &mut report.members[i].checks;
                checks.push(compare("decoded_sha256", &against, expected, &digest));
                checks.push(compare("decoded_bytes", &against, &letters, &decoded.len().to_string()));
            }
        }
    }
    Ok(())
}

/// Groups of listed or granted names that are equal under Unicode lower case and differ as text.
fn case_groups(report: &ImportReport) -> Vec<CaseGroup> {
    let mut by_lower: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for m in &report.members {
        let digest = m.stored_sha256.clone().unwrap_or_else(|| m.listed.stored_sha256.clone());
        by_lower.entry(m.name.to_lowercase()).or_default().push((m.name.clone(), digest));
    }
    for (name, digest) in &report.unlisted {
        by_lower.entry(name.to_lowercase()).or_default().push((name.clone(), digest.clone()));
    }
    by_lower.into_values().filter(|v| v.len() > 1).map(|names| CaseGroup { names }).collect()
}

/// Bounded gzip decoding of a captured member, with its `gzip_stream` check. A stream that would exceed the caller's
/// limit refuses the import; a corrupt stream is a failed check and yields no decoded bytes.
fn decode(
    stored: &[u8],
    limits: &Limits,
    total: &mut usize,
    against: &str,
) -> Result<(Option<Vec<u8>>, IntegrityCheck), ArchiveError> {
    let room = limits.max_member_decoded_bytes.min(limits.max_total_decoded_bytes.saturating_sub(*total));
    let check = |actual: &str, result| IntegrityCheck {
        check: "gzip_stream",
        against: against.to_string(),
        expected: "a valid gzip stream (CRC-32 and length checked)".to_string(),
        actual: actual.to_string(),
        result,
    };
    match gunzip(stored, room) {
        Ok(d) => {
            count_decoded(d.len(), limits, total)?;
            Ok((Some(d), check("valid", CheckResult::Pass)))
        }
        Err(e) if e.0 == "DEFLATE output exceeds its limit" => {
            Err(ArchiveError::limit("member_decoded", room, room + 1))
        }
        Err(e) => Ok((None, check(e.0, CheckResult::Fail))),
    }
}

/// The member's declared form: a name ending in `.gz` declares a gzip stream (RFC 1952). The stored bytes never decide
/// it, so a damaged signature fails the `gzip_stream` check instead of passing as plain bytes.
fn declared_gzip(name: &str) -> bool {
    name.ends_with(".gz")
}

fn count_decoded(n: usize, limits: &Limits, total: &mut usize) -> Result<(), ArchiveError> {
    if n > limits.max_member_decoded_bytes {
        return Err(ArchiveError::limit("member_decoded", limits.max_member_decoded_bytes, n));
    }
    *total += n;
    if *total > limits.max_total_decoded_bytes {
        return Err(ArchiveError::limit("total_decoded", limits.max_total_decoded_bytes, *total));
    }
    Ok(())
}

fn compare(check: &'static str, against: &str, expected: &str, actual: &str) -> IntegrityCheck {
    IntegrityCheck {
        check,
        against: against.to_string(),
        expected: expected.to_string(),
        actual: actual.to_string(),
        result: if expected == actual { CheckResult::Pass } else { CheckResult::Fail },
    }
}

fn opt(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("")
}

fn need_text(v: &Value, key: &str) -> Result<String, ArchiveError> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(ArchiveError::format("a manifest entry lacks a text field"))
}

fn listed(e: &Value, role: Role) -> Result<Listed, ArchiveError> {
    let num = |k: &str| e.get(k).and_then(Value::as_i64).and_then(|n| u64::try_from(n).ok());
    let digest = |k: &str| -> Result<Option<String>, ArchiveError> {
        match e.get(k) {
            None => Ok(None),
            Some(Value::String(s)) if crate::is_sha256(s) => Ok(Some(s.clone())),
            Some(_) => Err(ArchiveError::format("a manifest digest is not 64 lowercase hex digits")),
        }
    };
    let l = Listed {
        git_blob: need_text(e, "git_blob")?,
        stored_sha256: digest("stored_sha256")?.ok_or(ArchiveError::format("an entry lacks stored_sha256"))?,
        stored_bytes: num("stored_bytes").ok_or(ArchiveError::format("an entry lacks stored_bytes"))?,
        decoded_sha256: digest("decoded_sha256")?,
        decoded_bytes: num("decoded_bytes"),
        canonical_sha256: digest("canonical_sha256")?,
        canonical_bytes: num("canonical_bytes"),
    };
    if role == Role::Record && (l.decoded_sha256.is_none() || l.canonical_sha256.is_none()) {
        return Err(ArchiveError::format("a record entry lacks its decoded or canonical digest"));
    }
    Ok(l)
}

impl ImportReport {
    /// The code an import command would refuse publication with: a digest mismatch when any integrity check fails.
    /// Recognition and interpretation outcomes never refuse; they stay recorded.
    pub fn refusal(&self) -> Option<&'static str> {
        self.members.iter().any(|m| m.integrity() == "fail").then_some(crate::DIGEST_MISMATCH).or_else(|| {
            self.manifest_checks.iter().any(|c| c.result == CheckResult::Fail).then_some(crate::DIGEST_MISMATCH)
        })
    }

    pub fn member(&self, name: &str) -> Option<usize> {
        self.members.iter().position(|m| m.name == name)
    }

    /// The stored bytes of a granted member.
    pub fn stored_bytes(&self, member: usize) -> Option<&[u8]> {
        self.stored.get(member).and_then(|b| b.as_deref())
    }

    /// Every failed integrity check behind a member's bytes, as `check against` text: the member's own checks and the
    /// corpus manifest's self-checks, since the manifest's recorded digests are what the member was checked against.
    pub fn failed_checks(&self, member: usize) -> Vec<String> {
        let own = self.members.get(member).map(|m| m.checks.as_slice()).unwrap_or(&[]);
        own.iter()
            .chain(&self.manifest_checks)
            .filter(|c| c.result == CheckResult::Fail)
            .map(|c| format!("{} against {}", c.check, c.against))
            .collect()
    }

    /// The report's counts. `producers_run` is always zero: nothing here can run one.
    pub fn totals(&self) -> BTreeMap<&'static str, usize> {
        let mut t = BTreeMap::new();
        let count = |f: &dyn Fn(&MemberReport) -> bool| self.members.iter().filter(|m| f(m)).count();
        t.insert("records", count(&|m| m.role == Role::Record));
        t.insert("auxiliary", count(&|m| m.role == Role::Auxiliary));
        t.insert("present", count(&|m| m.present));
        t.insert("missing", count(&|m| !m.present));
        t.insert("integrity_pass", count(&|m| m.integrity() == "pass"));
        t.insert("integrity_fail", count(&|m| m.integrity() == "fail"));
        t.insert("declared", count(&|m| matches!(m.recognition, Recognition::Declared(_))));
        t.insert("absent_format", count(&|m| matches!(m.recognition, Recognition::Absent(_))));
        t.insert("invalid_format", count(&|m| matches!(m.recognition, Recognition::InvalidFormat(_))));
        t.insert(
            "absent_discriminated",
            count(&|m| matches!(m.recognition, Recognition::Absent(Discrimination::Discriminated { .. }))),
        );
        t.insert("unparsed", count(&|m| matches!(m.recognition, Recognition::Unparsed(_))));
        t.insert("referenced", count(&|m| matches!(m.recognition, Recognition::Referenced { .. })));
        let sum = |f: &dyn Fn(&Interpretation) -> usize| {
            self.members.iter().filter_map(|m| m.interpretation.as_ref()).map(f).sum::<usize>()
        };
        t.insert("occurrences", sum(&|i| i.occurrences));
        t.insert("occurrences_facts", sum(&|i| i.facts));
        t.insert("occurrences_recorded", sum(&|i| i.recorded));
        t.insert("occurrences_unmapped", sum(&|i| i.unmapped));
        t.insert("occurrences_absent_format", sum(&|i| i.absent_format));
        t.insert("prose", sum(&|i| i.prose));
        t.insert("payload_booleans", sum(&|i| i.payload_booleans));
        t.insert("unlisted", self.unlisted.len());
        t.insert("producers_run", 0);
        t
    }

    /// The report as JSON, for the caller's granted output.
    pub fn to_json(&self) -> Value {
        let s = |x: &str| Value::String(x.to_string());
        let o = |pairs: Vec<(&str, Value)>| Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect());
        let n = |x: usize| Value::Int(x as i64);
        let ns = |x: &Option<String>| x.as_ref().map_or(Value::Null, |v| s(v));
        let check = |c: &IntegrityCheck| {
            o(vec![
                ("check", s(c.check)),
                ("against", s(&c.against)),
                ("expected", s(&c.expected)),
                ("actual", s(&c.actual)),
                ("result", s(if c.result == CheckResult::Pass { "pass" } else { "fail" })),
            ])
        };
        let recognition = |r: &Recognition| match r {
            Recognition::NotAvailable => o(vec![("kind", s("not_available"))]),
            Recognition::Declared(f) => o(vec![("kind", s("declared")), ("format", s(f))]),
            Recognition::Absent(d) => {
                let d = match d {
                    Discrimination::Undiscriminated => o(vec![("kind", s("recorded")), ("reason", s(ABSENT_FORMAT))]),
                    Discrimination::Discriminated { rule, pointer, inner_format } => o(vec![
                        ("kind", s("discriminated")),
                        ("rule", n(*rule)),
                        ("pointer", s(pointer)),
                        ("inner_format", s(inner_format)),
                        ("review", s(&self.discriminator_review)),
                    ]),
                    Discrimination::PrerequisiteUnmet { rule, reason } => {
                        o(vec![("kind", s("prerequisite_unmet")), ("rule", n(*rule)), ("reason", s(reason))])
                    }
                    Discrimination::Ambiguous { rules } => o(vec![
                        ("kind", s("ambiguous")),
                        ("rules", Value::Array(rules.iter().map(|r| n(*r)).collect())),
                    ]),
                };
                o(vec![("kind", s("absent")), ("format", Value::Null), ("discrimination", d)])
            }
            Recognition::InvalidFormat(v) => {
                o(vec![("kind", s("invalid_format")), ("format", v.clone()), ("reason", s(INVALID_FORMAT))])
            }
            Recognition::Unparsed(why) => o(vec![("kind", s("unparsed")), ("reason", s(why))]),
            Recognition::Referenced { by, pointer, encoding } => {
                o(vec![("kind", s("referenced")), ("by", s(by)), ("pointer", s(pointer)), ("encoding", s(encoding))])
            }
            Recognition::Unrecognised => o(vec![("kind", s("unrecognised"))]),
        };
        let members = self
            .members
            .iter()
            .map(|m| {
                let i = m.interpretation;
                o(vec![
                    ("name", s(&m.name)),
                    ("role", s(if m.role == Role::Record { "record" } else { "auxiliary" })),
                    ("present", Value::Bool(m.present)),
                    (
                        "listed",
                        o(vec![
                            ("git_blob", s(&m.listed.git_blob)),
                            ("stored_sha256", s(&m.listed.stored_sha256)),
                            ("stored_bytes", n(m.listed.stored_bytes as usize)),
                            ("decoded_sha256", ns(&m.listed.decoded_sha256)),
                        ]),
                    ),
                    ("stored_sha256", ns(&m.stored_sha256)),
                    ("decoded_sha256", ns(&m.decoded_sha256)),
                    ("recognition", recognition(&m.recognition)),
                    (
                        "integrity",
                        o(vec![
                            ("result", s(m.integrity())),
                            ("checks", Value::Array(m.checks.iter().map(check).collect())),
                        ]),
                    ),
                    (
                        "interpretation",
                        i.map_or(Value::Null, |i| {
                            o(vec![
                                ("occurrences", n(i.occurrences)),
                                ("facts", n(i.facts)),
                                ("recorded", n(i.recorded)),
                                ("unmapped", n(i.unmapped)),
                                ("absent_format", n(i.absent_format)),
                                ("prose", n(i.prose)),
                                ("payload_booleans", n(i.payload_booleans)),
                            ])
                        }),
                    ),
                    ("recheck", o(vec![("result", s("not_run")), ("reason", s(m.recheck))])),
                ])
            })
            .collect();
        let occurrences = self
            .occurrences
            .iter()
            .map(|g| {
                let (outcome, rule, reason) = match &g.outcome {
                    OccurrenceOutcome::Facts { rule } => ("facts", n(*rule), Value::Null),
                    OccurrenceOutcome::Recorded { rule, reason } => {
                        ("recorded", rule.map_or(Value::Null, n), s(reason))
                    }
                    OccurrenceOutcome::Unmapped { reason } => ("unmapped", Value::Null, s(reason)),
                };
                o(vec![
                    ("member", n(g.member)),
                    ("format", ns(&g.format)),
                    ("field", s(&g.field)),
                    ("value", g.value.clone()),
                    ("class", s(g.class.text())),
                    ("outcome", s(outcome)),
                    ("rule", rule),
                    ("reason", reason),
                    ("pointers", Value::Array(g.pointers.iter().map(|p| s(p)).collect())),
                ])
            })
            .collect();
        let prose = self
            .prose
            .iter()
            .map(|((m, f, field), c)| {
                o(vec![("member", n(*m)), ("format", ns(f)), ("field", s(field)), ("count", n(*c))])
            })
            .collect();
        let payload = self
            .payload_booleans
            .iter()
            .map(|((k, b), c)| o(vec![("key", s(k)), ("value", Value::Bool(*b)), ("count", n(*c))]))
            .collect();
        let stage = self
            .stage_manifests
            .iter()
            .map(|sm| {
                o(vec![
                    ("member", n(sm.member)),
                    ("fields", Value::Object(sm.fields.clone())),
                    (
                        "rows",
                        Value::Array(
                            sm.rows
                                .iter()
                                .map(|r| {
                                    o(vec![
                                        ("path", s(&r.path)),
                                        ("member", r.member.map_or(Value::Null, n)),
                                        ("status", s(r.status)),
                                        ("original_name", ns(&r.original_name)),
                                        ("original_sha256", ns(&r.original_sha256)),
                                        (
                                            "original_bytes",
                                            s("absent: the captured source edition is not part of the archive"),
                                        ),
                                        ("transformation", ns(&r.transformation)),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                    ("not_listed", Value::Array(sm.not_listed.iter().map(|x| s(x)).collect())),
                ])
            })
            .collect();
        let cases = self
            .case_only_names
            .iter()
            .map(|g| {
                o(vec![
                    (
                        "names",
                        Value::Array(
                            g.names
                                .iter()
                                .map(|(name, d)| o(vec![("name", s(name)), ("stored_sha256", s(d))]))
                                .collect(),
                        ),
                    ),
                    ("distinct_digests", Value::Bool(g.distinct_digests())),
                ])
            })
            .collect();
        let totals = Value::Object(self.totals().into_iter().map(|(k, v)| (k.to_string(), n(v))).collect());
        o(vec![
            ("format", s("c600-archive-import-report")),
            ("version", n(1)),
            ("table", o(vec![("edition", s(&self.table_edition)), ("sha256", s(&self.table_sha256))])),
            (
                "discriminator",
                o(vec![
                    ("edition", s(&self.discriminator_edition)),
                    ("sha256", s(&self.discriminator_sha256)),
                    ("review", s(&self.discriminator_review)),
                ]),
            ),
            (
                "manifest",
                o(vec![
                    ("name", s(&self.manifest_name)),
                    ("sha256", s(&self.manifest_sha256)),
                    ("checks", Value::Array(self.manifest_checks.iter().map(check).collect())),
                ]),
            ),
            (
                "limits",
                o(vec![
                    ("max_members", n(self.limits.max_members)),
                    ("max_manifest_bytes", n(self.limits.max_manifest_bytes)),
                    ("max_member_stored_bytes", n(self.limits.max_member_stored_bytes)),
                    ("max_total_stored_bytes", n(self.limits.max_total_stored_bytes)),
                    ("max_member_decoded_bytes", n(self.limits.max_member_decoded_bytes)),
                    ("max_total_decoded_bytes", n(self.limits.max_total_decoded_bytes)),
                    ("max_occurrences", n(self.limits.max_occurrences)),
                ]),
            ),
            ("members", Value::Array(members)),
            ("occurrences", Value::Array(occurrences)),
            ("prose", Value::Array(prose)),
            ("payload_booleans", Value::Array(payload)),
            ("stage_manifests", Value::Array(stage)),
            ("case_only_names", Value::Array(cases)),
            (
                "unlisted",
                Value::Array(
                    self.unlisted.iter().map(|(name, d)| o(vec![("name", s(name)), ("stored_sha256", s(d))])).collect(),
                ),
            ),
            ("refusal", self.refusal().map_or(Value::Null, s)),
            ("totals", totals),
        ])
    }
}
