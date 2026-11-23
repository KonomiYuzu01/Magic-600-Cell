//! Stepping RT1's evaluator at the run's own LG4/DET.20 positions (chapter 04 EV2.12; LP4-5).
//!
//! - A [`Boundary`] is the stop before one operation of DET.55: the position the run's meter has reached (its root's
//!   admitted steps, DET.20), the next operation's locator and source range (DET.56, LG4.12), the enclosing call sites
//!   and the current frame's bound slots by their source names. Stepping consumes that position. It keeps no step
//!   counter and no tariff of its own, and a gate that lets every operation through leaves the run's charges, rows and
//!   result exactly as they are without one.
//! - [`crate::interp::Loaded::run_gated`] calls a [`Gate`] at every boundary, before the operation is admitted, on the
//!   caller's thread. For an ordinary call the boundary after the `call` row is the callee's entry, and the one after
//!   `return` is the caller's continuation, since both are operations of the run. An interruption the gate returns
//!   stops the run there, as a pending cancellation does (DET.56, KN7.6).
//! - The interpreter starts no thread (CT-DET-21). A debugger that steps a run one advance at a time drives the gate
//!   from a thread of its own: `c600-debug`'s `stepper`.
//!
//! # Drafts (LP4-5)
//!
//! - **Stop IDs.** A boundary's is `<position>/<locator>` ([`Boundary::id`]). A child scope that absorbs a refused step
//!   resumes in its stop branch at the same position, under another locator, so the IDs stay distinct.
//! - **Locals.** A completed `let` is a local until its scope exits; a loop's value so far is its accumulator's slot.

use crate::meter::{Amounts, Interruption, SourceRange};
use crate::value::Value;

/// Where an operation is: its module, its function and the path of its IR node in the function's body ("root" or
/// dot-joined child indices, as [`crate::interp::SourceRanges`] reads it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub module: String,
    pub function: String,
    pub path: String,
}

/// An enclosing call site: its locator text, its operation and its source range, if the run has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub at: String,
    pub operation: Operation,
    pub source: Option<SourceRange>,
}

/// Which frame slot a local is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Param,
    Capture,
    Let,
}

/// A bound slot of the current frame: a completed immutable local, by its source name.
#[derive(Clone, Debug)]
pub struct Local {
    pub name: String,
    pub slot: Slot,
    pub value: Value,
}

/// The stop before one operation (EV2.12).
#[derive(Clone, Debug)]
pub struct Boundary {
    /// The DET.20 position: the steps the run's root has admitted. The next operation, once admitted, is the step
    /// after it.
    pub step: u128,
    /// The next operation's stop locator (DET.56): its call path and node path, or `entry#…` around the entry call.
    pub at: String,
    /// The next operation's name (DET.55).
    pub op: String,
    /// The next operation's node; `None` for the operations around the entry call.
    pub operation: Option<Operation>,
    /// The next operation's source range, when the run has a source map with one.
    pub source: Option<SourceRange>,
    /// The enclosing call sites, outermost first.
    pub sites: Vec<Site>,
    /// The current frame's bound slots, parameters first, then captures, then `let` slots in slot order.
    pub locals: Vec<Local>,
    /// The root's use so far and its limits (KN7.7); memory in `used` is the peak.
    pub used: Amounts,
    pub limits: Amounts,
    /// The root's live memory now.
    pub live: u128,
    /// The innermost active scope's name.
    pub scope: String,
}

impl Boundary {
    /// The stop's ID: `<position>/<locator>`.
    pub fn id(&self) -> String {
        format!("{}/{}", self.step, self.at)
    }
}

/// What a stepped run calls before each operation (EV2.12). `None` lets the operation go ahead; an interruption stops
/// the run there.
pub trait Gate {
    fn before(&mut self, boundary: Boundary) -> Option<Interruption>;
}
