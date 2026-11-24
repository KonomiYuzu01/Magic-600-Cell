//! The host a debugged run sees (EV2.1, EV2.9): reads pass to the host behind it; previews and sinks never do.
//!
//! [`ReadOnly`] answers a preview's tariff, and any sink, with `EV2:capability.debug_write_forbidden`, so the operation
//! is charged its step only and the run stops there with that code, at that site, before anything reaches the engine,
//! the session or the boundary's store. Snapshot, declassified, file and clock reads pass through: the host behind
//! decides what it grants, and a stepped run's host is the pinned input it inspects. The wrapper keeps each refused
//! effect, by kind and target, for the debugger to show.

use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::value::Value;

use crate::code::DEBUG_WRITE_FORBIDDEN;

/// An effect a debugged run attempted and the debugger refused: its kind and its target (a preview target, a
/// workspace record, a file path or an exact role).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub kind: &'static str,
    pub target: String,
}

/// A host that passes reads to `host` and refuses every write (see the module's description).
pub struct ReadOnly<'h> {
    host: &'h mut dyn Host,
    refused: Vec<Refused>,
}

impl<'h> ReadOnly<'h> {
    pub fn new(host: &'h mut dyn Host) -> ReadOnly<'h> {
        ReadOnly { host, refused: Vec::new() }
    }

    /// The effects refused so far, in order.
    pub fn refused(&self) -> &[Refused] {
        &self.refused
    }

    fn refuse(&mut self, kind: &'static str, target: String) -> &'static str {
        self.refused.push(Refused { kind, target });
        DEBUG_WRITE_FORBIDDEN
    }
}

/// The kind and target of a preview request, or `None` for a read.
fn preview(request: &Request) -> Option<(&'static str, String)> {
    match request {
        Request::Propose { target, .. } => Some(("preview", (*target).to_owned())),
        Request::ProposeMenu { target, .. } => Some(("preview_menu", (*target).to_owned())),
        _ => None,
    }
}

impl Host for ReadOnly<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match preview(request) {
            Some((kind, target)) => Err(self.refuse(kind, target)),
            None => self.host.tariff(request),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match preview(request) {
            Some((kind, target)) => Err(self.refuse(kind, target)),
            None => self.host.perform(request),
        }
    }

    fn sink(&mut self, sink: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        let (kind, target) = match sink {
            Sink::View => ("view", String::new()),
            Sink::Workspace { record, .. } => ("workspace", (*record).to_owned()),
            Sink::File { path } => ("file", (*path).to_owned()),
            Sink::Exact { role } => ("exact", format!("{role:?}")),
        };
        Err(self.refuse(kind, target))
    }
}
