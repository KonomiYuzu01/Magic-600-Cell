//! The messages of the runtime protocol (draft `c600-rt1-wire-1`, KI.15's RT1 message hole) and [`SCHEMA`], their
//! complete field list.
//!
//! Parent to child: `run` (one job), `answer` (the reply to one forwarded call) and `cancel` (an interruption of the
//! current run). Child to parent: `hello` (once, at start), `call` (one forwarded host call) and `done` (the end of
//! the current run). No message has a field for a user-action token or any other authority: a token never reaches the
//! runtime (KN5.15), and `tests/protocol.rs` holds every message against [`SCHEMA`].

use std::collections::BTreeSet;

use c600_check::Profile;
use c600_digest::sha256;
use c600_run::interp::{Request, RowTariff, Run, Sink};
use c600_run::meter::{Interruption, Summary};
use c600_run::value::Value;

use crate::codec::{self, Codes, OwnedRequest, OwnedSink, Unencodable};
use crate::program::{Ended, Job, JobOptions, Refused, Stage};
use crate::wire::{Dec, Enc, Malformed, PROTOCOL};

/// Message kind bytes. Parent-to-child kinds have the high bit clear, child-to-parent kinds have it set.
pub mod kind {
    pub const RUN: u8 = 0x01;
    pub const ANSWER: u8 = 0x02;
    pub const CANCEL: u8 = 0x03;
    pub const HELLO: u8 = 0x81;
    pub const CALL: u8 = 0x82;
    pub const DONE: u8 = 0x83;
}

/// One message kind and every field name its encoding may write, in the order written. Variant fields are dotted.
#[derive(Clone, Copy, Debug)]
pub struct MessageSchema {
    pub name: &'static str,
    pub kind: u8,
    pub fields: &'static [&'static str],
}

const REQUEST_FIELDS: [&str; 18] = [
    "request.snapshot",
    "request.snapshot.query",
    "request.snapshot.type",
    "request.declassified",
    "request.declassified.id",
    "request.declassified.type",
    "request.file_read",
    "request.file_read.path",
    "request.file_read.type",
    "request.clock",
    "request.propose",
    "request.propose.word",
    "request.propose.target",
    "request.propose.request",
    "request.snapshot_with",
    "request.snapshot_with.query",
    "request.snapshot_with.arg",
    "request.snapshot_with.type",
];

/// Every message and field of the protocol. Values and IR types inside a field are data, not fields: a value has no
/// variant that holds an authority, and a type is a signature.
pub const SCHEMA: [MessageSchema; 6] = [
    MessageSchema {
        name: "run",
        kind: kind::RUN,
        fields: &[
            "run",
            "module",
            "entry",
            "sources",
            "args",
            "limits",
            "tariffs",
            "draft",
            "seed",
            "record",
            "profile",
            "grant",
            "declassifiers",
            "source_ranges",
            "built_in",
        ],
    },
    MessageSchema {
        name: "answer",
        kind: kind::ANSWER,
        fields: &["run", "call", "answer.tariff", "answer.value", "answer.delivered", "answer.refused.code"],
    },
    MessageSchema { name: "cancel", kind: kind::CANCEL, fields: &["run", "why"] },
    MessageSchema { name: "hello", kind: kind::HELLO, fields: &["protocol", "ir", "contract"] },
    MessageSchema {
        name: "call",
        kind: kind::CALL,
        fields: &[
            "run",
            "call",
            "ack",
            "call.tariff",
            "call.perform",
            "call.sink",
            REQUEST_FIELDS[0],
            REQUEST_FIELDS[1],
            REQUEST_FIELDS[2],
            REQUEST_FIELDS[3],
            REQUEST_FIELDS[4],
            REQUEST_FIELDS[5],
            REQUEST_FIELDS[6],
            REQUEST_FIELDS[7],
            REQUEST_FIELDS[8],
            REQUEST_FIELDS[9],
            REQUEST_FIELDS[10],
            REQUEST_FIELDS[11],
            REQUEST_FIELDS[12],
            REQUEST_FIELDS[13],
            REQUEST_FIELDS[14],
            REQUEST_FIELDS[15],
            REQUEST_FIELDS[16],
            REQUEST_FIELDS[17],
            "sink.view",
            "sink.workspace",
            "sink.workspace.record",
            "sink.workspace.type",
            "sink.file",
            "sink.file.path",
            "sink.exact",
            "sink.exact.role",
            "sink.value",
            "sink.bytes",
        ],
    },
    MessageSchema {
        name: "done",
        kind: kind::DONE,
        fields: &[
            "run",
            "ack",
            "result.ran",
            "result.ran.summary",
            "result.ran.value",
            "result.ran.output",
            "result.refused",
            "result.refused.stage",
            "result.refused.codes",
            "result.refused.detail",
        ],
    },
];

/// The protocol, IR and execution contract a child announces in `hello`.
pub const IR: &str = c600_ir::IR_VERSION;
pub const CONTRACT: &str = c600_ir::EXEC_CONTRACT;

fn start(e: &mut Enc, kind: u8) {
    e.u8(PROTOCOL);
    e.u8(kind);
}

fn profile_code(p: Profile) -> u8 {
    match p {
        Profile::Solving => 0,
        Profile::ToolAuthoring => 1,
        Profile::Research => 2,
    }
}

fn profile_from(code: u8) -> Result<Profile, Malformed> {
    Ok(match code {
        0 => Profile::Solving,
        1 => Profile::ToolAuthoring,
        2 => Profile::Research,
        _ => return Err(Malformed::new("an unknown profile")),
    })
}

fn stage_code(s: Stage) -> u8 {
    match s {
        Stage::Parse => 0,
        Stage::Lower => 1,
        Stage::Admit => 2,
        Stage::Load => 3,
        Stage::Start => 4,
    }
}

fn stage_from(code: u8) -> Result<Stage, Malformed> {
    Ok(match code {
        0 => Stage::Parse,
        1 => Stage::Lower,
        2 => Stage::Admit,
        3 => Stage::Load,
        4 => Stage::Start,
        _ => return Err(Malformed::new("an unknown refusal stage")),
    })
}

// ---------------------------------------------------------------------------------------------------------------
// Parent to child

/// The parent's reply to one forwarded call.
#[derive(Clone, Debug)]
pub enum Answer {
    Tariff(RowTariff),
    Value(Value),
    Delivered,
    /// The host's refusal code.
    Refused(String),
}

/// A message the child reads.
#[derive(Debug)]
pub enum ToChild {
    Run { run: u64, job: Job },
    Answer { run: u64, call: u64, answer: Answer },
    Cancel { run: u64, why: Interruption },
}

pub fn encode_run(run: u64, job: &Job, e: &mut Enc) -> Result<(), Unencodable> {
    start(e, kind::RUN);
    e.field("run");
    e.u64(run);
    e.field("module");
    e.text(&job.module);
    e.field("entry");
    e.text(&job.entry);
    e.field("sources");
    e.count(job.sources.len());
    for s in &job.sources {
        e.bytes(s);
    }
    e.field("args");
    e.count(job.args.len());
    for a in &job.args {
        codec::encode_value(a, e)?;
    }
    let o = &job.options;
    e.field("limits");
    codec::encode_amounts(&o.limits, e);
    e.field("tariffs");
    codec::encode_tariffs(&o.tariffs, e);
    e.field("draft");
    codec::encode_draft(&o.draft, e);
    e.field("seed");
    match &o.seed {
        None => e.u8(0),
        Some((seed, path)) => {
            e.u8(1);
            e.bytes(seed);
            e.count(path.len());
            for p in path {
                e.text(p);
            }
        }
    }
    e.field("record");
    e.bool(o.record);
    e.field("profile");
    e.u8(profile_code(o.profile));
    e.field("grant");
    e.count(o.grant.len());
    for c in &o.grant {
        e.u8(codec::capability_code(*c));
    }
    e.field("declassifiers");
    e.count(o.declassifiers.len());
    for d in &o.declassifiers {
        e.text(d);
    }
    e.field("source_ranges");
    e.bool(o.source_ranges);
    e.field("built_in");
    match &o.built_in {
        None => e.u8(0),
        Some(rows) => {
            e.u8(1);
            e.count(rows.len());
            for row in rows {
                codec::encode_row(row, e);
            }
        }
    }
    Ok(())
}

pub fn encode_answer(run: u64, call: u64, answer: &Answer, e: &mut Enc) -> Result<(), Unencodable> {
    start(e, kind::ANSWER);
    e.field("run");
    e.u64(run);
    e.field("call");
    e.u64(call);
    match answer {
        Answer::Tariff(t) => {
            e.field("answer.tariff");
            e.u8(0);
            codec::encode_tariff(t, e);
        }
        Answer::Value(v) => {
            e.field("answer.value");
            e.u8(1);
            codec::encode_value(v, e)?;
        }
        Answer::Delivered => {
            e.field("answer.delivered");
            e.u8(2);
        }
        Answer::Refused(code) => {
            e.field("answer.refused.code");
            e.u8(3);
            e.text(code);
        }
    }
    Ok(())
}

pub fn encode_cancel(run: u64, why: Interruption, e: &mut Enc) {
    start(e, kind::CANCEL);
    e.field("run");
    e.u64(run);
    e.field("why");
    e.u8(codec::interruption_code(why));
}

fn header(d: &mut Dec) -> Result<u8, Malformed> {
    if d.u8()? != PROTOCOL {
        return Err(Malformed::new("another protocol version"));
    }
    d.u8()
}

pub fn decode_to_child(bytes: &[u8]) -> Result<ToChild, Malformed> {
    let mut d = Dec::new(bytes);
    let message = match header(&mut d)? {
        kind::RUN => {
            let run = d.u64()?;
            let module = d.text()?.to_string();
            let entry = d.text()?.to_string();
            let n = d.count(8)?;
            let sources = (0..n).map(|_| Ok(d.bytes()?.to_vec())).collect::<Result<Vec<_>, Malformed>>()?;
            let n = d.count(1)?;
            let args = (0..n).map(|_| codec::decode_value(&mut d)).collect::<Result<Vec<_>, _>>()?;
            let limits = codec::decode_amounts(&mut d)?;
            let tariffs = codec::decode_tariffs(&mut d)?;
            let draft = codec::decode_draft(&mut d)?;
            let seed = match d.u8()? {
                0 => None,
                1 => {
                    let seed = d.bytes()?.to_vec();
                    let n = d.count(8)?;
                    let path = (0..n).map(|_| Ok(d.text()?.to_string())).collect::<Result<Vec<_>, Malformed>>()?;
                    Some((seed, path))
                }
                _ => return Err(Malformed::new("an option flag other than 0 or 1")),
            };
            let record = d.bool()?;
            let profile = profile_from(d.u8()?)?;
            let n = d.count(1)?;
            let grant = (0..n).map(|_| codec::capability_from(d.u8()?)).collect::<Result<Vec<_>, _>>()?;
            let n = d.count(8)?;
            let declassifiers = (0..n).map(|_| Ok(d.text()?.to_string())).collect::<Result<Vec<_>, Malformed>>()?;
            let source_ranges = d.bool()?;
            let built_in = match d.u8()? {
                0 => None,
                1 => {
                    let n = d.count(1)?;
                    let mut rows = BTreeSet::new();
                    for _ in 0..n {
                        let row = codec::decode_row(&mut d)?;
                        if rows.last().is_some_and(|last| *last >= row) {
                            return Err(Malformed::new("class rows out of order or repeated"));
                        }
                        rows.insert(row);
                    }
                    Some(rows)
                }
                _ => return Err(Malformed::new("an option flag other than 0 or 1")),
            };
            let options = JobOptions {
                limits,
                tariffs,
                draft,
                seed,
                record,
                profile,
                grant,
                declassifiers,
                source_ranges,
                built_in,
            };
            ToChild::Run { run, job: Job { sources, module, entry, args, options } }
        }
        kind::ANSWER => {
            let run = d.u64()?;
            let call = d.u64()?;
            let answer = match d.u8()? {
                0 => Answer::Tariff(codec::decode_tariff(&mut d)?),
                1 => Answer::Value(codec::decode_value(&mut d)?),
                2 => Answer::Delivered,
                3 => Answer::Refused(d.text()?.to_string()),
                _ => return Err(Malformed::new("an unknown answer")),
            };
            ToChild::Answer { run, call, answer }
        }
        kind::CANCEL => ToChild::Cancel { run: d.u64()?, why: codec::interruption_from(d.u8()?)? },
        _ => return Err(Malformed::new("a message kind the child does not read")),
    };
    d.end()?;
    Ok(message)
}

// ---------------------------------------------------------------------------------------------------------------
// Child to parent

/// A host call as the child forwards it.
pub enum Call<'a> {
    Tariff(&'a Request<'a>),
    Perform(&'a Request<'a>),
    Sink(&'a Sink<'a>, &'a Value, &'a [u8]),
}

/// A host call as the parent receives it.
#[derive(Debug)]
pub enum Forwarded {
    Tariff(OwnedRequest),
    Perform(OwnedRequest),
    Sink(OwnedSink, Value, Vec<u8>),
}

/// A message the parent reads. `digest` is the SHA-256 of a call's request bytes (everything after `ack`), which
/// identifies a duplicate delivery (KI.5).
#[derive(Debug)]
pub enum ToParent {
    Hello { protocol: u32, ir: String, contract: String },
    Call { run: u64, call: u64, ack: u64, digest: [u8; 32], forwarded: Forwarded },
    Done { run: u64, ack: u64, ended: Ended },
}

pub fn encode_hello(e: &mut Enc) {
    start(e, kind::HELLO);
    e.field("protocol");
    e.u32(u32::from(PROTOCOL));
    e.field("ir");
    e.text(IR);
    e.field("contract");
    e.text(CONTRACT);
}

pub fn encode_call(run: u64, call: u64, ack: u64, c: &Call, e: &mut Enc) -> Result<(), Unencodable> {
    start(e, kind::CALL);
    e.field("run");
    e.u64(run);
    e.field("call");
    e.u64(call);
    e.field("ack");
    e.u64(ack);
    match c {
        Call::Tariff(r) => {
            e.field("call.tariff");
            e.u8(0);
            codec::encode_request(r, e)?;
        }
        Call::Perform(r) => {
            e.field("call.perform");
            e.u8(1);
            codec::encode_request(r, e)?;
        }
        Call::Sink(s, v, bytes) => {
            e.field("call.sink");
            e.u8(2);
            codec::encode_sink(s, e)?;
            e.field("sink.value");
            codec::encode_value(v, e)?;
            e.field("sink.bytes");
            e.bytes(bytes);
        }
    }
    Ok(())
}

pub fn encode_done(run: u64, ack: u64, ended: &Ended, e: &mut Enc) -> Result<(), Unencodable> {
    start(e, kind::DONE);
    e.field("run");
    e.u64(run);
    e.field("ack");
    e.u64(ack);
    match ended {
        Ended::Ran(r) => {
            e.field("result.ran");
            e.u8(0);
            e.field("result.ran.summary");
            codec::encode_summary(&r.summary, e);
            e.field("result.ran.value");
            match &r.value {
                None => e.u8(0),
                Some(v) => {
                    e.u8(1);
                    codec::encode_value(v, e)?;
                }
            }
            e.field("result.ran.output");
            match &r.output {
                None => e.u8(0),
                Some(o) => {
                    e.u8(1);
                    e.bytes(o);
                }
            }
        }
        Ended::Refused(r) => {
            e.field("result.refused");
            e.u8(1);
            e.field("result.refused.stage");
            e.u8(stage_code(r.stage));
            e.field("result.refused.codes");
            e.count(r.codes.len());
            for c in &r.codes {
                e.text(c);
            }
            e.field("result.refused.detail");
            e.text(&r.detail);
        }
    }
    Ok(())
}

fn decode_summary_value(d: &mut Dec, codes: &Codes) -> Result<(Summary, Option<Value>, Option<Vec<u8>>), Malformed> {
    let summary = codec::decode_summary(d, codes)?;
    let value = match d.u8()? {
        0 => None,
        1 => Some(codec::decode_value(d)?),
        _ => return Err(Malformed::new("an option flag other than 0 or 1")),
    };
    let output = match d.u8()? {
        0 => None,
        1 => Some(d.bytes()?.to_vec()),
        _ => return Err(Malformed::new("an option flag other than 0 or 1")),
    };
    Ok((summary, value, output))
}

/// Reads a child's message. `codes` are the stop codes a summary may name.
pub fn decode_to_parent(bytes: &[u8], codes: &Codes) -> Result<ToParent, Malformed> {
    let mut d = Dec::new(bytes);
    let message = match header(&mut d)? {
        kind::HELLO => {
            ToParent::Hello { protocol: d.u32()?, ir: d.text()?.to_string(), contract: d.text()?.to_string() }
        }
        kind::CALL => {
            let run = d.u64()?;
            let call = d.u64()?;
            let ack = d.u64()?;
            let digest = sha256(&bytes[bytes.len() - d.remaining()..]);
            let forwarded = match d.u8()? {
                0 => Forwarded::Tariff(codec::decode_request(&mut d)?),
                1 => Forwarded::Perform(codec::decode_request(&mut d)?),
                2 => {
                    let sink = codec::decode_sink(&mut d)?;
                    let value = codec::decode_value(&mut d)?;
                    Forwarded::Sink(sink, value, d.bytes()?.to_vec())
                }
                _ => return Err(Malformed::new("an unknown call")),
            };
            ToParent::Call { run, call, ack, digest, forwarded }
        }
        kind::DONE => {
            let run = d.u64()?;
            let ack = d.u64()?;
            let ended = match d.u8()? {
                0 => {
                    let (summary, value, output) = decode_summary_value(&mut d, codes)?;
                    Ended::Ran(Run { summary, value, output })
                }
                1 => {
                    let stage = stage_from(d.u8()?)?;
                    let n = d.count(8)?;
                    let codes = (0..n).map(|_| Ok(d.text()?.to_string())).collect::<Result<Vec<_>, Malformed>>()?;
                    Ended::Refused(Refused { stage, codes, detail: d.text()?.to_string() })
                }
                _ => return Err(Malformed::new("an unknown result")),
            };
            ToParent::Done { run, ack, ended }
        }
        _ => return Err(Malformed::new("a message kind the parent does not read")),
    };
    d.end()?;
    Ok(message)
}
