//! FP.9 on c600-command's KN5 layer over KN4's journal ([`JournalStore`]): each record value an action writes is a
//! content-addressed record object that the action's frame cites, so a record value up to FP.9's `record_decoded`
//! bound (64 MiB) is written and read back, and one byte more is refused (PS1.11; draft format
//! `c600-command-journal-draft-1`). The frame's other fields, a record write's kept part blocks among them, stay in the
//! frame under its 16 MiB draft bound. The object is written and synced before its frame, as the start state's is; a
//! missing or altered object ends the history in read-only recovery (KN5.23). Each test names the rows it covers
//! headless, with PHS.62 markers.

mod common;

use c600_command::{
    code, CommandLayer, Completion, CompletionKind, Direction, Error, Provenance, Receipt, RecordValue, Secret, Shell,
    Status, Table, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_session::disk::{Fault, MemDisk};
use c600_session::journal::{self, Tail};
use c600_session::persist::codes::SIZE;
use c600_session::JournalStore;

const SECRET: [u8; 32] = [7; 32];
const BLOCK: &str = "block";
/// FP.9's `record_decoded` bound, the lower of its two record rows.
const BOUND: usize = 67_108_864;

/// A host reader that takes every block.
struct Reader;

impl Provenance for Reader {
    fn check(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
    fn admit(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

struct Fixture {
    layer: CommandLayer<JournalStore>,
    shell: Shell,
}

fn table() -> Table {
    Table::h08().with_drafts()
}

/// A new session at the solved state on an empty memory disk.
fn create(disk: &MemDisk) -> Fixture {
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let secret = Secret::fixed(SECRET);
    let mut layer = CommandLayer::new(k.clone(), table(), secret, store, "s1", k.solved(), ProtectedSet::none());
    layer.set_provenance(Box::new(Reader));
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell") }
}

/// The session the disk holds, reopened under a new layer and shell, as a restart.
fn reopen(disk: &MemDisk) -> Fixture {
    disk.end_holder();
    let store = JournalStore::open(Box::new(disk.clone())).expect("the journal opens");
    let mut layer = match CommandLayer::open(common::kernel(), table(), Secret::fixed([9; 32]), store) {
        Ok(layer) => layer,
        Err(error) => panic!("the session does not reopen: {error:?}"),
    };
    layer.set_provenance(Box::new(Reader));
    Fixture { layer, shell: Shell::new(Secret::fixed([9; 32]), "shell") }
}

impl Fixture {
    /// Writes `value` to the record `name` through `workspace.set`.
    fn write(&mut self, name: &str, value: &RecordValue) -> Receipt {
        let args = json::object([
            ("payload", value.payload.clone()),
            ("provenance", Value::String(value.provenance.clone())),
            ("record", Value::String(name.into())),
        ]);
        self.shell.invoke(&mut self.layer, RECORD_ROW, args).unwrap()
    }

    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

/// A record value whose canonical bytes, its object's bytes, are `len` long: a payload of `a`s.
fn value_of(len: usize) -> RecordValue {
    let empty = RecordValue { payload: Value::String(String::new()), provenance: BLOCK.into() };
    let overhead = empty.to_json().canonical().len();
    RecordValue { payload: Value::String("a".repeat(len - overhead)), provenance: BLOCK.into() }
}

fn small(text: &str) -> RecordValue {
    RecordValue { payload: Value::String(text.into()), provenance: BLOCK.into() }
}

fn object(disk: &MemDisk, value: &RecordValue) -> Option<Vec<u8>> {
    disk.durable(&format!("objects/{}", value.digest()))
}

/// The journal's records, in order.
fn records(disk: &MemDisk) -> Vec<Value> {
    let bytes = disk.durable("journal").expect("the disk holds a journal");
    let scan = journal::scan(&bytes);
    assert_eq!(scan.tail, Tail::Clean);
    scan.payloads.iter().map(|payload| json::parse(payload).expect("a record")).collect()
}

/// The transactions of the journal's action records, in order.
fn actions(disk: &MemDisk) -> Vec<Value> {
    let actions =
        records(disk).into_iter().filter(|r| r.get("record").and_then(Value::as_str) == Some("command_action"));
    actions.map(|record| record.get("transaction").expect("a transaction").clone()).collect()
}

/// The value field of a transaction's `record` write, as the frame holds it.
fn cited(transaction: &Value) -> &Value {
    transaction.get("record").and_then(|record| record.get("value")).expect("a record write")
}

fn reference(value: &RecordValue) -> Value {
    json::object([("object", Value::String(value.digest()))])
}

/// Replaces the journal with its records, the last action's record value replaced by `value`.
fn rewrite_last_value(disk: &MemDisk, value: Value) {
    let mut records = records(disk);
    let last = records.iter_mut().rev().find(|r| r.get("record").and_then(Value::as_str) == Some("command_action"));
    let Some(Value::Object(fields)) = last else { panic!("no action record") };
    let Some(Value::Object(transaction)) = fields.get_mut("transaction") else { panic!("no transaction") };
    let Some(Value::Object(write)) = transaction.get_mut("record") else { panic!("no record write") };
    write.insert("value".into(), value);
    let mut bytes = Vec::new();
    for record in &records {
        bytes.extend(journal::frame(record.canonical().as_bytes()));
    }
    disk.overwrite("journal", &bytes);
}

/// FP.9 (`record_stored`, `record_decoded`; read = write), PS1.11, KN5.23: a record whose object is exactly FP.9's
/// bound is written as an object that its frame cites by content digest, and a reopen reads it back; one byte more is
/// refused with `PS1:resource_limit.persistence_size` and writes nothing, also when the disk already holds an object
/// under its digest with its bytes, so a value already stored is still charged.
#[test]
fn a_record_at_fp9s_bound_is_an_object_that_a_reopen_reads() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    let big = value_of(BOUND);
    assert!(completed(&f.write("big", &big)));
    let stored = object(&disk, &big).expect("the record's object");
    assert_eq!((stored.len(), stored.as_slice()), (BOUND, big.to_json().canonical().as_bytes()));
    let written = actions(&disk);
    assert_eq!(written.len(), 1);
    assert_eq!(cited(&written[0]), &reference(&big), "the frame cites the object by the value's content digest");
    let frame = journal::scan(&disk.durable("journal").unwrap()).payloads.last().unwrap().len();
    assert!(frame < 4096, "the frame holds a reference, not the value: {frame} bytes");

    let over = value_of(BOUND + 1);
    let journal = disk.durable("journal");
    let refused = f.write("over", &over);
    assert_eq!(code_of(&refused), SIZE);
    assert_eq!((object(&disk, &over), disk.durable("journal")), (None, journal.clone()), "the refusal wrote");
    // The same value again, its object put on the disk by hand: the charge does not depend on what the disk holds.
    disk.overwrite(&format!("objects/{}", over.digest()), over.to_json().canonical().as_bytes());
    assert_eq!(code_of(&f.write("over", &over)), SIZE);
    assert_eq!(disk.durable("journal"), journal, "the refusal wrote");
    assert_eq!(f.layer.record("over"), None);
    let kept = (f.layer.records().clone(), f.layer.revision().workspace);
    drop(f);

    let g = reopen(&disk);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(g.layer.record("big"), Some(&big));
    assert_eq!((g.layer.records().clone(), g.layer.revision().workspace), kept);
}

/// KN5.23, ONE.70 (part: a power loss after a record object is written and synced and before the frame that cites it
/// leaves the object and no frame; the reopen keeps the previous record, ignores the object, and writes only the epoch
/// record every open writes; a later write of the same value finds the object, syncs it and writes its frame).
#[test]
fn a_power_loss_between_an_object_and_its_frame_keeps_the_previous_record() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    let (a, b) = (small("first"), small("second"));
    assert!(completed(&f.write("w", &a)));
    let before = disk.durable("journal").unwrap();
    // Writes `next` and `next + 1` create and sync B's object; the frame's append, `next + 2`, and every later write
    // fail.
    let next = disk.writes();
    disk.fail_from(next + 2);
    let lost = f.write("w", &b);
    assert_eq!(lost.status, Status::UnknownOutcome);
    disk.crash();
    drop(f);
    assert_eq!(object(&disk, &b), Some(b.to_json().canonical().into_bytes()), "the object is durable");
    assert_eq!((disk.durable("journal").unwrap(), actions(&disk).len()), (before.clone(), 1), "and its frame is not");

    let mut g = reopen(&disk);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!((g.layer.record("w"), g.layer.records()["w"].version), (Some(&a), 1));
    let after = disk.durable("journal").unwrap();
    assert!(after.starts_with(&before), "the reopen changed the journal it read");
    let added = records(&disk);
    assert_eq!(added.len(), journal::scan(&before).payloads.len() + 1);
    assert_eq!(added.last().unwrap().get("record").and_then(Value::as_str), Some("command_epoch"));
    assert_eq!(object(&disk, &b), Some(b.to_json().canonical().into_bytes()), "the reopen changed the object");

    let writes = disk.writes();
    assert!(completed(&g.write("w", &b)));
    assert_eq!(disk.writes(), writes + 3, "the object's sync, then the frame's append and sync");
    let kept = g.layer.records().clone();
    drop(g);
    let h = reopen(&disk);
    assert!(h.layer.recovery().is_none(), "{:?}", h.layer.recovery());
    assert_eq!((h.layer.record("w"), h.layer.records()), (Some(&b), &kept));
}

/// KN5.23, ONE.70 (part: an action whose object reached the disk but whose object sync failed is rolled back; a
/// retry of the same value syncs the object before the frame that cites it, so a power loss after the retry keeps the
/// record).
#[test]
fn an_object_whose_sync_failed_is_synced_before_a_retry_cites_it() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    let a = small("unsynced");
    // Write `next` creates the object and `next + 1` syncs it; that sync fails.
    let next = disk.writes();
    disk.fail_write(next + 1, Fault::Error);
    let failed = f.write("w", &a);
    assert!(matches!(failed.status, Status::Failed(_)), "{:?}", failed.status);
    assert_eq!((object(&disk, &a), actions(&disk).len()), (None, 0), "the object and the frame are not durable");
    assert!(completed(&f.write("w", &a)));
    disk.crash();
    drop(f);
    assert_eq!(object(&disk, &a), Some(a.to_json().canonical().into_bytes()), "the retry synced the object");
    let g = reopen(&disk);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(g.layer.record("w"), Some(&a));
}

/// KN5.23 (part: a record object that is missing, altered, or another value's bytes ends the history before the
/// action that cites it, and the layer opens in read-only recovery, writes nothing and refuses a write); the control
/// case holds the value inline, as journals written before record objects do, and reads.
#[test]
fn a_missing_or_altered_object_opens_in_read_only_recovery() {
    let (a, b) = (small("first"), small("second"));
    for case in ["missing", "altered", "another value", "inline"] {
        let disk = MemDisk::new();
        let mut f = create(&disk);
        assert!(completed(&f.write("w", &a)), "{case}");
        assert!(completed(&f.write("v", &b)), "{case}");
        drop(f);
        let name = format!("objects/{}", b.digest());
        match case {
            "missing" => rewrite_last_value(&disk, json::object([("object", Value::String("0".repeat(64)))])),
            "altered" => {
                let mut bytes = disk.durable(&name).unwrap();
                let at = bytes.len() / 2;
                bytes[at] ^= 0x01;
                disk.overwrite(&name, &bytes);
            }
            "another value" => disk.overwrite(&name, a.to_json().canonical().as_bytes()),
            _ => rewrite_last_value(&disk, b.to_json()),
        }
        let journal = disk.durable("journal");
        let mut g = reopen(&disk);
        if case == "inline" {
            assert!(g.layer.recovery().is_none(), "{case}: {:?}", g.layer.recovery());
            assert_eq!((g.layer.record("w"), g.layer.record("v")), (Some(&a), Some(&b)), "{case}");
            continue;
        }
        assert_eq!(g.layer.recovery().map(|e| e.code), Some(code::READ_ONLY_RECOVERY), "{case}");
        assert_eq!((g.layer.record("w"), g.layer.record("v")), (Some(&a), None), "{case}");
        assert_eq!(code_of(&g.write("u", &a)), code::READ_ONLY_RECOVERY, "{case}");
        assert_eq!(disk.durable("journal"), journal, "{case}: read-only recovery wrote");
    }
}

/// FP.9, KN5.23 (part: a value that the disk already holds as an object, written under another name or by an undo and
/// a redo, is cited by the same digest and written once; every action still reopens to the same records).
#[test]
fn a_value_already_stored_is_cited_again_and_written_once() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    let a = small("shared");
    assert!(completed(&f.write("w", &a)));
    let writes = disk.writes();
    assert!(completed(&f.write("v", &a)));
    assert_eq!(disk.writes(), writes + 2, "only the frame's append and sync");
    assert!(matches!(f.step(Direction::Undo).status, Status::Committed(_)));
    assert_eq!(f.layer.record("v"), None);
    let writes = disk.writes();
    assert!(matches!(f.step(Direction::Redo).status, Status::Committed(_)));
    assert_eq!(disk.writes(), writes + 2, "only the frame's append and sync");
    let written = actions(&disk);
    assert_eq!(written.len(), 4);
    let cites: Vec<&Value> = written.iter().map(cited).collect();
    assert_eq!(cites, [&reference(&a), &reference(&a), &Value::Null, &reference(&a)]);
    let kept = f.layer.records().clone();
    drop(f);
    let g = reopen(&disk);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(g.layer.records(), &kept);
}

/// CT-PS1-02 (part: the store keeps a 6,775-digit integer byte for byte through a reopen; not run: an FP.6 reader in
/// the store, which keeps payloads as canonical bytes and has none, and the keep-the-source rule after a digit
/// refusal).
#[test]
fn a_record_with_a_big_integer_reads_back_byte_for_byte() {
    let lexeme: String = std::iter::once('1').chain("0123456789".chars().cycle().take(6774)).collect();
    assert_eq!(lexeme.len(), 6775);
    let value = RecordValue { payload: Value::Number(lexeme.clone()), provenance: BLOCK.into() };
    let disk = MemDisk::new();
    let mut f = create(&disk);
    assert!(completed(&f.write("n", &value)));
    drop(f);
    let g = reopen(&disk);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    let read = g.layer.record("n").expect("the record");
    assert_eq!(read.payload.canonical(), lexeme, "the payload's canonical bytes are the lexeme");
    assert_eq!(read, &value);
}

/// CT-LG3-09 (part: R-12 at the session store: an overwrite of a record that carries a provenance block with one that
/// carries another block, while the disk fails the write, is refused and leaves the record's payload and block as they
/// were, also after a power loss and a reopen; not run: the session's assistance provenance record, which no crate
/// keeps).
#[test]
fn a_failed_write_of_a_record_with_another_block_keeps_the_record_and_its_block() {
    for offset in [0, 1] {
        let disk = MemDisk::new();
        let mut f = create(&disk);
        let kept = RecordValue { payload: Value::String("kept".into()), provenance: "block-a".into() };
        let other = RecordValue { payload: Value::String("other".into()), provenance: "block-b".into() };
        assert!(completed(&f.write("w", &kept)));
        let (journal, records) = (disk.durable("journal"), f.layer.records().clone());
        // Write `next` creates the other record's object and `next + 1` syncs it; the write at `offset` fails.
        let next = disk.writes();
        disk.fail_write(next + offset, Fault::Error);
        let failed = f.write("w", &other);
        assert!(matches!(failed.status, Status::Failed(_)), "{offset}: {:?}", failed.status);
        assert_eq!(f.layer.record("w"), Some(&kept), "{offset}: payload and block");
        assert_eq!(f.layer.records(), &records, "{offset}");
        assert_eq!(disk.durable("journal"), journal, "{offset}: the refusal wrote no frame");
        disk.crash();
        drop(f);
        let g = reopen(&disk);
        assert!(g.layer.recovery().is_none(), "{offset}: {:?}", g.layer.recovery());
        assert_eq!(g.layer.record("w"), Some(&kept), "{offset}: after a reopen");
        assert_eq!(g.layer.records()["w"].version, 1, "{offset}");
    }
}
