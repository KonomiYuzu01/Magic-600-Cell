//! PS3.17's `export_check_bundle` in FP.23's framing: a ZIP archive, method STORE, with `manifest.json` first and the
//! digest-named members `objects/<stored-sha256>` after it in ASCII digest order.
//!
//! The bundle carries each envelope's source record and its present dependencies by digest, keeps source names as
//! metadata only (case-only names may name distinct digests without overwrite), and lists every missing dependency.
//! A missing dependency prevents the check that requires it; bundle integrity implies no replay, and a historic PASS
//! receipt implies no new reproduction. The bundle carries no checker pins or check reports, so `checkers` is empty.
//! Nothing is executed: a producer's source text travels as bytes, if at all, and nothing here can run it.
//!
//! Only bytes that passed import integrity travel: an envelope or a member dependency with any failed integrity check
//! is refused, and every carried record is compared with the digest its corpus manifest records, a value independent
//! of the bytes in hand.
//!
//! The FP.11 wrapper is written with `identity` and `provenance` null; the evidence envelopes themselves (FP.19) are
//! not built in R0, so `envelopes` is empty and each source record is carried by digest instead. Dependencies with no
//! recorded digest cannot be FP.8 `DigestRef`s, so they are listed in the optional FP.10 extension
//! `missing_without_digest`.

use std::collections::BTreeMap;

use c600_digest::sha256_hex;
use c600_kernel::archive::crc32;
use c600_kernel::json::Value;

use crate::import::{ImportReport, LimitsRef, Recognition};
use crate::ArchiveError;

/// One dependency an envelope declares: present with its bytes, or missing with the reason it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    /// The source name the record cites (metadata).
    pub name: String,
    /// The stored-byte digest the record states, when it states one.
    pub sha256: Option<String>,
    /// The granted bytes, when present.
    pub bytes: Option<Vec<u8>>,
    /// What the dependency is for, or why it is missing, word for word.
    pub reason: String,
    /// The failed import integrity checks behind `bytes` when they come from a granted member; export refuses any.
    pub failed_checks: Vec<String>,
}

/// An envelope reference for export: the source record's bytes and identity with its declared dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceEnvelopeRef {
    pub name: String,
    pub stored_sha256: String,
    /// The stored-byte digest the corpus manifest records for the member: the independent value export checks.
    pub listed_sha256: String,
    /// The member's failed import integrity checks (its own and the corpus manifest's); export refuses any.
    pub failed_checks: Vec<String>,
    pub decoded_sha256: Option<String>,
    pub bytes: Vec<u8>,
    /// The record's own format, or `None` for an absent format.
    pub format: Option<String>,
    pub table_edition: String,
    pub table_sha256: String,
    /// The captured source edition from a stage manifest: name, digest and transformation, kept as metadata.
    pub original: Option<(String, String, String)>,
    pub dependencies: Vec<Dependency>,
    /// True when the envelope's scope holds revision-bound context, which persistence refuses (PS1.35).
    pub revision_bound: bool,
}

/// PS3.17's `CheckBundle`: the ZIP bytes, its manifest and the carrier's external stored digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckBundle {
    pub zip: Vec<u8>,
    pub manifest: Value,
    pub sha256: String,
}

impl ImportReport {
    /// The envelope reference for a granted record with a declared or an absent format, or `None` otherwise. It
    /// carries the member's failed integrity checks, which `export_check_bundle` refuses.
    pub fn envelope(&self, name: &str) -> Option<EvidenceEnvelopeRef> {
        let i = self.member(name)?;
        let m = &self.members[i];
        let bytes = self.stored_bytes(i)?.to_vec();
        let format = match &m.recognition {
            Recognition::Declared(f) => Some(f.clone()),
            Recognition::Absent(_) => None,
            _ => return None,
        };
        let original = self.stage_manifests.iter().flat_map(|s| &s.rows).find(|r| r.member == Some(i)).and_then(|r| {
            Some((r.original_name.clone()?, r.original_sha256.clone()?, r.transformation.clone().unwrap_or_default()))
        });
        Some(EvidenceEnvelopeRef {
            name: m.name.clone(),
            stored_sha256: m.stored_sha256.clone()?,
            listed_sha256: m.listed.stored_sha256.clone(),
            failed_checks: self.failed_checks(i),
            decoded_sha256: m.decoded_sha256.clone(),
            bytes,
            format,
            table_edition: self.table_edition.clone(),
            table_sha256: self.table_sha256.clone(),
            original,
            dependencies: Vec::new(),
            revision_bound: false,
        })
    }

    /// A dependency carried from a listed member: its bytes when granted, missing otherwise, with the member's failed
    /// integrity checks; `None` when no member has that name.
    pub fn dependency(&self, member: &str, name: &str, sha256: Option<&str>, reason: &str) -> Option<Dependency> {
        let i = self.member(member)?;
        Some(Dependency {
            name: name.to_string(),
            sha256: sha256.map(str::to_string),
            bytes: self.stored_bytes(i).map(<[u8]>::to_vec),
            reason: reason.to_string(),
            failed_checks: self.failed_checks(i),
        })
    }
}

impl EvidenceEnvelopeRef {
    /// Declares a dependency from the caller's bytes: present when `bytes` is given, missing otherwise.
    pub fn with_dependency(self, name: &str, sha256: Option<&str>, bytes: Option<Vec<u8>>, reason: &str) -> Self {
        self.with(Dependency {
            name: name.to_string(),
            sha256: sha256.map(str::to_string),
            bytes,
            reason: reason.to_string(),
            failed_checks: Vec::new(),
        })
    }

    /// Declares a dependency built elsewhere, such as `ImportReport::dependency`.
    pub fn with(mut self, dependency: Dependency) -> Self {
        self.dependencies.push(dependency);
        self
    }
}

/// Writes the bundle. Every carried byte is checked against its digest first; nothing is executed.
pub fn export_check_bundle(
    envelopes: &[EvidenceEnvelopeRef],
    limits: LimitsRef<'_>,
) -> Result<CheckBundle, ArchiveError> {
    let limits = limits.0;
    let s = |x: &str| Value::String(x.to_string());
    let o = |pairs: Vec<(&str, Value)>| Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect());
    let digest_ref = |hex: &str| o(vec![("role", s("bytes")), ("sha256", s(hex))]);

    let mut objects: BTreeMap<String, (Vec<u8>, &'static str, Option<String>)> = BTreeMap::new();
    let mut sources: Vec<Value> = Vec::new();
    let mut missing: BTreeMap<String, Value> = BTreeMap::new();
    let mut missing_without_digest: Vec<Value> = Vec::new();
    let mut total = 0usize;
    let mut table: Option<(String, String)> = None;
    for e in envelopes {
        if e.revision_bound {
            return Err(ArchiveError {
                code: crate::SCOPE_NOT_IMMUTABLE,
                detail: "an envelope's scope holds revision-bound context".to_string(),
            });
        }
        if let Some(first) = e.failed_checks.first() {
            return Err(ArchiveError::digest(&format!("an envelope's member failed import integrity: {first}")));
        }
        // The independent value is the corpus manifest's recorded digest, not one computed from these bytes.
        let actual = sha256_hex(&e.bytes);
        if actual != e.listed_sha256 || actual != e.stored_sha256 {
            return Err(ArchiveError::digest("an envelope's bytes disagree with the digest its manifest records"));
        }
        match &table {
            None => table = Some((e.table_edition.clone(), e.table_sha256.clone())),
            Some(t) if *t != (e.table_edition.clone(), e.table_sha256.clone()) => {
                return Err(ArchiveError::format("envelopes from two import-table editions"));
            }
            Some(_) => {}
        }
        sources.push(o(vec![
            ("name", s(&e.name)),
            ("stored", digest_ref(&e.stored_sha256)),
            ("original", e.original.as_ref().map_or(Value::Null, |(_, d, _)| digest_ref(d))),
            (
                "transformation",
                s(e.original.as_ref().map_or("none recorded: the archive's own stored bytes", |(_, _, t)| t.as_str())),
            ),
        ]));
        total += e.bytes.len();
        objects.insert(e.stored_sha256.clone(), (e.bytes.clone(), "record", e.decoded_sha256.clone()));
        for d in &e.dependencies {
            if let (Some(_), Some(first)) = (&d.bytes, d.failed_checks.first()) {
                return Err(ArchiveError::digest(&format!("a dependency's member failed import integrity: {first}")));
            }
            match (&d.bytes, &d.sha256) {
                (Some(b), Some(hex)) => {
                    if sha256_hex(b) != *hex {
                        return Err(ArchiveError::digest("a dependency's bytes disagree with its stated digest"));
                    }
                    total += b.len();
                    objects.entry(hex.clone()).or_insert((b.clone(), "dependency", None));
                    sources.push(o(vec![
                        ("name", s(&d.name)),
                        ("stored", digest_ref(hex)),
                        ("original", Value::Null),
                        ("transformation", s("none recorded: the archive's own stored bytes")),
                    ]));
                }
                (Some(_), None) => {
                    return Err(ArchiveError::digest("a present dependency has no stated digest to check"));
                }
                (None, Some(hex)) => {
                    missing.insert(hex.clone(), digest_ref(hex));
                }
                (None, None) => missing_without_digest.push(o(vec![("name", s(&d.name)), ("reason", s(&d.reason))])),
            }
        }
        if objects.len() + 1 > limits.max_members {
            return Err(ArchiveError::limit("bundle_members", limits.max_members, objects.len() + 1));
        }
        if total > limits.max_total_decoded_bytes {
            return Err(ArchiveError::limit("bundle_bytes", limits.max_total_decoded_bytes, total));
        }
    }
    sources.sort_by_key(Value::canonical);
    sources.dedup();
    let (edition, table_sha) = table.unwrap_or_default();
    let members: Vec<Value> = objects
        .iter()
        .map(|(hex, (bytes, role, decoded))| {
            o(vec![
                ("name", s(&format!("objects/{hex}"))),
                (
                    "blob",
                    o(vec![
                        ("stored", digest_ref(hex)),
                        ("decoded", decoded.as_deref().map_or(Value::Null, digest_ref)),
                        ("stored_bytes", Value::Int(bytes.len() as i64)),
                        ("decoded_bytes", Value::Null),
                        ("encoding", s("captured")),
                    ]),
                ),
                ("role", s(role)),
            ])
        })
        .collect();
    let manifest = o(vec![
        ("format", s("c600-research-bundle")),
        ("version", Value::Int(1)),
        ("identity", Value::Null),
        (
            "payload",
            o(vec![
                ("members", Value::Array(members)),
                ("sources", Value::Array(sources)),
                ("envelopes", Value::Array(Vec::new())),
                (
                    "table",
                    o(vec![
                        ("format", s("c600-import-table")),
                        ("version", Value::Int(1)),
                        ("content", o(vec![("role", s("content")), ("sha256", s(&table_sha))])),
                    ]),
                ),
                ("checkers", Value::Array(Vec::new())),
                ("missing", Value::Array(missing.into_values().collect())),
            ]),
        ),
        // FP.11: "`provenance` is FP.17; null is allowed only for a record that is neither a word, macro nor workspace
        // record". A `c600-research-bundle` manifest (FP.4, FP.23) is none of those, so null is the stated absence.
        ("provenance", Value::Null),
        ("extensions", extensions(&edition, missing_without_digest)),
    ]);
    let mut manifest_bytes = manifest.canonical().into_bytes();
    manifest_bytes.push(b'\n');
    let mut entries: Vec<(String, Vec<u8>)> = vec![("manifest.json".to_string(), manifest_bytes)];
    entries.extend(objects.into_iter().map(|(hex, (bytes, _, _))| (format!("objects/{hex}"), bytes)));
    let zip = store_zip(&entries)?;
    let sha256 = sha256_hex(&zip);
    Ok(CheckBundle { zip, manifest, sha256 })
}

/// FP.10's extension list: `[{name, required, value}]` with unique names in scalar order. Every entry is optional
/// (`required: false`) and changes no interpretation: the table is identified by `payload.table`'s digest, and the
/// other entries are notes and names of dependencies that have no digest to list under `payload.missing`.
fn extensions(table_edition: &str, missing_without_digest: Vec<Value>) -> Value {
    let s = |x: &str| Value::String(x.to_string());
    let mut entries: Vec<(&str, Value)> = vec![
        ("checker_status", s("this bundle carries no checker pins or check reports: its checkers list is empty")),
        ("missing_without_digest", Value::Array(missing_without_digest)),
        ("producers", s("no archived producer is executed; source text, if carried, is data")),
        ("replay", s("this bundle asserts no replay; a historic PASS receipt implies no new portable reproduction")),
        ("table_edition", s(table_edition)),
    ];
    entries.sort_by(|a, b| a.0.cmp(b.0));
    Value::Array(
        entries
            .into_iter()
            .map(|(name, value)| {
                c600_kernel::json::object([("name", s(name)), ("required", Value::Bool(false)), ("value", value)])
            })
            .collect(),
    )
}

/// FP.23's ZIP: STORE, UTF-8 flag only, 1980-01-01 00:00:00, version 20, attributes zero, no extras, comments or data
/// descriptors, CRC and sizes in both headers, no ZIP64.
fn store_zip(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, ArchiveError> {
    const FLAGS: u16 = 0x0800;
    const DOS_DATE: u16 = (1 << 5) | 1;
    let mut out = Vec::new();
    let mut central = Vec::new();
    let small = |n: usize| u32::try_from(n).ok().filter(|v| *v != u32::MAX);
    for (name, data) in entries {
        let offset = small(out.len()).ok_or(ArchiveError::limit("zip_offset", u32::MAX as usize - 1, out.len()))?;
        let size = small(data.len()).ok_or(ArchiveError::limit("zip_member", u32::MAX as usize - 1, data.len()))?;
        let name_len = u16::try_from(name.len()).map_err(|_| ArchiveError::format("a member name is too long"))?;
        let crc = crc32(data);
        let mut common = Vec::new();
        for v in [20u16, FLAGS, 0, 0, DOS_DATE] {
            common.extend_from_slice(&v.to_le_bytes());
        }
        for v in [crc, size, size] {
            common.extend_from_slice(&v.to_le_bytes());
        }
        common.extend_from_slice(&name_len.to_le_bytes());
        common.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&common);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&common);
        for v in [0u16, 0, 0] {
            central.extend_from_slice(&v.to_le_bytes());
        }
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let directory = small(out.len()).ok_or(ArchiveError::limit("zip_offset", u32::MAX as usize - 1, out.len()))?;
    let count = u16::try_from(entries.len()).ok().filter(|c| *c != u16::MAX).ok_or(ArchiveError::limit(
        "zip_entries",
        u16::MAX as usize - 1,
        entries.len(),
    ))?;
    let central_len = small(central.len()).ok_or(ArchiveError::limit("zip_directory", 0, central.len()))?;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    for v in [0u16, 0, count, count] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&central_len.to_le_bytes());
    out.extend_from_slice(&directory.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    Ok(out)
}
