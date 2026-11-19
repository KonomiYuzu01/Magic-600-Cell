//! The LP3 source route: the frozen oracle cases O01 to O07 replayed so that each step's proposal comes from running
//! the built-in 600-cell-Full tool's 600 Cell Script program (LP3, C5).
//!
//! ```text
//! c600-lp3-source build
//! c600-lp3-source trace --assets <dir> [--host in-process|process] [--runtime <file>] [--receipts <file>]
//!                       [--tamper <case>:<index>] <case> <trace> [<case> <trace> ...]
//! ```
//!
//! `trace` opens one session per case at the case's initial state, as `c600-lp3-oracle` does, with the same session
//! name and the same shell. For each step the runner expands the step's recipe to its literal word through the kernel,
//! outside any program and without a state read ([`c600_tools::expand`]). The tool then runs in the solving profile
//! with that word as its pinned absolute argument, inside a command the solver invoked (ONE.34; finding B2 of the KN5
//! boundary attack, the owner's reading of 18 November 2026): the shell invokes the tool's row,
//! [`c600_tools::process::TOOL_ROW`], with the word, the layer admits a program job, and the tool runs as that job's
//! run, `job-<id>`. It runs on the host `--host` names: `in-process` (the default) runs it on `c600-run` inside this
//! process, and `process` runs it in `c600-runtime`'s child process ([`c600_tools::process`]), started from
//! `--runtime` or from the `c600-runtime` binary beside this one. On either host the same boundary and bridge answer
//! the program in this process: its one proposal reaches the command layer through the host bridge
//! ([`c600_tools::bridge`]) as the run's `operation.preview` invocation, and the program receives only the fixed
//! acknowledgement. The runner gives the bridge the solver's word steps of a recipe of plain word steps only, so a
//! proposal of exactly that recipe's literal word, when it is longer than one chunk of 100,000 letters, goes in the
//! recipe's own word steps; every other proposal goes in one word step (LB2.3, LB2.4). The layer runs the preview
//! job, and the runner then reports the solver's command with the program's acknowledgement: the layer builds a run's
//! preview only while the host holds the run's job. The shell presents the built preview and commits it by the key
//! route's commit chord, `Ctrl+Shift+Enter`, with a token bound to the presented preview. Undo and redo follow
//! `c600-lp3-oracle`'s schedule, by `Ctrl+Z` and `Ctrl+Y`. A recipe with no literal word stops at the expansion with
//! the kernel's refusal, before any command: it and a literal word the layer refuses at admission are `invalid-input`
//! steps. On either host a word the kernel refused that has no transfer encoding (a letter outside 1 to 1200 in
//! magnitude) is refused at the run's start stage: it is an `invalid-input` step whose solver's command fails with
//! that refusal's code, with no invocation.
//!
//! Each session's layer serves H-08 with the command layer's draft rows and holds the host's provenance reader
//! ([`c600_tools::workspace`]), which keeps every run's blocks, so a host can write and preview workspace records. The
//! runner writes none.
//!
//! The trace has `c600-lp3-oracle`'s form and candidate block, so the two routes' traces compare byte for byte.
//! `--receipts` writes every receipt the layer gave, the solver's commands and their reports included, in order, with
//! each one's caller and, per step, the host the
//! literal word was given to, the literal digest the tool proposed, the kernel's refusal of a recipe with no literal
//! word, the start-stage refusal of a word with no transfer encoding, and the preview's result digest rebased onto the
//! case recipe's draft and the command route's preview number (this one's less the solver's commands so far, since
//! each takes a job number). The summary line, each case and each step record name the host. `--tamper <case>:<index>`
//! negates the first letter of that step's literal word before the tool runs, as a negative control; the tampered word
//! is not the recipe's literal word, so it goes in one word step. Exit codes and the summary line follow
//! `c600-lp3-oracle`; the timings are informational.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use c600_command::{
    code, digest, word_draft, Caller, CommandLayer, Completion, CompletionKind, Direction, Error, HeadlessFrames,
    Receipt, Secret, Shell, Status,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Effect, IdentityBlock, Kernel, ProtectedSet, Refusal, Snapshot, SupportRow};
use c600_run::boundary::Store as RunStore;
use c600_run::meter::Outcome as RunOutcome;
use c600_runtime::Refused;
use c600_tools::bridge::{base, proposal_args, Bridge, Steps, WORD_CHUNK};
use c600_tools::case::{read_document, CASE_FORMAT};
use c600_tools::expand::{expand, literal_digest, word_steps, Expansion};
use c600_tools::process::{
    invoke_tool, run_in_process, runtime_beside, stage_name, start_error, Cause, ProcessHost, ToolCommand, IN_PROCESS,
    PROCESS,
};
use c600_tools::tool::{Tool, ROW, SOURCE};
use c600_tools::workspace::{self, Reader};

const TRACE_FORMAT: &str = "C600-ORACLE-TRACE-draft";
/// The trace's candidate: the command layer's loop, as `c600-lp3-oracle` names it, so the traces compare byte for byte.
const CANDIDATE: &str = "c600-command-lp3-loop";
const ROUTE: &str = "source";
const USAGE: &str = "usage: c600-lp3-source build | trace --assets <dir> [--host in-process|process] \
                     [--runtime <file>] [--receipts <file>] [--tamper <case>:<index>] <case> <trace> \
                     [<case> <trace> ...]";

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
            eprintln!("c600-lp3-source: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

struct Options {
    assets: String,
    /// Whether the tool runs on the process host.
    process: bool,
    /// The runtime's binary, for the process host; None for the one beside this binary.
    runtime: Option<String>,
    receipts: Option<String>,
    tamper: Option<(String, i64)>,
    files: Vec<String>,
}

fn tamper(value: &str) -> Option<(String, i64)> {
    let (case, index) = value.split_once(':')?;
    let index = index.parse().ok().filter(|&index: &i64| index >= 0)?;
    (!case.is_empty()).then(|| (case.to_owned(), index))
}

fn options(rest: &[String]) -> Result<Options, Failure> {
    let (mut assets, mut receipts, mut tampered, mut host, mut runtime) = (None, None, None, None, None);
    let mut index = 0;
    while index + 1 < rest.len() && rest[index].starts_with("--") {
        let value = rest[index + 1].clone();
        match rest[index].as_str() {
            "--assets" if assets.is_none() => assets = Some(value),
            "--receipts" if receipts.is_none() => receipts = Some(value),
            "--runtime" if runtime.is_none() => runtime = Some(value),
            "--host" if host.is_none() && value == IN_PROCESS => host = Some(false),
            "--host" if host.is_none() && value == PROCESS => host = Some(true),
            "--tamper" if tampered.is_none() => tampered = Some(tamper(&value).ok_or_else(|| malformed(USAGE))?),
            _ => return Err(malformed(USAGE)),
        }
        index += 2;
    }
    let files = rest[index..].to_vec();
    let process = host.unwrap_or(false);
    match assets {
        Some(assets) if !files.is_empty() && files.len() % 2 == 0 && (process || runtime.is_none()) => {
            Ok(Options { assets, process, runtime, receipts, tamper: tampered, files })
        }
        _ => Err(malformed(USAGE)),
    }
}

fn run(args: &[String]) -> Result<(), Failure> {
    match args.split_first() {
        Some((command, [])) if command == "build" => {
            println!("{}", c600_tools::build());
            Ok(())
        }
        Some((command, rest)) if command == "trace" => trace(&options(rest)?),
        _ => Err(malformed(USAGE)),
    }
}

fn seconds(start: Instant) -> Value {
    Value::Number(format!("{:.3}", start.elapsed().as_secs_f64()))
}

fn trace(options: &Options) -> Result<(), Failure> {
    let start = Instant::now();
    let kernel = Arc::new(
        Kernel::load(Path::new(&options.assets)).map_err(|error| malformed(format!("model check failed: {error}")))?,
    );
    let load_seconds = seconds(start);
    let tool = Tool::full_600_cell().map_err(|error| malformed(format!("the tool does not load: {error}")))?;
    let build = c600_command::build();
    let host = if options.process { PROCESS } else { IN_PROCESS };
    let mut process = match (options.process, &options.runtime) {
        (false, _) => None,
        (true, Some(path)) => Some(ProcessHost::new(PathBuf::from(path))),
        (true, None) => {
            let own = std::env::current_exe().map_err(|_| malformed("cannot find the runtime's binary"))?;
            Some(ProcessHost::new(runtime_beside(&own)))
        }
    };
    let mut cases = Vec::new();
    let mut receipts = Vec::new();
    for pair in options.files.chunks(2) {
        let started = Instant::now();
        let case = read_document(Path::new(&pair[0])).map_err(malformed)?;
        let replayed = replay(&kernel, &tool, process.as_mut(), &case, &build, options.tamper.as_ref())?;
        let mut text = replayed.trace.canonical();
        text.push('\n');
        std::fs::write(&pair[1], text).map_err(|_| malformed("cannot write a trace file"))?;
        cases.push(json::object([
            ("case_id", Value::String(replayed.case_id.clone())),
            ("steps", Value::Int(replayed.steps as i64)),
            ("committed", Value::Int(replayed.committed as i64)),
            ("undone", Value::Int(replayed.undone as i64)),
            ("redone", Value::Int(replayed.redone as i64)),
            ("runs", Value::Int(replayed.runs as i64)),
            ("start_refused", Value::Int(replayed.start_refused as i64)),
            ("receipts", Value::Int(replayed.receipts.len() as i64)),
            ("seconds", seconds(started)),
        ]));
        receipts.push(json::object([
            ("case_id", Value::String(replayed.case_id)),
            ("host", Value::String(host.into())),
            ("receipts", Value::Array(replayed.receipts)),
            ("callers", Value::Array(replayed.callers)),
            ("steps", Value::Array(replayed.records)),
            ("tail", Value::Int(replayed.tail as i64)),
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
        ("route", Value::String(ROUTE.into())),
        ("build", Value::String(build)),
        ("tools", Value::String(c600_tools::build())),
        ("host", Value::String(host.into())),
        ("runtime_spawns", process.as_ref().map_or(Value::Null, |process| Value::Int(process.spawns() as i64))),
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

/// One trace step, in `c600-lp3-oracle`'s form. `effect` is `None` for an invalid input, whose effect fields are null.
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

/// What the receipts file records of one step beside its receipts.
#[derive(Default)]
struct Record {
    literal: Option<(String, usize)>,
    refusal: Option<Value>,
    /// The start-stage refusal of a word with no transfer encoding, on either host.
    start: Option<Value>,
    draft: Option<String>,
    /// The digest of the preview's own summary, under this route's preview number and draft.
    result: Option<String>,
    rebased: Option<String>,
}

/// What became of one literal word given to the host.
enum Proposed {
    /// The tool ran inside the solver's command and its one proposal was sent: the layer's receipt and the command,
    /// which the step reports once the preview is built.
    Sent { receipt: Receipt, command: ToolCommand },
    /// The host refused the run at its start stage because the word has no transfer encoding. The tool did not run,
    /// no proposal reached the command layer, and the host reported the solver's command as its job's failure.
    NoTransfer(Refused),
}

fn refusal(refusal: &Refusal) -> Value {
    json::object([("code", Value::String(refusal.code.into())), ("detail", Value::String(refusal.message.clone()))])
}

/// One session of the source route: the layer, its provenance reader, its shell, the headless display's frames, the
/// tool, its host, and every receipt in order with its caller.
struct Session<'k> {
    kernel: &'k Kernel,
    tool: &'k Tool,
    /// The process host; None runs the tool in process.
    process: Option<&'k mut ProcessHost>,
    layer: CommandLayer,
    /// The layer's provenance reader, which keeps the blocks of every run.
    reader: Reader,
    shell: Shell,
    frames: HeadlessFrames,
    tamper: Option<i64>,
    runs: usize,
    start_refused: usize,
    /// The solver's commands invoked so far, one per word given to the tool. Each takes a job number, so the command
    /// route's job and preview numbers are this route's less this count.
    commands: u64,
    receipts: Vec<Value>,
    callers: Vec<Value>,
}

impl Session<'_> {
    fn keep(&mut self, receipt: &Receipt, caller: &Caller) {
        self.receipts.push(receipt.to_json());
        self.callers.push(match caller {
            Caller::Shell => json::object([("caller", Value::String("shell".into()))]),
            Caller::Runtime { run } => {
                json::object([("caller", Value::String("runtime".into())), ("run", Value::String(run.clone()))])
            }
        });
    }

    /// One press of `chord` on the key route.
    fn press(&mut self, chord: &str) -> Result<Receipt, Failure> {
        let receipt = match self.shell.press(&mut self.layer, chord, false) {
            Ok(Some(receipt)) => receipt,
            Ok(None) => return Err(disagree(format!("`{chord}` was dropped as a repeat"))),
            Err(error) => return Err(disagree(format!("the shell stopped `{chord}`: {}", error.code))),
        };
        self.keep(&receipt, &Caller::Shell);
        Ok(receipt)
    }

    /// Runs the tool on `word` on the session's host, inside the command the solver invokes to run it: the shell
    /// invokes the tool's row with the word and the host takes the job ([`invoke_tool`]). The entry runs as the job's
    /// run through a bridge given `steps`, the solver's word steps of the step's recipe. Returns the run's one
    /// invocation's receipt with the command, which the step reports once the preview is built. Or returns the
    /// start-stage refusal of a word with no transfer encoding, which both hosts give before the run reaches the layer;
    /// the host then reports the command as its job's failure.
    fn propose(&mut self, word: &[i32], steps: Option<Steps>) -> Result<Proposed, Failure> {
        let command = invoke_tool(&mut self.shell, &mut self.layer, self.tool, word)
            .map_err(|detail| disagree(format!("the solver's command to run the tool: {detail}")))?;
        self.commands += 1;
        self.keep(&command.receipt, &Caller::Shell);
        let run = command.run();
        let read = base(&command.job.revision);
        let mut store = RunStore::default();
        // The one invocation's arguments: the word in one word step, or in the solver's word steps (LB2.4).
        let args = proposal_args(word, steps.as_ref());
        let mut bridge = Bridge::new(&mut self.layer, &run).with_steps(steps);
        let result = match self.process.as_deref_mut() {
            None => run_in_process(self.tool, word, &run, &read, &mut bridge, &mut store),
            Some(host) => host.run(self.tool, SOURCE, word, &run, &read, &mut bridge, &mut store),
        };
        self.reader.keep_run(&store);
        let ran = match result {
            Ok(ran) => ran,
            Err(stopped) => {
                let reached = !bridge.sent.is_empty() || !stopped.shell.is_empty();
                drop(bridge);
                let start = start_error(&stopped.cause).filter(|_| !reached);
                return match (stopped.cause, start) {
                    (Cause::Refused(refused), Some(error)) => {
                        self.start_refused += 1;
                        self.report(command, Some(error))?;
                        Ok(Proposed::NoTransfer(refused))
                    }
                    (Cause::Refused(refused), None) => Err(disagree(format!(
                        "the host refused the tool's run at the {} stage: {}",
                        stage_name(refused.stage),
                        refused.codes.join(" ")
                    ))),
                    (Cause::OtherSource, _) => Err(disagree("the process host was given another source")),
                    (Cause::Lost(fault), _) => {
                        Err(disagree(format!("the runtime's process was lost: {}", fault.code())))
                    }
                };
            }
        };
        let sent = std::mem::take(&mut bridge.sent);
        drop(bridge);
        self.runs += 1;
        if ran.run.summary.outcome != RunOutcome::Completed {
            return Err(disagree(format!("the tool's run ended {:?}", ran.run.summary.outcome)));
        }
        let acknowledgement = format!("{{\"base\":\"{read}\",\"id\":\"{run}/1\"}}\n");
        if ran.run.output.as_deref() != Some(acknowledgement.as_bytes()) {
            return Err(disagree("the program received more than the fixed acknowledgement"));
        }
        let [sent] = &sent[..] else {
            return Err(disagree(format!("the tool's run made {} invocations instead of one", sent.len())));
        };
        let invocation = &sent.invocation;
        let expected = Caller::Runtime { run: run.clone() };
        if invocation.row != ROW
            || sent.caller != expected
            || invocation.token.is_some()
            || invocation.args.canonical() != args.canonical()
        {
            return Err(disagree("the bridge's invocation is not the runtime's preview of the literal word"));
        }
        self.keep(&sent.receipt, &sent.caller);
        Ok(Proposed::Sent { receipt: sent.receipt.clone(), command })
    }

    /// Reports the job of the solver's command that ran the tool and keeps the command's receipt. A run that ran
    /// returns the program's fixed acknowledgement ([`ToolCommand::acknowledge`]); a run refused at its start stage
    /// fails with `refusal`.
    fn report(&mut self, command: ToolCommand, refusal: Option<Error>) -> Result<(), Failure> {
        let receipt = match refusal {
            Some(error) => command.finish(&mut self.layer, Err(error)),
            None => command.acknowledge(&mut self.layer),
        };
        let receipt = receipt.ok_or_else(|| disagree("the tool's job was not reported"))?;
        self.keep(&receipt, &Caller::Shell);
        Ok(())
    }

    /// Expands `recipe`, runs the tool on its literal word, and presents and commits the preview the run proposed.
    fn step(&mut self, index: i64, recipe: &Value, record: &mut Record) -> Result<Outcome, Failure> {
        let (mut word, unchecked) = match expand(self.kernel, recipe) {
            Expansion::Literal(letters) => (letters, None),
            Expansion::Unchecked { letters, refusal: kernel } => (letters, Some(refusal(&kernel))),
            Expansion::Refused(kernel) => {
                record.refusal = Some(refusal(&kernel));
                return Ok(Outcome::Invalid);
            }
        };
        // The solver's word steps, for a recipe of plain word steps only, are taken before any tampering: a tampered
        // word is not the recipe's literal word, so it goes in one word step.
        let steps = word_steps(recipe).and_then(|lengths| Steps::new(&word, lengths));
        if self.tamper == Some(index) {
            match word.first_mut() {
                Some(letter) => *letter = -*letter,
                None => word.push(1),
            }
        }
        record.literal = Some((literal_digest(&word), word.len()));
        let (admitted, command) = match (self.propose(&word, steps)?, unchecked) {
            (Proposed::Sent { receipt, command }, _) => (receipt, command),
            // Only a word the kernel already refused may stop here: every letter of a literal word is a generator.
            // The record keeps the kernel's refusal of the recipe beside the start-stage refusal.
            (Proposed::NoTransfer(refused), Some(kernel)) => {
                record.start = Some(json::object([
                    ("stage", Value::String(stage_name(refused.stage).into())),
                    ("codes", Value::Array(refused.codes.into_iter().map(Value::String).collect())),
                    ("recipe_refusal", kernel),
                ]));
                return Ok(Outcome::Invalid);
            }
            (Proposed::NoTransfer(_), None) => {
                return Err(disagree("a literal word the kernel expanded has no transfer encoding"));
            }
        };
        let id = match &admitted.status {
            Status::Running(id) => *id,
            Status::Refused(error) if error.code.starts_with("KN2:") => {
                self.report(command, None)?;
                return Ok(Outcome::Invalid);
            }
            other => return Err(unexpected("operation.preview", other)),
        };
        if self.layer.run_jobs() != 1 {
            return Err(disagree("the preview job did not run"));
        }
        // The runtime's command IDs are its own scope (ruling 4), so the lookup names the caller.
        let caller = Caller::Runtime { run: command.run() };
        let built = self.layer.kernel_outcome(&caller, &admitted.command, &admitted.request);
        self.keep(&built, &caller);
        if !matches!(built.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. })) {
            return Err(unexpected("operation.preview", &built.status));
        }
        // The preview is built, so the host reports the solver's command; the solver then sees the preview.
        self.report(command, None)?;
        let presented = self.frames.present(&mut self.shell, &mut self.layer, id);
        presented.map_err(|error| disagree(format!("present: {}", error.code)))?;
        let preview = self.layer.preview().filter(|p| p.id == id).ok_or_else(|| disagree("the preview is gone"))?;
        // The result digest covers the draft's digest and the preview's number. The command route drafts the case's
        // recipe; the source route drafts the literal word, in one word step or, longer than one chunk, in the
        // recipe's word steps. The command route makes no solver's command, so its preview's number is this one's
        // less the solver's commands so far. The rebased digest is the preview's result over the case recipe's draft
        // under the command route's number.
        let mut rebased = preview.clone();
        rebased.deps.draft = digest(&word_draft(recipe));
        rebased.id = preview.id.checked_sub(self.commands).ok_or_else(|| disagree("a preview before its command"))?;
        record.draft = Some(preview.deps.draft.clone());
        record.result = Some(digest(&preview.summary()));
        record.rebased = Some(digest(&rebased.summary()));
        let seen = Seen { effect: preview.effect.clone(), conflicts: preview.protection.conflicts.clone() };
        let (base, end) = (preview.base.digest().to_owned(), preview.end.as_ref().map(|s| s.digest().to_owned()));
        let receipt = self.press("Ctrl+Shift+Enter")?;
        match &receipt.status {
            Status::Committed(commit) => {
                let end = end.ok_or_else(|| disagree("a preview without an end state was committed"))?;
                if commit.pre != base || commit.post != end || self.layer.snapshot().digest() != end {
                    return Err(disagree("the commit record differs from the preview it committed"));
                }
                Ok(Outcome::Committed(seen))
            }
            Status::Refused(error) if error.code == code::ORBIT_WOULD_MOVE => {
                self.dismiss()?;
                Ok(Outcome::Protected(seen))
            }
            other => Err(unexpected("operation.execute", other)),
        }
    }

    /// The solver dismisses the preview a refused commit left shown, with the key route's Escape: only the solver's
    /// action ends a shown preview, so the next run's preview is not admitted while it stays (findings B1 and L17 of the
    /// KN5 boundary attack, the owner's reading of 18 November 2026). The cancel changes no revision and takes no job,
    /// so it is not kept with the step's receipts; it shifts only the shell's later command IDs.
    fn dismiss(&mut self) -> Result<(), Failure> {
        match self.shell.press(&mut self.layer, "Escape", false) {
            Ok(Some(receipt)) if matches!(receipt.status, Status::Completed(_)) => Ok(()),
            Ok(Some(receipt)) => Err(unexpected("operation.preview.cancel", &receipt.status)),
            Ok(None) => Err(disagree("`Escape` was dropped as a repeat")),
            Err(error) => Err(disagree(format!("the shell stopped `Escape`: {}", error.code))),
        }
    }

    /// Undoes or redoes the top action by its chord; the state must then be `expected`.
    fn back(&mut self, direction: Direction, expected: &str) -> Result<(), Failure> {
        let (row, chord) = match direction {
            Direction::Undo => ("session.undo", "Ctrl+Z"),
            Direction::Redo => ("session.redo", "Ctrl+Y"),
        };
        let receipt = self.press(chord)?;
        match &receipt.status {
            Status::Committed(commit) if commit.post == expected && self.layer.snapshot().digest() == expected => {
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
    runs: usize,
    start_refused: usize,
    trace: Value,
    receipts: Vec<Value>,
    callers: Vec<Value>,
    records: Vec<Value>,
    tail: usize,
}

fn replay(
    kernel: &Arc<Kernel>,
    tool: &Tool,
    process: Option<&mut ProcessHost>,
    case: &Value,
    build: &str,
    tamper: Option<&(String, i64)>,
) -> Result<Replayed, Failure> {
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
        workspace::table(),
        secret.clone(),
        Default::default(),
        &format!("oracle-{case_id}"),
        start,
        protected,
    );
    let reader = workspace::install(&mut layer);
    let shell = Shell::new(secret, &case_id);
    let tamper = tamper.filter(|(case, _)| *case == case_id).map(|&(_, index)| index);
    let host = if process.is_some() { PROCESS } else { IN_PROCESS };
    let mut session = Session {
        kernel,
        tool,
        process,
        layer,
        reader,
        shell,
        frames: HeadlessFrames::new(),
        tamper,
        runs: 0,
        start_refused: 0,
        commands: 0,
        receipts: Vec::new(),
        callers: Vec::new(),
    };
    let mut memo = HashMap::new();
    // The state before the first step and after every committed step of a chain.
    let mut states = vec![initial.clone()];
    let (mut committed, mut undone, mut redone) = (0, 0, 0);
    let mut out = Vec::with_capacity(ordered.len());
    let mut records = Vec::with_capacity(ordered.len());
    for (index, recipe) in ordered {
        let before = session.layer.snapshot().digest().to_owned();
        if independent && before != initial {
            return Err(disagree("an independent step did not start from the initial state"));
        }
        let first = session.receipts.len();
        let mut record = Record::default();
        let row = match session.step(index, recipe, &mut record)? {
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
        let text_or_null = |value: Option<String>| value.map_or(Value::Null, Value::String);
        let given = record.literal.is_some();
        let (literal, letters) = match record.literal {
            Some((digest, letters)) => (Value::String(digest), Value::Int(letters as i64)),
            None => (Value::Null, Value::Null),
        };
        records.push(json::object([
            ("index", Value::Int(index)),
            ("receipts", Value::Int((session.receipts.len() - first) as i64)),
            ("host", if given { Value::String(host.into()) } else { Value::Null }),
            ("literal", literal),
            ("letters", letters),
            ("expansion_refusal", record.refusal.unwrap_or(Value::Null)),
            ("start_refusal", record.start.unwrap_or(Value::Null)),
            ("draft", text_or_null(record.draft)),
            ("case_draft", Value::String(digest(&word_draft(recipe)))),
            ("result", text_or_null(record.result)),
            ("rebased_result", text_or_null(record.rebased)),
        ]));
    }
    let first = session.receipts.len();
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
    let tail = session.receipts.len() - first;
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
    Ok(Replayed {
        case_id,
        steps: count,
        committed,
        undone,
        redone,
        runs: session.runs,
        start_refused: session.start_refused,
        trace,
        receipts: session.receipts,
        callers: session.callers,
        records,
        tail,
    })
}
