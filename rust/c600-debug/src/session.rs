//! Stepping and inspection (EV2.1, EV2.2, EV2.12; LP4-5).
//!
//! A [`Debugger`] holds debugging sessions by run name. Each names a [`Target`]: the checked module and its digest, the
//! executed sources by digest, the IR version and execution contract, and the pinned input it inspects (EV2.1). Its run
//! executes on a [`Stepper`] at the run's own LG4/DET.20 positions (`c600-run`'s gate); the debugger defines no step
//! counter or
//! tariff of its own.
//!
//! - [`Debugger::debug_step`] is `debug_step(run, expected_stop, request_id) -> StepResult` (EV2.12). One advance is in
//!   flight per run; a duplicate request ID returns its latest result and advances nothing, so a suspended kernel call is
//!   neither dispatched nor charged again; a request ID is run-local, and one used with another run, or with another
//!   expected stop, is `EV2:stale.debug_stop`, as is a stop that is no longer current.
//! - A host row the run performs is a suspended kernel call: the step answers `waiting(call)` at the call's awaiting
//!   stop with the call site's source and no return value; once the call completes, the request that dispatched it, and
//!   any that joined it, answer `stopped` at the continuation with the call's result, or the run's end.
//! - [`StepResult`]'s `values` hold only the completed immutable locals of the stop's frame, by reference; `source` is
//!   the executed module's range for the stop and its call sites, innermost first, or an explicit
//!   `unavailable(source_map_missing | source_bytes_missing)`; `state` is `stopped`, `waiting(call)`,
//!   `terminal(outcome)` or `unavailable(step_mapping_missing)`. A target whose executor has no step mapping, such as the
//!   runtime's own process, gets `unavailable(step_mapping_missing)` before anything runs.
//! - Terminal reasons stay distinct: completed, a deterministic cut-off with its budget's code, a runtime failure with
//!   its code, a cancellation, and a wall-clock safety stop (`KN7:resource_limit.safety_stop`), which leaves the run
//!   incomplete. Each carries the meter's real use and coverage; none is an empty negative.
//! - [`Debugger::inspect_value`] is `inspect_value(run, value_reference)` (EV2.2, [`crate::inspect`]).
//!
//! # Drafts
//!
//! - **Value references**: `<stop>#<index>` for a stop's local, `<stop>#result` for a completed call's result at the
//!   continuation, and `end#value` for a completed run's returned value. A reference stays readable while its session
//!   lives.
//! - **Call IDs**: the position of the host row that dispatched the call, in decimal.
//! - **The executor's result** is [`Finished`]: the interpreter's run, or a refusal before it ran.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Instant;

use c600_run::interp::{Host, Loaded, Options, Run, SourceRanges};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::{self, Amounts, DraftTariffs, Interruption, Outcome, SourceRange, Tariffs};
use c600_run::step::{Boundary, Slot};
use c600_run::trace::Relation;
use c600_run::value::Value;

use crate::code::{REPLAY_DEPENDENCY_MISSING, SAFETY_STOP, STALE_DEBUG_STOP};
use crate::host::ReadOnly;
use crate::inspect::{inspect, Inspection, Origin};
use crate::profile::{self, CostProfile};
use crate::sources::selects;
use crate::stepper::{Completed, Line, Stale, Stepped, Stepper};
use crate::{DebugError, Pinned};

/// How a target's run executes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Executor {
    /// RT1's evaluator in this process, which exposes its LG4/DET.20 boundaries (EV2.12).
    Evaluator,
    /// An executor with no executor-to-contract step mapping, by name, such as the runtime's own process: stepping
    /// is not admitted on it.
    Unmapped(String),
}

/// What a debugging session names (EV2.1).
#[derive(Clone, Debug)]
pub struct Target {
    /// The entry's module and function.
    pub module: String,
    pub function: String,
    /// The executed version's digest: its closure digest, or its module's source digest (lowercase hex).
    pub module_digest: String,
    /// The executed modules' bytes, by SHA-256 (lowercase hex).
    pub sources: BTreeMap<String, Arc<[u8]>>,
    pub ir_version: String,
    pub execution_contract: String,
    /// The pinned input the session inspects.
    pub input: Pinned,
    pub executor: Executor,
}

/// What a stepped run's executor returns: the interpreter's run, or a refusal before it ran.
#[derive(Debug)]
pub enum Finished {
    Ran(Run),
    Refused { code: &'static str, detail: String },
}

/// The interpreter's options of a stepped run, owned so the run's thread can hold them.
#[derive(Clone)]
pub struct RunOptions {
    pub limits: Amounts,
    pub tariffs: Tariffs,
    pub draft: DraftTariffs,
    /// The generator's seed and stream path (DET.61).
    pub seed: Option<(Vec<u8>, Vec<String>)>,
    /// Whether the run keeps one row per operation.
    pub record: bool,
    pub sources: Option<Arc<dyn SourceRanges + Send + Sync>>,
    pub menu: Option<Arc<MenuAlphabet>>,
    pub relation: Option<Arc<Relation>>,
}

impl RunOptions {
    /// Test tariffs, rows kept, no seed, no source map, no menu alphabet and no trace relation.
    pub fn new(limits: Amounts) -> RunOptions {
        RunOptions {
            limits,
            tariffs: Tariffs::TEST,
            draft: DraftTariffs::TEST,
            seed: None,
            record: true,
            sources: None,
            menu: None,
            relation: None,
        }
    }

    /// Calls `f` with these options as the interpreter takes them; a stepped run has no cancellation flag, since its
    /// gate interrupts it.
    pub fn with<T>(&self, f: impl FnOnce(&Options) -> T) -> T {
        let path: Vec<&str> = self.seed.iter().flat_map(|(_, p)| p.iter().map(String::as_str)).collect();
        let sources = self.sources.as_deref().map(|s| -> &dyn SourceRanges { s });
        let options = Options {
            limits: self.limits,
            tariffs: self.tariffs,
            draft: self.draft,
            seed: self.seed.as_ref().map(|(seed, _)| (&seed[..], &path[..])),
            record: self.record,
            cancel: None,
            sources,
            menu: self.menu.as_deref(),
            relation: self.relation.as_deref(),
        };
        f(&options)
    }
}

/// The executor of `module.function` of `loaded` on RT1's evaluator, with `args`, on `host` behind [`ReadOnly`]: what
/// [`Debugger::open`] runs for a program it steps in this process.
pub fn evaluate(
    loaded: Arc<Loaded>,
    module: String,
    function: String,
    args: Vec<Value>,
    mut host: Box<dyn Host + Send>,
    options: RunOptions,
) -> impl FnOnce(&mut Line<Finished>) -> Finished + Send + 'static {
    move |line| {
        let mut read_only = ReadOnly::new(&mut *host);
        let mut dispatching = line.dispatching(&mut read_only);
        options.with(|o| match loaded.run_gated(&module, &function, args, &mut dispatching, o, line) {
            Ok(run) => Finished::Ran(run),
            Err(refusal) => Finished::Refused { code: refusal.code, detail: refusal.detail },
        })
    }
}

/// A stop's position: the run's DET.20 position and the stop's locator (DET.56).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub step: u128,
    pub at: String,
}

/// A completed immutable local of a stop, by reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueRef {
    pub name: String,
    pub slot: Slot,
    pub reference: String,
}

/// Why a stop has no source range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMissing {
    /// The run has no source map, or the map has no range for the stop's operation.
    SourceMap,
    /// The session does not hold the bytes a range names, or they do not hold that range (EV1.10).
    SourceBytes,
}

/// A stop's source (EV2.12): the executed module's ranges, the stop's first and then its call sites', or why there is
/// none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Mapped(Vec<SourceRange>),
    Unavailable(SourceMissing),
}

/// Why a run ended (KN7.3, KN7.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Completed,
    CutOff,
    Error,
    Cancelled,
    SafetyStop,
}

/// DET.30's coverage of an ended run: only a completed run covers its declared set; a deterministic cut-off keeps what
/// it covered; any other end is open and establishes no absence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    Complete,
    WithinBudget,
    Open,
}

impl Coverage {
    /// The coverage of an ended run of outcome `outcome`.
    pub fn of(outcome: Outcome) -> Coverage {
        match outcome {
            Outcome::Completed => Coverage::Complete,
            Outcome::CutOff => Coverage::WithinBudget,
            Outcome::Error | Outcome::Cancelled => Coverage::Open,
        }
    }
}

/// An ended run: its reason, code and stop position, the meter's real use and its coverage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terminal {
    pub reason: Reason,
    pub code: Option<&'static str>,
    pub stop: Option<meter::Position>,
    pub used: Amounts,
    pub coverage: Coverage,
}

/// A suspended call's answer at its continuation: the call and its result's reference, or the host's refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallResult {
    pub call: String,
    pub result: Result<String, &'static str>,
}

/// Why stepping is not available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    StepMappingMissing,
}

/// Where a step left the run (EV2.12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Stopped { completed: Option<CallResult> },
    Waiting { call: String },
    Terminal(Terminal),
    Unavailable(Unavailable),
}

/// EV2.12's `StepResult`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepResult {
    pub run: String,
    pub module_digest: String,
    pub execution_contract: String,
    pub position: Position,
    pub stop_id: String,
    pub values: Vec<ValueRef>,
    pub source: Source,
    pub state: State,
}

/// One debugging session.
struct Session {
    name: String,
    target: Target,
    stepper: Option<Stepper<Finished>>,
    /// A wall-clock safety stop the host declared, with the use the run had reached.
    safety: Option<Terminal>,
    /// The stops a result named, for their value references.
    stops: BTreeMap<String, Arc<Boundary>>,
    /// Completed calls' results and the run's returned value, by reference.
    results: BTreeMap<String, Value>,
    /// The run's limits, read at its first stop.
    limits: Amounts,
    started: Instant,
}

impl Session {
    fn base(&self) -> StepResult {
        StepResult {
            run: self.name.clone(),
            module_digest: self.target.module_digest.clone(),
            execution_contract: self.target.execution_contract.clone(),
            position: Position { step: 0, at: "entry".into() },
            stop_id: String::new(),
            values: Vec::new(),
            source: Source::Unavailable(SourceMissing::SourceMap),
            state: State::Unavailable(Unavailable::StepMappingMissing),
        }
    }

    fn unmapped(&self) -> StepResult {
        StepResult { stop_id: "unmapped".into(), ..self.base() }
    }

    /// The source of a stop: its range and its call sites', innermost first (EV2.12, EV1.10).
    fn source(&self, stop: &Boundary) -> Source {
        let Some(range) = &stop.source else { return Source::Unavailable(SourceMissing::SourceMap) };
        let mut ranges = vec![range.clone()];
        ranges.extend(stop.sites.iter().rev().filter_map(|site| site.source.clone()));
        let held = |r: &SourceRange| self.target.sources.get(&r.digest).is_some_and(|bytes| selects(bytes, r));
        if ranges.iter().all(held) {
            Source::Mapped(ranges)
        } else {
            Source::Unavailable(SourceMissing::SourceBytes)
        }
    }

    fn values(id: &str, stop: &Boundary) -> Vec<ValueRef> {
        let reference = |i: usize| format!("{id}#{i}");
        stop.locals
            .iter()
            .enumerate()
            .map(|(i, l)| ValueRef { name: l.name.clone(), slot: l.slot, reference: reference(i) })
            .collect()
    }

    fn call_result(&mut self, stop: &str, completed: &Completed) -> CallResult {
        let result = match &completed.result {
            Ok(value) => {
                let reference = format!("{stop}#result");
                self.results.insert(reference.clone(), value.clone());
                Ok(reference)
            }
            Err(code) => Err(*code),
        };
        CallResult { call: completed.call.to_string(), result }
    }

    fn at_stop(&mut self, stop: &Arc<Boundary>, id: String) -> StepResult {
        if self.stops.is_empty() {
            self.limits = stop.limits;
        }
        self.stops.insert(id.clone(), Arc::clone(stop));
        StepResult {
            position: Position { step: stop.step, at: stop.at.clone() },
            values: Session::values(&id, stop),
            source: self.source(stop),
            stop_id: id,
            ..self.base()
        }
    }

    fn result(&mut self, stepped: &Stepped) -> StepResult {
        if let Some(terminal) = self.safety.clone() {
            return self.ended(terminal, None);
        }
        match stepped {
            Stepped::Stopped { stop, completed } => {
                let id = stop.id();
                let completed = completed.as_ref().map(|c| self.call_result(&id, c));
                StepResult { state: State::Stopped { completed }, ..self.at_stop(stop, id) }
            }
            Stepped::Waiting { call, site } => {
                let mut result = self.at_stop(site, stepped.id());
                result.position.step = *call;
                StepResult { state: State::Waiting { call: call.to_string() }, ..result }
            }
            Stepped::Ended { completed } => {
                let completed = completed.clone();
                let terminal = self.terminal();
                self.ended(terminal, completed)
            }
        }
    }

    /// The run's terminal record, from its executor's result.
    fn terminal(&self) -> Terminal {
        match self.stepper.as_ref().and_then(Stepper::outcome) {
            Some(Finished::Ran(run)) => {
                let ended = &run.summary;
                let reason = match ended.outcome {
                    Outcome::Completed => Reason::Completed,
                    Outcome::CutOff => Reason::CutOff,
                    Outcome::Error => Reason::Error,
                    Outcome::Cancelled => Reason::Cancelled,
                };
                let coverage = Coverage::of(ended.outcome);
                Terminal { reason, code: ended.code, stop: ended.stop.clone(), used: ended.used, coverage }
            }
            Some(Finished::Refused { code, .. }) => Terminal {
                reason: Reason::Error,
                code: Some(code),
                stop: None,
                used: Amounts::default(),
                coverage: Coverage::Open,
            },
            None => Terminal {
                reason: Reason::Cancelled,
                code: None,
                stop: None,
                used: Amounts::default(),
                coverage: Coverage::Open,
            },
        }
    }

    fn ended(&mut self, terminal: Terminal, completed: Option<Arc<Completed>>) -> StepResult {
        let id = "end".to_owned();
        let source = match terminal.stop.as_ref() {
            None => Source::Mapped(Vec::new()),
            Some(position) => match &position.source {
                None => Source::Unavailable(SourceMissing::SourceMap),
                Some(range) => match self.target.sources.get(&range.digest) {
                    Some(bytes) if selects(bytes, range) => Source::Mapped(vec![range.clone()]),
                    _ => Source::Unavailable(SourceMissing::SourceBytes),
                },
            },
        };
        let mut values = Vec::new();
        if let Some(Finished::Ran(Run { value: Some(value), .. })) = self.stepper.as_ref().and_then(Stepper::outcome) {
            if terminal.reason == Reason::Completed {
                self.results.insert("end#value".into(), value.clone());
                values.push(ValueRef { name: "result".into(), slot: Slot::Let, reference: "end#value".into() });
            }
        }
        if let Some(c) = completed {
            let _ = self.call_result(&id, &c);
        }
        let at = terminal.stop.as_ref().map_or_else(|| "end".to_owned(), |p| p.at.clone());
        StepResult {
            position: Position { step: terminal.used.steps, at },
            stop_id: id,
            values,
            source,
            state: State::Terminal(terminal),
            ..self.base()
        }
    }

    fn current(&mut self) -> StepResult {
        match self.stepper.as_mut() {
            None => self.unmapped(),
            Some(stepper) => {
                let stepped = stepper.poll().clone();
                self.result(&stepped)
            }
        }
    }
}

fn stale(why: Stale) -> DebugError {
    let detail = match why {
        Stale::Obsolete => "the request names a stop that is no longer the run's current stop (EV2.12)",
        Stale::Reused => "the request ID was used before with another run or expected stop (EV2.12)",
    };
    DebugError::new(STALE_DEBUG_STOP, detail)
}

fn missing(what: impl Into<String>) -> DebugError {
    DebugError::new(REPLAY_DEPENDENCY_MISSING, what)
}

/// The debugging sessions of one host, by run name (see the module's description).
#[derive(Default)]
pub struct Debugger {
    sessions: BTreeMap<String, Session>,
    /// Each request ID and the run it was used with: an ID is run-local (EV2.12).
    requests: HashMap<String, String>,
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger::default()
    }

    /// Opens the session `run` on `target`. A target whose executor has a step mapping starts `execute` on its own
    /// thread, stopped at its first boundary, and the result is that stop; any other target gets
    /// `unavailable(step_mapping_missing)` and nothing runs. A run name already open is refused.
    pub fn open<F>(&mut self, run: &str, target: Target, execute: F) -> Result<StepResult, DebugError>
    where
        F: FnOnce(&mut Line<Finished>) -> Finished + Send + 'static,
    {
        if self.sessions.contains_key(run) {
            return Err(stale(Stale::Reused));
        }
        let stepper = match target.executor {
            Executor::Evaluator => Some(Stepper::spawn(execute)),
            Executor::Unmapped(_) => None,
        };
        let session = Session {
            name: run.to_owned(),
            target,
            stepper,
            safety: None,
            stops: BTreeMap::new(),
            results: BTreeMap::new(),
            limits: Amounts::default(),
            started: Instant::now(),
        };
        let session = self.sessions.entry(run.to_owned()).or_insert(session);
        Ok(session.current())
    }

    fn session(&mut self, run: &str) -> Result<&mut Session, DebugError> {
        self.sessions.get_mut(run).ok_or_else(|| missing(format!("no debugging session `{run}` is open (EV2.1)")))
    }

    /// `debug_step(run, expected_stop, request_id) -> StepResult` (EV2.12; see the module's description).
    pub fn debug_step(&mut self, run: &str, expected_stop: &str, request_id: &str) -> Result<StepResult, DebugError> {
        let known = match self.requests.get(request_id) {
            Some(owner) if owner != run => return Err(stale(Stale::Reused)),
            Some(_) => true,
            None => false,
        };
        let session = self.session(run)?;
        let result = match session.stepper.as_mut() {
            None => session.unmapped(),
            Some(_) if session.safety.is_some() => {
                if !known && expected_stop != "end" {
                    return Err(stale(Stale::Obsolete));
                }
                session.current()
            }
            Some(stepper) => {
                let stepped = stepper.step(expected_stop, request_id).map_err(stale)?;
                session.result(&stepped)
            }
        };
        self.requests.insert(request_id.to_owned(), run.to_owned());
        Ok(result)
    }

    /// The run's current stop, after taking a completed call's answer if it arrived; nothing advances.
    pub fn current(&mut self, run: &str) -> Result<StepResult, DebugError> {
        Ok(self.session(run)?.current())
    }

    /// Waits until the run's call in flight completes, without advancing past its continuation.
    pub fn wait(&mut self, run: &str) -> Result<StepResult, DebugError> {
        let session = self.session(run)?;
        if session.safety.is_none() {
            if let Some(stepper) = session.stepper.as_mut() {
                stepper.wait();
            }
        }
        Ok(session.current())
    }

    /// Cancels the run (KN7.6): at its current stop, or at its next boundary after a call in flight.
    pub fn cancel(&mut self, run: &str, why: Interruption) -> Result<StepResult, DebugError> {
        let session = self.session(run)?;
        if let Some(stepper) = session.stepper.as_mut() {
            stepper.interrupt(why);
        }
        Ok(session.current())
    }

    /// The host's wall-clock safety stop of the run (KN7.10): the session ends at once as incomplete, with
    /// `KN7:resource_limit.safety_stop` and the use the run had reached; the run is interrupted at its next boundary and
    /// its later end is not reported.
    pub fn safety_stop(&mut self, run: &str) -> Result<StepResult, DebugError> {
        let session = self.session(run)?;
        if session.safety.is_none() {
            let used = match session.stepper.as_ref().map(Stepper::state) {
                Some(Stepped::Stopped { stop, .. }) | Some(Stepped::Waiting { site: stop, .. }) => stop.used,
                _ => session.terminal().used,
            };
            if let Some(stepper) = session.stepper.as_mut() {
                if let Stepped::Ended { .. } = stepper.state() {
                    return Ok(session.current());
                }
                stepper.interrupt(Interruption::ByUser);
            }
            let terminal = Terminal {
                reason: Reason::SafetyStop,
                code: Some(SAFETY_STOP),
                stop: None,
                used,
                coverage: Coverage::Open,
            };
            session.safety = Some(terminal);
        }
        Ok(session.current())
    }

    /// `inspect_value(run, value_reference)` (EV2.2): a local of a stop a result named, a completed call's result, or
    /// a completed run's value. An unknown reference is `EV2:input.replay_dependency_missing`.
    pub fn inspect_value(&self, run: &str, value_reference: &str) -> Result<Inspection, DebugError> {
        let session =
            self.sessions.get(run).ok_or_else(|| missing(format!("no debugging session `{run}` is open (EV2.1)")))?;
        let unknown = || missing(format!("`{value_reference}` names no value of run `{run}` (EV2.2)"));
        let (stop, part) = value_reference.rsplit_once('#').ok_or_else(unknown)?;
        let input = &session.target.input;
        if let Some(value) = session.results.get(value_reference) {
            let boundary = session.stops.get(stop);
            let origin = Origin {
                stop: stop.to_owned(),
                name: part.to_owned(),
                slot: None,
                module: boundary.and_then(|b| b.operation.as_ref()).map(|o| o.module.clone()),
                function: boundary.and_then(|b| b.operation.as_ref()).map(|o| o.function.clone()),
                source: boundary.and_then(|b| b.source.clone()),
            };
            return Ok(inspect(value, origin, input));
        }
        let boundary = session.stops.get(stop).ok_or_else(unknown)?;
        let local = part.parse::<usize>().ok().and_then(|i| boundary.locals.get(i)).ok_or_else(unknown)?;
        let origin = Origin {
            stop: stop.to_owned(),
            name: local.name.clone(),
            slot: Some(local.slot),
            module: boundary.operation.as_ref().map(|o| o.module.clone()),
            function: boundary.operation.as_ref().map(|o| o.function.clone()),
            source: boundary.source.clone(),
        };
        Ok(inspect(&local.value, origin, input))
    }

    /// The run's cost profile so far, or at its end (EV2.7), with the elapsed time since the session opened as display
    /// data.
    pub fn profile(&mut self, run: &str) -> Result<CostProfile, DebugError> {
        let session = self.session(run)?;
        let elapsed = Some(session.started.elapsed());
        let budget = session.name.clone();
        let current = session.current();
        let mut profile = match (&current.state, session.stepper.as_ref().and_then(Stepper::outcome)) {
            (State::Terminal(t), _) if t.reason == Reason::SafetyStop => {
                let mut p = profile::so_far(&budget, session.limits, t.used);
                p.stop = Some((SAFETY_STOP, "safety_stop"));
                p
            }
            (State::Terminal(_), Some(Finished::Ran(ran))) => {
                profile::of_summary(&budget, session.limits, &ran.summary, Vec::new())
            }
            _ => {
                let used = session.stops.get(&current.stop_id).map_or_else(Amounts::default, |b| b.used);
                profile::so_far(&budget, session.limits, used)
            }
        };
        profile.elapsed = elapsed;
        Ok(profile)
    }

    /// The session's target.
    pub fn target(&self, run: &str) -> Option<&Target> {
        self.sessions.get(run).map(|s| &s.target)
    }

    /// Closes the session: a run still going is interrupted at its next boundary. Its request IDs stay used.
    pub fn close(&mut self, run: &str) {
        self.sessions.remove(run);
    }

    /// The names of the open sessions.
    pub fn runs(&self) -> BTreeSet<String> {
        self.sessions.keys().cloned().collect()
    }
}
