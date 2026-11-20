//! FP.38's `c600-pick` record, chapter 07's `PickRecord` (BND.54), and the resolution of FP.17's `pick` field.
//!
//! [`check_payload`] checks a pick record's null provenance and its payload. [`super::record::write_record`] runs it
//! on every `c600-pick` record it writes. [`resolve`] follows a block's `pick` to its record in FP.10's order (MFF-02),
//! as the format packet's `pick_result` does:
//!
//! 1. the Ref's `format` and `version`, then its resolution, then the resolved record's `format` and `version`. An
//!    unknown or newer pair is `PS2:input.persistence_version`;
//! 2. the Ref's shape: an added or missing field is `PS1:input.persistence_format`;
//! 3. the Ref's kind, the record's `format` and `version` against the Ref, and `chosen` against the payload's.
//!
//! A pick whose record does not resolve, whose Ref names another kind, whose record's `format` or `version` differs
//! from its Ref, or whose `chosen` differs is `PS1:capability.provenance_incompatible` (BND.21, BND.85). Whether a
//! resolved pick is admitted is the boundary's question (BND.55), not this reader's.

use std::collections::BTreeMap;

use super::codec::{sha_of, Json};
use super::codes::{FORMAT, PROVENANCE};
use super::record::{follow, has_exactly, ident, record_header};
use super::{require, Refusal};

/// FP.38's payload fields.
const PAYLOAD: [&str; 5] = ["list", "fixed", "options", "chosen", "target"];
/// The payload's fields that hold a `DigestRef(content)`.
const DIGESTS: [&str; 3] = ["list", "options", "chosen"];

/// A pick that resolved to its record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The SHA-256 of the chosen option's content, which both `chosen` fields give.
    pub chosen: String,
    /// Whether the list is a fixed entry or a fixed object set (BND.54's `fixed`).
    pub fixed: bool,
}

fn is_content_ref(value: Option<&Json>) -> bool {
    let sha = value.and_then(sha_of);
    value.is_some_and(|v| {
        has_exactly(v, &["role", "sha256"]) && v.get("role").and_then(Json::as_text) == Some("content")
    }) && sha.is_some_and(|s| s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// FP.38: null `provenance`, and a payload of exactly `list`, `fixed`, `options`, `chosen` and `target`, with a Bool
/// `fixed` and a `DigestRef(content)` in `list`, `options` and `chosen`. `target` is any value. Anything else is
/// `PS1:input.persistence_format`. Answers `fixed`.
pub fn check_payload(record: &Json) -> Result<bool, Refusal> {
    require(record.get("provenance") == Some(&Json::Null), FORMAT)?;
    let payload = record.get("payload").ok_or_else(|| Refusal::new(FORMAT))?;
    require(has_exactly(payload, &PAYLOAD), FORMAT)?;
    let Some(Json::Bool(fixed)) = payload.get("fixed") else { return Err(Refusal::new(FORMAT)) };
    require(DIGESTS.iter().all(|key| is_content_ref(payload.get(key))), FORMAT)?;
    Ok(*fixed)
}

/// Follows `block`'s `pick` to its `c600-pick` record among `records` (by content digest) in FP.10's order. The block
/// must be an FP.17 block with a non-null `pick`; anything else is `PS1:capability.provenance_incompatible`.
pub fn resolve(block: &Json, records: &BTreeMap<String, Json>) -> Result<Resolved, Refusal> {
    c600_run::provenance::validate(block).map_err(|_| Refusal::new(PROVENANCE))?;
    let pick = block.get("pick").filter(|pick| **pick != Json::Null).ok_or_else(|| Refusal::new(PROVENANCE))?;
    let reference = pick.get("record").ok_or_else(|| Refusal::new(PROVENANCE))?;
    let record = follow(reference, records, PROVENANCE)?;
    require(reference.get("format").and_then(Json::as_text) == Some(ident("pick").as_str()), PROVENANCE)?;
    record_header(record)?;
    let same = record.get("format") == reference.get("format") && record.get("version") == reference.get("version");
    require(same, PROVENANCE)?;
    let fixed = check_payload(record)?;
    let chosen = record.get("payload").and_then(|payload| payload.get("chosen"));
    require(chosen == pick.get("chosen"), PROVENANCE)?;
    let chosen = chosen.and_then(sha_of).ok_or_else(|| Refusal::new(PROVENANCE))?.to_string();
    Ok(Resolved { chosen, fixed })
}
