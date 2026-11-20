//! The host's row-answer cache (DET.16, DET.23, DET.46, KN7.8; LP3 draft).
//!
//! [`Cached`] answers the rows a host names through a [`RowCache`]. A named row is a snapshot read whose answer is a
//! function of the read's inputs: the scope the host gives (the state it reads, such as a revision's digest, and
//! whatever else decides the answer), the row, its argument and its type. KN2.4's rows at a revision are such reads
//! (ONE.10). A row whose answer depends on what the host served before, such as LX-3's `tick`, is not, and a host names
//! no such row.
//!
//! - **Charges** (KN7.8, DET.16, DET.23, EV2.7). The tariff is always the host's, declared before admission from the
//!   request alone. So a hit is charged as a miss, and a read after an eviction costs nothing extra. A hit, an eviction
//!   or a rejected entry changes no value, charge, continuation or coverage: the run cannot tell them apart.
//! - **Entries** (DET.46). An entry is kept under its key, the read's inputs. It records those inputs, the answer and
//!   the SHA-256 of the answer's canonical bytes (DET.59) when it was captured. Before reuse the cache checks the
//!   recorded inputs against the read's and the answer against its digest. An entry that fails a check is dropped: the
//!   code `LG4:input.cache_entry_mismatch` goes to the host envelope ([`Cached::envelope`]), never to the run, the read
//!   is a miss, and the host's answer replaces the entry. As in `c600-build`, the checks catch a changed or damaged
//!   entry and authenticate nothing: an entry rewritten whole with a consistent digest passes, and a cache is its
//!   host's alone.
//! - **Sharing, eviction and relocation** (DET.16, DET.49, KN2.15). One [`RowCache`] may serve concurrent runs: a hit
//!   shares the immutable answer, and each run is charged for it in full (DET.24). [`RowCache::clear`] evicts every
//!   entry. [`RowCache::entries`] and [`RowCache::keep`] move captured entries to another cache, which checks them as
//!   it checks its own.
//!
//! A host's refusal is never kept, nor an answer without canonical bytes. No other request is cached: no proposal,
//! sink, clock, file or declassified read. The key lives in memory only and is no stored format; the type enters it in
//! its debug form. CT-RTE-07's B12 runs the frozen corpus's six host variants through it (`tests/ct_rte_07_full.rs`).

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use c600_digest::sha256;
use c600_run::encode::encode;
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::value::Value;

use crate::code::CACHE_ENTRY_MISMATCH;

/// A row's answer and the inputs it was captured from.
#[derive(Clone, Debug)]
pub struct Entry {
    /// The read's inputs as [`Cached`] keys them: the scope, the row, its argument and its type.
    pub inputs: Vec<u8>,
    pub answer: Value,
    /// The SHA-256 of the answer's canonical bytes when it was captured (DET.59).
    pub digest: [u8; 32],
}

/// Entries by key. Concurrent runs may share one.
#[derive(Debug, Default)]
pub struct RowCache {
    entries: Mutex<BTreeMap<Vec<u8>, Entry>>,
}

impl RowCache {
    fn lock(&self) -> MutexGuard<'_, BTreeMap<Vec<u8>, Entry>> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Evicts every entry (KN2.15).
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// Every entry with its key, in key order.
    pub fn entries(&self) -> Vec<(Vec<u8>, Entry)> {
        self.lock().iter().map(|(key, entry)| (key.clone(), entry.clone())).collect()
    }

    /// Keeps `entry` under `key`, unchecked until a read finds it.
    pub fn keep(&self, key: Vec<u8>, entry: Entry) {
        self.lock().insert(key, entry);
    }
}

/// A host whose named rows are answered through a [`RowCache`].
pub struct Cached<'a> {
    host: &'a mut dyn Host,
    cache: &'a RowCache,
    scope: &'a [u8],
    rows: &'a [&'a str],
    envelope: Vec<&'static str>,
}

impl<'a> Cached<'a> {
    /// `host`, with its rows `rows` answered through `cache` for reads in `scope`.
    pub fn new(host: &'a mut dyn Host, cache: &'a RowCache, scope: &'a [u8], rows: &'a [&'a str]) -> Cached<'a> {
        Cached { host, cache, scope, rows, envelope: Vec::new() }
    }

    /// The host-envelope diagnostics of the reads so far, in order (DET.46).
    pub fn envelope(&self) -> &[&'static str] {
        &self.envelope
    }

    /// The key of a snapshot read of a named row, its inputs with each part after its length; `None` for any other
    /// request.
    fn key(&self, request: &Request) -> Option<Vec<u8>> {
        let (kind, query, arg, ty) = match *request {
            Request::Snapshot { query, ty } => ("snapshot", query, b"".as_slice(), ty),
            Request::SnapshotWith { query, arg, ty } => ("snapshot_with", query, arg, ty),
            _ => return None,
        };
        if !self.rows.contains(&query) {
            return None;
        }
        let ty = format!("{ty:?}");
        let mut key = Vec::new();
        for part in [self.scope, kind.as_bytes(), query.as_bytes(), arg, ty.as_bytes()] {
            key.extend_from_slice(&(part.len() as u64).to_be_bytes());
            key.extend_from_slice(part);
        }
        Some(key)
    }
}

/// The SHA-256 of a value's canonical bytes; `None` for a value that has none.
fn digest(value: &Value) -> Option<[u8; 32]> {
    let mut bytes = Vec::new();
    encode(value, &mut bytes).ok()?;
    Some(sha256(&bytes))
}

impl Host for Cached<'_> {
    /// The host's tariff, for a hit too (KN7.8).
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.host.tariff(request)
    }

    /// A kept answer that passes its checks; otherwise the host's answer, kept for the next read.
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let Some(key) = self.key(request) else {
            return self.host.perform(request);
        };
        let kept = self.cache.lock().get(&key).cloned();
        if let Some(entry) = kept {
            if entry.inputs == key && digest(&entry.answer) == Some(entry.digest) {
                return Ok(entry.answer);
            }
            self.cache.lock().remove(&key);
            self.envelope.push(CACHE_ENTRY_MISMATCH);
        }
        let answer = self.host.perform(request)?;
        if let Some(sha) = digest(&answer) {
            self.cache.keep(key.clone(), Entry { inputs: key, answer: answer.clone(), digest: sha });
        }
        Ok(answer)
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.host.sink(sink, value, bytes)
    }
}
