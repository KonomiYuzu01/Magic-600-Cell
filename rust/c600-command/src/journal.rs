//! KN4.3's journal query (LP2 draft for LP3's C8; its final form stays with EN4's command-table freeze): a page of the
//! move tree's events on one line, newest first, from a named event or the head toward the session's start.
//! [`crate::CommandLayer::journal`] reads the move tree, which a reopen rebuilds from the durable journal, so it
//! answers for the events of every epoch, with Jumble on or off. It replays nothing and changes nothing, and its cost
//! is linear in the page and flat in the history (R-13).

use c600_kernel::json::{self, Value};

use crate::token::Secret;

/// The row that serves the query to a program's snapshot read through the host bridge (KN4.3; draft).
pub const JOURNAL_ROW: &str = "journal";

/// The most events one page holds (draft). Every entry has a bounded size, so a page's cost is bounded too.
pub const JOURNAL_PAGE_LIMIT: u64 = 4096;

/// What an event of the move tree did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// The session's start, event `e0`.
    Start,
    /// A commit of a recipe.
    Commit,
    /// `jumble.on`'s crossing into the jumbling puzzle.
    Crossing,
    /// A pose move with Jumble on.
    Pose,
    /// `jumble.off`'s exit back to 600-cell-Full.
    Exit,
}

impl EntryKind {
    /// The kind's name in an answer: `start`, `commit`, `crossing`, `pose` or `exit`.
    pub fn name(self) -> &'static str {
        match self {
            EntryKind::Start => "start",
            EntryKind::Commit => "commit",
            EntryKind::Crossing => "crossing",
            EntryKind::Pose => "pose",
            EntryKind::Exit => "exit",
        }
    }
}

/// One event of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    /// The event's action order; its name is [`event_name`] of it.
    pub order: u64,
    pub kind: EntryKind,
    /// A commit's retained recipe length in signed primitive letters; 0 for every other kind.
    pub letters: u64,
    /// For the start, whether its state is the solved state; false for every other kind.
    pub solved: bool,
    /// An exit's mark: the SHA-256 of its exit record's canonical JSON, the value an `exit_root` entry names, never
    /// the poses (ONE.56); `None` for every other kind.
    pub exit: Option<String>,
}

impl JournalEntry {
    /// The event's name, `e<order>`.
    pub fn event(&self) -> String {
        event_name(self.order)
    }
}

/// A page of events, newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalPage {
    pub entries: Vec<JournalEntry>,
    /// The cursor that continues the walk, or `None` once the page reaches the start.
    pub next: Option<String>,
}

/// An event's name: `e` and its action order in decimal, as a revision's `head` names it.
pub fn event_name(order: u64) -> String {
    format!("e{order}")
}

/// The action order an event's name gives: `e` and a canonical decimal, `0|[1-9][0-9]*`, within `u64`. None for any
/// other text.
pub fn event_order(name: &str) -> Option<u64> {
    let digits = name.strip_prefix('e')?;
    let canonical = match digits.as_bytes() {
        [] | [b'0', _, ..] => false,
        bytes => bytes.iter().all(u8::is_ascii_digit),
    };
    if canonical {
        digits.parse().ok()
    } else {
        None
    }
}

/// The mark that binds a cursor to its session, its walk's `from` (`None` for a walk from the head) and the event it
/// continues at: HMAC-SHA-256 under the launch's secret of their canonical JSON, in lowercase hexadecimal.
fn binding(secret: &Secret, session: &str, from: Option<u64>, next: u64) -> String {
    let from = Value::String(from.map(event_name).unwrap_or_default());
    let fields = [
        ("from", from),
        ("next", Value::String(event_name(next))),
        ("query", Value::String(JOURNAL_ROW.to_owned())),
        ("session", Value::String(session.to_owned())),
    ];
    c600_digest::hex(&secret.mac(json::object(fields).canonical().as_bytes()))
}

/// The cursor that continues the walk from `from` at the event `next`: `e<next>:` and the [`binding`] mark. It grants
/// nothing, since any event can be read as a walk's `from`. The mark keeps a walk on its own line: a cursor of another
/// walk, session or launch is refused, and none can be made without the launch's secret.
pub(crate) fn cursor(secret: &Secret, session: &str, from: Option<u64>, next: u64) -> String {
    format!("{}:{}", event_name(next), binding(secret, session, from, next))
}

/// The event a cursor continues at, if [`cursor`] made it in this launch for this session and this walk's `from`.
pub(crate) fn cursor_next(cursor: &str, secret: &Secret, session: &str, from: Option<u64>) -> Option<u64> {
    let (event, mark) = cursor.split_once(':')?;
    let next = event_order(event)?;
    (mark == binding(secret, session, from, next)).then_some(next)
}
