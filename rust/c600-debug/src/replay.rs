//! Deterministic replay (EV2.3, EV2.4, EV2.9; LP4-6): `replay_run(record) -> { outcome, comparison, profile,
//! evidence_references }`.
//!
//! [`Replayer::replay_run`] first preflights the record's [`ReproKey`]: every module by its source digest, the edition,
//! the IR version and execution contract, the identity block, the input's level, the cost units, the seed and the
//! budgets, before anything is evaluated (EV2.3). A module is found by its digest only, so a file of the same name with
//! other bytes is never a replacement: a missing digest is `EV2:input.replay_dependency_missing` naming it, and another
//! model, menu identity, menu order or contract revision is `KN1:input.identity_mismatch` with the expected and the
//! available identity and the first field that differs. A refused preflight evaluates nothing, and the record stays as
//! it was, inspectable as recorded evidence without a replay claim.
//!
//! The replay then re-evaluates the record's entry on RT1 under the key's budgets and cost units. Its host is the
//! record's own trace ([`Event`]): every read is answered from the inputs the record captured, every tariff is the
//! recorded one, and every preview and sink is compared with the recorded effect at the same trace position and never
//! executed (EV2.9, owner, 23 November 2026). No live host exists on this path, so an effect the record does not hold
//! is `EV2:capability.debug_write_forbidden` and nothing is written; the replay grants no ambient file or clock.
//!
//! The comparison covers the trace position by position, then the outcome, code and stop, the coverage, the meter's
//! use, child scopes and rows, the result's canonical bytes and the output bytes (EV2.4). The first difference is
//! `EV2:evidence.replay_mismatch` with the differing field and its position where one exists. A record that ended in a
//! wall-clock safety stop is not run: it stays incomplete and not reproducible, with no negative claim (KN7.10).
//! [`Kind`] says whether the replay was a complete re-execution of a run with no effects, or a re-evaluation whose
//! effects were inspected in the record.
//!
//! # Drafts
//!
//! Chapter 05's `RunRecord` and chapter 09's `ReproKey` and comparison rules are not written as forms yet;
//! [`RunRecord`] and [`ReproKey`] are the debugger's typed reading of them, and the record producer is the session's
//! (not this crate's). EV2.9's contract is settled in chapter 09's DET.7 as amended under PHS.52, and what a record
//! holds for it in overlay OV-1 (`docs/progress/1.0/language-overlays.md`). Elapsed time and receipts a new session
//! would issue are not compared. The module bytes the key names were checked when the record was made; the replay
//! relowers them and the loader revalidates the IR, without a second checker run.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use c600_digest::sha256_hex;
use c600_ir::ExactRole;
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Options, Request, RowTariff, Run, Sink};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::{Amounts, ChildRecord, DraftTariffs, Outcome, Position, Row, Tariffs};
use c600_run::value::{equal, Value};
use c600_syntax::{lower, parse, EDITION};

use crate::code::{DEBUG_WRITE_FORBIDDEN, IDENTITY_MISMATCH, REPLAY_DEPENDENCY_MISSING, REPLAY_MISMATCH, SAFETY_STOP};
use crate::profile::{self, Alphabet, CostProfile, WordCost};
use crate::session::Coverage;
use crate::sources::Executed;
use crate::{DebugError, Identity, Pinned};

/// An executed module of a record: its name and its source bytes' SHA-256 (lowercase hex).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleRef {
    pub name: String,
    pub digest: String,
}

/// The record's reproducibility key (chapter 09's `ReproKey`, as the debugger reads it).
#[derive(Clone, Debug)]
pub struct ReproKey {
    /// Every executed module, in lowering order.
    pub sources: Vec<ModuleRef>,
    pub edition: String,
    pub ir_version: String,
    pub execution_contract: String,
    /// The pinned input the run read: its identity block, level and digest.
    pub input: Pinned,
    /// The entry and its arguments.
    pub module: String,
    pub function: String,
    pub args: Vec<Value>,
    /// The generator's seed and stream path (DET.61).
    pub seed: Option<(Vec<u8>, Vec<String>)>,
    /// The deterministic budgets.
    pub limits: Amounts,
    /// The name of the cost units the run was charged in.
    pub cost_units: String,
}

/// A host request as a record holds it. A kernel row of the identity tower (T1 draft) is held by its row, its arguments
/// as the run held them (a host's handles by the ids the recorded host gave them, which name them to that host only)
/// and its declared result type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestKey {
    Snapshot { query: String, ty: String },
    SnapshotWith { query: String, arg: Vec<u8>, ty: String },
    Declassified { id: String, ty: String },
    FileRead { path: String, ty: String },
    Clock,
    Propose { word: Vec<i32>, target: String, request: Vec<u8> },
    ProposeMenu { letters: Vec<[u32; 2]>, menu: String, target: String, request: Vec<u8> },
    Kernel { row: String, args: String, ty: String },
}

impl RequestKey {
    pub fn of(request: &Request) -> RequestKey {
        match *request {
            Request::Snapshot { query, ty } => RequestKey::Snapshot { query: query.into(), ty: format!("{ty:?}") },
            Request::SnapshotWith { query, arg, ty } => {
                RequestKey::SnapshotWith { query: query.into(), arg: arg.to_vec(), ty: format!("{ty:?}") }
            }
            Request::Declassified { id, ty } => RequestKey::Declassified { id: id.into(), ty: format!("{ty:?}") },
            Request::FileRead { path, ty } => RequestKey::FileRead { path: path.into(), ty: format!("{ty:?}") },
            Request::Clock => RequestKey::Clock,
            Request::Propose { word, target, request } => {
                RequestKey::Propose { word: word.to_vec(), target: target.into(), request: request.to_vec() }
            }
            Request::ProposeMenu { letters, menu, target, request } => RequestKey::ProposeMenu {
                letters: letters.to_vec(),
                menu: format!("{menu:?}"),
                target: target.into(),
                request: request.to_vec(),
            },
            Request::Kernel { op, args, ty } => {
                RequestKey::Kernel { row: format!("{op:?}"), args: format!("{args:?}"), ty: format!("{ty:?}") }
            }
        }
    }

    /// Whether the request is a preview, an effect the replay inspects.
    pub fn is_proposal(&self) -> bool {
        matches!(self, RequestKey::Propose { .. } | RequestKey::ProposeMenu { .. })
    }
}

/// A sink as a record holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SinkKey {
    View,
    Workspace { record: String, ty: String },
    File { path: String },
    Exact { role: ExactRole },
}

impl SinkKey {
    pub fn of(sink: &Sink) -> SinkKey {
        match *sink {
            Sink::View => SinkKey::View,
            Sink::Workspace { record, ty } => SinkKey::Workspace { record: record.into(), ty: format!("{ty:?}") },
            Sink::File { path } => SinkKey::File { path: path.into() },
            Sink::Exact { role } => SinkKey::Exact { role },
        }
    }
}

/// One step of a record's host trace, in the order the run made them.
#[derive(Clone, Debug)]
pub enum Event {
    /// A row's declared tariff, or the host's refusal.
    Tariff { request: RequestKey, answer: Result<RowTariff, &'static str> },
    /// A performed row's answer: a captured input, or a preview's receipt.
    Perform { request: RequestKey, answer: Result<Value, &'static str> },
    /// A delivered value's bytes and the sink's answer.
    Sink { sink: SinkKey, bytes: Vec<u8>, answer: Result<(), &'static str> },
}

impl Event {
    fn describe(&self) -> String {
        match self {
            Event::Tariff { request, .. } => format!("tariff of {request:?}"),
            Event::Perform { request, .. } => format!("row {request:?}"),
            Event::Sink { sink, bytes, .. } => format!("sink {sink:?} of {} bytes", bytes.len()),
        }
    }
}

/// The recorded result of a run.
#[derive(Clone, Debug)]
pub struct Recorded {
    pub outcome: Outcome,
    pub code: Option<&'static str>,
    pub stop: Option<Position>,
    pub coverage: Coverage,
    pub used: Amounts,
    pub children: Vec<ChildRecord>,
    pub rows: Vec<Row>,
    pub value: Option<Value>,
    pub output: Option<Vec<u8>>,
}

impl Recorded {
    /// The record of an ended run.
    pub fn of(run: &Run) -> Recorded {
        let ended = &run.summary;
        Recorded {
            outcome: ended.outcome,
            code: ended.code,
            stop: ended.stop.clone(),
            coverage: Coverage::of(ended.outcome),
            used: ended.used,
            children: ended.children.clone(),
            rows: ended.rows.clone(),
            value: run.value.clone(),
            output: run.output.clone(),
        }
    }

    /// The record of a run its host ended by a wall-clock safety stop, at the use it had reached (KN7.10).
    pub fn safety_stopped(used: Amounts) -> Recorded {
        Recorded {
            outcome: Outcome::Cancelled,
            code: Some(SAFETY_STOP),
            stop: None,
            coverage: Coverage::Open,
            used,
            children: Vec::new(),
            rows: Vec::new(),
            value: None,
            output: None,
        }
    }

    /// Whether the record ended in a wall-clock safety stop.
    pub fn is_safety_stop(&self) -> bool {
        self.code == Some(SAFETY_STOP)
    }
}

/// A run record (chapter 05's `RunRecord`, as the debugger reads it): its name, key, host trace and result.
#[derive(Clone, Debug)]
pub struct RunRecord {
    pub name: String,
    pub key: ReproKey,
    pub events: Vec<Event>,
    pub result: Recorded,
}

/// The first difference between a replay and its record (EV2.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// `EV2:evidence.replay_mismatch`.
    pub code: &'static str,
    /// The differing field: `trace`, `effects`, `outcome`, `code`, `stop`, `coverage`, `used`, `children`, `rows`,
    /// `result` or `output`.
    pub field: &'static str,
    /// The trace event or row where they differ, where one exists.
    pub position: Option<String>,
    pub expected: String,
    pub found: String,
}

/// A replay's comparison with its record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Comparison {
    Match,
    Mismatch(Mismatch),
    /// The record is incomplete and not reproducible, by its code: no claim either way.
    Incomplete {
        code: &'static str,
    },
}

/// What a replay did (EV2.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A complete deterministic re-execution: the run had no effects, and its reads were the record's captured inputs.
    Reexecution,
    /// A re-evaluation whose effects, `inspected` of them, were compared with the record and never executed.
    EffectInspection { inspected: usize },
    /// The record was not run: it is incomplete.
    NotRun,
}

/// EV2.3's result: `{ outcome, comparison, profile, evidence_references }`, and what kind of replay it was.
#[derive(Clone, Debug)]
pub struct Replay {
    /// The replayed outcome, or `None` for a record that was not run.
    pub outcome: Option<Outcome>,
    pub comparison: Comparison,
    pub profile: CostProfile,
    /// The digests the replay resolved: `module:<digest>` for each module and `input:<level>:<digest>`.
    pub evidence_references: Vec<String>,
    pub kind: Kind,
}

impl Replay {
    /// The comparison as a verdict: a match, `EV2:evidence.replay_mismatch`, or the incomplete record's own code.
    pub fn verdict(&self) -> Result<(), DebugError> {
        match &self.comparison {
            Comparison::Match => Ok(()),
            Comparison::Mismatch(m) => Err(DebugError::new(
                m.code,
                format!(
                    "the replay differs from the record in {} at {}: expected {}, found {} (EV2.4)",
                    m.field,
                    m.position.as_deref().unwrap_or("the result"),
                    m.expected,
                    m.found
                ),
            )),
            Comparison::Incomplete { code } => Err(DebugError::new(
                code,
                "the record is incomplete and not reproducible; the replay makes no claim either way (KN7.10)",
            )),
        }
    }
}

/// The record's trace as the replay's host (see the module's description).
struct Replaying<'r> {
    events: &'r [Event],
    next: usize,
    divergence: Option<Mismatch>,
    forbidden: Option<String>,
    inspected: usize,
    words: Vec<WordCost>,
}

impl<'r> Replaying<'r> {
    fn new(events: &'r [Event]) -> Replaying<'r> {
        Replaying { events, next: 0, divergence: None, forbidden: None, inspected: 0, words: Vec::new() }
    }

    fn diverge(&mut self, field: &'static str, expected: String, found: String) -> &'static str {
        if self.divergence.is_none() {
            let position = Some(format!("event {}", self.next));
            self.divergence = Some(Mismatch { code: REPLAY_MISMATCH, field, position, expected, found });
        }
        REPLAY_MISMATCH
    }

    /// Ends the run at an effect the record does not hold: a live write would be needed.
    fn forbid(&mut self, found: String) -> &'static str {
        if self.forbidden.is_none() {
            self.forbidden = Some(found);
        }
        DEBUG_WRITE_FORBIDDEN
    }

    fn halted(&self) -> bool {
        self.divergence.is_some() || self.forbidden.is_some()
    }

    /// The next event, or the code that ends the run when the trace has ended or diverged.
    fn next_event(&mut self, effect: bool, found: impl FnOnce() -> String) -> Result<&'r Event, &'static str> {
        if self.halted() {
            return Err(self.divergence.as_ref().map_or(DEBUG_WRITE_FORBIDDEN, |m| m.code));
        }
        match self.events.get(self.next) {
            Some(event) => Ok(event),
            None if effect => Err(self.forbid(found())),
            None => Err(self.diverge("trace", "the end of the trace".into(), found())),
        }
    }
}

impl Host for Replaying<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        let key = RequestKey::of(request);
        let event = self.next_event(key.is_proposal(), || format!("tariff of {key:?}"))?;
        match event {
            Event::Tariff { request: recorded, answer } if *recorded == key => {
                self.next += 1;
                *answer
            }
            other => {
                let field = if key.is_proposal() { "effects" } else { "trace" };
                Err(self.diverge(field, other.describe(), format!("tariff of {key:?}")))
            }
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let key = RequestKey::of(request);
        let event = self.next_event(key.is_proposal(), || format!("row {key:?}"))?;
        match event {
            Event::Perform { request: recorded, answer } if *recorded == key => {
                self.next += 1;
                match &key {
                    RequestKey::Propose { word, .. } => {
                        self.inspected += 1;
                        let letters = word.len() as u128;
                        self.words.push(WordCost { alphabet: Alphabet::RetainedGenerators, letters });
                    }
                    RequestKey::ProposeMenu { letters, .. } => {
                        self.inspected += 1;
                        let letters = letters.len() as u128;
                        self.words.push(WordCost { alphabet: Alphabet::MenuLetters, letters });
                    }
                    _ => {}
                }
                answer.clone()
            }
            other => {
                let field = if key.is_proposal() { "effects" } else { "trace" };
                Err(self.diverge(field, other.describe(), format!("row {key:?}")))
            }
        }
    }

    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        let key = SinkKey::of(sink);
        let event = self.next_event(true, || format!("sink {key:?} of {} bytes", bytes.len()))?;
        match event {
            Event::Sink { sink: recorded, bytes: held, answer } if *recorded == key && held[..] == *bytes => {
                self.next += 1;
                self.inspected += 1;
                *answer
            }
            other => Err(self.diverge("effects", other.describe(), format!("sink {key:?} of {} bytes", bytes.len()))),
        }
    }
}

/// A replayer: the module bytes it holds by digest, the identity block available to it, the editions and cost units it
/// supports, and the menu alphabet for a jumbling identity.
pub struct Replayer {
    modules: BTreeMap<String, Arc<[u8]>>,
    pub identity: Identity,
    pub editions: BTreeSet<String>,
    pub cost_units: BTreeMap<String, (Tariffs, DraftTariffs)>,
    pub menu: Option<Arc<MenuAlphabet>>,
}

/// The name of the test cost units, the contract vectors' unmeasured values.
pub const TEST_COST_UNITS: &str = "test";

fn missing(detail: String) -> DebugError {
    DebugError::new(REPLAY_DEPENDENCY_MISSING, detail)
}

fn bytes_of(value: &Value) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    encode(value, &mut out).ok().map(|()| out)
}

fn same_value(a: &Option<Value>, b: &Option<Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => match (bytes_of(a), bytes_of(b)) {
            (Some(x), Some(y)) => x == y,
            (None, None) => equal(a, b),
            _ => false,
        },
        _ => false,
    }
}

fn shown(value: &Option<Value>) -> String {
    match value {
        None => "no value".into(),
        Some(v) => match bytes_of(v) {
            Some(bytes) => format!("{} result bytes, SHA-256 {}", bytes.len(), sha256_hex(&bytes)),
            None => format!("{v:?}"),
        },
    }
}

impl Replayer {
    /// A replayer for `identity`, holding no module, reading this parser's edition, with the test cost units.
    pub fn new(identity: Identity) -> Replayer {
        Replayer {
            modules: BTreeMap::new(),
            identity,
            editions: BTreeSet::from([EDITION.to_owned()]),
            cost_units: BTreeMap::from([(TEST_COST_UNITS.to_owned(), (Tariffs::TEST, DraftTariffs::TEST))]),
            menu: None,
        }
    }

    /// Holds module bytes under their SHA-256, which it returns.
    pub fn hold(&mut self, bytes: &[u8]) -> String {
        let digest = sha256_hex(bytes);
        self.modules.insert(digest.clone(), Arc::from(bytes));
        digest
    }

    /// EV2.3's preflight: every module's bytes by digest, and the cost units, or the first blocker.
    fn preflight(&self, key: &ReproKey) -> Result<(Vec<Arc<[u8]>>, Tariffs, DraftTariffs), DebugError> {
        let mut files = Vec::new();
        for module in &key.sources {
            let bytes = self.modules.get(&module.digest).ok_or_else(|| {
                missing(format!(
                    "module {} with source digest {} is not held; no other file replaces it (EV2.3)",
                    module.name, module.digest
                ))
            })?;
            files.push(Arc::clone(bytes));
        }
        if !self.editions.contains(&key.edition) {
            return Err(missing(format!("edition {} is not supported here (EV2.3)", key.edition)));
        }
        if key.ir_version != c600_ir::IR_VERSION || key.execution_contract != c600_ir::EXEC_CONTRACT {
            return Err(missing(format!(
                "IR version {} under execution contract {} is not supported here (EV2.3)",
                key.ir_version, key.execution_contract
            )));
        }
        if let Some(field) = key.input.identity.differs(&self.identity) {
            return Err(DebugError::new(
                IDENTITY_MISMATCH,
                format!(
                    "the record's identity differs in {field}: expected {:?}, available {:?} (EV2.3, KN1.4)",
                    key.input.identity, self.identity
                ),
            ));
        }
        if key.input.identity.menu.is_some() && self.menu.is_none() {
            return Err(missing("the record's menu alphabet is not held (EV2.3)".into()));
        }
        if !matches!(key.input.level.as_str(), "labels" | "poses") || key.input.digest.is_empty() {
            return Err(missing(format!(
                "the record's input {}:{} names no level-qualified digest (EV2.3)",
                key.input.level, key.input.digest
            )));
        }
        let (tariffs, draft) = *self
            .cost_units
            .get(&key.cost_units)
            .ok_or_else(|| missing(format!("cost units {} are not held (EV2.3)", key.cost_units)))?;
        Ok((files, tariffs, draft))
    }

    /// `replay_run(record)` (EV2.3, EV2.4, EV2.9; see the module's description). A preflight blocker is the error; a
    /// difference from the record is the result's [`Comparison::Mismatch`].
    pub fn replay_run(&self, record: &RunRecord) -> Result<Replay, DebugError> {
        let key = &record.key;
        let mut evidence: Vec<String> = key.sources.iter().map(|m| format!("module:{}", m.digest)).collect();
        evidence.push(format!("input:{}:{}", key.input.level, key.input.digest));
        if record.result.is_safety_stop() {
            let mut profile = profile::so_far(&record.name, key.limits, record.result.used);
            profile.stop = Some((SAFETY_STOP, "safety_stop"));
            return Ok(Replay {
                outcome: None,
                comparison: Comparison::Incomplete { code: SAFETY_STOP },
                profile,
                evidence_references: evidence,
                kind: Kind::NotRun,
            });
        }
        let (files, tariffs, draft) = self.preflight(key)?;
        let parsed: Vec<_> = files.iter().map(|b| parse(b)).collect();
        if let Some((m, _)) = key.sources.iter().zip(&parsed).find(|(_, f)| !f.diagnostics.is_empty()) {
            return Err(missing(format!("module {} ({}) does not parse under its edition (EV2.3)", m.name, m.digest)));
        }
        let lowered = lower(&parsed).map_err(|e| missing(format!("the record's modules do not lower: {e} (EV2.3)")))?;
        let loaded = load(&lowered.program).map_err(|r| DebugError::new(r.code, r.detail))?;
        let held: Vec<&[u8]> = files.iter().map(|b| &b[..]).collect();
        let executed = Executed::new(&lowered, &held);
        let path: Vec<&str> = key.seed.iter().flat_map(|(_, p)| p.iter().map(String::as_str)).collect();
        let options = Options {
            limits: key.limits,
            tariffs,
            draft,
            seed: key.seed.as_ref().map(|(seed, _)| (&seed[..], &path[..])),
            record: true,
            cancel: None,
            sources: Some(&executed),
            menu: self.menu.as_deref(),
            relation: None,
        };
        let mut host = Replaying::new(&record.events);
        let run = loaded
            .run(&key.module, &key.function, key.args.clone(), &mut host, &options)
            .map_err(|r| DebugError::new(r.code, r.detail))?;
        if let Some(found) = host.forbidden.take() {
            return Err(DebugError::new(
                DEBUG_WRITE_FORBIDDEN,
                format!("the record holds no effect for {found}; replay never writes to a live sink (EV2.9)"),
            ));
        }
        let comparison = match host.divergence.take() {
            Some(m) => Comparison::Mismatch(m),
            None if host.next < record.events.len() => Comparison::Mismatch(Mismatch {
                code: REPLAY_MISMATCH,
                field: "trace",
                position: Some(format!("event {}", host.next)),
                expected: record.events[host.next].describe(),
                found: "the end of the run".into(),
            }),
            None => compare(&record.result, &run),
        };
        let kind =
            if host.inspected == 0 { Kind::Reexecution } else { Kind::EffectInspection { inspected: host.inspected } };
        let words = std::mem::take(&mut host.words);
        let ended = &run.summary;
        Ok(Replay {
            outcome: Some(ended.outcome),
            comparison,
            profile: profile::of_summary(&record.name, key.limits, ended, words),
            evidence_references: evidence,
            kind,
        })
    }
}

/// The first difference between `recorded` and `run`'s result, in EV2.4's order.
fn compare(recorded: &Recorded, run: &Run) -> Comparison {
    let ended = &run.summary;
    let differ = |field: &'static str, position: Option<String>, expected: String, found: String| {
        Comparison::Mismatch(Mismatch { code: REPLAY_MISMATCH, field, position, expected, found })
    };
    if recorded.outcome != ended.outcome {
        return differ("outcome", None, recorded.outcome.name().into(), ended.outcome.name().into());
    }
    if recorded.code != ended.code {
        return differ("code", None, format!("{:?}", recorded.code), format!("{:?}", ended.code));
    }
    if recorded.stop != ended.stop {
        return differ("stop", None, format!("{:?}", recorded.stop), format!("{:?}", ended.stop));
    }
    let coverage = Coverage::of(ended.outcome);
    if recorded.coverage != coverage {
        return differ("coverage", None, format!("{:?}", recorded.coverage), format!("{coverage:?}"));
    }
    if recorded.used != ended.used {
        return differ("used", None, format!("{:?}", recorded.used), format!("{:?}", ended.used));
    }
    if recorded.children != ended.children {
        let i = recorded.children.iter().zip(&ended.children).take_while(|(a, b)| a == b).count();
        return differ(
            "children",
            Some(format!("child {i}")),
            format!("{:?}", recorded.children.get(i)),
            format!("{:?}", ended.children.get(i)),
        );
    }
    if !recorded.rows.is_empty() && recorded.rows != ended.rows {
        let i = recorded.rows.iter().zip(&ended.rows).take_while(|(a, b)| a == b).count();
        let at = recorded.rows.get(i).or(ended.rows.get(i)).map_or("", |r| r.at.as_str());
        return differ(
            "rows",
            Some(format!("row {i} at {at}")),
            format!("{:?}", recorded.rows.get(i)),
            format!("{:?}", ended.rows.get(i)),
        );
    }
    if !same_value(&recorded.value, &run.value) {
        return differ("result", None, shown(&recorded.value), shown(&run.value));
    }
    if recorded.output != run.output {
        let len = |o: &Option<Vec<u8>>| o.as_ref().map_or("no output".into(), |b| format!("{} output bytes", b.len()));
        return differ("output", None, len(&recorded.output), len(&run.output));
    }
    Comparison::Match
}
