//! The draft `journal` row, KN4.3's page of the move tree, served to an entry by the host bridge (KN5; LP3's C8; a
//! draft until the command table freezes at LP3). An entry's snapshot read with an argument,
//! `read_snapshot<T>("journal", page)`, runs on `c600-run` through `Boundary` and [`Bridge`] onto a command layer, and
//! the bridge answers it from LP2's `CommandLayer::journal` (`c600_tools::kernel_rows`). The entry kinds of Jumble on
//! are not run here: a layer in Jumble on needs a pinned twist menu, which `c600-jumble` builds, and this crate's tests
//! do not depend on it; `c600-command`'s `tests/journal.rs` runs them on the layer. Covers no conformance ID.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::TYPE_MISMATCH;
use c600_command::{
    code, draft_args, CommandLayer, HeadlessFrames, JournalEntry, JournalPage, Secret, Shell, Status, Table,
    JOURNAL_PAGE_LIMIT,
};
use c600_digest::sha256;
use c600_kernel::json::{self, Value as KernelJson};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{journal_tariff, journal_type, JOURNAL};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const MODULE: &str = "journals";

/// The page argument as LP3's C8 passes it, and with other types for `limit` and `from`.
const PAGE: &str = "record { cursor: Text, from: Text, limit: Nat }";
const PAGE_INT: &str = "record { cursor: Text, from: Text, limit: Int }";
const PAGE_FROM_INT: &str = "record { cursor: Text, from: Int, limit: Nat }";

/// A journal entry as C8 declares it.
const ENTRY: &str = "record { event: Text, exit: Text, kind: Text, letters: Nat, solved: Bool }";

/// A stand-in for an entry over the row, with variants that declare another order and `Int` for `letters`, declare
/// types the row does not answer, pass another argument or read a row the bridge does not serve.
fn source() -> String {
    let entry = |name: &str, param: &str, answer: &str, query: &str| {
        format!(
            "export fn {name}(asked: {param}) -> {answer} effects [snapshot] {{\n    \
             return read_snapshot<{answer}>(\"{query}\", asked);\n}}\n\n"
        )
    };
    let answer = format!("record {{ entries: List<{ENTRY}>, next: Text }}");
    let reordered = "record { next: Text, entries: List<record { solved: Bool, letters: Int, kind: Text, exit: Text, \
                     event: Text }> }";
    let texts = "record { entries: List<record { event: Text, exit: Text, kind: Text, letters: Text, solved: Bool }>, \
                 next: Text }";
    let short = "record { entries: List<record { event: Text, exit: Text, kind: Text, letters: Nat }>, next: Text }";
    let flat = format!("record {{ entries: {ENTRY}, next: Text }}");
    [
        "edition \"6cs-draft-1\";\nmodule journals requires [];\n\n".to_owned(),
        entry("page", PAGE, &answer, JOURNAL),
        entry("page_any", PAGE, reordered, JOURNAL),
        entry("page_texts", PAGE, texts, JOURNAL),
        entry("page_short", PAGE, short, JOURNAL),
        entry("page_flat", PAGE, &flat, JOURNAL),
        entry("page_int", PAGE_INT, &answer, JOURNAL),
        entry("page_from_int", PAGE_FROM_INT, &answer, JOURNAL),
        entry("page_one", "Int", &answer, JOURNAL),
        entry("colours", PAGE, &answer, "colours"),
        entry("colours_int", PAGE_INT, &answer, "colours"),
        entry("colours_from_int", PAGE_FROM_INT, &answer, "colours"),
        entry("colours_one", "Int", &answer, "colours"),
    ]
    .concat()
    .trim_end()
    .to_owned()
        + "\n"
}

const LIMITS: Amounts = Amounts { steps: 1 << 20, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };

const SECRET: [u8; 32] = [5; 32];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    let kernel = KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => Arc::new(kernel),
        Err(error) => panic!("the model in assets/ loads: {error}"),
    });
    kernel.clone()
}

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| load(&lower_source(&source()).expect("the stand-in lowers")).expect("the stand-in loads"))
}

/// A layer whose session starts at `start`, with no protected orbit.
fn layer_at(start: Snapshot) -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed(SECRET), Default::default(), MODULE, start, none)
}

/// A layer at solved with a commit of each of `words`.
fn layer_with(words: &[&[i64]]) -> CommandLayer {
    let mut layer = layer_at(kernel().solved());
    let (mut shell, mut frames) = (Shell::new(Secret::fixed(SECRET), "shell"), HeadlessFrames::new());
    for letters in words {
        let recipe = json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap();
        let receipt = shell.invoke(&mut layer, "operation.preview", draft_args(&recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(layer.run_jobs(), 1);
        frames.present(&mut shell, &mut layer, id).unwrap();
        let args = json::object([("preview", KernelJson::Int(id as i64))]);
        let press = shell.prepare(&layer, "operation.execute", args).unwrap();
        assert!(matches!(shell.send(&mut layer, press).status, Status::Committed(_)));
    }
    layer
}

/// The context of one run of `entry` in the solving profile, built in, with a state read (LG3.13).
fn context(layer: &CommandLayer, entry: &str) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(source().as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    Context { run: MODULE.into(), base, producer, reads_state: true, inputs: vec![named_word_block()], pins: pins() }
}

/// Runs `entry` on `arg` through a boundary and a bridge onto `layer`; a read sends no invocation and changes nothing.
fn run(layer: &mut CommandLayer, entry: &str, arg: Value) -> Run {
    let before = (layer.revision().clone(), layer.store().log.len());
    let context = context(layer, entry);
    let mut bridge = Bridge::new(layer, MODULE);
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(MODULE, entry, vec![arg], &mut boundary, &Options::new(LIMITS)).expect("the run starts");
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before, "a read changes nothing");
    run
}

fn answer(run: &Run) -> &Value {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
    run.value.as_ref().expect("an answer")
}

/// The kernel cost of a read of `entry` on `arg` that stops with `expected`.
fn refused(layer: &mut CommandLayer, entry: &str, arg: &Value, expected: &str) -> u128 {
    let read = run(layer, entry, arg.clone());
    let summary = &read.summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Error, Some(expected)), "{entry}: {:?}", summary.stop);
    summary.used.kernel_cost
}

/// A page argument.
fn page(cursor: &str, from: &str, limit: i64) -> Value {
    let fields = vec![("cursor", Value::text(cursor)), ("from", Value::text(from)), ("limit", Value::int(limit))];
    Value::record(fields)
}

/// The answer a read declared as C8 declares it gives for a page of the layer.
fn expected(page: &JournalPage) -> Value {
    let entry = |entry: &JournalEntry| {
        Value::record(vec![
            ("event", Value::text(&entry.event())),
            ("exit", Value::text(entry.exit.as_deref().unwrap_or(""))),
            ("kind", Value::text(entry.kind.name())),
            ("letters", Value::int(entry.letters)),
            ("solved", Value::Bool(entry.solved)),
        ])
    };
    let entries = Value::list(page.entries.iter().map(entry).collect());
    Value::record(vec![("entries", entries), ("next", Value::text(page.next.as_deref().unwrap_or("")))])
}

fn names(value: &Value) -> Vec<String> {
    let Value::Record(fields) = value else { panic!("a record: {value:?}") };
    fields.fields().iter().map(|(name, _)| name.as_str().to_owned()).collect()
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.get(name).unwrap_or_else(|| panic!("a field {name}: {value:?}"))
}

fn text(value: &Value) -> &str {
    let Value::Text(text) = value else { panic!("a text: {value:?}") };
    text.as_str()
}

/// An entry reads pages of the journal as `CommandLayer::journal` gives them, from the head or a named event, and
/// pages on by passing each answer's `next` back as its `cursor` until it is "". The answer's fields, and each entry's,
/// follow the declared order, and `letters` may be declared `Int`. A start that is not solved says so. A read changes
/// nothing and sends no invocation.
#[test]
fn an_entry_reads_the_journal_as_the_layer_pages_it() {
    let mut layer = layer_with(&[&[2, -1], &[5], &[17, -305, 911]]);
    for (from, limit) in [("", 10), ("", 2), ("e2", 1), ("e2", 4), ("e0", 1)] {
        let order = from.strip_prefix('e').map(|order| order.parse().unwrap());
        let page = layer.journal(order, limit as u64, None).unwrap();
        let read = run(&mut layer, "page", self::page("", from, limit));
        assert!(equal(answer(&read), &expected(&page)), "{from:?}, {limit}");
    }
    let whole = layer.journal(None, JOURNAL_PAGE_LIMIT, None).unwrap();
    let kinds: Vec<&str> = whole.entries.iter().map(|entry| entry.kind.name()).collect();
    assert_eq!(kinds, ["commit", "commit", "commit", "start"]);
    let letters: Vec<u64> = whole.entries.iter().map(|entry| entry.letters).collect();
    assert_eq!(letters, [3, 1, 2, 0]);

    let (mut cursor, mut events) = (String::new(), Vec::new());
    loop {
        let read = run(&mut layer, "page", page(&cursor, "", 3));
        let Value::List(entries) = field(answer(&read), "entries") else { panic!("a list") };
        events.extend(entries.items().iter().map(|entry| text(field(entry, "event")).to_owned()));
        cursor = text(field(answer(&read), "next")).to_owned();
        if cursor.is_empty() {
            break;
        }
    }
    assert_eq!(events, ["e3", "e2", "e1", "e0"]);

    let any = run(&mut layer, "page_any", page("", "", 10));
    assert_eq!(names(answer(&any)), ["next", "entries"]);
    let Value::List(entries) = field(answer(&any), "entries") else { panic!("a list") };
    for (got, entry) in entries.items().iter().zip(&whole.entries) {
        assert_eq!(names(got), ["solved", "letters", "kind", "exit", "event"]);
        assert!(equal(field(got, "letters"), &Value::int(entry.letters)));
        assert_eq!(text(field(got, "event")), entry.event());
    }

    let recipe = kernel().word(&[17, -305, 911]).unwrap();
    let scrambled = match kernel().derive(&kernel().solved(), &recipe, &ProtectedSet::none()) {
        Ok(Derivation::Derived(derived)) => derived.state,
        _ => panic!("a scramble derives"),
    };
    let mut scrambled = layer_at(scrambled);
    let read = run(&mut scrambled, "page", page("", "", 1));
    let Value::List(entries) = field(answer(&read), "entries") else { panic!("a list") };
    assert!(equal(field(&entries.items()[0], "solved"), &Value::Bool(false)));
    assert_eq!(text(field(&entries.items()[0], "kind")), "start");
}

/// The bridge refuses before the row runs, at the cost of the read's step, a declared type it does not answer
/// (`LG2:type.mismatch`) and an argument of another form, a `limit` outside 1 to the page limit or a `from` that is not
/// an event's name (`KN5:input.argument_form`). Once the read is charged its `limit`, the layer refuses a `from` the
/// move tree does not hold (`KN2:stale.revision_gone`) and a cursor of another walk (`KN5:input.argument_form`).
#[test]
fn the_row_refuses_what_it_does_not_answer() {
    let mut layer = layer_with(&[&[2], &[3]]);
    let arg = page("", "", 2);
    for entry in ["page_texts", "page_short", "page_flat"] {
        let cost = refused(&mut layer, entry, &arg, TYPE_MISMATCH);
        assert_eq!(cost, refused(&mut layer, "colours", &arg, NOT_GRANTED), "{entry}: refused before the row runs");
    }
    let one = Value::int(2);
    assert_eq!(
        refused(&mut layer, "page_one", &one, code::ARGUMENT_FORM),
        refused(&mut layer, "colours_one", &one, NOT_GRANTED)
    );
    let from_int = Value::record(vec![("cursor", Value::text("")), ("from", Value::int(1)), ("limit", Value::int(2))]);
    assert_eq!(
        refused(&mut layer, "page_from_int", &from_int, code::ARGUMENT_FORM),
        refused(&mut layer, "colours_from_int", &from_int, NOT_GRANTED)
    );
    for limit in [0, -1, JOURNAL_PAGE_LIMIT as i64 + 1] {
        let arg = page("", "", limit);
        let cost = refused(&mut layer, "page_int", &arg, code::ARGUMENT_FORM);
        assert_eq!(cost, refused(&mut layer, "colours_int", &arg, NOT_GRANTED), "limit {limit}");
    }
    for from in ["x", "e", "e01", "E1", "e-1", " e1", "e18446744073709551616"] {
        let arg = page("", from, 2);
        let cost = refused(&mut layer, "page", &arg, code::ARGUMENT_FORM);
        assert_eq!(cost, refused(&mut layer, "colours", &arg, NOT_GRANTED), "{from:?}");
    }

    let charged = journal_tariff(2).kernel_cost;
    assert_eq!(charged, 2);
    let gone = page("", "e3", 2);
    let cost = refused(&mut layer, "page", &gone, code::REVISION_GONE);
    assert_eq!(cost, refused(&mut layer, "colours", &gone, NOT_GRANTED) + charged, "charged");
    let next = layer.journal(None, 1, None).unwrap().next.expect("a cursor");
    for (cursor, from) in [("e1", ""), ("x", ""), (next.as_str(), "e1")] {
        let arg = page(cursor, from, 2);
        let cost = refused(&mut layer, "page", &arg, code::ARGUMENT_FORM);
        assert_eq!(cost, refused(&mut layer, "colours", &arg, NOT_GRANTED) + charged, "{cursor:?}: charged");
    }
}

/// The tariff of a page of `limit` events charges one walk step per event and bounds the answer by `limit` of the
/// largest entries and the longest cursor; the bridge charges it for each read, and every answer fits it.
#[test]
fn the_tariff_covers_the_largest_answers() {
    let event = format!("e{}", u64::MAX);
    let mark = "f".repeat(64);
    let largest = Value::record(vec![
        ("event", Value::text(&event)),
        ("exit", Value::text(&mark)),
        ("kind", Value::text("crossing")),
        ("letters", Value::int(u64::MAX)),
        ("solved", Value::Bool(true)),
    ]);
    for limit in [1, 2, 7, JOURNAL_PAGE_LIMIT] {
        let tariff = journal_tariff(limit);
        assert_eq!(tariff.kernel_cost, u128::from(limit));
        let entries = Value::list(vec![largest.clone(); limit as usize]);
        let answer = Value::record(vec![("entries", entries), ("next", Value::text(&format!("{event}:{mark}")))]);
        assert_eq!(answer.size(), tariff.result_bound, "limit {limit}");
    }

    let mut layer = layer_with(&[&[2, -1], &[5], &[9]]);
    let mut bridge = Bridge::new(&mut layer, "direct");
    let ty = journal_type();
    for limit in [1, 2, 4, 9] {
        let mut arg = Vec::new();
        encode(&page("", "", limit as i64), &mut arg).expect("a canonical argument");
        let request = Request::SnapshotWith { query: JOURNAL, arg: &arg, ty: &ty };
        let tariff = bridge.tariff(&request).unwrap();
        assert_eq!(tariff, journal_tariff(limit));
        let got = bridge.perform(&request).unwrap();
        assert!(got.size() <= tariff.result_bound, "{} > {}", got.size(), tariff.result_bound);
    }
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
}
