//! PS3's research archive import and exchange (chapter 05 section 11; `docs/progress/1.0/language-lp5-plan.md`,
//! pieces R0 and R1).
//!
//! Import reads granted files only, keeps recognition, integrity, interpretation and recheck as separate results, keeps
//! an unmapped value word for word (PS3.10) and never executes a producer (PS3.17). Product code never walks a checkout
//! or runs Git; the Git-blob manifest is read by the test driver, which grants the extracted members as files.
//!
//! - [`table`]: PS3.10's import table, edition 1, keyed by (format, field, value) in FP.21's rule shape.
//! - [`discriminator`]: PS3.12 and FP.22's manifest/source-digest/JSON-pointer discriminator for absent-format records.
//! - [`enumerate`]: EVD.48's enumeration of (format, field, value) occurrences.
//! - [`import`]: `import_archive` with PS3.14's manifest checks, PS3.15's digest identity and the import inventory.
//! - [`bundle`]: PS3.17's `export_check_bundle` in FP.23's ZIP STORE framing, with missing dependencies declared.
//!
//! Nothing here calls a kernel engine: the kernel crate is used for its container readers only (gzip, DEFLATE, ZIP and
//! strict JSON), and nothing here starts a process or opens a network connection.

#![forbid(unsafe_code)]

pub mod bundle;
pub mod discriminator;
pub mod enumerate;
pub mod import;
pub mod table;

pub use bundle::{check_identity, export_check_bundle, CheckBundle, Dependency, EvidenceEnvelopeRef};
pub use discriminator::{Discrimination, Discriminator};
pub use import::{import_archive, GrantedFile, GrantedFiles, ImportReport, Limits, LimitsRef};
pub use table::{Facts, ImportTable, ImportTableRef, Outcome};

use std::fmt;

/// Chapter 13's codes this crate raises (05 `import_archive` and `export_check_bundle`).
pub const FORMAT: &str = "PS1:input.persistence_format";
pub const DIGEST_MISMATCH: &str = "PS1:input.digest_mismatch";
pub const SIZE_LIMIT: &str = "PS1:resource_limit.persistence_size";
pub const SCOPE_NOT_IMMUTABLE: &str = "PS1:evidence.scope_not_immutable";

/// Why an import or an export was refused, with a registered code and a detail that never quotes input bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveError {
    pub code: &'static str,
    pub detail: String,
}

impl ArchiveError {
    pub fn format(detail: &str) -> ArchiveError {
        ArchiveError { code: FORMAT, detail: detail.to_string() }
    }

    pub fn digest(detail: &str) -> ArchiveError {
        ArchiveError { code: DIGEST_MISMATCH, detail: detail.to_string() }
    }

    /// FP.9: a resource refusal names the row, the limit and the attempted count.
    pub fn limit(row: &str, limit: usize, attempted: usize) -> ArchiveError {
        ArchiveError { code: SIZE_LIMIT, detail: format!("{row}: limit {limit}, attempted {attempted}") }
    }
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ArchiveError {}

/// FP.8: 64 lowercase hex digits.
pub fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
