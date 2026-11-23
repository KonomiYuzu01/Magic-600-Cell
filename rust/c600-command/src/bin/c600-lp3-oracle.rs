//! The LP3 loop runner: the frozen oracle cases O01 to O07 through the command layer, the headless vertical path of
//! the 600-cell-Full tool.
//!
//! ```text
//! c600-lp3-oracle build
//! c600-lp3-oracle trace --assets <dir> --route command|key [--receipts <file>] <case> <trace> [<case> <trace> ...]
//! ```
//!
//! `trace` opens one session per case at the case's initial state and drives every step through
//! [`CommandLayer::invoke`]: `operation.preview` with the step's recipe as an inline draft, the preview job, the
//! shell's presentation of the built preview, and `operation.execute` with a token bound to that preview. No scramble
//! row is built, so the initial word reaches the kernel before the session opens, as in LP2's M4 adapter. A draft the
//! kernel refuses at admission is an `invalid-input` step; a commit refused at step 8 with
//! `KN2:protection.orbit_would_move` is a `protected` step with the preview's effects; a committed step reads its
//! state from the layer. In an independent case every committed step is undone with `session.undo` and must return to
//! the initial state. In a chain case, after the last step, every committed step is undone and then redone, and each
//! must land on the state the trace recorded for it, which `tools/oracle/compare.py` checks against the oracle.
//!
//! `--route command` invokes the rows with absolute arguments. `--route key` presses the draft key map's chords
//! (`Ctrl+Enter`, `Ctrl+Shift+Enter`, `Ctrl+KeyZ`, `Ctrl+KeyY`), which the shell resolves to rows and absolute
//! arguments at the press. `--receipts` writes every receipt in order, so the two routes can be compared byte for byte.
//! A malformed case or a model mismatch exits with 2 and writes no trace for that case; any answer of the command layer
//! the loop does not expect exits with 1. Both commands print one line to stdout: the build string, or a JSON summary
//! with timings, which are informational.

use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use c600_command::{
    code, draft_args, word_draft, CommandLayer, Completion, CompletionKind, Direction, HeadlessFrames, Receipt, Secret,
    Shell, Status, Table,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{archive, Derivation, Effect, IdentityBlock, Kernel, ProtectedSet, Snapshot, SupportRow};

const CASE_FORMAT: &str = "C600-ORACLE-CASE-draft";
const TRACE_FORMAT: &str = "C600-ORACLE-TRACE-draft";
const CANDIDATE: &str = "c600-command-lp3-loop";
const USAGE: &str = "usage: c600-lp3-oracle build | trace --assets <dir> --route command|key [--receipts <file>] \
                     <case> <trace> [<case> <trace> ...]";
/// 0.4's word limit; the initial word enters the kernel in words of at most this many letters.
const WORD_CHUNK: usize = 100_000;
/// The largest decompressed case the runner reads.
const CASE_LIMIT: usize = 1 << 30;

struct Failure {
    code: u8,
    message: String,
}

fn malformed(message: impl Into<String>) -> Failure {
    Failure { code: 2, message: message.into() }
}

fn disagree(message: impl Into<String>) -> Failure {
    Failure { code: 1, message: message.into() }
}

fn unexpected(row: &str, status: &Status) -> Failure {
    disagree(format!("`{row}` answered {status:?}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("c600-lp3-oracle: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    Command,
    Key,
}

impl Route {
    fn name(self) -> &'static str {
        match self {
            Route::Command => "command",
            Route::Key => "key",
        }
    }
}

struct Options {
    assets: String,
    route: Route,
    receipts: Option<String>,
    files: Vec<String>,
}

fn options(rest: &[String]) -> Result<Options, Failure> {
    let (mut assets, mut route, mut receipts) = (None, None, None);
    let mut index = 0;
    while index + 1 < rest.len() && rest[index].starts_with("--") {
        let value = rest[index + 1].clone();
        match rest[index].as_str() {
            "--assets" if assets.is_none() => assets = Some(value),
            "--route" if route.is_none() => {
                route = Some(match value.as_str() {
                    "command" => Route::Command,
                    "key" => Route::Key,
                    _ => return Err(malformed(USAGE)),
                })
            }
            "--receipts" if receipts.is_none() => receipts = Some(value),
            _ => return Err(malformed(USAGE)),
        }
        index += 2;
    }
    let files = rest[index..].to_vec();
    match (assets, route) {
        (Some(assets), Some(route)) if !files.is_empty() && files.len() % 2 == 0 => {
            Ok(Options { assets, route, receipts, files })
        }
        _ => Err(malformed(USAGE)),
    }
}

fn run(args: &[String]) -> Result<(), Failure> {
    match args.split_first() {
        Some((command, [])) if command == "build" => {
            println!("{}", c600_command::build());
            Ok(())
        }
        Some((command, rest)) if command == "trace" => trace(&options(rest)?),
        _ => Err(malformed(USAGE)),
    }
}

fn seconds(start: Instant) -> Value {
    Value::Number(format!("{:.3}", start.elapsed().as_secs_f64()))
}

fn read_document(path: &Path) -> Result<Value, Failure> {
    let bytes = std::fs::read(path).map_err(|_| malformed("cannot read a case file"))?;
    let bytes = if bytes.starts_with(&[0x1f, 0x8b]) {
        archive::gunzip(&bytes, CASE_LIMIT).map_err(|error| malformed(format!("unreadable gzip case: {error}")))?
    } else {
        bytes
    };
    json::parse(&bytes).map_err(|error| malformed(format!("malformed case JSON: {error}")))
}

fn trace(options: &Options) -> Result<(), Failure> {
    let start = Instant::now();
    let kernel = Arc::new(
        Kernel::load(Path::new(&options.assets)).map_err(|error| malformed(format!("model check failed: {error}")))?,
    );
    let load_seconds = seconds(start);
    let build = c600_command::build();
    let mut cases = Vec::new();
    let mut receipts = Vec::new();
    for pair in options.files.chunks(2) {
        let started = Instant::now();
        let case = read_document(Path::new(&pair[0]))?;
        let replayed = replay(&kernel, &case, options.route, &build)?;
        let mut text = replayed.trace.canonical();
        text.push('\n');
        std::fs::write(&pair[1], text).map_err(|_| malformed("cannot write a trace file"))?;
        cases.push(json::object([
            ("case_id", Value::String(replayed.case_id.clone())),
            ("steps", Value::Int(replayed.steps as i64)),
            ("committed", Value::Int(replayed.committed as i64)),
            ("undone", Value::Int(replayed.undone as i64)),
            ("redone", Value::Int(replayed.redone as i64)),
            ("receipts", Value::Int(replayed.receipts.len() as i64)),
            ("seconds", seconds(started)),
        ]));
        receipts.push(json::object([
            ("case_id", Value::String(replayed.case_id)),
            ("receipts", Value::Array(replayed.receipts)),
        ]));
    }
    if let Some(path) = &options.receipts {
        let mut text = json::object([("cases", Value::Array(receipts))]).canonical();
        text.push('\n');
        std::fs::write(path, text).map_err(|_| malformed("cannot write the receipts file"))?;
    }
    let summary = json::object([
        ("command", Value::String("trace".into())),
        ("candidate", Value::String(CANDIDATE.into())),
        ("route", Value::String(options.route.name().into())),
        ("build", Value::String(build)),
        ("load_seconds", load_seconds),
        ("cases", Value::Array(cases)),
        ("total_seconds", seconds(start)),
    ]);
    println!("{}", summary.canonical());
    Ok(())
}

fn text(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).filter(|text| !text.is_empty()).map(str::to_owned)
}

fn integers(value: Option<&Value>) -> Option<Vec<i64>> {
    value?.as_array()?.iter().map(Value::as_i64).collect()
}

fn progress(kernel: &Kernel, memo: &mut HashMap<String, String>, state: &Snapshot) -> Result<String, Failure> {
    if let Some(sha256) = memo.get(state.digest()) {
        return Ok(sha256.clone());
    }
    let sha256 = kernel.progress(state).map_err(|refusal| malformed(refusal.to_string()))?.sha256;
    memo.insert(state.digest().to_owned(), sha256.clone());
    Ok(sha256)
}

fn rows(rows: &[SupportRow]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json::object([
                    ("orbit", Value::Int(row.orbit as i64)),
                    ("stickers", Value::Int(row.stickers as i64)),
                    ("pieces", Value::Int(row.pieces as i64)),
                ])
            })
            .collect(),
    )
}

/// One trace step, in the M4 adapter's form. `effect` is `None` for an invalid input, whose effect fields are null.
fn step(
    index: i64,
    error: Option<&str>,
    state: &Snapshot,
    effect: Option<(&Effect, &[SupportRow])>,
    progress: String,
) -> Value {
    let effect_fields = match effect {
        Some((effect, conflicts)) => [
            Value::String(effect.net_sha256.clone()),
            Value::Int(effect.moved_stickers() as i64),
            Value::Int(effect.moved_pieces as i64),
            Value::String(effect.expansion_sha256.clone()),
            Value::Int(effect.primitive_count as i64),
            rows(&effect.support),
            rows(conflicts),
        ],
        None => std::array::from_fn(|_| Value::Null),
    };
    let [net, stickers, pieces, expansion, count, support, conflicts] = effect_fields;
    json::object([
        ("index", Value::Int(index)),
        ("accepted", Value::Bool(error.is_none())),
        ("error", error.map_or(Value::Null, |error| Value::String(error.to_owned()))),
        ("state_hash", Value::String(state.digest().to_owned())),
        ("net_sha256", net),
        ("net_moved_stickers", stickers),
        ("net_moved_pieces", pieces),
        ("expansion_sha256", expansion),
        ("primitive_count", count),
        ("support", support),
        ("conflicts", conflicts),
        ("progress_sha256", Value::String(progress)),
    ])
}

/// What a step's preview reported: its effect and its protection conflicts.
struct Seen {
    effect: Effect,
    conflicts: Vec<SupportRow>,
}

enum Outcome {
    Invalid,
    Protected(Seen),
    Committed(Seen),
}

/// One session of the loop: the layer, its shell, the headless display's frames and every receipt in order.
struct Session {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    route: Route,
    receipts: Vec<Value>,
}

impl Session {
    /// One user action: `row` with absolute arguments on the command route, `chord` on the key route.
    fn act(&mut self, row: &str, args: Value, chord: &str) -> Result<Receipt, Failure> {
        let answer = match self.route {
            Route::Command => self.shell.invoke(&mut self.layer, row, args).map(Some),
            Route::Key => self.shell.press(&mut self.layer, chord, false),
        };
        let receipt = match answer {
            Ok(Some(receipt)) => receipt,
            Ok(None) => return Err(disagree(format!("`{chord}` was dropped as a repeat"))),
            Err(error) => return Err(disagree(format!("the shell stopped `{row}`: {}", error.code))),
        };
        self.receipts.push(receipt.to_json());
        Ok(receipt)
    }

    /// Previews `recipe`, presents the preview and commits it.
    fn step(&mut self, recipe: &Value) -> Result<Outcome, Failure> {
        if self.route == Route::Key {
            self.shell.draft = Some(word_draft(recipe));
        }
        let admitted = self.act("operation.preview", draft_args(recipe), "Ctrl+Enter")?;
        let id = match &admitted.status {
            Status::Running(id) => *id,
            Status::Refused(error) if error.code.starts_with("KN2:") => return Ok(Outcome::Invalid),
            other => return Err(unexpected("operation.preview", other)),
        };
        if self.layer.run_jobs() != 1 {
            return Err(disagree("the preview job did not run"));
        }
        let built = self.layer.outcome(&admitted.command, &admitted.request);
        let built = built.ok_or_else(|| disagree("the preview has no receipt"))?;
        self.receipts.push(built.to_json());
        if !matches!(built.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. })) {
            return Err(unexpected("operation.preview", &built.status));
        }
        let presented = self.frames.present(&mut self.shell, &mut self.layer, id);
        presented.map_err(|error| disagree(format!("present: {}", error.code)))?;
        let preview = self.layer.preview().filter(|p| p.id == id).ok_or_else(|| disagree("the preview is gone"))?;
        let seen = Seen { effect: preview.effect.clone(), conflicts: preview.protection.conflicts.clone() };
        let (base, end) = (preview.base.digest().to_owned(), preview.end.as_ref().map(|s| s.digest().to_owned()));
        let receipt =
            self.act("operation.execute", json::object([("preview", Value::Int(id as i64))]), "Ctrl+Shift+Enter")?;
        match &receipt.status {
            Status::Committed(record) => {
                let end = end.ok_or_else(|| disagree("a preview without an end state was committed"))?;
                if record.pre != base || record.post != end || self.layer.snapshot().digest() != end {
                    return Err(disagree("the commit record differs from the preview it committed"));
                }
                Ok(Outcome::Committed(seen))
            }
            Status::Refused(error) if error.code == code::ORBIT_WOULD_MOVE => Ok(Outcome::Protected(seen)),
            other => Err(unexpected("operation.execute", other)),
        }
    }

    /// Undoes or redoes the top action; the state must then be `expected`.
    fn back(&mut self, direction: Direction, expected: &str) -> Result<(), Failure> {
        let hint = self.layer.undo_hint(direction).ok_or_else(|| disagree("no action to undo or redo"))?;
        let (row, chord) = match direction {
            Direction::Undo => ("session.undo", "Ctrl+KeyZ"),
            Direction::Redo => ("session.redo", "Ctrl+KeyY"),
        };
        let receipt = self.act(row, json::object([("hint", hint.to_json())]), chord)?;
        match &receipt.status {
            Status::Committed(record) if record.post == expected && self.layer.snapshot().digest() == expected => {
                Ok(())
            }
            Status::Committed(_) => Err(disagree(format!("`{row}` did not reach the recorded state"))),
            other => Err(unexpected(row, other)),
        }
    }
}

struct Replayed {
    case_id: String,
    steps: usize,
    committed: usize,
    undone: usize,
    redone: usize,
    trace: Value,
    receipts: Vec<Value>,
}

fn replay(kernel: &Arc<Kernel>, case: &Value, route: Route, build: &str) -> Result<Replayed, Failure> {
    if case.get("format").and_then(Value::as_str) != Some(CASE_FORMAT) {
        return Err(malformed("wrong case format"));
    }
    let case_id = text(case.get("case_id")).ok_or_else(|| malformed("missing case_id"))?;
    let model = case.get("model");
    let model_id = text(model.and_then(|m| m.get("model_id"))).ok_or_else(|| malformed("missing model.model_id"))?;
    let manifest =
        text(model.and_then(|m| m.get("manifest_sha256"))).ok_or_else(|| malformed("missing model.manifest_sha256"))?;
    kernel.check_identity(&IdentityBlock::full(&model_id)).map_err(|refusal| {
        malformed(format!("case model identity differs from the loaded model ({})", refusal.code))
    })?;
    if manifest != kernel.manifest_sha256() {
        return Err(malformed("case manifest_sha256 differs from the loaded model"));
    }
    let independent = match case.get("mode").and_then(Value::as_str) {
        Some("independent") => true,
        Some("chain") => false,
        _ => return Err(malformed("malformed case mode")),
    };
    let initial = case.get("initial");
    let moves = integers(initial.and_then(|i| i.get("moves"))).ok_or_else(|| malformed("malformed initial moves"))?;
    let expected =
        text(initial.and_then(|i| i.get("state_hash"))).ok_or_else(|| malformed("missing initial state_hash"))?;
    let protected = integers(case.get("protected")).ok_or_else(|| malformed("malformed protected orbits"))?;
    let protected = kernel.protected_set(&protected).map_err(|_| malformed("malformed protected orbits"))?;
    let steps = case.get("steps").and_then(Value::as_array).filter(|steps| !steps.is_empty());
    let steps = steps.ok_or_else(|| malformed("missing steps"))?;
    let mut ordered = Vec::with_capacity(steps.len());
    for item in steps {
        let index = item.get("index").and_then(Value::as_i64).filter(|&index| index >= 0);
        let index = index.ok_or_else(|| malformed("malformed step index"))?;
        let recipe = item.get("recipe").ok_or_else(|| malformed("missing step recipe"))?;
        ordered.push((index, recipe));
    }
    ordered.sort_by_key(|&(index, _)| index);
    if ordered.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(malformed("duplicate step index"));
    }

    let none = ProtectedSet::none();
    let mut start = kernel.solved();
    for chunk in moves.chunks(WORD_CHUNK) {
        let recipe = kernel.word(chunk).map_err(|_| malformed("malformed initial moves"))?;
        start = match kernel.derive(&start, &recipe, &none) {
            Ok(Derivation::Derived(derived)) => derived.state,
            Ok(Derivation::Refused(_)) => return Err(malformed("the initial word was refused")),
            Err(error) => return Err(malformed(format!("model check failed: {error}"))),
        };
    }
    if start.digest() != expected {
        return Err(malformed("initial state_hash differs from the loaded model's replay"));
    }
    let initial = start.digest().to_owned();

    let secret = Secret::launch();
    let mut layer = CommandLayer::new(
        kernel.clone(),
        Table::h08(),
        secret.clone(),
        Default::default(),
        &format!("oracle-{case_id}"),
        start,
        protected,
    );
    // The oracle loop commits each preview on the frame that presents it, a preview after a refused step included;
    // ADR-D01.9's settle interval, which holds that press, is c600-command's `tests/settle.rs` concern and not an
    // oracle case's.
    layer.set_settle(Duration::ZERO);
    let shell = Shell::new(secret, &case_id);
    let mut session = Session { layer, shell, frames: HeadlessFrames::new(), route, receipts: Vec::new() };
    let mut memo = HashMap::new();
    // The state before the first step and after every committed step of a chain.
    let mut states = vec![initial.clone()];
    let (mut committed, mut undone, mut redone) = (0, 0, 0);
    let mut out = Vec::with_capacity(ordered.len());
    for (index, recipe) in ordered {
        let before = session.layer.snapshot().digest().to_owned();
        if independent && before != initial {
            return Err(disagree("an independent step did not start from the initial state"));
        }
        let row = match session.step(recipe)? {
            Outcome::Invalid => {
                let state = session.layer.snapshot();
                step(index, Some("invalid-input"), state, None, progress(kernel, &mut memo, state)?)
            }
            Outcome::Protected(seen) => {
                let state = session.layer.snapshot();
                if state.digest() != before {
                    return Err(disagree("a refused commit changed the state"));
                }
                let sha256 = progress(kernel, &mut memo, state)?;
                step(index, Some("protected"), state, Some((&seen.effect, &seen.conflicts)), sha256)
            }
            Outcome::Committed(seen) => {
                committed += 1;
                let state = session.layer.snapshot();
                let sha256 = progress(kernel, &mut memo, state)?;
                let row = step(index, None, state, Some((&seen.effect, &seen.conflicts)), sha256);
                if independent {
                    session.back(Direction::Undo, &initial)?;
                    undone += 1;
                } else {
                    states.push(state.digest().to_owned());
                }
                row
            }
        };
        out.push(row);
    }
    if !independent {
        for k in (1..states.len()).rev() {
            session.back(Direction::Undo, &states[k - 1])?;
            undone += 1;
        }
        for state in &states[1..] {
            session.back(Direction::Redo, state)?;
            redone += 1;
        }
    }
    let count = out.len();
    let trace = json::object([
        ("format", Value::String(TRACE_FORMAT.into())),
        ("case_id", Value::String(case_id.clone())),
        (
            "model",
            json::object([
                ("model_id", Value::String(kernel.identity().model_id)),
                ("manifest_sha256", Value::String(kernel.manifest_sha256().to_owned())),
            ]),
        ),
        (
            "candidate",
            json::object([("name", Value::String(CANDIDATE.into())), ("build", Value::String(build.to_owned()))]),
        ),
        ("steps", Value::Array(out)),
    ]);
    Ok(Replayed { case_id, steps: count, committed, undone, redone, trace, receipts: session.receipts })
}
