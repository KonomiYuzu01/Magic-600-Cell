//! A program job's output as the runtime's sinks give it (draft; LP3's C6, its second common need):
//! `CommandLayer::finish_program_job_with` and `JobOutput`. A job reports the value its entry returned, the views it
//! published and the workspace records it wrote. The frozen text is silent on how the layer takes them, so these tests
//! follow the draft rulings recorded in `src/lib.rs`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, digest, draft_args, CommandLayer, Completion, CompletionKind, Direction, Error, HeadlessFrames, JobOutput,
    Kind, MemoryStore, Overlay, Provenance, Receipt, RecordValue, RecordWrite, ResultRef, Secret, Shell, Status, Table,
    Transaction, TransactionKind, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

const COMPARE: &str = "structure.compare";
const SCOPE: &str = "view.effect.scope";
/// A language-defined file row the tests register.
const EXPORT: &str = "tool.export";
/// A language-defined workspace row the tests register: its completion writes the output's records as they are.
/// (`set.save` writes the saved set its entry names instead: `tests/saved_sets.rs`.)
const STORE: &str = "tool.store";

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

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/representations".into(), entry: entry.into() }
}

fn compare_args(a: i64, b: i64) -> Value {
    json::object([("cells", Value::Array(vec![Value::Int(a), Value::Int(b)]))])
}

fn scope_args(scope: &str) -> Value {
    json::object([("scope", text(scope))])
}

fn record_args(name: &str, payload: Value, provenance: &str) -> Value {
    json::object([("payload", payload), ("provenance", text(provenance)), ("record", text(name))])
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

fn value(payload: Value, provenance: &str) -> RecordValue {
    RecordValue { payload, provenance: provenance.to_owned() }
}

/// An output with `records`, each a name, its payload and its provenance block.
fn output(result: Value, views: Vec<Value>, records: &[(&str, Value, &str)]) -> JobOutput {
    let records = records.iter().map(|(name, payload, block)| ((*name).to_owned(), value(payload.clone(), block)));
    JobOutput { value: result, views, records: records.collect() }
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

/// The host's provenance reader as the tests stand it in: it refuses the block `refused` and accepts every other.
struct Reader;

impl Provenance for Reader {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        match block {
            b"refused" => Err(Error::new(code::PROVENANCE_INCOMPATIBLE, "the test reader refuses the block")),
            _ => Ok(()),
        }
    }

    fn admit(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state that protects nothing, on H-08's table with the layer's drafts (`workspace.set`), with
/// the representations tool's query and view rows served, a workspace overlay and a file overlay registered, and the
/// test reader when `reader`.
fn fixture(reader: bool) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    if reader {
        layer.set_provenance(Box::new(Reader));
    }
    serve(&mut layer);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// What a host does at every open: serves its rows and registers its overlays again.
fn serve(layer: &mut CommandLayer) {
    layer.serve_row(COMPARE, overlay("compare_cells")).unwrap();
    layer.serve_row(SCOPE, overlay("effect_scope")).unwrap();
    layer.register_overlay(EXPORT, Kind::File, &[], overlay("export")).unwrap();
    layer.register_overlay(STORE, Kind::Workspace, &[], overlay("store")).unwrap();
}

/// A reopen of `layer`'s store, which replays its actions without a provenance reader.
fn reopen(layer: &CommandLayer) -> CommandLayer {
    let store = layer.store().clone();
    let mut reopened = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    serve(&mut reopened);
    reopened
}

impl Fixture {
    /// Invokes `row` through the command route, takes its job and returns the job's ID and the receipt that admitted it.
    fn start(&mut self, row: &str, args: Value) -> (u64, Receipt) {
        let started = self.shell.invoke(&mut self.layer, row, args).unwrap();
        let Status::Running(job) = started.status else { panic!("{row}: {:?}", started.status) };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job), "{row}");
        (job, started)
    }

    fn write(&mut self, name: &str, payload: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, RECORD_ROW, record_args(name, payload, "block")).unwrap()
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let press = self.shell.prepare(&self.layer, "operation.execute", preview_args(id)).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// The undo or redo of the action on top of its stack.
    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    /// The record's version and value as the layer holds it.
    fn record(&self, name: &str) -> (u64, Option<Value>) {
        let record = self.layer.records().get(name).cloned().unwrap_or_default();
        (record.version, record.value.map(|value| value.payload))
    }

    /// What an output that fails its checks may not change: the revision, the records and the durable log.
    fn fixed(&self) -> (String, String, usize) {
        let (revision, records) = (self.layer.revision(), self.layer.records());
        (format!("{revision:?}"), format!("{records:?}"), self.layer.store().log.len())
    }
}

/// A view row's and a query row's views complete them with the digest of the value and the views together, transiently:
/// no revision moves and nothing is written. The digest is the SHA-256 of the canonical `{"records", "value",
/// "views"}`, and it differs from the value's own (draft).
#[test]
fn views_complete_a_view_row_and_a_query_row_with_their_digest() {
    let mut f = fixture(false);
    let before = f.layer.revision().clone();
    let views = vec![json::object([("cells", letters(&[1, 2]))]), text("scope")];
    let cases = [
        (SCOPE, scope_args("body"), CompletionKind::ViewPublished),
        (COMPARE, compare_args(1, 2), CompletionKind::Query),
    ];
    for (row, args, kind) in cases {
        let (job, _) = f.start(row, args);
        let out = output(Value::Null, views.clone(), &[]);
        let form = json::object([
            ("records", Value::Array(Vec::new())),
            ("value", Value::Null),
            ("views", Value::Array(views.clone())),
        ]);
        assert_eq!(out.digest(), digest(&form), "{row}");
        assert_ne!(out.digest(), digest(&Value::Null), "{row}");
        let done = f.layer.finish_program_job_with(job, Ok(out.clone())).unwrap();
        let reference = ResultRef { job: Some(job), digest: out.digest() };
        assert_eq!(
            done.status,
            Status::Completed(Completion { kind, revision: before.clone(), result: Some(reference) })
        );
        assert_eq!((f.layer.revision(), f.layer.store().log.len()), (&before, 0), "{row}");
    }
}

/// A value alone keeps its digest: `JobOutput::of`'s digest is the value's own, and `finish_program_job` answers what
/// `finish_program_job_with` answers for the same value, for a query row and for a workspace row, whose durable action
/// is the same too.
#[test]
fn a_value_alone_completes_as_before() {
    let set = json::object([("members", letters(&[3, 5]))]);
    assert_eq!(JobOutput::of(set.clone()).digest(), digest(&set));
    let mut answers = Vec::new();
    for with in [false, true] {
        let mut f = fixture(false);
        let mut receipts = Vec::new();
        for (row, args) in [(COMPARE, compare_args(1, 2)), (STORE, json::object([]))] {
            let (job, _) = f.start(row, args);
            let done = match with {
                false => f.layer.finish_program_job(job, Ok(set.clone())),
                true => f.layer.finish_program_job_with(job, Ok(JobOutput::of(set.clone()))),
            };
            receipts.push(done.unwrap());
        }
        let Status::Completed(Completion { result: Some(ResultRef { digest: written, .. }), .. }) = &receipts[1].status
        else {
            panic!("{:?}", receipts[1].status)
        };
        assert_eq!(written, &digest(&set));
        answers.push((receipts, f.layer.store().log.clone()));
    }
    assert_eq!(answers[0], answers[1]);
}

/// CT-LG3-03 (part: a workspace row's completion is one durable action that moves the workspace revision once; not
/// run: the undo of a language-defined row's action, which the layer does not give).
/// A workspace row's records are written in its completion's one durable action: every record's version advances,
/// a record written before keeps counting, the workspace revision advances once, and the journal keeps the writes in
/// the order of their names with the output's digest. A reopen without a reader writes them again, and the receipt
/// survives it (KN5.23; draft).
#[test]
fn a_workspace_rows_records_are_written_in_its_one_durable_action() {
    let mut f = fixture(true);
    assert!(completed(&f.write("b", letters(&[1]))));
    assert_eq!(f.record("b"), (1, Some(letters(&[1]))));
    let before = f.layer.revision().clone();
    let (job, started) = f.start(STORE, json::object([]));
    let out = output(
        json::object([("members", letters(&[3, 5]))]),
        vec![text("shown")],
        &[("b", letters(&[2]), "block"), ("a", letters(&[3, 5]), "block")],
    );
    let done = f.layer.finish_program_job_with(job, Ok(out.clone())).unwrap();
    let mut moved = before.clone();
    moved.workspace += 1;
    let reference = ResultRef { job: Some(job), digest: out.digest() };
    let expected = Completion { kind: CompletionKind::Workspace, revision: moved.clone(), result: Some(reference) };
    assert_eq!(done.status, Status::Completed(expected));
    assert_eq!(f.layer.revision(), &moved);
    assert_eq!((f.record("a"), f.record("b")), ((1, Some(letters(&[3, 5]))), (2, Some(letters(&[2])))));

    let log = f.layer.store().log.clone();
    let kinds: Vec<TransactionKind> = log.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [TransactionKind::Record, TransactionKind::Workspace]);
    let writes = vec![
        RecordWrite { name: "a".into(), value: Some(value(letters(&[3, 5]), "block")) },
        RecordWrite { name: "b".into(), value: Some(value(letters(&[2]), "block")) },
    ];
    assert_eq!(log[1].records, writes);
    assert_eq!(log[1].result, Some(out.digest()));

    let g = reopen(&f.layer);
    assert_eq!((g.revision().workspace, g.records()), (moved.workspace, f.layer.records()));
    assert_eq!(g.outcome(&started.command, &started.request), Some(done));
}

/// CT-LG3-03 and CT-LB7-01 (part: a view row's job that returns a record fails with `LG3:capability.not_granted` and
/// writes nothing; not run: the undo of a language-defined workspace row's action).
/// Only a workspace row's job writes records: a record from a query, view or file row's job fails the job with
/// `LG3:capability.not_granted`, and nothing is written (draft).
#[test]
fn a_record_from_any_other_row_fails_its_job_and_writes_nothing() {
    let mut f = fixture(true);
    let cases = [(COMPARE, compare_args(1, 2)), (SCOPE, scope_args("complete")), (EXPORT, json::object([]))];
    for (row, args) in cases {
        let before = f.fixed();
        let (job, _) = f.start(row, args);
        let out = output(Value::Null, vec![text("shown")], &[("a", letters(&[1]), "block")]);
        let done = f.layer.finish_program_job_with(job, Ok(out)).unwrap();
        assert_eq!(code_of(&done), code::NOT_GRANTED, "{row}");
        assert_eq!(f.fixed(), before, "{row}");
    }
    assert!(f.layer.records().is_empty());
}

/// The host's reader checks every record's block, as for `workspace.set` (PS1.16): without a reader the job fails with
/// `KN5:input.row_not_built`, and a block the reader refuses fails it with the reader's error, even beside a block it
/// accepts. A job whose output fails writes nothing; a job a commit ended meanwhile is dropped and writes nothing; an
/// error the host reports fails the job as before.
#[test]
fn a_workspace_rows_records_pass_the_hosts_reader_or_nothing_is_written() {
    let mut bare = fixture(false);
    let before = bare.fixed();
    let (job, _) = bare.start(STORE, json::object([]));
    let out = output(Value::Null, Vec::new(), &[("a", letters(&[1]), "block")]);
    assert_eq!(code_of(&bare.layer.finish_program_job_with(job, Ok(out)).unwrap()), code::ROW_NOT_BUILT);
    assert_eq!(bare.fixed(), before);

    let mut f = fixture(true);
    let before = f.fixed();
    let (job, _) = f.start(STORE, json::object([]));
    let out = output(Value::Null, Vec::new(), &[("a", letters(&[1]), "block"), ("b", letters(&[2]), "refused")]);
    let done = f.layer.finish_program_job_with(job, Ok(out)).unwrap();
    let Status::Failed(error) = &done.status else { panic!("{:?}", done.status) };
    assert_eq!(
        (error.code, error.detail.as_str()),
        (code::PROVENANCE_INCOMPATIBLE, "the test reader refuses the block")
    );
    assert_eq!(f.fixed(), before);

    let (job, _) = f.start(STORE, json::object([]));
    let failed = Error::new(code::NOT_GRANTED, "the entry failed");
    let done = f.layer.finish_program_job_with(job, Err(failed.clone())).unwrap();
    assert_eq!(done.status, Status::Failed(failed));
    assert_eq!(f.fixed(), before);

    let (job, _) = f.start(STORE, json::object([]));
    assert!(committed(&f.commit(&word(&[1]))));
    let late = output(Value::Null, Vec::new(), &[("a", letters(&[1]), "block")]);
    assert!(f.layer.finish_program_job_with(job, Ok(late)).is_none());
    assert!(f.layer.records().is_empty());
    assert_eq!(f.layer.store().log.len(), 1);
}

/// No undo reaches a job's writes, so an undo or redo of an earlier write of the same record would write over them.
/// Those earlier writes leave both stacks when the job's records are written, and the writes of other records and the
/// commits keep their places. A reopen cuts the history the same way (draft).
#[test]
fn no_undo_or_redo_writes_over_a_jobs_records() {
    let mut f = fixture(true);
    assert!(completed(&f.write("a", letters(&[1]))));
    assert!(completed(&f.write("keep", letters(&[9]))));
    assert!(committed(&f.commit(&word(&[1]))));
    assert!(completed(&f.write("a", letters(&[2]))));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.record("a"), (3, Some(letters(&[1]))));
    let commit = f.layer.undo_hint(Direction::Undo).unwrap().target;
    assert!(f.layer.undo_hint(Direction::Redo).is_some());

    let (job, _) = f.start(STORE, json::object([]));
    let out = output(Value::Null, Vec::new(), &[("a", letters(&[5]), "block")]);
    assert!(completed(&f.layer.finish_program_job_with(job, Ok(out)).unwrap()));
    assert_eq!(f.record("a"), (4, Some(letters(&[5]))));
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), Some(commit));
    assert!(f.layer.undo_hint(Direction::Redo).is_none());

    let g = reopen(&f.layer);
    for direction in [Direction::Undo, Direction::Redo] {
        assert_eq!(g.undo_hint(direction), f.layer.undo_hint(direction), "{direction:?}");
    }
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(f.layer.undo_hint(Direction::Undo).is_none());
    assert_eq!((f.record("a"), f.record("keep")), ((4, Some(letters(&[5]))), (2, None)));
}

/// The journal form (draft): a workspace completion has its job's `budget` as an 18th key (`tests/budget.rs`); one
/// with records has `records` as a 19th, a list of writes in the order of their names, and reads back to itself.
/// `records` on another kind of action, an empty list, names out of order or repeated, a write with no value and a
/// list that is not one do not read.
#[test]
fn a_workspace_completion_journals_its_records() {
    let mut f = fixture(true);
    let (job, _) = f.start(STORE, json::object([]));
    let out = output(Value::Null, Vec::new(), &[("b", letters(&[2]), "block"), ("a", letters(&[1]), "block")]);
    assert!(completed(&f.layer.finish_program_job_with(job, Ok(out)).unwrap()));
    let (job, _) = f.start(STORE, json::object([]));
    assert!(completed(&f.layer.finish_program_job(job, Ok(Value::Null)).unwrap()));
    assert!(completed(&f.write("w", letters(&[3]))));
    let log = f.layer.store().log.clone();
    let keys: Vec<usize> = log.iter().map(|t| t.to_json().as_object().unwrap().len()).collect();
    assert_eq!(keys, [19, 18, 18]);
    let names: Vec<Value> = log[0]
        .to_json()
        .get("records")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.get("name").unwrap().clone())
        .collect();
    assert_eq!(names, [text("a"), text("b")]);
    for transaction in &log {
        assert_eq!(Transaction::from_json(&transaction.to_json()).as_ref(), Some(transaction));
    }

    let object_of = |transaction: &Transaction| match transaction.to_json() {
        Value::Object(fields) => fields,
        _ => panic!("a transaction is an object"),
    };
    let records = log[0].to_json().get("records").unwrap().clone();
    let Value::Array(writes) = records.clone() else { panic!("a list") };
    let null = json::object([("name", text("c")), ("value", Value::Null)]);
    let malformed = [
        (object_of(&log[1]), Value::Array(Vec::new()), "an empty list"),
        (object_of(&log[1]), Value::Array(vec![writes[1].clone(), writes[0].clone()]), "names out of order"),
        (object_of(&log[1]), Value::Array(vec![writes[0].clone(), writes[0].clone()]), "a name repeated"),
        (object_of(&log[1]), Value::Array(vec![writes[0].clone(), null]), "a write with no value"),
        (object_of(&log[1]), writes[0].clone(), "a write that is not in a list"),
        (object_of(&log[2]), records, "a record write with records"),
    ];
    for (mut fields, records, what) in malformed {
        fields.insert("records".into(), records);
        assert_eq!(Transaction::from_json(&Value::Object(fields)), None, "{what}");
    }
    let mut commit = fixture(false);
    assert!(committed(&commit.commit(&word(&[1]))));
    let mut fields = object_of(&commit.layer.store().log[0]);
    fields.insert("records".into(), log[0].to_json().get("records").unwrap().clone());
    assert_eq!(Transaction::from_json(&Value::Object(fields)), None, "a commit with records");
}
