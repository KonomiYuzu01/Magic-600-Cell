//! The host bridge from `c600-run` onto `c600-command`: the runtime's [`Host`] for one run of a tool in the solving
//! profile, used through [`c600_run::boundary::Boundary`] (LP3, C5).
//!
//! A proposal that passes preview admission becomes one invocation of the row its target names, made as the runtime:
//! [`Caller::Runtime`] for the run, a fresh command ID, the layer's current epoch, the literal word as an inline draft,
//! `{"draft": {"recipe": [{"kind": "word", "moves": [...]}]}}`, and no token. The bridge filters no row: the command
//! layer refuses a runtime invocation of a commit or app row, and any runtime invocation that carries a token
//! (ADR-L02, KN5.15). The bridge answers the boundary with a short form of the layer's receipt, which only the shell
//! sees; the program gets the boundary's fixed acknowledgement (BND.42, BND.43). The bridge grants no other request:
//! no state read, no workspace or file write and no clock.

use c600_command::{Caller, CommandLayer, Invocation, MemoryStore, Receipt, Revision, Status, Store, Token};
use c600_kernel::json::{self, Value as Json};
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::value::Value;

/// The result bound of a proposal's answer (a draft; the command table prices the row when it freezes).
pub const RESULT_BOUND: u128 = 4096;

/// The revision a run reads, as the acknowledgement's `base`: `<session>@<epoch>/<head>`.
pub fn base(revision: &Revision) -> String {
    format!("{}@{}/{}", revision.session, revision.epoch, revision.head)
}

/// The absolute arguments of a proposal of a literal word: `{"draft": {"recipe": [{"kind": "word", "moves": [...]}]}}`.
pub fn word_args(word: &[i32]) -> Json {
    let moves = Json::Array(word.iter().map(|&letter| Json::Int(i64::from(letter))).collect());
    let step = json::object([("kind", Json::String("word".into())), ("moves", moves)]);
    json::object([("draft", json::object([("recipe", Json::Array(vec![step]))]))])
}

/// One invocation the bridge sent, with the layer's receipt.
#[derive(Clone, Debug)]
pub struct Sent {
    pub invocation: Invocation,
    pub receipt: Receipt,
}

/// The runtime's host for one run on one session's command layer.
pub struct Bridge<'l, S: Store = MemoryStore> {
    layer: &'l mut CommandLayer<S>,
    run: String,
    /// Every invocation the bridge sent, in order.
    pub sent: Vec<Sent>,
}

impl<'l, S: Store> Bridge<'l, S> {
    /// The host of run `run` on `layer`. A run's name is new in its session: the bridge's command IDs are
    /// `<run>/<n>`, and the layer answers a command ID it knows by its outcome lookup.
    pub fn new(layer: &'l mut CommandLayer<S>, run: &str) -> Self {
        Bridge { layer, run: run.to_owned(), sent: Vec::new() }
    }

    /// One invocation of `row` with absolute arguments, as the runtime: [`Caller::Runtime`] for this run, a fresh
    /// command ID and the layer's current epoch. A proposal never carries a token; `token` is there so a test can show
    /// that the layer refuses a runtime invocation carrying one.
    pub fn invoke(&mut self, row: &str, args: Json, token: Option<Token>) -> Receipt {
        let invocation = Invocation {
            command: format!("{}/{}", self.run, self.sent.len() + 1),
            row: row.to_owned(),
            args,
            token,
            caller: Caller::Runtime { run: self.run.clone() },
            epoch: self.layer.revision().epoch,
        };
        let receipt = self.layer.invoke(invocation.clone());
        self.sent.push(Sent { invocation, receipt: receipt.clone() });
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
            // KN5.9's cost: the exact length of the literal word (LG2.17).
            Request::Propose { word, .. } => {
                Ok(RowTariff { kernel_cost: word.len() as u128, result_bound: RESULT_BOUND })
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let Request::Propose { word, target, .. } = request else {
            return Err(NOT_GRANTED);
        };
        let base = base(self.layer.revision());
        let receipt = self.invoke(target, word_args(word), None);
        match &receipt.status {
            Status::Refused(error) | Status::Failed(error) => Err(error.code),
            _ => Ok(answer(&receipt, &base)),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}
