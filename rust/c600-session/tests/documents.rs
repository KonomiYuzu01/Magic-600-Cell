//! LP4-14's document service over the session store (ADR-D01.79 to .84 as revision 6's R-2 and R-3 amend them; draft):
//! the session's history converted into a log that compiles to the session's state; the live session saved at
//! `app.exit` with its open documents and the current preferences; opening and compiling a log, which change no live
//! state and bind a log only when its compile passed; and `file.log.import` through the command layer on the journal,
//! which needs a passed check of the file's current bytes, writes the recovery checkpoint, keeps the current
//! preferences and is undone by `session.undo`.

mod common;

use std::sync::{Arc, Mutex, PoisonError};

use c600_command::{
    draft_args, CheckedFile, CommandLayer, Completion, CompletionKind, Direction, DocumentKind, Documents, Error,
    HeadlessFrames, ImportPlan, LogFormat, Receipt, Secret, Shell, Status, Table, TransactionKind,
    DOCUMENT_COMPILE_ROW, DOCUMENT_NEW_ROW, DOCUMENT_OPEN_ROW, EXIT_ROW, LOG_CHECK_ROW, LOG_EXPORT_ROW, LOG_IMPORT_ROW,
    SESSION_DOCUMENT, WHOLE_SESSION,
};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_log::{CursorAction, DocumentIdentity, MoveAction};
use c600_session::disk::MemDisk;
use c600_session::documents::{session_log, DocumentStore, MemFiles};
use c600_session::JournalStore;

const SECRET: [u8; 32] = [7; 32];
const SESSION_FILE: &str = "sessions/s1.c600log";
const LOG_FILE: &str = "logs/a.c600log";
/// PS1.2's code for bytes that are no longer the checked ones.
const DIGEST_MISMATCH: &str = "PS1:input.digest_mismatch";

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

/// The document service, shared with the test after the layer took it, so the test can play the host's editor.
#[derive(Clone)]
struct Shared(Arc<Mutex<DocumentStore>>);

impl Shared {
    fn with<T>(&self, f: impl FnOnce(&mut DocumentStore) -> T) -> T {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl Documents for Shared {
    fn create(&mut self, kind: DocumentKind) -> Result<Value, Error> {
        self.with(|s| s.create(kind))
    }
    fn open(&mut self, file: &str) -> Result<Value, Error> {
        self.with(|s| s.open(file))
    }
    fn save(&mut self, document: &str) -> Result<Value, Error> {
        self.with(|s| s.save(document))
    }
    fn save_as(&mut self, document: &str, destination: &str) -> Result<Value, Error> {
        self.with(|s| s.save_as(document, destination))
    }
    fn compile(&mut self, document: &str) -> Result<Value, Error> {
        self.with(|s| s.compile(document))
    }
    fn close(&mut self, document: &str, discard: bool) -> Result<Value, Error> {
        self.with(|s| s.close(document, discard))
    }
    fn check(&mut self, file: &str) -> Result<Value, Error> {
        self.with(|s| s.check(file))
    }
    fn export(&mut self, format: LogFormat, destination: &str) -> Result<Value, Error> {
        self.with(|s| s.export(format, destination))
    }
    fn import_plan(&mut self, checked: &CheckedFile) -> Result<ImportPlan, Error> {
        self.with(|s| s.import_plan(checked))
    }
    fn checkpoint(&mut self) -> Result<String, Error> {
        self.with(|s| s.checkpoint())
    }
    fn exit(&mut self) -> Result<Value, Error> {
        self.with(|s| s.exit())
    }
}

struct Fixture {
    layer: CommandLayer<JournalStore>,
    shell: Shell,
    frames: HeadlessFrames,
    docs: Shared,
}

/// A session at the solved state on a fresh memory disk, with the document service over `files` installed and
/// `preferences` current.
fn fixture(files: &MemFiles, preferences: &str) -> Fixture {
    let k = common::kernel();
    let store = JournalStore::create(Box::new(MemDisk::new())).expect("an empty disk takes a session");
    let reader = store.history_reader();
    let mut layer = CommandLayer::new(
        k.clone(),
        Table::h08(),
        Secret::fixed(SECRET),
        store,
        "s1",
        k.solved(),
        ProtectedSet::none(),
    );
    let mut service = DocumentStore::new(k, Table::h08(), reader, Box::new(files.clone()), SESSION_FILE);
    service.set_preferences(Some(preferences.to_owned()));
    let docs = Shared(Arc::new(Mutex::new(service)));
    layer.set_documents(Box::new(docs.clone()));
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), docs }
}

impl Fixture {
    fn press(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    fn commit(&mut self, letters: &[i64]) {
        let receipt = self.press("operation.preview", draft_args(&word(letters)));
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let receipt = self.press("operation.execute", json::object([("preview", Value::Int(id as i64))]));
        assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    }

    fn step(&mut self, direction: Direction) {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        let receipt = self.press(row, json::object([("hint", hint.to_json())]));
        assert!(matches!(receipt.status, Status::Committed(_)), "{row}: {:?}", receipt.status);
    }

    /// Runs a job row and answers the service's answer that its receipt names.
    fn job(&mut self, row: &str, args: Value) -> Value {
        let started = self.press(row, args);
        assert!(matches!(started.status, Status::Running(_)), "{row}: {:?}", started.status);
        assert_eq!(self.layer.run_jobs(), 1);
        let receipt = self.layer.kernel_outcome(&c600_command::Caller::Shell, &started.command, &started.request);
        self.answer(&receipt)
    }

    fn answer(&self, receipt: &Receipt) -> Value {
        let Status::Completed(Completion { result: Some(result), .. }) = &receipt.status else {
            panic!("not completed: {:?}", receipt.status)
        };
        self.layer.document_result(&result.digest).expect("the answer is kept").clone()
    }

    fn digest(&self) -> String {
        self.layer.snapshot().digest().to_owned()
    }

    /// The labels' digest, the revision and the journal's length.
    fn live(&self) -> (String, c600_command::Revision, usize) {
        let transactions = self.layer.store().clone_history().transactions.len();
        (self.digest(), self.layer.revision().clone(), transactions)
    }
}

/// What the journal holds, read through a reader of the same store.
trait CloneHistory {
    fn clone_history(&self) -> c600_command::History;
}

impl CloneHistory for JournalStore {
    fn clone_history(&self) -> c600_command::History {
        self.history_reader().history().expect("the journal holds the session")
    }
}

/// A compiled log's saved state, when its compile passed.
fn compiled_state(bytes: &[u8]) -> Option<String> {
    let compiled = c600_log::compile(&common::kernel(), bytes);
    compiled.passed().then(|| compiled.state().map(|s| s.digest().to_owned())).flatten()
}

/// LP4-14's conversion: commits, an undo, a new branch, an undo and a redo, and a protection change convert into a log
/// whose commits carry their legal witnesses, whose undo and redo are cursor moves and whose protection change is a
/// workspace node. The log compiles, every gate passed, to the session's state.
#[test]
fn the_session_history_converts_to_a_log_that_compiles_to_the_session_state() {
    let files = MemFiles::new();
    let mut f = fixture(&files, "{}");
    f.commit(&[1]);
    f.commit(&[2, 3]);
    f.step(Direction::Undo);
    f.commit(&[4]);
    f.step(Direction::Undo);
    f.step(Direction::Redo);
    let receipt = f.press("protection.orbit.set", json::object([("orbits", Value::Array(vec![Value::Int(0)]))]));
    assert!(matches!(receipt.status, Status::Completed(_)), "{:?}", receipt.status);

    let history = f.layer.store().clone_history();
    let log = session_log(&common::kernel(), &Table::h08(), &history).unwrap();
    let kinds: Vec<&str> = log.moves.iter().map(|entry| entry.action.kind()).collect();
    assert_eq!(kinds, ["commit", "commit", "cursor", "commit", "cursor", "cursor", "workspace"]);
    let cursors: Vec<CursorAction> = log
        .moves
        .iter()
        .filter_map(|entry| match &entry.action {
            MoveAction::Cursor(cursor) => Some(cursor.action),
            _ => None,
        })
        .collect();
    assert_eq!(cursors, [CursorAction::Undo, CursorAction::Undo, CursorAction::Redo]);
    let bytes = c600_log::write(&log).unwrap();
    assert_eq!(compiled_state(&bytes), Some(f.digest()), "the log compiles to the session's state");
}

/// ADR-D01.79: `app.exit` saves the live session at its file without a prompt, with every open document's contents,
/// a dirty one included, and the current preferences. The saved log compiles to the session's state.
#[test]
fn the_live_session_is_saved_at_exit_with_its_open_documents() {
    let files = MemFiles::new();
    let mut f = fixture(&files, r#"{"theme":"mine"}"#);
    f.commit(&[1, 2]);
    let created = f.press(DOCUMENT_NEW_ROW, json::object([("kind", text("script"))]));
    let id = f.answer(&created).get("document").and_then(Value::as_str).unwrap().to_owned();
    f.docs.with(|s| s.edit(&id, b"// a draft\n")).unwrap();

    let exit = f.press(EXIT_ROW, json::object([]));
    let Status::Completed(Completion { kind, .. }) = &exit.status else { panic!("{:?}", exit.status) };
    assert_eq!(*kind, CompletionKind::App);
    assert_eq!(f.answer(&exit).get("file"), Some(&text(SESSION_FILE)));
    let saved = files.get(SESSION_FILE).expect("the live session is saved");
    assert_eq!(compiled_state(&saved), Some(f.digest()));
    let log = c600_log::read(&saved).unwrap();
    assert_eq!(log.preferences.as_deref(), Some(r#"{"theme":"mine"}"#));
    assert_eq!(log.documents.len(), 1);
    let document = &log.documents[0];
    assert_eq!((&document.identity, document.dirty), (&DocumentIdentity::Untitled(id), true));
    assert_eq!(document.contents, b"// a draft\n");
    // The live session does not close.
    let close = f.press("file.document.close", json::object([("document", text(SESSION_DOCUMENT))]));
    assert_eq!(close.status.code(), Some(c600_command::code::ARGUMENT_FORM));
}

/// ADR-D01.81 and revision 6's R-2: opening and compiling a log change no live state; a compile result names the
/// SHA-256 of the bytes it compiled, and only a log whose compile passed is bound to its saved state.
#[test]
fn opening_and_compiling_a_log_change_no_live_state_and_bind_only_a_passed_log() {
    let files = MemFiles::new();
    let mut a = fixture(&files, "{}");
    a.commit(&[1, 2]);
    let exported = a.job(
        LOG_EXPORT_ROW,
        json::object([
            ("destination", text(LOG_FILE)),
            ("format", text(LogFormat::Log.name())),
            ("history", text(WHOLE_SESSION)),
        ]),
    );
    assert_eq!(exported.get("file"), Some(&text(LOG_FILE)));
    let good = files.get(LOG_FILE).unwrap();
    let mut log = c600_log::read(&good).unwrap();
    log.saved.state = common::kernel().solved().digest().to_owned();
    files.put("logs/bad.c600log", &c600_log::write(&log).unwrap());

    let mut b = fixture(&files, "{}");
    b.commit(&[5]);
    let before = b.live();
    for (file, bound) in [(LOG_FILE, true), ("logs/bad.c600log", false)] {
        let opened = b.press(DOCUMENT_OPEN_ROW, json::object([("file", text(file))]));
        assert_eq!(b.answer(&opened).get("document"), Some(&text(file)));
        let compiled = b.job(DOCUMENT_COMPILE_ROW, json::object([("document", text(file))]));
        let sha = c600_digest::sha256_hex(&files.get(file).unwrap());
        assert_eq!(compiled.get("sha256"), Some(&text(&sha)), "{file}");
        assert_eq!(compiled.get("bound"), Some(&Value::Bool(bound)), "{file}");
        let state = if bound { text(&a.digest()) } else { Value::Null };
        assert_eq!(compiled.get("state"), Some(&state), "{file}");
        assert_eq!(b.live(), before, "{file}");
    }
}

/// ADR-D01.82 on the journal: a log exported from one session is checked and imported into another. The import needs
/// a passed check of the file's current bytes; it writes the recovery checkpoint of the session before it, which
/// compiles to that state; it lands on the log's saved state as one commit; it keeps the current preferences, which
/// the next save records; and `session.undo` returns to the state before it. The session's log after the import
/// still compiles to its state.
#[test]
fn a_log_import_keeps_the_preferences_writes_its_checkpoint_and_undoes() {
    let files = MemFiles::new();
    let mut a = fixture(&files, r#"{"theme":"theirs"}"#);
    a.commit(&[1, 2]);
    a.commit(&[3]);
    let exported = a.docs.with(|s| s.export(LogFormat::Log, LOG_FILE)).unwrap();
    assert_eq!(exported.get("file"), Some(&text(LOG_FILE)));

    let mut b = fixture(&files, r#"{"theme":"mine"}"#);
    b.commit(&[5]);
    let before = b.live();
    let checked = |sha: &str| json::object([("file", text(LOG_FILE)), ("sha256", text(sha))]);
    let import = |sha: &str| json::object([("checked", checked(sha)), ("confirm", Value::Bool(true))]);

    // No check of the file: nothing is written.
    let sha = c600_digest::sha256_hex(&files.get(LOG_FILE).unwrap());
    assert_eq!(b.press(LOG_IMPORT_ROW, import(&sha)).status.code(), Some(c600_command::code::RESULT_GONE));
    // A check of bytes that then change: the import is refused with the digest mismatch.
    let check = b.job(LOG_CHECK_ROW, json::object([("file", text(LOG_FILE))]));
    assert_eq!(check.get("passed"), Some(&Value::Bool(true)));
    assert_eq!(check.get("sha256"), Some(&text(&sha)));
    let good = files.get(LOG_FILE).unwrap();
    files.put(LOG_FILE, b"not a log");
    assert_eq!(b.press(LOG_IMPORT_ROW, import(&sha)).status.code(), Some(DIGEST_MISMATCH));
    // A check that did not pass: refused with its first problem's code.
    let failed = b.job(LOG_CHECK_ROW, json::object([("file", text(LOG_FILE))]));
    assert_eq!(failed.get("passed"), Some(&Value::Bool(false)));
    let bad_sha = c600_digest::sha256_hex(b"not a log");
    let refused = b.press(LOG_IMPORT_ROW, import(&bad_sha));
    assert!(refused.status.code().is_some_and(|code| code.starts_with("PS")), "{:?}", refused.status);
    assert_eq!(b.live(), before, "no refusal writes the journal");
    let checkpoint = format!("{}.checkpoint-1.c600log", SESSION_FILE.trim_end_matches(".c600log"));
    assert!(files.get(&checkpoint).is_none(), "no checkpoint before a refusal");

    files.put(LOG_FILE, &good);
    let receipt = b.press(LOG_IMPORT_ROW, import(&sha));
    assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    assert_eq!(b.digest(), a.digest(), "the import lands on the log's saved state");
    assert_eq!(b.layer.import_checkpoint(), Some(checkpoint.as_str()));
    let saved = files.get(&checkpoint).expect("the recovery checkpoint");
    assert_eq!(compiled_state(&saved), Some(before.0.clone()), "the checkpoint holds the session before the import");
    let history = b.layer.store().clone_history();
    let last = history.transactions.last().unwrap();
    assert_eq!((last.kind, last.row.as_str()), (TransactionKind::Commit, LOG_IMPORT_ROW));
    assert_eq!(b.docs.with(|s| s.preferences().map(str::to_owned)), Some(r#"{"theme":"mine"}"#.to_owned()));
    let session = b.docs.with(|s| s.session()).unwrap();
    assert_eq!(session.preferences.as_deref(), Some(r#"{"theme":"mine"}"#), "the current preferences are kept");
    assert_eq!(compiled_state(&c600_log::write(&session).unwrap()), Some(b.digest()));

    b.step(Direction::Undo);
    assert_eq!(b.digest(), before.0, "session.undo returns to the state before the import");

    // A service started again over the same files overwrites no earlier checkpoint.
    let reader = b.layer.store().history_reader();
    let mut again = DocumentStore::new(common::kernel(), Table::h08(), reader, Box::new(files.clone()), SESSION_FILE);
    assert_eq!(again.checkpoint().unwrap(), checkpoint.replace(".checkpoint-1.", ".checkpoint-2."));
    assert_eq!(files.get(&checkpoint), Some(saved), "the first checkpoint is kept");
}

/// A node that leaves the session's undo and redo stacks alone (a workspace completion of undo class `none`, which the
/// layer journals without touching either stack) can stand between an undo and the redo that repeats it. The log's
/// redo would not go forward from that node, so the conversion writes the cursor move as a `select`, and the log still
/// compiles to the session's state.
#[test]
fn a_redo_past_a_node_off_the_undo_stacks_converts_as_a_select() {
    let files = MemFiles::new();
    let mut f = fixture(&files, "{}");
    f.commit(&[1]);
    f.step(Direction::Undo);
    f.step(Direction::Redo);
    let mut history = f.layer.store().clone_history();
    assert_eq!(history.transactions.len(), 3);
    let mut completion = history.transactions[1].clone();
    completion.kind = TransactionKind::Workspace;
    completion.order = 1000;
    completion.target = None;
    completion.row = "tool.fixture.inspect".into();
    completion.job = Some(1);
    completion.result = Some("r".into());
    history.transactions.insert(2, completion);
    let log = session_log(&common::kernel(), &Table::h08(), &history).unwrap();
    let cursors: Vec<CursorAction> = log
        .moves
        .iter()
        .filter_map(|entry| match &entry.action {
            MoveAction::Cursor(cursor) => Some(cursor.action),
            _ => None,
        })
        .collect();
    assert_eq!(cursors, [CursorAction::Undo, CursorAction::Select]);
    assert_eq!(compiled_state(&c600_log::write(&log).unwrap()), Some(f.digest()));
}
