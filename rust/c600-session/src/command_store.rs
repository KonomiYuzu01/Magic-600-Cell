//! The command layer's durable store over KN4's journal: M6's durable semantics under c600-command's KN5 (chapter 01
//! section 6).
//!
//! [`JournalStore`] implements [`Store`] and [`Reopen`] on a [`Disk`]. Every record is one journal frame, appended and
//! synced before the layer adopts what it records (R-06); the session's start state is a state object, written and
//! synced before the frame that cites it. A failed append that the store cuts back is rolled back; any other failure
//! leaves the fate unknown until the journal is read again, which then decides it (KN5.23, ONE.70).
//! [`c600_command::CommandLayer::open`] replays the history through the kernel: a damaged frame, or a record that does
//! not read, ends the history, and the layer opens in read-only recovery.
//!
//! The record shapes are drafts (format `c600-command-journal-draft-1`, apart from M6's `c600-session-draft-1`; each is
//! one canonical JSON value):
//!
//! - first and once, `{"record": "command_session", "format", "session", "identity", "epoch", "start", "orbits",
//!   "profile"}`, where `start` names the start state's object, which is the state's digest, and `profile` is the
//!   session's profile by BND.15's name (BND.17). A session record without `profile`, as journals written before it
//!   have, reads as `solving`;
//! - `{"record": "command_epoch", "epoch"}` for every epoch a layer opened after the first, so no open draws it again;
//! - `{"record": "command_action", "transaction"}` for every durable action, in [`Transaction::to_json`]'s form. The
//!   receipt is a function of the transaction, so the effect and its receipt are one write (KN5.23).

use std::cell::RefCell;

use c600_command::{
    identity_from_json, identity_json, Error, Fate, Genesis, History, Profile, Reopen, Store, Transaction,
};
use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value};
use c600_kernel::SLOTS;

use crate::disk::Disk;
use crate::journal::MAX_PAYLOAD;
use crate::store::{self as journal, Fate as Written};
use crate::types::code;

/// The record format (a draft).
pub const FORMAT: &str = "c600-command-journal-draft-1";

const SESSION: &str = "command_session";
const EPOCH: &str = "command_epoch";
const ACTION: &str = "command_action";

/// A store for one session of the command layer, on a journal.
pub struct JournalStore(RefCell<Inner>);

struct Inner {
    journal: journal::Store,
    held: Held,
    /// An append whose fate was unknown: the journal is read again before anything relies on what it holds.
    unsettled: bool,
}

/// What the journal holds, in order.
#[derive(Default)]
struct Held {
    genesis: Option<Genesis>,
    epochs: Vec<u64>,
    transactions: Vec<Transaction>,
    /// Where the history stops early: a damaged frame or a record that does not read (KN5.23).
    damaged: Option<Error>,
}

enum Record {
    Session {
        session: String,
        identity: c600_kernel::IdentityBlock,
        epoch: u64,
        start: String,
        orbits: Vec<usize>,
        profile: Profile,
    },
    Epoch(u64),
    Action(Box<Transaction>),
}

fn io(e: std::io::Error) -> Error {
    Error::new(code::DISK_FULL, format!("the session storage failed: {e}"))
}

/// An I/O error at open. A session lock that another opener holds is `KN4:busy.session_locked` (draft). Only the
/// lock answers `WouldBlock`, since a regular file's read, cut or sync never does.
fn open_io(e: std::io::Error) -> Error {
    match e.kind() {
        std::io::ErrorKind::WouldBlock => {
            Error::new(code::SESSION_LOCKED, format!("the session is open elsewhere: {e}"))
        }
        _ => io(e),
    }
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn nat(value: &Value) -> Option<u64> {
    value.as_i64().and_then(|n| u64::try_from(n).ok())
}

/// A state object's bytes: the labels as little-endian 32-bit integers, so its name is the state's digest (as M6).
fn object_bytes(labels: &[i32]) -> Vec<u8> {
    labels.iter().flat_map(|label| label.to_le_bytes()).collect()
}

fn object_labels(bytes: &[u8]) -> Option<Vec<i32>> {
    if bytes.len() != 4 * SLOTS {
        return None;
    }
    Some(bytes.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

/// The canonical bytes of a record, checked to fit a frame and to read back as the same value.
fn checked(value: &Value) -> Result<Vec<u8>, Error> {
    let bytes = value.canonical().into_bytes();
    if bytes.len() > MAX_PAYLOAD {
        return Err(Error::new(code::RECORD_TOO_LARGE, format!("{} bytes", bytes.len())));
    }
    if json::parse(&bytes).ok().as_ref() != Some(value) {
        return Err(Error::new(code::NONCANONICAL_VALUE, "the record does not read back as itself"));
    }
    Ok(bytes)
}

fn session_record(genesis: &Genesis) -> Value {
    json::object([
        ("record", text(SESSION)),
        ("format", text(FORMAT)),
        ("session", text(&genesis.session)),
        ("identity", identity_json(&genesis.identity)),
        ("epoch", Value::Int(genesis.epoch as i64)),
        ("start", text(&genesis.digest)),
        ("orbits", Value::Array(genesis.orbits.iter().map(|&orbit| Value::Int(orbit as i64)).collect())),
        ("profile", text(genesis.profile.name())),
    ])
}

/// Reads one record strictly: canonical bytes, a known record name, every field and no other.
fn read_record(payload: &[u8]) -> Option<Record> {
    let value = json::parse(payload).ok()?;
    (value.canonical().as_bytes() == payload).then_some(())?;
    let fields = value.as_object()?.len();
    match value.get("record")?.as_str()? {
        SESSION if value.get("format")?.as_str()? == FORMAT => {
            let profile = match (fields, value.get("profile")) {
                (7, None) => Profile::Solving,
                (8, Some(name)) => Profile::from_name(name.as_str()?)?,
                _ => return None,
            };
            let orbits = value.get("orbits")?.as_array()?;
            let orbits = orbits.iter().map(|o| nat(o).and_then(|n| usize::try_from(n).ok())).collect::<Option<_>>()?;
            Some(Record::Session {
                session: value.get("session")?.as_str()?.to_owned(),
                identity: identity_from_json(value.get("identity")?)?,
                epoch: nat(value.get("epoch")?)?,
                start: value.get("start")?.as_str()?.to_owned(),
                orbits,
                profile,
            })
        }
        EPOCH if fields == 2 => Some(Record::Epoch(nat(value.get("epoch")?)?)),
        ACTION if fields == 2 => Some(Record::Action(Box::new(Transaction::from_json(value.get("transaction")?)?))),
        _ => None,
    }
}

/// Reads what the journal holds: the session record first, then epochs and actions. The start state's object must be
/// on the disk and match its name. A record that does not read, a second session record or a damaged frame ends the
/// history there.
fn load(store: &mut journal::Store, records: Vec<Vec<u8>>, damaged: Option<u64>) -> Result<Held, Error> {
    let mut records = records.into_iter();
    let mut held = Held::default();
    let Some(first) = records.next() else {
        return match damaged {
            None => Ok(held),
            Some(at) => Err(Error::new(code::PERSISTENCE_FORMAT, format!("the journal is damaged at byte {at}"))),
        };
    };
    let Some(Record::Session { session, identity, epoch, start, orbits, profile }) = read_record(&first) else {
        return Err(Error::new(code::UNKNOWN_SESSION, "the disk holds no command-layer session"));
    };
    let bytes = store.get_object(&start).map_err(io)?;
    let labels = bytes.filter(|bytes| sha256_hex(bytes) == start).and_then(|bytes| object_labels(&bytes));
    let Some(labels) = labels else {
        return Err(Error::new(code::PERSISTENCE_FORMAT, "the start state's object is missing or damaged"));
    };
    held.genesis = Some(Genesis { session, identity, epoch, labels, digest: start, orbits, profile });
    for (index, payload) in records.enumerate() {
        match read_record(&payload) {
            Some(Record::Epoch(epoch)) => held.epochs.push(epoch),
            Some(Record::Action(transaction)) => held.transactions.push(*transaction),
            _ => {
                let detail = format!("journal record {} does not read", index + 2);
                held.damaged = Some(Error::new(c600_command::code::READ_ONLY_RECOVERY, detail));
                return Ok(held);
            }
        }
    }
    if let Some(at) = damaged {
        let detail = format!("the journal is damaged at byte {at}");
        held.damaged = Some(Error::new(c600_command::code::READ_ONLY_RECOVERY, detail));
    }
    Ok(held)
}

impl JournalStore {
    /// A store for a new session on an empty disk; [`c600_command::CommandLayer::new`] writes the session's start
    /// through [`Store::begin`]. A disk that holds a whole or a damaged frame is refused before anything is written;
    /// the torn remnant of a first record whose append did not finish is cut off (as M6's `Session::create`). A disk
    /// whose session lock another opener holds is refused first, with `KN4:busy.session_locked` (draft, KN4.1).
    pub fn create(disk: Box<dyn Disk>) -> Result<JournalStore, Error> {
        let mut opened = journal::Store::read_journal(disk).map_err(open_io)?;
        if !opened.records.is_empty() || opened.damaged.is_some() {
            return Err(Error::new(code::PERSISTENCE_FORMAT, "the disk already holds a journal"));
        }
        opened.settle().map_err(io)?;
        Ok(JournalStore(RefCell::new(Inner { journal: opened.store, held: Held::default(), unsettled: false })))
    }

    /// The store of the session a disk holds, for [`c600_command::CommandLayer::open`]. The journal is settled first: a
    /// torn final frame is cut off and what was written is synced (as M6's `Session::open`). An empty journal opens, and
    /// its history is refused with `KN4:input.unknown_session`. A session whose lock another opener holds is refused
    /// with `KN4:busy.session_locked` (draft) before its journal is read (KN4.1).
    pub fn open(disk: Box<dyn Disk>) -> Result<JournalStore, Error> {
        let opened = journal::Store::open(disk).map_err(open_io)?;
        let mut store = opened.store;
        let held = load(&mut store, opened.records, opened.damaged)?;
        Ok(JournalStore(RefCell::new(Inner { journal: store, held, unsettled: false })))
    }
}

impl Inner {
    /// Reads the journal again after a write whose fate was unknown, so that what it holds decides the fate. A read or
    /// a sync that fails leaves the fate unknown.
    fn settle(&mut self) {
        if !self.unsettled {
            return;
        }
        let Ok((records, damaged)) = self.journal.reread() else { return };
        if let Ok(held) = load(&mut self.journal, records, damaged) {
            self.held = held;
            self.unsettled = false;
        }
    }

    /// Appends one record as one frame and syncs it.
    fn append(&mut self, record: &Value) -> Fate {
        self.settle();
        if self.unsettled {
            return Fate::Unknown;
        }
        if let Some(error) = &self.held.damaged {
            return Fate::RolledBack(error.clone());
        }
        let bytes = match checked(record) {
            Ok(bytes) => bytes,
            Err(error) => return Fate::RolledBack(error),
        };
        match self.journal.append(&bytes) {
            Ok(()) => Fate::Durable,
            Err(Written::RolledBack) => Fate::RolledBack(Error::new(code::DISK_FULL, "the record was not written")),
            Err(Written::Unknown) => {
                self.unsettled = true;
                Fate::Unknown
            }
        }
    }
}

impl Store for JournalStore {
    fn transact(&mut self, transaction: &Transaction) -> Fate {
        let inner = self.0.get_mut();
        if inner.held.genesis.is_none() {
            return Fate::RolledBack(Error::new(code::UNKNOWN_SESSION, "the journal holds no session"));
        }
        let record = json::object([("record", text(ACTION)), ("transaction", transaction.to_json())]);
        let fate = inner.append(&record);
        if fate == Fate::Durable {
            inner.held.transactions.push(transaction.clone());
        }
        fate
    }

    /// Whether the journal holds the action, read again first when an append's fate was unknown; `None` while that
    /// reread fails.
    fn durable(&self, event: &str) -> Option<bool> {
        let mut inner = self.0.borrow_mut();
        inner.settle();
        (!inner.unsettled).then(|| inner.held.transactions.iter().any(|t| t.event == event))
    }

    fn begin(&mut self, genesis: &Genesis) -> Fate {
        let inner = self.0.get_mut();
        inner.settle();
        if inner.held.genesis.is_some() {
            return Fate::RolledBack(Error::new(c600_command::code::SESSION_EXISTS, "the journal holds a session"));
        }
        let bytes = object_bytes(&genesis.labels);
        if sha256_hex(&bytes) != genesis.digest {
            return Fate::RolledBack(Error::new(code::PERSISTENCE_FORMAT, "the start state does not match its digest"));
        }
        let held = matches!(inner.journal.get_object(&genesis.digest), Ok(Some(stored)) if stored == bytes);
        if !held && inner.journal.put_object(&genesis.digest, &bytes).is_err() {
            return Fate::RolledBack(Error::new(code::DISK_FULL, "the start state was not written"));
        }
        let fate = inner.append(&session_record(genesis));
        if fate == Fate::Durable {
            inner.held.genesis = Some(genesis.clone());
        }
        fate
    }

    fn begin_epoch(&mut self, epoch: u64) -> Fate {
        let inner = self.0.get_mut();
        let fate = inner.append(&json::object([("record", text(EPOCH)), ("epoch", Value::Int(epoch as i64))]));
        if fate == Fate::Durable {
            inner.held.epochs.push(epoch);
        }
        fate
    }
}

impl Reopen for JournalStore {
    fn history(&mut self) -> Result<History, Error> {
        let inner = self.0.get_mut();
        inner.settle();
        let Some(genesis) = inner.held.genesis.clone() else {
            return Err(Error::new(code::UNKNOWN_SESSION, "the journal holds no session"));
        };
        Ok(History {
            genesis,
            epochs: inner.held.epochs.clone(),
            transactions: inner.held.transactions.clone(),
            damaged: inner.held.damaged.clone(),
        })
    }
}
