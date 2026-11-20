//! The repair list through the command layer, taken up by C8 (LP3 plan section 8; LP2's pull request 326, KN5's job
//! that names its cap): H-08's `jumble.repair.list {snapshot, cap, max_chunk}` run as a job by
//! `CommandLayer::run_jobs`, beside the jumbling tool's own entry `jumble_repair.repair_list_at(cap)` run at the
//! layer's head through the host bridge (`Rows::at_head`). The tests read the layer's and the tool's answers and add no
//! rule of their own.
//!
//! The two answers differ only in form, and the tests compare them through it:
//!
//! - The job keeps M8's list whole and adds the layer's `statement`; the bridge serves five of its fields (status,
//!   input digest, parameters, message and rows) and the tool returns them as read, with no `statement`, since the
//!   wording is the layer's (BND.2).
//! - The job's rows are arrays `[x, d, y, z, delta]` and the bridge's records `{ x, d, y, z, delta }`: both are
//!   compared as five integers each, in order. A list with no rows is `null` in the job and the empty list in the run,
//!   whose declared type has no null.
//! - The run's value is canonical bytes, an integer written `{"int": "<digits>"}`, and a field the entry declares
//!   `List<Text>` holds the job's one text, or none for its null.
//!
//! The walk is the I_a fixture's (`research/jumbling/fixtures/wj-I_a.json`) in a solving session with LP2's drafted
//! rows admitted. After its first two records caps 57 and 86 are the admissible misaligned caps and the list at cap 57
//! has rows. After the first record alone cap 57 is admissible and misaligned but meets no misaligned cap, so its
//! completed list is empty: the job words it as BND.2 states and the tool returns the empty rows as read. Cap 57 at
//! solved, after `jumble.on`, and cap 0 after two records are closed caps: both answer `error` with the same message
//! and no rows.
//!
//! Neither side picks a cap. Each run of the entry reads the `repair_list` row once, with the cap it was given, and the
//! layer queues nothing beyond the job the invocation names. The layer's refusals of an invocation with no cap, or with
//! a cap that is not a `CapId`, are `c600-command/tests/repair_list.rs`'s
//! `no_job_starts_for_a_cap_the_invocation_does_not_name`, and are not repeated here. A pass is source and fixture
//! evidence.

mod common;

use std::sync::OnceLock;

use c600_jumble_tool::library::{self, REPAIR};
use c600_jumble_tool::rows::Rows;
use c600_jumble_tool::word::Alphabet;
use c600_kernel::json::{self, Value};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Sink};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::{Amounts, Outcome};
use c600_run::value::Value as RunValue;

use common::{committed, engine, Fixture, Session};

/// Run limits wide enough for every run here.
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The fields the bridge serves and the entry returns.
const READ_FIELDS: [&str; 5] = ["input_digest", "message", "parameters", "rows", "status"];

/// The names of a row's five integers in the bridge's records, in the job's array order.
const ROW: [&str; 5] = ["x", "d", "y", "z", "delta"];

/// The library's modules, lowered together, checked with no diagnostic and loaded once.
fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| {
        let program = library::program(&[]).expect("the library lowers");
        let report = c600_check::check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        load(&program).expect("the library loads")
    })
}

/// `fixture`'s menu as the pinned alphabet of a run's menu words.
fn pinned(fixture: &Fixture) -> MenuAlphabet {
    Alphabet::of(engine(), &fixture.menu()).expect("an inverse-closed menu").pinned().expect("pinned")
}

/// The bridge at the head, with a record of each row the run read and the argument it read it with.
struct Recorded<'a> {
    rows: Rows<'a>,
    reads: Vec<(String, Option<String>)>,
}

impl Host for Recorded<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.rows.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<RunValue, &'static str> {
        let read = match *request {
            Request::Snapshot { query, .. } => (query.to_owned(), None),
            Request::SnapshotWith { query, arg, .. } => {
                (query.to_owned(), Some(String::from_utf8_lossy(arg).trim_end().to_owned()))
            }
            _ => ("another request".to_owned(), None),
        };
        self.reads.push(read);
        self.rows.perform(request)
    }

    fn sink(&mut self, sink: &Sink, value: &RunValue, bytes: &[u8]) -> Result<(), &'static str> {
        self.rows.sink(sink, value, bytes)
    }
}

/// What a run of `repair_list_at` gave: its outcome and code, its value read back from canonical bytes as JSON, and the
/// rows it read with their arguments.
#[derive(Debug)]
struct Ran {
    outcome: Outcome,
    code: Option<&'static str>,
    value: Option<Value>,
    reads: Vec<(String, Option<String>)>,
}

/// `jumble_repair.repair_list_at(cap)` at `session`'s head through the bridge.
fn tool(session: &Session, pinned: &MenuAlphabet, cap: u32) -> Ran {
    let mut host = Recorded { rows: Rows::at_head(engine(), &session.layer), reads: Vec::new() };
    let options = Options { menu: Some(pinned), ..Options::new(WIDE) };
    let args = vec![RunValue::int(cap)];
    let run = loaded().run(REPAIR.name, "repair_list_at", args, &mut host, &options).expect("the run starts");
    let value = run.value.as_ref().map(|value| {
        let mut out = Vec::new();
        encode(value, &mut out).expect("a canonical value");
        json::parse(&out).expect("canonical bytes are JSON")
    });
    Ran { outcome: run.summary.outcome, code: run.summary.code, value, reads: host.reads }
}

/// `value` with each canonical integer `{"int": "<digits>"}` read back as an integer.
fn plain(value: &Value) -> Value {
    match value {
        Value::Object(fields) => match (fields.len(), fields.get("int").and_then(Value::as_str)) {
            (1, Some(digits)) => Value::Int(digits.parse().expect("an integer in range")),
            _ => Value::Object(fields.iter().map(|(name, inner)| (name.clone(), plain(inner))).collect()),
        },
        Value::Array(items) => Value::Array(items.iter().map(plain).collect()),
        other => other.clone(),
    }
}

/// The items of a field, for a field the entry may declare a list: none for null, the elements of an array, else the
/// one value.
fn items(value: &Value) -> Vec<Value> {
    match plain(value) {
        Value::Null => Vec::new(),
        Value::Array(items) => items,
        one => vec![one],
    }
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    value.get(name).unwrap_or_else(|| panic!("no field `{name}` in {value:?}"))
}

/// The job's rows as five integers each: none for null.
fn job_rows(list: &Value) -> Vec<[i64; 5]> {
    let rows = match field(list, "rows") {
        Value::Null => return Vec::new(),
        rows => rows.as_array().expect("rows are an array"),
    };
    let row = |row: &Value| {
        let row = row.as_array().expect("a row is an array");
        assert_eq!(row.len(), 5, "a row is five integers");
        std::array::from_fn(|i| row[i].as_i64().expect("an integer"))
    };
    rows.iter().map(row).collect()
}

/// The run's rows as five integers each, in the job's array order.
fn read_rows(read: &Value) -> Vec<[i64; 5]> {
    let rows = field(read, "rows").as_array().expect("the declared list");
    let row = |row: &Value| {
        assert_eq!(row.as_object().map(|row| row.len()), Some(5), "a row is five integers and no mark");
        ROW.map(|name| field(row, name).as_i64().expect("an integer"))
    };
    rows.iter().map(row).collect()
}

/// The job's kept list `list` at `cap` and the tool's run at the same head agree: status, input digest, parameters,
/// message and rows, in order. The run completes, reads the `repair_list` row once with `cap`, and returns no
/// `statement`. Returns the rows.
fn agree(list: &Value, ran: &Ran, cap: i64, what: &str) -> Vec<[i64; 5]> {
    assert_eq!((ran.outcome, ran.code), (Outcome::Completed, None), "{what}: the entry completes");
    let named = format!(r#"{{"int":"{cap}"}}"#);
    assert_eq!(ran.reads, [("repair_list".to_owned(), Some(named))], "{what}: one read, with the named cap");
    let read = plain(ran.value.as_ref().expect("the list as read"));
    let names: Vec<&str> = read.as_object().expect("a record").keys().map(String::as_str).collect();
    assert_eq!(names, READ_FIELDS, "{what}: the five read fields and no `statement`");

    for name in ["status", "input_digest"] {
        assert_eq!(field(&read, name), field(list, name), "{what}: {name}");
    }
    assert_eq!(items(field(&read, "message")), items(field(list, "message")), "{what}: message");
    let (theirs, ours) = (field(&read, "parameters"), field(list, "parameters"));
    let keys = |value: &Value| value.as_object().expect("parameters").keys().cloned().collect::<Vec<_>>();
    assert_eq!(keys(theirs), keys(ours), "{what}: the parameters' names");
    for name in keys(ours) {
        assert_eq!(items(field(theirs, &name)), items(field(ours, &name)), "{what}: parameter {name}");
    }
    assert_eq!(field(ours, "cap"), &Value::Int(cap), "{what}: the job's list names its cap");

    let rows = job_rows(list);
    assert_eq!(read_rows(&read), rows, "{what}: the rows, as five integers each, in order");
    let error = field(list, "status").as_str() == Some("error");
    assert_eq!(field(list, "rows") == &Value::Null, error, "{what}: only an error has null rows");
    rows
}

/// The job at `cap` and the tool's read at the same head, which must agree. The layer queues nothing more. Returns the
/// job, its kept list and the rows.
fn both(session: &mut Session, pinned: &MenuAlphabet, cap: i64, what: &str) -> (u64, Value, Vec<[i64; 5]>) {
    let (job, list) = session.list(cap);
    assert_eq!(session.layer.run_jobs(), 0, "{what}: no job beyond the one the invocation names");
    let ran = tool(session, pinned, cap as u32);
    let rows = agree(&list, &ran, cap, what);
    (job, list, rows)
}

/// The I_a session with its first `n` records pressed.
fn walked(fixture: &Fixture, n: usize) -> Session {
    let mut session = Session::entered_solving(fixture);
    for (index, twist) in fixture.twists[..n].iter().enumerate() {
        assert!(committed(&session.press(twist)), "record {index}");
    }
    session
}

/// Step 1: after the first two I_a records, at the named cap 57, the job's kept list without `statement` and the
/// tool's `repair_list_at(57)` at the same head agree, and the list has rows, so the job's `statement` is null.
#[test]
fn at_a_named_misaligned_cap_the_jobs_list_and_the_tools_read_agree() {
    let fixture = Fixture::load("I_a");
    let pinned = pinned(&fixture);
    let mut session = walked(&fixture, 2);
    let (_, list, rows) = both(&mut session, &pinned, 57, "cap 57 after two records");
    assert_eq!(field(&list, "status").as_str(), Some("completed"));
    assert!(!rows.is_empty(), "pairs at cap 57 lower N");
    assert_eq!(field(&list, "statement"), &Value::Null, "a list with rows needs no statement");
    assert_eq!(field(&list, "input_digest").as_str(), Some(session.layer.revision().digest.as_str()));
}

/// Step 2: at a closed cap, cap 57 at solved after `jumble.on` and cap 0 after two records, both answer `error` with
/// the same message and no rows, and the job words nothing. At cap 57 after the first record alone, an admissible
/// misaligned cap that meets no misaligned cap, the list completes empty: the job words it as BND.2 states and the
/// tool returns the empty rows as read.
#[test]
fn a_closed_cap_is_an_error_on_both_sides_and_an_empty_list_is_worded_by_the_layer_only() {
    let fixture = Fixture::load("I_a");
    let pinned = pinned(&fixture);
    let closed = [(0, 57, "cap 57 at solved"), (2, 0, "cap 0 after two records")];
    for (records, cap, what) in closed {
        let mut session = walked(&fixture, records);
        let (_, list, rows) = both(&mut session, &pinned, cap, what);
        assert_eq!(field(&list, "status").as_str(), Some("error"), "{what}");
        assert!(rows.is_empty(), "{what}: no rows");
        assert!(!items(field(&list, "message")).is_empty(), "{what}: the message says why");
        assert_eq!(field(&list, "statement"), &Value::Null, "{what}: an error carries no statement");
    }

    let mut session = walked(&fixture, 1);
    let (_, list, rows) = both(&mut session, &pinned, 57, "cap 57 after one record");
    assert_eq!(field(&list, "status").as_str(), Some("completed"), "cap 57 is admissible and misaligned");
    assert!(rows.is_empty(), "no misaligned cap meets cap 57");
    assert_eq!(field(&list, "statement").as_str(), Some("No pair at cap 57 lowers N."), "BND.2's wording");
}

/// Step 3: after one more press the earlier job's list is dropped, and a new job at the new revision agrees with the
/// tool's read at the new head. Undo brings back the earlier head and drops that list too; there a new job and the
/// tool's read agree again, with the list of step 1.
#[test]
fn a_press_drops_the_kept_list_and_undo_brings_back_the_same_list() {
    let fixture = Fixture::load("I_a");
    let pinned = pinned(&fixture);
    let mut session = walked(&fixture, 2);
    let (first, before, rows) = both(&mut session, &pinned, 57, "cap 57 after two records");

    assert!(committed(&session.press(&fixture.twists[2])), "record 2");
    assert_eq!(session.layer.repair_list(first), None, "the press drops the earlier list");
    let (next, after, _) = both(&mut session, &pinned, 57, "cap 57 after three records");
    assert_ne!(field(&after, "input_digest"), field(&before, "input_digest"), "the list reads the new head");
    assert_eq!(field(&after, "input_digest").as_str(), Some(session.layer.revision().digest.as_str()));

    assert!(committed(&session.undo()), "the undo of record 2");
    assert_eq!(session.layer.repair_list(next), None, "the undo drops that list");
    let (_, again, again_rows) = both(&mut session, &pinned, 57, "cap 57 after the undo");
    assert_eq!(again, before, "the undo brings back the list of step 1");
    assert_eq!(again_rows, rows);
}

/// Step 4: neither side picks a cap. At the head after two records, where caps 57 and 86 are both admissible and
/// misaligned, each run reads the `repair_list` row once, with the cap it was given (`agree`), and each invocation
/// queues one job, for its own cap, and nothing more (`both`). Each list names its own cap. The refusals of an
/// invocation that names no cap are `c600-command/tests/repair_list.rs`'s.
#[test]
fn neither_side_picks_a_cap() {
    let fixture = Fixture::load("I_a");
    let pinned = pinned(&fixture);
    let mut session = walked(&fixture, 2);
    for cap in [86, 57] {
        let (_, list, rows) = both(&mut session, &pinned, cap, &format!("cap {cap} after two records"));
        assert_eq!(field(field(&list, "parameters"), "cap"), &Value::Int(cap), "the list names cap {cap}");
        assert!(!rows.is_empty(), "cap {cap} has rows");
        let other = if cap == 57 { 86 } else { 57 };
        assert!(rows.iter().all(|row| row[1] == other), "cap {cap}'s pairs meet cap {other} only");
    }
}
