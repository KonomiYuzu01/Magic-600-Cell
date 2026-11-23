//! The executed modules' source maps and bytes (LG4.12, EV1.10): what a stop's source range names.
//!
//! [`Executed`] holds one lowering's source maps and the bytes it lowered, by their SHA-256. It answers the
//! interpreter's source ranges for the executed version, never a draft's: a draft's bytes have another digest, and a
//! draft that does not lower has no map at all (EV1.7, CT-LG4-07).

use std::collections::BTreeMap;
use std::sync::Arc;

use c600_digest::sha256_hex;
use c600_run::interp::SourceRanges;
use c600_run::meter::SourceRange;
use c600_syntax::{Lowered, SourceMap};

/// The source maps of one lowering and the bytes it lowered, by SHA-256 (lowercase hex).
#[derive(Clone, Debug, Default)]
pub struct Executed {
    maps: Vec<SourceMap>,
    bytes: BTreeMap<String, Arc<[u8]>>,
}

impl Executed {
    /// The maps of `lowered`, and `files`, the bytes it lowered, each keyed by its SHA-256.
    pub fn new(lowered: &Lowered, files: &[&[u8]]) -> Executed {
        let bytes = files.iter().map(|b| (sha256_hex(b), Arc::from(*b))).collect();
        Executed { maps: lowered.sources.clone(), bytes }
    }

    /// The same maps without the bytes: the run's ranges name digests whose bytes the debugger does not hold.
    pub fn without_bytes(&self) -> Executed {
        Executed { maps: self.maps.clone(), bytes: BTreeMap::new() }
    }

    /// The bytes held, by SHA-256.
    pub fn bytes(&self) -> &BTreeMap<String, Arc<[u8]>> {
        &self.bytes
    }
}

impl SourceRanges for Executed {
    fn range(&self, module: &str, function: &str, path: &str) -> Option<SourceRange> {
        let map = self.maps.iter().find(|m| m.module == module)?;
        let span = map.span(function, path)?;
        Some(SourceRange { digest: map.digest.clone(), start: span.start, end: span.end })
    }
}

/// Whether `range` selects whole scalars of `bytes`, whose SHA-256 must be the range's digest (EV1.10): its ends lie on
/// UTF-8 scalar boundaries within the bytes, and the bytes are UTF-8.
pub fn selects(bytes: &[u8], range: &SourceRange) -> bool {
    if sha256_hex(bytes) != range.digest || range.start > range.end || range.end > bytes.len() {
        return false;
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.is_char_boundary(range.start) && text.is_char_boundary(range.end),
        Err(_) => false,
    }
}
