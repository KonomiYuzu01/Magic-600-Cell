//! CT-BND-03 and CT-EV4-10 (parts) on the host's provenance reader (`c600_tools::workspace::Reader`, C5) with the
//! command layer over M6's journal: a word carried through a workspace record, a recording and both conversions keeps
//! its FP.17 block, which the conversion's block lists in its parts (BND.24, BND.32, EV4.13, EV4.23), and a research
//! run's word that a recording carries cannot be laundered into a solving preview (BND.84, EV4.16). The test reader's
//! version of these cases, and the rest of LP4-9's tests, are `c600-command`'s `tests/recording.rs`. The recorder is
//! the session's own (`JournalStore::recorder`).

mod common;

use c600_command::{
    code, Fate, RecordedStep, Recorder, Status, RECORDING_CONVERT_ROW, RECORDING_START_ROW, RECORDING_STOP_ROW,
};
use c600_digest::sha256;
use c600_kernel::json::{self, Value};
use c600_run::provenance::{self, field, Author, Producer, Profile};
use c600_session::disk::MemDisk;
use common::layer::{completed, record_draft, refused, text, Fixture};
use common::{contract, manual};

fn named(key: &str, value: &str) -> Value {
    json::object([(key, Value::String(value.into()))])
}

fn convert(f: &mut Fixture, name: &str, target: &str) -> c600_command::Receipt {
    let args = json::object([("recording", Value::String(name.into())), ("target", Value::String(target.into()))]);
    f.shell.invoke(&mut f.layer, RECORDING_CONVERT_ROW, args).expect("prepared")
}

/// The block of the record a conversion wrote, read as FP.17.
fn block_of(f: &Fixture, record: &str) -> provenance::Json {
    let value = f.layer.record(record).unwrap_or_else(|| panic!("{record}"));
    provenance::read(value.provenance.as_bytes()).expect("the conversion's block reads")
}

/// CT-BND-03 (part: workspace record, recording, draft and source conversion; not run: a file, since the layer builds
/// no file export or import of a word, and a macro, since `macro.save` takes an inline word and writes its own block,
/// not the word's).
/// CT-EV4-10 (part: both conversions keep provenance through parts; a source-produced word is a program's,
/// `profile: solving`; not run: no laundering and no old permission, in the next test).
/// The solver writes a word
/// with their own block, previews it from the record and commits it while recording; the recorded word step carries
/// the record's block. The draft conversion is the solver's own word, a `manual` block whose parts name that block,
/// and its preview is admitted; the source conversion's block is a program's, the generated module with entry
/// `replay`, whose parts name the same block.
#[test]
fn ct_bnd_03_a_word_keeps_its_block_through_a_recording_and_its_conversions() {
    let mut f = Fixture::create(Box::new(MemDisk::new()));
    let recorder = f.layer.store().recorder();
    f.layer.set_recorder(Box::new(recorder)).unwrap();
    let own = manual();
    assert!(completed(&f.write("word", &[1, 2], &own)));
    let start = f.shell.invoke(&mut f.layer, RECORDING_START_ROW, named("name", "p")).unwrap();
    assert!(completed(&start));
    let id = f.preview(record_draft("word"));
    let press = f.press(id);
    assert!(matches!(f.shell.send(&mut f.layer, press).status, Status::Committed(_)));
    let stop = f.shell.invoke(&mut f.layer, RECORDING_STOP_ROW, named("recording", "p")).unwrap();
    assert!(completed(&stop));
    let p = f.layer.recording("p").unwrap().clone();
    assert_eq!(p.steps[1].block(), Some(text(&own).as_str()), "the word step carries the record's block");

    assert!(completed(&convert(&mut f, "p", "draft")));
    let draft = block_of(&f, "draft/recording/p");
    assert_eq!((field::producer(&draft), field::parts(&draft)), ("manual", vec![sha256(&own)]));
    let previewed = f.ask(record_draft("draft/recording/p"));
    assert!(matches!(previewed.status, Status::Running(_)), "the solver's own word: {:?}", previewed.status);
    f.layer.run_jobs();

    assert!(completed(&convert(&mut f, "p", "source")));
    let source = block_of(&f, "source/recording/p");
    assert_eq!((field::producer(&source), field::profile(&source)), ("program", "solving"));
    assert_eq!(field::parts(&source), vec![sha256(&own)]);
    let module = f.layer.record("source/recording/p").unwrap().payload.clone();
    let module = module.get("source").and_then(|s| s.get("text")).and_then(Value::as_str).unwrap().to_owned();
    assert!(module.contains("propose(word[1, 2], target(\"operation.preview\"))"), "{module}");
}

/// CT-EV4-10 (part: no laundering of research output into a solving preview; not run: provenance through
/// both conversions, in the test above, and a recorded confirmation never reviving, in `c600-command`). A research
/// run's word, which a recording carries with its block, converts to a draft whose block lists the research block as a
/// part: the reader refuses the draft's preview with the code it gives the research block itself, and the fresh
/// preview of the word step is refused the same way.
#[test]
fn ct_ev4_10_a_research_word_is_not_laundered() {
    let mut f = Fixture::create(Box::new(MemDisk::new()));
    let producer = Producer {
        contract: contract(),
        checker: None,
        author: Author::Own,
        module: sha256(b"research module"),
        entry: "survey".into(),
        profile: Profile::Research,
    };
    let research = provenance::canonical(&provenance::program(&producer, false, &[]));
    f.reader.keep(&research);
    let mut recording = c600_command::Recording {
        name: "laundry".into(),
        session: "s1".into(),
        identity: c600_command::identity_json(&common::kernel().identity()),
        started: Value::Null,
        steps: Vec::new(),
    };
    let word = json::parse(br#"[{"kind": "word", "moves": [3]}]"#).unwrap();
    let committed = json::object([
        ("command", Value::String("shell/0".into())),
        ("request", Value::String("r".into())),
        ("job", Value::Null),
        ("status", json::object([("name", Value::String("committed".into()))])),
    ]);
    recording.steps.push(RecordedStep {
        row: "operation.execute".into(),
        arguments: json::object([]),
        word: Some(json::object([
            ("recipe", word),
            ("source", Value::Null),
            ("provenance", Value::String(text(&research))),
        ])),
        depends: Vec::new(),
        module: None,
        action: Some(1),
        command: "shell/0".into(),
        request: "r".into(),
        outcome: Some(committed),
    });
    // The recording as one read from elsewhere: written to the session's recorder before the layer installs it.
    let mut recorder = f.layer.store().recorder();
    assert_eq!(recorder.save_recording(&recording), Fate::Durable);
    f.layer.set_recorder(Box::new(recorder)).unwrap();

    use c600_command::Provenance;
    let own_refusal = f.reader.admit(&research).expect_err("a research word is not previewed in a solving session");
    assert!(completed(&convert(&mut f, "laundry", "draft")));
    let draft = block_of(&f, "draft/recording/laundry");
    assert_eq!(field::parts(&draft), vec![sha256(&research)]);
    let laundered = f.ask(record_draft("draft/recording/laundry"));
    assert_eq!(refused(&laundered), Some(own_refusal.code), "the part is read at preview");

    let played = f.shell.invoke(&mut f.layer, c600_command::RECORDING_PLAY_ROW, named("recording", "laundry")).unwrap();
    assert!(matches!(played.status, Status::Running(_)));
    f.layer.run_jobs();
    let fresh = f.shell.step_preview(&mut f.layer, "laundry").unwrap_err();
    assert_eq!(fresh.code, own_refusal.code, "the word step's fresh preview");
    assert_ne!(own_refusal.code, code::ROW_NOT_BUILT);
}

/// The labels of a block that reads as FP.17: its form, author, producer, profile and whether it may read the state.
fn labels(block: &[u8]) -> (String, String, String, String, bool) {
    let read = provenance::read(block).expect("the block reads");
    let (form, author, producer) = (field::form(&read), field::author(&read), field::producer(&read));
    (form.into(), author.into(), producer.into(), field::profile(&read).into(), field::may_read_state(&read))
}

/// CT-PS1-17 (part: a state-derived word survives workspace save and read, a power loss and a reopen, a recording and
/// both conversions with its labels under capability separation; not run: rename, since no H-08 row renames a
/// workspace record, and the file round trip, since the layer builds no file export or import of a word). LQ21,
/// RTE.24, PS1.16, PS1.32, BND.24, BND.32 on C5's reader. The word is a built-in solving tool's output whose run read
/// the state (`read_state: yes`). `workspace.set` saves it; the record reads back byte for byte with its labels, and
/// again after a power loss and a reopen. Previewed from its record and committed while recording, its word step
/// carries the block; both conversions list it as a part, which the reader keeps and gives back with the same labels.
/// The draft is admitted at preview, since its part is a built-in tool's output; the source conversion's own block is
/// a program's that reads no state, its part keeping the state-read label. After a second power loss the recording
/// and both conversions still carry the block.
#[test]
fn ct_ps1_17_a_state_derived_word_keeps_its_labels() {
    let disk = MemDisk::new();
    let producer = Producer {
        contract: contract(),
        checker: Some(common::checker()),
        author: Author::BuiltIn,
        module: sha256(b"a built-in tool's module"),
        entry: "derive".into(),
        profile: Profile::Solving,
    };
    let derived = provenance::canonical(&provenance::program(&producer, true, &[]));
    let want = labels(&derived);
    assert_eq!(want, ("capability_separation".into(), "built_in".into(), "program".into(), "solving".into(), true));

    let mut f = Fixture::create(Box::new(disk.clone()));
    assert!(completed(&f.write("derived", &[1, 2], &derived)));
    assert_eq!(f.layer.record("derived").map(|v| v.provenance.clone()), Some(text(&derived)), "read back");
    drop(f);
    disk.crash();
    let mut f = Fixture::open(Box::new(disk.clone()));
    let kept = f.layer.record("derived").expect("the record after a reopen").provenance.clone();
    assert_eq!((kept.as_bytes(), labels(kept.as_bytes())), (&derived[..], want.clone()), "byte for byte");

    let recorder = f.layer.store().recorder();
    f.layer.set_recorder(Box::new(recorder)).unwrap();
    assert!(completed(&f.shell.invoke(&mut f.layer, RECORDING_START_ROW, named("name", "p")).unwrap()));
    let id = f.preview(record_draft("derived"));
    let press = f.press(id);
    assert!(matches!(f.shell.send(&mut f.layer, press).status, Status::Committed(_)));
    assert!(completed(&f.shell.invoke(&mut f.layer, RECORDING_STOP_ROW, named("recording", "p")).unwrap()));
    let p = f.layer.recording("p").unwrap().clone();
    assert_eq!(p.steps[1].block(), Some(text(&derived).as_str()), "the word step carries the block");
    for target in ["draft", "source"] {
        let name = format!("{target}/recording/p");
        assert!(completed(&convert(&mut f, "p", target)), "{target}");
        assert_eq!(field::parts(&block_of(&f, &name)), vec![sha256(&derived)], "{target}");
        let record = f.layer.record(&name).unwrap().provenance.clone();
        let parts = c600_command::Provenance::parts(&f.reader, record.as_bytes());
        assert_eq!(parts, [text(&derived)], "{target}: the part is the block itself");
        assert_eq!(labels(parts[0].as_bytes()), want, "{target}: with its labels");
    }
    let previewed = f.ask(record_draft("draft/recording/p"));
    assert!(matches!(previewed.status, Status::Running(_)), "the draft is admitted: {:?}", previewed.status);
    f.layer.run_jobs();
    let source = block_of(&f, "source/recording/p");
    assert_eq!((field::producer(&source), field::may_read_state(&source)), ("program", false), "the module's own");

    drop(f);
    disk.crash();
    let mut g = Fixture::open(Box::new(disk.clone()));
    let recorder = g.layer.store().recorder();
    g.layer.set_recorder(Box::new(recorder)).unwrap();
    let step = g.layer.recording("p").map(|p| p.steps[1].block().map(str::to_owned));
    assert_eq!(step, Some(Some(text(&derived))), "the recording's word step, after a reopen");
    for target in ["draft", "source"] {
        assert_eq!(field::parts(&block_of(&g, &format!("{target}/recording/p"))), vec![sha256(&derived)], "{target}");
    }
    assert!(matches!(g.ask(record_draft("draft/recording/p")).status, Status::Running(_)), "admitted after a reopen");
}
