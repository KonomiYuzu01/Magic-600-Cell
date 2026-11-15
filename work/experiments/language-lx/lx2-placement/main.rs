#[allow(dead_code)]
#[path = "../lx6-engine-port/engine.rs"] mod engine;
#[allow(dead_code)]
#[path = "../lx6-engine-port/json.rs"] mod json;
#[allow(dead_code)]
#[path = "../lx6-engine-port/sha256.rs"] mod sha256;
mod faults;
mod kernel;
mod runtime;
mod wire;

use std::{env, fs, io, path::Path, process::{Child, Command, Stdio},
    sync::{Arc, mpsc}, thread, time::Duration};
use json::{J, object, string};
use kernel::Kernel;
use runtime::{KernelPort, PipePort};

const STACK_BYTES: usize = 128 * 1024;
const WATCHDOG_MS: u64 = 1000;

struct Running {
    go: mpsc::Sender<()>, done: mpsc::Receiver<Result<J, String>>,
    thread: Option<thread::JoinHandle<()>>, child: Option<Child>,
}

fn start(kernel: Arc<Kernel>, spec: J, placement: &str, journal: &Path) -> Result<(Running, J), String> {
    let snapshot = kernel.snapshot();
    let benchmark = spec.get("mode")?.text()? == "benchmark";
    let (ready_tx, ready_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let (child, handle, pid) = if placement == "inside_engine" {
        let handle = thread::Builder::new().name("lx2-runtime".into()).stack_size(STACK_BYTES).spawn(move || {
            let mut port = runtime::DirectPort { kernel, snapshot, benchmark, ready: ready_tx, go: go_rx };
            let _ = done_tx.send(runtime::script(&mut port, &spec));
        }).map_err(|_| "cannot start runtime thread")?;
        (None, handle, std::process::id())
    } else if placement == "own_process" {
        let stderr = fs::File::create(journal.with_extension("runtime.stderr")).map_err(|_| "runtime stderr")?;
        let mut command = Command::new(env::current_exe().map_err(|_| "executable path")?);
        command.arg("runtime").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(stderr);
        #[cfg(windows)] {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // No interactive console window.
        }
        let mut child = command.spawn().map_err(|_| "cannot start runtime process")?;
        let pid = child.id();
        let mut input = child.stdout.take().unwrap(); let mut output = child.stdin.take().unwrap();
        wire::write(&mut output, spec.canonical().as_bytes())?;
        let handle = thread::spawn(move || {
            let mut serve = || -> Result<J, String> {
                loop {
                    let request = json::parse(&wire::read(&mut input)?)?;
                    let row = request.get("row")?.text()?;
                    if row == "_done" {
                        let answer = request.get("outcome")?.clone();
                        wire::reply(&mut output, Ok(Vec::new()))?;
                        return Ok(answer);
                    }
                    let result = if row == "_armed" {
                        ready_tx.send(request.get("prefix")?.clone()).map_err(|_| "ready channel")?;
                        go_rx.recv().map_err(|_| "release channel")?;
                        Ok(Vec::new())
                    } else { runtime::dispatch(&kernel, &snapshot, &request, benchmark) };
                    wire::reply(&mut output, result)?;
                }
            };
            let _ = done_tx.send(serve());
        });
        (Some(child), handle, pid)
    } else { return Err("unknown placement".into()); };
    let mut running = Running { go: go_tx, done: done_rx, thread: Some(handle), child };
    match ready_rx.recv_timeout(Duration::from_secs(20)) {
        Ok(prefix) => Ok((running, object([("runtime_pid", json::num(pid)), ("prefix", prefix)]))),
        Err(_) => { running.cleanup(); Err("runtime did not reach the armed barrier".into()) },
    }
}

impl Running {
    fn finish(&mut self, fault: &str, benchmark: bool) -> Result<J, String> {
        let timeout = if benchmark { Duration::from_secs(120) } else { Duration::from_millis(WATCHDOG_MS) };
        let mut timeout_kill = false;
        let received = match self.done.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) if self.child.is_none() => {
                // Rust std cannot terminate a runaway thread. Do not pretend that
                // a timeout report ended this job; observe the session before cleanup.
                return Ok(object([("job_terminated", J::Bool(false)), ("outcome", J::Null),
                    ("observation", string("runtime_thread_still_running"))]));
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if benchmark || fault != "loop_native" { return Err("unexpected runtime timeout".into()); }
                self.child.as_mut().unwrap().kill().map_err(|_| "watchdog kill failed")?;
                timeout_kill = true;
                self.done.recv_timeout(Duration::from_secs(10)).map_err(|_| "runtime did not die")?
            },
            Err(_) => return Err("runtime completion channel failed".into()),
        };
        let outcome = match received {
            Ok(outcome) => outcome,
            Err(_) if self.child.is_some() => object([("status", string("error")),
                ("code", string(if timeout_kill { "runtime.watchdog_killed" } else { "runtime.died" }))]),
            Err(e) => return Err(e),
        };
        if let Some(child) = &mut self.child {
            let exit = child.wait().map_err(|_| "cannot reap runtime")?;
            if outcome.get("code")?.text()?.starts_with("runtime.") && exit.success() {
                return Err("runtime reported death but exited successfully".into());
            }
        }
        if let Some(handle) = self.thread.take() { handle.join().map_err(|_| "runtime host thread panicked")?; }
        Ok(object([("job_terminated", J::Bool(true)), ("outcome", outcome),
            ("observation", string("runtime_ended"))]))
    }

    fn cleanup(&mut self) {
        if let Some(child) = &mut self.child {
            if matches!(child.try_wait(), Ok(None)) { let _ = child.kill(); }
            let _ = child.wait();
        }
        // A direct native loop cannot be joined; process shutdown is explicit cleanup.
    }
}

fn runtime_process() -> Result<(), String> {
    thread::Builder::new().name("lx2-runtime".into()).stack_size(STACK_BYTES).spawn(|| -> Result<(), String> {
        let mut input = io::stdin().lock(); let output = io::stdout().lock();
        let spec = json::parse(&wire::read(&mut input)?)?;
        let mut port = PipePort { input, output };
        let outcome = runtime::script(&mut port, &spec)?;
        port.call(&object([("row", string("_done")), ("outcome", outcome)]))?;
        Ok(())
    }).map_err(|_| "cannot start runtime thread")?.join().map_err(|_| "runtime panicked")?
}

fn kernel_process(args: &[String]) -> Result<(), String> {
    if args.len() != 7 { return Err("expected kernel model pose provenance journal placement".into()); }
    let model = engine::Model::load(&fs::read(&args[2]).map_err(|_| "model read")?)?;
    let pose = fs::read(&args[3]).map_err(|_| "pose read")?;
    let provenance = json::parse(&fs::read(&args[4]).map_err(|_| "provenance read")?)?;
    let journal = Path::new(&args[5]);
    let kernel = Kernel::new(model, pose, provenance, journal)?;
    let mut running: Option<Running> = None;
    let mut fault = String::new(); let mut benchmark = false;
    let mut input = io::stdin().lock(); let mut output = io::stdout().lock();
    let mut serve = || -> Result<(), String> {
        loop {
            let request = json::parse(&wire::read(&mut input)?)?;
            let op = request.get("op")?.text()?;
            if op == "release" {
                // The acknowledgement is flushed before the destructive fault starts.
                wire::reply(&mut output, Ok(b"{\"released\":true}".to_vec()))?;
                running.as_ref().ok_or("no runtime")?.go.send(()).map_err(|_| "release failed")?;
                continue;
            }
            let result = match op {
                "status" => Ok(kernel.status()),
                "preview" => kernel.preview(kernel.snapshot(), request.get("word")?, request.get("provenance")?),
                "commit" => kernel.commit(request.get("preview")?),
                "undo" => kernel.undo(),
                "start" => {
                    if running.is_some() { return Err("one runtime per trial".into()); }
                    let spec = request.get("spec")?.clone();
                    fault = spec.get("fault")?.text()?.to_owned();
                    benchmark = spec.get("mode")?.text()? == "benchmark";
                    let (handle, ready) = start(kernel.clone(), spec, &args[6], journal)?;
                    running = Some(handle); Ok(ready)
                },
                "kill_runtime" => {
                    running.as_mut().ok_or("no runtime")?.child.as_mut().ok_or("driver must kill the shared host")?
                        .kill().map_err(|_| "forced kill failed")?;
                    Ok(object([("killed", J::Bool(true))]))
                },
                "finish" => running.as_mut().ok_or("no runtime")?.finish(&fault, benchmark),
                "shutdown" => { wire::reply(&mut output, Ok(b"null".to_vec()))?; return Ok(()); },
                _ => Err("unknown control command".into()),
            };
            wire::reply(&mut output, result.map(|j| j.canonical().into_bytes()))?;
        }
    };
    let result = serve();
    if let Some(handle) = &mut running { handle.cleanup(); }
    result
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("runtime") => runtime_process(),
        Some("kernel") => kernel_process(&args),
        Some("self-test") => { sha256::self_test(); json::self_test(); Ok(()) },
        _ => Err("expected kernel, runtime or self-test".into()),
    };
    if let Err(error) = result { eprintln!("LX-2: {error}"); std::process::exit(2); }
}
