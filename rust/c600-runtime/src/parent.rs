//! The parent side: starts the runtime process, sends it runs and serves its forwarded host calls.
//!
//! [`Runtime::run`] sends one job, then serves the child's calls from the caller's host until the child reports the
//! end of the run. Whatever happens to the child, the parent returns: the run's end, or a [`Fault`] that says what
//! was lost. After a fault the child has been killed and reaped, and the next run starts a fresh one. Three threads
//! per child keep the parent from ever blocking on it: one writes frames to its input, one reads frames from its
//! output and one keeps the end of its standard error. The reader hands over at most [`READ_AHEAD`] frames before it
//! waits, so a child that floods its output holds a bounded number of frames in the parent.
//!
//! Every forwarded call carries an id and the id of the last call whose answer the child received. The child is
//! synchronous: it sends a new call only after the answer to the last one arrived, so a new call must acknowledge
//! that answer and at most one answer is open. The parent keeps the open answer's bytes until the next call or the end
//! of the run acknowledges it, and answers a repeated delivery of the open call, as first sent, from that record
//! instead of calling the host again (KI.5). Anything else is a protocol fault: a new call that does not acknowledge
//! the last answer, a repeated call with another request or acknowledgement, a call already acknowledged or a
//! skipped id.

use std::ffi::OsString;
use std::fmt;
use std::io::{ErrorKind, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use c600_run::interp::{self, Host};
use c600_run::meter::{Interruption, Outcome};

use crate::codec::Codes;
use crate::message::{self, Answer, Forwarded, ToParent, CONTRACT, IR};
use crate::program::{Ended, Job, Refused, Stage};
use crate::wire::{self, Enc, FrameError, MAX_FRAME, PROTOCOL};
use crate::FAULT_VARIABLE;

/// The stop code of a lost runtime (KN7.13, ERR.19): the job outcome is `error`.
pub const RUNTIME_LOST: &str = "KN7:resource_limit.runtime_lost";
/// The stop code of the wall-clock safety stop (KN7.10, ERR.16): the run is cut off and not reproducible.
pub const SAFETY_STOP: &str = "KN7:resource_limit.safety_stop";

/// The bytes of the child's standard error that a fault keeps, from its end.
pub const STDERR_TAIL: usize = 8 << 10;

/// The frames the output reader hands over before it waits for the parent to take one. The child is synchronous,
/// so one is enough.
pub const READ_AHEAD: usize = 1;

/// How to start the runtime process.
#[derive(Clone, Debug)]
pub struct Config {
    /// The `c600-runtime` binary.
    pub program: PathBuf,
    /// Variables set for the child on top of the inherited environment, from which the fault-rehearsal variable is
    /// removed.
    pub env: Vec<(OsString, OsString)>,
    /// How long a new child may take to announce itself.
    pub start_timeout: Duration,
}

impl Config {
    pub fn new(program: impl Into<PathBuf>) -> Config {
        Config { program: program.into(), env: Vec::new(), start_timeout: Duration::from_secs(60) }
    }
}

/// What went wrong with the child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    /// The child did not start or did not announce itself in time.
    Spawn,
    /// The child announced another protocol, IR version or execution contract.
    Incompatible,
    /// The child ended: it exited, was killed, panicked or aborted, or its output closed or broke off.
    Lost,
    /// A frame that does not decode as a message.
    Malformed,
    /// A frame whose length is above [`MAX_FRAME`].
    Oversized,
    /// A message out of order: see the module documentation.
    Protocol,
    /// The run passed its wall-clock deadline.
    Deadline,
}

/// A terminal outcome for a run whose child was lost. The child has been killed and reaped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub kind: FaultKind,
    pub detail: String,
    /// The child's exit status, when it reported one.
    pub exit: Option<i32>,
    /// The end of the child's standard error, at most [`STDERR_TAIL`] bytes.
    pub stderr: String,
}

impl Fault {
    /// The stop code: [`SAFETY_STOP`] for a passed deadline, [`RUNTIME_LOST`] otherwise.
    pub fn code(&self) -> &'static str {
        match self.kind {
            FaultKind::Deadline => SAFETY_STOP,
            _ => RUNTIME_LOST,
        }
    }

    /// The job outcome: cut off for a passed deadline (ERR.16), an error otherwise (ERR.19).
    pub fn outcome(&self) -> Outcome {
        match self.kind {
            FaultKind::Deadline => Outcome::CutOff,
            _ => Outcome::Error,
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?}): {}", self.code(), self.kind, self.detail)
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

enum Event {
    Frame(Vec<u8>),
    End(FrameError),
}

/// The child's input as [`Canceller`]s reach it. It is emptied when the process ends, so that no handle keeps the
/// writer thread and the child's input open.
type CancelInput = Arc<Mutex<Option<Sender<Vec<u8>>>>>;

/// One running child and its threads.
struct Process {
    child: Arc<Mutex<Child>>,
    id: u32,
    input: Sender<Vec<u8>>,
    cancel_input: CancelInput,
    events: Receiver<Event>,
    stderr: Arc<Mutex<Vec<u8>>>,
    stderr_done: Receiver<()>,
    /// The run in progress, 0 between runs; [`Canceller`] reads it.
    current: Arc<AtomicU64>,
}

type Failure = (FaultKind, String);

fn failure(kind: FaultKind, detail: impl Into<String>) -> Failure {
    (kind, detail.into())
}

fn spawn_thread(name: &str, f: impl FnOnce() + Send + 'static) -> bool {
    std::thread::Builder::new().name(name.into()).spawn(f).is_ok()
}

impl Process {
    fn spawn(config: &Config) -> Result<Process, Fault> {
        let mut command = Command::new(&config.program);
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).env_remove(FAULT_VARIABLE);
        for (key, value) in &config.env {
            command.env(key, value);
        }
        let mut child = command.spawn().map_err(|e| Fault {
            kind: FaultKind::Spawn,
            detail: format!("the runtime did not start: {}", e.kind()),
            exit: None,
            stderr: String::new(),
        })?;
        let (Some(mut stdin), Some(mut stdout), Some(mut stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            unreachable!("all three streams are piped");
        };
        let id = child.id();
        let (input, frames) = mpsc::channel::<Vec<u8>>();
        let (sender, events) = mpsc::sync_channel(READ_AHEAD);
        let tail = Arc::new(Mutex::new(Vec::new()));
        let (done, stderr_done) = mpsc::channel();
        let mut process = Process {
            child: Arc::new(Mutex::new(child)),
            id,
            cancel_input: Arc::new(Mutex::new(Some(input.clone()))),
            input,
            events,
            stderr: Arc::clone(&tail),
            stderr_done,
            current: Arc::new(AtomicU64::new(0)),
        };

        let writer = spawn_thread("c600-runtime-writer", move || {
            for frame in frames {
                if wire::write_frame(&mut stdin, &frame).is_err() {
                    break;
                }
            }
        });
        let reader = spawn_thread("c600-runtime-reader", move || loop {
            match wire::read_frame(&mut stdout) {
                Ok(frame) => {
                    if sender.send(Event::Frame(frame)).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = sender.send(Event::End(e));
                    return;
                }
            }
        });
        let keeper = spawn_thread("c600-runtime-stderr", move || {
            let mut buffer = [0u8; 4096];
            loop {
                match stderr.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut tail = lock(&tail);
                        tail.extend_from_slice(&buffer[..n]);
                        if tail.len() > STDERR_TAIL {
                            let cut = tail.len() - STDERR_TAIL;
                            tail.drain(..cut);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
            let _ = done.send(());
        });
        if !(writer && reader && keeper) {
            return Err(process.end(failure(FaultKind::Spawn, "no thread for the runtime's streams")));
        }

        let hello = match process.events.recv_timeout(config.start_timeout) {
            Ok(Event::Frame(frame)) => match message::decode_to_parent(&frame, &Codes::default()) {
                Ok(ToParent::Hello { protocol, ir, contract }) => {
                    if protocol == u32::from(PROTOCOL) && ir == IR && contract == CONTRACT {
                        Ok(())
                    } else {
                        Err(failure(
                            FaultKind::Incompatible,
                            format!("the runtime speaks protocol {protocol}, IR {ir} and contract {contract}"),
                        ))
                    }
                }
                Ok(_) => Err(failure(FaultKind::Protocol, "a message before hello")),
                Err(e) => Err(failure(FaultKind::Malformed, format!("hello: {e}"))),
            },
            Ok(Event::End(e)) => Err(stream_failure(e)),
            Err(RecvTimeoutError::Timeout) => Err(failure(FaultKind::Spawn, "no hello within the start timeout")),
            Err(RecvTimeoutError::Disconnected) => Err(failure(FaultKind::Lost, "the output reader stopped")),
        };
        match hello {
            Ok(()) => Ok(process),
            Err(f) => Err(process.end(f)),
        }
    }

    fn alive(&self) -> bool {
        matches!(lock(&self.child).try_wait(), Ok(None))
    }

    /// Kills and reaps the child and collects what it left on standard error.
    fn end(&mut self, (kind, detail): Failure) -> Fault {
        let exit = {
            let mut child = lock(&self.child);
            let _ = child.kill();
            child.wait().ok().and_then(|status| status.code())
        };
        let _ = self.stderr_done.recv_timeout(Duration::from_secs(5));
        let stderr = String::from_utf8_lossy(&lock(&self.stderr)).into_owned();
        Fault { kind, detail, exit, stderr }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        lock(&self.cancel_input).take();
        let mut child = lock(&self.child);
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn stream_failure(e: FrameError) -> Failure {
    match e {
        FrameError::Closed => failure(FaultKind::Lost, "the runtime's output closed"),
        FrameError::Truncated => failure(FaultKind::Lost, "the runtime's output ended inside a frame"),
        FrameError::Oversized(n) => {
            failure(FaultKind::Oversized, format!("a frame of {n} bytes, above the bound of {MAX_FRAME}"))
        }
        FrameError::Io(e) => failure(FaultKind::Lost, format!("the runtime's output failed: {e}")),
    }
}

/// Kills the child from anywhere, for example from inside a host call; the run in progress ends as
/// [`FaultKind::Lost`].
#[derive(Clone)]
pub struct Killer(Arc<Mutex<Child>>);

impl Killer {
    /// Kills the child and waits until it has exited.
    pub fn kill(&self) {
        let mut child = lock(&self.0);
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Interrupts the run in progress (DET.56, KN7.6). The run stops at its next continuation with the interruption's
/// code, as in process. A handle that outlives its child holds nothing of it.
#[derive(Clone)]
pub struct Canceller {
    input: CancelInput,
    current: Arc<AtomicU64>,
}

impl Canceller {
    /// Sends the interruption and returns at once: `true` when it went to a run in progress, `false` between runs
    /// or after the child is gone, when it does nothing. A run is in progress from the moment its frame is queued
    /// for the child, so the interruption always follows its run.
    pub fn cancel(&self, why: Interruption) -> bool {
        let run = self.current.load(Ordering::SeqCst);
        if run == 0 {
            return false;
        }
        let mut e = Enc::new();
        message::encode_cancel(run, why, &mut e);
        lock(&self.input).as_ref().is_some_and(|input| input.send(e.bytes).is_ok())
    }
}

/// The KI.5 record of one run. The child is synchronous, so at most one answer is open: the last one, until the
/// next call or the end of the run acknowledges it.
#[derive(Default)]
struct Record {
    /// The request digest and encoded answer of call `last`, until the child acknowledges it.
    open: Option<([u8; 32], Vec<u8>)>,
    /// The id of the last call answered, 0 before the first.
    last: u64,
    /// Repeated deliveries answered from the record.
    repeats: u64,
}

impl Record {
    /// `None` for a new call, which must acknowledge the last answer; the recorded answer for a repeated delivery
    /// of the open call, which carries its request and acknowledgement as first sent.
    fn admit(&mut self, call: u64, ack: u64, digest: &[u8; 32]) -> Result<Option<Vec<u8>>, Failure> {
        let protocol = |detail: String| Err(failure(FaultKind::Protocol, detail));
        if call == self.last.saturating_add(1) {
            if ack != self.last {
                return protocol(format!("call {call} acknowledges call {ack}, not the last answer ({})", self.last));
            }
            self.open = None;
            return Ok(None);
        }
        if call < self.last || (call == self.last && ack == call) {
            return protocol(format!("call {call} again after its acknowledgement"));
        }
        if call > self.last {
            return protocol(format!("call {call} skips call {}", self.last.saturating_add(1)));
        }
        match &self.open {
            Some((d, _)) if d != digest => protocol(format!("call {call} again with another request")),
            Some(_) if ack != call - 1 => protocol(format!("call {call} again with another acknowledgement ({ack})")),
            Some((_, answer)) => Ok(Some(answer.clone())),
            None => protocol(format!("call {call} again after its acknowledgement")),
        }
    }
}

fn serve_call(host: &mut dyn Host, forwarded: &Forwarded, codes: &mut Codes) -> Answer {
    let mut refused = |code: &'static str| {
        codes.admit(code);
        Answer::Refused(code.to_string())
    };
    match forwarded {
        Forwarded::Tariff(r) => match host.tariff(&r.as_request()) {
            Ok(t) => Answer::Tariff(t),
            Err(code) => refused(code),
        },
        Forwarded::Perform(r) => match host.perform(&r.as_request()) {
            Ok(v) => Answer::Value(v),
            Err(code) => refused(code),
        },
        Forwarded::Sink(sink, value, bytes) => match host.sink(&sink.as_sink(), value, bytes) {
            Ok(()) => Answer::Delivered,
            Err(code) => refused(code),
        },
    }
}

fn encode_answer(run: u64, call: u64, answer: &Answer) -> Vec<u8> {
    let mut e = Enc::new();
    let problem = match message::encode_answer(run, call, answer, &mut e) {
        Ok(()) if e.bytes.len() <= MAX_FRAME => return e.bytes,
        // A host value outside its canonical form cannot cross.
        Err(_) => interp::code::NONCANONICAL_VALUE,
        // The way back from the host is part of the host.
        Ok(()) => interp::code::HOST_EXHAUSTED,
    };
    let mut e = Enc::new();
    message::encode_answer(run, call, &Answer::Refused(problem.to_string()), &mut e).expect("a refusal always encodes");
    e.bytes
}

/// The parent's handle on the runtime process. It starts a child on first use and after every fault.
pub struct Runtime {
    config: Config,
    process: Option<Process>,
    spawns: u64,
    next_run: u64,
    repeats: u64,
}

impl Runtime {
    pub fn new(config: Config) -> Runtime {
        Runtime { config, process: None, spawns: 0, next_run: 0, repeats: 0 }
    }

    /// The configuration; a change applies from the next child on.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Starts a child unless a live one is waiting.
    pub fn start(&mut self) -> Result<(), Fault> {
        if self.process.as_ref().is_some_and(Process::alive) {
            return Ok(());
        }
        self.process = None;
        self.spawns += 1;
        self.process = Some(Process::spawn(&self.config)?);
        Ok(())
    }

    /// Ends the current child, if any; the next run starts a fresh one.
    pub fn stop(&mut self) {
        self.process = None;
    }

    /// A handle that kills the current child.
    pub fn killer(&self) -> Option<Killer> {
        self.process.as_ref().map(|p| Killer(Arc::clone(&p.child)))
    }

    /// A handle that interrupts the current child's run in progress.
    pub fn canceller(&self) -> Option<Canceller> {
        self.process.as_ref().map(|p| Canceller { input: Arc::clone(&p.cancel_input), current: Arc::clone(&p.current) })
    }

    /// The operating-system id of the current child.
    pub fn child_id(&self) -> Option<u32> {
        self.process.as_ref().map(|p| p.id)
    }

    /// How many children this handle started.
    pub fn spawns(&self) -> u64 {
        self.spawns
    }

    /// How many repeated call deliveries were answered from the KI.5 record, over all runs.
    pub fn repeats(&self) -> u64 {
        self.repeats
    }

    /// Runs `job` in the child and serves its host calls from `host`. `deadline` is the wall-clock safety stop of
    /// the run, measured from its start with host time included; it is not part of the deterministic key (DET.17).
    /// The parent checks it only while it waits for the child's next message: a host call that blocks is not
    /// interrupted, and a deadline it passes is noticed after it returns.
    ///
    /// A job that cannot be sent never reaches the child: arguments without a canonical transfer form are refused
    /// at the start stage with `PS1:input.noncanonical_value`, and a job above the frame bound is refused at the
    /// start stage without a code. In process `Loaded::run` refuses with that code the arguments it has no form
    /// for; a word letter of 0 has a form there but none on the wire.
    pub fn run(&mut self, job: &Job, host: &mut dyn Host, deadline: Option<Duration>) -> Result<Ended, Fault> {
        let started = Instant::now();
        let run = self.next_run + 1;
        let mut e = Enc::new();
        if message::encode_run(run, job, &mut e).is_err() {
            return Ok(Ended::Refused(Refused {
                stage: Stage::Start,
                codes: vec![interp::code::NONCANONICAL_VALUE.to_string()],
                detail: "an argument has no canonical transfer form".into(),
            }));
        }
        if e.bytes.len() > MAX_FRAME {
            return Ok(Ended::Refused(Refused {
                stage: Stage::Start,
                codes: Vec::new(),
                detail: format!("the job takes {} bytes, above the frame bound of {MAX_FRAME}", e.bytes.len()),
            }));
        }
        self.next_run = run;
        self.start()?;
        let mut process = self.process.take().expect("a started child");
        let mut record = Record::default();
        let result = serve(&mut process, run, e.bytes, host, &mut record, started, deadline);
        process.current.store(0, Ordering::SeqCst);
        self.repeats += record.repeats;
        match result {
            Ok(ended) => {
                self.process = Some(process);
                Ok(ended)
            }
            Err(f) => Err(process.end(f)),
        }
    }
}

fn serve(
    process: &mut Process,
    run: u64,
    frame: Vec<u8>,
    host: &mut dyn Host,
    record: &mut Record,
    started: Instant,
    deadline: Option<Duration>,
) -> Result<Ended, Failure> {
    let send = |process: &Process, frame: Vec<u8>| {
        process.input.send(frame).map_err(|_| failure(FaultKind::Lost, "the runtime's input closed"))
    };
    send(process, frame)?;
    // Published only once the run's frame is queued, so a cancel never overtakes its run on the way to the child.
    process.current.store(run, Ordering::SeqCst);
    let mut codes = Codes::default();
    loop {
        let event = match deadline {
            None => process.events.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(d) => process.events.recv_timeout(d.saturating_sub(started.elapsed())),
        };
        let frame = match event {
            Ok(Event::Frame(frame)) => frame,
            Ok(Event::End(e)) => return Err(stream_failure(e)),
            Err(RecvTimeoutError::Timeout) => return Err(failure(FaultKind::Deadline, "the run passed its deadline")),
            Err(RecvTimeoutError::Disconnected) => return Err(failure(FaultKind::Lost, "the output reader stopped")),
        };
        let message = message::decode_to_parent(&frame, &codes).map_err(|e| failure(FaultKind::Malformed, e.0))?;
        match message {
            ToParent::Hello { .. } => return Err(failure(FaultKind::Protocol, "hello during a run")),
            ToParent::Done { run: r, ack, ended } => {
                if r != run {
                    return Err(failure(FaultKind::Protocol, format!("the end of run {r} during run {run}")));
                }
                if ack != record.last {
                    return Err(failure(
                        FaultKind::Protocol,
                        format!("the end of the run acknowledges call {ack} of {}", record.last),
                    ));
                }
                return Ok(ended);
            }
            ToParent::Call { run: r, call, ack, digest, forwarded } => {
                if r != run {
                    return Err(failure(FaultKind::Protocol, format!("a call of run {r} during run {run}")));
                }
                if let Some(answer) = record.admit(call, ack, &digest)? {
                    record.repeats += 1;
                    send(process, answer)?;
                    continue;
                }
                let answer = encode_answer(run, call, &serve_call(host, &forwarded, &mut codes));
                record.open = Some((digest, answer.clone()));
                record.last = call;
                send(process, answer)?;
            }
        }
    }
}
