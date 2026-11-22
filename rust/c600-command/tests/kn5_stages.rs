//! KN5's stages 3 and 5 against the command layer: the workspace store (`workspace.set`, a draft that names a record,
//! and a record write's undo and redo) and the presented-frame report whose stamp a word commit's token binds. Each
//! test names the conformance rows it covers headless with PHS.62 markers. Where the frozen text is silent, the test
//! follows the draft rulings recorded in `src/lib.rs` and `src/frame.rs` and says so.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, Caller, CommandLayer, Completion, CompletionKind, Direction, Error, FrameChain, HeadlessFrames,
    Invocation, Kind, MemoryStore, Overlay, PresentedFrame, PreviewState, Provenance, Receipt, RecordRead, RecordValue,
    RecordWrite, ResolvedDraft, Secret, Shell, ShownPreview, Status, Table, Transaction, TransactionKind, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn letters(moves: &[i64]) -> Value {
    Value::Array(moves.iter().map(|&letter| Value::Int(letter)).collect())
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// `workspace.set`'s arguments.
fn record_args(name: &str, payload: Value, provenance: &str) -> Value {
    json::object([("payload", payload), ("provenance", text(provenance)), ("record", text(name))])
}

/// A draft that names record `name` (a `ResolvedDraftRef`).
fn record_draft(name: &str) -> Value {
    json::object([("draft", json::object([("record", text(name))]))])
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

/// The host's provenance reader as the tests stand it in: at a write it refuses the block `refused` (PS1.16), and at a
/// preview's admission it refuses a record's word whose block is `foreign` (KN5.26); it accepts every other block.
struct Reader;

impl Provenance for Reader {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        match block {
            b"refused" => Err(Error::new(code::PROVENANCE_INCOMPATIBLE, "the test reader refuses the block")),
            _ => Ok(()),
        }
    }

    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        match block {
            b"foreign" => Err(Error::new(code::NOT_GRANTED, "the test reader refuses the word in this session")),
            _ => Ok(()),
        }
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state whose table holds `workspace.set` (H-08's row since the freeze, CM.17), with the test
/// reader when `reader`.
fn fixture(reader: bool) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    if reader {
        layer.set_provenance(Box::new(Reader));
    }
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// A reopen of `layer`'s store, which replays its record writes without a provenance reader.
fn reopen(layer: &CommandLayer) -> CommandLayer {
    let store = layer.store().clone();
    CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap()
}

impl Fixture {
    /// `workspace.set` through the command route.
    fn write(&mut self, name: &str, payload: Value, provenance: &str) -> Receipt {
        self.shell.invoke(&mut self.layer, RECORD_ROW, record_args(name, payload, provenance)).unwrap()
    }

    /// `operation.preview` with `args` through the command route.
    fn ask(&mut self, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, "operation.preview", args).unwrap()
    }

    /// Previews the draft in `args` and runs the job; returns the preview's ID.
    fn preview_with(&mut self, args: Value) -> u64 {
        let receipt = self.ask(args);
        let Status::Running(job) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        job
    }

    fn preview(&mut self, recipe: &Value) -> u64 {
        self.preview_with(draft_args(recipe))
    }

    /// The next headless frame, which shows the pending preview through the built-in chain, reported for `id`.
    fn present(&mut self, id: u64) {
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
    }

    /// Reports `frame` for `id` through the shell, which must answer `no_fresh_preview`.
    fn not_presented(&mut self, id: u64, frame: &PresentedFrame, what: &str) {
        let error = self.shell.present(&mut self.layer, id, frame).unwrap_err();
        assert_eq!(error.code, code::NO_FRESH_PREVIEW, "{what}: {error:?}");
    }

    /// The commit chord for preview `id`, prepared and not sent.
    fn press(&mut self, id: u64) -> Invocation {
        self.shell.prepare(&self.layer, "operation.execute", preview_args(id)).unwrap()
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let id = self.preview(recipe);
        self.present(id);
        let press = self.press(id);
        self.shell.send(&mut self.layer, press)
    }

    /// The undo or redo of the action on top of its stack; no action here releases an orbit, so none confirms.
    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    /// What no refusal may change: the state digest, the revision, the records and the durable log.
    fn fixed(&self) -> (String, String, String, usize) {
        let r = self.layer.revision();
        (r.digest.clone(), format!("{r:?}"), format!("{:?}", self.layer.records()), self.layer.store().log.len())
    }

    /// Delivers `invocation` on `caller`'s channel, which must be refused with `expected` and change nothing.
    fn refused_on(&mut self, caller: &Caller, invocation: Invocation, expected: &str, what: &str) -> Error {
        let before = self.fixed();
        let receipt = self.layer.invoke(caller, invocation);
        assert_eq!(self.fixed(), before, "{what}");
        match receipt.status {
            Status::Refused(error) if error.code == expected => error,
            status => panic!("{what}: expected {expected}, got {status:?}"),
        }
    }

    fn refused(&mut self, invocation: Invocation, expected: &str, what: &str) -> Error {
        self.refused_on(&Caller::Shell, invocation, expected, what)
    }
}

/// CT-KN5-04 (part: a record write is durable with its receipt, so its lookup and its retry answer the same
/// `completed(workspace)` after a restart and after a reopen, which replays the write without a provenance reader, and
/// write nothing, and the same command ID with other content is refused; the journal's part is in
/// `c600-session/tests/kn5_command.rs`), KN5.18, KN5.23, PS1.16, L8.
#[test]
fn a_record_write_keeps_its_receipt_across_a_restart_and_a_reopen() {
    let mut f = fixture(true);
    let before = f.layer.revision().clone();
    let asked = f.shell.prepare(&f.layer, RECORD_ROW, record_args("w", letters(&[1, 2]), "block")).unwrap();
    assert!(asked.token.is_none(), "a record write takes no token");
    let done = f.shell.send(&mut f.layer, asked.clone());
    let Status::Completed(Completion { kind: CompletionKind::Workspace, revision, result: None }) = &done.status else {
        panic!("{:?}", done.status)
    };
    let after = f.layer.revision().clone();
    assert_eq!(revision, &after);
    let puzzle = (&before.head, before.state, &before.digest, before.protection);
    assert_eq!((&after.head, after.state, &after.digest, after.protection), puzzle);
    assert_eq!(after.workspace, before.workspace + 1);
    let value = RecordValue { payload: letters(&[1, 2]), provenance: "block".into() };
    assert_eq!((f.layer.record("w"), f.layer.records()["w"].version), (Some(&value), 1));
    let log = f.layer.store().log.clone();
    assert_eq!(log.len(), 1);
    assert_eq!((log[0].kind, &log[0].pre, &log[0].post), (TransactionKind::Record, &before.digest, &before.digest));
    assert_eq!(log[0].record, Some(RecordWrite { name: "w".into(), value: Some(value.clone()) }));
    assert_eq!(log[0].receipt(), done);

    // The retry answers the stored receipt; other content under the command ID is refused and writes nothing.
    assert_eq!(f.layer.invoke(&Caller::Shell, asked.clone()), done);
    let other = Invocation { args: record_args("w", letters(&[3]), "block"), ..asked.clone() };
    assert_eq!(code_of(&f.layer.invoke(&Caller::Shell, other)), code::COMMAND_ID_REUSED);
    assert_eq!((f.layer.store().log.len(), f.layer.records()["w"].version), (1, 1));

    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.outcome(&asked.command, &done.request), Some(done.clone()));
    assert_eq!(f.layer.invoke(&Caller::Shell, asked.clone()), done);
    assert_eq!((f.layer.store().log.len(), f.layer.record("w")), (1, Some(&value)));

    let mut reopened = reopen(&f.layer);
    assert_eq!((reopened.record("w"), reopened.records()["w"].version), (Some(&value), 1));
    assert_eq!(reopened.outcome(&asked.command, &done.request), Some(done.clone()));
    assert_eq!(reopened.invoke(&Caller::Shell, asked), done);
    assert_eq!((reopened.store().log.len(), reopened.revision().workspace), (1, 1));
}

/// CT-ONE-21 (part: a restart ends the preview built from a draft, and its prepared commit cannot commit it, while
/// the draft, a workspace record with its durable receipt, stays across the restart and a reopen and builds a fresh
/// preview that commits; the receipts, the transient outcomes and the old epoch's permissions run in `layer.rs`; not
/// run: the value-store handles and derived values of ONE.70, which no crate holds), ONE.35, ONE.43, ONE.70.
#[test]
fn a_restart_ends_a_drafts_preview_and_keeps_the_draft() {
    let mut f = fixture(true);
    let written = f.write("w", letters(&[1, 2]), "block");
    assert!(completed(&written));
    let value = f.layer.record("w").cloned().expect("the draft");
    let p = f.preview_with(record_draft("w"));
    f.present(p);
    let early = f.press(p);
    let log = f.layer.store().log.len();

    f.layer.restart(Secret::fixed([8; 32]));
    f.shell.restart(Secret::fixed([8; 32]));
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Discarded));
    assert!(f.layer.preview().is_none(), "restart never revives a preview");
    assert_eq!((f.layer.record("w"), f.layer.records()["w"].version), (Some(&value), 1));
    let epoch = f.layer.revision().epoch;
    assert_eq!(code_of(&f.layer.invoke(&Caller::Shell, Invocation { epoch, ..early })), code::TOKEN_INVALID);
    assert_eq!(f.layer.store().log.len(), log, "the old press committed nothing");

    let reopened = reopen(&f.layer);
    assert_eq!((reopened.record("w"), reopened.records()["w"].version), (Some(&value), 1));
    assert!(reopened.preview().is_none());

    let q = f.preview_with(record_draft("w"));
    assert_ne!(q, p);
    f.present(q);
    let press = f.press(q);
    assert!(committed(&f.shell.send(&mut f.layer, press)));
    assert_eq!(f.layer.record("w"), Some(&value), "a commit keeps the draft (ONE.35)");
}

/// Every source file under `dir`, with its path, recursively.
fn sources(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            out.push((path, text));
        }
    }
    out
}

/// CT-EVD-21 (part: the command layer's commit path: the layer depends on no evidence crate and no source names one,
/// and a preview, its commit and an undo answer the same, with the same revisions and digests, whether the workspace's
/// records hold `proved` and a small claim or `refuted` and a large one; the evidence library's half runs in
/// `c600-evidence/tests/sinks.rs`, the policy, grants and admission in `c600-run/tests/tables.rs`, and the session
/// store's commit modules in `c600-session/tests/commit.rs`), EVD.56.
#[test]
fn the_commit_path_answers_the_same_whatever_words_and_claims_the_workspace_holds() {
    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let dependencies = manifest.split("[dependencies]").nth(1).expect("a dependency table");
    let dependencies = dependencies
        .split(
            "
[",
        )
        .next()
        .unwrap();
    assert!(dependencies.contains("c600-kernel") && !dependencies.contains("c600-evidence"), "{dependencies}");
    let files = sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    assert!(files.len() > 5);
    for (path, text) in &files {
        assert!(!text.contains("c600_evidence"), "{} names the evidence crate", path.display());
    }

    let run = |verdict: &str, claim: i64| {
        let mut f = fixture(true);
        assert!(completed(&f.write("verdict", text(verdict), "block")));
        assert!(completed(&f.write("claim", Value::Int(claim), "block")));
        assert!(completed(&f.write("w", letters(&[1, 2]), "block")));
        let p = f.preview_with(record_draft("w"));
        f.present(p);
        let available = f.layer.availability("operation.execute");
        let press = f.press(p);
        let done = f.shell.send(&mut f.layer, press);
        assert!(committed(&done), "{:?}", done.status);
        let after = f.layer.revision().clone();
        let undone = f.step(Direction::Undo);
        assert!(committed(&undone), "{:?}", undone.status);
        let back = f.layer.revision().clone();
        (
            available,
            format!("{:?}", done.status),
            format!("{after:?}"),
            format!("{:?}", undone.status),
            format!("{back:?}"),
        )
    };
    assert_eq!(run("proved", 1), run("refuted", 1_000_000));
}

/// CT-KN5-09 (part: a record write and its undo and redo move no puzzle, so they keep the head, the state revision,
/// the pending preview and a running program job, and the undo succeeds while that job runs; the undo of a record's
/// first write removes its value and the redo writes it again, each a workspace inverse that answers `committed` with
/// equal puzzle hashes and advances the workspace revision and the record's version; an inline recipe's preview reads
/// no record, so a press from before them commits; a reopen replays them; not run: the program job's host), KN5.18,
/// KN5.20, LI.26, L4.
#[test]
fn a_record_write_and_its_undo_keep_the_puzzle_the_preview_and_the_jobs() {
    let mut f = fixture(true);
    let overlay = Overlay { module: "tools/full".into(), entry: "inspect".into() };
    f.layer.register_overlay("tool.full.inspect", Kind::Query, &[], overlay).unwrap();
    assert!(committed(&f.commit(&word(&[1]))));
    let query = f.shell.invoke(&mut f.layer, "tool.full.inspect", json::object([])).unwrap();
    assert!(matches!(query.status, Status::Running(_)), "{:?}", query.status);
    assert_eq!(f.layer.take_program_job().unwrap().row, "tool.full.inspect");
    let p = f.preview(&word(&[2]));
    f.present(p);
    let press = f.press(p);
    let start = f.layer.revision().workspace;
    assert!(completed(&f.write("w", letters(&[3]), "block")));
    let value = RecordValue { payload: letters(&[3]), provenance: "block".into() };

    for (direction, expected, version) in [(Direction::Undo, None, 2), (Direction::Redo, Some(&value), 3)] {
        let before = f.layer.revision().clone();
        let receipt = f.step(direction);
        let Status::Committed(record) = &receipt.status else { panic!("{direction:?}: {:?}", receipt.status) };
        assert_eq!((&record.pre, &record.post), (&before.digest, &before.digest), "{direction:?}");
        let after = f.layer.revision().clone();
        let puzzle = (&before.head, before.state, &before.digest, before.protection);
        assert_eq!((&after.head, after.state, &after.digest, after.protection), puzzle, "{direction:?}");
        assert_eq!(after.workspace, before.workspace + 1, "{direction:?}");
        assert_eq!((f.layer.record("w"), f.layer.records()["w"].version), (expected, version), "{direction:?}");
        let written = f.layer.store().log.last().unwrap().record.clone();
        assert_eq!(written, Some(RecordWrite { name: "w".into(), value: expected.cloned() }), "{direction:?}");
        assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented), "{direction:?}");
        assert_eq!(f.layer.outcome(&query.command, &query.request).as_ref(), Some(&query), "{direction:?}");
    }
    assert_eq!(f.layer.revision().workspace, start + 3);
    assert!(committed(&f.shell.send(&mut f.layer, press)));

    let reopened = reopen(&f.layer);
    assert_eq!(reopened.records(), f.layer.records());
    let (a, b) = (reopened.revision(), f.layer.revision());
    let expected = (&b.head, b.state, &b.digest, b.workspace, b.protection);
    assert_eq!((&a.head, a.state, &a.digest, a.workspace, a.protection), expected);
}

/// KN0.8 and KN5.11, and CT-KN5-13 (part: the layer asks the host's adapter about a record's block when it admits a
/// draft that names the record, also for a record that a reopen restored, and refuses the draft with the adapter's
/// refusal, leaving the slot as it was; not run: the BND.84/BND.85 adapter itself, which LP3's host supplies, and
/// picks, draft insertion, composition and file round trips), KN5.26. A draft `{"record": name}` previews the letters
/// the record holds, the same word as the letters inline, and its draft digest covers the draft, the letters and the
/// record's content digest and version. A missing record and a record that holds no list of letters are
/// `named_object_invalid` (M6's code), a session without a reader `not_granted` (draft), and a draft with any other
/// form `argument_form` (draft).
#[test]
fn a_record_draft_previews_the_records_letters_once_the_reader_admits_them() {
    let mut f = fixture(true);
    let kept = f.preview(&word(&[5]));
    f.present(kept);
    assert!(completed(&f.write("text", text("not letters"), "block")));
    assert!(completed(&f.write("wide", letters(&[1 << 40]), "block")));
    assert!(completed(&f.write("theirs", letters(&[1]), "foreign")));
    let malformed =
        [json::object([("record", Value::Int(5))]), json::object([("record", text("w")), ("recipe", word(&[1]))])];
    let mut cases: Vec<(String, Value, &str)> = vec![
        ("a missing record".into(), record_draft("missing"), code::NAMED_OBJECT_INVALID),
        ("a record of text".into(), record_draft("text"), code::NAMED_OBJECT_INVALID),
        ("a letter that does not fit".into(), record_draft("wide"), code::NAMED_OBJECT_INVALID),
        ("a block the reader refuses".into(), record_draft("theirs"), code::NOT_GRANTED),
    ];
    for (i, draft) in malformed.into_iter().enumerate() {
        cases.push((format!("malformed draft {i}"), json::object([("draft", draft)]), code::ARGUMENT_FORM));
    }
    for (what, args, expected) in cases {
        let before = (f.fixed(), f.layer.generation());
        let receipt = f.ask(args);
        assert_eq!(code_of(&receipt), expected, "{what}");
        assert_eq!((f.fixed(), f.layer.generation()), before, "{what}");
        assert_eq!(f.layer.preview().map(|p| p.id), Some(kept), "{what}");
        assert_eq!((f.layer.preview_state(kept), f.layer.run_jobs()), (Some(PreviewState::Presented), 0), "{what}");
    }

    // The record's word.
    assert!(completed(&f.write("w", letters(&[1, 2]), "block")));
    let p = f.preview_with(record_draft("w"));
    let preview = f.layer.preview().expect("the record's preview").clone();
    assert_eq!(preview.id, p);
    let read = RecordRead { record: "w".into(), digest: f.layer.record("w").unwrap().digest(), version: 1 };
    let source = json::object([("record", text("w"))]);
    let resolved = ResolvedDraft { source, letters: vec![1, 2], reads: vec![read] };
    assert_eq!(preview.resolved.as_ref(), Some(&resolved));
    assert_eq!(preview.deps.draft, resolved.digest());
    let mut inline = fixture(false);
    inline.preview(&word(&[1, 2]));
    let same = inline.layer.preview().unwrap();
    assert_eq!(
        (&preview.literal, preview.end.as_ref().map(|s| s.digest())),
        (&same.literal, same.end.as_ref().map(|s| s.digest()))
    );

    // A reopen restores the records but not the reader: the draft is refused until the host gives one, and the
    // restored block then reaches the reader.
    let mut bare = reopen(&f.layer);
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let refused = shell.invoke(&mut bare, "operation.preview", record_draft("w")).unwrap();
    assert_eq!(code_of(&refused), code::NOT_GRANTED);
    assert_eq!(bare.availability(RECORD_ROW).unwrap_err().code, code::ROW_NOT_BUILT);
    bare.set_provenance(Box::new(Reader));
    let restored = shell.invoke(&mut bare, "operation.preview", record_draft("theirs")).unwrap();
    assert_eq!(code_of(&restored), code::NOT_GRANTED);
    let admitted = shell.invoke(&mut bare, "operation.preview", record_draft("w")).unwrap();
    assert!(matches!(admitted.status, Status::Running(_)), "{:?}", admitted.status);
}

/// KN0.8 and KN5.16: a record draft's preview reads its record again at every freshness check, so after a write of
/// the record, the same value written again, or the write's undo, the commit row is unavailable and a press from
/// before is `revision_changed`, since a record's version never goes back; a write of another record leaves it fresh,
/// and the press commits. CT-KN5-06 (part: a record edit and its undo restore the record's value, and a press from
/// before them is still stale).
#[test]
fn a_record_drafts_preview_goes_stale_when_its_record_changes() {
    for case in ["another value", "the same value", "undone", "edited and undone", "another record"] {
        let mut f = fixture(true);
        assert!(completed(&f.write("w", letters(&[1, 2]), "block")));
        let p = f.preview_with(record_draft("w"));
        f.present(p);
        let press = f.press(p);
        match case {
            "another value" => assert!(completed(&f.write("w", letters(&[2, 1]), "block"))),
            "the same value" => assert!(completed(&f.write("w", letters(&[1, 2]), "block"))),
            "undone" => assert!(committed(&f.step(Direction::Undo))),
            "edited and undone" => {
                assert!(completed(&f.write("w", letters(&[2, 1]), "block")));
                assert!(committed(&f.step(Direction::Undo)));
                let restored = f.layer.records()["w"].clone();
                assert_eq!((restored.value.map(|v| v.payload), restored.version), (Some(letters(&[1, 2])), 3));
            }
            _ => assert!(completed(&f.write("v", letters(&[3]), "block"))),
        }
        assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented), "{case}");
        if case == "another record" {
            assert_eq!(f.layer.availability("operation.execute"), Ok(()), "{case}");
            assert!(committed(&f.shell.send(&mut f.layer, press)), "{case}");
        } else {
            let unavailable = f.layer.availability("operation.execute").unwrap_err();
            assert_eq!(unavailable.code, code::REVISION_CHANGED, "{case}");
            f.refused(press, code::REVISION_CHANGED, case);
        }
    }
}

/// CT-LI-01 (whole): the commit row's availability and the commit share KN0.8's freshness check, against LI.33's edits.
/// A record draft's preview p is presented and two presses C and D are prepared for it, then one edit runs: a write of
/// a record p does not read leaves the row available and D commits; a write of the record p reads, a protection change
/// to the same set and that change's undo each make the row `revision_changed` and refuse D with the same code; C's
/// commit and its undo restore the start digest, and with p gone the row and D both answer `no_fresh_preview`. A
/// version never goes back, so equal labels, an equal protected set or an equal record value revive no press.
/// `c600-session/tests/commit.rs` runs the same edits against the session's own gates.
#[test]
fn availability_and_the_commit_agree_on_li_33s_edits() {
    let edits = ["unread record", "read record", "protection", "protection undone", "commit undone"];
    for edit in edits {
        let mut f = fixture(true);
        assert!(completed(&f.write("w", letters(&[1, 2]), "block")));
        let start = f.layer.revision().digest.clone();
        let p = f.preview_with(record_draft("w"));
        f.present(p);
        let d = f.press(p);
        let c = f.press(p);
        let expected = match edit {
            "unread record" => {
                assert!(completed(&f.write("keymap", letters(&[9]), "block")));
                None
            }
            "read record" => {
                assert!(completed(&f.write("w", letters(&[2, 1]), "block")));
                Some(code::REVISION_CHANGED)
            }
            "protection" | "protection undone" => {
                let orbits = json::object([("orbits", Value::Array(Vec::new()))]);
                let set = f.shell.invoke(&mut f.layer, "protection.orbit.set", orbits).unwrap();
                assert!(matches!(set.status, Status::Completed(_)), "{edit}: {:?}", set.status);
                if edit == "protection undone" {
                    assert!(committed(&f.step(Direction::Undo)), "{edit}");
                }
                assert!(f.layer.protected().orbits().is_empty(), "{edit}");
                assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented), "{edit}");
                Some(code::REVISION_CHANGED)
            }
            _ => {
                let receipt = f.shell.send(&mut f.layer, c.clone());
                assert!(committed(&receipt), "{edit}: {:?}", receipt.status);
                assert_ne!(f.layer.revision().digest, start);
                assert!(committed(&f.step(Direction::Undo)), "{edit}");
                assert_eq!(f.layer.revision().digest, start, "the undo restores the start digest");
                assert_eq!(f.shell.send(&mut f.layer, c.clone()), receipt, "a retry of C answers C's receipt");
                Some(code::NO_FRESH_PREVIEW)
            }
        };
        match expected {
            None => {
                assert_eq!(f.layer.availability("operation.execute"), Ok(()), "{edit}");
                assert!(committed(&f.shell.send(&mut f.layer, d)), "{edit}");
            }
            Some(expected) => {
                let unavailable = f.layer.availability("operation.execute").unwrap_err();
                assert_eq!(unavailable.code, expected, "{edit}: {unavailable:?}");
                f.refused(d, expected, edit);
            }
        }
    }
}

/// PS1.16 and KN5.15 (draft rulings for the codes): `workspace.set` without the host's provenance reader is not built
/// (`row_not_built`, also in its availability), the runtime may not invoke it (`not_granted`), a block the reader
/// refuses is refused with the reader's error, and a bad argument is `argument_form`; none of them writes anything.
#[test]
fn a_record_write_is_refused_without_its_reader_and_never_reaches_the_runtime() {
    let mut f = fixture(false);
    assert_eq!(f.layer.availability(RECORD_ROW).unwrap_err().code, code::ROW_NOT_BUILT);
    let asked = f.shell.prepare(&f.layer, RECORD_ROW, record_args("w", letters(&[1]), "block")).unwrap();
    f.refused(asked, code::ROW_NOT_BUILT, "no reader");

    let mut f = fixture(true);
    assert_eq!(f.layer.availability(RECORD_ROW), Ok(()));
    let runtime = Caller::Runtime { run: "run-1".into() };
    let epoch = f.layer.revision().epoch;
    let args = record_args("w", letters(&[1]), "block");
    let theirs =
        Invocation { command: "run-1/1".into(), row: RECORD_ROW.into(), args, token: None, epoch, budget: None };
    f.refused_on(&runtime, theirs, code::NOT_GRANTED, "the runtime");
    let asked = f.shell.prepare(&f.layer, RECORD_ROW, record_args("w", letters(&[1]), "refused")).unwrap();
    let error = f.refused(asked, code::PROVENANCE_INCOMPATIBLE, "a refused block");
    assert_eq!(error.detail, "the test reader refuses the block");
    let bad = [
        (
            "a record name that is not text",
            json::object([("payload", letters(&[1])), ("provenance", text("block")), ("record", Value::Int(1))]),
        ),
        (
            "a block that is not text",
            json::object([("payload", letters(&[1])), ("provenance", Value::Int(1)), ("record", text("w"))]),
        ),
        ("a missing payload", json::object([("provenance", text("block")), ("record", text("w"))])),
    ];
    for (what, args) in bad {
        let asked = f.shell.prepare(&f.layer, RECORD_ROW, args).unwrap();
        f.refused(asked, code::ARGUMENT_FORM, what);
    }
    assert!(f.layer.records().is_empty());
}

/// CT-ONE-29 (part, at the layer: a frame that draws the pending preview through a user stage does not present it, and
/// the layer refuses its report, as it refuses a frame that shows no preview and one that shows another preview;
/// none of them gives the shell a binding, so the commit chord stops at the shell's gate; only a frame drawn by the
/// built-in chain presents the preview, and the shell's binding takes that frame's stamp; not run: a renderer's user
/// stage, for which `c600-tools/tests/views.rs` runs the display's composition), ONE.41, RPI.76, and RM.26's order
/// (draft): a report older than the newest one is refused and changes nothing, and the newest one repeated changes
/// nothing.
#[test]
fn only_a_frame_of_the_built_in_chain_presents_the_pending_preview() {
    let mut f = fixture(false);
    let p = f.preview(&word(&[1]));
    let preview = f.layer.preview().unwrap().clone();
    let mut another = ShownPreview::of(&preview);
    another.literal = "0".repeat(64);
    let frames = [
        ("no preview", f.frames.other(None, FrameChain::BuiltIn)),
        ("a user stage", f.frames.other(Some(&preview), FrameChain::UserStage)),
        (
            "another preview",
            PresentedFrame { preview: Some(another), stamp: f.frames.next(), chain: FrameChain::BuiltIn },
        ),
    ];
    for (what, frame) in &frames {
        f.not_presented(p, frame, what);
        assert_eq!((f.layer.preview_state(p), f.shell.presented()), (Some(PreviewState::Ready), None), "{what}");
        assert_eq!(f.layer.frame(), Some(frame), "{what}");
        let gate = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap_err();
        assert_eq!(gate.code, code::NO_FRESH_PREVIEW, "{what}");
    }

    let first = f.frames.built_in(&preview);
    f.shell.present(&mut f.layer, p, &first).unwrap();
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented));
    assert_eq!(f.shell.presented().map(|bound| (bound.id, &bound.frame)), Some((p, &first.stamp)));

    let generation = f.layer.generation();
    let mut older = first.clone();
    older.stamp.sequence -= 1;
    assert_eq!(f.layer.present(p, &older).unwrap_err().code, code::NO_FRESH_PREVIEW);
    assert_eq!(f.layer.present(p, &first).map(|shown| shown.id), Ok(p));
    assert_eq!((f.layer.generation(), f.layer.frame()), (generation, Some(&first)));
    assert_eq!(HeadlessFrames::after(&f.layer).next().sequence, first.stamp.sequence + 1);
    let press = f.press(p);
    assert!(committed(&f.shell.send(&mut f.layer, press)));

    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.frame(), None);
}

/// ONE.41 and RM.26 (draft): a word commit's token binds the stamp of the frame that presented its preview, and the
/// commit is refused with `no_fresh_preview` unless that frame belongs to the preview's current presentation, the run
/// of frames since the newest frame that started showing it on one device epoch, surface and surface generation. A
/// frame that shows no preview ends the run, as does a new device epoch or a reopened surface; a frame that continues
/// the run keeps a token from its first frame valid. CT-ONE-04 (part: with B received and A still shown, the report of
/// A's frame is refused, the shell keeps its binding to A with that frame's stamp, and the chord's token for A is
/// refused as replaced; B's own frame then presents B, and its chord commits; not run: the shell's display).
/// CT-ONE-26 (part: the token binds the presented preview and the stamp of the frame that presented it, headless; not
/// run: the on-screen half (C9)).
#[test]
fn a_token_binds_the_frame_run_that_presents_its_preview() {
    let mut f = fixture(false);
    let p = f.preview(&word(&[1]));
    f.present(p);
    let early = f.press(p);
    let none = f.frames.other(None, FrameChain::BuiltIn);
    f.not_presented(p, &none, "a frame with no preview");
    assert_eq!((f.layer.preview_state(p), f.shell.presented()), (Some(PreviewState::Ready), None));
    f.present(p);
    let error = f.refused(early, code::NO_FRESH_PREVIEW, "a token from an earlier presentation");
    assert_eq!(error.detail, format!("the token's frame does not present preview {p} now"));

    let late = f.press(p);
    f.frames.recover();
    f.present(p);
    f.refused(late, code::NO_FRESH_PREVIEW, "a token from the device epoch before");

    let stale = f.press(p);
    let mut reopened = f.frames.next();
    reopened.surface_generation += 1;
    let frame = PresentedFrame::built_in(f.layer.preview().unwrap(), reopened.clone());
    f.shell.present(&mut f.layer, p, &frame).unwrap();
    f.refused(stale, code::NO_FRESH_PREVIEW, "a token from the surface before");

    let press = f.press(p);
    let mut later = f.frames.next();
    later.surface_generation += 1;
    let frame = PresentedFrame::built_in(f.layer.preview().unwrap(), later.clone());
    f.shell.present(&mut f.layer, p, &frame).unwrap();
    assert_eq!(f.shell.presented().map(|bound| &bound.frame), Some(&later));
    assert!(committed(&f.shell.send(&mut f.layer, press)), "a token from the first frame of the run");

    // CT-ONE-04: B is received while the display still shows A.
    let mut f = fixture(false);
    let a = f.preview(&word(&[1]));
    f.present(a);
    let shown = f.layer.preview().unwrap().clone();
    let b = f.preview(&word(&[2]));
    let still = f.frames.built_in(&shown);
    f.not_presented(a, &still, "A's frame while B is pending");
    assert_eq!(f.layer.preview_state(b), Some(PreviewState::Ready));
    assert_eq!(f.shell.presented().map(|bound| (bound.id, &bound.frame)), Some((a, &still.stamp)));
    let press = f.press(a);
    f.refused(press, code::PREVIEW_REPLACED, "A's chord while B is pending");
    f.present(b);
    let press = f.press(b);
    assert!(committed(&f.shell.send(&mut f.layer, press)));
}

/// The journal form (draft): a commit's transaction has its 17 keys and no `record`, a record write's and its undo's
/// have `record` as an 18th key (an undo of a first write with a `null` value), each reads back to itself, and a
/// transaction whose `record` is `null` is refused, so a 17-key journal reads as before. A commit that carries a
/// record, and a record write without one or with a `null` value, do not read.
#[test]
fn a_record_write_journals_its_record_and_a_commit_does_not() {
    let mut f = fixture(true);
    assert!(committed(&f.commit(&word(&[1]))));
    assert!(completed(&f.write("w", letters(&[1]), "block")));
    assert!(committed(&f.step(Direction::Undo)));
    let log = f.layer.store().log.clone();
    let kinds: Vec<TransactionKind> = log.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [TransactionKind::Commit, TransactionKind::Record, TransactionKind::Undo]);
    let keys: Vec<usize> = log.iter().map(|t| t.to_json().as_object().unwrap().len()).collect();
    assert_eq!(keys, [17, 18, 18]);
    assert_eq!(log[0].to_json().get("record"), None);
    let undone = json::object([("name", text("w")), ("value", Value::Null)]);
    assert_eq!(log[2].to_json().get("record"), Some(&undone));
    for transaction in &log {
        assert_eq!(Transaction::from_json(&transaction.to_json()).as_ref(), Some(transaction));
    }
    let Value::Object(mut fields) = log[0].to_json() else { panic!("a transaction is an object") };
    fields.insert("record".into(), Value::Null);
    assert_eq!(Transaction::from_json(&Value::Object(fields)), None);

    let object_of = |transaction: &Transaction| match transaction.to_json() {
        Value::Object(fields) => fields,
        _ => panic!("a transaction is an object"),
    };
    let (mut commit, mut bare, mut empty) = (object_of(&log[0]), object_of(&log[1]), object_of(&log[1]));
    commit.insert("record".into(), undone.clone());
    bare.remove("record");
    empty.insert("record".into(), undone);
    let malformed = [(commit, "a commit with a record"), (bare, "a record write without one"), (empty, "a null write")];
    for (fields, what) in malformed {
        assert_eq!(Transaction::from_json(&Value::Object(fields)), None, "{what}");
    }
}

/// CT-KN5-09 (part: a reopen restores the undo and redo of record writes, so a reopened layer undoes and redoes them
/// as the live layer does, whether the reopen falls between an undo and its redo or after the redo), KN5.20, KN5.23,
/// LI.26.
#[test]
fn a_reopened_layer_undoes_and_redoes_record_writes_as_the_live_layer_does() {
    let mut live = fixture(true);
    assert!(completed(&live.write("w", letters(&[1]), "block")));
    assert!(completed(&live.write("w", letters(&[2]), "block")));
    assert!(committed(&live.step(Direction::Undo)));
    let reopened = |live: &Fixture| Fixture {
        layer: reopen(&live.layer),
        shell: Shell::new(Secret::fixed(SECRET), "shell"),
        frames: HeadlessFrames::new(),
    };
    let same = |a: &Fixture, b: &Fixture, what: &str| {
        let (x, y) = (a.layer.revision(), b.layer.revision());
        assert_eq!(a.layer.records(), b.layer.records(), "{what}");
        let puzzle = (&y.head, y.state, &y.digest, y.workspace, y.protection);
        assert_eq!((&x.head, x.state, &x.digest, x.workspace, x.protection), puzzle, "{what}");
    };

    let mut between = reopened(&live);
    same(&between, &live, "reopened between the undo and its redo");
    for direction in [Direction::Redo, Direction::Undo, Direction::Undo, Direction::Redo] {
        assert!(committed(&live.step(direction)), "live {direction:?}");
        assert!(committed(&between.step(direction)), "reopened {direction:?}");
        same(&between, &live, &format!("{direction:?}"));
    }
    let value = RecordValue { payload: letters(&[1]), provenance: "block".into() };
    assert_eq!((live.layer.record("w"), live.layer.records()["w"].version), (Some(&value), 7));

    let mut after = reopened(&live);
    for (direction, expected) in [(Direction::Undo, None), (Direction::Redo, Some(&value))] {
        assert!(committed(&live.step(direction)), "live {direction:?}");
        assert!(committed(&after.step(direction)), "reopened after the redo, {direction:?}");
        same(&after, &live, &format!("after the redo, {direction:?}"));
        assert_eq!(after.layer.record("w"), expected, "{direction:?}");
    }
}

/// The shell's presented binding follows the state (KN5 follow-up; draft): a record write's undo and redo answer
/// `committed` but move no state and keep the preview presented, so the shell keeps its binding and a press prepared
/// after them commits; a commit and a commit's undo move the state, end the preview and drop the binding, so the shell
/// itself answers a later press with `KN5:stale.no_fresh_preview`.
#[test]
fn the_shell_keeps_its_presented_preview_while_the_state_stays() {
    let mut f = fixture(true);
    let p = f.preview(&word(&[2]));
    f.present(p);
    assert!(completed(&f.write("w", letters(&[3]), "block")));
    for direction in [Direction::Undo, Direction::Redo] {
        let state = f.layer.revision().state;
        assert!(committed(&f.step(direction)), "{direction:?}");
        assert_eq!(f.layer.revision().state, state, "{direction:?}");
        assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented), "{direction:?}");
    }
    let press = f.press(p);
    assert!(committed(&f.shell.send(&mut f.layer, press)));
    let error = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap_err();
    assert_eq!(error.code, code::NO_FRESH_PREVIEW, "{error:?}");

    let q = f.preview(&word(&[1]));
    f.present(q);
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.preview_state(q), Some(PreviewState::Discarded));
    let error = f.shell.prepare(&f.layer, "operation.execute", preview_args(q)).unwrap_err();
    assert_eq!(error.code, code::NO_FRESH_PREVIEW, "{error:?}");
}
