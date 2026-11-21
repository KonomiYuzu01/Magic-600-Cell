//! The durable leg of LP2's saved-set drafts (`set.save`; U1-057, U4-037, PS1.28, KN5.23; LP2's pull request 337) on
//! KN5's command layer over M6's journal (`JournalStore`), with C5's provenance reader installed and the row served as
//! LP3's host serves it at every open. The tests finish the save's job with the entry's record under a hand-entered
//! block, where the host runs the representations tool's `save_set`: `c600-repr`'s `tests/served_rows.rs` runs the
//! entry itself, and `c600-command`'s `tests/saved_sets.rs` runs the drafts on the layer's memory store.

mod common;

use std::collections::BTreeMap;

use c600_command::{
    code, Caller, Completion, CompletionKind, Direction, Invocation, JobOutput, Overlay, Receipt, RecordValue,
    ResultRef, SavedSet, Status, RECORD_ROW, REPLACE, SET_SAVE_RECORD, SET_SAVE_ROW,
};
use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value};
use c600_session::disk::{Disk, FsDisk, MemDisk};
use common::layer::{completed, record_args, refused, text, Fixture};
use common::{manual, TempDir, JOURNAL};

/// The tool module and entry that serve `set.save` in LP3's host: `c600-repr`'s representations tool.
const MODULE: &str = "representations";
const ENTRY: &str = "save_set";

/// What the host does at every open: serves `set.save` with the tool's entry. Serving is not journaled.
fn serve(f: &mut Fixture) {
    let overlay = Overlay { module: MODULE.into(), entry: ENTRY.into() };
    f.layer.serve_row(SET_SAVE_ROW, overlay).expect("the host serves the row");
}

fn string(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// An integer in the runtime's value form, from its decimal.
fn int(digits: &str) -> Value {
    json::object([("int", string(digits))])
}

/// `set.save`'s arguments as the solver enters them: plain integers.
fn save_args(kind: &str, members: &[i64], name: &str) -> Value {
    let members = Value::Array(members.iter().copied().map(Value::Int).collect());
    json::object([("kind", string(kind)), ("members", members), ("name", string(name))])
}

/// `set.save`'s arguments with `replace`.
fn replacing(kind: &str, members: &[i64], name: &str, replace: Value) -> Value {
    let Value::Object(mut fields) = save_args(kind, members, name) else { unreachable!("an object") };
    fields.insert(REPLACE.to_owned(), replace);
    Value::Object(fields)
}

/// The record `set.save`'s entry writes, `{kind, members, name}` in the runtime's value form.
fn entry(kind: &str, members: &[&str], name: &str) -> Value {
    let members = Value::Array(members.iter().copied().map(int).collect());
    json::object([("kind", string(kind)), ("members", members), ("name", string(name))])
}

/// The output of `set.save`'s entry: its one record, under a hand-entered block (FP.17) that C5's reader checks.
fn saved(record: Value) -> JobOutput {
    let value = RecordValue { payload: record, provenance: text(&manual()) };
    JobOutput { value: Value::Null, views: Vec::new(), records: BTreeMap::from([(SET_SAVE_RECORD.to_owned(), value)]) }
}

/// A save from the shell: prepares and sends `set.save` with `args`, which must be admitted, takes its job and finishes
/// it with the entry's `record`, where the host would run the entry. Returns the invocation, the job and the receipt.
fn save(f: &mut Fixture, args: Value, record: Value) -> (Invocation, u64, Receipt) {
    let asked = f.shell.prepare(&f.layer, SET_SAVE_ROW, args).expect("the shell prepares the save");
    let started = f.shell.send(&mut f.layer, asked.clone());
    let Status::Running(job) = started.status else { panic!("the save was not admitted: {:?}", started.status) };
    assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(job));
    let done = f.layer.finish_program_job_with(job, Ok(saved(record))).expect("the job was running");
    (asked, job, done)
}

/// What a refusal may not change: the revision, the records, the pending preview and the journal's bytes.
fn fixed(f: &Fixture, disk: &dyn Fn() -> Box<dyn Disk>) -> String {
    let journal = disk().read(JOURNAL).expect("a readable disk").expect("a journal");
    let pending = f.layer.preview().map(|preview| preview.id);
    format!("{:?} {:?} {pending:?} {}", f.layer.revision(), f.layer.records(), sha256_hex(&journal))
}

/// The durable leg on a disk that `disk` hands out again for each session, with `power_loss` between the sessions.
///
/// A save's completion is one durable action (KN5.23): its receipt, the saved set's record and the workspace revision
/// come back after a power loss and in a new session, and the result reference's digest follows from the record read
/// back. The save is the next undo's target (H-08's undo class `workspace`, LB7.2, since the command table's freeze),
/// and that too survives: an undo of a replacing save gives back the set it replaced. An identical retry answers the
/// stored receipt, and a save under the taken name without `replace: true`, a `replace` that is no boolean and a
/// `workspace.set` of the saved set's record are refused; none of these writes the journal. A save with
/// `replace: true` replaces the set, and the replacement comes back after another power loss.
fn durable_leg(disk: &dyn Fn() -> Box<dyn Disk>, power_loss: &dyn Fn()) {
    // The solver writes a record, an undoable workspace action, and then saves the pieces 5, 3 and 5 as `pair`.
    let mut f = Fixture::create(disk());
    serve(&mut f);
    assert!(completed(&f.write("sheet", &[1, 2], &manual())));
    let write = f.hint(Direction::Undo);
    let before = f.layer.revision().clone();
    let pair = entry("identity", &["5", "3", "5"], "pair");
    let (asked, job, done) = save(&mut f, save_args("identity", &[5, 3, 5], "pair"), pair);

    // One durable action at the next workspace revision: the result reference carries the exact set's digest under
    // the model the job read, and the action writes the saved set's own record, not the entry's.
    let model = before.identity.clone();
    let set = SavedSet { name: "pair".into(), kind: "identity", members: vec![3, 5] };
    let mut moved = before.clone();
    moved.workspace += 1;
    let reference = ResultRef { job: Some(job), digest: set.digest(&model) };
    let completion =
        Completion { kind: CompletionKind::Workspace, revision: moved.clone(), result: Some(reference.clone()) };
    assert_eq!(done.status, Status::Completed(completion));
    assert_eq!(f.layer.revision(), &moved);
    let record = RecordValue { payload: set.payload(&model, &before), provenance: text(&manual()) };
    assert_eq!(f.layer.record("set/pair"), Some(&record));
    assert_eq!(f.layer.record(SET_SAVE_RECORD), None, "the entry's record is not written");

    // The save took the next action order and is undoable: the undo hint targets it, not the write before it.
    let hint = f.hint(Direction::Undo);
    assert_eq!((hint.order, hint.target), (write.order + 1, write.order + 1));
    assert_ne!(hint.target, write.target);
    let kept = f.kept();
    drop(f);
    power_loss();

    // A new session opens the disk, and its host serves the row again. The revision, the record and the undo order are
    // durable, and so is the save's receipt.
    let mut f = Fixture::open(disk());
    serve(&mut f);
    assert_eq!(f.kept(), kept);
    assert_eq!(f.layer.outcome(&done.command, &done.request).as_ref(), Some(&done));
    let stored = f.layer.record("set/pair").expect("the saved set").clone();
    assert_eq!(stored, record);

    // The record read back gives the result reference's digest again: its kind and members under its model.
    let field = |name: &str| stored.payload.get(name).cloned().expect("a field of the saved set");
    let named = json::object([("kind", field("kind")), ("members", field("members")), ("name", field("name"))]);
    let read = SavedSet::from_entry(&named).expect("the saved set's fields");
    assert_eq!(read.digest(field("model").as_str().expect("the model's identity")), reference.digest);

    // The identical save sent again answers the stored receipt, takes no job and writes nothing.
    let unchanged = fixed(&f, disk);
    assert_eq!(f.layer.invoke(&Caller::Shell, asked), done);
    assert!(f.layer.take_program_job().is_none());
    assert_eq!(fixed(&f, disk), unchanged);

    // A save under the taken name without `replace: true`, a `replace` that is no boolean and a `workspace.set` of the
    // saved set's record are refused at admission: no job, and the journal's bytes stay.
    let refusals = [
        (SET_SAVE_ROW, save_args("identity", &[7], "pair"), code::SAVED_SET_EXISTS),
        (SET_SAVE_ROW, replacing("identity", &[7], "pair", Value::Int(1)), code::ARGUMENT_FORM),
        (RECORD_ROW, record_args("set/pair", &[7], &manual()), code::ARGUMENT_FORM),
    ];
    for (row, args, expected) in refusals {
        let receipt = f.shell.invoke(&mut f.layer, row, args).expect("the shell prepares the row");
        assert_eq!(refused(&receipt), Some(expected), "{row}: {:?}", receipt.status);
        assert!(f.layer.take_program_job().is_none(), "{row}");
        assert_eq!(fixed(&f, disk), unchanged, "{row}");
    }

    // A save with `replace: true` replaces the set, and the record's version advances.
    let args = replacing("position", &[7], "pair", Value::Bool(true));
    let (_, _, replaced) = save(&mut f, args, entry("position", &["7"], "pair"));
    assert!(completed(&replaced), "{:?}", replaced.status);
    let current = f.layer.records().get("set/pair").cloned().expect("the saved set");
    let payload = current.value.as_ref().map(|value| &value.payload).expect("a value");
    assert_eq!(current.version, 2);
    assert_eq!(payload.get("kind"), Some(&string("position")));
    assert_eq!(payload.get("members"), Some(&Value::Array(vec![int("7")])));
    let kept = f.kept();
    drop(f);
    power_loss();

    // After another power loss the replacement comes back, and the undo order holds both saves above the write: an
    // undo of the replacing save gives back the first set, the next undo removes it, and only the one after that
    // removes the write.
    let mut f = Fixture::open(disk());
    assert_eq!(f.kept(), kept);
    assert_eq!(f.layer.records().get("set/pair"), Some(&current));
    let undo = |f: &mut Fixture| {
        let hint = f.hint(Direction::Undo);
        let undone = f.step(Direction::Undo, hint);
        assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);
    };
    undo(&mut f);
    assert_eq!(f.layer.record("set/pair"), Some(&record), "the first set is given back");
    undo(&mut f);
    assert_eq!(f.layer.record("set/pair"), None);
    assert_eq!(f.hint(Direction::Undo).target, write.target);
    undo(&mut f);
    assert_eq!(f.layer.record("sheet"), None);
}

/// The durable leg on a memory disk with a power loss between the sessions: only what the store synced survives.
#[test]
fn a_saved_set_survives_a_power_loss() {
    let disk = MemDisk::new();
    durable_leg(&|| Box::new(disk.clone()) as Box<dyn Disk>, &|| disk.crash());
}

/// The durable leg on a directory of the file system that a new session opens.
#[test]
fn a_saved_set_survives_a_new_session_on_disk() {
    let dir = TempDir::new("saved-sets");
    let path = dir.path().to_path_buf();
    durable_leg(&|| Box::new(FsDisk::new(&path).expect("a store directory")) as Box<dyn Disk>, &|| {});
}

/// Two saves of one name admitted before either completes, on a memory disk: the first writes the set, and the second,
/// without `replace: true`, fails at its completion with `KN5:input.saved_set_exists` and leaves the journal's bytes as
/// they were. After a power loss the record and the receipt are the first save's.
#[test]
fn a_name_taken_meanwhile_fails_the_later_save_and_journals_nothing() {
    let disk = MemDisk::new();
    let handed = || Box::new(disk.clone()) as Box<dyn Disk>;
    let mut f = Fixture::create(handed());
    serve(&mut f);
    let first = f.shell.invoke(&mut f.layer, SET_SAVE_ROW, save_args("identity", &[3], "pair")).expect("prepared");
    let Status::Running(one) = first.status else { panic!("{:?}", first.status) };
    assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(one));
    let second = f.shell.invoke(&mut f.layer, SET_SAVE_ROW, save_args("identity", &[5], "pair")).expect("prepared");
    let Status::Running(two) = second.status else { panic!("{:?}", second.status) };
    let done = f.layer.finish_program_job_with(one, Ok(saved(entry("identity", &["3"], "pair")))).expect("running");
    assert!(completed(&done), "{:?}", done.status);
    assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(two));
    let unchanged = fixed(&f, &handed);
    let late = f.layer.finish_program_job_with(two, Ok(saved(entry("identity", &["5"], "pair")))).expect("running");
    assert_eq!(refused(&late), Some(code::SAVED_SET_EXISTS));
    assert_eq!(fixed(&f, &handed), unchanged);

    let kept = f.kept();
    drop(f);
    disk.crash();
    let f = Fixture::open(handed());
    assert_eq!(f.kept(), kept);
    let record = f.layer.records().get("set/pair").expect("the saved set");
    let members = record.value.as_ref().and_then(|value| value.payload.get("members"));
    assert_eq!((record.version, members), (1, Some(&Value::Array(vec![int("3")]))));
    assert_eq!(f.layer.outcome(&done.command, &done.request).as_ref(), Some(&done));
}
