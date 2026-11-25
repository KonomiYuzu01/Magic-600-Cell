//! Driving a gated run one advance at a time (chapter 04 EV2.12; LP4-5).
//!
//! `c600-run`'s [`Loaded::run_gated`](c600_run::interp::Loaded::run_gated) stops before each operation at its
//! LG4/DET.20 boundary and calls a [`Gate`] there, on the thread that runs it; the interpreter starts no thread of its
//! own (CT-DET-21). [`Stepper`] runs such a run on a thread of the debugger's and drives its gate:
//!
//! - An advance admits the one operation after the current stop and runs to the next boundary, to the run's end, or to
//!   the dispatch of a host row, which [`Line::dispatching`] reports. A dispatched row is a suspended kernel call: the
//!   advance answers [`Stepped::Waiting`] at the call's awaiting stop, with the call site and no return value. When the
//!   call completes, the same advance reaches the continuation, [`Stepped::Stopped`] with the call's completed result,
//!   or the run's end; it never dispatches the call again, and the stepper does not go on past the continuation.
//! - Requests (EV2.12). A request names the stop it expects and a run-local request ID. Only one advance is in flight.
//!   A duplicate ID returns its latest result and advances nothing, so it neither dispatches nor charges anything again.
//!   A new ID that names the current stop advances once; one that names the awaiting stop of the call in flight joins
//!   that call, also once it has completed and until the run advances again; any other stop is obsolete. An ID used
//!   with another stop is refused. A request at the end changes nothing.
//!
//! # Drafts (LP4-5)
//!
//! - **Stop IDs.** A boundary's is `<position>/<locator>`, a dispatched call's awaiting stop `<call>/await`, where the
//!   call is the position of the host row that dispatched it, and the end's `end`.
//! - **Suspended calls.** Every host row a stepped run performs is treated as a suspended kernel call; a sink
//!   (publication, workspace or file write, exact hand-over) is not one. Admission and its tariff stay before the row
//!   (DET.23), and the call's charges are the row's: waiting adds none.
//! - **Interruption.** [`Stepper::interrupt`] stops the run at its next boundary (KN7.6): at once when it is stopped, and
//!   after a call in flight completes otherwise, so a cancellation keeps the coverage the run really completed.
//! - **Threads.** The run's thread owns the run and its host; the stepper holds only channels. A dropped stepper
//!   interrupts the run at its next boundary and does not wait for a call in flight.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;

use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::Interruption;
use c600_run::step::{Boundary, Gate};
use c600_run::value::Value;

/// A suspended call's answer: the call (the position of the host row that dispatched it) and what the host gave.
#[derive(Clone, Debug)]
pub struct Completed {
    pub call: u128,
    pub result: Result<Value, &'static str>,
}

/// Where a stepped run is.
#[derive(Clone, Debug)]
pub enum Stepped {
    /// At a boundary; after a suspended call, with that call's completed result.
    Stopped { stop: Arc<Boundary>, completed: Option<Arc<Completed>> },
    /// A host row was dispatched at `site`'s operation and has not completed. `call` is the row's position.
    Waiting { call: u128, site: Arc<Boundary> },
    /// The run ended; after a suspended call, with that call's completed result.
    Ended { completed: Option<Arc<Completed>> },
}

impl Stepped {
    /// The stop's ID (see "Stop IDs").
    pub fn id(&self) -> String {
        match self {
            Stepped::Stopped { stop, .. } => stop.id(),
            Stepped::Waiting { call, .. } => format!("{call}/await"),
            Stepped::Ended { .. } => "end".to_owned(),
        }
    }
}

/// Why a request was refused (EV2.12's `EV2:stale.debug_stop`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stale {
    /// The request names a stop that is not the run's current one.
    Obsolete,
    /// The request ID was used before with another expected stop.
    Reused,
}

enum Event<R> {
    Stopped(Boundary),
    Dispatched,
    Answered(Result<Value, &'static str>),
    Ended(R),
}

enum Control {
    Advance,
    Interrupt(Interruption),
}

fn interruption(code: u8) -> Option<Interruption> {
    match code {
        1 => Some(Interruption::ByUser),
        2 => Some(Interruption::RevisionReplaced),
        _ => None,
    }
}

fn flag(why: Interruption) -> u8 {
    match why {
        Interruption::ByUser => 1,
        Interruption::RevisionReplaced => 2,
    }
}

/// The run's side of a stepped run: the [`Gate`] it passes to [`Loaded::run_gated`](c600_run::interp::Loaded::run_gated), and the host
/// wrapper that reports each dispatched host row ([`Line::dispatching`]).
pub struct Line<R> {
    events: Sender<Event<R>>,
    controls: Receiver<Control>,
    interrupt: Arc<AtomicU8>,
}

impl<R: Send> Line<R> {
    /// `host` with each performed row reported to the stepper as a suspended call, and its answer after it.
    pub fn dispatching<'h>(&self, host: &'h mut dyn Host) -> Dispatching<'h, R> {
        Dispatching { host, events: self.events.clone() }
    }
}

impl<R: Send> Gate for Line<R> {
    fn before(&mut self, boundary: Boundary) -> Option<Interruption> {
        if let Some(why) = interruption(self.interrupt.load(Ordering::SeqCst)) {
            return Some(why);
        }
        if self.events.send(Event::Stopped(boundary)).is_err() {
            return Some(Interruption::ByUser);
        }
        match self.controls.recv() {
            Ok(Control::Advance) => None,
            Ok(Control::Interrupt(why)) => Some(why),
            Err(_) => Some(Interruption::ByUser),
        }
    }
}

/// A host whose performed rows a stepped run reports as suspended calls ([`Line::dispatching`]). Tariffs and sinks
/// pass through unreported.
pub struct Dispatching<'h, R> {
    host: &'h mut dyn Host,
    events: Sender<Event<R>>,
}

impl<R: Send> Host for Dispatching<'_, R> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.host.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let _ = self.events.send(Event::Dispatched);
        let answer = self.host.perform(request);
        let _ = self.events.send(Event::Answered(answer.clone()));
        answer
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.host.sink(sink, value, bytes)
    }
}

struct Asked {
    expected: String,
    result: Stepped,
}

/// A gated run on a thread of its own, advanced one operation per request (EV2.12; see the module's description).
/// `R` is what the run's closure returns, such as the interpreter's [`Run`](c600_run::interp::Run).
pub struct Stepper<R> {
    events: Receiver<Event<R>>,
    controls: Sender<Control>,
    interrupt: Arc<AtomicU8>,
    thread: Option<JoinHandle<()>>,
    state: Stepped,
    outcome: Option<R>,
    /// The answer of the call in flight, once its host gave it.
    answered: Option<Arc<Completed>>,
    requests: HashMap<String, Asked>,
    /// The requests whose result is the call in flight: the one that advanced and those that joined it.
    attached: Vec<String>,
    /// The awaiting stop of the call that completed last, while the run has not advanced since.
    completed: Option<String>,
}

impl<R: Send + 'static> Stepper<R> {
    /// Starts `run` on a thread of its own and waits for its first stop, or its end when it stops before any
    /// operation. `run` passes its [`Line`] to [`Loaded::run_gated`](c600_run::interp::Loaded::run_gated) as the gate, and its host through
    /// [`Line::dispatching`].
    pub fn spawn<F>(run: F) -> Stepper<R>
    where
        F: FnOnce(&mut Line<R>) -> R + Send + 'static,
    {
        let (events_out, events) = mpsc::channel();
        let (controls, controls_in) = mpsc::channel();
        let interrupt = Arc::new(AtomicU8::new(0));
        let mut line = Line { events: events_out, controls: controls_in, interrupt: Arc::clone(&interrupt) };
        let thread = std::thread::Builder::new()
            .name("c600-step".into())
            .spawn(move || {
                let ended = run(&mut line);
                let _ = line.events.send(Event::Ended(ended));
            })
            .expect("a stepping thread starts");
        let mut stepper = Stepper {
            events,
            controls,
            interrupt,
            thread: Some(thread),
            state: Stepped::Ended { completed: None },
            outcome: None,
            answered: None,
            requests: HashMap::new(),
            attached: Vec::new(),
            completed: None,
        };
        stepper.receive(None, true);
        stepper
    }

    /// The run's current stop.
    pub fn state(&self) -> &Stepped {
        &self.state
    }

    /// What the run's closure returned, once it ended.
    pub fn outcome(&self) -> Option<&R> {
        self.outcome.as_ref()
    }

    /// One request (EV2.12): advances the run once when `expected` names its current boundary, joins the call in
    /// flight when `expected` names that call's awaiting stop, and otherwise refuses. A duplicate `request` returns its
    /// latest result and advances nothing.
    pub fn step(&mut self, expected: &str, request: &str) -> Result<Stepped, Stale> {
        self.poll();
        if let Some(asked) = self.requests.get(request) {
            return if asked.expected == expected { Ok(asked.result.clone()) } else { Err(Stale::Reused) };
        }
        let result = if expected == self.state.id() {
            match &self.state {
                Stepped::Stopped { stop, .. } => {
                    let site = Arc::clone(stop);
                    self.advance(site)
                }
                Stepped::Waiting { .. } | Stepped::Ended { .. } => self.state.clone(),
            }
        } else if self.completed.as_deref() == Some(expected) {
            self.state.clone()
        } else {
            return Err(Stale::Obsolete);
        };
        if let Stepped::Waiting { .. } = result {
            self.attached.push(request.to_owned());
        }
        self.requests.insert(request.to_owned(), Asked { expected: expected.to_owned(), result: result.clone() });
        Ok(result)
    }

    /// Takes the completion of a call in flight if it has arrived, without waiting and without advancing.
    pub fn poll(&mut self) -> &Stepped {
        if let Stepped::Waiting { .. } = self.state {
            let awaiting = self.state.id();
            while matches!(self.state, Stepped::Waiting { .. }) && self.receive(None, false) {}
            if !matches!(self.state, Stepped::Waiting { .. }) {
                self.complete(awaiting);
            }
        }
        &self.state
    }

    /// Waits until a call in flight completes, without advancing past its continuation.
    pub fn wait(&mut self) -> &Stepped {
        if let Stepped::Waiting { .. } = self.state {
            let awaiting = self.state.id();
            while matches!(self.state, Stepped::Waiting { .. }) {
                self.receive(None, true);
            }
            self.complete(awaiting);
        }
        &self.state
    }

    /// Stops the run at its next boundary with `why` (KN7.6): at once when it is stopped there, after the call in
    /// flight otherwise. Nothing happens once it ended.
    pub fn interrupt(&mut self, why: Interruption) {
        self.interrupt.store(flag(why), Ordering::SeqCst);
        if let Stepped::Stopped { .. } = self.state {
            self.completed = None;
            self.attached.clear();
            let _ = self.controls.send(Control::Interrupt(why));
            while !matches!(self.state, Stepped::Ended { .. }) {
                self.receive(None, true);
            }
        }
    }

    fn advance(&mut self, site: Arc<Boundary>) -> Stepped {
        self.completed = None;
        self.attached.clear();
        self.answered = None;
        let _ = self.controls.send(Control::Advance);
        self.receive(Some(site), true);
        self.state.clone()
    }

    /// Takes the run's next event: blocking, or only one that has arrived. `site` is the boundary an advance left,
    /// which names a call it dispatches. Returns whether an event was taken.
    fn receive(&mut self, site: Option<Arc<Boundary>>, block: bool) -> bool {
        loop {
            let event =
                if block { self.events.recv().map_err(|_| TryRecvError::Disconnected) } else { self.events.try_recv() };
            match event {
                Ok(Event::Stopped(stop)) => {
                    let completed = self.answered.take();
                    self.state = Stepped::Stopped { stop: Arc::new(stop), completed };
                }
                Ok(Event::Dispatched) => match &site {
                    Some(site) => self.state = Stepped::Waiting { call: site.step + 1, site: Arc::clone(site) },
                    // A row dispatched with no stop before it has no call site; the run goes on to its next event.
                    None => continue,
                },
                Ok(Event::Answered(result)) => {
                    let call = match &self.state {
                        Stepped::Waiting { call, .. } => *call,
                        _ => site.as_ref().map_or(0, |s| s.step + 1),
                    };
                    self.answered = Some(Arc::new(Completed { call, result }));
                    continue;
                }
                Ok(Event::Ended(outcome)) => {
                    self.outcome = Some(outcome);
                    let completed = self.answered.take();
                    self.state = Stepped::Ended { completed };
                }
                Err(TryRecvError::Empty) => return false,
                Err(TryRecvError::Disconnected) => self.lost(),
            }
            return true;
        }
    }

    /// The run's thread ended without its end event: its closure panicked. The panic goes on in the caller.
    fn lost(&mut self) -> ! {
        let thread = self.thread.take().expect("the stepping thread is joined once");
        match thread.join() {
            Err(panic) => std::panic::resume_unwind(panic),
            Ok(()) => panic!("the stepping thread ended without its end event"),
        }
    }

    fn complete(&mut self, awaiting: String) {
        self.completed = Some(awaiting);
        let result = self.state.clone();
        for request in self.attached.drain(..) {
            if let Some(asked) = self.requests.get_mut(&request) {
                asked.result = result.clone();
            }
        }
    }
}

impl<R> Drop for Stepper<R> {
    fn drop(&mut self) {
        self.interrupt.store(flag(Interruption::ByUser), Ordering::SeqCst);
    }
}
