//! 0.4's preview certificate `C600-STUDIO-CERTIFICATE-v1` (PS1.39) and its import states (PS3.7).
//!
//! `session.py` digests the initial public object, then adds diagnostic and context fields and, on the way out, a
//! preview token. The identifier is therefore recomputed over exactly those initial fields: a later field never enters
//! it and never changes what the import records. An imported certificate is a recorded claim of state B (the emitted
//! net action was verified in 0.4) with check mode "stated: not replayed" (EVD.47); no import upgrades it. State C,
//! the contextual check (`conflicts`), and state D, the preview token, are dropped: neither is persisted as evidence
//! or permission.

use std::collections::BTreeMap;

use c600_digest::sha256_hex;
use c600_evidence::CheckMode;
use c600_kernel::json::{self, Value};

use super::codes::{DIGEST, FORMAT};
use super::{require, Refusal};

/// The retained identifier of a 0.4 preview certificate.
pub const CERTIFICATE: &str = "C600-STUDIO-CERTIFICATE-v1";

/// The initial public object `session.py` digests into `certificate_id`, as `verify.py` lists it.
pub const ID_FIELDS: [&str; 11] = [
    "format",
    "model_id",
    "recipe",
    "source_to_destination_sha256",
    "primitive_count",
    "star_count",
    "support",
    "pre_state",
    "post_state",
    "evidence",
    "native_windows_equivalence",
];

/// State C (PS3.7): a check against the protection at preview time. Never persisted or resurrected as evidence.
pub const CONTEXTUAL: [&str; 1] = ["conflicts"];
/// State D (PS3.7): the preview token. Never persisted or imported as permission.
pub const PERMISSION: [&str; 1] = ["token"];

/// PS3.7's import states that an import can produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// B: the emitted net action was verified in 0.4. A recorded claim; only an independent 1.0 re-verification for
    /// its immutable scope changes it.
    RecordedNetAction,
}

/// An imported certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    /// The recomputed identifier, equal to the recorded one.
    pub id: String,
    /// The initial public object, as recorded.
    pub initial: Value,
    pub state: State,
    pub mode: CheckMode,
    /// The later fields kept as recorded context: diagnostics, timing, the note and the assistance label.
    pub context: BTreeMap<String, Value>,
    /// The fields of states C and D that were present and dropped.
    pub dropped: Vec<String>,
}

/// The identifier of a certificate object: SHA-256 of the canonical initial public object, exactly as `session.py`
/// computes it.
pub fn certificate_id(certificate: &Value) -> Result<String, Refusal> {
    let fields = certificate.as_object().ok_or(Refusal::new(FORMAT))?;
    let mut initial = BTreeMap::new();
    for name in ID_FIELDS {
        let value = fields.get(name).ok_or(Refusal::new(FORMAT))?;
        initial.insert(name.to_string(), value.clone());
    }
    Ok(sha256_hex(Value::Object(initial).canonical().as_bytes()))
}

/// Imports a 0.4 certificate's bytes: the identifier over the initial fields only, the claim as recorded, the later
/// fields as context, and states C and D dropped.
pub fn import_certificate(bytes: &[u8]) -> Result<Certificate, Refusal> {
    let value = json::parse(bytes).map_err(|_| Refusal::new(FORMAT))?;
    let fields = value.as_object().ok_or(Refusal::new(FORMAT))?;
    require(fields.get("format").and_then(Value::as_str) == Some(CERTIFICATE), FORMAT)?;
    let id = certificate_id(&value)?;
    let recorded = fields.get("certificate_id").and_then(Value::as_str).ok_or(Refusal::new(FORMAT))?;
    require(recorded == id, DIGEST)?;
    let mut initial = BTreeMap::new();
    let mut context = BTreeMap::new();
    let mut dropped = Vec::new();
    for (name, field) in fields {
        if ID_FIELDS.contains(&name.as_str()) {
            initial.insert(name.clone(), field.clone());
        } else if CONTEXTUAL.contains(&name.as_str()) || PERMISSION.contains(&name.as_str()) {
            dropped.push(name.clone());
        } else if name != "certificate_id" {
            context.insert(name.clone(), field.clone());
        }
    }
    Ok(Certificate {
        id,
        initial: Value::Object(initial),
        state: State::RecordedNetAction,
        mode: CheckMode::StatedNotReplayed,
        context,
        dropped,
    })
}
