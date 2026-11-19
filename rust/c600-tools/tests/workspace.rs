//! The host's provenance reader ([`c600_tools::workspace::Reader`]) on the command layer (KN5's stage 3): a
//! `workspace.set` whose block the reader refuses is refused with the reader's error and writes nothing (PS1.16), and a
//! draft that names a record, `{"record": name}`, is admitted or refused by the reader's preview admission (KN5.26,
//! BND.84, BND.85). Each test names the conformance rows it covers headless with PHS.62 markers.

mod process_support;

use c600_command::{
    code, CommandLayer, Completion, CompletionKind, Provenance, Receipt, RecordValue, Shell, Status, RECORD_ROW,
};
use c600_digest::sha256;
use c600_kernel::json::{self, Value as Json};
use c600_kernel::ProtectedSet;
use c600_run::boundary::{Answer, Store as RunStore};
use c600_run::provenance::{self, field, Author, Pin, Producer, Profile};
use c600_tools::bridge::{base, Bridge};
use c600_tools::process::{invoke_tool, run_in_process};
use c600_tools::tool::{self, named_word_block, Tool};
use c600_tools::workspace::{self, Reader};
use process_support::{kernel, secret};

const WORD: [i32; 3] = [3, -7, 12];

struct Fixture {
    layer: CommandLayer,
    reader: Reader,
    shell: Shell,
}

/// A session at the solved state on the host's table, with the host's reader installed as `c600-lp3-source` installs
/// it: under the pins of the tools' runs, keeping no block yet.
fn fixture() -> Fixture {
    let kernel = kernel();
    let solved = kernel.solved();
    let none = ProtectedSet::none();
    let mut layer =
        CommandLayer::new(kernel, workspace::table(), secret(), Default::default(), "records", solved, none);
    let reader = workspace::install(&mut layer);
    Fixture { layer, reader, shell: Shell::new(secret(), "shell") }
}

fn letters(word: &[i32]) -> Json {
    Json::Array(word.iter().map(|&letter| Json::Int(i64::from(letter))).collect())
}

/// `workspace.set`'s arguments: a word's letters under `record`, with `block` as the record's provenance.
fn record_args(record: &str, word: &[i32], block: &str) -> Json {
    json::object([
        ("payload", letters(word)),
        ("provenance", Json::String(block.into())),
        ("record", Json::String(record.into())),
    ])
}

/// A draft that names record `record` (a `ResolvedDraftRef`).
fn record_draft(record: &str) -> Json {
    json::object([("draft", json::object([("record", Json::String(record.into()))]))])
}

fn text(block: &[u8]) -> String {
    String::from_utf8(block.to_vec()).expect("a block is UTF-8 text")
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

/// The block of a solver's own tool's output (BND.22): a program producer with author `own`, in the solving profile.
fn own_tool_block() -> Vec<u8> {
    let producer = Producer {
        contract: tool::contract(),
        checker: Some(tool::checker()),
        author: Author::Own,
        module: sha256(b"a module the solver wrote\n"),
        entry: "mine".into(),
        profile: Profile::Solving,
    };
    provenance::canonical(&provenance::program(&producer, false, &[]))
}

impl Fixture {
    /// `workspace.set` from the shell.
    fn write(&mut self, record: &str, word: &[i32], block: &str) -> Receipt {
        self.shell.invoke(&mut self.layer, RECORD_ROW, record_args(record, word, block)).expect("prepared")
    }

    /// `operation.preview` from the shell.
    fn ask(&mut self, args: Json) -> Receipt {
        self.shell.invoke(&mut self.layer, "operation.preview", args).expect("prepared")
    }

    /// Previews the draft in `args`, which must be admitted, and builds the preview; returns its ID.
    fn preview_with(&mut self, args: Json) -> u64 {
        let receipt = self.ask(args);
        let Status::Running(id) = receipt.status else { panic!("the draft was not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        let built = self.layer.outcome(&receipt.command, &receipt.request).expect("the preview's outcome");
        let ready = matches!(built.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. }));
        assert!(ready, "{:?}", built.status);
        id
    }

    /// What a refusal may not change: the revision, the records, the durable log and the pending preview.
    fn fixed(&self) -> String {
        let pending = self.layer.preview().map(|p| p.id);
        format!("{:?} {:?} {} {pending:?}", self.layer.revision(), self.layer.records(), self.layer.store().log.len())
    }
}

/// PS1.16 and BND.21 (draft for the code: `PS1:capability.provenance_incompatible`, the code
/// `c600_run::provenance::read` gives): `workspace.set` with a block that does not read as a canonical FP.17 block is
/// refused with the reader's own error, code and detail, and writes nothing: no record, no workspace revision, no
/// durable action. A block that reads is written whatever its admission would say: the check reads the block alone, so
/// a solver tool's word is stored, and its preview is refused at admission (KN5.26; the next test).
#[test]
fn a_record_write_whose_block_the_reader_refuses_is_refused_with_the_readers_error() {
    let mut f = fixture();
    let manual = text(&named_word_block());
    let unterminated = manual.strip_suffix('\n').expect("a canonical block ends with a newline").to_owned();
    let spaced = manual.replacen(',', ", ", 1);
    let extra = manual.replacen('{', "{\"agent\":\"x\",", 1);
    assert!(f.reader.check(manual.as_bytes()).is_ok(), "the unchanged block reads");
    for (what, block) in [
        ("text that is no block", "not a block".to_owned()),
        ("a block without its final newline", unterminated),
        ("a block in another layout", spaced),
        ("a block with a field FP.17 does not define", extra),
    ] {
        let expected = f.reader.check(block.as_bytes()).expect_err(what);
        assert_eq!(expected.code, code::PROVENANCE_INCOMPATIBLE, "{what}");
        let before = f.fixed();
        let receipt = f.write("w", &WORD, &block);
        assert_eq!(receipt.status, Status::Refused(expected), "{what}");
        assert_eq!(f.fixed(), before, "{what}");
    }
    assert!(f.layer.records().is_empty());
    assert_eq!(f.layer.revision().workspace, 0);

    let own = text(&own_tool_block());
    for (record, block) in [("typed", manual), ("theirs", own.clone())] {
        assert!(completed(&f.write(record, &WORD, &block)), "{record}");
        assert_eq!(f.layer.record(record), Some(&RecordValue { payload: letters(&WORD), provenance: block }));
    }
    assert_eq!(f.layer.revision().workspace, 2);
    assert_eq!(f.reader.admit(own.as_bytes()).unwrap_err().code, code::NOT_GRANTED);
}

/// KN5.26 and CT-KN5-13 (part: the BND.84/BND.85 adapter LP3's host gives the layer, at the admission of a draft that
/// names a workspace record: a hand-entered word passes, a solver tool's word is refused, and a built-in tool's word
/// passes once the host keeps its run's blocks and fails closed on its missing part before, each refusal the
/// adapter's code and detail and none installing a preview; not run: picks, draft insertion, composition, file round
/// trips, imported 0.4 macros and KN5.9's preview with a protection conflict), and CT-BND-29 (part: a hand-entered
/// word in a record passes BND.84's producer check through the command layer, and a contract pin the policy does not
/// hold is refused with `PS1:capability.provenance_incompatible`).
#[test]
fn a_record_draft_is_admitted_or_refused_by_the_readers_admission() {
    // The 600-cell-Full tool's proposal of WORD, from a run on a scratch session inside the command the solver invoked
    // there, with the run's store.
    let tool = Tool::full_600_cell().expect("the tool builds");
    let (mut scratch, mut scratch_shell) = process_support::session("scratch");
    let command = invoke_tool(&mut scratch_shell, &mut scratch, &tool, &WORD).expect("the tool's row");
    let (name, read) = (command.run(), base(&command.job.revision));
    let mut run = RunStore::default();
    let mut bridge = Bridge::new(&mut scratch, &name);
    let ran = run_in_process(&tool, &WORD, &name, &read, &mut bridge, &mut run).expect("the tool runs");
    let [shown] = &ran.shell[..] else { panic!("one proposal: {:?}", ran.shell) };
    assert!(matches!(shown.answer, Answer::Previewed(_)), "{:?}", shown.answer);
    assert_eq!(shown.word, WORD);
    let proposed = run.blocks[&shown.block].clone();
    let read_back = provenance::read(&proposed).expect("the proposal's block reads");
    assert_eq!((field::author(&read_back), field::producer(&read_back)), ("built_in", "program"));
    assert_eq!(field::parts(&read_back), [sha256(&named_word_block())], "the named word is its one part");

    let mut f = fixture();
    let other = Pin::of("another-contract", b"a contract the policy in force does not hold\n");
    let unheld = provenance::canonical(&provenance::manual(&other));
    for (record, block) in
        [("typed", named_word_block()), ("proposed", proposed), ("theirs", own_tool_block()), ("unheld", unheld)]
    {
        assert!(completed(&f.write(record, &WORD, &text(&block))), "{record}");
    }

    for (record, expected, rule) in [
        ("theirs", code::NOT_GRANTED, "(BND.84)"),
        ("proposed", code::NOT_GRANTED, "(BND.21, BND.85)"),
        ("unheld", code::PROVENANCE_INCOMPATIBLE, "(FP.17)"),
    ] {
        let block = f.layer.record(record).expect("the record").provenance.clone();
        let theirs = f.reader.admit(block.as_bytes()).expect_err(record);
        let before = f.fixed();
        let receipt = f.ask(record_draft(record));
        assert_eq!(receipt.status, Status::Refused(theirs.clone()), "{record}");
        assert_eq!(theirs.code, expected, "{record}");
        assert!(theirs.detail.ends_with(rule), "{record}: {}", theirs.detail);
        assert_eq!((f.fixed(), f.layer.run_jobs()), (before, 0), "{record}");
    }

    // A hand-entered word has no part to find.
    let typed = f.preview_with(record_draft("typed"));
    // The built-in tool's word passes once the host keeps the run's blocks, which hold its part.
    f.reader.keep_run(&run);
    let proposed = f.preview_with(record_draft("proposed"));
    assert_ne!(typed, proposed);
    let preview = f.layer.preview().expect("the record's preview");
    assert_eq!(preview.id, proposed);
    let resolved = preview.resolved.as_ref().expect("a record draft");
    let word: Vec<i64> = WORD.iter().map(|&letter| i64::from(letter)).collect();
    assert_eq!((resolved.letters.clone(), resolved.reads[0].record.as_str()), (word, "proposed"));
}
