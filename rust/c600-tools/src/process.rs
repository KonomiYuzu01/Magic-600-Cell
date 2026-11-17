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

use std::path::{Path, PathBuf};
use std::time::Duration;

use c600_check::Profile;
use c600_digest::sha256_hex;
use c600_run::boundary::{Boundary, Context, Shown, Store};
use c600_run::interp::{code, Host};
use c600_run::value::Value;
use c600_runtime::codec::encode_value;
use c600_runtime::wire::Enc;
use c600_runtime::{Config, Ended, Fault, Job, JobOptions, Refused, Runtime, Stage};

use crate::tool::{Ran, Tool, LIMITS};

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

/// Runs the tool's entry on a literal word in this process ([`Tool::run`]) and stops at the start stage, before any
/// host call, where [`ProcessHost::run`] stops. A word the process host cannot send, one with a letter outside 1 to
/// 1200 in magnitude, is refused with the interpreter's non-canonical value code before the tool runs, as the parent
/// refuses its job (`c600_runtime::Runtime::run`); an entry or argument that `Loaded::run` refuses stops there too.
pub fn run_in_process(
    tool: &Tool,
    word: &[i32],
    name: &str,
    base: &str,
    engine: &mut dyn Host,
    store: &mut Store,
) -> Result<Ran, Stopped> {
    let refused = |codes: Vec<String>, detail: String| Stopped {
        cause: Cause::Refused(Refused { stage: Stage::Start, codes, detail }),
        shell: Vec::new(),
    };
    if encode_value(&Value::word(word), &mut Enc::new()).is_err() {
        let detail = "an argument has no canonical transfer form".to_owned();
        return Err(refused(vec![code::NONCANONICAL_VALUE.into()], detail));
    }
    tool.run(word, name, base, engine, store).map_err(|refusal| refused(vec![refusal.code.into()], refusal.detail))
}

/// The job of one run of `tool` on a literal word: the source bytes, the manifest's module and entry, the word as the
/// entry's one argument, [`LIMITS`], the test tariffs and no source ranges, as [`Tool::run_in`] runs it in process.
/// The job's profile is the column `c600_check::admit` applies in the child, tool authoring: that function has no
/// built-in column, and its solving column refuses every previewing program.
pub fn job(tool: &Tool, source: &str, word: &[i32]) -> Job {
    let mut options = JobOptions::new(LIMITS);
    options.profile = Profile::ToolAuthoring;
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

    /// Runs the tool's entry on a literal word in the child, through a boundary onto `engine`. `source` is the
    /// module's source, which must be the one `tool` was admitted from: the built-in column's admission is this
    /// process's ([`Tool::new`]), and the child compiles the same bytes.
    pub fn run_in(
        &mut self,
        tool: &Tool,
        source: &str,
        word: &[i32],
        context: Context,
        engine: &mut dyn Host,
        store: &mut Store,
    ) -> Result<Ran, Stopped> {
        if sha256_hex(source.as_bytes()) != tool.manifest.source_sha256 {
            return Err(Stopped { cause: Cause::OtherSource, shell: Vec::new() });
        }
        let job = job(tool, source, word);
        let mut boundary = Boundary::new(engine, store, context);
        let cause = match self.runtime.run(&job, &mut boundary, self.deadline) {
            Ok(Ended::Ran(run)) => return Ok(Ran { run, shell: boundary.shell, captures: boundary.captures }),
            Ok(Ended::Refused(refused)) => Cause::Refused(refused),
            Err(fault) => Cause::Lost(fault),
        };
        Err(Stopped { cause, shell: boundary.shell })
    }

    /// Runs the entry on a literal word in the solving profile with [`Tool::context`].
    pub fn run(
        &mut self,
        tool: &Tool,
        source: &str,
        word: &[i32],
        name: &str,
        base: &str,
        engine: &mut dyn Host,
        store: &mut Store,
    ) -> Result<Ran, Stopped> {
        self.run_in(tool, source, word, tool.context(name, base), engine, store)
    }
}
