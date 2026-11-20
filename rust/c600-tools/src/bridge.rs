//! The host bridge from `c600-run` onto `c600-command`: the runtime's [`Host`] for one run of a tool in the solving
//! profile, used through [`c600_run::boundary::Boundary`] (LP3, C5).
//!
//! A proposal that passes preview admission becomes one invocation of the row its target names, sent on the run's
//! channel: the bridge stamps [`Caller::Runtime`] for the run, as the host does (the envelope names no caller;
//! ruling 4, PHS.52), with a fresh command ID, the layer's current epoch, the literal word as an inline draft,
//! `{"draft": {"recipe": [{"kind": "word", "moves": [...]}]}}`, and no token. The bridge filters no row: the command
//! layer refuses a runtime invocation of a commit or app row, and any runtime invocation that carries a token
//! (ADR-L02, KN5.15). The bridge answers the boundary with a short form of the layer's receipt, which only the shell
//! sees; the program gets the boundary's fixed acknowledgement (BND.42, BND.43). Besides proposals, the bridge serves
//! four reads, each to a snapshot read with an argument ([`crate::kernel_rows`]; KN5, LP2 draft): KN2.4's
//! `positions` and `labels` rows at the layer's head, which LB2's exact completion reads (CT-LB2-04); KN2.4's
//! `effect` row, a recipe's net map, which reads no state and which LB2's recipes and reports read (CT-LB2-01); and
//! KN2.4's `protection` row, a captured lock judged against a recipe at the layer's head, which LB2's Net and Strict
//! protection reads (CT-LB2-02).
//! Capability separation keeps an entry that reads them from also proposing: admission refuses an entry that does both
//! (LQ21, LG3.13, BND.39). The bridge grants no other request: no other state read, no read without an argument, no
//! workspace or file write and no clock.
//!
//! The draft carries the word in one word step, with one exception (LB2.3, LB2.4, LB1.11). For one run a host may give
//! the bridge the solver's word steps of the run's input recipe ([`Steps`]). A proposal of exactly their literal word
//! that is longer than one chunk ([`WORD_CHUNK`]) goes in the solver's word steps, in order, in the one invocation
//! ([`proposal_args`]). The bridge never chooses where to cut: every other word goes in one word step, and the layer
//! refuses it whole when it is longer than one chunk.

use c600_command::{Caller, CommandLayer, Invocation, MemoryStore, Receipt, Revision, Status, Store, Token};
use c600_kernel::json::{self, Value as Json};
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::value::Value;

use crate::kernel_rows;

/// The result bound of a proposal's answer (a draft; the command table prices the row when it freezes).
pub const RESULT_BOUND: u128 = 4096;

/// 0.4's word limit, which LB2.4 keeps: the most letters one word step of a draft carries (KN2).
pub const WORD_CHUNK: usize = 100_000;

/// The revision a run reads, as the acknowledgement's `base`: `<session>@<epoch>/<head>`.
pub fn base(revision: &Revision) -> String {
    format!("{}@{}/{}", revision.session, revision.epoch, revision.head)
}

/// The absolute arguments of a proposal of a literal word in one word step:
/// `{"draft": {"recipe": [{"kind": "word", "moves": [...]}]}}`.
pub fn word_args(word: &[i32]) -> Json {
    steps_args(&[word])
}

/// The absolute arguments of a proposal of a literal word in word steps, in order:
/// `{"draft": {"recipe": [{"kind": "word", "moves": [...]}, ...]}}`. One step gives [`word_args`].
pub fn steps_args(steps: &[&[i32]]) -> Json {
    let step = |word: &&[i32]| {
        let moves = Json::Array(word.iter().map(|&letter| Json::Int(i64::from(letter))).collect());
        json::object([("kind", Json::String("word".into())), ("moves", moves)])
    };
    json::object([("draft", json::object([("recipe", Json::Array(steps.iter().map(step).collect()))]))])
}

/// The solver's word steps of one run's input recipe: the recipe's literal word and the length of each of its word
/// steps, in order. A host gives them to the bridge only for an input recipe of plain word steps; a recipe with a star
/// step or a word step with another field has none ([`crate::expand::word_steps`]), and its literal word goes in one
/// word step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Steps {
    literal: Vec<i32>,
    lengths: Vec<usize>,
}

impl Steps {
    /// The word steps of `literal` with these lengths, in order; None when the lengths do not add up to the literal
    /// word's length. That is the only check: the host passes the lengths of the input recipe's own word steps
    /// ([`crate::expand::word_steps`] of the recipe whose literal word is `literal`), never lengths it chose.
    pub fn new(literal: &[i32], lengths: Vec<usize>) -> Option<Steps> {
        let total = lengths.iter().try_fold(0usize, |total, &length| total.checked_add(length))?;
        (total == literal.len()).then(|| Steps { literal: literal.to_vec(), lengths })
    }

    /// `word` cut at the solver's step boundaries when it is exactly the literal word, letter for letter; None for
    /// any other word, one of the same length included.
    pub fn cut<'w>(&self, word: &'w [i32]) -> Option<Vec<&'w [i32]>> {
        if word != self.literal.as_slice() {
            return None;
        }
        let mut rest = word;
        let mut steps = Vec::with_capacity(self.lengths.len());
        for &length in &self.lengths {
            let (step, tail) = rest.split_at(length);
            steps.push(step);
            rest = tail;
        }
        Some(steps)
    }
}

/// The absolute arguments of a proposal of `word`, given the solver's word steps of the run's input recipe, if the
/// host gave them (LB2.3, LB2.4, LB1.11). A word longer than one chunk that is exactly the literal of `steps` goes in
/// the solver's word steps, in order ([`steps_args`]). Every other word goes in one word step ([`word_args`]): a word
/// of at most one chunk, whatever the solver's steps, and a word that is not the solver's literal, which the layer
/// refuses whole when it is longer than one chunk. The bridge never chooses where to cut.
pub fn proposal_args(word: &[i32], steps: Option<&Steps>) -> Json {
    match steps.filter(|_| word.len() > WORD_CHUNK).and_then(|steps| steps.cut(word)) {
        Some(cut) => steps_args(&cut),
        None => word_args(word),
    }
}

/// One invocation the bridge sent, with the caller it stamped and the layer's receipt.
#[derive(Clone, Debug)]
pub struct Sent {
    pub invocation: Invocation,
    pub caller: Caller,
    pub receipt: Receipt,
}

/// The runtime's host for one run on one session's command layer.
pub struct Bridge<'l, S: Store = MemoryStore> {
    layer: &'l mut CommandLayer<S>,
    run: String,
    /// The solver's word steps of the run's input recipe, if the host gave them.
    steps: Option<Steps>,
    /// Every invocation the bridge sent, in order.
    pub sent: Vec<Sent>,
}

impl<'l, S: Store> Bridge<'l, S> {
    /// The host of run `run` on `layer`. A run's name is new in its session: the bridge's command IDs are
    /// `<run>/<n>`, and the layer answers a command ID it knows by its outcome lookup. The bridge has no solver's word
    /// steps: every proposal goes in one word step.
    pub fn new(layer: &'l mut CommandLayer<S>, run: &str) -> Self {
        Bridge { layer, run: run.to_owned(), steps: None, sent: Vec::new() }
    }

    /// This bridge with the solver's word steps of the run's input recipe, which the host gives for the run; a
    /// proposal's arguments are then [`proposal_args`] of the word and these steps. None keeps one word step.
    pub fn with_steps(mut self, steps: Option<Steps>) -> Self {
        self.steps = steps;
        self
    }

    /// One invocation of `row` with absolute arguments on this run's channel, stamped [`Caller::Runtime`] for the run,
    /// with a fresh command ID and the layer's current epoch. A proposal never carries a token; `token` is there so a
    /// test can show that the layer refuses a runtime invocation carrying one.
    pub fn invoke(&mut self, row: &str, args: Json, token: Option<Token>) -> Receipt {
        let invocation = Invocation {
            command: format!("{}/{}", self.run, self.sent.len() + 1),
            row: row.to_owned(),
            args,
            token,
            epoch: self.layer.revision().epoch,
        };
        let caller = Caller::Runtime { run: self.run.clone() };
        let receipt = self.layer.invoke(&caller, invocation.clone());
        self.sent.push(Sent { invocation, caller, receipt: receipt.clone() });
        receipt
    }
}

/// What the shell sees of the layer's answer to a proposal: the revision the run read, the command ID, the status's
/// name and the job.
fn answer(receipt: &Receipt, base: &str) -> Value {
    let mut fields = vec![
        ("base", Value::text(base)),
        ("id", Value::text(&receipt.command)),
        ("status", Value::tag(receipt.status.name())),
    ];
    if let Some(job) = receipt.job {
        fields.push(("job", Value::int(job)));
    }
    Value::receipt(fields)
}

impl<S: Store> Host for Bridge<'_, S> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            // KN5.9's cost: the exact length of the literal word (LG2.17), whatever word steps carry it, so a chunk
            // cannot raise the total (LB2.4).
            Request::Propose { word, .. } => {
                Ok(RowTariff { kernel_cost: word.len() as u128, result_bound: RESULT_BOUND })
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::POSITIONS => {
                kernel_rows::positions_read_tariff(arg, ty)
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::LABELS => {
                kernel_rows::labels_read_tariff(arg, ty)
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::EFFECT => {
                kernel_rows::effect_read_tariff(self.layer.kernel(), arg, ty)
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::PROTECTION => {
                kernel_rows::protection_read_tariff(self.layer.kernel(), arg, ty)
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        if let Request::SnapshotWith { query, arg, ty } = request {
            if *query == kernel_rows::POSITIONS {
                return kernel_rows::positions(self.layer, arg, ty);
            }
            if *query == kernel_rows::LABELS {
                return kernel_rows::labels(self.layer, arg, ty);
            }
            if *query == kernel_rows::EFFECT {
                return kernel_rows::effect(self.layer, arg, ty);
            }
            if *query == kernel_rows::PROTECTION {
                return kernel_rows::protection(self.layer, arg, ty);
            }
        }
        let Request::Propose { word, target, .. } = request else {
            return Err(NOT_GRANTED);
        };
        let base = base(self.layer.revision());
        let args = proposal_args(word, self.steps.as_ref());
        let receipt = self.invoke(target, args, None);
        match &receipt.status {
            Status::Refused(error) | Status::Failed(error) => Err(error.code),
            _ => Ok(answer(&receipt, &base)),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}
