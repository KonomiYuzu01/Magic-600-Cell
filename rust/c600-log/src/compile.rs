//! Compiling a log (ADR-D01.81, revision 6's R-2): the format and the model identity (gate `model_match`), every move
//! replayed with its legal witness, the final full-state hash, the embedded macros' witnesses and the embedded scripts,
//! compiled by the editor services.
//!
//! Compile reads bytes and returns a result; it holds no live session and changes none, and it never runs a script:
//! scripts go to `c600_editor::diagnose`, which parses, checks and lowers and runs nothing, and this crate has no path
//! to `c600-run` (`tests/dependencies.rs`). The saved state is bound only when the format, `model_match`, `moves` and
//! `final_hash` gates all passed; otherwise the log is read-only with its problems and no state, and nothing is drawn
//! from a partial replay. The result names the SHA-256 of the bytes it compiled, so any edit makes it stale.

use std::collections::{HashMap, HashSet};

use c600_editor::{diagnose, Document, Envelope, ModuleContext, Units, Verdict};
use c600_kernel::{Derivation, Error, Kernel, ProtectedSet, Snapshot, Step};

use crate::log::{self, Author, Commit, Log, MoveAction, Start};
use crate::problem::{code, Gate, Problem};

/// What a gate did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateResult {
    Passed,
    Failed,
    /// An earlier gate failed, so this one could not run.
    NotRun,
}

/// The editor's answer for one embedded script.
#[derive(Clone, Debug)]
pub struct ScriptCheck {
    pub name: String,
    pub author: Author,
    /// `None` when the script's text differs from its recorded source digest, so it was not diagnosed.
    pub verdict: Option<Verdict>,
    pub diagnostics: Vec<Envelope>,
}

/// The result of compiling one log's bytes.
#[derive(Clone, Debug)]
pub struct Compiled {
    bytes_sha256: String,
    gates: Vec<(Gate, GateResult)>,
    problems: Vec<Problem>,
    log: Option<Log>,
    state: Option<Snapshot>,
    scripts: Vec<ScriptCheck>,
}

impl Compiled {
    /// The SHA-256 of the bytes this result is for.
    pub fn bytes_sha256(&self) -> &str {
        &self.bytes_sha256
    }

    /// Whether this result is for exactly these bytes (revision 6, R-2): any edit makes it stale.
    pub fn binds(&self, bytes: &[u8]) -> bool {
        c600_digest::sha256_hex(bytes) == self.bytes_sha256
    }

    /// Every gate in order, with what it did.
    pub fn gates(&self) -> &[(Gate, GateResult)] {
        &self.gates
    }

    pub fn gate(&self, gate: Gate) -> GateResult {
        self.gates.iter().find(|(g, _)| *g == gate).map_or(GateResult::NotRun, |(_, r)| *r)
    }

    /// The problems, for the Problems tab, in gate order.
    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }

    /// The log as read, when the format gate passed. Reading it grants nothing: it is the recorded data.
    pub fn log(&self) -> Option<&Log> {
        self.log.as_ref()
    }

    /// The saved state, only when the format, `model_match`, `moves` and `final_hash` gates passed.
    pub fn state(&self) -> Option<&Snapshot> {
        self.state.as_ref()
    }

    /// Whether the log opens read-only with its diagnostics and no binding (revision 6, R-2).
    pub fn read_only(&self) -> bool {
        self.state.is_none()
    }

    /// Whether every gate passed.
    pub fn passed(&self) -> bool {
        self.gates.iter().all(|(_, r)| *r == GateResult::Passed)
    }

    pub fn scripts(&self) -> &[ScriptCheck] {
        &self.scripts
    }
}

/// Compiles a log's stored bytes against the loaded model.
pub fn compile(kernel: &Kernel, bytes: &[u8]) -> Compiled {
    let mut compiled = Compiled {
        bytes_sha256: c600_digest::sha256_hex(bytes),
        gates: Vec::new(),
        problems: Vec::new(),
        log: None,
        state: None,
        scripts: Vec::new(),
    };
    let log = match log::read(bytes) {
        Ok(log) => log,
        Err(problem) => {
            compiled.problems.push(problem);
            compiled.gates = Gate::COMPILE
                .into_iter()
                .map(|g| (g, if g == Gate::Format { GateResult::Failed } else { GateResult::NotRun }))
                .collect();
            return compiled;
        }
    };
    compiled.gates.push((Gate::Format, GateResult::Passed));

    let model = kernel
        .check_identity(&log.identity)
        .map_err(|refusal| Problem::new(refusal.code, Gate::ModelMatch, "identity", refusal.message));
    let model_ok = record(&mut compiled, Gate::ModelMatch, model.map(|_| ()));

    let head = if model_ok {
        let replayed = replay(kernel, &log);
        let ok = record(&mut compiled, Gate::Moves, replayed.as_ref().map(|_| ()).map_err(Clone::clone));
        ok.then(|| replayed.ok()).flatten()
    } else {
        compiled.gates.push((Gate::Moves, GateResult::NotRun));
        None
    };
    match head {
        Some(state) => {
            let result = if state.digest() == log.saved.state {
                Ok(())
            } else {
                Err(Problem::new(
                    code::CHECKPOINT_REPLAY,
                    Gate::FinalHash,
                    "payload.log.saved.state",
                    "the replayed state at the saved cursor differs from the saved full-state hash",
                ))
            };
            if record(&mut compiled, Gate::FinalHash, result) {
                compiled.state = Some(state);
            }
        }
        None => compiled.gates.push((Gate::FinalHash, GateResult::NotRun)),
    }

    if model_ok {
        let problems = macros(kernel, &log);
        record_all(&mut compiled, Gate::Macros, problems);
    } else {
        compiled.gates.push((Gate::Macros, GateResult::NotRun));
    }

    let (checks, problems) = scripts(&log);
    compiled.scripts = checks;
    record_all(&mut compiled, Gate::Scripts, problems);
    compiled.log = Some(log);
    compiled
}

fn record(compiled: &mut Compiled, gate: Gate, result: Result<(), Problem>) -> bool {
    match result {
        Ok(()) => {
            compiled.gates.push((gate, GateResult::Passed));
            true
        }
        Err(problem) => {
            compiled.problems.push(problem);
            compiled.gates.push((gate, GateResult::Failed));
            false
        }
    }
}

fn record_all(compiled: &mut Compiled, gate: Gate, problems: Vec<Problem>) {
    let result = if problems.is_empty() { GateResult::Passed } else { GateResult::Failed };
    compiled.problems.extend(problems);
    compiled.gates.push((gate, result));
}

/// The start state of a log.
pub(crate) fn start_state(kernel: &Kernel, log: &Log) -> Result<Snapshot, Problem> {
    match &log.start {
        Start::Solved => Ok(kernel.solved()),
        Start::Labels(labels) => kernel.state_from_labels(labels.clone()).ok_or_else(|| {
            Problem::new(code::CHECKPOINT_REPLAY, Gate::Moves, "payload.log.start", "the start labels are not a state")
        }),
    }
}

fn moves_problem(code: &'static str, at: String, message: impl Into<String>) -> Problem {
    Problem::new(code, Gate::Moves, at, message)
}

fn kernel_error(error: Error, at: String) -> Problem {
    match error {
        Error::Refusal(refusal) => moves_problem(refusal.code, at, refusal.message),
        Error::Certificate(error) => moves_problem(code::CHECKPOINT_REPLAY, at, error.0),
    }
}

/// The derived state of a derivation; a refusal names its code.
fn derived(derivation: Derivation, at: String) -> Result<Snapshot, Problem> {
    match derivation {
        Derivation::Derived(derived) => Ok(derived.state),
        Derivation::Refused(refused) => Err(moves_problem(refused.refusal.code, at, refused.refusal.message)),
    }
}

/// Checks a commit's witness: the recipe normalizes under 0.4's checks and its counts are the recorded ones.
pub(crate) fn witness(kernel: &Kernel, commit: &Commit, at: &str) -> Result<c600_kernel::Recipe, Problem> {
    let recipe = kernel
        .recipe(&commit.recipe)
        .map_err(|refusal| moves_problem(refusal.code, format!("{at}.recipe"), refusal.message))?;
    let stars = recipe.steps().iter().filter(|step| matches!(step, Step::Star(_))).count() as u64;
    if recipe.primitive_count() as u64 != commit.primitive_count || stars != commit.stars {
        return Err(moves_problem(
            code::CHECKPOINT_REPLAY,
            format!("{at}.primitive_count"),
            "the recorded counts differ from the legal witness",
        ));
    }
    Ok(recipe)
}

/// Replays every node of the move history from the start with its legal witness and returns the state at the saved
/// cursor. A state is kept only while a later node or the saved cursor still needs it.
pub(crate) fn replay(kernel: &Kernel, log: &Log) -> Result<Snapshot, Problem> {
    let none = ProtectedSet::none();
    let commits: HashMap<&str, (usize, &Commit)> = log
        .moves
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match &e.action {
            MoveAction::Commit(c) => Some((e.id.as_str(), (i, c))),
            _ => None,
        })
        .collect();
    // The last entry that reads each state; the saved cursor reads its state after the last entry.
    let mut last_use: HashMap<Option<&str>, usize> = HashMap::new();
    for (i, entry) in log.moves.iter().enumerate() {
        if let Some(parent) = entry.action.parent() {
            last_use.insert(parent, i);
        }
    }
    last_use.insert(log.saved.moves.as_deref(), usize::MAX);
    let mut states: HashMap<Option<&str>, Snapshot> = HashMap::new();
    states.insert(None, start_state(kernel, log)?);

    for (i, entry) in log.moves.iter().enumerate() {
        let at = format!("payload.log.history.moves[{i}]");
        let Some(parent) = entry.action.parent() else { continue };
        let base = states.get(&parent).cloned().ok_or_else(|| {
            moves_problem(code::CHECKPOINT_REPLAY, at.clone(), "the node's parent state is not known")
        })?;
        let state = match &entry.action {
            MoveAction::Commit(commit) => {
                if base.digest() != commit.pre {
                    return Err(moves_problem(
                        code::CHECKPOINT_REPLAY,
                        format!("{at}.pre"),
                        "the pre state differs from the replayed state",
                    ));
                }
                let recipe = witness(kernel, commit, &at)?;
                let derivation = kernel.derive(&base, &recipe, &none).map_err(|e| kernel_error(e, at.clone()))?;
                let state = derived(derivation, at.clone())?;
                if state.digest() != commit.post {
                    return Err(moves_problem(
                        code::CHECKPOINT_REPLAY,
                        format!("{at}.post"),
                        "the post state differs from the replay of the legal witness",
                    ));
                }
                state
            }
            MoveAction::PartialUndo(undo) => {
                let Some(&(_, commit)) = commits.get(undo.entry.as_str()) else {
                    return Err(moves_problem(code::CHECKPOINT_REPLAY, format!("{at}.entry"), "no commit has this id"));
                };
                let recipe = witness(kernel, commit, &at)?;
                let letters = kernel.letters(&recipe).map_err(|r| moves_problem(r.code, at.clone(), r.message))?;
                let keep = usize::try_from(undo.keep).unwrap_or(usize::MAX).min(letters.len());
                let mut replay =
                    kernel.replay(&base, &none).map_err(|r| moves_problem(r.code, at.clone(), r.message))?;
                for chunk in letters[..keep].chunks(65_536) {
                    let chunk: Vec<i64> = chunk.iter().map(|&m| i64::from(m)).collect();
                    replay.push(&chunk).map_err(|r| moves_problem(r.code, at.clone(), r.message))?;
                }
                let state = derived(replay.finish(), at.clone())?;
                if state.digest() != undo.post {
                    return Err(moves_problem(
                        code::CHECKPOINT_REPLAY,
                        format!("{at}.post"),
                        "the post state differs from the replay of the kept letters",
                    ));
                }
                state
            }
            MoveAction::Workspace(_) => base,
            MoveAction::Cursor(_) => unreachable!("a cursor action has no parent"),
        };
        if last_use.get(&parent) == Some(&i) {
            states.remove(&parent);
        }
        let id = Some(entry.id.as_str());
        if last_use.contains_key(&id) {
            states.insert(id, state);
        }
    }
    states.remove(&log.saved.moves.as_deref()).ok_or_else(|| {
        Problem::new(code::CHECKPOINT_REPLAY, Gate::Moves, "payload.log.saved.moves", "the saved cursor names no state")
    })
}

/// Every embedded macro's witness: it normalizes, its count is the expansion's, and its expansion digest is the
/// signed int32-LE letters' (FP.8).
fn macros(kernel: &Kernel, log: &Log) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (i, m) in log.macros.iter().enumerate() {
        let at = format!("payload.log.macros[{i}]");
        let recipe = match kernel.recipe(&m.steps) {
            Ok(recipe) => recipe,
            Err(refusal) => {
                problems.push(Problem::new(refusal.code, Gate::Macros, format!("{at}.steps"), refusal.message));
                continue;
            }
        };
        let letters = match kernel.letters(&recipe) {
            Ok(letters) => letters,
            Err(refusal) => {
                problems.push(Problem::new(refusal.code, Gate::Macros, format!("{at}.steps"), refusal.message));
                continue;
            }
        };
        let bytes: Vec<u8> = letters.iter().flat_map(|m| m.to_le_bytes()).collect();
        if letters.len() as u64 != m.primitive_count || c600_digest::sha256_hex(&bytes) != m.expansion {
            problems.push(Problem::new(
                code::DIGEST_MISMATCH,
                Gate::Macros,
                format!("{at}.expansion"),
                "the macro's count or expansion digest differs from its witness",
            ));
        }
    }
    problems
}

/// Every embedded script, diagnosed by the editor services and never run. The scripts of the log are each other's
/// digest-pinned dependencies.
fn scripts(log: &Log) -> (Vec<ScriptCheck>, Vec<Problem>) {
    let mut context = ModuleContext::new();
    let mut added = HashSet::new();
    for script in &log.scripts {
        if added.insert(script.source.as_str()) && c600_digest::sha256_hex(script.text.as_bytes()) == script.source {
            context.add_dependency(script.text.as_bytes(), None);
        }
    }
    let mut checks = Vec::new();
    let mut problems = Vec::new();
    for (i, script) in log.scripts.iter().enumerate() {
        let at = format!("payload.log.scripts[{i}]");
        let mut check =
            ScriptCheck { name: script.name.clone(), author: script.author, verdict: None, diagnostics: Vec::new() };
        if c600_digest::sha256_hex(script.text.as_bytes()) != script.source {
            problems.push(Problem::new(
                code::DIGEST_MISMATCH,
                Gate::Scripts,
                format!("{at}.source"),
                "the script's text differs from its source digest",
            ));
            checks.push(check);
            continue;
        }
        let document = Document::open(script.text.as_bytes(), Units::Utf8);
        match diagnose(&document, document.digest(), &context) {
            Ok(diagnosis) => {
                for envelope in &diagnosis.diagnostics {
                    problems.push(Problem::new(
                        envelope.code,
                        Gate::Scripts,
                        format!("{at} {}", script.name),
                        &envelope.summary,
                    ));
                }
                if diagnosis.diagnostics.is_empty() && !matches!(diagnosis.verdict, Verdict::Checked(_)) {
                    problems.push(Problem::new(
                        code::PERSISTENCE_FORMAT,
                        Gate::Scripts,
                        format!("{at} {}", script.name),
                        "the editor gave no verdict for the script",
                    ));
                }
                check.diagnostics = diagnosis.diagnostics;
                check.verdict = Some(diagnosis.verdict);
            }
            Err(envelope) => {
                problems.push(Problem::new(
                    envelope.code,
                    Gate::Scripts,
                    format!("{at} {}", script.name),
                    &envelope.summary,
                ));
                check.diagnostics = vec![envelope];
            }
        }
        checks.push(check);
    }
    (checks, problems)
}
