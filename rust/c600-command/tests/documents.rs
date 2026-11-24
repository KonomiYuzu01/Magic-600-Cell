//! LP4-14's document rows and redesigned log rows on the command layer (ADR-D01.79 to .84 as revision 6 amends them;
//! draft), with a fake document service in place of `c600_session::documents::DocumentStore`, whose own tests read and
//! write real logs. These tests pin the layer's half: without a service every row is `KN5:input.row_not_built`;
//! opening, saving, compiling and checking change no live state; `file.log.import` takes its confirmation and its file
//! check before its token, writes the recovery checkpoint, stops every playback state and commits one undoable entry
//! that a reopen replays; and an import that would move a protected orbit, or that names another model, another start
//! or another state, is refused and changes nothing.

use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use c600_command::{
    code, draft_args, ActionEntry, Caller, CheckedFile, CommandLayer, CompletionKind, Direction, DocumentKind,
    Documents, Error, HeadlessFrames, ImportPlan, LogFormat, MemoryRecorder, MemoryStore, PlayStatus, PlaybackState,
    Receipt, Recorded, Secret, Shell, Status, Stop, Table, TransactionKind, DOCUMENT_CLOSE_ROW, DOCUMENT_COMPILE_ROW,
    DOCUMENT_NEW_ROW, DOCUMENT_OPEN_ROW, DOCUMENT_ROWS, DOCUMENT_SAVE_AS_ROW, DOCUMENT_SAVE_ROW, EXIT_ROW,
    LOG_CHECK_ROW, LOG_EXPORT_ROW, LOG_IMPORT_ROW, SESSION_DOCUMENT, WHOLE_SESSION,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Kernel, ORBITS};

const SECRET: [u8; 32] = [7; 32];
/// The SHA-256 the fake's check names (any 64 lowercase hexadecimal digits).
const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
/// PS1.2's code for bytes that are no longer the checked ones.
const DIGEST_MISMATCH: &str = "PS1:input.digest_mismatch";
/// The code the fake's import plan answers when the log names another model.
const IDENTITY_MISMATCH: &str = c600_kernel::codes::IDENTITY_MISMATCH;
/// The code the layer answers when no proved path joins the head and the log's state.
const BRANCH_PATH_UNPROVED: &str = "KN4:input.branch_path_unproved";

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| {
            let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
            match Kernel::load(&assets) {
                Ok(kernel) => Arc::new(kernel),
                Err(error) => panic!("the model in assets/ does not load: {error}"),
            }
        })
        .clone()
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

/// The full-state hash of the solved state after `letters`.
fn reached(letters: &[i64]) -> String {
    let k = kernel();
    let recipe = k.recipe(&word(letters)).unwrap();
    match k.derive(&k.solved(), &recipe, &k.protected_set(&[]).unwrap()).unwrap() {
        Derivation::Derived(derived) => derived.state.digest().to_owned(),
        Derivation::Refused(refused) => panic!("{refused:?}"),
    }
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

/// What the fake service was asked, in order, and what it answers.
#[derive(Default)]
struct Calls {
    asked: Vec<String>,
    /// The plan of the one passed check it holds.
    plan: Option<ImportPlan>,
    /// Whether the checked file's bytes changed since its check.
    changed: bool,
    checkpoint_fails: bool,
}

/// A document service that keeps no documents: it answers each row with a small record of what it was asked.
#[derive(Clone, Default)]
struct Fake(Arc<Mutex<Calls>>);

impl Fake {
    fn calls(&self) -> MutexGuard<'_, Calls> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn asked(&self) -> Vec<String> {
        self.calls().asked.clone()
    }

    fn answer(&self, what: &str, fields: Vec<(&str, Value)>) -> Result<Value, Error> {
        self.calls().asked.push(what.to_owned());
        let mut fields: Vec<(String, Value)> = fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
        fields.push(("asked".to_owned(), text(what)));
        Ok(Value::Object(fields.into_iter().collect()))
    }
}

impl Documents for Fake {
    fn create(&mut self, kind: DocumentKind) -> Result<Value, Error> {
        self.answer("create", vec![("document", text("untitled-1")), ("kind", text(kind.name()))])
    }

    fn open(&mut self, file: &str) -> Result<Value, Error> {
        self.answer("open", vec![("document", text(file))])
    }

    fn save(&mut self, document: &str) -> Result<Value, Error> {
        self.answer("save", vec![("document", text(document))])
    }

    fn save_as(&mut self, document: &str, destination: &str) -> Result<Value, Error> {
        self.answer("save_as", vec![("document", text(document)), ("file", text(destination))])
    }

    fn compile(&mut self, document: &str) -> Result<Value, Error> {
        self.answer("compile", vec![("document", text(document)), ("passed", Value::Bool(true))])
    }

    fn close(&mut self, document: &str, discard: bool) -> Result<Value, Error> {
        if document == SESSION_DOCUMENT {
            return Err(Error::new(code::ARGUMENT_FORM, "the live session does not close"));
        }
        self.answer("close", vec![("document", text(document)), ("discarded", Value::Bool(discard))])
    }

    fn check(&mut self, file: &str) -> Result<Value, Error> {
        self.answer("check", vec![("file", text(file)), ("sha256", text(SHA)), ("passed", Value::Bool(true))])
    }

    fn export(&mut self, format: LogFormat, destination: &str) -> Result<Value, Error> {
        self.answer("export", vec![("format", text(format.name())), ("file", text(destination))])
    }

    fn import_plan(&mut self, checked: &CheckedFile) -> Result<ImportPlan, Error> {
        let mut calls = self.calls();
        calls.asked.push("import_plan".to_owned());
        if calls.changed {
            return Err(Error::new(DIGEST_MISMATCH, "the file's bytes changed since its check"));
        }
        match &calls.plan {
            Some(plan) if &plan.checked == checked => Ok(plan.clone()),
            _ => Err(Error::new(code::RESULT_GONE, "no passed check of this file and digest")),
        }
    }

    fn checkpoint(&mut self) -> Result<String, Error> {
        let mut calls = self.calls();
        calls.asked.push("checkpoint".to_owned());
        if calls.checkpoint_fails {
            return Err(Error::new(code::COMMIT_FATE, "the checkpoint's write failed"));
        }
        Ok("checkpoint-1".to_owned())
    }

    fn exit(&mut self) -> Result<Value, Error> {
        self.answer("exit", vec![("document", text(SESSION_DOCUMENT))])
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    fake: Fake,
}

/// A session at the solved state that protects `orbits`, with the fake service installed when `service` holds.
fn fixture(orbits: &[i64], service: bool) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(orbits).unwrap());
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let fake = Fake::default();
    if service {
        layer.set_documents(Box::new(fake.clone()));
    }
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), fake }
}

fn checked() -> CheckedFile {
    CheckedFile { file: "logs/a.c600log".to_owned(), sha256: SHA.to_owned() }
}

/// A passed check's plan of a log that starts at the solved state and saves its cursor after `letters`.
fn plan(letters: &[i64]) -> ImportPlan {
    let k = kernel();
    ImportPlan {
        checked: checked(),
        model: k.identity().model_id,
        start: k.solved().digest().to_owned(),
        line: word(letters),
        state: reached(letters),
        preferences: Some(r#"{"theme":"log's"}"#.to_owned()),
    }
}

fn import_args(confirm: bool) -> Value {
    let mut args = vec![("checked".to_owned(), checked().to_json())];
    if confirm {
        args.push(("confirm".to_owned(), Value::Bool(true)));
    }
    Value::Object(args.into_iter().collect())
}

/// Every document row with arguments of its forms.
fn args_of(row: &str) -> Value {
    let document = || ("document", text("untitled-1"));
    match row {
        DOCUMENT_NEW_ROW => json::object([("kind", text("log"))]),
        DOCUMENT_OPEN_ROW => json::object([("file", text("logs/a.c600log"))]),
        DOCUMENT_SAVE_ROW | DOCUMENT_COMPILE_ROW => json::object([document()]),
        DOCUMENT_SAVE_AS_ROW => json::object([document(), ("destination", text("logs/b.c600log"))]),
        DOCUMENT_CLOSE_ROW => json::object([document(), ("discard", Value::Bool(true))]),
        LOG_EXPORT_ROW => json::object([
            ("destination", text("logs/out.c600log")),
            ("format", text(LogFormat::Log.name())),
            ("history", text(WHOLE_SESSION)),
        ]),
        LOG_CHECK_ROW => json::object([("file", text("logs/a.c600log"))]),
        LOG_IMPORT_ROW => import_args(true),
        EXIT_ROW => json::object([]),
        other => panic!("not a document row: {other}"),
    }
}

impl Fixture {
    fn press(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    /// Previews, presents and commits `letters`.
    fn commit(&mut self, letters: &[i64]) {
        let receipt = self.press("operation.preview", draft_args(&word(letters)));
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let receipt = self.press("operation.execute", json::object([("preview", Value::Int(id as i64))]));
        assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    }

    /// The live state: the labels' digest, the revision and the journal's length.
    fn live(&self) -> (String, c600_command::Revision, usize) {
        (self.layer.snapshot().digest().to_owned(), self.layer.revision().clone(), self.layer.store().log.len())
    }

    /// The service's answer a completed receipt's result names, with the completion's kind.
    fn answer(&self, receipt: &Receipt) -> (CompletionKind, Value) {
        let Status::Completed(completion) = &receipt.status else { panic!("not completed: {:?}", receipt.status) };
        let result = completion.result.as_ref().expect("a result");
        (completion.kind, self.layer.document_result(&result.digest).expect("the answer is kept").clone())
    }
}

/// Without the host's document service, each of the ten rows (the six document rows, the three log rows and
/// `app.exit`) is `KN5:input.row_not_built` and writes nothing, as before LP4-14.
#[test]
fn without_a_document_service_every_document_row_is_not_built() {
    let mut f = fixture(&[], false);
    let before = f.live();
    for row in DOCUMENT_ROWS {
        assert_eq!(f.layer.availability(row).unwrap_err().code, code::ROW_NOT_BUILT, "{row}");
        let receipt = f.press(row, args_of(row));
        assert_eq!(code_of(&receipt), code::ROW_NOT_BUILT, "{row}");
    }
    assert_eq!(f.live(), before);
}

/// ADR-D01.79 to .81: a new document, an open, a save, a save as, a close and the exit's save are answered at once from
/// the service; a compile, a check and an export run as jobs on the exclusive lane. None takes an action number or
/// changes the labels, the revision or the journal: opening or compiling a log binds no live state.
#[test]
fn opening_saving_and_compiling_documents_change_no_live_state() {
    let mut f = fixture(&[], true);
    f.commit(&[3]);
    let before = f.live();
    for (row, kind, asked) in [
        (DOCUMENT_NEW_ROW, CompletionKind::App, "create"),
        (DOCUMENT_OPEN_ROW, CompletionKind::File, "open"),
        (DOCUMENT_SAVE_ROW, CompletionKind::File, "save"),
        (DOCUMENT_SAVE_AS_ROW, CompletionKind::File, "save_as"),
        (DOCUMENT_CLOSE_ROW, CompletionKind::App, "close"),
        (EXIT_ROW, CompletionKind::App, "exit"),
    ] {
        let receipt = f.press(row, args_of(row));
        assert_eq!(receipt.job, None, "{row}");
        let (completed, answer) = f.answer(&receipt);
        assert_eq!(completed, kind, "{row}");
        assert_eq!(answer.get("asked"), Some(&text(asked)), "{row}");
        assert_eq!(f.live(), before, "{row}");
    }
    for (row, kind, asked) in [
        (DOCUMENT_COMPILE_ROW, CompletionKind::Query, "compile"),
        (LOG_CHECK_ROW, CompletionKind::Query, "check"),
        (LOG_EXPORT_ROW, CompletionKind::File, "export"),
    ] {
        let started = f.press(row, args_of(row));
        let Status::Running(job) = started.status else { panic!("{row}: {:?}", started.status) };
        assert_eq!(f.layer.run_jobs(), 1, "{row}");
        let receipt = f.layer.kernel_outcome(&Caller::Shell, &started.command, &started.request);
        let (completed, answer) = f.answer(&receipt);
        assert_eq!(completed, kind, "{row}");
        assert_eq!(answer.get("asked"), Some(&text(asked)), "{row}: job {job}");
        assert_eq!(f.live(), before, "{row}");
    }
    // An unknown format, a malformed checked file and the live session's close are refused, and change nothing.
    let bad = json::object([
        ("destination", text("logs/out.c600log")),
        ("format", text("pdf")),
        ("history", text(WHOLE_SESSION)),
    ]);
    assert_eq!(code_of(&f.press(LOG_EXPORT_ROW, bad)), code::ARGUMENT_FORM);
    let bad = json::object([("checked", json::object([("file", text("a")), ("sha256", text("00"))]))]);
    assert_eq!(code_of(&f.press(LOG_IMPORT_ROW, bad)), code::ARGUMENT_FORM);
    let session = json::object([("document", text(SESSION_DOCUMENT))]);
    assert_eq!(code_of(&f.press(DOCUMENT_CLOSE_ROW, session)), code::ARGUMENT_FORM);
    assert_eq!(f.live(), before);
}

/// ADR-D01.82: `file.log.import` is refused, writing nothing, without the solver's confirmation, without a passed check
/// of the same file digest, and when the file's bytes changed since their check. Both gates come before the token's
/// authentication, which spends it: a press whose token another secret minted is refused by each gate first, and with
/// `KN5:auth.token_invalid` only once both hold. The shell's press then commits.
#[test]
fn an_import_needs_its_confirmation_and_its_file_check_before_its_token() {
    let mut f = fixture(&[], true);
    let before = f.live();
    let mut forger = Shell::new(Secret::fixed([9; 32]), "forger");
    let mut forged = |f: &mut Fixture, confirm: bool| {
        let forged = forger.prepare(&f.layer, LOG_IMPORT_ROW, import_args(confirm)).unwrap();
        f.layer.invoke(&Caller::Shell, forged)
    };
    assert_eq!(code_of(&forged(&mut f, false)), code::CONFIRMATION_REQUIRED);
    assert_eq!(code_of(&f.press(LOG_IMPORT_ROW, import_args(false))), code::CONFIRMATION_REQUIRED);
    assert!(f.fake.asked().is_empty(), "no file check is read without the confirmation");

    assert_eq!(code_of(&forged(&mut f, true)), code::RESULT_GONE, "no passed check of this file");
    assert_eq!(code_of(&f.press(LOG_IMPORT_ROW, import_args(true))), code::RESULT_GONE);
    f.fake.calls().plan = Some(plan(&[1, 2]));
    f.fake.calls().changed = true;
    assert_eq!(code_of(&forged(&mut f, true)), DIGEST_MISMATCH, "the bytes changed since their check");
    assert_eq!(code_of(&f.press(LOG_IMPORT_ROW, import_args(true))), DIGEST_MISMATCH);
    f.fake.calls().changed = false;
    assert_eq!(code_of(&forged(&mut f, true)), code::TOKEN_INVALID, "the token, once both gates hold");
    assert_eq!(f.live(), before);
    assert!(!f.fake.asked().contains(&"checkpoint".to_owned()), "no checkpoint before a refusal");

    let receipt = f.press(LOG_IMPORT_ROW, import_args(true));
    assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
}

/// ADR-D01.82 and LP4-14: an import from a head that is not the start writes the recovery checkpoint, then commits one
/// journal entry, a retained move under `file.log.import` whose word goes from the head back to the start and along
/// the log's line, and lands on the log's saved state. `session.undo` returns to the state before the import, and a
/// reopen replays the import to the same state. The log's preferences are not applied: the layer holds none.
#[test]
fn an_import_commits_one_undoable_entry_after_its_recovery_checkpoint() {
    let mut f = fixture(&[], true);
    f.commit(&[5]);
    let (before, _, log) = f.live();
    f.fake.calls().plan = Some(plan(&[1, 2]));
    let receipt = f.press(LOG_IMPORT_ROW, import_args(true));
    assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    assert_eq!(f.fake.asked(), ["import_plan", "checkpoint"]);
    assert_eq!(f.layer.import_checkpoint(), Some("checkpoint-1"));
    assert_eq!(f.layer.snapshot().digest(), reached(&[1, 2]));
    assert_eq!(f.layer.store().log.len(), log + 1, "one journal entry");
    let entry = f.layer.store().log.last().unwrap();
    assert_eq!((entry.kind, entry.row.as_str()), (TransactionKind::Commit, LOG_IMPORT_ROW));
    assert_eq!((entry.pre.as_str(), entry.post.as_str()), (before.as_str(), reached(&[1, 2]).as_str()));

    let reopened = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), f.layer.store().clone()).unwrap();
    assert_eq!(reopened.snapshot().digest(), reached(&[1, 2]), "a reopen replays the import");

    let hint = f.layer.undo_hint(Direction::Undo).expect("the import is on top");
    let undo = f.press("session.undo", json::object([("hint", hint.to_json())]));
    assert!(matches!(undo.status, Status::Committed(_)), "{:?}", undo.status);
    assert_eq!(f.layer.snapshot().digest(), before, "the undo returns to the state before the import");
}

/// KN2: an import is a commit under the protected set and the policy in force, so a log whose line would move a
/// protected orbit is refused with the protection code; a log of another model, of another start or whose line does
/// not reach its saved state is refused too. None writes the checkpoint or the journal.
#[test]
fn an_import_keeps_every_protected_orbit_and_refuses_another_model_start_or_state() {
    let every: Vec<i64> = (0..ORBITS as i64).collect();
    let mut f = fixture(&every, true);
    let before = f.live();
    f.fake.calls().plan = Some(plan(&[1]));
    assert_eq!(code_of(&f.press(LOG_IMPORT_ROW, import_args(true))), code::ORBIT_WOULD_MOVE);

    let mut f2 = fixture(&[], true);
    let before2 = f2.live();
    let cases = [
        (ImportPlan { model: "another-model".into(), ..plan(&[1]) }, IDENTITY_MISMATCH),
        (ImportPlan { start: reached(&[4]), ..plan(&[1]) }, BRANCH_PATH_UNPROVED),
        (ImportPlan { state: reached(&[2]), ..plan(&[1]) }, code::STATE_HASH_MISMATCH),
    ];
    for (bad, expected) in cases {
        f2.fake.calls().plan = Some(bad);
        assert_eq!(code_of(&f2.press(LOG_IMPORT_ROW, import_args(true))), expected);
    }
    assert!(!f.fake.asked().contains(&"checkpoint".to_owned()));
    assert!(!f2.fake.asked().contains(&"checkpoint".to_owned()));
    assert_eq!((f.live(), f2.live()), (before, before2));
}

/// LP4-14's reading for playback states (EV4.18, EV4.22): an import replaces the action numbers a playback's map names,
/// so every playback state is stopped with `KN7:cancelled.revision_replaced` and its map emptied, durably, before the
/// import commits (a finished one keeps its status and only its map is emptied); a later play starts from its recorded step against the imported session. A checkpoint that fails
/// answers `failed` and stops nothing.
#[test]
fn an_import_stops_every_playback_and_empties_its_action_map() {
    let mut f = fixture(&[], true);
    let recorder = MemoryRecorder::new();
    let mut waiting = PlaybackState::new("p", "digest-of-p", 2);
    waiting.actions = vec![ActionEntry { recorded: 1, played: 1 }];
    waiting.status = PlayStatus::Waiting(2);
    let mut finished = PlaybackState::new("f", "digest-of-f", 3);
    finished.actions = vec![ActionEntry { recorded: 2, played: 2 }];
    finished.status = PlayStatus::Finished;
    let mut held = Recorded::default();
    held.playbacks.insert("p".to_owned(), waiting.clone());
    held.playbacks.insert("f".to_owned(), finished.clone());
    recorder.put(held);
    f.layer.set_recorder(Box::new(recorder.clone())).unwrap();
    f.fake.calls().plan = Some(plan(&[1, 2]));

    f.fake.calls().checkpoint_fails = true;
    let (before, writes) = (f.live(), recorder.writes());
    let failed = f.press(LOG_IMPORT_ROW, import_args(true));
    assert!(matches!(failed.status, Status::Failed(_)), "{:?}", failed.status);
    assert_eq!((f.live(), recorder.writes()), (before, writes), "a failed checkpoint stops nothing");
    assert_eq!(f.layer.playback("p"), Some(&waiting));

    f.fake.calls().checkpoint_fails = false;
    let receipt = f.press(LOG_IMPORT_ROW, import_args(true));
    assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    let stopped = PlaybackState {
        actions: Vec::new(),
        status: PlayStatus::Stopped(Stop::Code(code::REVISION_REPLACED.to_owned())),
        ..waiting
    };
    assert_eq!(f.layer.playback("p"), Some(&stopped));
    assert_eq!(recorder.held().playbacks.get("p"), Some(&stopped), "durably");
    // A finished playback stays finished; only its map, which names pre-import actions, is emptied.
    let emptied = PlaybackState { actions: Vec::new(), ..finished };
    assert_eq!(recorder.held().playbacks.get("f"), Some(&emptied));
}
