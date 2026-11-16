//! The store: the journal file and the content-addressed state objects on a [`Disk`].
//!
//! One durable action is one frame, appended and synced before the engine replies (R-06). When the append or its sync
//! fails, the store cuts the journal back to its last durable length and syncs that. Only a cut that succeeds proves
//! the action absent; any other failure leaves the action's fate unknown, and the session turns read-only (KN5.23,
//! ONE.70). A state object is written and synced before the frame that cites it, so a failed object write leaves the
//! journal as it was.

use std::io;

use crate::disk::Disk;
use crate::journal::{self, Tail};

pub(crate) const JOURNAL: &str = "journal";

/// Why a write did not become durable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fate {
    /// The action is proved absent.
    RolledBack,
    /// Nothing proves whether the action is durable.
    Unknown,
}

pub(crate) struct Store {
    disk: Box<dyn Disk>,
    /// The durable length of the journal.
    len: u64,
    /// The reads made so far (R-13 counts them).
    pub(crate) reads: u64,
}

/// What opening found.
pub(crate) struct Opened {
    pub store: Store,
    pub records: Vec<Vec<u8>>,
    /// The offset of a damaged frame; the records stop before it.
    pub damaged: Option<u64>,
    /// What follows the last whole frame, when there is a journal.
    tail: Option<Tail>,
}

impl Opened {
    /// Makes what was read durable before anything uses it: a frame written but not synced before a restart is synced
    /// now. A torn final frame, which a crash during an append leaves, is cut off first. A damaged journal is left as
    /// it is.
    pub(crate) fn settle(&mut self) -> io::Result<()> {
        match self.tail {
            None | Some(Tail::Damaged(_)) => Ok(()),
            Some(tail) => {
                if tail == Tail::Torn {
                    self.store.disk.truncate(JOURNAL, self.store.len)?;
                }
                self.store.disk.sync(JOURNAL)
            }
        }
    }
}

impl Store {
    /// Reads the journal and settles it (see [`Opened::settle`]).
    pub(crate) fn open(disk: Box<dyn Disk>) -> io::Result<Opened> {
        let mut opened = Store::read_journal(disk)?;
        opened.settle()?;
        Ok(opened)
    }

    /// Reads the journal and writes nothing.
    pub(crate) fn read_journal(disk: Box<dyn Disk>) -> io::Result<Opened> {
        let mut store = Store { disk, len: 0, reads: 0 };
        let Some(bytes) = store.read(JOURNAL)? else {
            return Ok(Opened { store, records: Vec::new(), damaged: None, tail: None });
        };
        let scan = journal::scan(&bytes);
        store.len = scan.end;
        let damaged = match scan.tail {
            Tail::Damaged(at) => Some(at),
            _ => None,
        };
        Ok(Opened { store, records: scan.payloads, damaged, tail: Some(scan.tail) })
    }

    pub(crate) fn read(&mut self, name: &str) -> io::Result<Option<Vec<u8>>> {
        self.reads += 1;
        self.disk.read(name)
    }

    /// Appends one record as a frame and syncs it.
    pub(crate) fn append(&mut self, payload: &[u8]) -> Result<(), Fate> {
        let bytes = journal::frame(payload);
        let written = self.disk.append(JOURNAL, &bytes).and_then(|()| self.disk.sync(JOURNAL));
        match written {
            Ok(()) => {
                self.len += bytes.len() as u64;
                Ok(())
            }
            Err(_) => match self.disk.truncate(JOURNAL, self.len).and_then(|()| self.disk.sync(JOURNAL)) {
                Ok(()) => Err(Fate::RolledBack),
                Err(_) => Err(Fate::Unknown),
            },
        }
    }

    /// Writes and syncs a state object. A failure leaves the journal untouched, so the action that needed the object
    /// is proved absent; an uncited partial object is harmless and is written again when it is next needed.
    pub(crate) fn put_object(&mut self, name: &str, bytes: &[u8]) -> Result<(), Fate> {
        let path = format!("objects/{name}");
        self.disk.create(&path, bytes).and_then(|()| self.disk.sync(&path)).map_err(|_| Fate::RolledBack)
    }

    pub(crate) fn get_object(&mut self, name: &str) -> io::Result<Option<Vec<u8>>> {
        self.read(&format!("objects/{name}"))
    }
}
