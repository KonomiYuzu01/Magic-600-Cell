//! The child side: the runtime process's main loop and its proxy host.
//!
//! The child writes `hello`, then runs one job at a time. Each job goes through [`run_in_process`] with a proxy host
//! that forwards every `tariff`, `perform` and `sink` call to the parent as one `call` message and blocks until its
//! `answer`. A reader thread drains standard input: it records each run's number as it reads the `run`, before the
//! main loop takes the run, hands `run` and `answer` messages to the main loop and applies `cancel` at once. So a
//! cancel that follows its run is never lost, and an interruption reaches the interpreter before the answer that
//! follows it.
//! The child exits when its input ends (the parent is gone) and exits with a message on standard error when the
//! parent breaks the protocol. Standard output carries frames only.

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};

use c600_run::interp::{self, Cancel, Host, Request, RowTariff, Sink};
use c600_run::meter::Interruption;
use c600_run::value::Value;

use crate::codec::{self, STATIC_CODES};
use crate::message::{self, Answer, Call, ToChild};
use crate::program::run_in_process;
use crate::wire::{self, Enc, FrameError};

/// Exit status when the parent breaks the protocol or a reply cannot be written.
const PROTOCOL_EXIT: i32 = 2;

/// The interruption of the current run: the run number in the high bits, the pending interruption in the low two.
/// A `cancel` for another run does not touch it.
struct Interrupt(AtomicU64);

impl Interrupt {
    /// Called by the input thread as it reads a `run`, before the main loop can take it, so a `cancel` read after
    /// the `run` always finds the run's number here.
    fn start(&self, run: u64) {
        self.0.store(run << 2, Ordering::SeqCst);
    }

    fn cancel(&self, run: u64, why: Interruption) {
        let set = (run << 2) | u64::from(codec::interruption_code(why));
        let _ = self.0.compare_exchange(run << 2, set, Ordering::SeqCst, Ordering::SeqCst);
    }
}

impl Cancel for Interrupt {
    fn pending(&self) -> Option<Interruption> {
        codec::interruption_from((self.0.load(Ordering::SeqCst) & 3) as u8).ok()
    }
}

fn die(why: &str) -> ! {
    let _ = writeln!(io::stderr(), "c600-runtime: {why}");
    std::process::exit(PROTOCOL_EXIT)
}

/// Host refusal codes as the interpreter needs them, `&'static str`. Known codes are reused; at most
/// [`INTERNED_CODES`] others are kept for the life of the process.
const INTERNED_CODES: usize = 1024;

fn intern(code: &str) -> &'static str {
    if let Some(c) = STATIC_CODES.iter().copied().find(|c| *c == code) {
        return c;
    }
    static TABLE: OnceLock<Mutex<Vec<&'static str>>> = OnceLock::new();
    let mut table = TABLE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(c) = table.iter().copied().find(|c| *c == code) {
        return c;
    }
    if table.len() >= INTERNED_CODES || code.len() > 256 {
        die("the parent sent more distinct refusal codes than the child keeps");
    }
    let leaked: &'static str = Box::leak(code.to_string().into_boxed_str());
    table.push(leaked);
    leaked
}

/// The child's end of the pipes and its per-run call counters.
struct Link {
    out: io::StdoutLock<'static>,
    rx: Receiver<ToChild>,
    /// Answers received over the life of the process, for the fault rehearsals.
    answers: u64,
    #[cfg(debug_assertions)]
    rehearsal: Option<rehearsal::Plan>,
    #[cfg(debug_assertions)]
    last_call: Option<(u64, u64, Vec<u8>)>,
}

impl Link {
    fn send(&mut self, payload: &[u8]) {
        if wire::write_frame(&mut self.out, payload).is_err() {
            // The parent is gone; nobody is left to answer.
            std::process::exit(0);
        }
    }

    fn receive(&mut self) -> ToChild {
        match self.rx.recv() {
            Ok(message) => message,
            Err(_) => std::process::exit(0),
        }
    }
}

/// The interpreter's host inside the child: every call goes to the parent.
struct Proxy<'a> {
    link: &'a mut Link,
    run: u64,
    next_call: u64,
    /// The last call whose answer arrived (KI.5's acknowledgement).
    ack: u64,
}

impl Proxy<'_> {
    fn call(&mut self, call: Call) -> Result<Answer, &'static str> {
        let id = self.next_call + 1;
        #[cfg(debug_assertions)]
        let ack = self.rehearsed_ack();
        #[cfg(not(debug_assertions))]
        let ack = self.ack;
        let mut e = Enc::new();
        if message::encode_call(self.run, id, ack, &call, &mut e).is_err() {
            // A value outside its canonical form or a type beyond the transfer bound cannot cross; the run stops
            // as for a value without a canonical form.
            return Err(interp::code::NONCANONICAL_VALUE);
        }
        if e.bytes.len() > wire::MAX_FRAME {
            // The way to the host is part of the host: a call above the frame bound exhausts it.
            return Err(interp::code::HOST_EXHAUSTED);
        }
        self.next_call = id;
        #[cfg(debug_assertions)]
        self.rehearse_before_send(id, &e.bytes);
        self.link.send(&e.bytes);
        let answer = self.await_answer(id);
        #[cfg(debug_assertions)]
        let answer = self.rehearse_after_answer(id, &e.bytes, answer);
        self.ack = id;
        self.link.answers += 1;
        Ok(answer)
    }

    fn await_answer(&mut self, id: u64) -> Answer {
        match self.link.receive() {
            ToChild::Answer { run, call, answer } if run == self.run && call == id => answer,
            ToChild::Answer { .. } => die("an answer to a call that is not waiting"),
            ToChild::Run { .. } => die("a run while a call waits for its answer"),
            ToChild::Cancel { .. } => die("a cancel reached the main loop"),
        }
    }
}

fn refused(answer: Answer) -> &'static str {
    match answer {
        Answer::Refused(code) => intern(&code),
        _ => die("an answer of another kind than the call"),
    }
}

impl Host for Proxy<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match self.call(Call::Tariff(request))? {
            Answer::Tariff(t) => Ok(t),
            other => Err(refused(other)),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match self.call(Call::Perform(request))? {
            Answer::Value(v) => Ok(v),
            other => Err(refused(other)),
        }
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        match self.call(Call::Sink(sink, value, bytes))? {
            Answer::Delivered => Ok(()),
            other => Err(refused(other)),
        }
    }
}

/// Runs the child: `hello`, then jobs until standard input ends.
pub fn main() {
    let mut out = io::stdout().lock();
    let mut hello = Enc::new();
    message::encode_hello(&mut hello);
    if wire::write_frame(&mut out, &hello.bytes).is_err() {
        return;
    }

    let interrupt = Arc::new(Interrupt(AtomicU64::new(0)));
    let (tx, rx) = mpsc::channel();
    let reader_interrupt = Arc::clone(&interrupt);
    let reader = std::thread::Builder::new().name("c600-runtime-input".into()).spawn(move || {
        let mut input = io::stdin().lock();
        loop {
            let frame = match wire::read_frame(&mut input) {
                Ok(frame) => frame,
                Err(FrameError::Closed) => std::process::exit(0),
                Err(e) => die(&format!("unreadable input frame: {e:?}")),
            };
            match message::decode_to_child(&frame) {
                Ok(ToChild::Cancel { run, why }) => reader_interrupt.cancel(run, why),
                Ok(message) => {
                    if let ToChild::Run { run, .. } = &message {
                        reader_interrupt.start(*run);
                    }
                    if tx.send(message).is_err() {
                        return;
                    }
                }
                Err(e) => die(&format!("malformed input message: {e}")),
            }
        }
    });
    if reader.is_err() {
        die("no input thread");
    }

    let mut link = Link {
        out,
        rx,
        answers: 0,
        #[cfg(debug_assertions)]
        rehearsal: rehearsal::plan(),
        #[cfg(debug_assertions)]
        last_call: None,
    };
    loop {
        let (run, job) = match link.receive() {
            ToChild::Run { run, job } => (run, job),
            _ => die("an answer outside a run"),
        };
        // The input thread started the run's interruption when it read the run.
        let mut proxy = Proxy { link: &mut link, run, next_call: 0, ack: 0 };
        let ended = run_in_process(&job, &mut proxy, Some(&*interrupt));
        let ack = proxy.ack;
        #[cfg(debug_assertions)]
        proxy.rehearse_before_send(ack + 1, &[]);
        let mut e = Enc::new();
        if message::encode_done(run, ack, &ended, &mut e).is_err() {
            die("a result without a transfer encoding");
        }
        if e.bytes.len() > wire::MAX_FRAME {
            die("a result above the frame bound");
        }
        link.send(&e.bytes);
    }
}

/// Fault rehearsals for the parent's tests. They exist only in debug builds; a release build has no trace of them
/// and ignores the variable.
#[cfg(debug_assertions)]
mod rehearsal {
    use std::time::Duration;

    use c600_ir::Ty;
    use c600_run::interp::Request;

    use super::{die, Answer, Call, Proxy};
    use crate::message;
    use crate::wire::{self, Enc, MAX_FRAME};
    use crate::FAULT_VARIABLE;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Kind {
        /// Panics.
        Panic,
        /// Aborts the process.
        Abort,
        /// Exits with status 3.
        Exit,
        /// Exits with status 0 before the result: standard output closes on a frame boundary.
        Close,
        /// Writes part of a frame, then exits.
        Truncate,
        /// Writes a frame that is not a message, then waits.
        Garbage,
        /// Writes a length prefix above the frame bound, then waits.
        Oversized,
        /// Waits forever.
        Hang,
        /// Sends every call twice from this point on, as after a lost answer (KI.5).
        Duplicate,
        /// Sends the last call again although its answer arrived and was acknowledged.
        Stale,
        /// Sends another request under the last call's id.
        Changed,
        /// Sends every new call with an acknowledgement of 0 from this point on, as a child that keeps no record of
        /// the answers it received.
        Unacked,
    }

    #[derive(Clone, Copy, Debug)]
    pub struct Plan {
        pub kind: Kind,
        /// The rehearsal starts at the first message after this many answers.
        pub after: u64,
    }

    /// Reads `C600_RUNTIME_FAULT=<kind>@<answers>`.
    pub fn plan() -> Option<Plan> {
        let text = std::env::var(FAULT_VARIABLE).ok()?;
        let (kind, after) = text.split_once('@')?;
        let kind = match kind {
            "panic" => Kind::Panic,
            "abort" => Kind::Abort,
            "exit" => Kind::Exit,
            "close" => Kind::Close,
            "truncate" => Kind::Truncate,
            "garbage" => Kind::Garbage,
            "oversized" => Kind::Oversized,
            "hang" => Kind::Hang,
            "duplicate" => Kind::Duplicate,
            "stale" => Kind::Stale,
            "changed" => Kind::Changed,
            "unacked" => Kind::Unacked,
            _ => return None,
        };
        Some(Plan { kind, after: after.parse().ok()? })
    }

    fn wait_forever() -> ! {
        loop {
            std::thread::sleep(Duration::from_secs(3600));
        }
    }

    impl Proxy<'_> {
        fn due(&self) -> Option<Kind> {
            let plan = self.link.rehearsal?;
            (self.link.answers >= plan.after).then_some(plan.kind)
        }

        /// The acknowledgement a new call carries: under `unacked`, 0 instead of the last answered call.
        pub(super) fn rehearsed_ack(&self) -> u64 {
            if self.due() == Some(Kind::Unacked) {
                0
            } else {
                self.ack
            }
        }

        /// Called before a call or the result is sent; `payload` is empty for the result.
        pub(super) fn rehearse_before_send(&mut self, id: u64, payload: &[u8]) {
            let Some(kind) = self.due() else { return };
            match kind {
                Kind::Panic => panic!("c600-runtime fault rehearsal: panic"),
                Kind::Abort => std::process::abort(),
                Kind::Exit => std::process::exit(3),
                Kind::Close => std::process::exit(0),
                Kind::Truncate => {
                    use std::io::Write;
                    let _ = self.link.out.write_all(&100u32.to_le_bytes());
                    let _ = self.link.out.write_all(&[1, 2, 3]);
                    let _ = self.link.out.flush();
                    std::process::exit(0);
                }
                Kind::Garbage => {
                    let _ = wire::write_frame(&mut self.link.out, b"\xffnot a message");
                    wait_forever();
                }
                Kind::Oversized => {
                    use std::io::Write;
                    let _ = self.link.out.write_all(&(MAX_FRAME as u32 + 1).to_le_bytes());
                    let _ = self.link.out.flush();
                    wait_forever();
                }
                Kind::Hang => wait_forever(),
                Kind::Stale | Kind::Changed => {
                    let Some((last, last_ack, mut bytes)) = self.link.last_call.clone() else { return };
                    if kind == Kind::Stale {
                        // The last call again, with an acknowledgement that names it.
                        bytes[18..26].copy_from_slice(&last.to_le_bytes());
                    } else {
                        // A request no program makes, so its digest differs from the original's.
                        let mut e = Enc::new();
                        let other =
                            Request::FileRead { path: "c600-runtime rehearsal: changed request", ty: &Ty::Unit };
                        if message::encode_call(self.run, last, last_ack, &Call::Tariff(&other), &mut e).is_err() {
                            die("rehearsal encoding");
                        }
                        bytes = e.bytes;
                    }
                    self.link.send(&bytes);
                    // The parent must refuse; it stops this process.
                    let _ = self.link.receive();
                    die("the parent answered a stale or changed call");
                }
                Kind::Duplicate | Kind::Unacked => {}
            }
            if !payload.is_empty() {
                self.link.last_call = Some((id, self.ack, payload.to_vec()));
            }
        }

        /// Under `duplicate`, sends the call again as if its answer were lost, and checks that the repeated
        /// answer is identical.
        pub(super) fn rehearse_after_answer(&mut self, id: u64, payload: &[u8], answer: Answer) -> Answer {
            if self.due() != Some(Kind::Duplicate) {
                if !payload.is_empty() {
                    self.link.last_call = Some((id, self.ack, payload.to_vec()));
                }
                return answer;
            }
            self.link.send(payload);
            let again = self.await_answer(id);
            let bytes = |a: &Answer| {
                let mut e = Enc::new();
                message::encode_answer(self.run, id, a, &mut e).map(|_| e.bytes).ok()
            };
            if bytes(&answer) != bytes(&again) {
                die("a repeated call got a different answer");
            }
            again
        }
    }
}
