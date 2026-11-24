//! The run-time boundary (LP2, M3): provenance blocks, stored words, preview admission and the fixed acknowledgement
//! under capability separation (chapter 07; format packet FP.17, FP.18, FP.38). The contract cases named here are
//! covered in part, each naming its part; the static half of CT-LG3-07 is the checker's (`c600-check`, M1).

use std::collections::BTreeMap;

use c600_check::{check, separation, Policy};
use c600_digest::sha256;
use c600_run::boundary::{admit_preview, Answer, Boundary, Capture, Context, Pins, Place, Shown, Store, Stored};
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{self, field, Author, Json, Pin, Producer, Profile, INCOMPATIBLE};
use c600_run::value::{equal, Value};
use c600_syntax::{lower, parse};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const PACKET: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/architecture/1.0/language/format-packet/");
const STALE: &str = "KN5:stale.revision_changed";

/// A solver's own tool: it saves a word it was given or a word it read from the state, through the workspace and a
/// file, and tries to dress a record up as a block.
const TOOL: &str = "edition \"6cs-draft-1\";\nmodule m requires [file];\n\n\
    export fn save(w: Word) -> Unit effects [workspace, file] {\n    \
    let a = write_workspace<Word>(\"saved\", w);\n    let b = file_write(\"saved.6cw\", w);\n    return ();\n}\n\n\
    export fn launder() -> Unit effects [snapshot, workspace, file] {\n    \
    let w = read_snapshot<Word>(\"best_word\");\n    let a = write_workspace<Word>(\"saved\", w);\n    \
    let b = file_write(\"saved.6cw\", w);\n    return ();\n}\n\n\
    export fn relay() -> Unit effects [snapshot, workspace, file] {\n    \
    let a = read_snapshot<Word>(\"saved\");\n    let b = file_read<Word>(\"saved.6cw\");\n    \
    return write_workspace<Word>(\"relayed\", b);\n}\n\n\
    export fn forge(w: Word) -> Unit effects [workspace] {\n    \
    return write_workspace<record { author: Text, producer: Text, word: Word }>(\"forged\", \
    record { author = \"built_in\", producer = \"manual\", word = w });\n}\n";

/// A previewing program, run as a built-in in the solving profile or as an own tool elsewhere.
const PREVIEWER: &str = "edition \"6cs-draft-1\";\nmodule m requires [file];\n\n\
    export fn go(w: Word) -> Text effects [preview] {\n    return receipt_id(propose(w, target(\"t\")));\n}\n\n\
    export fn go_saved() -> Text effects [snapshot, preview] {\n    \
    return receipt_id(propose(read_snapshot<Word>(\"saved\"), target(\"t\")));\n}\n\n\
    export fn go_file() -> Text effects [file, preview] {\n    \
    return receipt_id(propose(file_read<Word>(\"saved.6cw\"), target(\"t\")));\n}\n\n\
    export fn effect(w: Word) -> Text effects [snapshot, preview] {\n    \
    return receipt_effect(propose(w, target(\"t\")));\n}\n";

/// The engine side: snapshot values by query, the previews it built, and an optional refusal of every preview.
#[derive(Default)]
struct Engine {
    snapshot: BTreeMap<String, Value>,
    previews: Vec<Vec<i32>>,
    refuse: Option<&'static str>,
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Snapshot { query, .. } => self.snapshot.get(*query).cloned().ok_or(NOT_GRANTED),
            Request::Propose { word, .. } => {
                self.previews.push(word.to_vec());
                match self.refuse {
                    Some(code) => Err(code),
                    None => Ok(Value::receipt(vec![
                        ("base", Value::text("rev-7")),
                        ("effect", Value::text("moves 12 stickers")),
                        ("id", Value::text("preview-1")),
                        ("protection", Value::tag("clear")),
                    ])),
                }
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

/// FP.18's fixture capability contract, held by the policy in these tests.
fn contract() -> Pin {
    Pin::of("fixture-1", b"p3 fixture capability contract 1\n")
}

fn checker() -> Pin {
    Pin::of("m1-draft", b"c600-check fixture\n")
}

fn pins() -> Pins {
    Pins { contracts: vec![contract()], checkers: vec![checker()] }
}

fn producer(source: &str, entry: &str, author: Author, profile: Profile) -> Producer {
    Producer {
        contract: contract(),
        checker: Some(checker()),
        author,
        module: sha256(source.as_bytes()),
        entry: entry.into(),
        profile,
    }
}

fn manual() -> Vec<u8> {
    provenance::canonical(&provenance::manual(&contract()))
}

fn imported() -> Vec<u8> {
    provenance::canonical(&provenance::import(&contract(), b"[{\"kind\":\"word\",\"moves\":[1,-1]}]"))
}

/// The block of a program's output that took in nothing.
fn output_of(author: Author, profile: Profile) -> Vec<u8> {
    provenance::canonical(&provenance::program(&producer(TOOL, "save", author, profile), false, &[]))
}

/// Parses, lowers, checks and loads one module `m`; gives the entry's static state-read fact.
fn build(source: &str, entry: &str) -> (Loaded, bool) {
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let reads = separation(report.summary("m", entry).expect("the entry"), &Policy::interim()).reads_state;
    (load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}")), reads)
}

struct Ran {
    run: Run,
    shell: Vec<Shown>,
    captures: Vec<Capture>,
}

/// Runs `m.<entry>` of `source` through a boundary, as the given author and profile.
fn run(
    source: &str,
    entry: &str,
    (author, profile): (Author, Profile),
    args: Vec<Value>,
    inputs: Vec<Vec<u8>>,
    engine: &mut Engine,
    store: &mut Store,
) -> Ran {
    let (loaded, reads_state) = build(source, entry);
    let context = Context {
        run: "run".into(),
        base: "rev-7".into(),
        producer: producer(source, entry, author, profile),
        reads_state,
        inputs,
        pins: pins(),
    };
    let mut boundary = Boundary::new(engine, store, context);
    let run = loaded.run("m", entry, args, &mut boundary, &Options::new(WIDE)).unwrap_or_else(|r| panic!("{r:?}"));
    Ran { run, shell: boundary.shell, captures: boundary.captures }
}

fn block_at(store: &Store, place: &Place) -> Json {
    provenance::read(&store.records[place].block).expect("the host's block reads")
}

fn ws(name: &str) -> Place {
    Place::Workspace(name.into())
}

fn file(name: &str) -> Place {
    Place::File(name.into())
}

fn word() -> Vec<Value> {
    vec![Value::word(&[1, -2])]
}

fn edit(json: &Json, key: &str, value: Option<Json>) -> Json {
    let Json::Object(mut map) = json.clone() else { panic!("an object") };
    match value {
        Some(v) => map.insert(key.into(), v),
        None => map.remove(key),
    };
    Json::Object(map)
}

/// The frozen digest of a provenance vector in `vectors.json`.
fn frozen_digest(vectors: &str, name: &str) -> String {
    let section = &vectors[vectors.find("\n  \"provenance\": {").expect("the provenance vectors")..];
    let entry = &section[section.find(&format!("\"{name}\": {{")).expect("the vector")..];
    let key = "\"expected_sha256\": \"";
    let at = entry.find(key).expect("a digest") + key.len();
    entry[at..at + 64].to_string()
}

/// CT-FP-02 (part: complete manual and import blocks under both forms, a pick with no resolving record, a missing
/// profile, an incompatible form and no permission; the non-null built-in pick under each form, which needs the
/// packet's `c600-pick` records, runs in `c600-session`'s `tests/persist_picks.rs`) and CT-BND-22 (part: flow-form
/// blocks stay in their form; the row runs in `conformance_bnd.rs`), with FP.17, FP.18 and FP.38: the four example
/// blocks read and digest as frozen, the runtime builds the capability-form ones byte for byte, their policy use
/// without the fixture pin is refused with `PS1:capability.provenance_incompatible`, and a malformed block never reads.
#[test]
fn fp18_blocks_validate_and_match_the_frozen_digests() {
    let examples = provenance::parse(&std::fs::read(format!("{PACKET}examples.json")).unwrap()).expect("examples");
    let vectors = String::from_utf8(std::fs::read(format!("{PACKET}vectors.json")).unwrap()).unwrap();
    let records = examples.get("records").unwrap();
    let block = |name: &str| records.get(name).and_then(|r| r.get("provenance")).unwrap().clone();
    let names = ["manual_flow", "imported_flow", "manual_capability", "imported_capability"];
    for name in names {
        let bytes = provenance::canonical(&block(name));
        assert_eq!(provenance::hex(&sha256(&bytes)), frozen_digest(&vectors, name), "{name}");
        assert_eq!(provenance::read(&bytes), Ok(block(name)), "{name}");
    }

    let text = |json: &Json, key: &str| json.get(key).and_then(Json::as_text).unwrap().to_string();
    let fixtures = examples.get("fixture_contracts").unwrap();
    let capability = Pin::of("fixture-1", text(fixtures, "capability_separation").as_bytes());
    let flow = Pin::of("fixture-1", text(fixtures, "flow_static").as_bytes());
    assert_eq!(capability, contract());
    let legacy = text(examples.get("sources").unwrap(), "legacy_recipe");
    assert_eq!(provenance::manual(&capability), block("manual_capability"));
    assert_eq!(provenance::import(&capability, legacy.as_bytes()), block("imported_capability"));
    assert_eq!(provenance::canonical(&block("imported_capability")), imported());

    // Valid storage, refused use: the fixture pins are no policy in force (FP.18). With both pins held, the
    // capability-form blocks pass BND.84's producer check and the flow-form ones stay in another form (CT-BND-22).
    let store = Store::default();
    let held = Pins { contracts: vec![capability, flow], checkers: Vec::new() };
    for name in names {
        let bytes = provenance::canonical(&block(name));
        assert_eq!(admit_preview(&bytes, &store, &Pins::default()).unwrap_err().code, INCOMPATIBLE, "{name}");
        let held = admit_preview(&bytes, &store, &held);
        if name.ends_with("_capability") {
            assert_eq!(held, Ok(()), "{name}");
        } else {
            assert_eq!(held.unwrap_err().code, INCOMPATIBLE, "{name}");
        }
    }

    // A pick whose `c600-pick` record does not resolve refuses (FP.38, vector `pick-no-record`).
    let digest = provenance::digest_ref("content", &[7; 32]);
    let reference =
        Json::object([("format", Json::text("c600-pick")), ("version", Json::Int(1)), ("content", digest.clone())]);
    let picked = edit(
        &provenance::program(&producer(TOOL, "save", Author::BuiltIn, Profile::Solving), false, &[]),
        "pick",
        Some(Json::object([("record", reference), ("chosen", digest)])),
    );
    let picked = provenance::canonical(&picked);
    assert!(provenance::read(&picked).is_ok());
    assert_eq!(admit_preview(&picked, &store, &pins()).unwrap_err().code, INCOMPATIBLE);

    // Malformed blocks: every one fails FP.17 and reads `unknown`, so its use is refused (BND.21).
    let good = block("manual_capability");
    let meta = good.get("block").unwrap();
    let malformed = [
        edit(&good, "legacy", None),
        edit(&good, "block", Some(edit(meta, "version", Some(Json::Int(2))))),
        edit(&good, "author", Some(Json::text("someone"))),
        edit(&good, "depends", Some(Json::object([("kind", Json::text("not_recorded"))]))),
        edit(&good, "depends", Some(Json::object([("read_state", Json::text("maybe"))]))),
        edit(&good, "profile", Some(Json::text("solving"))),
        edit(&good, "profile", None),
    ];
    for bad in &malformed {
        let bytes = provenance::canonical(bad);
        assert!(provenance::read(&bytes).is_err(), "{bad:?}");
        assert_eq!(admit_preview(&bytes, &store, &pins()).unwrap_err().code, INCOMPATIBLE);
    }
    let spaced = String::from_utf8(manual()).unwrap().replace(':', ": ");
    for text in [spaced.as_str(), "{\"a\":1,\"a\":2}", "\"\\ud800\"", "1e0", "01", "[1,]"] {
        assert!(provenance::read(text.as_bytes()).is_err(), "{text}");
    }
    assert!(provenance::parse(b"{\"a\":[1,-2,\"\\u00e9\\ud83d\\ude00\"],\"b\":null}").is_some());
}

/// CT-BND-01 (part: workspace and file writes and a forged record; the whole row runs in `conformance_bnd.rs`),
/// CT-BND-03 (part: the workspace record and file routes; a recording and both conversions, draft and source, run in
/// `c600-durable`'s `tests/ct_bnd_03_recording_blocks.rs` (LP4-9); not run: macros, which the command layer and
/// the session build (LP3, M6)) and CT-BND-04 (part: two parts read through the workspace and a
/// file; the whole row runs in `conformance_bnd.rs`): a stored word carries the block the host built, whatever the
/// program writes, and a later run that reads it through the workspace and a file cites both blocks in read order.
#[test]
fn stored_words_carry_the_host_block_and_later_runs_cite_it() {
    let (mut engine, mut store) = (Engine::default(), Store::default());
    let own = (Author::Own, Profile::ToolAuthoring);
    let saved = run(TOOL, "save", own, word(), vec![manual()], &mut engine, &mut store);
    assert_eq!(saved.run.summary.outcome, Outcome::Completed);
    let expected =
        provenance::program(&producer(TOOL, "save", Author::Own, Profile::ToolAuthoring), false, &[sha256(&manual())]);
    for place in [ws("saved"), file("saved.6cw")] {
        assert_eq!(block_at(&store, &place), expected);
        assert!(equal(&store.records[&place].value, &Value::word(&[1, -2])));
        assert_eq!(store.records[&place].bytes, b"[1,-2]\n");
    }
    assert_eq!(store.blocks.get(&sha256(&manual())), Some(&manual()), "the input's block is kept");

    // A record that imitates a block is a value; the stored block is still the host's (BND.22).
    run(TOOL, "forge", own, word(), vec![manual()], &mut engine, &mut store);
    let forged = block_at(&store, &ws("forged"));
    assert_eq!((field::author(&forged), field::producer(&forged)), ("own", "program"));
    assert_eq!(
        forged,
        provenance::program(&producer(TOOL, "forge", Author::Own, Profile::ToolAuthoring), false, &[sha256(&manual())])
    );

    // `relay` reads the workspace record, then the file: both blocks are parts, in read order. Its entry holds a
    // snapshot read, so `read_state` is `yes` although neither part read the state.
    let first = store.records[&ws("saved")].block.clone();
    store.records.get_mut(&file("saved.6cw")).unwrap().block = output_of(Author::BuiltIn, Profile::Solving);
    let second = store.records[&file("saved.6cw")].block.clone();
    run(TOOL, "relay", own, vec![], vec![], &mut engine, &mut store);
    let relayed = block_at(&store, &ws("relayed"));
    assert_eq!(field::parts(&relayed), [sha256(&first), sha256(&second)]);
    assert_eq!(relayed.get("depends"), Some(&Json::object([("read_state", Json::text("yes"))])));
    assert!(equal(&store.records[&ws("relayed")].value, &Value::word(&[1, -2])));
}

/// CT-BND-02 (part: a damaged or legacy part joins as `read_state: yes` and is refused at preview admission; not run:
/// the session's tables (M6); the command layer's rows are in `c600-command/tests/kn5_admission.rs`): a damaged or
/// legacy part never reads as `read_state: no`, and a damaged block is kept but never admitted as `own` or `manual`
/// (BND.21, FP.17). CT-PS1-18 (part: the run-level join of a legacy part; the rest is in `legacy.rs`).
#[test]
fn damaged_and_legacy_parts_never_read_as_no() {
    let solving = (Author::BuiltIn, Profile::Solving);
    let (loaded_reads_state, damaged) = (build(PREVIEWER, "go_file").1, {
        let mut bytes = manual();
        bytes.truncate(bytes.len() - 2);
        bytes
    });
    assert!(!loaded_reads_state, "the checker cannot see what a granted file holds");
    let mut cases = Vec::new();
    for (block, read_state, admitted) in
        [(manual(), "no", true), (imported(), "yes", true), (damaged.clone(), "yes", false)]
    {
        let (mut engine, mut store) = (Engine::default(), Store::default());
        let stored = Stored { value: Value::word(&[3]), bytes: b"[3]\n".to_vec(), block: block.clone() };
        store.records.insert(file("saved.6cw"), stored);
        let ran = run(PREVIEWER, "go_file", solving, vec![], vec![], &mut engine, &mut store);
        let shown = &ran.shell[0];
        let proposed = provenance::read(&store.blocks[&shown.block]).unwrap();
        assert_eq!(proposed.get("depends"), Some(&Json::object([("read_state", Json::text(read_state))])));
        assert_eq!(field::parts(&proposed), [sha256(&block)]);
        assert_eq!(store.blocks.get(&sha256(&block)), Some(&block), "the bytes are kept (PS1.33)");
        match (&shown.answer, admitted) {
            (Answer::Previewed(_), true) => {}
            (Answer::Refused(r), false) => assert_eq!(r.code, INCOMPATIBLE),
            (answer, _) => panic!("{answer:?}"),
        }
        cases.push(ran.run.output);
    }
    assert!(cases.iter().all(|o| o.as_deref() == Some(&b"\"run/1\"\n"[..])));
}

/// CT-BND-07 (part: a built preview, an admission refusal and a stale refusal; the whole row runs in
/// `conformance_bnd.rs`), CT-BND-11 (part: the solving profile; not run: ONE.32's fields in the other profiles, which
/// never arise, since BND.60 makes every proposal there a capture) and CT-BND-12 (part: the program's acknowledgement
/// and the shell's record; not run: the shell's display of reason and remedy, which no crate renders yet), with BND.42
/// to BND.44: in the solving profile a
/// previewing program receives one fixed acknowledgement, `{base, id}`, whether the preview was built, refused at
/// admission or refused by the engine. Its output and every charge row are the same; the shell keeps the full answer.
/// An outcome field is not granted to it.
#[test]
fn a_solving_run_gets_one_fixed_acknowledgement() {
    assert!(!build(PREVIEWER, "go").1, "reading a receipt's ID reads no state (BND.42)");
    let solving = (Author::BuiltIn, Profile::Solving);
    let tool_word = output_of(Author::Own, Profile::ToolAuthoring);
    let cases = [(manual(), None), (tool_word.clone(), None), (manual(), Some(STALE))];
    let mut summaries = Vec::new();
    for (entry, outputs) in [("go", Some(&b"\"run/1\"\n"[..])), ("effect", None)] {
        for (i, (input, refuse)) in cases.iter().enumerate() {
            let mut engine = Engine { refuse: *refuse, ..Engine::default() };
            let ran = run(PREVIEWER, entry, solving, word(), vec![input.clone()], &mut engine, &mut Store::default());
            assert_eq!(ran.run.output.as_deref(), outputs, "{entry} {i}");
            if outputs.is_none() {
                assert_eq!((ran.run.summary.outcome, ran.run.summary.code), (Outcome::Error, Some(NOT_GRANTED)));
                assert!(ran.run.summary.stop.as_ref().unwrap().at.ends_with("#read_receipt"));
            }
            let shown = &ran.shell[0];
            assert_eq!((shown.id.as_str(), shown.word.as_slice(), shown.target.as_str()), ("run/1", &[1, -2][..], "t"));
            match (i, &shown.answer) {
                (0, Answer::Previewed(receipt)) => {
                    let Value::Receipt(r) = receipt else { panic!("a receipt") };
                    assert!(r.get("effect").is_some() && r.get("protection").is_some());
                    assert_eq!(engine.previews, [vec![1, -2]]);
                }
                (1, Answer::Refused(r)) => {
                    assert_eq!(r.code, NOT_GRANTED);
                    assert!(engine.previews.is_empty(), "admission installs no preview (CT-BND-27)");
                }
                (2, Answer::Refused(r)) => assert_eq!((r.code, engine.previews.len()), (STALE, 1)),
                (_, answer) => panic!("{i}: {answer:?}"),
            }
            summaries.push(ran.run.summary);
        }
    }
    assert!(summaries[..3].iter().all(|s| s == &summaries[0]));
    assert!(summaries[3..].iter().all(|s| s == &summaries[3]));
}

/// CT-BND-14 (part: the engine and the program; the command layer's refusal and the session's digest are in
/// `c600-command`'s `tests/profile.rs`),
/// with BND.60: in tool authoring and research, `propose` goes to a capture. The engine builds no preview, the capture
/// keeps the word and its block for a test to read, and the program gets the acknowledgement.
#[test]
fn other_profiles_capture_and_build_no_preview() {
    for (author, profile) in [(Author::Own, Profile::ToolAuthoring), (Author::BuiltIn, Profile::Research)] {
        let (mut engine, mut store) = (Engine::default(), Store::default());
        let ran = run(PREVIEWER, "go", (author, profile), word(), vec![manual()], &mut engine, &mut store);
        assert_eq!(ran.run.output.as_deref(), Some(&b"\"run/1\"\n"[..]));
        assert!(engine.previews.is_empty() && ran.shell.is_empty());
        let capture = &ran.captures[0];
        assert_eq!(
            (capture.id.as_str(), capture.word.as_slice(), capture.target.as_str()),
            ("run/1", &[1, -2][..], "t")
        );
        let block = provenance::read(&store.blocks[&capture.block]).unwrap();
        assert_eq!((field::author(&block), field::profile(&block)), (author.name(), profile.name()));
        assert_eq!(field::parts(&block), [sha256(&manual())]);

        let (mut engine, mut store) = (Engine::default(), Store::default());
        let ran = run(PREVIEWER, "effect", (author, profile), word(), vec![manual()], &mut engine, &mut store);
        assert_eq!(ran.run.summary.code, Some(NOT_GRANTED), "a capture has no outcome");
        assert!(engine.previews.is_empty() && ran.captures.len() == 1);
    }
}

/// CT-LG3-07 (part: laundering through a saved word, refused at the runtime's preview admission; the checker's half
/// runs in `c600-check`'s `ct_lg3_07_*` tests; not run: a state-reading built-in previewer, since the built-in tool
/// column waits for the class review (BND.37, `rust/README.md` M1)), CT-BND-13 (whole: research output directly,
/// through the workspace and through a file) and CT-BND-26 to CT-BND-30 (parts; their rows run in
/// `conformance_bnd.rs`), with BND.61, BND.84 and BND.85: a word a solver's tool saved, through the workspace or a
/// file, is refused when a solving run proposes it, at preview admission and with `LG3:capability.not_granted`; so is
/// research output, a tool block nested in the parts, and a part whose content is missing. A hand-entered word and an
/// imported macro pass. The flow rule stays off (CT-BND-24): reading the state is recorded, not refused.
#[test]
fn saved_tool_words_are_refused_at_preview_admission() {
    let solving = (Author::BuiltIn, Profile::Solving);
    let previews = |store: &mut Store, entry: &str| {
        let mut engine = Engine::default();
        let ran = run(PREVIEWER, entry, solving, vec![], vec![], &mut engine, store);
        assert_eq!(ran.run.output.as_deref(), Some(&b"\"run/1\"\n"[..]), "the fixed acknowledgement");
        match &ran.shell[0].answer {
            Answer::Previewed(_) => Ok(engine.previews.len()),
            Answer::Refused(r) => {
                assert!(engine.previews.is_empty(), "no preview is installed");
                Err(r.code)
            }
        }
    };

    // A tool reads the state and saves the word: through the workspace and through a file, the word is refused.
    for author in [Author::Own, Author::ForkedBuiltIn, Author::Imported] {
        let mut store = Store::default();
        let mut engine = Engine::default();
        engine.snapshot.insert("best_word".into(), Value::word(&[5, -6]));
        let tool = (author, Profile::ToolAuthoring);
        assert_eq!(
            run(TOOL, "launder", tool, vec![], vec![], &mut engine, &mut store).run.summary.outcome,
            Outcome::Completed
        );
        assert_eq!(
            block_at(&store, &ws("saved")).get("depends"),
            Some(&Json::object([("read_state", Json::text("yes"))]))
        );
        assert_eq!(previews(&mut store, "go_saved"), Err(NOT_GRANTED), "{author:?} through the workspace");
        assert_eq!(previews(&mut store, "go_file"), Err(NOT_GRANTED), "{author:?} through a file");
    }

    // Research output, directly, through the workspace and through a file (BND.61).
    let research = output_of(Author::BuiltIn, Profile::Research);
    let mut engine = Engine::default();
    let direct = run(PREVIEWER, "go", solving, word(), vec![research.clone()], &mut engine, &mut Store::default());
    assert!(matches!(&direct.shell[0].answer, Answer::Refused(r) if r.code == NOT_GRANTED));
    let mut store = Store::default();
    run(TOOL, "save", (Author::BuiltIn, Profile::Research), word(), vec![manual()], &mut engine, &mut store);
    assert_eq!(previews(&mut store, "go_saved"), Err(NOT_GRANTED));
    assert_eq!(previews(&mut store, "go_file"), Err(NOT_GRANTED));

    // Stored words whose blocks the test plants: what passes BND.84's producer check and what does not.
    let nested = {
        let inner = output_of(Author::Own, Profile::ToolAuthoring);
        let block =
            provenance::program(&producer(TOOL, "save", Author::BuiltIn, Profile::Solving), false, &[sha256(&inner)]);
        (provenance::canonical(&block), Some(inner))
    };
    let missing = {
        let block = provenance::program(&producer(TOOL, "save", Author::BuiltIn, Profile::Solving), false, &[[9; 32]]);
        (provenance::canonical(&block), None)
    };
    let unknown = {
        let block = edit(&provenance::manual(&contract()), "author", Some(Json::text("unknown")));
        (provenance::canonical(&block), None)
    };
    let cases = [
        ("hand-entered", (manual(), None), Ok(1)),
        ("imported macro", (imported(), None), Ok(1)),
        ("built-in output", (output_of(Author::BuiltIn, Profile::Solving), None), Ok(1)),
        ("nested tool block", nested, Err(NOT_GRANTED)),
        ("missing part", missing, Err(NOT_GRANTED)),
        ("unknown author", unknown, Err(NOT_GRANTED)),
    ];
    for (what, (block, extra), expected) in cases {
        for entry in ["go_saved", "go_file"] {
            let mut store = Store::default();
            if let Some(extra) = &extra {
                store.keep(extra);
            }
            let place = if entry == "go_saved" { ws("saved") } else { file("saved.6cw") };
            store
                .records
                .insert(place, Stored { value: Value::word(&[3]), bytes: b"[3]\n".to_vec(), block: block.clone() });
            assert_eq!(previews(&mut store, entry), expected, "{what} via {entry}");
        }
    }
}
