//! KN5's command layer over M6's journal, as LP3's host sets it up: `c600-command`'s layer on a `JournalStore`, the
//! host's table (H-08 with the layer's draft rows) and C5's provenance reader (`c600_tools::workspace::Reader`) under
//! the fixture's policy pins, with a shell and a headless display.

use std::time::Duration;

use c600_command::{
    ActionHint, CommandLayer, Completion, CompletionKind, Direction, HeadlessFrames, Invocation, Receipt, Secret,
    Shell, Status, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_session::disk::Disk;
use c600_session::JournalStore;
use c600_tools::bridge::word_args;
use c600_tools::workspace::{self, Reader};

use super::{kernel, pins};

/// The per-launch secret of the layers and shells in these tests (a fixed test value).
pub const SECRET: [u8; 32] = [7; 32];

/// One open of a session: the layer over the journal, the shell, the headless display's frames and the host's reader,
/// which the layer holds a clone of.
pub struct Fixture {
    pub layer: CommandLayer<JournalStore>,
    pub shell: Shell,
    pub frames: HeadlessFrames,
    pub reader: Reader,
}

/// A word's letters, as a record's payload and a draft hold them.
pub fn letters(word: &[i32]) -> Value {
    Value::Array(word.iter().map(|&letter| Value::Int(i64::from(letter))).collect())
}

/// A provenance block as `workspace.set` takes it: the block's canonical JSON as text.
pub fn text(block: &[u8]) -> String {
    String::from_utf8(block.to_vec()).expect("a block is UTF-8 text")
}

/// `workspace.set`'s arguments: `word` under `record`, with `block` as the record's provenance.
pub fn record_args(record: &str, word: &[i32], block: &[u8]) -> Value {
    json::object([
        ("payload", letters(word)),
        ("provenance", Value::String(text(block))),
        ("record", Value::String(record.into())),
    ])
}

/// `operation.preview`'s arguments for a draft that names record `record` (`ResolvedDraftRef`, KN5.26).
pub fn record_draft(record: &str) -> Value {
    json::object([("draft", json::object([("record", Value::String(record.into()))]))])
}

/// Whether a receipt is a workspace row's completion.
pub fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

/// The refusal code of a receipt, if it was refused or failed.
pub fn refused(receipt: &Receipt) -> Option<&'static str> {
    receipt.status.code()
}

impl Fixture {
    /// A new session `s1` at the solved state, with no protected orbit, on `disk`, which must hold none.
    pub fn create(disk: Box<dyn Disk>) -> Fixture {
        let k = kernel();
        let store = JournalStore::create(disk).expect("an empty disk takes a session");
        let none = ProtectedSet::none();
        let table = workspace::table();
        Fixture::with(CommandLayer::new(k.clone(), table, Secret::fixed(SECRET), store, "s1", k.solved(), none))
    }

    /// The session `disk` holds, opened under a new layer, which draws a new epoch and replays the journal, with a new
    /// shell, display and reader. The reader keeps no block: a reopen restores the records, not the host's blocks.
    pub fn open(disk: Box<dyn Disk>) -> Fixture {
        let store = JournalStore::open(disk).expect("the journal opens");
        let layer = match CommandLayer::open(kernel(), workspace::table(), Secret::fixed(SECRET), store) {
            Ok(layer) => layer,
            Err(error) => panic!("the session does not reopen: {error:?}"),
        };
        assert!(layer.recovery().is_none(), "{:?}", layer.recovery());
        Fixture::with(layer)
    }

    fn with(mut layer: CommandLayer<JournalStore>) -> Fixture {
        let reader = Reader::new(pins());
        layer.set_provenance(Box::new(reader.clone()));
        // ADR-D01.9's settle interval is tested in c600-command's tests/settle.rs; these sessions commit on the first
        // presented press.
        layer.set_settle(Duration::ZERO);
        Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), reader }
    }

    /// `workspace.set` from the shell: the solver's explicit workspace command.
    pub fn write(&mut self, record: &str, word: &[i32], block: &[u8]) -> Receipt {
        let args = record_args(record, word, block);
        self.shell.invoke(&mut self.layer, RECORD_ROW, args).expect("the shell prepares the write")
    }

    /// `operation.preview` from the shell.
    pub fn ask(&mut self, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, "operation.preview", args).expect("the shell prepares the preview")
    }

    /// Previews the draft in `args`, which must be admitted, builds the preview and presents it through the headless
    /// display's built-in chain; returns the preview's ID.
    pub fn preview(&mut self, args: Value) -> u64 {
        let receipt = self.ask(args);
        let Status::Running(id) = receipt.status else { panic!("the draft was not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        let built = self.layer.outcome(&receipt.command, &receipt.request).expect("the preview's outcome");
        let ready = matches!(built.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. }));
        assert!(ready, "{:?}", built.status);
        self.frames.present(&mut self.shell, &mut self.layer, id).expect("the preview is presented");
        id
    }

    /// The commit chord for preview `id`, prepared and not sent.
    pub fn press(&mut self, id: u64) -> Invocation {
        let args = json::object([("preview", Value::Int(id as i64))]);
        self.shell.prepare(&self.layer, "operation.execute", args).expect("a token for the presented preview")
    }

    /// Previews, presents and commits the literal word `word` from the shell: one state commit.
    pub fn commit(&mut self, word: &[i32]) -> Receipt {
        let id = self.preview(word_args(word));
        let press = self.press(id);
        let receipt = self.shell.send(&mut self.layer, press);
        assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
        receipt
    }

    /// The hint for `direction`, which must name an action.
    pub fn hint(&self, direction: Direction) -> ActionHint {
        self.layer.undo_hint(direction).expect("an eligible target")
    }

    /// `session.undo` or `session.redo` from the shell with `hint`.
    pub fn step(&mut self, direction: Direction, hint: ActionHint) -> Receipt {
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).expect("prepared")
    }

    /// What a reopen keeps: the revision without its epoch, the records and both action hints.
    pub fn kept(&self) -> String {
        let mut revision = self.layer.revision().clone();
        revision.epoch = 0;
        let (undo, redo) = (self.layer.undo_hint(Direction::Undo), self.layer.undo_hint(Direction::Redo));
        format!("{revision:?} {:?} {undo:?} {redo:?}", self.layer.records())
    }
}
