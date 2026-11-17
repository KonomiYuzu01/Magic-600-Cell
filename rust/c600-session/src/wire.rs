//! Canonical forms: the journal's records, request digests (LI.5) and the preview draft digest (KN0.8). Every record is
//! one canonical `c600-json-1` value (FP.5); counters are decimal strings (PS1.5). The record shapes are M6 drafts
//! (format `c600-session-draft-1`, `rust/README.md`).

use std::sync::Arc;

use c600_digest::sha256_hex;
use c600_kernel::{IdentityBlock, Puzzle};
use c600_run::encode::encode;
use c600_run::provenance::{canonical, parse, Json};

use crate::journal::MAX_PAYLOAD;
use crate::types::{code, Envelope, Error, Input, JobKind, Lane, Origin, RecordValue, Row, WordSource};

pub(crate) const FORMAT: &str = "c600-session-draft-1";

pub(crate) fn num(n: u64) -> Json {
    Json::Text(n.to_string())
}

fn ints(values: impl IntoIterator<Item = i64>) -> Json {
    Json::List(values.into_iter().map(Json::Int).collect())
}

pub(crate) fn identity(block: &IdentityBlock) -> Json {
    let opt = |v: &Option<String>| v.as_deref().map_or(Json::Null, Json::text);
    let puzzle = match block.puzzle {
        Puzzle::Full => "full",
        Puzzle::Jumbling => "jumbling",
    };
    Json::object([
        ("puzzle", Json::text(puzzle)),
        ("model_id", Json::text(&block.model_id)),
        ("menu_id", opt(&block.menu_id)),
        ("menu_order", opt(&block.menu_order)),
        ("contract_rev", opt(&block.contract_rev)),
    ])
}

/// The canonical bytes of a record that will be stored, checked to read back as the same value.
pub(crate) fn checked(value: &Json) -> Result<Vec<u8>, Error> {
    let bytes = canonical(value);
    if bytes.len() > MAX_PAYLOAD {
        return Err(Error::new(code::RECORD_TOO_LARGE, format!("{} bytes", bytes.len())));
    }
    if parse(&bytes).as_ref() != Some(value) {
        return Err(Error::new(code::NONCANONICAL_VALUE, "the record does not read back as itself"));
    }
    Ok(bytes)
}

/// The digest of a workspace record's content.
pub(crate) fn content_digest(value: &RecordValue) -> String {
    sha256_hex(&canonical(&record_value(value)))
}

fn record_value(value: &RecordValue) -> Json {
    let provenance = String::from_utf8_lossy(&value.provenance).into_owned();
    Json::object([("payload", value.payload.clone()), ("provenance", Json::Text(provenance))])
}

/// KN0.8's `draft`: the literal letters, the resolved argument and the read set (record, content digest, version).
pub(crate) fn draft_digest(letters: &[i32], source: &WordSource, reads: &[(String, String, u64)]) -> String {
    let reads = reads
        .iter()
        .map(|(name, digest, version)| {
            Json::object([("record", Json::text(name)), ("digest", Json::text(digest)), ("version", num(*version))])
        })
        .collect();
    sha256_hex(&canonical(&Json::object([
        ("letters", ints(letters.iter().map(|&m| i64::from(m)))),
        ("source", word_source(source)),
        ("reads", Json::List(reads)),
    ])))
}

fn word_source(source: &WordSource) -> Json {
    match source {
        WordSource::Literal(letters) => Json::object([("literal", ints(letters.iter().map(|&m| i64::from(m))))]),
        WordSource::Record(name) => Json::object([("record", Json::text(name))]),
    }
}

/// A row's canonical arguments.
pub(crate) fn args(row: &Row) -> Json {
    match row {
        Row::Propose { word } => Json::object([("word", word_source(word))]),
        Row::Execute { preview } => Json::object([("preview", num(*preview))]),
        Row::Undo { hint, confirm } | Row::Redo { hint, confirm } => Json::object([
            ("hint", Json::object([("order", num(hint.order)), ("target", num(hint.target))])),
            ("confirm", Json::Bool(*confirm)),
        ]),
        Row::SwitchLine { line } => Json::object([("line", num(*line as u64))]),
        Row::Checkpoint { name } => Json::object([("name", Json::text(name))]),
        Row::WorkspaceSet { record, payload, provenance } => Json::object([
            ("record", Json::text(record)),
            ("payload", payload.clone()),
            ("provenance", Json::Text(c600_digest::sha256_hex(provenance))),
        ]),
        Row::ProtectionSet { orbits, confirm } => {
            Json::object([("orbits", ints(orbits.iter().copied())), ("confirm", Json::Bool(*confirm))])
        }
        Row::JobSubmit(spec) => {
            let values = spec
                .args
                .iter()
                .map(|v| {
                    let mut bytes = Vec::new();
                    match encode(v, &mut bytes) {
                        Ok(()) => Json::Text(sha256_hex(&bytes)),
                        Err(_) => Json::Null,
                    }
                })
                .collect();
            let inputs = spec
                .inputs
                .iter()
                .map(|input| match input {
                    Input::Current => Json::text("current"),
                    Input::Handle(h) => Json::object([
                        ("handle", num(h.id)),
                        ("epoch", num(h.revision.epoch)),
                        ("state", num(h.revision.state)),
                        ("digest", Json::text(&h.revision.digest)),
                    ]),
                })
                .collect();
            let lane = match spec.lane {
                Lane::Exclusive => "exclusive",
                Lane::Program => "program",
            };
            let kind = match spec.kind {
                JobKind::Query => "query",
                JobKind::File => "file",
                JobKind::Pinned => "pinned",
            };
            let amount = |n: u128| Json::Text(n.to_string());
            Json::object([
                ("program", Json::text(&spec.program_digest)),
                ("module", Json::text(&spec.module)),
                ("function", Json::text(&spec.function)),
                ("args", Json::List(values)),
                ("inputs", Json::List(inputs)),
                ("lane", Json::text(lane)),
                ("kind", Json::text(kind)),
                (
                    "budget",
                    Json::object([
                        ("steps", amount(spec.budget.steps)),
                        ("kernel_cost", amount(spec.budget.kernel_cost)),
                        ("memory", amount(spec.budget.memory)),
                        ("output", amount(spec.budget.output)),
                    ]),
                ),
            ])
        }
        Row::JobStop { job } => Json::object([("job", Json::object([("epoch", num(job.epoch)), ("n", num(job.n))]))]),
    }
}

pub(crate) fn args_digest(row: &Row) -> String {
    sha256_hex(&canonical(&args(row)))
}

/// LI.5's request digest: everything the invocation delivers, the token included, and the origin the channel stamped
/// on it (PHS.52 ruling 4).
pub fn request_digest(origin: Origin, envelope: &Envelope) -> String {
    let origin = match origin {
        Origin::Shell => "shell",
        Origin::Program => "program",
    };
    let token = envelope.token.as_ref().map_or(Json::Null, |t| t.json());
    sha256_hex(&canonical(&Json::object([
        ("session", Json::text(&envelope.session)),
        ("epoch", num(envelope.epoch)),
        ("command", Json::text(&envelope.command)),
        ("origin", Json::text(origin)),
        ("row", Json::text(envelope.row.name())),
        ("args", args(&envelope.row)),
        ("token", token),
    ])))
}

/// The first record of a journal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    pub session: String,
    pub model: String,
    pub start: String,
}

/// A durable action as the journal stores it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Act {
    Move { letters: Arc<[i32]> },
    Undo { target: u64 },
    Redo { target: u64 },
    Switch { line: usize },
    Checkpoint { name: String, object: String },
    Record { name: String, value: Option<RecordValue> },
    Protection { orbits: Vec<usize> },
}

/// One action record: the action, its invocation and the revision it produced. Admission and terminal records are
/// this one record (KN5.23; draft).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Frame {
    pub order: u64,
    pub command: String,
    pub request: String,
    pub row: String,
    pub epoch: u64,
    pub act: Act,
    pub pre: String,
    pub post: String,
    pub head: u64,
    pub state: u64,
    pub workspace: u64,
    pub protection: u64,
}

pub(crate) fn header_json(header: &Header) -> Json {
    Json::object([
        ("kind", Json::text("session")),
        ("format", Json::text(FORMAT)),
        ("session", Json::text(&header.session)),
        ("model", Json::text(&header.model)),
        ("start", Json::text(&header.start)),
    ])
}

fn act_json(act: &Act) -> Json {
    match act {
        Act::Move { letters } => {
            Json::object([("type", Json::text("move")), ("letters", ints(letters.iter().map(|&m| i64::from(m))))])
        }
        Act::Undo { target } => Json::object([("type", Json::text("undo")), ("target", num(*target))]),
        Act::Redo { target } => Json::object([("type", Json::text("redo")), ("target", num(*target))]),
        Act::Switch { line } => Json::object([("type", Json::text("switch")), ("line", num(*line as u64))]),
        Act::Checkpoint { name, object } => Json::object([
            ("type", Json::text("checkpoint")),
            ("name", Json::text(name)),
            ("object", Json::text(object)),
        ]),
        Act::Record { name, value } => Json::object([
            ("type", Json::text("record")),
            ("name", Json::text(name)),
            ("value", value.as_ref().map_or(Json::Null, record_value)),
        ]),
        Act::Protection { orbits } => {
            Json::object([("type", Json::text("protection")), ("orbits", ints(orbits.iter().map(|&o| o as i64)))])
        }
    }
}

pub(crate) fn frame_json(frame: &Frame) -> Json {
    Json::object([
        ("kind", Json::text("action")),
        ("order", num(frame.order)),
        ("command", Json::text(&frame.command)),
        ("request", Json::text(&frame.request)),
        ("row", Json::text(&frame.row)),
        ("epoch", num(frame.epoch)),
        ("act", act_json(&frame.act)),
        ("pre", Json::text(&frame.pre)),
        ("post", Json::text(&frame.post)),
        ("head", num(frame.head)),
        ("state", num(frame.state)),
        ("workspace", num(frame.workspace)),
        ("protection", num(frame.protection)),
    ])
}

fn text(value: &Json, key: &str) -> Option<String> {
    value.get(key)?.as_text().map(str::to_owned)
}

fn count(value: &Json, key: &str) -> Option<u64> {
    let s = value.get(key)?.as_text()?;
    let n: u64 = s.parse().ok()?;
    (n.to_string() == s).then_some(n)
}

fn int_list(value: &Json, key: &str) -> Option<Vec<i64>> {
    match value.get(key)? {
        Json::List(items) => items.iter().map(|i| if let Json::Int(n) = i { Some(*n) } else { None }).collect(),
        _ => None,
    }
}

/// Reads a stored record: canonical bytes of a header or an action record.
pub(crate) enum Stored {
    Header(Header),
    Frame(Frame),
}

pub(crate) fn read_record(bytes: &[u8]) -> Option<Stored> {
    let value = parse(bytes)?;
    if canonical(&value) != bytes {
        return None;
    }
    match value.get("kind")?.as_text()? {
        "session" => {
            if value.get("format")?.as_text()? != FORMAT {
                return None;
            }
            let header = Header {
                session: text(&value, "session")?,
                model: text(&value, "model")?,
                start: text(&value, "start")?,
            };
            (header_json(&header) == value).then_some(Stored::Header(header))
        }
        "action" => {
            let act = value.get("act")?;
            let act = match act.get("type")?.as_text()? {
                "move" => {
                    let letters: Option<Vec<i32>> =
                        int_list(act, "letters")?.into_iter().map(|m| i32::try_from(m).ok()).collect();
                    Act::Move { letters: letters?.into() }
                }
                "undo" => Act::Undo { target: count(act, "target")? },
                "redo" => Act::Redo { target: count(act, "target")? },
                "switch" => Act::Switch { line: usize::try_from(count(act, "line")?).ok()? },
                "checkpoint" => Act::Checkpoint { name: text(act, "name")?, object: text(act, "object")? },
                "record" => {
                    let value = match act.get("value")? {
                        Json::Null => None,
                        v => Some(RecordValue {
                            payload: v.get("payload")?.clone(),
                            provenance: text(v, "provenance")?.into_bytes(),
                        }),
                    };
                    Act::Record { name: text(act, "name")?, value }
                }
                "protection" => {
                    let orbits: Option<Vec<usize>> =
                        int_list(act, "orbits")?.into_iter().map(|o| usize::try_from(o).ok()).collect();
                    Act::Protection { orbits: orbits? }
                }
                _ => return None,
            };
            let frame = Frame {
                order: count(&value, "order")?,
                command: text(&value, "command")?,
                request: text(&value, "request")?,
                row: text(&value, "row")?,
                epoch: count(&value, "epoch")?,
                act,
                pre: text(&value, "pre")?,
                post: text(&value, "post")?,
                head: count(&value, "head")?,
                state: count(&value, "state")?,
                workspace: count(&value, "workspace")?,
                protection: count(&value, "protection")?,
            };
            (frame_json(&frame) == value).then_some(Stored::Frame(frame))
        }
        _ => None,
    }
}
