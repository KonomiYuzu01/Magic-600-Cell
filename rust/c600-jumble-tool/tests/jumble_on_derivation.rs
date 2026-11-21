//! CT-LB3-07 (chapter 03, LB3.5) on LB3.1's "Mode crossing prerequisites", the Jumble-on half (LP3, C8): Full labels
//! with journal derivation. `jumble_crossing.jumble_on_facts` runs through LP2's interpreter against `c600-tools`' host
//! bridge ([`Bridge`]) onto a command layer with the I_a menu pinned, and reads KN4.3's draft `journal` row (LP2's
//! pull request 383) from the head. Each answer is checked against `jumble.on` pressed at the same head:
//!
//! - an entrance the head's line reaches through retained commits only is the entrance of the crossing record that
//!   `jumble.on` keeps, `solved_witness` or `exit_root` with the exit's event and the digest of its exit record, with as
//!   many signed primitive letters as the record counts;
//! - a head with Jumble on is `jumble.on`'s `KN5:input.retained_tool_needs_jumble_off`, and a line with no entrance (a
//!   session that started away from solved) is its `KN5:input.jumble_on_labels_only`, each changing nothing;
//! - an undo of the entry, of a commit and of the exit moves the head, and the answer follows it: an undone exit keeps
//!   its event and mark.
//!
//! The entry selects nothing and decides no crossing: every read sends no invocation and changes nothing, and each
//! answer has exactly its seven fields. As built it reads one page; a variant that reads one event a page reads a page
//! for each event it walks, passing each page's cursor on, to the same answer, and one that stops after two such pages
//! answers that it reached no entrance.
//! A pass is source and fixture evidence.

mod common;

use std::collections::BTreeSet;
use std::sync::OnceLock;

use c600_command::{code, digest, TransactionKind, JOURNAL_PAGE_LIMIT};
use c600_digest::sha256;
use c600_jumble::levels;
use c600_jumble_tool::library::CROSSING;
use c600_kernel::json::Value;
use c600_kernel::{Derivation, ProtectedSet};
use c600_library::lower_source;
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Sink};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{self, Author, Json, Producer, Profile};
use c600_run::value::Value as RunValue;
use c600_tools::bridge::{base, Bridge};
use c600_tools::tool::{checker, contract, named_word_block, pins};

use common::{committed, engine, kernel, Fixture, Session};

/// Run limits wide enough for every run here.
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

const RUN: &str = "c8-crossing";
const ENTRY: &str = "jumble_on_facts";

/// The answer's fields, in order.
const FIELDS: [&str; 7] = ["commits", "crossing", "entrance", "exit", "jumble_on", "primitives", "reached"];

/// The module as built, a variant that reads one event a page, and that variant stopped after two pages.
fn variants() -> &'static [(String, Loaded); 3] {
    static LOADED: OnceLock<[(String, Loaded); 3]> = OnceLock::new();
    LOADED.get_or_init(|| {
        let one = CROSSING.source.replace("limit = 4096", "limit = 1");
        let two = one.replace("repeat(256, start,", "repeat(2, start,");
        assert!(one != CROSSING.source && two != one, "the variants differ from the module as built");
        [CROSSING.source.to_owned(), one, two].map(|source| {
            let program = lower_source(&source).expect("the module lowers");
            let report = c600_check::check(&program).expect("well-formed IR");
            assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
            let loaded = load(&program).expect("the module loads");
            (source, loaded)
        })
    })
}

/// What the entry answers.
#[derive(Clone, Debug, PartialEq)]
struct Facts {
    commits: i64,
    crossing: String,
    entrance: String,
    exit: String,
    jumble_on: bool,
    primitives: i64,
    reached: bool,
}

impl Facts {
    fn solved_witness(commits: i64, primitives: i64) -> Facts {
        Facts { entrance: "solved_witness".to_owned(), ..Facts::reached(commits, primitives) }
    }

    fn exit_root(crossing: &str, exit: &str, commits: i64, primitives: i64) -> Facts {
        Facts {
            crossing: crossing.to_owned(),
            entrance: "exit_root".to_owned(),
            exit: exit.to_owned(),
            ..Facts::reached(commits, primitives)
        }
    }

    fn jumble_on() -> Facts {
        Facts { jumble_on: true, ..Facts::reached(0, 0) }
    }

    /// A walk that reached the start or a node that is no commit, and no entrance.
    fn reached(commits: i64, primitives: i64) -> Facts {
        let none = String::new;
        Facts { commits, crossing: none(), entrance: none(), exit: none(), jumble_on: false, primitives, reached: true }
    }
}

fn number(value: &RunValue) -> i64 {
    match value {
        RunValue::Int(n) => n.to_string().parse().expect("an integer of i64"),
        other => panic!("no integer: {other:?}"),
    }
}

fn truth(value: &RunValue) -> bool {
    match value {
        RunValue::Bool(b) => *b,
        other => panic!("no Bool: {other:?}"),
    }
}

fn string(value: &RunValue) -> String {
    match value {
        RunValue::Text(text) => text.as_str().to_owned(),
        other => panic!("no Text: {other:?}"),
    }
}

/// A host that keeps every row a run reads, with its argument, and leaves the rest to `inner`.
struct Counting<'h, H: Host> {
    inner: &'h mut H,
    reads: Vec<(String, Option<Json>)>,
}

impl<H: Host> Host for Counting<'_, H> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.inner.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<RunValue, &'static str> {
        match *request {
            Request::Snapshot { query, .. } => self.reads.push((query.to_owned(), None)),
            Request::SnapshotWith { query, arg, .. } => self.reads.push((query.to_owned(), provenance::parse(arg))),
            _ => {}
        }
        self.inner.perform(request)
    }

    fn sink(&mut self, sink: &Sink, value: &RunValue, bytes: &[u8]) -> Result<(), &'static str> {
        self.inner.sink(sink, value, bytes)
    }
}

/// A page's argument: its cursor, its `from` and its limit, written canonically.
fn page(cursor: &str, limit: i64) -> Json {
    let limit = Json::object([("int", Json::text(&limit.to_string()))]);
    Json::object([("cursor", Json::text(cursor)), ("from", Json::text("")), ("limit", limit)])
}

/// The cursor a read passed.
fn cursor(read: &(String, Option<Json>)) -> &str {
    match read.1.as_ref().and_then(|arg| arg.get("cursor")) {
        Some(Json::Text(cursor)) => cursor,
        other => panic!("a page argument: {other:?}"),
    }
}

/// Variant `variant` of the entry, run at `session`'s head through a boundary and the bridge, in the solving profile,
/// built in, with a state read (LG3.13): its answer and the rows it read, each with its argument. A read sends no
/// invocation and changes nothing.
fn read(session: &mut Session, variant: usize, what: &str) -> (Facts, Vec<(String, Option<Json>)>) {
    let (source, loaded) = &variants()[variant];
    let layer = &mut session.layer;
    let before = (layer.revision().clone(), layer.store().log.len());
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(source.as_bytes()),
        entry: ENTRY.into(),
        profile: Profile::Solving,
    };
    let (base, inputs) = (base(layer.revision()), vec![named_word_block()]);
    let context = Context { run: RUN.into(), base, producer, reads_state: true, inputs, pins: pins() };
    let mut bridge = Bridge::new(&mut *layer, RUN);
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let mut host = Counting { inner: &mut boundary, reads: Vec::new() };
    let run = loaded.run(CROSSING.name, ENTRY, vec![], &mut host, &Options::new(WIDE)).expect("the run starts");
    let reads = host.reads;
    drop(boundary);
    assert!(bridge.sent.is_empty(), "{what}: a read sends no invocation");
    drop(bridge);
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before, "{what}: a read changes nothing");
    let summary = &run.summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Completed, None), "{what}: {:?}", summary.stop);
    let Some(RunValue::Record(record)) = &run.value else { panic!("{what}: a record: {:?}", run.value) };
    let names: Vec<&str> = record.fields().iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, FIELDS, "{what}: the answer's fields and no other");
    let get = |name: &str| record.get(name).expect("a field");
    let facts = Facts {
        commits: number(get("commits")),
        crossing: string(get("crossing")),
        entrance: string(get("entrance")),
        exit: string(get("exit")),
        jumble_on: truth(get("jumble_on")),
        primitives: number(get("primitives")),
        reached: truth(get("reached")),
    };
    (facts, reads)
}

/// The entry's answer at the head, which must be `expected`. As built it reads one page of the journal from the head.
/// Read one event a page, it reads a page for each event it walks, the first with no cursor and each other with the
/// cursor the page before it gave, to the same answer; stopped after two such pages, a longer walk reaches no entrance.
fn facts(session: &mut Session, expected: &Facts, what: &str) {
    let (facts, reads) = read(session, 0, what);
    assert_eq!(&facts, expected, "{what}");
    let limit = i64::try_from(JOURNAL_PAGE_LIMIT).expect("a small limit");
    assert_eq!(reads, [("journal".to_owned(), Some(page("", limit)))], "{what}: one page from the head");
    let (paged, reads) = read(session, 1, what);
    assert_eq!(&paged, expected, "{what}: one event a page");
    let walked = usize::try_from(expected.commits).expect("a count") + 1;
    assert_eq!(reads.len(), walked, "{what}: a page for each event walked");
    assert_eq!(reads[0], ("journal".to_owned(), Some(page("", 1))), "{what}: the first page");
    let cursors: BTreeSet<&str> = reads[1..].iter().map(cursor).collect();
    assert!(!cursors.contains("") && cursors.len() == walked - 1, "{what}: each page on from a cursor of its own");
    for read in &reads[1..] {
        assert_eq!(read, &("journal".to_owned(), Some(page(cursor(read), 1))), "{what}: a page on");
    }
    let (short, reads) = read(session, 2, what);
    assert_eq!(reads.len(), walked.min(2), "{what}: at most two pages");
    if walked <= 2 {
        assert_eq!(&short, expected, "{what}: two pages reach the entrance");
    } else {
        let (commits, entrance) = (short.commits, short.entrance.as_str());
        assert!(!short.reached && commits == 2 && entrance.is_empty(), "{what}: two pages reach none: {short:?}");
    }
}

/// `jumble.on` at the head, which must commit, checked against `expected`, the entry's answer there: the crossing
/// record the entry's transaction keeps has its entrance, its exit's event and mark, and as many letters.
fn enter(session: &mut Session, expected: &Facts, what: &str) {
    let receipt = session.on();
    assert!(committed(&receipt), "{what}: `jumble.on` answered {:?}", receipt.status);
    let transaction = session.layer.store().log.last().expect("the entry's transaction");
    assert!(matches!(transaction.kind, TransactionKind::Crossing), "{what}: {:?}", transaction.kind);
    let record = transaction.crossing.clone().expect("the entry's crossing record");
    let text = |name: &str| record.get(name).and_then(Value::as_str).map(str::to_owned);
    assert_eq!(text("entrance").as_deref(), Some(expected.entrance.as_str()), "{what}: the entrance");
    let letters = record.get("letters").and_then(Value::as_i64);
    assert_eq!(letters, Some(expected.primitives), "{what}: the letters replayed");
    if expected.entrance == "exit_root" {
        assert_eq!(text("crossing").as_deref(), Some(expected.crossing.as_str()), "{what}: the exit's event");
        assert_eq!(text("exit").as_deref(), Some(expected.exit.as_str()), "{what}: the exit's mark");
    } else {
        assert!(record.get("crossing").is_none() && record.get("exit").is_none(), "{what}: no exit");
    }
}

/// `jumble.on` at the head, which must be refused with `code` and change nothing.
fn refused(session: &mut Session, code: &str, what: &str) {
    let before = session.fixed();
    let receipt = session.on();
    assert_eq!(receipt.status.code(), Some(code), "{what}: `jumble.on` answered {:?}", receipt.status);
    assert_eq!(session.fixed(), before, "{what}: the refusal changes nothing");
}

/// CT-LB3-07 (LB3.1's "Mode crossing prerequisites", the Jumble-on half). In a session at solved under I_a's menu:
/// the solved witness through two commits, with Jumble off; Jumble on after the entry and after a pose move; the exit
/// root of `jumble.off` with no commit and with one; and the head moved by undo and redo. In a session that started
/// away from solved, no entrance. At each head the entry's answer is checked, and `jumble.on` pressed there agrees.
#[test]
fn ct_lb3_07_the_crossing_entry_reports_the_jumble_on_derivation() {
    let fixture = Fixture::load("I_a");
    let mut session = Session::pinned_at(&fixture, kernel().solved());
    facts(&mut session, &Facts::solved_witness(0, 0), "at the start");
    assert!(committed(&session.commit(&[2, -1, 7])));
    facts(&mut session, &Facts::solved_witness(1, 3), "after a commit");
    assert!(committed(&session.commit(&[5])));
    let witness = Facts::solved_witness(2, 4);
    facts(&mut session, &witness, "after two commits");
    enter(&mut session, &witness, "from the solved witness");

    facts(&mut session, &Facts::jumble_on(), "at the entry");
    refused(&mut session, code::RETAINED_TOOL_NEEDS_JUMBLE_OFF, "at the entry");
    let retained = levels::primitive(engine(), 1).expect("a retained generator");
    assert!(committed(&session.press(&retained)), "a pose move");
    facts(&mut session, &Facts::jumble_on(), "after a pose move");
    refused(&mut session, code::RETAINED_TOOL_NEEDS_JUMBLE_OFF, "after a pose move");

    let receipt = session.off();
    assert!(committed(&receipt), "`jumble.off` answered {:?}", receipt.status);
    let exit = session.layer.store().log.last().expect("the exit's transaction");
    assert!(matches!(exit.kind, TransactionKind::Exit), "{:?}", exit.kind);
    let (event, mark) = (exit.event.clone(), digest(exit.crossing.as_ref().expect("the exit record")));
    let head = session.layer.journal(None, 1, None).expect("the journal reads").entries[0].clone();
    assert_eq!((head.event(), head.exit.as_deref()), (event.clone(), Some(mark.as_str())), "the journal's exit");
    let at_exit = Facts::exit_root(&event, &mark, 0, 0);
    facts(&mut session, &at_exit, "at the exit");
    assert!(committed(&session.commit(&[17, -305, 911])));
    let root = Facts::exit_root(&event, &mark, 1, 3);
    facts(&mut session, &root, "after a commit past the exit");
    enter(&mut session, &root, "from the exit root");

    assert!(committed(&session.undo()), "the entry's undo");
    facts(&mut session, &root, "after the entry's undo");
    assert!(committed(&session.undo()), "the commit's undo");
    facts(&mut session, &at_exit, "after the commit's undo");
    assert!(committed(&session.undo()), "the exit's undo");
    facts(&mut session, &Facts::jumble_on(), "after the exit's undo");
    assert!(committed(&session.redo()), "the exit's redo");
    facts(&mut session, &at_exit, "after the exit's redo: the undone exit keeps its event and mark");
    enter(&mut session, &at_exit, "from the exit root with no commit");

    let word = kernel().word(&[17, -305, 911]).expect("a word");
    let scrambled = match kernel().derive(&kernel().solved(), &word, &ProtectedSet::none()) {
        Ok(Derivation::Derived(derived)) => derived.state,
        _ => panic!("a scramble derives"),
    };
    let mut elsewhere = Session::pinned_at(&fixture, scrambled);
    facts(&mut elsewhere, &Facts::reached(0, 0), "at a start away from solved");
    refused(&mut elsewhere, code::JUMBLE_ON_LABELS_ONLY, "at a start away from solved");
    assert!(committed(&elsewhere.commit(&[2, -1, 7])));
    assert!(committed(&elsewhere.commit(&[5])));
    assert!(committed(&elsewhere.commit(&[-5])));
    facts(&mut elsewhere, &Facts::reached(3, 5), "after three commits from a start away from solved");
    refused(&mut elsewhere, code::JUMBLE_ON_LABELS_ONLY, "after three commits from a start away from solved");
}
