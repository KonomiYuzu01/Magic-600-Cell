//! Brief SF2 (27 Nov 2026): the key path off the frame thread ([`crate::m6h_key_post`], [`crate::m6h_key_poll`]).
//!
//! A posted key waits in the host's queue in press order. When no batch is in flight, the host hands the session's
//! [`Turns`] and every waiting input to its one key worker thread, which routes them in order exactly as the
//! synchronous [`crate::m6h_key_down`] does (the armed row first, at the head the key will twist, then the key, then
//! the layer's jobs) and hands `Turns` back with one [`Done`] per input. Only the frame thread applies a `Done` (the
//! adoption it defers, the answer it queues for the poll), so nothing the frames show moves before its commit answer
//! exists. While `Turns` is away, every call that needs the session waits for the batch, a call that writes the
//! session also routes the inputs still waiting first, and a presentation report waits in [`Keys::reports`]. Nothing
//! here changes what the session writes or its order.

use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use c600_command::PresentedFrame as LayerFrame;
use c600_display::plan::TransformInput;

use crate::turns::{Head, Press, Turns};

/// One posted input, in press order.
pub(crate) enum Input {
    /// A key-down; `arm` is the `(cap, item)` whose certified row the shell sweeps, read just before the key.
    Down {
        chord: String,
        repeat: bool,
        arm: Option<(u32, u64)>,
    },
    Up(String),
    FocusLost,
}

pub(crate) struct Posted {
    pub ticket: u64,
    pub input: Input,
}

/// What the worker did with one posted input.
pub(crate) struct Done {
    pub ticket: u64,
    /// True for a key-down, which the poll always answers; a key-up or focus loss is answered only when it failed.
    pub down: bool,
    pub press: Result<Press, String>,
    /// The head after a committed press or another head change, which the frame thread defers for adoption.
    pub head: Option<Result<Head, String>>,
    /// The armed row, read at the head before the key.
    pub row: Option<Result<Option<TransformInput>, String>>,
}

/// An applied input's answer, waiting for [`crate::m6h_key_poll`].
pub(crate) struct Answer {
    pub ticket: u64,
    pub code: i32,
    pub error: String,
    pub press: u32,
    pub engine_revision: u64,
    /// The armed row and the engine revision handed to the head it was read at.
    pub row: Option<(TransformInput, u64)>,
}

type Batch = (Turns, Vec<Posted>);
type Back = (Turns, Vec<Done>);

/// The key worker: one thread, started with the first batch, that routes each batch in order.
struct Worker {
    to: Sender<Batch>,
    from: Receiver<Back>,
}

impl Worker {
    fn start() -> Option<Worker> {
        let (to, inbox) = channel::<Batch>();
        let (outbox, from) = channel::<Back>();
        let run = move || {
            while let Ok((mut turns, batch)) = inbox.recv() {
                let done = batch.into_iter().map(|posted| route(&mut turns, posted)).collect();
                if outbox.send((turns, done)).is_err() {
                    return;
                }
            }
        };
        std::thread::Builder::new().name("turn-keys".to_owned()).spawn(run).ok()?;
        Some(Worker { to, from })
    }
}

/// The host's key queue.
#[derive(Default)]
pub(crate) struct Keys {
    next: u64,
    worker: Option<Worker>,
    /// Whether a batch is with the worker (the session's `Turns` with it).
    flying: bool,
    /// Posted inputs not handed to the worker yet.
    pub waiting: VecDeque<Posted>,
    /// Applied answers, in press order.
    pub ready: VecDeque<Answer>,
    /// The newest polled answer's row, which [`crate::m6h_key_row`] writes.
    pub row: Option<(u64, TransformInput)>,
    /// Presentation reports made while `Turns` was away, delivered in order when it is back.
    pub reports: Vec<LayerFrame>,
}

impl Keys {
    pub fn post(&mut self, input: Input) -> u64 {
        self.next += 1;
        self.waiting.push_back(Posted { ticket: self.next, input });
        self.next
    }

    pub fn flying(&self) -> bool {
        self.flying
    }

    /// Whether a batch is in flight or an input is still waiting.
    pub fn busy(&self) -> bool {
        self.flying || !self.waiting.is_empty()
    }

    /// Hands `turns` and every waiting input to the worker; gives `turns` back when the worker cannot take it.
    pub fn launch(&mut self, turns: Turns) -> Result<(), Turns> {
        if self.worker.is_none() {
            self.worker = Worker::start();
        }
        let Some(worker) = &self.worker else { return Err(turns) };
        let batch: Vec<Posted> = self.waiting.drain(..).collect();
        match worker.to.send((turns, batch)) {
            Ok(()) => {
                self.flying = true;
                Ok(())
            }
            Err(error) => {
                let (turns, batch) = error.0;
                self.waiting = batch.into_iter().collect();
                self.worker = None;
                Err(turns)
            }
        }
    }

    /// The batch's outcome: `Ok(None)` while it runs and `wait` is false; an error when the worker died with it.
    pub fn land(&mut self, wait: bool) -> Result<Option<Back>, String> {
        if !self.flying {
            return Ok(None);
        }
        let Some(worker) = &self.worker else { return Err("the key worker is gone".to_owned()) };
        let back = if wait {
            worker.from.recv().map_err(|_| "the key worker stopped with the session".to_owned())?
        } else {
            match worker.from.try_recv() {
                Ok(back) => back,
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => return Err("the key worker stopped with the session".to_owned()),
            }
        };
        self.flying = false;
        Ok(Some(back))
    }
}

/// Routes one posted input as the synchronous calls do.
pub(crate) fn route(turns: &mut Turns, posted: Posted) -> Done {
    match posted.input {
        Input::Down { chord, repeat, arm } => {
            let row = arm.map(|(cap, item)| turns.twist_transform(cap, item));
            let press = turns.key_down(&chord, repeat);
            let head = match press {
                Ok(Press::Committed | Press::Other) => Some(turns.head()),
                _ => None,
            };
            turns.run_jobs();
            Done { ticket: posted.ticket, down: true, press, head, row }
        }
        Input::Up(chord) => {
            let press = turns.key_up(&chord).map(|()| Press::None);
            Done { ticket: posted.ticket, down: false, press, head: None, row: None }
        }
        Input::FocusLost => {
            let press = turns.focus_lost().map(|()| Press::None);
            Done { ticket: posted.ticket, down: false, press, head: None, row: None }
        }
    }
}
