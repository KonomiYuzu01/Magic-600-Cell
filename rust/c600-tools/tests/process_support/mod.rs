//! Shared by the process-host, Net, oversize and retry-lead tests: the model, sessions on the command layer, the
//! runtime's binary, and one run of the tool on either host inside the command the solver invokes to run it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use c600_command::{CommandLayer, Receipt, Secret, Shell, Status, Table};
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Kernel, ProtectedSet};
use c600_run::boundary::Store;
use c600_tools::bridge::{Bridge, Sent, Steps};
use c600_tools::process::{
    invoke_tool, run_in_process, ProcessHost, Stopped, ToolCommand, IN_PROCESS, PROCESS, RUNTIME_BINARY,
};
use c600_tools::tool::{Ran, Tool, SOURCE};

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

pub fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    let kernel = KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => Arc::new(kernel),
        Err(error) => panic!("the model in assets/ loads: {error}"),
    });
    kernel.clone()
}

/// The secret every session of these tests shares with its shell.
pub fn secret() -> Secret {
    Secret::fixed([5; 32])
}

/// A layer named `name` at the solved state with `protected` orbits, and its shell. ADR-D01.9's settle interval is
/// zero here: these tests commit a run's preview on the press after its first frame, and a preview a run prepared waits
/// out the interval (c600-command `tests/settle.rs` covers the wait).
pub fn session_with(name: &str, protected: ProtectedSet) -> (CommandLayer, Shell) {
    let kernel = kernel();
    let solved = kernel.solved();
    let mut layer = CommandLayer::new(kernel, Table::h08(), secret(), Default::default(), name, solved, protected);
    layer.set_settle(Duration::ZERO);
    (layer, Shell::new(secret(), "shell"))
}

/// A layer named `name` at the solved state with no protected orbit, and its shell.
pub fn session(name: &str) -> (CommandLayer, Shell) {
    session_with(name, ProtectedSet::none())
}

/// The head digest, the head event, the state revision and the number of durable transactions.
pub fn fingerprint(layer: &CommandLayer) -> (String, String, u64, usize) {
    let revision = layer.revision();
    (layer.snapshot().digest().to_owned(), revision.head.clone(), revision.state, layer.store().log.len())
}

/// The program's whole output: the fixed acknowledgement of its one proposal.
pub fn acknowledgement(base: &str, run: &str) -> Vec<u8> {
    format!("{{\"base\":\"{base}\",\"id\":\"{run}/1\"}}\n").into_bytes()
}

/// A recipe in the 0.4 form with one word step per chunk.
pub fn chunked(chunks: &[&[i64]]) -> Json {
    let step = |moves: &&[i64]| {
        json::object([
            ("kind", Json::String("word".into())),
            ("moves", Json::Array(moves.iter().map(|&letter| Json::Int(letter)).collect())),
        ])
    };
    Json::Array(chunks.iter().map(step).collect())
}

/// The `c600-runtime` binary. A test of this package gets no path to another package's binary from cargo, so the
/// first call builds it with the cargo that runs the test, in the test's profile, and takes the path cargo reports.
pub fn runtime() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    let built = BUILT.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let mut command = Command::new(cargo);
        command.current_dir(repository().join("rust"));
        command.args(["build", "--locked", "--offline", "-p", "c600-runtime", "--bin", RUNTIME_BINARY]);
        command.arg("--message-format=json");
        if !cfg!(debug_assertions) {
            command.arg("--release");
        }
        let out = command.output().expect("cargo starts");
        assert!(out.status.success(), "cargo builds the runtime's binary: {}", String::from_utf8_lossy(&out.stderr));
        let mut executable = out.stdout.split(|&byte| byte == b'\n').filter_map(|line| {
            let message = json::parse(line).ok()?;
            let name = message.get("target")?.get("name")?.as_str()?;
            let reason = message.get("reason")?.as_str()?;
            let executable = message.get("executable")?.as_str()?;
            (reason == "compiler-artifact" && name == RUNTIME_BINARY).then(|| PathBuf::from(executable))
        });
        executable.next_back().expect("cargo reports the runtime's executable")
    });
    assert!(built.is_file(), "the runtime's binary exists");
    built.clone()
}

/// A process host on the built runtime.
pub fn process_host() -> ProcessHost {
    ProcessHost::new(runtime())
}

/// The two hosts' names, in-process first.
pub const HOSTS: [&str; 2] = [IN_PROCESS, PROCESS];

/// The host named `host` for [`run_on`]: `None` in process, `process` otherwise.
pub fn on<'p>(host: &str, process: &'p mut ProcessHost) -> Option<&'p mut ProcessHost> {
    (host == PROCESS).then_some(process)
}

/// Runs `tool` on `word` through a bridge onto `layer` inside the command the solver invokes to run it (finding B2 of
/// the KN5 boundary attack, ONE.34): `shell` invokes the tool's row and the host takes the job, whose run, `job-<id>`,
/// the entry runs as. In process when `host` is `None` and in the runtime's child otherwise. Returns the run, or why
/// the host left none, the bridge's invocations and the command, which the caller reports ([`report`]) once the
/// layer has built the run's preview: a preview is built only while the host holds the run's job.
pub fn run_on(
    host: Option<&mut ProcessHost>,
    tool: &Tool,
    layer: &mut CommandLayer,
    shell: &mut Shell,
    word: &[i32],
) -> (Result<Ran, Stopped>, Vec<Sent>, ToolCommand) {
    run_with(host, tool, layer, shell, word, None)
}

/// [`run_on`] through a bridge given `steps`, the solver's word steps of the run's input recipe.
pub fn run_with(
    host: Option<&mut ProcessHost>,
    tool: &Tool,
    layer: &mut CommandLayer,
    shell: &mut Shell,
    word: &[i32],
    steps: Option<Steps>,
) -> (Result<Ran, Stopped>, Vec<Sent>, ToolCommand) {
    let command = invoke_tool(shell, layer, tool, word).unwrap_or_else(|error| panic!("the tool's row: {error}"));
    let run = command.run();
    let mut bridge = Bridge::new(layer, &run).with_steps(steps).with_budget(command.job.budget);
    let mut store = Store::default();
    let ran = match host {
        None => run_in_process(tool, &command, &mut bridge, &mut store),
        Some(host) => host.run(tool, SOURCE, &command, &mut bridge, &mut store),
    };
    (ran, bridge.sent, command)
}

/// Reports the job of the solver's command once its run ran, with the program's fixed acknowledgement as the result;
/// returns the command's receipt, which is completed.
pub fn report(layer: &mut CommandLayer, command: ToolCommand) -> Receipt {
    let receipt = command.acknowledge(layer).expect("the layer holds the tool's job");
    assert!(matches!(receipt.status, Status::Completed(_)), "the tool's command: {receipt:?}");
    receipt
}
