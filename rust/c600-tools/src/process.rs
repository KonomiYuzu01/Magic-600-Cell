//! The process host: one run of a built-in tool in the runtime's own process (route RT1, LQ9; RTE.21, ONE.69).
//!
//! [`ProcessHost`] sends a run to `c600-runtime`'s child process as a job (`c600_runtime::parent::Runtime::run`): the
//! bytes of the module's source, the entry and the literal word. The child parses, lowers, admits, loads and runs the
//! program and forwards every host call to this process. Here the same run-time boundary
//! ([`c600_run::boundary::Boundary`]) and the same engine side answer them as they answer the in-process host
//! ([`crate::tool::Tool::run_in`]): for the source route the engine side is [`crate::bridge::Bridge`], so each proposal
//! the boundary admits is one runtime invocation of `operation.preview` on the command layer. The child holds the
//! program and its word and nothing else: no token and no secret has a field on the wire (KN5.15), the command layer
//! stays in this process, and the program's answer to a proposal is the boundary's fixed acknowledgement (BND.42).
//!
//! Both hosts refuse the same words at the start stage. A word with a letter outside 1 to 1200 in magnitude, chapter
//! 01's generators, has no transfer encoding: on the process host the parent refuses its job before it reaches the
//! child, and in process [`run_in_process`] refuses it the same way before the tool runs. `Loaded::run` itself refuses
//! only a letter of magnitude above 1200, which no alphabet has. Neither run reaches the engine side
//! ([`Cause::Refused`]).
//!
//! On either host a run starts inside a command the solver invoked (ONE.34; finding B2 of the KN5 boundary attack,
//! the owner's reading of 18 November 2026). The shell invokes the tool's row, [`TOOL_ROW`], with the word, the layer
//! admits a program job, and the host takes it ([`invoke_tool`]). The host runs the entry as the job's run, `job-<id>`,
//! which the bridge stamps on the run's invocations, on the job's word at the job's revision, and under the job's
//! budget: the budget the solver's command declared and the layer admitted (KN7.7; [`ToolCommand::limits`]). Then it
//! reports the job ([`ToolCommand::finish`]) once the preview the run proposed is built: the layer prepares and builds
//! a run's preview only while the host holds the job. A run refused at its start stage is reported as the job's
//! failure.

use std::path::{Path, PathBuf};
use std::time::Duration;

use c600_check::Profile;
use c600_command::{CommandLayer, Error as LayerError, Kind, Overlay, ProgramJob, Receipt, Shell, Status};
use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value as Json};
use c600_run::boundary::{Boundary, Context, Shown, Store};
use c600_run::interp::{code, Host};
use c600_run::meter::Amounts;
use c600_run::value::Value;
use c600_runtime::codec::encode_value;
use c600_runtime::wire::Enc;
use c600_runtime::{Config, Ended, Fault, Job, JobOptions, Refused, Runtime, Stage};

use crate::tool::{class_rows, limits, Ran, Tool, LIMITS};

/// The name of the in-process host in records: the tool's program runs on `c600-run` inside the caller's process.
pub const IN_PROCESS: &str = "in-process";
/// The name of the process host in records: the tool's program runs in `c600-runtime`'s child process.
pub const PROCESS: &str = "process";
/// The file stem of the runtime's binary.
pub const RUNTIME_BINARY: &str = "c600-runtime";

/// Why a run left no interpreter run.
#[derive(Debug)]
pub enum Cause {
    /// The source handed over is not the one the tool was admitted from. Nothing was sent.
    OtherSource,
    /// The run was refused before its first metered operation: at the start stage (on either host a word with no
    /// transfer encoding, and on the process host a job above the frame bound) or by the child's compile stage.
    Refused(Refused),
    /// The child was lost or passed its deadline; it has been killed and reaped (KN7.13).
    Lost(Fault),
}

impl Cause {
    /// Whether the run was refused because an argument has no transfer encoding: the start-stage refusal with the
    /// interpreter's non-canonical value code. The run reached neither the child nor the engine side.
    pub fn no_transfer(&self) -> bool {
        matches!(self, Cause::Refused(refused) if refused.stage == Stage::Start
            && refused.codes.len() == 1 && refused.codes[0] == code::NONCANONICAL_VALUE)
    }
}

/// The name of a compile or start stage in records.
pub fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Parse => "parse",
        Stage::Lower => "lower",
        Stage::Admit => "admit",
        Stage::Load => "load",
        Stage::Start => "start",
    }
}

/// A run that left no interpreter run. `shell` still holds what the boundary recorded for each proposal that reached it
/// before the run stopped.
#[derive(Debug)]
pub struct Stopped {
    pub cause: Cause,
    pub shell: Vec<Shown>,
}

/// Runs the tool's entry in this process as the job of `command`, the solver's command that ran the tool: on the job's
/// word, as the job's run at the job's revision ([`Tool::context`]) and under the job's budget
/// ([`ToolCommand::limits`]; [`Tool::run_under`]). It stops at the start stage, before any host call, where
/// [`ProcessHost::run`] stops. A word the process host cannot send, one with a letter outside 1 to 1200 in magnitude,
/// is refused with the interpreter's non-canonical value code before the tool runs, as the parent refuses its job
/// (`c600_runtime::Runtime::run`); an entry or argument that `Loaded::run` refuses stops there too.
pub fn run_in_process(
    tool: &Tool,
    command: &ToolCommand,
    engine: &mut dyn Host,
    store: &mut Store,
) -> Result<Ran, Stopped> {
    let refused = |codes: Vec<String>, detail: String| Stopped {
        cause: Cause::Refused(Refused { stage: Stage::Start, codes, detail }),
        shell: Vec::new(),
    };
    if encode_value(&Value::word(command.word()), &mut Enc::new()).is_err() {
        let detail = "an argument has no canonical transfer form".to_owned();
        return Err(refused(vec![code::NONCANONICAL_VALUE.into()], detail));
    }
    let context = tool.context(&command.run(), &command.base());
    tool.run_under(command.word(), context, command.limits(), engine, store)
        .map_err(|refusal| refused(vec![refusal.code.into()], refusal.detail))
}

/// The job of one run of `tool` on a literal word under [`LIMITS`], the shell's default budget ([`job_under`]).
pub fn job(tool: &Tool, source: &str, word: &[i32]) -> Job {
    job_under(tool, source, word, LIMITS)
}

/// The job of one run of `tool` on a literal word under `limits`: the source bytes, the manifest's module and entry,
/// the word as the entry's one argument, `limits`, the test tariffs and no source ranges, as [`Tool::run_under`] runs
/// it in process. The job runs in the solving profile and names the tool's class rows ([`class_rows`],
/// `JobOptions::built_in`), so the child admits the entry in the built-in column with `c600_check::admit_built_in`, as
/// [`Tool::new`] does in this process (chapter 07 section 8). Solving is the profile of the place where LP3's hosts
/// start a run, a command in a solving session (BND.17). The tool editor and a research workspace, which would set the
/// other two, do not exist yet, and in a session of another profile the layer refuses a run's `operation.preview`
/// (BND.16, BND.60).
pub fn job_under(tool: &Tool, source: &str, word: &[i32], limits: Amounts) -> Job {
    let mut options = JobOptions::new(limits);
    options.profile = Profile::Solving;
    options.built_in = Some(class_rows(&tool.manifest));
    options.source_ranges = false;
    Job {
        sources: vec![source.as_bytes().to_vec()],
        module: tool.manifest.module.into(),
        entry: tool.manifest.entry.into(),
        args: vec![Value::word(word)],
        options,
    }
}

/// The runtime's binary beside `executable`, where cargo puts the binaries of one build.
pub fn runtime_beside(executable: &Path) -> PathBuf {
    executable.with_file_name(format!("{RUNTIME_BINARY}{}", std::env::consts::EXE_SUFFIX))
}

/// The runtime's own process as the host of a tool's runs. One child serves every run until a fault ends it; the
/// next run then starts a fresh one.
pub struct ProcessHost {
    runtime: Runtime,
    /// The wall-clock safety stop of one run (KN7.10); it is outside the deterministic key (DET.17). None by default.
    pub deadline: Option<Duration>,
}

impl ProcessHost {
    /// A host that starts `program`, the `c600-runtime` binary, on its first run.
    pub fn new(program: impl Into<PathBuf>) -> ProcessHost {
        ProcessHost { runtime: Runtime::new(Config::new(program)), deadline: None }
    }

    /// How many children this host started.
    pub fn spawns(&self) -> u64 {
        self.runtime.spawns()
    }

    /// The operating-system id of the current child.
    pub fn child_id(&self) -> Option<u32> {
        self.runtime.child_id()
    }

    /// Runs the tool's entry on a literal word in the child under `limits` ([`job_under`]), through a boundary onto
    /// `engine`. `source` is the module's source, which must be the one `tool` was admitted from: the built-in column's
    /// admission is this process's ([`Tool::new`]), and the child compiles the same bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn run_in(
        &mut self,
        tool: &Tool,
        source: &str,
        word: &[i32],
        context: Context,
        limits: Amounts,
        engine: &mut dyn Host,
        store: &mut Store,
    ) -> Result<Ran, Stopped> {
        if sha256_hex(source.as_bytes()) != tool.manifest.source_sha256 {
            return Err(Stopped { cause: Cause::OtherSource, shell: Vec::new() });
        }
        let job = job_under(tool, source, word, limits);
        let mut boundary = Boundary::new(engine, store, context);
        let cause = match self.runtime.run(&job, &mut boundary, self.deadline) {
            Ok(Ended::Ran(run)) => return Ok(Ran { run, shell: boundary.shell, captures: boundary.captures }),
            Ok(Ended::Refused(refused)) => Cause::Refused(refused),
            Err(fault) => Cause::Lost(fault),
        };
        Err(Stopped { cause, shell: boundary.shell })
    }

    /// Runs the tool's entry in the child as the job of `command`, as [`run_in_process`] runs it in this process: on
    /// the job's word, as the job's run at the job's revision ([`Tool::context`]) and under the job's budget
    /// ([`ToolCommand::limits`]).
    pub fn run(
        &mut self,
        tool: &Tool,
        source: &str,
        command: &ToolCommand,
        engine: &mut dyn Host,
        store: &mut Store,
    ) -> Result<Ran, Stopped> {
        let context = tool.context(&command.run(), &command.base());
        self.run_in(tool, source, command.word(), context, command.limits(), engine, store)
    }
}

/// The language-defined row through which the solver runs the built-in tool on a word (KN5.3; draft). It is a query
/// row whose one argument, `w`, is the word, and the tool's entry serves it.
pub const TOOL_ROW: &str = "tool.full_600_cell.preview_word";

/// The arguments of [`TOOL_ROW`] for `word`.
pub fn tool_args(word: &[i32]) -> Json {
    json::object([("w", Json::Array(word.iter().map(|&letter| Json::Int(letter.into())).collect()))])
}

/// The word in [`TOOL_ROW`]'s arguments, if they hold one.
fn args_word(args: &Json) -> Option<Vec<i32>> {
    let letters = args.as_object()?.get("w")?.as_array()?;
    letters.iter().map(|letter| letter.as_i64().and_then(|letter| i32::try_from(letter).ok())).collect()
}

/// Serves [`TOOL_ROW`] with `tool`'s entry on `layer`. A layer that has the row keeps it.
pub fn serve_tool<S: c600_command::Store>(layer: &mut CommandLayer<S>, tool: &Tool) -> Result<(), LayerError> {
    if layer.table().row(TOOL_ROW).is_some() {
        return Ok(());
    }
    let overlay = Overlay { module: tool.manifest.module.into(), entry: tool.manifest.entry.into() };
    layer.register_overlay(TOOL_ROW, Kind::Query, &[("w", "Word")], overlay)
}

/// A command the solver invoked to run the tool: the shell's invocation of [`TOOL_ROW`] and the program job the host
/// took for it. The job's run ([`ToolCommand::run`]) is inside the command until the host reports the job
/// ([`ToolCommand::finish`]).
#[derive(Debug)]
pub struct ToolCommand {
    /// The shell's receipt for the invocation: running, with the job.
    pub receipt: Receipt,
    /// The program job the host took.
    pub job: ProgramJob,
    /// The word in the job's arguments, which [`invoke_tool`] checked is the solver's.
    word: Vec<i32>,
}

impl ToolCommand {
    /// The job's run, `job-<id>`: the name the host runs the entry as, which the bridge stamps on the run's invocations.
    pub fn run(&self) -> String {
        self.job.run()
    }

    /// The word the host runs: the one in the job's arguments, [`TOOL_ROW`]'s `w`.
    pub fn word(&self) -> &[i32] {
        &self.word
    }

    /// The revision the run reads, the job's, as its text: the context's and the acknowledgement's `base`.
    pub fn base(&self) -> String {
        crate::bridge::base(&self.job.revision)
    }

    /// The run limits of the job's budget ([`crate::tool::limits`]): the budget the solver's command declared and the
    /// layer admitted under its ceiling (KN7.7), which the host runs the entry under.
    pub fn limits(&self) -> Amounts {
        limits(self.job.budget)
    }

    /// The result of a run that ran: the program's fixed acknowledgement of its one proposal (BND.42), the base the run
    /// read and the proposal's ID, `{"base": "<base>", "id": "<run>/1"}`.
    pub fn acknowledgement(&self) -> Json {
        json::object([("base", Json::String(self.base())), ("id", Json::String(format!("{}/1", self.run())))])
    }

    /// Reports the job with the run's result and returns the receipt of the solver's command. The host reports it after
    /// the preview the run proposed is built, since a run's preview is built only while the host holds its job.
    pub fn finish<S: c600_command::Store>(
        self,
        layer: &mut CommandLayer<S>,
        result: Result<Json, LayerError>,
    ) -> Option<Receipt> {
        layer.finish_program_job(self.job.job, result)
    }

    /// Reports the job of a run that ran, with its [`acknowledgement`](Self::acknowledgement) as the result.
    pub fn acknowledge<S: c600_command::Store>(self, layer: &mut CommandLayer<S>) -> Option<Receipt> {
        let result = self.acknowledgement();
        self.finish(layer, Ok(result))
    }
}

/// The command the solver invokes to run `tool` on `word`: the shell invokes [`TOOL_ROW`] with the word, which
/// [`serve_tool`] serves first if the layer has no such row, and the host takes the program job the layer admitted. The
/// job must carry the word the host runs. A refusal, the shell's or the layer's, or a job that is not the row's, is
/// the error's text.
pub fn invoke_tool<S: c600_command::Store>(
    shell: &mut Shell,
    layer: &mut CommandLayer<S>,
    tool: &Tool,
    word: &[i32],
) -> Result<ToolCommand, String> {
    serve_tool(layer, tool).map_err(|error| format!("{TOOL_ROW} is not served: {}", error.code))?;
    let receipt = shell.invoke(layer, TOOL_ROW, tool_args(word)).map_err(|error| error.code)?;
    let id = match &receipt.status {
        Status::Running(id) => *id,
        other => return Err(format!("{TOOL_ROW} was answered {other:?}")),
    };
    let job = layer.take_program_job().ok_or_else(|| format!("{TOOL_ROW} queued no program job"))?;
    if job.job != id || job.row != TOOL_ROW || args_word(&job.args).as_deref() != Some(word) {
        return Err(format!("the program job {} is not {TOOL_ROW}'s on the word", job.job));
    }
    Ok(ToolCommand { receipt, job, word: word.to_vec() })
}

/// The error a host reports for its job when the run was refused at the start stage because an argument has no
/// transfer encoding ([`Cause::no_transfer`]): the interpreter's non-canonical value code with the refusal's detail.
/// Other causes have no error here.
pub fn start_error(cause: &Cause) -> Option<LayerError> {
    match cause {
        Cause::Refused(refused) if cause.no_transfer() => {
            Some(LayerError::new(code::NONCANONICAL_VALUE, refused.detail.clone()))
        }
        _ => None,
    }
}
