//! FP.29's migration record and FP.28's publication (PS2.1 to PS2.9).
//!
//! A conversion is checked whole before anything durable is written: the converter's declared inputs are read again
//! after it ran (PS2.7, R-08), every output record passes `write_record`, a word that claims no semantic change keeps
//! its literal meaning (PS2.5), and the migration record binds source, target, converter and outcomes (PS2.3). The
//! [`Publisher`] then writes a new generation and its publication metadata, makes them durable, and only then appends
//! the active pointer (PS2.4, FP.28). The previous generation and every original stay stored.
//!
//! # Store layout (draft)
//!
//! Every object is stored once under `objects/<sha256 of its bytes>`; records are FP.5 bytes, so the name is also
//! their content digest. A generation is an archive object `{"objects":[DigestRef(bytes)]}` listing its records and
//! retained originals, and FP.28's metadata names it. The pointer `active` is an append-only log of lines, each the
//! 64 hexadecimal digits of one metadata object and a newline. Only a complete line that names a durable, valid
//! generation is a switch. A torn tail is not, and the next publication cuts it durably before it appends, so a later
//! append never completes it. A last complete line that is not a digest is damage: the reader refuses it rather than
//! fall back to an older line.

use std::collections::BTreeMap;
use std::io;

use c600_digest::sha256_hex;

use super::codec::{canonical, charge, content_ref, digest_ref, read_canonical, sha_of, Json};
use super::codes::{DIGEST, FORMAT, IDENTITY, PROVENANCE};
use super::facts::{self, Derived, Functions};
use super::record::{has_exactly, read_record, record_header, reference, supported, write_record, Read};
use super::{require, Refusal};
use crate::disk::Disk;

/// FP.28's code for a switch that may or may not have happened and could not be read back.
pub const UNKNOWN_OUTCOME: &str = "PS3:unknown_outcome.import_publication";
/// FP.29's payload fields.
pub const PAYLOAD: [&str; 7] = ["source", "target", "converter", "inputs", "outcomes", "checks", "semantic_changes"];
const PUBLICATION: [&str; 7] = ["format", "version", "id", "previous", "next", "archive", "validation"];
const POINTER: &str = "active";

/// FP.29's per-item status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Converted,
    Recorded,
    Refused,
}

impl Status {
    pub fn wire(self) -> &'static str {
        match self {
            Status::Converted => "converted",
            Status::Recorded => "recorded",
            Status::Refused => "refused",
        }
    }
}

/// One affected item with its typed reason (PS2.3's Outcome).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub item: String,
    pub status: Status,
    pub reason: String,
}

impl Outcome {
    pub fn new(item: &str, status: Status, reason: &str) -> Outcome {
        Outcome { item: item.to_string(), status, reason: reason.to_string() }
    }

    fn wire(&self) -> Json {
        Json::object([
            ("item", Json::text(&self.item)),
            ("status", Json::text(self.status.wire())),
            ("reason", Json::text(&self.reason)),
        ])
    }
}

/// The recorded input of `migrate_record`: the old format, version and edition, the original bytes, and the identity
/// the source records (KN0.1 wire form), which no conversion may change (PS2.4).
pub struct RecordedInput<'a> {
    pub format: &'a str,
    pub version: Json,
    pub edition: Option<&'a str>,
    pub bytes: &'a [u8],
    pub identity: Json,
}

/// What a converter produced: the target record if any item converted, each item's outcome, the records holding the
/// preservation checks, and the semantic changes it declares.
pub struct Conversion {
    pub target: Option<Json>,
    pub outcomes: Vec<Outcome>,
    pub checks: Vec<Json>,
    pub semantic_changes: Vec<String>,
}

/// A converter: its FP.11 pin, a reader of its declared inputs as digest references (mapping tables, function
/// editions), and the conversion itself.
pub struct Converter<'a> {
    pub pin: Json,
    pub inputs: &'a mut dyn FnMut() -> Vec<Json>,
    pub convert: &'a dyn Fn(&[u8]) -> Result<Conversion, Refusal>,
}

/// A checked migration: the `c600-migration` record, the target and the check records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Migration {
    pub record: Json,
    pub target: Option<Json>,
    pub checks: Vec<Json>,
}

impl Migration {
    /// The package that publishes this migration's target with the original bytes retained, or `None` when no item
    /// converted: then nothing activates and the original stays as it was (FP.29).
    pub fn package(&self, original: &[u8]) -> Option<Package> {
        let head = self.target.clone()?;
        let mut records = vec![head.clone(), self.record.clone()];
        records.extend(self.checks.iter().cloned());
        Some(Package { records, originals: vec![original.to_vec()], head, validation: self.record.clone() })
    }
}

fn bad() -> Refusal {
    Refusal::new(FORMAT)
}

fn is_ref(value: &Json) -> bool {
    has_exactly(value, &["format", "version", "content"])
        && value.get("format").and_then(Json::as_text).is_some()
        && matches!(value.get("version"), Some(Json::Int(_)))
        && is_digest(value.get("content"), Some("content"))
}

fn is_digest(value: Option<&Json>, role: Option<&str>) -> bool {
    let Some(value) = value else { return false };
    let sha = value.get("sha256").and_then(Json::as_text);
    has_exactly(value, &["role", "sha256"])
        && sha.is_some_and(|s| s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        && value.get("role").and_then(Json::as_text).is_some_and(|r| role.is_none_or(|want| r == want))
}

fn is_pin(value: &Json) -> bool {
    has_exactly(value, &["version", "source"])
        && value.get("version").and_then(Json::as_text).is_some()
        && is_digest(value.get("source"), Some("source"))
}

fn list<'a>(value: &'a Json, name: &str) -> Result<&'a [Json], Refusal> {
    match value.get(name) {
        Some(Json::List(items)) => Ok(items),
        _ => Err(bad()),
    }
}

/// FP.29's payload: the exact shapes, and a target exactly when some item converted, so a refused or recorded
/// conversion has no fabricated target.
pub fn check_payload(payload: &Json) -> Result<(), Refusal> {
    require(has_exactly(payload, &PAYLOAD), FORMAT)?;
    let source = payload.get("source").ok_or_else(bad)?;
    require(has_exactly(source, &["format", "version", "edition", "bytes"]), FORMAT)?;
    require(source.get("format").and_then(Json::as_text).is_some(), FORMAT)?;
    require(matches!(source.get("edition"), Some(Json::Null | Json::Text(_))), FORMAT)?;
    require(is_digest(source.get("bytes"), Some("bytes")), FORMAT)?;
    let target = payload.get("target").ok_or_else(bad)?;
    require(*target == Json::Null || is_ref(target), FORMAT)?;
    require(is_pin(payload.get("converter").ok_or_else(bad)?), FORMAT)?;
    require(list(payload, "inputs")?.iter().all(|d| is_digest(Some(d), None)), FORMAT)?;
    let outcomes = list(payload, "outcomes")?;
    require(!outcomes.is_empty(), FORMAT)?;
    let mut converted = false;
    for outcome in outcomes {
        require(has_exactly(outcome, &["item", "status", "reason"]), FORMAT)?;
        require(outcome.get("item").and_then(Json::as_text).is_some(), FORMAT)?;
        require(outcome.get("reason").and_then(Json::as_text).is_some(), FORMAT)?;
        let status = outcome.get("status").and_then(Json::as_text);
        require(matches!(status, Some("converted" | "recorded" | "refused")), FORMAT)?;
        converted |= status == Some("converted");
    }
    require(converted == (*target != Json::Null), FORMAT)?;
    require(list(payload, "checks")?.iter().all(is_ref), FORMAT)?;
    require(list(payload, "semantic_changes")?.iter().all(|t| t.as_text().is_some()), FORMAT)
}

/// PS2.5's preserved fields of a word, in the order a change is reported.
pub const WORD_FIELDS: [&str; 7] = ["identity", "provenance", "alphabet", "start", "length", "letters", "parts"];

/// The PS2.5 fields in which `new` differs from `old`: identity, provenance, alphabet, start digest, expanded length,
/// literal letters and part pins. Chunk encoding and annotations generate no letters and may change.
pub fn word_changes(old: &Json, new: &Json) -> Vec<&'static str> {
    let at = |record: &Json, field: &str| match field {
        "identity" | "provenance" => record.get(field).cloned(),
        _ => record.get("payload").and_then(|p| p.get(field)).cloned(),
    };
    WORD_FIELDS.iter().copied().filter(|f| at(old, f) != at(new, f)).collect()
}

/// The refusal for a conversion that changed a preserved word field while declaring no semantic change.
fn preservation_code(field: &str) -> &'static str {
    match field {
        "identity" => IDENTITY,
        "provenance" => PROVENANCE,
        _ => DIGEST,
    }
}

/// PS2.3: the source fields describe the bytes. Bytes given as a `c600-` record must be one, with that format, version
/// and identity in their header, whether or not this reader can activate that version, and a `c600-` record is never
/// given as another format; other bytes (0.4's formats) are bound by their digest and read by their converter.
fn check_source(input: &RecordedInput) -> Result<(), Refusal> {
    let is_record = |format: Option<&str>| format.is_some_and(|f| f.starts_with("c600-"));
    if !is_record(Some(input.format)) {
        let stored = read_canonical(input.bytes).ok();
        return require(!is_record(stored.as_ref().and_then(|old| old.get("format")).and_then(Json::as_text)), FORMAT);
    }
    let old = read_canonical(input.bytes)?;
    require(old.get("format").and_then(Json::as_text) == Some(input.format), FORMAT)?;
    require(old.get("version") == Some(&input.version), FORMAT)?;
    require(old.get("identity") == Some(&input.identity), IDENTITY)
}

/// Chapter 05's `migrate_record`: runs the converter between two reads of its declared inputs, checks every output,
/// and returns the migration record that binds source, target and converter. Nothing is written here; publish the
/// result with a [`Publisher`].
pub fn migrate_record(input: &RecordedInput, converter: &mut Converter) -> Result<Migration, Refusal> {
    charge("record_decoded", input.bytes.len() as u128)?;
    check_source(input)?;
    require(is_pin(&converter.pin), FORMAT)?;
    let before = (converter.inputs)();
    let conversion = (converter.convert)(input.bytes)?;
    // R-08: the inputs are compared after the build; equal output hashes never excuse a changed input.
    require((converter.inputs)() == before, DIGEST)?;
    if let Some(target) = &conversion.target {
        write_record(target)?;
        require(target.get("identity") == Some(&input.identity), IDENTITY)?;
        let is_word = |format: &str| format == "c600-word";
        let target_format = target.get("format").and_then(Json::as_text).unwrap_or("");
        if is_word(input.format) && is_word(target_format) && conversion.semantic_changes.is_empty() {
            let old = read_canonical(input.bytes)?;
            if let Some(field) = word_changes(&old, target).first() {
                return Err(Refusal::new(preservation_code(field)));
            }
        }
    }
    for check in &conversion.checks {
        write_record(check)?;
    }
    let payload = Json::object([
        (
            "source",
            Json::object([
                ("format", Json::text(input.format)),
                ("version", input.version.clone()),
                ("edition", input.edition.map_or(Json::Null, Json::text)),
                ("bytes", digest_ref("bytes", input.bytes)),
            ]),
        ),
        ("target", conversion.target.as_ref().map_or(Json::Null, reference)),
        ("converter", converter.pin.clone()),
        ("inputs", Json::List(before)),
        ("outcomes", Json::List(conversion.outcomes.iter().map(Outcome::wire).collect())),
        ("checks", Json::List(conversion.checks.iter().map(reference).collect())),
        ("semantic_changes", Json::List(conversion.semantic_changes.iter().map(|t| Json::text(t)).collect())),
    ]);
    check_payload(&payload)?;
    let record = Json::object([
        ("format", Json::text("c600-migration")),
        ("version", Json::Int(1)),
        ("identity", input.identity.clone()),
        ("payload", payload),
        ("provenance", Json::Null),
    ]);
    write_record(&record)?;
    Ok(Migration { record, target: conversion.target, checks: conversion.checks })
}

/// PS2.6 and FP.29's edition change of a facts envelope: a new derived record under the new function editions, the
/// old record left as it was, both interpretations, the caches the change invalidates and the migration record.
#[derive(Clone, Debug)]
pub struct EditionChange {
    pub record: Json,
    pub note: Json,
    pub old: Vec<(String, Derived)>,
    pub new: Vec<(String, Derived)>,
    pub stale: Vec<Json>,
    pub migration: Migration,
}

fn derived_reason(derived: &Derived) -> (Status, String) {
    match derived {
        Derived::Word(w) => (Status::Converted, format!("row {}: {}", w.row, w.display())),
        Derived::Inconsistent(e) => (Status::Converted, format!("row 0: {}", e.code)),
        Derived::NoVerdict => (Status::Recorded, "no verdict: function edition unavailable".to_string()),
    }
}

/// Changes the function editions of a stored evidence record. The facts and the old record stay unchanged, so the
/// old interpretation is reproducible; the new record differs only in `editions` and its surviving caches.
pub fn change_editions(
    old_bytes: &[u8],
    editions: &Json,
    functions: &Functions,
    converter: &Json,
) -> Result<EditionChange, Refusal> {
    let old = read_canonical(old_bytes)?;
    record_header(&old)?;
    require(old.get("format").and_then(Json::as_text) == Some("c600-evidence"), FORMAT)?;
    let payload = old.get("payload").ok_or_else(bad)?;
    let stored = facts::admit(payload)?;
    facts::check_editions(editions)?;
    let input = facts::fact_input(payload);
    let note = facts::edition_note(&stored.editions, editions, &input)?;
    let before = facts::derive(&stored, &stored.editions, functions);
    let after = facts::derive(&stored, editions, functions);
    let caches = list(payload, "caches")?;
    let stale = facts::stale_caches(caches, editions);
    let Json::Object(mut fields) = payload.clone() else { return Err(bad()) };
    fields.insert("editions".to_string(), editions.clone());
    fields.insert("caches".to_string(), Json::List(caches.iter().filter(|c| !stale.contains(c)).cloned().collect()));
    let Json::Object(mut header) = old.clone() else { return Err(bad()) };
    header.insert("payload".to_string(), Json::Object(fields));
    let record = Json::Object(header);
    let changed: Vec<String> = list(&note, "changed")?
        .iter()
        .filter_map(Json::as_text)
        .map(|f| format!("{f} function edition changed"))
        .collect();
    // The record converts; each root records its new derived output or inconsistency, or no verdict.
    let mut outcomes = vec![Outcome::new("record", Status::Converted, "new function editions; facts unchanged")];
    for (root, derived) in &after {
        let (status, reason) = derived_reason(derived);
        outcomes.push(Outcome::new(&format!("root {root}"), status, &reason));
    }
    let inputs = vec![
        digest_ref("content", &canonical(&input)),
        digest_ref("content", &canonical(&stored.editions)),
        digest_ref("content", &canonical(editions)),
    ];
    let mut read_inputs = || inputs.clone();
    let convert = |_: &[u8]| {
        Ok(Conversion {
            target: Some(record.clone()),
            outcomes: outcomes.clone(),
            checks: Vec::new(),
            semantic_changes: changed.clone(),
        })
    };
    let mut converter = Converter { pin: converter.clone(), inputs: &mut read_inputs, convert: &convert };
    let source = RecordedInput {
        format: "c600-evidence",
        version: Json::Int(1),
        edition: None,
        bytes: old_bytes,
        identity: old.get("identity").cloned().unwrap_or(Json::Null),
    };
    let migration = migrate_record(&source, &mut converter)?;
    Ok(EditionChange { record, note, old: before, new: after, stale, migration })
}

/// PS2.7 and FP.19: a cache is reused only when every recorded input and every function edition equals the current
/// ones exactly. Caches fail closed (PS2.8).
pub fn cache_reusable(cache: &Json, inputs: &[Json], functions: &[Json]) -> bool {
    cache.get("inputs") == Some(&Json::List(inputs.to_vec()))
        && cache.get("functions") == Some(&Json::List(functions.to_vec()))
}

/// What one publication writes: its records, the original bytes it retains, the record it activates and the record
/// that validates it (FP.28's `next` and `validation`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub records: Vec<Json>,
    pub originals: Vec<Vec<u8>>,
    pub head: Json,
    pub validation: Json,
}

/// Why a publication did not switch, or may have.
#[derive(Debug)]
pub enum PublishError {
    /// Validation or replay refused the package; nothing was written.
    Refused(Refusal),
    /// A write or a durability barrier before the switch failed, or the switch was read back as not done: the
    /// previous generation stays active and a retry is safe.
    NotPublished(io::Error),
    /// `PS3:unknown_outcome.import_publication`: the switch may have happened and the pointer could not be read
    /// back and validated (FP.28).
    UnknownOutcome,
}

impl From<Refusal> for PublishError {
    fn from(refusal: Refusal) -> PublishError {
        PublishError::Refused(refusal)
    }
}

/// FP.28's local publication store over a [`Disk`].
pub struct Publisher<D: Disk> {
    disk: D,
}

fn object_name(sha: &str) -> String {
    format!("objects/{sha}")
}

fn is_hex64(text: &[u8]) -> bool {
    text.len() == 64 && text.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl<D: Disk> Publisher<D> {
    pub fn new(disk: D) -> Publisher<D> {
        Publisher { disk }
    }

    /// One stored object, checked against its name.
    pub fn object(&mut self, sha: &str) -> Result<Vec<u8>, PublishError> {
        let bytes = self.disk.read(&object_name(sha)).map_err(PublishError::NotPublished)?.ok_or_else(bad)?;
        require(sha256_hex(&bytes) == sha, DIGEST)?;
        Ok(bytes)
    }

    /// The metadata digest the pointer log names last, if any: its last complete line, which must be 64 hexadecimal
    /// digits. A torn tail is no line; a damaged last line is refused, never skipped for an older one.
    fn pointer(&mut self) -> Result<Option<String>, PublishError> {
        let Some(log) = self.disk.read(POINTER).map_err(PublishError::NotPublished)? else { return Ok(None) };
        let Some(end) = log.iter().rposition(|b| *b == b'\n') else { return Ok(None) };
        let start = log[..end].iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        let line = &log[start..end];
        require(is_hex64(line), FORMAT)?;
        Ok(Some(String::from_utf8_lossy(line).into_owned()))
    }

    /// A stored record this reader activates, or the generation is refused with the reader's code (PS2.2): a head or
    /// validation record of a newer version or with an unknown required extension never activates.
    fn active_record(&mut self, sha: &str) -> Result<Json, PublishError> {
        let bytes = self.object(sha)?;
        let format = read_canonical(&bytes)?.get("format").and_then(Json::as_text).unwrap_or("").to_string();
        match read_record(&bytes, &format)? {
            Read::Active(record) => Ok(record),
            Read::Recorded { code, .. } => Err(PublishError::Refused(Refusal::new(code))),
        }
    }

    /// Validates the generation that metadata `sha` names: the metadata's FP.28 shape, the archive and every object
    /// it lists, the head and the validation record among them.
    fn generation(&mut self, sha: &str) -> Result<Json, PublishError> {
        let meta = read_canonical(&self.object(sha)?)?;
        require(has_exactly(&meta, &PUBLICATION), FORMAT)?;
        supported(&meta)?;
        require(meta.get("format").and_then(Json::as_text) == Some("c600-publication"), FORMAT)?;
        require(meta.get("id").and_then(Json::as_text).is_some(), FORMAT)?;
        let previous = meta.get("previous").ok_or_else(bad)?;
        require(*previous == Json::Null || is_digest(Some(previous), Some("content")), FORMAT)?;
        require(is_digest(meta.get("next"), Some("content")), FORMAT)?;
        require(is_digest(meta.get("archive"), Some("bytes")), FORMAT)?;
        let validation = meta.get("validation").ok_or_else(bad)?;
        require(is_ref(validation), FORMAT)?;
        let archive_sha = meta.get("archive").and_then(sha_of).ok_or_else(bad)?.to_string();
        let archive = read_canonical(&self.object(&archive_sha)?)?;
        require(has_exactly(&archive, &["objects"]), FORMAT)?;
        let mut listed = Vec::new();
        for entry in list(&archive, "objects")? {
            require(is_digest(Some(entry), Some("bytes")), FORMAT)?;
            let sha = sha_of(entry).ok_or_else(bad)?.to_string();
            self.object(&sha)?;
            listed.push(sha);
        }
        let next = meta.get("next").and_then(sha_of).ok_or_else(bad)?.to_string();
        require(listed.contains(&next), FORMAT)?;
        self.active_record(&next)?;
        let checked = validation.get("content").and_then(sha_of).ok_or_else(bad)?.to_string();
        require(listed.contains(&checked), FORMAT)?;
        let record = self.active_record(&checked)?;
        require(record.get("format") == validation.get("format"), FORMAT)?;
        require(record.get("version") == validation.get("version"), FORMAT)?;
        Ok(meta)
    }

    /// The active publication's metadata, validated with its generation; `None` before the first publication. A
    /// damaged generation is refused, never replaced by an older one.
    pub fn active(&mut self) -> Result<Option<Json>, PublishError> {
        match self.pointer()? {
            None => Ok(None),
            Some(sha) => self.generation(&sha).map(Some),
        }
    }

    /// Writes one object durably unless the same bytes are already stored.
    fn put(&mut self, bytes: &[u8]) -> io::Result<String> {
        let sha = sha256_hex(bytes);
        let name = object_name(&sha);
        if self.disk.read(&name)?.is_some_and(|stored| stored == bytes) {
            self.disk.sync(&name)?;
            return Ok(sha);
        }
        self.disk.create(&name, bytes)?;
        self.disk.sync(&name)?;
        Ok(sha)
    }

    /// The complete validation before any write: each record as `write_record` writes it, the head and the
    /// validation record among them, and a migration record's source among the originals and its target the head.
    fn validate(package: &Package) -> Result<Vec<Vec<u8>>, Refusal> {
        let mut objects = Vec::new();
        let mut names = Vec::new();
        for record in &package.records {
            let bytes = write_record(record)?;
            names.push(sha256_hex(&bytes));
            objects.push(bytes);
        }
        let head = content_ref(&package.head);
        require(names.iter().any(|n| Some(n.as_str()) == sha_of(&head)), FORMAT)?;
        let check = content_ref(&package.validation);
        require(names.iter().any(|n| Some(n.as_str()) == sha_of(&check)), FORMAT)?;
        if package.validation.get("format").and_then(Json::as_text) == Some("c600-migration") {
            let payload = package.validation.get("payload").ok_or_else(bad)?;
            check_payload(payload)?;
            let source = payload.get("source").and_then(|s| s.get("bytes")).and_then(sha_of).ok_or_else(bad)?;
            require(package.originals.iter().any(|o| sha256_hex(o) == source), FORMAT)?;
            require(payload.get("target") == Some(&reference(&package.head)), FORMAT)?;
            // The preservation evidence is published with the migration, never left as a dangling reference.
            for check in list(payload, "checks")? {
                let sha = check.get("content").and_then(sha_of).ok_or_else(bad)?;
                require(names.iter().any(|n| n == sha), FORMAT)?;
            }
        }
        objects.extend(package.originals.iter().cloned());
        Ok(objects)
    }

    /// FP.28's publication: validation and `replay` from the package alone, then the durable generation and
    /// metadata, then the durable pointer switch. A failure before the switch leaves the previous generation
    /// active; a failure at the switch is read back from the pointer.
    pub fn publish(
        &mut self,
        id: &str,
        package: &Package,
        replay: &dyn Fn(&Package) -> Result<(), Refusal>,
    ) -> Result<Json, PublishError> {
        let previous = self.pointer()?;
        if let Some(sha) = &previous {
            self.generation(sha)?;
        }
        let objects = Self::validate(package)?;
        replay(package)?;
        let mut listed: Vec<Json> = objects.iter().map(|o| digest_ref("bytes", o)).collect();
        listed.sort_by(|a, b| sha_of(a).cmp(&sha_of(b)));
        listed.dedup();
        let archive = canonical(&Json::object([("objects", Json::List(listed))]));
        let meta = Json::object([
            ("format", Json::text("c600-publication")),
            ("version", Json::Int(1)),
            ("id", Json::text(id)),
            (
                "previous",
                previous.as_ref().map_or(Json::Null, |sha| {
                    Json::object([("role", Json::text("content")), ("sha256", Json::text(sha))])
                }),
            ),
            ("next", content_ref(&package.head)),
            ("archive", digest_ref("bytes", &archive)),
            ("validation", reference(&package.validation)),
        ]);
        let meta_bytes = canonical(&meta);
        for object in &objects {
            self.put(object).map_err(PublishError::NotPublished)?;
        }
        self.put(&archive).map_err(PublishError::NotPublished)?;
        let named = self.put(&meta_bytes).map_err(PublishError::NotPublished)?;
        // A torn tail is no switch. It is cut, durably, before the next line, so no later append completes it.
        let log = self.disk.read(POINTER).map_err(PublishError::NotPublished)?;
        if let Some(log) = log.filter(|l| !l.ends_with(b"\n") && !l.is_empty()) {
            let whole = log.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
            let cut = self.disk.truncate(POINTER, whole as u64).and_then(|()| self.disk.sync(POINTER));
            cut.map_err(PublishError::NotPublished)?;
        }
        let mut line = named.as_bytes().to_vec();
        line.push(b'\n');
        match self.disk.append(POINTER, &line).and_then(|()| self.disk.sync(POINTER)) {
            Ok(()) => Ok(meta),
            Err(error) => self.reconcile(&named, previous.as_deref(), meta, error),
        }
    }

    /// FP.28's uncertain switch: make the pointer log durable as it stands, read it back and validate what it names.
    fn reconcile(
        &mut self,
        named: &str,
        previous: Option<&str>,
        meta: Json,
        error: io::Error,
    ) -> Result<Json, PublishError> {
        match self.disk.read(POINTER) {
            Ok(None) => return Err(PublishError::NotPublished(error)),
            Ok(Some(_)) => {}
            Err(_) => return Err(PublishError::UnknownOutcome),
        }
        if self.disk.sync(POINTER).is_err() {
            return Err(PublishError::UnknownOutcome);
        }
        match self.pointer() {
            Ok(Some(sha)) if sha == named => match self.generation(&sha) {
                Ok(_) => Ok(meta),
                Err(_) => Err(PublishError::UnknownOutcome),
            },
            Ok(current) if current.as_deref() == previous => Err(PublishError::NotPublished(error)),
            _ => Err(PublishError::UnknownOutcome),
        }
    }

    /// The disk, for a test that crashes and reopens it.
    pub fn into_disk(self) -> D {
        self.disk
    }
}

/// The records of a validated generation, by content digest: what a reader activates.
pub fn records_of<D: Disk>(publisher: &mut Publisher<D>, meta: &Json) -> Result<BTreeMap<String, Json>, PublishError> {
    let archive = meta.get("archive").and_then(sha_of).ok_or_else(bad)?.to_string();
    let archive = read_canonical(&publisher.object(&archive)?)?;
    let mut records = BTreeMap::new();
    for entry in list(&archive, "objects")? {
        let sha = sha_of(entry).ok_or_else(bad)?.to_string();
        let bytes = publisher.object(&sha)?;
        if let Ok(record) = read_canonical(&bytes) {
            if record_header(&record).is_ok() {
                records.insert(sha, record);
            }
        }
    }
    Ok(records)
}
