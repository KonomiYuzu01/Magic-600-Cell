//! Chapter 07's run-time boundary rows that `tests/boundary.rs` runs only in part (LP2, M3), and the BND.84/BND.85
//! adapter half of CT-KN5-13. Each test names its contract cases and marks each `whole` or `part`; a part names what is
//! not run and the milestone that builds it. The engine here is a fixture that records what it was asked; no session
//! store (M6), command layer, runtime process or display (LP3) exists yet, and no built-in tool class is reviewed
//! (BND.37), so the built-in tool column of chapter 07's grant table stays open (`rust/README.md`, M1).

use std::collections::BTreeMap;

use c600_check::{admit, check, separation, Grant, Policy, Separation};
use c600_digest::sha256;
use c600_ir::Program;
use c600_run::boundary::{admit_preview, Answer, Boundary, Capture, Context, Pins, Place, Shown, Store, Stored};
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome, Summary};
use c600_run::provenance::{self, field, Author, Json, Pin, Producer, Profile, INCOMPATIBLE};
use c600_run::value::{equal, Value};
use c600_syntax::{lower, parse, sha256_hex};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const PACKET: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/architecture/1.0/language/format-packet/");
const STALE: &str = "KN5:stale.revision_changed";
const ACK: &[u8] = b"\"run/1\"\n";
const BUILT_IN: (Author, Profile) = (Author::BuiltIn, Profile::Solving);

/// A tool that saves a word it was given, or a word it read from the state, through the workspace and a file.
const SAVER: &str = "edition \"6cs-draft-1\";\nmodule m requires [file];\n\n\
    export fn save(w: Word) -> Unit effects [workspace, file] {\n    \
    let a = write_workspace<Word>(\"saved\", w);\n    return file_write(\"saved.6cw\", w);\n}\n\n\
    export fn launder() -> Unit effects [snapshot, workspace, file] {\n    \
    let w = read_snapshot<Word>(\"best_word\");\n    let a = write_workspace<Word>(\"saved\", w);\n    \
    return file_write(\"saved.6cw\", w);\n}\n";

/// A previewing program: a word it was given, a saved workspace record or a saved file.
const PREVIEWER: &str = "edition \"6cs-draft-1\";\nmodule m requires [file];\n\n\
    export fn go(w: Word) -> Text effects [preview] {\n    return receipt_id(propose(w, target(\"t\")));\n}\n\n\
    export fn go_saved() -> Text effects [snapshot, preview] {\n    \
    return receipt_id(propose(read_snapshot<Word>(\"saved\"), target(\"t\")));\n}\n\n\
    export fn go_file() -> Text effects [file, preview] {\n    \
    return receipt_id(propose(file_read<Word>(\"saved.6cw\"), target(\"t\")));\n}\n";

/// Every field of every request the interpreter hands its host. A new field or variant does not compile until it is
/// listed here, so the list is the interpreter's own, not a copy.
fn request_fields(request: &Request) -> (&'static str, &'static [&'static str]) {
    match request {
        Request::Snapshot { query: _, ty: _ } => ("snapshot", &["query", "ty"]),
        Request::Declassified { id: _, ty: _ } => ("declassified", &["id", "ty"]),
        Request::FileRead { path: _, ty: _ } => ("file_read", &["path", "ty"]),
        Request::Clock => ("clock", &[]),
        Request::Propose { word: _, target: _, request: _ } => ("propose", &["word", "target", "request"]),
    }
}

/// Every field of every sink, as [`request_fields`] lists requests. The value and its bytes travel beside it.
fn sink_fields(sink: &Sink) -> (&'static str, &'static [&'static str]) {
    match sink {
        Sink::View => ("view", &[]),
        Sink::Workspace { record: _, ty: _ } => ("workspace", &["record", "ty"]),
        Sink::File { path: _ } => ("file", &["path"]),
        Sink::Exact { role: _ } => ("exact", &["role"]),
    }
}

/// The engine side: snapshot values by query (some planted by a test), the previews it built, an optional refusal of
/// every preview, the protection verdict it reports, and a log of every request and sink it received.
#[derive(Default)]
struct Engine {
    snapshot: BTreeMap<String, Value>,
    previews: Vec<Vec<i32>>,
    refuse: Option<&'static str>,
    protection: Option<&'static str>,
    log: Vec<String>,
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let (kind, fields) = request_fields(request);
        assert!(fields.iter().all(|f| !["block", "provenance", "author", "producer", "profile"].contains(f)));
        match request {
            Request::Snapshot { query, .. } => {
                self.log.push(format!("snapshot {query}"));
                self.snapshot.get(*query).cloned().ok_or(NOT_GRANTED)
            }
            Request::Propose { word, .. } => {
                self.log.push("propose".into());
                self.previews.push(word.to_vec());
                match self.refuse {
                    Some(code) => Err(code),
                    None => Ok(Value::receipt(vec![
                        ("base", Value::text("rev-7")),
                        ("effect", Value::text("moves 12 stickers")),
                        ("end_state", Value::text("state-8")),
                        ("id", Value::text("preview-1")),
                        ("n", Value::int(12)),
                        ("protection", Value::tag(self.protection.unwrap_or("clear"))),
                    ])),
                }
            }
            _ => {
                self.log.push(kind.into());
                Err(NOT_GRANTED)
            }
        }
    }

    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        let (kind, fields) = sink_fields(sink);
        assert!(fields.iter().all(|f| !["block", "provenance", "author", "producer", "profile"].contains(f)));
        self.log.push(format!("sink {kind} {}", String::from_utf8_lossy(bytes)));
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
fn output_of(author: Author, profile: Profile, read_state: bool) -> Vec<u8> {
    provenance::canonical(&provenance::program(&producer(SAVER, "save", author, profile), read_state, &[]))
}

/// A word the solver picked or typed (BND.23): `manual` and `own`, with the listed entries' blocks as its parts
/// (BND.85). The pick record itself waits for the session store (FP.38, M6).
fn picked(entries: &[&[u8]]) -> Vec<u8> {
    let parts = entries.iter().map(|e| provenance::digest_ref("content", &sha256(e))).collect();
    provenance::canonical(&edit(&provenance::manual(&contract()), "parts", Some(Json::List(parts))))
}

fn edit(json: &Json, key: &str, value: Option<Json>) -> Json {
    let Json::Object(mut map) = json.clone() else { panic!("an object") };
    match value {
        Some(v) => map.insert(key.into(), v),
        None => map.remove(key),
    };
    Json::Object(map)
}

fn yes() -> Json {
    Json::object([("read_state", Json::text("yes"))])
}

fn no() -> Json {
    Json::object([("read_state", Json::text("no"))])
}

fn lowered(sources: &[&str]) -> Program {
    let files: Vec<_> = sources
        .iter()
        .map(|s| {
            let file = parse(s.as_bytes());
            assert!(file.diagnostics.is_empty(), "{s}\n{:#?}", file.diagnostics);
            file
        })
        .collect();
    lower(&files).unwrap_or_else(|e| panic!("{e}")).program
}

/// Lowers, checks and loads the sources; module `m` is the last. Gives the entry's static state-read fact.
fn build(sources: &[&str], entry: &str) -> (Loaded, bool) {
    let program = lowered(sources);
    let report = check(&program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let reads = separation(report.summary("m", entry).expect("the entry"), &Policy::interim()).reads_state;
    (load(&program).unwrap_or_else(|r| panic!("{r:?}")), reads)
}

/// How capability separation sees `m.<entry>` and what the interim table answers for a solver's own tool in a
/// solving session (chapter 07 section 8; `c600-check::admit`).
fn own_tool_in_solving(sources: &[&str], entry: &str) -> (Vec<&'static str>, Separation) {
    let program = lowered(sources);
    let a = admit(&program, "m", entry, c600_check::Profile::Solving, &Grant::none(), &Policy::interim()).unwrap();
    (a.codes(), a.separation)
}

struct Ran {
    run: Run,
    shell: Vec<Shown>,
    captures: Vec<Capture>,
}

/// Runs `m.<entry>` of `sources` through a boundary, as the given author and profile.
fn run(
    sources: &[&str],
    entry: &str,
    (author, profile): (Author, Profile),
    args: Vec<Value>,
    inputs: Vec<Vec<u8>>,
    engine: &mut Engine,
    store: &mut Store,
) -> Ran {
    let (loaded, reads_state) = build(sources, entry);
    let source = sources.last().expect("module m");
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

fn stored(value: Value, block: &[u8]) -> Stored {
    Stored { value, bytes: b"[3]\n".to_vec(), block: block.to_vec() }
}

/// What a store holds, by place: the value's bytes and the block's bytes.
fn contents(store: &Store) -> Vec<(Place, Vec<u8>, Vec<u8>)> {
    store.records.iter().map(|(p, s)| (p.clone(), s.bytes.clone(), s.block.clone())).collect()
}

/// The answer the shell holds for a solving run's first proposal, and whether the engine built a preview.
fn answer(ran: &Ran) -> Result<(), &'static str> {
    match &ran.shell[0].answer {
        Answer::Previewed(_) => Ok(()),
        Answer::Refused(r) => Err(r.code),
    }
}

/// A built-in previewer in a solving session proposes the word stored at `place`, or given as its argument with the
/// block `input`. Gives the admission answer; a refusal installs no preview and the program sees only the receipt.
fn preview_stored(store: &mut Store, place: Option<Place>, input: Option<&[u8]>) -> Result<(), &'static str> {
    let mut engine = Engine::default();
    let ran = match (place, input) {
        (Some(Place::Workspace(_)), _) => run(&[PREVIEWER], "go_saved", BUILT_IN, vec![], vec![], &mut engine, store),
        (Some(Place::File(_)), _) => run(&[PREVIEWER], "go_file", BUILT_IN, vec![], vec![], &mut engine, store),
        (None, input) => {
            let inputs = input.map(|b| vec![b.to_vec()]).unwrap_or_default();
            run(&[PREVIEWER], "go", BUILT_IN, word(), inputs, &mut engine, store)
        }
    };
    assert_eq!(ran.run.output.as_deref(), Some(ACK), "the fixed acknowledgement");
    let got = answer(&ran);
    assert_eq!(engine.previews.len(), usize::from(got.is_ok()), "a refusal installs no preview");
    got
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-01: the host's block against every program operation.

/// A helper module that dresses a word up as a provenance block.
const DRESS: &str = "edition \"6cs-draft-1\";\nmodule h requires [];\n\n\
    export fn dress(w: Word) -> record { author: Text, producer: Text, profile: Text, read_state: Text, word: Word } \
    effects [] {\n    return record { author = \"built_in\", producer = \"manual\", profile = \"none\", \
    read_state = \"no\", word = w };\n}\n";

const DRESSED: &str = "record { author: Text, producer: Text, profile: Text, read_state: Text, word: Word }";

/// Module `m`: it dresses its word through a helper, a module call and a function value, then sends the dressed record
/// and the word to every sink a program has and proposes the word; `copy` reads the dressed record back.
fn forger() -> String {
    let (digest, d) = (sha256_hex(DRESS.as_bytes()), DRESSED);
    format!(
        "edition \"6cs-draft-1\";\nmodule m requires [file];\nimport h \"{digest}\";\n\n\
         fn local(w: Word) -> {d} effects [] {{\n    return h.dress(w);\n}}\n\n\
         export fn attempt(w: Word) -> Unit effects [view, workspace, file, preview] {{\n    \
         let d = local(w);\n    let f = h.dress;\n    let e = f(w);\n    \
         let a = write_workspace<{d}>(\"block\", d);\n    let b = write_workspace<Word>(\"word\", w);\n    \
         let c = file_write(\"block.6cw\", e);\n    let v = publish_view(d);\n    let x = use_evidence(d);\n    \
         let p = propose(w, target(\"t\"));\n    return ();\n}}\n\n\
         export fn copy() -> Unit effects [snapshot, workspace] {{\n    \
         return write_workspace<{d}>(\"copied\", read_snapshot<{d}>(\"block\"));\n}}\n"
    )
}

/// CT-BND-01 (whole): no program operation sets a field of the block (BND.22). The interpreter reaches its host only
/// through the requests and sinks that `request_fields` and `sink_fields` list with every field, and none carries a
/// block. A program that dresses a word up as a block through a helper, a module call and a function value, sends it
/// to the workspace, a file, the view and the evidence sink and proposes the word, leaves every stored block and the
/// proposal's block exactly as the host built them from the run's context, for each author class and profile; a later
/// run that reads the dressed record cites the host's block, not what the record claims.
#[test]
fn ct_bnd_01_no_program_operation_sets_a_provenance_field() {
    let m = forger();
    let classes = [
        (Author::Own, Profile::ToolAuthoring),
        (Author::ForkedBuiltIn, Profile::Solving),
        (Author::Imported, Profile::Research),
        (Author::BuiltIn, Profile::Solving),
    ];
    for (author, profile) in classes {
        let (mut engine, mut store) = (Engine::default(), Store::default());
        let ran = run(&[DRESS, &m], "attempt", (author, profile), word(), vec![manual()], &mut engine, &mut store);
        assert_eq!(ran.run.summary.outcome, Outcome::Completed);
        let host = provenance::program(&producer(&m, "attempt", author, profile), false, &[sha256(&manual())]);
        for place in [ws("block"), ws("word"), file("block.6cw")] {
            assert_eq!(store.records[&place].block, provenance::canonical(&host), "{place:?}");
        }
        let proposed: Vec<[u8; 32]> =
            ran.shell.iter().map(|s| s.block).chain(ran.captures.iter().map(|c| c.block)).collect();
        assert_eq!(proposed, [sha256(&provenance::canonical(&host))], "the proposal's block");
        let dressed = &store.records[&ws("block")].bytes;
        assert!(String::from_utf8_lossy(dressed).contains("built_in"), "the record says what the program wrote");
        assert!(engine.log.iter().any(|l| l.starts_with("sink view")), "{:?}", engine.log);
        assert!(engine.log.iter().any(|l| l.starts_with("sink exact")), "{:?}", engine.log);

        run(&[DRESS, &m], "copy", (author, profile), vec![], vec![], &mut engine, &mut store);
        let copied = block_at(&store, &ws("copied"));
        assert_eq!(field::parts(&copied), [sha256(&provenance::canonical(&host))]);
        assert_eq!((field::author(&copied), field::producer(&copied)), (author.name(), "program"));
        assert_eq!(field::profile(&copied), profile.name());
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-02: missing and unreadable fields.

/// CT-BND-02 (part: preview admission, the one row built that admits `own`; not run: the session's tables, M6, and the
/// command layer's rows, LP3): each of FP.17's nine fields, missing or unreadable, makes a hand-entered, an imported
/// and a built-in block read `unknown` (BND.21). Preview admission refuses it with
/// `PS1:capability.provenance_incompatible` although the same block passes whole; as a run's input its bytes are kept
/// (PS1.33), it joins as `read_state: yes`, never `no`, and every word that cites it is refused. A readable block whose
/// author is `unknown` is refused too.
#[test]
fn ct_bnd_02_a_missing_or_unreadable_field_reads_unknown_never_own() {
    let empty = Store::default();
    let blocks = [
        ("hand-entered", provenance::manual(&contract())),
        ("imported", provenance::import(&contract(), b"[]")),
        ("built-in", provenance::program(&producer(SAVER, "save", Author::BuiltIn, Profile::Solving), false, &[])),
    ];
    for (what, good) in &blocks {
        assert_eq!(admit_preview(&provenance::canonical(good), &empty, &pins()), Ok(()), "{what} passes whole");
        let Json::Object(map) = good else { panic!("an object") };
        assert_eq!(map.len(), 9);
        for key in map.keys() {
            for damaged in [edit(good, key, None), edit(good, key, Some(Json::Int(7)))] {
                let bytes = provenance::canonical(&damaged);
                assert!(provenance::read(&bytes).is_err(), "{what}: {key}");
                assert_eq!(admit_preview(&bytes, &empty, &pins()).unwrap_err().code, INCOMPATIBLE, "{what}: {key}");

                let (mut engine, mut store) = (Engine::default(), Store::default());
                run(&[SAVER], "save", BUILT_IN, word(), vec![bytes.clone()], &mut engine, &mut store);
                let saved = &store.records[&ws("saved")];
                assert_eq!(provenance::read(&saved.block).unwrap().get("depends"), Some(&yes()), "{what}: {key}");
                assert_eq!(store.blocks.get(&sha256(&bytes)), Some(&bytes), "{what}: {key}");
                assert_eq!(admit_preview(&saved.block, &store, &pins()).unwrap_err().code, INCOMPATIBLE);
            }
        }
    }
    let unknown = provenance::canonical(&edit(&provenance::manual(&contract()), "author", Some(Json::text("unknown"))));
    assert_eq!(field::author(&provenance::read(&unknown).unwrap()), "unknown");
    assert_eq!(admit_preview(&unknown, &empty, &pins()).unwrap_err().code, NOT_GRANTED);
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-04: joins.

/// Chooses between two given words through a state-derived branch, or through a stored flag.
const CHOOSER: &str = "edition \"6cs-draft-1\";\nmodule m requires [file];\n\n\
    export fn by_state(a: Word, b: Word) -> Unit effects [snapshot, workspace] {\n    \
    let w = match exact_less(read_snapshot<Nat>(\"n_c0\"), 3) {\n        true => a;\n        _ => b;\n    };\n    \
    return write_workspace<Word>(\"chosen\", w);\n}\n\n\
    export fn by_file(a: Word, b: Word) -> Unit effects [file, workspace] {\n    \
    let w = match exact_less(file_read<Nat>(\"flag\"), 1) {\n        true => a;\n        _ => b;\n    };\n    \
    return write_workspace<Word>(\"chosen\", w);\n}\n";

/// CT-BND-04 (whole): a word built from parts carries their join under capability separation (BND.24). Two given words
/// whose blocks record `read_state: no` are chosen through a branch on the state: either outcome stores a word whose
/// block cites both parts and records `yes`, and the two blocks are the same bytes. Chosen through a stored flag, the
/// word records what the flag's block records, `no` or `yes`, whatever the letters; parts are cited once each, in
/// first-read order.
#[test]
fn ct_bnd_04_a_word_built_from_parts_carries_their_join() {
    let (a, b) = (manual(), output_of(Author::BuiltIn, Profile::Solving, false));
    let args = || vec![Value::word(&[1]), Value::word(&[2])];
    let mut blocks = Vec::new();
    for (n, chosen) in [(1, 1), (5, 2)] {
        let (mut engine, mut store) = (Engine::default(), Store::default());
        engine.snapshot.insert("n_c0".into(), Value::int(n));
        run(&[CHOOSER], "by_state", BUILT_IN, args(), vec![a.clone(), b.clone()], &mut engine, &mut store);
        let record = &store.records[&ws("chosen")];
        assert!(equal(&record.value, &Value::word(&[chosen])));
        let block = provenance::read(&record.block).unwrap();
        assert_eq!(block.get("depends"), Some(&yes()), "a labelled branch keeps the label");
        assert_eq!(field::parts(&block), [sha256(&a), sha256(&b)]);
        blocks.push(record.block.clone());
    }
    assert_eq!(blocks[0], blocks[1], "the record does not depend on the branch taken");

    let flag_of = |read_state| {
        let maker = producer(CHOOSER, "by_file", Author::BuiltIn, Profile::Solving);
        provenance::canonical(&provenance::program(&maker, read_state, &[]))
    };
    for (flag_read_state, depends) in [(false, no()), (true, yes())] {
        let flag = flag_of(flag_read_state);
        for (value, chosen) in [(0, 1), (1, 2)] {
            let (mut engine, mut store) = (Engine::default(), Store::default());
            store.records.insert(file("flag"), stored(Value::int(value), &flag));
            run(&[CHOOSER], "by_file", BUILT_IN, args(), vec![a.clone(), b.clone()], &mut engine, &mut store);
            let record = &store.records[&ws("chosen")];
            assert!(equal(&record.value, &Value::word(&[chosen])));
            let block = provenance::read(&record.block).unwrap();
            assert_eq!(block.get("depends"), Some(&depends), "flag {flag_read_state}, value {value}");
            assert_eq!(field::parts(&block), [sha256(&a), sha256(&b), sha256(&flag)]);
        }
    }
    let flag = flag_of(false);
    let (mut engine, mut store) = (Engine::default(), Store::default());
    store.records.insert(file("flag"), stored(Value::int(0), &flag));
    run(&[CHOOSER], "by_file", BUILT_IN, args(), vec![a.clone(), a.clone()], &mut engine, &mut store);
    assert_eq!(field::parts(&block_at(&store, &ws("chosen"))), [sha256(&a), sha256(&flag)]);
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-05: DET.50's first six families (LX-3 corpus `must-refuse.json`, MR01 to MR12).

/// The families written in this draft's syntax, each a solver's own tool with entry `m.go`.
fn families() -> Vec<(&'static str, Vec<String>)> {
    let head = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n";
    let one = |body: &str| vec![format!("{head}{body}")];
    let selector = "edition \"6cs-draft-1\";\nmodule selector requires [];\n\n\
        export fn pick() -> Word effects [snapshot] {\n    \
        return match exact_eq(read_snapshot<Nat>(\"nu2_0\"), 1) {\n        true => word[+1];\n        \
        _ => word[+2];\n    };\n}\n";
    vec![
        (
            "largest_n_c",
            one("export fn go(w: Word) -> Text effects [snapshot, preview] {\n    \
                 let t = match exact_less(read_snapshot<Nat>(\"n_c0\"), read_snapshot<Nat>(\"n_c1\")) {\n        \
                 true => target(\"c1\");\n        _ => target(\"c0\");\n    };\n    \
                 return receipt_id(propose(w, t));\n}\n"),
        ),
        (
            "search_rebuild",
            one("export fn go(t: Target) -> Text effects [snapshot, preview] {\n    \
                 let best = match exact_less(\
                 read_snapshot<Nat>(\"score_a\"), read_snapshot<Nat>(\"score_b\")) {\n        \
                 true => word[+2];\n        _ => word[+1];\n    };\n    \
                 return receipt_id(propose(best, t));\n}\n"),
        ),
        (
            "f2_f5_solve",
            one("export fn go(t: Target) -> Text effects [snapshot, preview] {\n    \
                 let correction = match exact_eq(read_snapshot<Nat>(\"nu2_0\"), 1) {\n        \
                 true => word[+1, +2];\n        _ => word[+2];\n    };\n    \
                 return receipt_id(propose(correction, t));\n}\n"),
        ),
        (
            "workspace_laundering",
            one("export fn go(t: Target) -> Text effects [snapshot, preview] {\n    \
                 return receipt_id(propose(read_snapshot<Word>(\"saved\"), t));\n}\n"),
        ),
        (
            "state_choice",
            one("export fn go(t: Target) -> Text effects [snapshot, preview] {\n    \
                 let r = match exact_less(0, read_snapshot<Nat>(\"n_c0\")) {\n        \
                 true => propose(word[+1], t);\n        _ => propose(word[+2], t);\n    };\n    \
                 return receipt_id(r);\n}\n"),
        ),
        (
            "module_flow",
            vec![
                selector.to_string(),
                format!(
                    "edition \"6cs-draft-1\";\nmodule m requires [];\nimport s \"{}\";\n\n\
                     export fn go(t: Target) -> Text effects [snapshot, preview] {{\n    \
                     return receipt_id(propose(s.pick(), t));\n}}\n",
                    sha256_hex(selector.as_bytes())
                ),
            ],
        ),
    ]
}

/// CT-BND-05 (part: the six families as a solver's own tools, and the cross-run routes of MR04, MR07 and MR08 through
/// the runtime; not run: the corpus's built-in `analysis` runs, since no built-in tool class is reviewed and the
/// built-in column stays open (BND.37, CT-BND-15, `rust/README.md` M1), and the corpus's session fields (labels,
/// revision, journal, protected slots, preferences, recovery checkpoint), which the session store holds (M6)) and
/// CT-LG3-07 (part: state-reading preview programs, laundering through a second module (`module_flow`) and through a
/// saved word (MR07); not run: a built-in state-reading previewer, for the same open column, and real declassifiers,
/// which wait for LQ2 (BND.54); `c600-check`'s `ct_lg3_07_*` tests run the mechanism with a stand-in). Each family
/// reads the state and prepares a preview, so capability separation sees both, and a solving session refuses it at
/// admission with `LG3:capability.not_granted`: it never runs, so it reads nothing. Through the runtime, a word a
/// tool-authoring or research run laundered from a planted state read, and a stored word whose block is unreadable,
/// are refused at preview admission with the corpus's codes; the engine builds no preview, the stored records stay as
/// they were, and the program gets the fixed acknowledgement and no planted data.
#[test]
fn ct_bnd_05_det50_families_are_refused_with_the_expected_codes() {
    for (family, sources) in families() {
        let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
        let (codes, seen) = own_tool_in_solving(&sources, "go");
        assert_eq!(codes, [NOT_GRANTED], "{family}");
        assert_eq!(seen, Separation { reads_state: true, prepares_previews: true }, "{family}");
    }

    let planted = [5, -6];
    let routes = [
        ("MR04 research rebuild", (Author::BuiltIn, Profile::Research), None, NOT_GRANTED),
        ("MR07 laundering", (Author::Own, Profile::ToolAuthoring), None, NOT_GRANTED),
        (
            "MR08 unknown provenance",
            (Author::BuiltIn, Profile::ToolAuthoring),
            Some(b"{\"author\":".to_vec()),
            INCOMPATIBLE,
        ),
    ];
    for (case, writer, unreadable, code) in routes {
        for place in [ws("saved"), file("saved.6cw")] {
            let (mut engine, mut store) = (Engine::default(), Store::default());
            engine.snapshot.insert("best_word".into(), Value::word(&planted));
            match &unreadable {
                None => run(&[SAVER], "launder", writer, vec![], vec![], &mut engine, &mut store),
                Some(bytes) => run(
                    &[SAVER],
                    "save",
                    writer,
                    vec![Value::word(&planted)],
                    vec![bytes.clone()],
                    &mut engine,
                    &mut store,
                ),
            };
            let before = contents(&store);
            let mut previewer = Engine::default();
            let entry = if matches!(place, Place::Workspace(_)) { "go_saved" } else { "go_file" };
            let ran = run(&[PREVIEWER], entry, BUILT_IN, vec![], vec![], &mut previewer, &mut store);
            assert_eq!(answer(&ran), Err(code), "{case} via {place:?}");
            assert!(previewer.previews.is_empty() && previewer.log.is_empty(), "{case}: {:?}", previewer.log);
            assert_eq!(ran.run.output.as_deref(), Some(ACK), "{case}: no planted data returned");
            assert_eq!(contents(&store), before, "{case}: the stored records are unchanged");
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-06, CT-BND-09, CT-BND-10: the stop, accumulation and budget channels.

const LETTERS: &str = "fn letter(i: Nat) -> Word effects [] {\n    \
    return if(exact_eq(i, 0), word[+1], if(exact_eq(i, 1), word[+2], word[+3]));\n}\n\n";

/// MR13 and MR14 in this draft's syntax: a state-dependent stop selects the pending preview, or decides whether a
/// constant preview is emitted at all.
fn stop_selected() -> String {
    format!(
        "edition \"6cs-draft-1\";\nmodule m requires [];\n\n{LETTERS}\
         fn scan(i: Nat) -> Unit effects [snapshot, preview] {{\n    \
         let r = propose(letter(i), target(\"t\"));\n    \
         return if(exact_less(i, read_snapshot<Nat>(\"stop\")), scan(exact_add(i, 1)), ());\n}}\n\n\
         export fn go() -> Unit effects [snapshot, preview] {{\n    return scan(0);\n}}\n\n\
         fn emit() -> Unit effects [preview] {{\n    let r = propose(word[+1], target(\"t\"));\n    return ();\n}}\n\n\
         export fn early() -> Unit effects [snapshot, preview] {{\n    \
         return if(exact_less(0, read_snapshot<Nat>(\"bit\")), (), emit());\n}}\n\n\
         fn scan_to(i: Nat, n: Nat) -> Unit effects [preview] {{\n    \
         let r = propose(letter(i), target(\"t\"));\n    \
         return if(exact_less(i, n), scan_to(exact_add(i, 1), n), ());\n}}\n\n\
         export fn public(n: Nat) -> Unit effects [preview] {{\n    return scan_to(0, n);\n}}\n"
    )
}

/// Two engine states that differ in every planted value the channel tests read.
fn two_states() -> [Engine; 2] {
    let state = |stop: u32, bit: u32, word: &[i32]| {
        let mut engine = Engine::default();
        engine.snapshot.insert("stop".into(), Value::int(stop));
        engine.snapshot.insert("bit".into(), Value::int(bit));
        engine.snapshot.insert("best_word".into(), Value::word(word));
        engine
    };
    [state(0, 0, &[7]), state(2, 1, &[-8, 9])]
}

/// What a run shows of itself: its summary and output, and the proposals the shell holds.
fn trace(ran: &Ran) -> (Summary, Option<Vec<u8>>, Vec<(String, Vec<i32>, String)>) {
    let shown = ran.shell.iter().map(|s| (s.id.clone(), s.word.clone(), s.target.clone())).collect();
    (ran.run.summary.clone(), ran.run.output.clone(), shown)
}

/// CT-BND-06 (part: capability separation as the fix in force for a solver's own tool, and a previewing program that
/// passes separation; not run: the built-in tool column, since a built-in class review that keeps a state-reading
/// built-in from previewing does not exist yet (BND.37, `rust/README.md` M1), and KN5.11's replacement of the pending
/// preview, which the engine and the session build (M4, M6)): under capability separation a previewing program reads no
/// state (BND.51). MR13's and MR14's stop-selected previews read the state and prepare previews, so separation sees
/// both and the interim table refuses them with `LG3:capability.not_granted`. A previewing program that passes
/// separation stops on its public input: in two different states it makes the same proposals, in the same order, with
/// the same summary, and the engine is never asked for the state.
#[test]
fn ct_bnd_06_a_stop_selected_preview_is_handled_by_capability_separation() {
    let source = stop_selected();
    for entry in ["go", "early"] {
        let (codes, seen) = own_tool_in_solving(&[&source], entry);
        assert_eq!((codes, seen), (vec![NOT_GRANTED], Separation { reads_state: true, prepares_previews: true }));
    }
    let (codes, seen) = own_tool_in_solving(&[&source], "public");
    assert_eq!((codes, seen), (vec![NOT_GRANTED], Separation { reads_state: false, prepares_previews: true }));

    let traces: Vec<_> = two_states()
        .into_iter()
        .map(|mut engine| {
            let ran =
                run(&[&source], "public", BUILT_IN, vec![Value::int(1)], vec![], &mut engine, &mut Store::default());
            assert_eq!(engine.log, ["propose", "propose"], "no state read");
            trace(&ran)
        })
        .collect();
    assert_eq!(traces[0].2.iter().map(|s| s.1.clone()).collect::<Vec<_>>(), [vec![1], vec![2]]);
    assert_eq!(traces[0], traces[1]);
}

/// MR19 in this draft's syntax: a run with a public index previews a constant word; the state-reading variant
/// previews it and stops early when a planted bit is set.
const ACCUMULATOR: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
    export fn indexed(i: Nat) -> Text effects [preview] {\n    \
    return receipt_id(propose(\
    if(exact_eq(i, 0), word[+1], if(exact_eq(i, 1), word[+2], word[+3])), target(\"t\")));\n}\n\n\
    export fn bit(i: Nat) -> Text effects [snapshot, preview] {\n    \
    let r = propose(if(exact_eq(i, 0), word[+1], word[+2]), target(\"t\"));\n    \
    return if(exact_less(0, read_snapshot<Nat>(\"bit\")), receipt_id(r), receipt_base(r));\n}\n";

/// CT-BND-09 (part: a solver's own tool and a previewing program that passes separation, across runs and through the
/// workspace; not run: the built-in tool column (BND.37, `rust/README.md` M1)): accumulation across runs does not arise
/// under capability separation (BND.47). MR19's state-reading variant is refused at admission; three runs with public
/// indexes 0, 1 and 2 of a program that passes separation give the same proposals, summaries and outputs in two
/// different states, so repeated runs add up nothing about the state; and a word a solver's tool stored after reading
/// the state is refused when a later run proposes it.
#[test]
fn ct_bnd_09_accumulation_across_runs_gains_nothing() {
    let (codes, seen) = own_tool_in_solving(&[ACCUMULATOR], "bit");
    assert_eq!((codes, seen), (vec![NOT_GRANTED], Separation { reads_state: true, prepares_previews: true }));
    let (codes, seen) = own_tool_in_solving(&[ACCUMULATOR], "indexed");
    assert_eq!((codes, seen), (vec![NOT_GRANTED], Separation { reads_state: false, prepares_previews: true }));

    let per_state: Vec<Vec<_>> = two_states()
        .into_iter()
        .map(|mut engine| {
            let mut store = Store::default();
            let traces: Vec<_> = (0..3)
                .map(|i| {
                    trace(&run(
                        &[ACCUMULATOR],
                        "indexed",
                        BUILT_IN,
                        vec![Value::int(i)],
                        vec![],
                        &mut engine,
                        &mut store,
                    ))
                })
                .collect();
            assert!(engine.log.iter().all(|l| l == "propose"), "{:?}", engine.log);
            traces
        })
        .collect();
    assert_eq!(per_state[0], per_state[1]);
    let words: Vec<_> = per_state[0].iter().map(|t| t.2[0].1.clone()).collect();
    assert_eq!(words, [vec![1], vec![2], vec![3]]);

    for mut engine in two_states() {
        let mut store = Store::default();
        run(&[SAVER], "launder", (Author::Own, Profile::Solving), vec![], vec![], &mut engine, &mut store);
        assert_eq!(preview_stored(&mut store, Some(ws("saved")), None), Err(NOT_GRANTED));
    }
}

/// Reads the amount used and the budget status after a preview, or after a state read, and keeps them.
const COUNTERS: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
    export fn after_preview(w: Word) -> Text effects [preview, view, budget] {\n    \
    let r = propose(w, target(\"t\"));\n    \
    let v = publish_view(record { used = used_steps(), left = unused_steps(), status = budget_status() });\n    \
    return receipt_id(r);\n}\n\n\
    export fn after_state() -> Unit effects [snapshot, workspace, budget] {\n    \
    let n = read_snapshot<Nat>(\"stop\");\n    \
    return write_workspace<Display<Nat>>(\"used\", used_steps());\n}\n";

/// CT-BND-10 (part: the capability-separation form for programs that pass it; not run: the flow-labels clause, since
/// labels are the unselected form (LQ21), and the built-in tool column (BND.37, `rust/README.md` M1)): a previewing
/// program reads no state, and declared costs depend only on inputs, so its counters do not vary with the state
/// (BND.44): the amount used, the amount left and the status it publishes after a preview are the same bytes in two
/// states and whether the engine built or refused the preview, with the same summary. A run that reads the state and
/// then the amount used keeps it under a block that records `read_state: yes`.
#[test]
fn ct_bnd_10_a_previewing_programs_counters_do_not_vary_with_the_state() {
    let (codes, seen) = own_tool_in_solving(&[COUNTERS], "after_preview");
    assert_eq!((codes, seen), (vec![NOT_GRANTED], Separation { reads_state: false, prepares_previews: true }));
    let mut seen = Vec::new();
    for (mut engine, refuse) in two_states().into_iter().zip([None, Some(STALE)]) {
        engine.refuse = refuse;
        let ran =
            run(&[COUNTERS], "after_preview", BUILT_IN, word(), vec![manual()], &mut engine, &mut Store::default());
        assert_eq!(ran.run.output.as_deref(), Some(ACK));
        let view: Vec<_> = engine.log.iter().filter(|l| l.starts_with("sink view")).cloned().collect();
        assert_eq!(view.len(), 1);
        seen.push((view, ran.run.summary));
    }
    assert_eq!(seen[0], seen[1]);

    let (mut engine, mut store) = (Engine::default(), Store::default());
    engine.snapshot.insert("stop".into(), Value::int(3));
    run(&[COUNTERS], "after_state", BUILT_IN, vec![], vec![], &mut engine, &mut store);
    assert_eq!(block_at(&store, &ws("used")).get("depends"), Some(&yes()));
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-07, CT-BND-11, CT-BND-12: preview outcomes and the fixed acknowledgement.

/// MR15 in this draft's syntax: a probe's refusal selects the next word. Each `outcome_*` entry reads one field.
const OUTCOMES: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
    export fn select(w: Word) -> Text effects [snapshot, preview] {\n    \
    let probe = propose(w, target(\"protected\"));\n    \
    let next = match receipt_error(probe) {\n        tag(\"KN5:stale.revision_changed\") => word[+1];\n        \
    _ => word[+2];\n    };\n    return receipt_id(propose(next, target(\"t\")));\n}\n\n\
    export fn outcome_effect(w: Word) -> Text effects [snapshot, preview] {\n    \
    return receipt_effect(propose(w, target(\"t\")));\n}\n\n\
    export fn outcome_protection(w: Word) -> Status effects [snapshot, preview] {\n    \
    return receipt_protection(propose(w, target(\"t\")));\n}\n\n\
    export fn outcome_end_state(w: Word) -> Text effects [snapshot, preview] {\n    \
    return receipt_end_state(propose(w, target(\"t\")));\n}\n\n\
    export fn outcome_n(w: Word) -> Nat effects [snapshot, preview] {\n    \
    return receipt_n(propose(w, target(\"t\")));\n}\n\n\
    export fn outcome_error(w: Word) -> Status effects [snapshot, preview] {\n    \
    return receipt_error(propose(w, target(\"t\")));\n}\n\n\
    export fn ack(w: Word) -> record { base: Text, id: Text } effects [preview] {\n    \
    let r = propose(w, target(\"t\"));\n    return record { base = receipt_base(r), id = receipt_id(r) };\n}\n";

const OUTCOME_ENTRIES: [&str; 6] =
    ["select", "outcome_effect", "outcome_protection", "outcome_end_state", "outcome_n", "outcome_error"];

/// The engine's possible answers to one proposal: built, refused at admission (the input is a solver's tool word),
/// refused by the engine for a stale revision, refused for another reason.
fn answers() -> [(Vec<u8>, Option<&'static str>); 4] {
    let tool_word = output_of(Author::Own, Profile::ToolAuthoring, true);
    [(manual(), None), (tool_word, None), (manual(), Some(STALE)), (manual(), Some("KN7:budget.kernel_cost"))]
}

/// CT-BND-07 (whole) and CT-LG3-07 (part: preview-outcome reads, every outcome field; the other clauses run in
/// `ct_bnd_05_*` and `c600-check`'s `ct_lg3_07_*` tests): refusal-driven selection under capability separation
/// (BND.43). MR15's program reads its probe's refusal to choose the next word: separation sees a state read, as for
/// every outcome field, and the interim table refuses it for a solver's tool. Run in a solving session, reading the
/// refusal or any outcome field is not granted: the run stops at the read with `LG3:capability.not_granted` and the
/// same summary whether the engine built the preview, admission refused it or the engine refused it with either code.
/// What the program does receive is the fixed acknowledgement, the same `{base, id}` in every case.
#[test]
fn ct_bnd_07_refusal_driven_selection_gets_the_fixed_acknowledgement() {
    for entry in OUTCOME_ENTRIES {
        let (codes, seen) = own_tool_in_solving(&[OUTCOMES], entry);
        assert_eq!((codes, seen), (vec![NOT_GRANTED], Separation { reads_state: true, prepares_previews: true }));
        let mut summaries = Vec::new();
        for (input, refuse) in answers() {
            let mut engine = Engine { refuse, ..Engine::default() };
            let ran = run(&[OUTCOMES], entry, BUILT_IN, word(), vec![input], &mut engine, &mut Store::default());
            assert_eq!((ran.run.summary.outcome, ran.run.summary.code), (Outcome::Error, Some(NOT_GRANTED)), "{entry}");
            assert!(ran.run.summary.stop.as_ref().unwrap().at.ends_with("#read_receipt"), "{entry}");
            assert_eq!(ran.shell.len(), 1, "{entry}: the probe only");
            summaries.push(ran.run.summary);
        }
        assert!(summaries.iter().all(|s| s == &summaries[0]), "{entry}");
    }
    let outputs: Vec<_> = answers()
        .into_iter()
        .map(|(input, refuse)| {
            let mut engine = Engine { refuse, ..Engine::default() };
            run(&[OUTCOMES], "ack", BUILT_IN, word(), vec![input], &mut engine, &mut Store::default()).run.output
        })
        .collect();
    assert_eq!(outputs[0].as_deref(), Some(&b"{\"base\":\"rev-7\",\"id\":\"run/1\"}\n"[..]));
    assert!(outputs.iter().all(|o| o == &outputs[0]));
}

/// CT-BND-11 (part: the solving profile, and the receipt a capture gives in tool authoring and research; not run:
/// ONE.32's public fields in the other profiles, which never arise here, since BND.60 makes every proposal there a
/// capture and no preview exists to report): in a solving session a previewing program gets the preview's ID and base
/// and no outcome field (BND.42); in tool authoring and research it gets the capture's ID and base, and an outcome
/// field is absent, so its read is not granted.
#[test]
fn ct_bnd_11_outcome_fields_by_profile() {
    for profile in [Profile::Solving, Profile::ToolAuthoring, Profile::Research] {
        let class = (Author::BuiltIn, profile);
        let ran = run(&[OUTCOMES], "ack", class, word(), vec![manual()], &mut Engine::default(), &mut Store::default());
        assert_eq!(ran.run.output.as_deref(), Some(&b"{\"base\":\"rev-7\",\"id\":\"run/1\"}\n"[..]), "{profile:?}");
        for entry in &OUTCOME_ENTRIES[1..] {
            let ran =
                run(&[OUTCOMES], entry, class, word(), vec![manual()], &mut Engine::default(), &mut Store::default());
            assert_eq!(ran.run.summary.code, Some(NOT_GRANTED), "{entry} in {profile:?}");
        }
    }
}

/// CT-BND-12 (part: the program's acknowledgement and the record the adapter keeps for the shell; not run: the shell
/// showing it to the solver with reason and remedy, which the command layer and the shell build (LP3)): a refused
/// proposal gives the program the fixed acknowledgement, the same bytes as a built preview (BND.43), while the shell's
/// record keeps the full answer: the refusal's code and reason for a refusal at admission or by the engine, and every
/// field of a built preview.
#[test]
fn ct_bnd_12_a_refused_proposal_gets_the_acknowledgement_and_the_shell_the_full_message() {
    let mut outputs = Vec::new();
    for (i, (input, refuse)) in answers().into_iter().enumerate() {
        let mut engine = Engine { refuse, ..Engine::default() };
        let ran = run(&[PREVIEWER], "go", BUILT_IN, word(), vec![input], &mut engine, &mut Store::default());
        match (i, &ran.shell[0].answer) {
            (0, Answer::Previewed(Value::Receipt(r))) => {
                for name in ["base", "effect", "end_state", "id", "n", "protection"] {
                    assert!(r.get(name).is_some(), "{name}");
                }
            }
            (1, Answer::Refused(r)) => {
                assert_eq!(r.code, NOT_GRANTED);
                assert!(r.detail.contains("solver-authored") && r.detail.contains("BND.84"), "{}", r.detail);
            }
            (2 | 3, Answer::Refused(r)) => assert_eq!(Some(r.code), refuse),
            (_, other) => panic!("{i}: {other:?}"),
        }
        outputs.push(ran.run.output);
    }
    assert!(outputs.iter().all(|o| o.as_deref() == Some(ACK)));
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-14: captures.

/// CT-BND-14 (part: the engine and the program; not run: the session's digest, which the session store keeps (M6)): in
/// tool authoring and research `propose` goes to a capture (BND.60). The engine is never asked to build a preview and
/// receives no request at all, the program's receipt holds only the capture's ID and base, no token, and the capture
/// keeps the word, the target and its block for a test to read.
#[test]
fn ct_bnd_14_a_capture_creates_no_preview_and_no_token() {
    for profile in [Profile::ToolAuthoring, Profile::Research] {
        for author in [Author::BuiltIn, Author::Own] {
            let (mut engine, mut store) = (Engine::default(), Store::default());
            let ran = run(&[OUTCOMES], "ack", (author, profile), word(), vec![manual()], &mut engine, &mut store);
            assert_eq!(ran.run.output.as_deref(), Some(&b"{\"base\":\"rev-7\",\"id\":\"run/1\"}\n"[..]));
            assert!(engine.log.is_empty() && engine.previews.is_empty() && ran.shell.is_empty(), "{:?}", engine.log);
            let capture = &ran.captures[0];
            assert_eq!((capture.word.as_slice(), capture.target.as_str()), (&[1, -2][..], "t"));
            let block = provenance::read(&store.blocks[&capture.block]).unwrap();
            assert_eq!((field::author(&block), field::profile(&block)), (author.name(), profile.name()));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-08 and CT-BND-26 to CT-BND-30: picks, drafts, files and restored workspaces at preview admission.

/// A pick record that names a `c600-pick` record the store does not hold (FP.38, vector `pick-no-record`).
fn with_pick_record(word: &[u8]) -> Vec<u8> {
    let digest = provenance::digest_ref("content", &[7; 32]);
    let reference =
        Json::object([("format", Json::text("c600-pick")), ("version", Json::Int(1)), ("content", digest.clone())]);
    let block = provenance::read(word).unwrap();
    provenance::canonical(&edit(&block, "pick", Some(Json::object([("record", reference), ("chosen", digest)]))))
}

/// CT-BND-08 (part: a picked word's block keeps the listed entry's block in `parts` and preview admission follows it;
/// not run: the pick record, which resolves only when the session store holds `c600-pick` records (FP.38, M6), and
/// BND.55's label rule, which is open until the joint setting (LQ2)): a word picked from a one-item list a solver's
/// tool built keeps that entry's block, so admission refuses it with `LG3:capability.not_granted`, directly and
/// through a later run; a word picked from a fixed entry of a built-in list passes. A pick that names an unresolved
/// record is refused with `PS1:capability.provenance_incompatible`.
#[test]
fn ct_bnd_08_a_one_item_pick_keeps_the_entry_block() {
    let one_item = output_of(Author::Own, Profile::ToolAuthoring, true);
    let fixed = output_of(Author::BuiltIn, Profile::Solving, false);
    for (entry, expected) in [(one_item, Err(NOT_GRANTED)), (fixed, Ok(()))] {
        let word = picked(&[&entry]);
        let mut store = Store::default();
        store.keep(&entry);
        assert_eq!(admit_preview(&word, &store, &pins()).map_err(|r| r.code), expected);
        assert_eq!(preview_stored(&mut store, None, Some(&word)), expected);
        assert_eq!(admit_preview(&with_pick_record(&word), &store, &pins()).unwrap_err().code, INCOMPATIBLE);
    }
}

/// Saves a word through a built-in solving run to the workspace and a file, then copies the store as a restore would
/// find it: every record with its value, bytes and block, and every kept block, by content digest.
fn saved_and_restored(input: &[u8]) -> Store {
    let (mut engine, mut store) = (Engine::default(), Store::default());
    run(&[SAVER], "save", BUILT_IN, word(), vec![input.to_vec()], &mut engine, &mut store);
    let mut restored = Store::default();
    for bytes in store.blocks.values() {
        restored.keep(bytes);
    }
    for (place, record) in &store.records {
        restored.records.insert(place.clone(), record.clone());
    }
    restored
}

/// CT-BND-26 (part: composition and a file round trip through runs, with the entry block in `parts`; not run: draft
/// insertion, which the command layer and the session build (LP3, M6), and the pick record (FP.38, M6)): a word picked
/// from the list of a tool of each solver-authored class, `own`, `forked_built_in`, `imported` and `unknown`, keeps the
/// listed entry's block when a later run takes it in and stores it, and through a file; the word's letters are kept,
/// and every later preview in a solving session refuses it with `LG3:capability.not_granted`, the tool block nested
/// two levels down in `parts`.
#[test]
fn ct_bnd_26_a_picked_tool_word_keeps_the_entry_block_and_is_refused() {
    for author in [Author::Own, Author::ForkedBuiltIn, Author::Imported, Author::Unknown] {
        let entry = output_of(author, Profile::ToolAuthoring, true);
        let word = picked(&[&entry]);
        let mut store = saved_and_restored(&word);
        store.keep(&entry);
        let saved = block_at(&store, &file("saved.6cw"));
        assert_eq!(field::parts(&saved), [sha256(&word)]);
        assert_eq!(field::parts(&provenance::read(&store.blocks[&sha256(&word)]).unwrap()), [sha256(&entry)]);
        assert!(equal(&store.records[&file("saved.6cw")].value, &Value::word(&[1, -2])), "the word is kept");
        for place in [ws("saved"), file("saved.6cw")] {
            assert_eq!(preview_stored(&mut store, Some(place.clone()), None), Err(NOT_GRANTED), "{author:?} {place:?}");
        }
        assert_eq!(preview_stored(&mut store, None, Some(&word)), Err(NOT_GRANTED), "{author:?} directly");
    }
}

/// CT-BND-27 (part: the record a restore must preserve; not run: persisting and restoring a workspace, which the
/// session store builds (M6)): a tool word's workspace record, copied as a restore finds it, keeps its block and its
/// recursive parts by content digest, and a solving preview of the restored word is refused at admission with
/// `LG3:capability.not_granted` and installs no preview.
#[test]
fn ct_bnd_27_a_restored_tool_word_is_refused_at_admission() {
    let (mut engine, mut store) = (Engine::default(), Store::default());
    engine.snapshot.insert("best_word".into(), Value::word(&[4]));
    run(&[SAVER], "launder", (Author::Own, Profile::Solving), vec![], vec![], &mut engine, &mut store);
    let tool_block = store.records[&ws("saved")].block.clone();
    let mut restored = saved_and_restored(&tool_block);
    restored.keep(&tool_block);
    assert_eq!(field::parts(&block_at(&restored, &ws("saved"))), [sha256(&tool_block)]);
    assert_eq!(restored.blocks.get(&sha256(&tool_block)), Some(&tool_block));
    assert_eq!(preview_stored(&mut restored, Some(ws("saved")), None), Err(NOT_GRANTED));
}

/// CT-BND-28 (part: missing content of a part and of a nested part; not run: following a pick's origin through its
/// record, which resolves in the session store (FP.38, M6)): a picked word whose listed entry has no stored content, or
/// whose entry is a built-in block citing a part with no stored content, is refused at preview admission with
/// `LG3:capability.not_granted`: the absence reads `unknown`, never as the word's own `manual` block or the entry's
/// built-in producer.
#[test]
fn ct_bnd_28_missing_origin_content_fails_closed() {
    let missing = picked(&[&b"never stored"[..]]);
    let inner = provenance::program(&producer(SAVER, "save", Author::BuiltIn, Profile::Solving), false, &[[9; 32]]);
    let inner = provenance::canonical(&inner);
    let nested = picked(&[&inner]);
    for (what, word, extra) in [("missing entry", missing, None), ("missing nested part", nested, Some(inner))] {
        let mut store = Store::default();
        if let Some(extra) = &extra {
            store.keep(extra);
        }
        let refusal = admit_preview(&word, &store, &pins()).unwrap_err();
        assert_eq!(refusal.code, NOT_GRANTED, "{what}");
        assert!(refusal.detail.contains("no stored content"), "{what}: {}", refusal.detail);
        assert_eq!(preview_stored(&mut store, None, Some(&word)), Err(NOT_GRANTED), "{what}");
    }
}

/// CT-BND-29 (part: hand-entered words, imported macros and a built-in-list entry kept in a picked word's parts, with
/// their ordinary pin gates, and retyping; not run: the pick record (FP.38, M6) and retyping through the command layer
/// (LP3)): a hand-entered word, an imported 0.4 macro and a word picked from a built-in list pass BND.84's producer
/// check and build a preview, unless an ordinary gate fails: a contract pin the policy does not hold refuses with
/// `PS1:capability.provenance_incompatible`. A word retyped from a tool's saved word is a new hand-entered word with
/// no parts: the host infers no tool origin for it.
#[test]
fn ct_bnd_29_manual_imported_and_built_in_list_words_pass_the_producer_check() {
    let built_in_entry = output_of(Author::BuiltIn, Profile::Solving, false);
    for (what, word) in
        [("hand-entered", manual()), ("imported", imported()), ("built-in pick", picked(&[&built_in_entry]))]
    {
        let mut store = Store::default();
        store.keep(&built_in_entry);
        assert_eq!(preview_stored(&mut store, None, Some(&word)), Ok(()), "{what}");
        let other = Pins { contracts: vec![Pin::of("fixture-2", b"another contract\n")], checkers: vec![checker()] };
        assert_eq!(admit_preview(&word, &store, &other).unwrap_err().code, INCOMPATIBLE, "{what}");
    }
    let (mut engine, mut store) = (Engine::default(), Store::default());
    engine.snapshot.insert("best_word".into(), Value::word(&[5, -6]));
    run(&[SAVER], "launder", (Author::Own, Profile::Solving), vec![], vec![], &mut engine, &mut store);
    let retyped = manual();
    assert!(field::parts(&provenance::read(&retyped).unwrap()).is_empty());
    let mut engine = Engine::default();
    let ran = run(&[PREVIEWER], "go", BUILT_IN, vec![Value::word(&[5, -6])], vec![retyped], &mut engine, &mut store);
    assert_eq!((answer(&ran), engine.previews), (Ok(()), vec![vec![5, -6]]));
}

/// CT-BND-30 (part: research blocks reached through a picked word's parts, recursive parts, a file and a restored
/// workspace record; not run: the shipped research library and its output, which are not built yet, drafts (LP3, M6)
/// and the pick record (FP.38, M6)): a research-profile block refuses a solving preview at the same admission point
/// with `LG3:capability.not_granted` (BND.61, BND.85): directly, as a picked word's listed entry, two levels down in
/// `parts`, and through a file and a restored workspace record; no route grants the preview.
#[test]
fn ct_bnd_30_research_blocks_refuse_the_preview_on_every_route() {
    let research = output_of(Author::BuiltIn, Profile::Research, false);
    let through_built_in = provenance::canonical(&provenance::program(
        &producer(SAVER, "save", Author::BuiltIn, Profile::Solving),
        false,
        &[sha256(&research)],
    ));
    let routes =
        [("directly", research.clone()), ("picked", picked(&[&research])), ("nested", picked(&[&through_built_in]))];
    for (what, word) in routes {
        let mut store = Store::default();
        store.keep(&research);
        store.keep(&through_built_in);
        let refusal = admit_preview(&word, &store, &pins()).unwrap_err();
        assert_eq!(refusal.code, NOT_GRANTED, "{what}");
        assert!(refusal.detail.contains("research output"), "{what}: {}", refusal.detail);
        assert_eq!(preview_stored(&mut store, None, Some(&word)), Err(NOT_GRANTED), "{what}");
        let mut restored = saved_and_restored(&word);
        restored.keep(&research);
        restored.keep(&through_built_in);
        for place in [ws("saved"), file("saved.6cw")] {
            assert_eq!(preview_stored(&mut restored, Some(place.clone()), None), Err(NOT_GRANTED), "{what} {place:?}");
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-BND-22: forms.

/// CT-BND-22 (part: an older block keeps its form, and a gated use it cannot justify is refused; not run: the recorded
/// migration that switches forms, which no milestone of LP2 builds (the session store and the importer, M6)): FP.18's
/// flow-form examples, taken in by a run, are kept byte for byte and still read in their form; a word that cites one is
/// refused at preview admission with `PS1:capability.provenance_incompatible`, even under a policy that holds the
/// flow-form contract pin.
#[test]
fn ct_bnd_22_an_older_block_keeps_its_form_and_its_gated_use_is_refused() {
    let examples = provenance::parse(&std::fs::read(format!("{PACKET}examples.json")).unwrap()).expect("examples");
    let records = examples.get("records").unwrap();
    let fixtures = examples.get("fixture_contracts").unwrap();
    let flow_pin = Pin::of("fixture-1", fixtures.get("flow_static").and_then(Json::as_text).unwrap().as_bytes());
    for name in ["manual_flow", "imported_flow"] {
        let older = provenance::canonical(records.get(name).and_then(|r| r.get("provenance")).unwrap());
        assert_eq!(field::form(&provenance::read(&older).unwrap()), "flow_static");
        let (mut engine, mut store) = (Engine::default(), Store::default());
        run(&[SAVER], "save", BUILT_IN, word(), vec![older.clone()], &mut engine, &mut store);
        assert_eq!(store.blocks.get(&sha256(&older)), Some(&older), "{name} is kept as it was");
        let saved = block_at(&store, &ws("saved"));
        assert_eq!((field::form(&saved), field::parts(&saved)), ("capability_separation", vec![sha256(&older)]));
        let held = Pins { contracts: vec![contract(), flow_pin.clone()], checkers: vec![checker()] };
        for policy in [pins(), held] {
            let refusal = admit_preview(&store.records[&ws("saved")].block, &store, &policy).unwrap_err();
            assert_eq!(refusal.code, INCOMPATIBLE, "{name}");
            assert!(refusal.detail.contains("another form"), "{name}: {}", refusal.detail);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// CT-KN5-13: the preview-admission adapter.

/// CT-KN5-13 (part: the BND.84/BND.85 adapter of M3; not run: picks through their records (FP.38, M6), draft insertion
/// (LP3, M6) and KN5.9's preview itself, which the kernel builds (M4)): in a solving session the adapter refuses a word
/// a solver's tool produced, through composition in a later run, a file round trip and a restored workspace record,
/// and fails closed on missing origin content, all with `LG3:capability.not_granted`. Hand-entered words, imported 0.4
/// macros and a built-in-list entry pass. A permitted word whose preview has a protection conflict is not refused by
/// the adapter: the engine builds the preview and the shell holds its verdict.
#[test]
fn ct_kn5_13_the_adapter_refuses_tool_words_and_passes_the_rest() {
    let tool = output_of(Author::Own, Profile::ToolAuthoring, true);
    let mut restored = saved_and_restored(&tool);
    for place in [ws("saved"), file("saved.6cw")] {
        assert_eq!(preview_stored(&mut restored, Some(place), None), Err(NOT_GRANTED));
    }
    assert_eq!(preview_stored(&mut Store::default(), None, Some(&tool)), Err(NOT_GRANTED), "composition");
    assert_eq!(preview_stored(&mut Store::default(), None, Some(&picked(&[b"gone"]))), Err(NOT_GRANTED));
    let entry = output_of(Author::BuiltIn, Profile::Solving, false);
    for word in [manual(), imported(), picked(&[&entry])] {
        let mut store = Store::default();
        store.keep(&entry);
        assert_eq!(preview_stored(&mut store, None, Some(&word)), Ok(()));
    }
    let mut engine = Engine { protection: Some("conflict"), ..Engine::default() };
    let ran = run(&[PREVIEWER], "go", BUILT_IN, word(), vec![manual()], &mut engine, &mut Store::default());
    assert_eq!(engine.previews, [vec![1, -2]]);
    let Answer::Previewed(Value::Receipt(receipt)) = &ran.shell[0].answer else { panic!("a preview") };
    assert!(equal(receipt.get("protection").unwrap(), &Value::tag("conflict")));
    assert_eq!(ran.run.output.as_deref(), Some(ACK));
}
