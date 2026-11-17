//! FP.4's catalogue, FP.10's dispatch and unknown-field rules, FP.11's references, and chapter 05's `read_record` and
//! `write_record` (PS1.11 to PS1.13, PS2.2, PS2.9).
//!
//! Dispatch comes first: the `format`/`version` pair of a record, and of each Ref before it is followed and then of the
//! record it resolves to, is checked before any version-specific field. An unknown pair, a newer version, an unknown
//! required extension or a former draft identifier of a frozen kind is `PS2:input.persistence_version`: the reader
//! answers [`Read::Recorded`] with the original bytes, which never activate.

use std::collections::BTreeMap;

use c600_kernel::{IdentityBlock, Puzzle};

use super::codec::{canonical, read_canonical, sha_of, Json};
use super::codes::{CANON, FORMAT, PROVENANCE, VERSION};
use super::{require, Refusal};

/// FP.4: the record kinds wholly or partly in the format freeze. Their identifiers carry no `-draft` suffix.
pub const FROZEN_KINDS: [&str; 25] = [
    "word",
    "recipe",
    "macro",
    "macro-set",
    "protection-set",
    "binding",
    "key-map",
    "filter",
    "preferences",
    "checkpoint",
    "replay-path",
    "workspace",
    "evidence",
    "import-table",
    "migration",
    "exit",
    "void",
    "transition",
    "admission",
    "receipt",
    "workspace-inverse",
    "session-record",
    "publication",
    "research-bundle",
    "pick",
];

/// The draft kinds this reader also dispatches (FP.37's playback state).
pub const DRAFT_KINDS: [&str; 1] = ["playback"];

/// FP.4's reserved descriptors: they keep `-draft` until their own packet and are not dispatched here.
pub const RESERVED: [&str; 7] = [
    "c600-source-draft",
    "c600-module-draft",
    "c600-lock-draft",
    "c600-layout-draft",
    "c600-recording-draft",
    "c600-proof-obligation-draft",
    "c600-run-draft",
];

/// PS1.12's existing identifiers, reused without renaming: a slash is part of the identifier. Each needs its own
/// compatibility reader or converter; the c600 reader keeps such bytes recorded.
pub const RETAINED: [&str; 31] = [
    "C600-STUDIO-SESSION-v1",
    "C600-STUDIO-CERTIFICATE-v1",
    "magic600-jumbling-store/1",
    "magic600-jumbling-render/1",
    "magic600-session-proto/1",
    "C600-MIGRATE",
    "t3p-certificate/1",
    "t3p-check/1",
    "t3p-scan/1",
    "t3p-pairs/1",
    "t3p-plus/1",
    "t3p-levels/1",
    "t3p-locality/1",
    "t3p-chain/1",
    "macro-certificate/1",
    "macro-check/1",
    "pstar-census/1",
    "pstar-step/1",
    "pstar2-stop/1",
    "pstar2/2",
    "s16-rr/1",
    "s16-pxrr/1",
    "s16-transport/1",
    "r22b10/1",
    "r22b10/2",
    "runner-run/3",
    "literal-lattice/1",
    "lattice-phase-play/1",
    "ia16",
    "IMPORT-MANIFEST.json",
    "sanitized-research-archive/1",
];

/// The kinds whose records must carry a provenance block (FP.11).
const PROVENANCE_REQUIRED: [&str; 3] = ["word", "macro", "workspace"];
const HEADER: [&str; 5] = ["format", "version", "identity", "payload", "provenance"];

/// FP.4's exact `format` of a c600 record kind.
pub fn ident(kind: &str) -> String {
    if FROZEN_KINDS.contains(&kind) {
        format!("c600-{kind}")
    } else {
        format!("c600-{kind}-draft")
    }
}

/// Where an identifier stands in the catalogue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A frozen FP.4 kind, dispatched by this reader.
    Frozen(&'static str),
    /// A draft kind this reader dispatches.
    Draft(&'static str),
    /// A reserved descriptor: no reader yet.
    Reserved,
    /// An existing identifier of PS1.12, read only by its compatibility reader.
    Retained,
    Unknown,
}

pub fn entry(format: &str) -> Entry {
    if let Some(kind) = FROZEN_KINDS.iter().find(|k| ident(k) == format) {
        return Entry::Frozen(kind);
    }
    if let Some(kind) = DRAFT_KINDS.iter().find(|k| ident(k) == format) {
        return Entry::Draft(kind);
    }
    if RESERVED.contains(&format) {
        return Entry::Reserved;
    }
    if RETAINED.contains(&format) {
        return Entry::Retained;
    }
    Entry::Unknown
}

/// FP.10's dispatched formats: the frozen kinds and the dispatched drafts. A former draft identifier of a frozen kind
/// is not among them.
pub fn known_format(format: &str) -> bool {
    matches!(entry(format), Entry::Frozen(_) | Entry::Draft(_))
}

/// The record kind of a dispatched format.
pub fn kind_of(format: &str) -> Option<&'static str> {
    match entry(format) {
        Entry::Frozen(kind) | Entry::Draft(kind) => Some(kind),
        _ => None,
    }
}

fn keys(value: &Json) -> Option<Vec<&str>> {
    match value {
        Json::Object(m) => Some(m.keys().map(String::as_str).collect()),
        _ => None,
    }
}

/// Whether `value` is an object with exactly these keys.
pub(crate) fn has_exactly(value: &Json, wanted: &[&str]) -> bool {
    keys(value).is_some_and(|k| k.len() == wanted.len() && wanted.iter().all(|w| k.contains(w)))
}

/// FP.10: dispatch a record's or a Ref's `format` and `version` before any version-specific check.
pub fn supported(header: &Json) -> Result<(), Refusal> {
    require(header.get("format").is_some() && header.get("version").is_some(), FORMAT)?;
    let format = header.get("format").and_then(Json::as_text);
    require(format.is_some_and(known_format) && header.get("version") == Some(&Json::Int(1)), VERSION)
}

/// FP.10 and FP.11's header: exactly the five fields and optional `extensions`, each extension with exactly a text
/// `name`, a Bool `required` and a `value`, names unique and sorted. An unknown required extension is
/// `PS2:input.persistence_version`.
pub fn record_header(record: &Json) -> Result<(), Refusal> {
    supported(record)?;
    header_fields(record)
}

/// [`record_header`] after dispatch: the fields and extensions, for a format whose own reader dispatched it, such as a
/// reserved draft ([`super::run`]).
pub(crate) fn header_fields(record: &Json) -> Result<(), Refusal> {
    let k = keys(record).ok_or(Refusal::new(FORMAT))?;
    require(HEADER.iter().all(|f| k.contains(f)), FORMAT)?;
    require(k.iter().all(|f| HEADER.contains(f) || *f == "extensions"), FORMAT)?;
    let mut names = Vec::new();
    if let Some(extensions) = record.get("extensions") {
        let Json::List(extensions) = extensions else { return Err(Refusal::new(FORMAT)) };
        for extension in extensions {
            require(has_exactly(extension, &["name", "required", "value"]), FORMAT)?;
            let name = extension.get("name").and_then(Json::as_text);
            let required = match extension.get("required") {
                Some(Json::Bool(b)) => Some(*b),
                _ => None,
            };
            require(name.is_some() && required.is_some(), FORMAT)?;
            require(required == Some(false), VERSION)?;
            names.push(name.expect("a checked name"));
        }
    }
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    require(names == sorted, FORMAT)
}

/// FP.10 (MFF-02) and FP.11: dispatch a Ref, follow its content digest, dispatch the record it resolves to, and only
/// then check the Ref's shape. A known Ref whose digest names no record gives `missing`. As in the format packet's
/// reference `follow`, the caller then checks the expected kind and FP.11's equality of the record's `format` and
/// `version` with the Ref's, since the refusal is the caller's: `FORMAT` for a receipt or a playback record,
/// `PROVENANCE` for a pick.
pub fn follow<'a>(
    reference: &Json,
    records: &'a BTreeMap<String, Json>,
    missing: &'static str,
) -> Result<&'a Json, Refusal> {
    supported(reference)?;
    let content = reference.get("content");
    let sha = content.and_then(sha_of).ok_or(Refusal::new(FORMAT))?;
    let record = records.get(sha).ok_or(Refusal::new(missing))?;
    supported(record)?;
    let content = content.expect("a checked content");
    let shaped = has_exactly(reference, &["format", "version", "content"])
        && has_exactly(content, &["role", "sha256"])
        && content.get("role").and_then(Json::as_text) == Some("content");
    require(shaped, FORMAT)?;
    Ok(record)
}

/// FP.11: a Ref to a record, by its content digest.
pub fn reference(record: &Json) -> Json {
    Json::object([
        ("format", record.get("format").cloned().unwrap_or(Json::Null)),
        ("version", record.get("version").cloned().unwrap_or(Json::Null)),
        ("content", super::codec::content_ref(record)),
    ])
}

/// The outcome of an active read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    /// A dispatched record of the expected format that passed its header and provenance checks.
    Active(Json),
    /// Bytes this reader cannot interpret: an unknown or newer format or version, a required extension, a former
    /// draft identifier or a retained identifier. They stay byte for byte and never activate (PS2.2, FP.10).
    Recorded { bytes: Vec<u8>, code: &'static str },
}

/// FP.17's block, dispatched by its own version first: a block of another wire version is an unknown nested
/// version, which the record cannot hide (FP.10).
pub(crate) fn check_provenance(record: &Json, kind: &str) -> Result<(), Refusal> {
    match record.get("provenance") {
        Some(Json::Null) => require(!PROVENANCE_REQUIRED.contains(&kind), FORMAT),
        Some(block) => {
            let version = block.get("block").and_then(|b| b.get("version"));
            require(version.is_some(), PROVENANCE)?;
            require(version == Some(&Json::Int(1)), VERSION)?;
            c600_run::provenance::validate(block).map_err(|_| Refusal::new(PROVENANCE))
        }
        None => Err(Refusal::new(FORMAT)),
    }
}

/// Chapter 05's `read_record`: canonical bytes, then dispatch, the header, the expected format and the provenance
/// block. A version refusal answers [`Read::Recorded`]; every other refusal is an error.
pub fn read_record(bytes: &[u8], expected: &str) -> Result<Read, Refusal> {
    let value = read_canonical(bytes)?;
    let checked = record_header(&value).and_then(|()| {
        let format = value.get("format").and_then(Json::as_text).unwrap_or("");
        require(format == expected, FORMAT)?;
        check_provenance(&value, kind_of(format).unwrap_or(""))
    });
    match checked {
        Ok(()) => Ok(Read::Active(value)),
        Err(refusal) if refusal.code == VERSION => Ok(Read::Recorded { bytes: bytes.to_vec(), code: VERSION }),
        Err(refusal) => Err(refusal),
    }
}

/// FP.5's value rules a writer must keep: integers within ±(2^53 − 1) and nesting within `json_depth`.
pub(crate) fn check_value(value: &Json, depth: u128) -> Result<(), Refusal> {
    match value {
        Json::Int(n) => require(n.unsigned_abs() <= 9_007_199_254_740_991, CANON),
        Json::List(items) => {
            super::codec::charge("json_depth", depth + 1)?;
            items.iter().try_for_each(|item| check_value(item, depth + 1))
        }
        Json::Object(map) => {
            super::codec::charge("json_depth", depth + 1)?;
            map.values().try_for_each(|item| check_value(item, depth + 1))
        }
        _ => Ok(()),
    }
}

/// Chapter 05's `write_record`: only a dispatched version-1 record with a well-formed header and provenance block is
/// written, as FP.5 bytes within `record_decoded`. An evidence envelope's scopes must be immutable
/// (`PS1:evidence.scope_not_immutable`, PS1.35).
pub fn write_record(record: &Json) -> Result<Vec<u8>, Refusal> {
    record_header(record)?;
    let format = record.get("format").and_then(Json::as_text).unwrap_or("");
    let kind = kind_of(format).unwrap_or("");
    check_provenance(record, kind)?;
    check_value(record, 0)?;
    if kind == "evidence" {
        super::facts::check_scopes(record.get("payload").unwrap_or(&Json::Null))?;
    }
    let bytes = canonical(record);
    super::codec::charge("record_decoded", bytes.len() as u128)?;
    Ok(bytes)
}

/// KN0.1's identity block as the FP.5 value a record's `identity` holds: the jumbling fields are null for
/// 600-cell-Full, and the menu fields for a jumbling state in free mode.
pub fn identity_json(block: &IdentityBlock) -> Json {
    let optional = |field: &Option<String>| field.as_deref().map_or(Json::Null, Json::text);
    let puzzle = match block.puzzle {
        Puzzle::Full => "full",
        Puzzle::Jumbling => "jumbling",
    };
    Json::object([
        ("puzzle", Json::text(puzzle)),
        ("model_id", Json::text(&block.model_id)),
        ("menu_id", optional(&block.menu_id)),
        ("menu_order", optional(&block.menu_order)),
        ("contract_rev", optional(&block.contract_rev)),
    ])
}
