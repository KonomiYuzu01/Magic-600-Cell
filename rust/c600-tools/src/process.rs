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
//! The two hosts differ in one place. A word with a letter outside 1 to 1200 in magnitude has no transfer encoding,
//! so its job is refused at the start stage and never reaches the child or the engine side ([`Cause::Refused`]); in
//! process the same word runs, and the command layer refuses its proposal.

use std::path::{Path, PathBuf};
use std::time::Duration;

use c600_check::Profile;
use c600_digest::sha256_hex;
use c600_run::boundary::{Boundary, Context, Shown, Store};
use c600_run::interp::{code, Host};
use c600_run::value::Value;
use c600_runtime::{Config, Ended, Fault, Job, JobOptions, Refused, Runtime, Stage};

use crate::tool::{Ran, Tool, LIMITS};

/// The name of the in-process host in records: the tool's program runs on `c600-run` inside the caller's process.
pub const IN_PROCESS: &str = "in-process";
/// The name of the process host in records: the tool's program runs in `c600-runtime`'s child process.
pub const PROCESS: &str = "process";
/// The file stem of the runtime's binary.
pub const RUNTIME_BINARY: &str = "c600-runtime";

/// Why a run on the process host left no interpreter run.
#[derive(Debug)]
pub enum Cause {
    /// The source handed over is not the one the tool was admitted from. Nothing was sent.
    OtherSource,
    /// The job was refused before its first metered operation: by the parent at the start stage (a word with no
    /// transfer encoding, or a job above the frame bound) or by the child's compile stage.
    Refused(Refused),
    /// The child was lost or passed its deadline; it has been killed and reaped (KN7.13).
    Lost(Fault),
}

impl Cause {
    /// Whether the job was refused because an argument has no transfer encoding: the parent's start-stage refusal
    /// with the interpreter's non-canonical value code. The job never reached the child.
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

/// A run on the process host that left no interpreter run. `shell` still holds what the boundary recorded for each
/// proposal that reached it before the run stopped.
#[derive(Debug)]
pub struct Stopped {
    pub cause: Cause,
    pub shell: Vec<Shown>,
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
