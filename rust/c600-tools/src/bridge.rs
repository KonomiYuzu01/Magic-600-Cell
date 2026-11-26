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
//! five reads, each to a snapshot read with an argument ([`crate::kernel_rows`]; KN5, LP2 draft): KN2.4's
//! `positions` and `labels` rows at the layer's head, which LB2's exact completion reads (CT-LB2-04); KN2.4's
//! `effect` row, a recipe's net map, which reads no state and which LB2's recipes and reports read (CT-LB2-01);
//! KN2.4's `protection` row, a captured lock judged against a recipe at the layer's head, which LB2's Net and Strict
//! protection reads (CT-LB2-02); and KN2.4's `replay` row for 600-cell-Full, a recipe replayed prefix by prefix at the
//! layer's head under the session's protected orbits, which LB2's named macro analysis reads. When the host gives it
//! the certified guarded setup tables ([`Bridge::with_setups`]), it serves a sixth read: the `setups` row, the
//! certified setups for a target the solver names, which only the setup-search exception may read and which LB2's
//! guarded setups read (CT-BND-18; BND.1). It answers setups in the
//! table's order, ranks none and runs none: applying one stays the solver's action. It also serves the three
//! structure reads of LP3's representations tool (C6): `structure.compare`, two cells compared, `filter.preview`, the
//! counts of a layer predicate, and `structure.layer`, its members, through LP2's [`CommandLayer::compare_cells`],
//! [`CommandLayer::layer_counts`] and [`CommandLayer::layer_members`], which read no state. And it serves the
//! `orbits` read, the pieces of named orbits, which LB2's orbit catalogue reads: model data, the orbit the kernel's
//! `positions` gives each piece, which reads no state of the session. It serves the `journal` read too, KN4.3's
//! page of the session's move tree, newest first, which LP3's C8 reads, through LP2's [`CommandLayer::journal`], which
//! changes nothing.
//! Capability separation keeps an entry that reads them from also proposing: admission refuses an entry that does both
//! (LQ21, LG3.13, BND.39). The bridge grants no other request: no other state read, no read without an argument, no
//! workspace or file write and no clock.
//!
//! The draft carries the word in one word step, with one exception (LB2.3, LB2.4, LB1.11). For one run a host may give
//! the bridge the solver's input recipe, exactly as written, with its literal word ([`Steps`]), built only from a
//! recipe the kernel expanded. A proposal of exactly that literal word that is longer than one chunk ([`WORD_CHUNK`])
//! goes in the one invocation as the solver's recipe, unchanged, whatever its steps: word steps, star steps, and word
//! steps with other fields ([`proposal_args`]). The bridge never cuts, splits or rewrites a word or a recipe: every
//! other word goes in one word step, and the layer refuses it whole when it is longer than one chunk.
//!
//! The bridge also serves the identity tower's kernel rows on 600-cell-Full ([`Request::Kernel`]; T2 draft), over the
//! kernel and the command layer's value store: [`tower`] says which rows, at what tariff, with what handles.

pub mod tower;

use std::sync::Arc;

use c600_command::values::Recipe;
use c600_command::ValueStore;

use c600_command::{
    draft_args, Budget, Caller, CommandLayer, Invocation, MemoryStore, Receipt, Revision, Status, Store, Token,
};
use c600_kernel::json::{self, Value as Json};
use c600_kernel::Kernel;
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::value::Value;

use crate::expand::{expand, Expansion};
use crate::kernel_rows::{self, SetupTable};

/// The result bound of a proposal's answer (a draft; H-08 prices only reads, CM.18).
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
    let moves = Json::Array(word.iter().map(|&letter| Json::Int(i64::from(letter))).collect());
    let step = json::object([("kind", Json::String("word".into())), ("moves", moves)]);
    draft_args(&Json::Array(vec![step]))
}

/// The solver's input recipe of one run, exactly as written, and its literal word: the kernel's expansion of that
/// recipe. A host gives it to the bridge for the run ([`Bridge::with_steps`]); it is built only from a recipe the
/// kernel expanded, so the literal word is never a word, a length or a cut a host chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Steps {
    literal: Vec<i32>,
    recipe: Json,
}

impl Steps {
    /// The solver's `recipe` with its literal word, when the kernel expands it ([`Expansion::Literal`]): any steps
    /// the kernel admits, star steps and word steps with other fields included. None when the kernel refuses it.
    pub fn of(kernel: &Kernel, recipe: &Json) -> Option<Steps> {
        match expand(kernel, recipe) {
            Expansion::Literal(literal) => Some(Steps { literal, recipe: recipe.clone() }),
            Expansion::Unchecked { .. } | Expansion::Refused(_) => None,
        }
    }

    /// The literal word: the kernel's expansion of the solver's recipe.
    pub fn literal(&self) -> &[i32] {
        &self.literal
    }

    /// The solver's recipe, exactly as written.
    pub fn recipe(&self) -> &Json {
        &self.recipe
    }
}

/// The absolute arguments of a proposal of `word`, given the solver's input recipe of the run, if the host gave it
/// (LB2.3, LB2.4, LB1.11). A word longer than one chunk that is exactly the literal word of `steps`, letter for
/// letter, goes as the solver's recipe, unchanged: `{"draft": {"recipe": <the solver's recipe>}}`, whatever its
/// steps. Every other word goes in one word step ([`word_args`]): a word of at most one chunk, whatever the recipe,
/// and a word that is not the solver's literal, which the layer refuses whole when it is longer than one chunk. The
/// bridge never cuts, splits or rewrites a word or a recipe.
pub fn proposal_args(word: &[i32], steps: Option<&Steps>) -> Json {
    match steps.filter(|steps| word.len() > WORD_CHUNK && word == steps.literal.as_slice()) {
        Some(steps) => draft_args(&steps.recipe),
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
    /// The budget the run's invocations of a row an entry serves declare: the run's own, if the host gave it, and
    /// otherwise every profile's ceiling, `Budget::CEILING`.
    budget: Budget,
    /// The solver's input recipe of the run and its literal word, if the host gave them.
    steps: Option<Steps>,
    /// The certified guarded setup tables, if the host gave them: the `setups` row reads them.
    setups: Option<Arc<SetupTable>>,
    /// Every invocation the bridge sent, in order.
    pub sent: Vec<Sent>,
}

impl<'l, S: Store> Bridge<'l, S> {
    /// The host of run `run` on `layer`. A run's name is new in its session: the bridge's command IDs are
    /// `<run>/<n>`, and the layer answers a command ID it knows by its outcome lookup. The bridge has no solver's
    /// recipe: every proposal goes in one word step. It has no setup tables: a `setups` read is not granted. It has
    /// no run's budget: an invocation of a row an entry serves declares every profile's ceiling.
    pub fn new(layer: &'l mut CommandLayer<S>, run: &str) -> Self {
        let budget = Budget::CEILING;
        Bridge { layer, run: run.to_owned(), budget, steps: None, setups: None, sent: Vec::new() }
    }

    /// This bridge with the run's budget, the budget of the program job the run is (`ProgramJob::budget`), which the
    /// host gives for the run: the run's invocations of a row an entry serves then declare it (KN7.7).
    pub fn with_budget(mut self, budget: Budget) -> Self {
        self.budget = budget;
        self
    }

    /// This bridge with the solver's input recipe of the run and its literal word ([`Steps::of`]), which the host gives
    /// for the run; a proposal's arguments are then [`proposal_args`] of the word and the recipe. None keeps one word
    /// step.
    pub fn with_steps(mut self, steps: Option<Steps>) -> Self {
        self.steps = steps;
        self
    }

    /// This bridge with the certified guarded setup tables ([`SetupTable::load`]), which the host gives for the run;
    /// the bridge then serves the `setups` row (CT-BND-18). None keeps the read not granted.
    pub fn with_setups(mut self, table: Option<Arc<SetupTable>>) -> Self {
        self.setups = table;
        self
    }

    /// The layer's value store (T2 draft; [`tower`]): what the run's kernel rows made in this epoch.
    pub fn values(&mut self) -> &mut ValueStore {
        self.layer.values_mut()
    }

    /// The digest of the value that a stored derivation `recipe` makes, recomputed in this epoch ([`tower::recompute`];
    /// T2 draft): its base resolves while it is the layer's head, whatever epoch the derivation was stored in (ONE.17,
    /// ONE.21).
    pub fn recompute(&mut self, recipe: &Recipe) -> Result<String, &'static str> {
        tower::recompute(self, recipe).map(|(_, digest)| digest)
    }

    /// One invocation of `row` with absolute arguments on this run's channel, stamped [`Caller::Runtime`] for the run,
    /// with a fresh command ID and the layer's current epoch. A proposal never carries a token; `token` is there so a
    /// test can show that the layer refuses a runtime invocation carrying one. An invocation of a row an entry serves
    /// declares the run's budget (KN7.7; [`Bridge::with_budget`], by default `Budget::CEILING`), and one of another row
    /// declares none, as the shell does.
    pub fn invoke(&mut self, row: &str, args: Json, token: Option<Token>) -> Receipt {
        let served = self.layer.table().row(row).is_some_and(|found| found.overlay.is_some());
        let invocation = Invocation {
            command: format!("{}/{}", self.run, self.sent.len() + 1),
            row: row.to_owned(),
            args,
            token,
            epoch: self.layer.revision().epoch,
            budget: served.then_some(self.budget),
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
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::REPLAY => {
                kernel_rows::replay_read_tariff(self.layer.kernel(), arg, ty)
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::SETUPS => match &self.setups {
                Some(table) => kernel_rows::setups_read_tariff(table, arg, ty),
                None => Err(NOT_GRANTED),
            },
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::STRUCTURE_COMPARE => {
                kernel_rows::structure_compare_read_tariff(arg, ty)
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::FILTER_PREVIEW => {
                kernel_rows::filter_preview_read_tariff(arg, ty)
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::STRUCTURE_LAYER => {
                kernel_rows::structure_layer_read_tariff(arg, ty)
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::ORBITS_ROW => {
                kernel_rows::orbits_read_tariff(arg, ty)
            }
            Request::SnapshotWith { query, arg, ty } if *query == kernel_rows::JOURNAL => {
                kernel_rows::journal_read_tariff(arg, ty)
            }
            Request::Kernel { op, args, .. } => tower::tariff(self, *op, args),
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        if let Request::Kernel { op, args, .. } = request {
            return tower::perform(self, *op, args);
        }
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
            if *query == kernel_rows::REPLAY {
                return kernel_rows::replay(self.layer, arg, ty);
            }
            if *query == kernel_rows::SETUPS {
                return match &self.setups {
                    Some(table) => kernel_rows::setups(self.layer, table, arg, ty),
                    None => Err(NOT_GRANTED),
                };
            }
            if *query == kernel_rows::STRUCTURE_COMPARE {
                return kernel_rows::structure_compare(self.layer, arg, ty);
            }
            if *query == kernel_rows::FILTER_PREVIEW {
                return kernel_rows::filter_preview(self.layer, arg, ty);
            }
            if *query == kernel_rows::STRUCTURE_LAYER {
                return kernel_rows::structure_layer(self.layer, arg, ty);
            }
            if *query == kernel_rows::ORBITS_ROW {
                return kernel_rows::orbits(self.layer, arg, ty);
            }
            if *query == kernel_rows::JOURNAL {
                return kernel_rows::journal(self.layer, arg, ty);
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
