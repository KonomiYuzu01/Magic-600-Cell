//! Journal travel (EV2.8) and a digest-pinned run's freshness (EV2.10; LP4-6).
//!
//! The debugger reads the session's history through [`JournalRows`], a read-only trait the host implements over its
//! query rows: `journal`, `revision_info`, `checkpoints` and `workspace_records` (KN4.3, LI.14), and the outcome lookup
//! `outcome(command, request)` (KN5.23, LI.30). Every method takes `&self` and the trait has no method that writes, so
//! travel cannot select an active journal line, reset the session, apply an inverse word, write session files or invoke
//! or retry a command.
//!
//! [`Travel`] keeps the event the debugger inspects. Opening an event moves only that cursor. An event the rows no
//! longer hold is `KN2:stale.revision_gone` and leaves the cursor where it was: the debugger never opens the current
//! revision in its place (KN0.6). A row this build does not serve reports its own code and nothing else is read.
//!
//! [`freshness`] says whether a digest-pinned run's input is still the session's head (EV2.10). A pinned run keeps its
//! input across a commit or an undo, and moving the travel cursor does not make its result current: reaching the new
//! revision needs a new run.

use crate::code::REVISION_GONE;
use crate::{DebugError, Pinned};

/// A row's refusal: its own code and detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowError {
    pub code: &'static str,
    pub detail: String,
}

/// One journal event (KN4.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The event's name, `e<order>`.
    pub event: String,
    pub order: u64,
    /// `start`, `commit`, `crossing`, `pose` or `exit`.
    pub kind: String,
    pub letters: u64,
    pub solved: bool,
    pub exit: Option<String>,
}

/// A page of events, newest first, and the cursor that continues it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub entries: Vec<Entry>,
    pub next: Option<String>,
}

/// The head revision's information (KN0.2, LI.14).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionInfo {
    pub session: String,
    pub epoch: u64,
    pub identity: String,
    pub mode: String,
    pub head: String,
    pub state: u64,
    pub workspace: u64,
    pub protection: u64,
    pub digest: String,
}

/// A recovery checkpoint (chapter 08).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub name: String,
    pub event: String,
    pub digest: String,
}

/// A workspace record's head: its name, version and content digest (KN0.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRow {
    pub name: String,
    pub version: u64,
    pub digest: String,
}

/// A stored outcome (KN5.23): the command and request it answers, its status and its job or event, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomeRow {
    pub command: String,
    pub request: String,
    pub status: String,
    pub job: Option<u64>,
    pub event: Option<String>,
}

/// The host's query rows that journal travel reads (see the module's description). Nothing here writes.
pub trait JournalRows {
    /// At most `limit` events from `from` (the head for `None`) toward the start, or the page `cursor` continues.
    fn journal(&self, from: Option<u64>, limit: u64, cursor: Option<&str>) -> Result<Page, RowError>;
    fn revision_info(&self) -> Result<RevisionInfo, RowError>;
    fn checkpoints(&self) -> Result<Vec<Checkpoint>, RowError>;
    fn workspace_records(&self) -> Result<Vec<RecordRow>, RowError>;
    /// The stored answer for a command ID, without invoking or retrying it; `None` when the ID is unknown.
    fn outcome(&self, command: &str, request: &str) -> Result<Option<OutcomeRow>, RowError>;
}

fn row(error: RowError) -> DebugError {
    DebugError::new(error.code, error.detail)
}

/// An opened event: its journal entry and the head the rows named when it was read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visit {
    pub entry: Entry,
    pub head: String,
    /// Whether the opened event is the head.
    pub at_head: bool,
}

/// Journal travel over a host's rows (EV2.8).
pub struct Travel<'r> {
    rows: &'r dyn JournalRows,
    cursor: Option<Visit>,
}

impl<'r> Travel<'r> {
    pub fn new(rows: &'r dyn JournalRows) -> Travel<'r> {
        Travel { rows, cursor: None }
    }

    /// The event the debugger inspects, once one is open.
    pub fn cursor(&self) -> Option<&Visit> {
        self.cursor.as_ref()
    }

    /// Opens the event `order` for inspection. An event the journal no longer holds is `KN2:stale.revision_gone`, with
    /// the cursor unchanged.
    pub fn open(&mut self, order: u64) -> Result<&Visit, DebugError> {
        let head = self.rows.revision_info().map_err(row)?.head;
        let page = self.rows.journal(Some(order), 1, None).map_err(row)?;
        let entry =
            page.entries.into_iter().next().filter(|e| e.order == order).ok_or_else(|| {
                DebugError::new(REVISION_GONE, format!("the journal answered no event e{order} (KN0.6)"))
            })?;
        let at_head = entry.event == head;
        Ok(self.cursor.insert(Visit { entry, head, at_head }))
    }

    /// A page of the journal, as the row answers it.
    pub fn page(&self, from: Option<u64>, limit: u64, cursor: Option<&str>) -> Result<Page, DebugError> {
        self.rows.journal(from, limit, cursor).map_err(row)
    }

    /// The head revision's information.
    pub fn head(&self) -> Result<RevisionInfo, DebugError> {
        self.rows.revision_info().map_err(row)
    }

    pub fn checkpoints(&self) -> Result<Vec<Checkpoint>, DebugError> {
        self.rows.checkpoints().map_err(row)
    }

    pub fn records(&self) -> Result<Vec<RecordRow>, DebugError> {
        self.rows.workspace_records().map_err(row)
    }

    /// Invocation recovery (KN5.23, LI.30): the stored outcome of a command ID, without invoking or retrying it.
    pub fn recover(&self, command: &str, request: &str) -> Result<Option<OutcomeRow>, DebugError> {
        self.rows.outcome(command, request).map_err(row)
    }
}

/// Whether a pinned input is still the session's head state (EV2.10, KN0.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Freshness {
    Current,
    /// The head moved: the pinned run's result keeps its input, `pinned`, and the head is `head`.
    Stale {
        pinned: String,
        head: String,
    },
}

/// The freshness of `pinned` against the head the rows name now.
pub fn freshness(pinned: &Pinned, rows: &dyn JournalRows) -> Result<Freshness, DebugError> {
    let head = rows.revision_info().map_err(row)?.digest;
    Ok(if head == pinned.digest {
        Freshness::Current
    } else {
        Freshness::Stale { pinned: pinned.digest.clone(), head }
    })
}
