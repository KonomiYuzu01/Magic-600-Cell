//! Saved sets (draft; U1-057, U4-037, PS1.28): `set.save`'s completion writes the saved set's own record
//! `set/<name>`, and its result reference carries the exact set's digest. The frozen text is silent on the record's
//! form, the digest's bytes and replacement, so these tests follow the draft rulings recorded in `src/sets.rs`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, saved_set_record, CommandLayer, Completion, CompletionKind, Error, JobOutput, Kind, MemoryStore, Overlay,
    Provenance, Receipt, RecordValue, RecordWrite, ResultRef, SavedSet, Secret, Shell, Status, Table, RECORD_ROW,
    SET_SAVE_RECORD, WORKSHEET_SAVE_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

const SAVE: &str = "set.save";
/// A language-defined workspace row the tests register: its completion writes the output's records as they are.
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

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// An integer in the runtime's value form, from its decimal.
fn int(digits: &str) -> Value {
    json::object([("int", text(digits))])
}

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/representations".into(), entry: entry.into() }
}

/// `set.save`'s arguments as the solver enters them: plain integers.
fn save_args(kind: &str, members: &[i64], name: &str) -> Value {
    let members = Value::Array(members.iter().map(|&n| Value::Int(n)).collect());
    json::object([("kind", text(kind)), ("members", members), ("name", text(name))])
}

/// `set.save`'s arguments with `replace`.
fn replacing(kind: &str, members: &[i64], name: &str, replace: Value) -> Value {
    let Value::Object(mut fields) = save_args(kind, members, name) else { unreachable!("an object") };
    fields.insert("replace".into(), replace);
    Value::Object(fields)
}

/// The record `set.save`'s entry writes, `{kind, members, name}` in the runtime's value form.
fn entry(kind: &str, members: &[&str], name: &str) -> Value {
    let members = Value::Array(members.iter().map(|digits| int(digits)).collect());
    json::object([("kind", text(kind)), ("members", members), ("name", text(name))])
}

/// An output whose records are `records`, each with the provenance block `block`.
fn output(records: Vec<(&str, Value)>) -> JobOutput {
    let value = |payload: Value| RecordValue { payload, provenance: "block".into() };
    JobOutput {
        value: Value::Null,
        views: Vec::new(),
        records: records.into_iter().map(|(name, payload)| (name.to_owned(), value(payload))).collect(),
    }
}

/// The output of `set.save`'s entry: its one record.
fn saved(record: Value) -> JobOutput {
    output(vec![(SET_SAVE_RECORD, record)])
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

/// The host's provenance reader as the tests stand it in: it accepts every block.
struct Reader;

impl Provenance for Reader {
    fn check(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
}

/// A session at the solved state that protects nothing, on H-08's table with the layer's drafts, with `set.save`
/// served, a workspace overlay registered and the test reader.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.set_provenance(Box::new(Reader));
    serve(&mut layer);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell") }
}

/// What a host does at every open: serves its rows and registers its overlays again.
fn serve(layer: &mut CommandLayer) {
    layer.serve_row(SAVE, overlay("save_set")).unwrap();
    layer.register_overlay(STORE, Kind::Workspace, &[], overlay("store")).unwrap();
}

impl Fixture {
    fn invoke(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    /// Invokes `row`, takes its job and returns the job's ID.
    fn start(&mut self, row: &str, args: Value) -> u64 {
        let started = self.invoke(row, args);
        let Status::Running(job) = started.status else { panic!("{row}: {:?}", started.status) };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job), "{row}");
        job
    }

    /// Saves `members` of `kind` under `name` through the command route, the entry writing them as given.
    fn save(&mut self, args: Value, record: Value) -> Receipt {
        let job = self.start(SAVE, args);
        self.layer.finish_program_job_with(job, Ok(saved(record))).unwrap()
    }

    /// What a refused or failed save may not change: the revision, the records and the durable log.
    fn fixed(&self) -> (String, String, usize) {
        let (revision, records) = (self.layer.revision(), self.layer.records());
        (format!("{revision:?}"), format!("{records:?}"), self.layer.store().log.len())
    }
}

/// A save writes the saved set's own record `set/pair` in its completion's one durable action, and not the entry's
/// record `set.save`. The record holds the kind, the members ascending and distinct, the model's identity, the name and
/// the capture revision with its counters in value form, under the entry's provenance block. The result reference
/// carries the exact set's digest, the journal keeps the write with that digest, and a reopen keeps the record.
#[test]
fn a_save_writes_the_saved_set_and_carries_its_exact_digest() {
    let mut f = fixture();
    let before = f.layer.revision().clone();
    let job = f.start(SAVE, save_args("identity", &[5, 3, 5], "pair"));
    let done = f.layer.finish_program_job_with(job, Ok(saved(entry("identity", &["5", "3", "5"], "pair")))).unwrap();

    let set = SavedSet { name: "pair".into(), kind: "identity", members: vec![3, 5] };
    let model = before.identity.clone();
    let mut moved = before.clone();
    moved.workspace += 1;
    let reference = ResultRef { job: Some(job), digest: set.digest(&model) };
    let expected = Completion { kind: CompletionKind::Workspace, revision: moved, result: Some(reference) };
    assert_eq!(done.status, Status::Completed(expected));

    let record = f.layer.record("set/pair").expect("the saved set's record").clone();
    assert_eq!(record.provenance, "block");
    let payload = &record.payload;
    assert_eq!(payload.as_object().map(|fields| fields.len()), Some(5));
    assert_eq!(payload.get("kind"), Some(&text("identity")));
    assert_eq!(payload.get("members"), Some(&Value::Array(vec![int("3"), int("5")])));
    assert_eq!(payload.get("model"), Some(&text(&model)));
    assert_eq!(payload.get("name"), Some(&text("pair")));
    let revision = payload.get("revision").expect("the capture revision");
    assert_eq!(revision.as_object().map(|fields| fields.len()), Some(9));
    assert_eq!(revision.get("session"), Some(&text(&before.session)));
    assert_eq!(revision.get("identity"), Some(&text(&model)));
    assert_eq!(revision.get("digest"), Some(&text(&before.digest)));
    assert_eq!(revision.get("state"), Some(&int(&before.state.to_string())));
    assert_eq!(revision.get("workspace"), Some(&int(&before.workspace.to_string())));
    assert_eq!(payload, &set.payload(&model, &before));
    assert!(f.layer.record(SET_SAVE_RECORD).is_none(), "the entry's record is not written");

    let last = f.layer.store().log.last().expect("a durable action").clone();
    let write = RecordWrite { name: saved_set_record("pair"), value: Some(record.clone()) };
    assert_eq!((last.records, last.result), (vec![write], Some(set.digest(&model))));

    let store = f.layer.store().clone();
    let mut g = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    serve(&mut g);
    assert_eq!(g.record("set/pair"), Some(&record));
}

/// The exact set's bytes are the runtime's own encoding of the value `{kind, members, model}` (FP.5): sorted keys,
/// compact separators, integers as `{"int": "n"}` and one terminal line feed, also for text the runtime escapes. Its
/// digest is their SHA-256. It does not depend on the name or on the members' order and repeats, and it changes with
/// the kind, the members and the model.
#[test]
fn the_exact_digest_is_the_sha256_of_the_runtimes_bytes() {
    use c600_run::value::Value as Run;
    let runtime = |kind: &str, members: &[u64], model: &str| {
        let members = Run::list(members.iter().map(|&n| Run::int(n)).collect());
        let value = Run::record(vec![("kind", Run::text(kind)), ("members", members), ("model", Run::text(model))]);
        let mut out = Vec::new();
        c600_run::encode::encode(&value, &mut out).expect("a canonical value");
        out
    };
    let set = SavedSet { name: "pair".into(), kind: "identity", members: vec![3, 5] };
    let literal = br#"{"kind":"identity","members":[{"int":"3"},{"int":"5"}],"model":"m0"}"#;
    assert_eq!(set.bytes("m0"), [&literal[..], b"\n"].concat());
    for model in ["m0", "\u{e9}\u{1}\u{8}\t\n\u{c}\r\"\\\u{1f}\u{7f}", ""] {
        assert_eq!(set.bytes(model), runtime("identity", &[3, 5], model), "{model:?}");
        assert_eq!(set.digest(model), c600_digest::sha256_hex(&set.bytes(model)), "{model:?}");
    }
    let empty = SavedSet { name: "none".into(), kind: "position", members: Vec::new() };
    assert_eq!(empty.bytes("m0"), runtime("position", &[], "m0"));

    let digest = |record: Value, model: &str| SavedSet::from_entry(&record).unwrap().digest(model);
    let pair = digest(entry("identity", &["3", "5"], "pair"), "m0");
    assert_eq!(pair, set.digest("m0"));
    assert_eq!(digest(entry("identity", &["5", "3", "3"], "other"), "m0"), pair);
    assert_ne!(digest(entry("position", &["3", "5"], "pair"), "m0"), pair);
    assert_ne!(digest(entry("identity", &["3"], "pair"), "m0"), pair);
    assert_ne!(digest(entry("identity", &["3", "5"], "pair"), "m1"), pair);
}

/// The entry's record is read in the runtime's value form: members canonical decimals below the kernel's `PIECES`,
/// sorted and deduplicated, an empty list passing. Any other form is `KN2:input.named_object_invalid`.
#[test]
fn an_entry_record_is_read_in_the_runtimes_value_form() {
    let last = (c600_kernel::PIECES - 1).to_string();
    let read = SavedSet::from_entry(&entry("position", &["5", "0", &last, "5"], "a\u{1}b")).unwrap();
    let members = vec![0, 5, u64::try_from(c600_kernel::PIECES - 1).unwrap()];
    assert_eq!(read, SavedSet { name: "a\u{1}b".into(), kind: "position", members });
    assert_eq!(SavedSet::from_entry(&entry("identity", &[], "none")).unwrap().members, Vec::<u64>::new());

    let beyond = c600_kernel::PIECES.to_string();
    let mut extra = entry("identity", &["3"], "pair");
    if let Value::Object(fields) = &mut extra {
        fields.insert("model".into(), text("m0"));
    }
    let record =
        |kind: Value, members: Value, name: Value| json::object([("kind", kind), ("members", members), ("name", name)]);
    let plain = record(text("identity"), Value::Array(vec![Value::Int(3)]), text("pair"));
    let nameless = record(text("identity"), Value::Array(Vec::new()), Value::Int(1));
    let malformed = [
        (entry("other", &["3"], "pair"), "another kind"),
        (entry("identity", &[&beyond], "pair"), "a member at PIECES"),
        (entry("identity", &["03"], "pair"), "a leading zero"),
        (entry("identity", &["-3"], "pair"), "a sign"),
        (entry("identity", &["+3"], "pair"), "a plus sign"),
        (entry("identity", &[""], "pair"), "no digits"),
        (entry("identity", &["99999999999999999999999"], "pair"), "beyond u64"),
        (plain, "a plain integer"),
        (nameless, "a name that is no text"),
        (json::object([("kind", text("identity")), ("name", text("pair"))]), "no members"),
        (extra, "an extra field"),
        (Value::Array(Vec::new()), "no object"),
    ];
    for (record, what) in malformed {
        let error = SavedSet::from_entry(&record).expect_err(what);
        assert_eq!(error.code, code::NAMED_OBJECT_INVALID, "{what}");
    }
}

/// A save under a name that holds a saved set is refused at admission with `KN5:input.saved_set_exists`, taking no
/// job and moving nothing, unless it carries `replace: true`; then its completion replaces the set, and the record's
/// version advances. `replace` is a boolean, and only `set.save` takes it: anything else is
/// `KN5:input.argument_form`.
#[test]
fn a_taken_name_is_refused_unless_the_save_replaces_it() {
    let mut f = fixture();
    assert!(completed(&f.save(save_args("identity", &[3, 5], "pair"), entry("identity", &["3", "5"], "pair"))));
    let fixed = f.fixed();
    let refused = [
        (save_args("identity", &[7], "pair"), code::SAVED_SET_EXISTS),
        (replacing("identity", &[7], "pair", Value::Bool(false)), code::SAVED_SET_EXISTS),
        (replacing("identity", &[7], "pair", Value::Int(1)), code::ARGUMENT_FORM),
        (replacing("identity", &[7], "pair", text("true")), code::ARGUMENT_FORM),
    ];
    for (args, expected) in refused {
        let receipt = f.invoke(SAVE, args.clone());
        assert_eq!(code_of(&receipt), expected, "{args:?}");
        assert!(f.layer.take_program_job().is_none(), "{args:?}");
        assert_eq!(f.fixed(), fixed, "{args:?}");
    }
    let other = f.invoke(STORE, json::object([("replace", Value::Bool(true))]));
    assert_eq!(code_of(&other), code::ARGUMENT_FORM);

    let args = replacing("position", &[7], "pair", Value::Bool(true));
    assert!(completed(&f.save(args, entry("position", &["7"], "pair"))));
    let record = f.layer.records().get("set/pair").cloned().expect("the saved set");
    let payload = record.value.expect("a value").payload;
    assert_eq!((record.version, payload.get("kind")), (2, Some(&text("position"))));
    assert_eq!(payload.get("members"), Some(&Value::Array(vec![int("7")])));

    let args = replacing("identity", &[1], "fresh", Value::Bool(true));
    assert!(completed(&f.save(args, entry("identity", &["1"], "fresh"))), "replace on a free name saves");
}

/// Two saves of one name admitted before either completes: the first writes the set, and the second's completion
/// fails with `KN5:input.saved_set_exists` and writes nothing, unless it carries `replace: true`.
#[test]
fn a_name_taken_meanwhile_fails_the_later_completion() {
    for replace in [false, true] {
        let mut f = fixture();
        let first = f.start(SAVE, save_args("identity", &[3], "pair"));
        let args = replacing("identity", &[5], "pair", Value::Bool(replace));
        let started = f.invoke(SAVE, args);
        let Status::Running(second) = started.status else { panic!("{:?}", started.status) };
        let one = f.layer.finish_program_job_with(first, Ok(saved(entry("identity", &["3"], "pair")))).unwrap();
        assert!(completed(&one));
        assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(second));
        let fixed = f.fixed();
        let two = f.layer.finish_program_job_with(second, Ok(saved(entry("identity", &["5"], "pair")))).unwrap();
        let record = f.layer.records().get("set/pair").cloned().expect("the saved set");
        let members = record.value.expect("a value").payload.get("members").cloned();
        if replace {
            assert!(completed(&two));
            assert_eq!((record.version, members), (2, Some(Value::Array(vec![int("5")]))));
        } else {
            assert_eq!(code_of(&two), code::SAVED_SET_EXISTS);
            assert_eq!(f.fixed(), fixed);
            assert_eq!((record.version, members), (1, Some(Value::Array(vec![int("3")]))));
        }
    }
}

/// An entry output that does not name the set the invocation names fails the job with
/// `KN2:input.named_object_invalid` and writes nothing: no record, another record, an extra record, a record the
/// entry's form does not read, and another kind, other members or another name than the invocation's.
#[test]
fn an_output_that_is_not_the_named_set_fails_the_job() {
    let mut f = fixture();
    let fixed = f.fixed();
    let pair = || entry("identity", &["3", "5"], "pair");
    let cases = [
        (output(Vec::new()), "no record"),
        (output(vec![("other", pair())]), "another record"),
        (output(vec![(SET_SAVE_RECORD, pair()), ("other", pair())]), "an extra record"),
        (saved(entry("identity", &["3", "05"], "pair")), "a noncanonical member"),
        (saved(entry("position", &["3", "5"], "pair")), "another kind"),
        (saved(entry("identity", &["3"], "pair")), "other members"),
        (saved(entry("identity", &["3", "5", "7"], "pair")), "more members"),
        (saved(entry("identity", &["3", "5"], "other")), "another name"),
    ];
    for (out, what) in cases {
        let job = f.start(SAVE, save_args("identity", &[5, 3], "pair"));
        let done = f.layer.finish_program_job_with(job, Ok(out)).unwrap();
        let Status::Failed(error) = &done.status else { panic!("{what}: {:?}", done.status) };
        assert_eq!(error.code, code::NAMED_OBJECT_INVALID, "{what}: {error:?}");
        assert_eq!(f.fixed(), fixed, "{what}");
    }
}

/// Names under `set/` are saved sets, which only `set.save`'s completion writes: `workspace.set` and `worksheet.save`
/// refuse them with `KN5:input.argument_form`, and another workspace row's record under `set/` fails its job with
/// `KN2:input.named_object_invalid`; none writes anything. Names that only begin alike are other records.
#[test]
fn only_set_save_writes_under_set() {
    let mut f = fixture();
    let fixed = f.fixed();
    let write = |name: &str| {
        json::object([("payload", Value::Array(Vec::new())), ("provenance", text("block")), ("record", text(name))])
    };
    assert_eq!(code_of(&f.invoke(RECORD_ROW, write("set/pair"))), code::ARGUMENT_FORM);
    let sheet = json::object([("draft", json::object([("record", text("w"))])), ("name", text("set/pair"))]);
    assert_eq!(code_of(&f.invoke(WORKSHEET_SAVE_ROW, sheet)), code::ARGUMENT_FORM);
    let job = f.start(STORE, json::object([]));
    let done = f.layer.finish_program_job_with(job, Ok(output(vec![("set/pair", Value::Null)]))).unwrap();
    assert_eq!(code_of(&done), code::NAMED_OBJECT_INVALID);
    assert_eq!(f.fixed(), fixed);

    for name in ["set", "sets/pair", "set.pair", "Set/pair"] {
        assert!(completed(&f.invoke(RECORD_ROW, write(name))), "{name}");
    }
}
