//! KN5's stage 3: the command layer's workspace store.
//!
//! A workspace record is a named value with its provenance block (PS1.16). `workspace.set` ([`RECORD_ROW`]) writes one
//! as a durable action whose receipt is `completed(workspace)`: the workspace revision and the record's version
//! advance, and the state, the preview slot and the jobs stay (KN5.18, KN5.23). Its undo and redo are workspace
//! inverses through `session.undo` and `session.redo` (LI.26). A draft `{"record": name}` is a `ResolvedDraftRef`: the
//! preview reads the record's letters, or a work sheet's complete operation ([`crate::worksheet`]), at admission and
//! keeps the record's content digest and version in its read set, so a later write of the record makes the preview
//! stale (KN0.8).
//!
//! The layer depends on the kernel only and cannot read a provenance block itself. The host gives it a [`Provenance`]
//! adapter ([`crate::CommandLayer::set_provenance`]), which reads a block when a record is written (PS1.16) and decides
//! whether a record's word may be previewed in the session (KN5.26; BND.84, BND.85). Without an adapter the layer
//! refuses both.
//!
//! A word built from parts cites its parts' blocks by content digest (BND.24), and admission reads every part,
//! recursively; a part whose block nobody holds reads unknown and is refused (BND.21, BND.85). So that a restored
//! workspace keeps a word's recursive parts (CT-BND-27; draft), the layer keeps the part blocks the reader holds for a
//! block it writes ([`Provenance::parts`]) in the session, by the SHA-256 of their bytes: the action that writes the
//! record journals those the session did not keep yet, a replay keeps them again, and nothing removes them, an undo
//! included, since a kept block is only content and grants nothing. At admission the layer gives them back to the
//! reader ([`Provenance::admit_kept`]).

use std::collections::BTreeMap;

use c600_kernel::json::{self, Value};

use crate::receipt::{digest, Error};
use crate::worksheet::Worksheet;

/// The row a record write takes: M6's name, which H-08 holds since the command table's freeze at LP3 (PHS.53, CM.17);
/// its `payload: Json` is OPEN, a later change under PHS.52.
pub const RECORD_ROW: &str = "workspace.set";

/// A record's value: its payload and its provenance block, the block's canonical FP.17 JSON as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordValue {
    pub payload: Value,
    pub provenance: String,
}

impl RecordValue {
    pub fn to_json(&self) -> Value {
        json::object([("payload", self.payload.clone()), ("provenance", Value::String(self.provenance.clone()))])
    }

    /// Reads [`RecordValue::to_json`]'s form strictly.
    pub fn from_json(value: &Value) -> Option<RecordValue> {
        (value.as_object()?.len() == 2).then_some(())?;
        Some(RecordValue {
            payload: value.get("payload")?.clone(),
            provenance: value.get("provenance")?.as_str()?.to_owned(),
        })
    }

    /// The content digest a draft's read set names (KN0.8): the SHA-256 of [`RecordValue::to_json`]'s canonical form,
    /// as M6's `wire::content_digest` computes it.
    pub fn digest(&self) -> String {
        digest(&self.to_json())
    }

    /// The letters the record holds: its payload when that is a list of integers that fit a letter.
    pub fn letters(&self) -> Option<Vec<i64>> {
        self.payload.as_array()?.iter().map(|item| item.as_i64().filter(|&n| i32::try_from(n).is_ok())).collect()
    }

    /// The operation a draft `{"record": name}` previews (draft): the record's letters, or a work sheet's complete
    /// operation, Prepare, Macro and Cleanup in that order ([`Worksheet::letters`]).
    pub fn operation(&self) -> Option<Vec<i64>> {
        self.letters().or_else(|| Worksheet::from_payload(&self.payload).map(|sheet| sheet.letters()))
    }
}

/// One record write: the record and its value after the write. An undo of a record's first write removes the value
/// (`None`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordWrite {
    pub name: String,
    pub value: Option<RecordValue>,
}

impl RecordWrite {
    pub fn to_json(&self) -> Value {
        json::object([
            ("name", Value::String(self.name.clone())),
            ("value", self.value.as_ref().map_or(Value::Null, RecordValue::to_json)),
        ])
    }

    /// Reads [`RecordWrite::to_json`]'s form strictly.
    pub fn from_json(value: &Value) -> Option<RecordWrite> {
        (value.as_object()?.len() == 2).then_some(())?;
        let record = match value.get("value")? {
            Value::Null => None,
            other => Some(RecordValue::from_json(other)?),
        };
        Some(RecordWrite { name: value.get("name")?.as_str()?.to_owned(), value: record })
    }
}

/// A record as the layer holds it: its version, which every write advances (an undo's too), and its value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub version: u64,
    pub value: Option<RecordValue>,
}

/// One record a draft read at admission: the record, its content digest (empty when it holds no value) and its version
/// (KN0.8). A record that does not exist reads as version 0 with no value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRead {
    pub record: String,
    pub digest: String,
    pub version: u64,
}

impl RecordRead {
    /// The read of record `name` as the layer holds it now.
    pub fn of(name: &str, record: Option<&WorkspaceRecord>) -> RecordRead {
        RecordRead {
            record: name.to_owned(),
            digest: record.and_then(|r| r.value.as_ref()).map_or_else(String::new, RecordValue::digest),
            version: record.map_or(0, |r| r.version),
        }
    }

    pub fn to_json(&self) -> Value {
        json::object([
            ("record", Value::String(self.record.clone())),
            ("digest", Value::String(self.digest.clone())),
            ("version", Value::Int(self.version as i64)),
        ])
    }
}

/// What the draft digest of a draft that names workspace records covers (KN0.8): the draft argument (`source`), the
/// letters it resolved to and the records read to resolve them. The layer keeps it with the preview and reads the same
/// records again for every freshness check, so a write of a record read, its undo included, makes the preview stale;
/// a record's version never goes back, so writing the old value again does not make it fresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedDraft {
    pub source: Value,
    pub letters: Vec<i64>,
    pub reads: Vec<RecordRead>,
}

impl ResolvedDraft {
    /// KN0.8's `draft` in M6's form (`wire::draft_digest`): the SHA-256 of the canonical `{"letters", "source",
    /// "reads"}`.
    pub fn digest(&self) -> String {
        digest(&json::object([
            ("letters", Value::Array(self.letters.iter().map(|&letter| Value::Int(letter)).collect())),
            ("source", self.source.clone()),
            ("reads", Value::Array(self.reads.iter().map(RecordRead::to_json).collect())),
        ]))
    }
}

/// The host's reader of provenance blocks. LP3's host implements it with `c600-run`'s `provenance::read` and
/// `boundary::admit_preview`, which the layer cannot depend on.
pub trait Provenance: Send + Sync {
    /// Whether `block` is a valid provenance block (PS1.16). A record write with a block it refuses is refused with its
    /// error, `PS1:capability.provenance_incompatible`.
    fn check(&self, block: &[u8]) -> Result<(), Error>;

    /// Whether a word whose block is `block` may be previewed in this session (KN5.26): in a solving session the host
    /// reads the block and its parts' blocks (BND.84, BND.85). A preview of a record's word with a block it refuses is
    /// refused at admission with its error, `LG3:capability.not_granted` or
    /// `PS1:capability.provenance_incompatible`.
    fn admit(&self, block: &[u8]) -> Result<(), Error>;

    /// The blocks of `block`'s parts, recursively, that the host holds, each as its text (BND.24, BND.85). The layer
    /// keeps those it does not keep yet with the action that writes a record with `block`, durably, and gives them back
    /// at admission ([`Provenance::admit_kept`]), so a restored workspace keeps a word's recursive parts (CT-BND-27;
    /// draft). Only text is kept, so a part block that is not UTF-8 stays with the host; the order does not matter, and
    /// the walk must end on a cycle. The kept blocks count toward the size of the action's journal record, so a journal
    /// that bounds its records refuses an action whose parts exceed the bound (`c600-session`'s journal, with
    /// `PS1:resource_limit.record_too_large`). The default holds none.
    fn parts(&self, _block: &[u8]) -> Vec<String> {
        Vec::new()
    }

    /// [`Provenance::admit`] with the part blocks the session keeps, by the SHA-256 hex of their bytes: a part that the
    /// host does not hold itself is read from `kept`, and one that neither holds reads unknown (BND.21, BND.85). The
    /// layer calls this one at admission; the default ignores `kept`.
    fn admit_kept(&self, block: &[u8], _kept: &BTreeMap<String, String>) -> Result<(), Error> {
        self.admit(block)
    }

    /// The block of the record a conversion of a recording writes (EV4.23, BND.24; LP4-9 draft): `target` is `draft`
    /// or `source`, and `parts` the blocks of the words that remain in the result, in recorded order. The block lists
    /// each part, so BND.84 reads them when the result is previewed; a `source` result is a program's. The default has
    /// no block to give, so the conversion is refused with `KN5:input.row_not_built`.
    fn assemble(&self, target: &str, _parts: &[String]) -> Result<String, Error> {
        let detail = format!("the host's provenance reader assembles no block for a `{target}` conversion");
        Err(Error::new(crate::receipt::code::ROW_NOT_BUILT, detail))
    }
}
