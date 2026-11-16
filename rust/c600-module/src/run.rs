//! Loading an approved closure and running its entry through the boundary (LG5.1, LG5.6, LG3.13, BND.39, BND.42).
//!
//! [`load`] refuses a closure whose digests differ from the approval's: different bytes never run under an approval
//! (LG5.1, LG5.6, CT-RTE-08). [`Ready::run`] builds the run's [`Context`] from the approval and puts two thin hosts
//! around `c600-run`'s [`Boundary`]: the outer one notes, at each proposal, whether the block the boundary will record
//! may read state; the inner one, between the boundary and the engine, refuses that proposal with
//! `LG3:capability.not_granted`. The boundary's preview admission runs first and keeps its own codes (BND.84, BND.85),
//! and the program gets the boundary's fixed acknowledgement `{base, id}` either way (BND.42, BND.43).

use std::cell::{Cell, RefCell};

use c600_run::boundary::{Answer, Boundary, Capture, Context, Pins, Shown, Store};
use c600_run::interp::{self, Host, Loaded, Options, Request, RowTariff, Run, Sink};
use c600_run::provenance::{self, field, hex, Json, Producer};
use c600_run::value::Value;

use crate::code::{DIGEST_MISMATCH, MODEL_NOT_SUPPORTED, NOT_GRANTED};
use crate::{refuse, Approval, Closure, Refusal};

/// A loaded closure and the approval it runs under.
pub struct Ready {
    loaded: Loaded,
    approval: Approval,
}

/// Loads a verified closure under an approval. A different closure or manifest digest is refused. The lock's bytes
/// may differ by their locations only, since a location never decides identity (LG5.1; Drafts: locations in the lock
/// only).
pub fn load(closure: &Closure, approval: &Approval) -> Result<Ready, Refusal> {
    if (closure.digest, closure.manifest_digest) != (approval.closure, approval.manifest) {
        let detail = format!(
            "the approval covers the closure {}; these bytes form the closure {} (LG5.1, LG5.6, RTE.45)",
            hex(&approval.closure),
            hex(&closure.digest)
        );
        return Err(refuse(DIGEST_MISMATCH, detail));
    }
    if closure.identity != approval.identity {
        return Err(refuse(MODEL_NOT_SUPPORTED, "the approval names another model identity (LG5.5)"));
    }
    let loaded = interp::load(&closure.program)?;
    Ok(Ready { loaded, approval: approval.clone() })
}

/// One run's inputs: the run's name and base revision, the entry's arguments and their stored blocks, and the pins the
/// policy in force can read.
pub struct RunRequest<'a> {
    pub run: &'a str,
    pub base: &'a str,
    pub args: Vec<Value>,
    pub inputs: Vec<Vec<u8>>,
    pub pins: Pins,
    pub options: Options<'a>,
}

/// The end of one run: the interpreter's result, what the shell shows for each proposal, the captures, and the
/// proposals that capability separation refused.
#[derive(Debug)]
pub struct Outcome {
    pub run: Run,
    pub shell: Vec<Shown>,
    pub captures: Vec<Capture>,
    /// The IDs of the proposals refused at the preview point because their block may read state (LG3.13, BND.39).
    pub separated: Vec<String>,
}

impl Ready {
    pub fn approval(&self) -> &Approval {
        &self.approval
    }

    /// Runs the approved entry with an engine and a store.
    pub fn run(&self, request: RunRequest, engine: &mut dyn Host, store: &mut Store) -> Result<Outcome, Refusal> {
        let a = &self.approval;
        let producer = Producer {
            contract: a.contract.clone(),
            checker: Some(a.checker.clone()),
            author: a.entry.author,
            module: a.module,
            entry: a.entry.function.clone(),
            profile: a.entry.profile,
        };
        let context = Context {
            run: request.run.to_string(),
            base: request.base.to_string(),
            producer,
            reads_state: a.admitted.separation.reads_state,
            inputs: request.inputs,
            pins: request.pins,
        };
        let gate = Gate::default();
        let mut split = Split { engine, gate: &gate };
        let boundary = Boundary::new(&mut split, store, context);
        let mut outer = Outer { boundary, gate: &gate };
        let run = self.loaded.run(&a.entry.module, &a.entry.function, request.args, &mut outer, &request.options)?;
        let shell = std::mem::take(&mut outer.boundary.shell);
        let captures = std::mem::take(&mut outer.boundary.captures);
        Ok(Outcome { run, shell, captures, separated: gate.separated.take() })
    }
}

/// What the two hosts share: whether the current proposal's block may read state, and the refused proposal IDs.
#[derive(Default)]
struct Gate {
    reads_state: Cell<bool>,
    refused: Cell<bool>,
    separated: RefCell<Vec<String>>,
}

/// Between the program and the boundary.
struct Outer<'a> {
    boundary: Boundary<'a>,
    gate: &'a Gate,
}

impl Host for Outer<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.boundary.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        if !matches!(request, Request::Propose { .. }) {
            return self.boundary.perform(request);
        }
        let block = self.boundary.block();
        self.gate.reads_state.set(provenance::read(&block).map_or(true, |b| field::may_read_state(&b)));
        self.gate.refused.set(false);
        let answer = self.boundary.perform(request);
        if self.gate.refused.get() {
            if let Some(shown) = self.boundary.shell.last() {
                self.gate.separated.borrow_mut().push(shown.id.clone());
            }
        }
        answer
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.boundary.sink(sink, value, bytes)
    }
}

/// Between the boundary and the engine. The boundary asks the engine for a preview only in the solving profile and
/// only after its own admission (BND.60, BND.84).
struct Split<'a> {
    engine: &'a mut dyn Host,
    gate: &'a Gate,
}

impl Host for Split<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.engine.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        if matches!(request, Request::Propose { .. }) && self.gate.reads_state.get() {
            self.gate.refused.set(true);
            return Err(NOT_GRANTED);
        }
        self.engine.perform(request)
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.engine.sink(sink, value, bytes)
    }
}

impl Outcome {
    /// The canonical result bytes CT-RTE-07 hashes (RTE.46; draft): the outcome, code, stop position, used amounts,
    /// output bytes, shell answers and captures, in FP.5's canonical JSON.
    pub fn canonical(&self) -> Vec<u8> {
        let s = &self.run.summary;
        let word = |w: &[i32]| Json::List(w.iter().map(|l| Json::Int(i64::from(*l))).collect());
        let nat = |n: u128| Json::Text(n.to_string());
        let shell = self.shell.iter().map(|row| {
            let answer = match &row.answer {
                Answer::Previewed(receipt) => {
                    // A receipt has no canonical form of its own; its fields encode as a record.
                    let fields = match receipt {
                        Value::Receipt(r) => Value::Record(r.clone()),
                        other => other.clone(),
                    };
                    let mut bytes = Vec::new();
                    let text = match c600_run::encode::encode(&fields, &mut bytes) {
                        Ok(()) => String::from_utf8_lossy(&bytes).into_owned(),
                        Err(_) => "noncanonical".to_string(),
                    };
                    Json::object([("kind", Json::text("previewed")), ("receipt", Json::Text(text))])
                }
                Answer::Refused(r) => Json::object([("code", Json::text(r.code)), ("kind", Json::text("refused"))]),
            };
            Json::object([
                ("answer", answer),
                ("block", Json::Text(hex(&row.block))),
                ("id", Json::text(&row.id)),
                ("separated", Json::Bool(self.separated.contains(&row.id))),
                ("target", Json::text(&row.target)),
                ("word", word(&row.word)),
            ])
        });
        let captures = self.captures.iter().map(|c| {
            Json::object([
                ("block", Json::Text(hex(&c.block))),
                ("id", Json::text(&c.id)),
                ("target", Json::text(&c.target)),
                ("word", word(&c.word)),
            ])
        });
        let stop = s.stop.as_ref().map_or(Json::Null, |p| {
            Json::object([("at", Json::text(&p.at)), ("phase", Json::text(p.phase)), ("scope", Json::text(&p.scope))])
        });
        let output = self.run.output.as_ref().map_or(Json::Null, |o| Json::Text(String::from_utf8_lossy(o).into()));
        provenance::canonical(&Json::object([
            ("captures", Json::List(captures.collect())),
            ("code", s.code.map_or(Json::Null, Json::text)),
            ("outcome", Json::text(s.outcome.name())),
            ("output", output),
            ("shell", Json::List(shell.collect())),
            ("stop", stop),
            (
                "used",
                Json::object([
                    ("kernel_cost", nat(s.used.kernel_cost)),
                    ("memory", nat(s.used.memory)),
                    ("output", nat(s.used.output)),
                    ("steps", nat(s.used.steps)),
                ]),
            ),
        ]))
    }
}
