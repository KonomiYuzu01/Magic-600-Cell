//! Filter previews and their freshness key (KN0.9, H15.4, LI.41; draft, LP3's C6).
//!
//! A `FilterRef` names a predicate: the result reference `{"job", "digest"}` of a completed workspace job of a row that
//! prepares one ([`PREDICATE_ROWS`]). It resolves to the records that job wrote, each read with its content digest and
//! version, while each still holds the job's write ([`ResolvedFilter`]). A completed `filter.preview` job's result is
//! a filter preview, which the layer keeps by its job ID with KN0.9's dependencies ([`FilterPreview`]). A
//! `FilterPreviewRef` is `{"epoch": Nat, "job": Nat}` ([`preview_ref`]): the job and the engine epoch it ran in. Job
//! IDs start again after a reopen, and no epoch comes back (L10), so the pair names one filter preview in the session's
//! life, and a reference from before a reopen never names a later preview. `filter.apply` alone takes the gate
//! [`FRESH_FILTER_PREVIEW`], which holds while the named filter preview was made in the reference's epoch and its
//! dependencies equal the same fields read now: the engine epoch, the identity, the mode,
//! the state revision and the resolved filter's digest, which reads the records again. A write of a record read, an
//! undo's included, makes the preview stale; an edit of any other record does not, and neither does a protection or
//! workspace revision of its own. A stale or unknown filter preview is refused with `KN5:stale.filter_preview`.

use c600_kernel::json::{self, Value};

use crate::receipt::{digest, ResultRef};
use crate::workspace::RecordRead;

/// The gate `filter.apply` alone takes (LI.41): KN0.9's `filter_fresh(f)` for its submitted `FilterPreviewRef`.
pub const FRESH_FILTER_PREVIEW: &str = "fresh_filter_preview";

/// The row whose completed jobs are the filter previews a `FilterPreviewRef` names.
pub const FILTER_PREVIEW_ROW: &str = "filter.preview";

/// The rows whose completed workspace jobs prepare a predicate that a `FilterRef` names (draft): `structure.layer`,
/// the only one an entry serves so far (LP3's C6).
pub const PREDICATE_ROWS: [&str; 1] = ["structure.layer"];

/// The parts of a `FilterRef`, `{"job": Nat, "digest": Text}`: the job and its result digest. `None` for any other
/// form.
pub(crate) fn filter_ref(value: &Value) -> Option<(u64, &str)> {
    let fields = value.as_object()?;
    let job = fields.get("job")?.as_i64().and_then(|job| u64::try_from(job).ok())?;
    let digest = fields.get("digest")?.as_str()?;
    (fields.len() == 2).then_some((job, digest))
}

/// The `FilterPreviewRef` `{"epoch": Nat, "job": Nat}` of the filter preview that `filter.preview` job `job` made in
/// engine epoch `epoch` (draft).
pub fn preview_ref(job: u64, epoch: u64) -> Value {
    json::object([("epoch", Value::Int(epoch as i64)), ("job", Value::Int(job as i64))])
}

/// The parts of a `FilterPreviewRef`, `{"epoch": Nat, "job": Nat}`: the job and its epoch. `None` for any other form.
pub(crate) fn filter_preview_ref(value: &Value) -> Option<(u64, u64)> {
    let fields = value.as_object()?;
    let natural = |key: &str| fields.get(key)?.as_i64().and_then(|n| u64::try_from(n).ok());
    let (job, epoch) = (natural("job")?, natural("epoch")?);
    (fields.len() == 2).then_some((job, epoch))
}

/// What a `FilterRef` resolved to: the reference as given and the records its predicate's job wrote, each read as the
/// layer held it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedFilter {
    pub source: Value,
    pub reads: Vec<RecordRead>,
}

impl ResolvedFilter {
    /// KN0.9's `Digest<resolved filter>` (draft): the SHA-256 of the canonical `{"reads", "source"}`, each read in
    /// [`RecordRead::to_json`]'s form, so a record's version and content digest are both bound.
    pub fn digest(&self) -> String {
        digest(&json::object([
            ("reads", Value::Array(self.reads.iter().map(RecordRead::to_json).collect())),
            ("source", self.source.clone()),
        ]))
    }
}

/// KN0.9's `FilterPreviewDeps`. A filter preview is fresh when these equal the same fields read now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterPreviewDeps {
    pub epoch: u64,
    pub identity: String,
    pub mode: &'static str,
    pub state: u64,
    /// [`ResolvedFilter::digest`] of the filter the preview was computed for.
    pub filter: String,
}

impl FilterPreviewDeps {
    pub fn to_json(&self) -> Value {
        json::object([
            ("epoch", Value::Int(self.epoch as i64)),
            ("identity", Value::String(self.identity.clone())),
            ("mode", Value::String(self.mode.into())),
            ("state", Value::Int(self.state as i64)),
            ("filter", Value::String(self.filter.clone())),
        ])
    }
}

/// A completed `filter.preview` job's result as the layer keeps it: its result reference, its dependencies and the
/// filter it resolved, whose records a freshness check reads again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterPreview {
    /// The `filter.preview` job that made it.
    pub job: u64,
    pub result: ResultRef,
    pub deps: FilterPreviewDeps,
    pub resolved: ResolvedFilter,
}

impl FilterPreview {
    /// The `FilterPreviewRef` that names it: its job and the epoch it was made in ([`preview_ref`]).
    pub fn reference(&self) -> Value {
        preview_ref(self.job, self.deps.epoch)
    }
}

/// A completed job of a [`PREDICATE_ROWS`] row: its result digest and the records it wrote, each with the version its
/// write left.
#[derive(Clone, Debug)]
pub(crate) struct Predicate {
    pub digest: String,
    pub writes: Vec<(String, u64)>,
}
