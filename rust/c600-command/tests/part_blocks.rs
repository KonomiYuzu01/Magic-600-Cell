//! The session's durable home for a word's part blocks (BND.24, BND.85; CT-BND-27; draft): a record write and a
//! workspace row's completion keep the part blocks the host's reader holds for the blocks they write, the journal
//! keeps them with the action, and a reopen gives them back to a reader that holds nothing, so preview admission reads
//! a restored word's recursive parts. Each test names the conformance rows it covers headless with PHS.62 markers.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use c600_command::{
    code, CommandLayer, Direction, Error, Fate, JobOutput, Kind, MemoryStore, Overlay, Provenance, Receipt,
    RecordValue, Secret, Shell, Status, Table, Transaction, TransactionKind, POLICY_ROW, RECORD_ROW,
};
use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const NOTE_ROW: &str = "tool.note";

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

fn letters(letters: &[i64]) -> Value {
    Value::Array(letters.iter().map(|&letter| Value::Int(letter)).collect())
}

/// A block of these tests, standing in for FP.17's: `{"name", "parts", "tool"}`, `parts` the SHA-256 hex of the part
/// blocks' texts (BND.24) and `tool` whether a solver's tool computed the word (BND.84).
fn block(name: &str, tool: bool, parts: &[&str]) -> String {
    let parts = parts.iter().map(|part| text(&sha256_hex(part.as_bytes()))).collect();
    json::object([("name", text(name)), ("parts", Value::Array(parts)), ("tool", Value::Bool(tool))]).canonical()
}

/// A block's name, whether a tool computed it, and its parts' digests.
fn read(block: &[u8]) -> Option<(String, bool, Vec<String>)> {
    let value = json::parse(block).ok()?;
    let parts: Option<Vec<String>> =
        value.get("parts")?.as_array()?.iter().map(|part| part.as_str().map(str::to_owned)).collect();
    let Value::Bool(tool) = value.get("tool")? else { return None };
    Some((value.get("name")?.as_str()?.to_owned(), *tool, parts?))
}

/// The host's reader: the blocks it holds by digest, as a host keeps a run's blocks while it runs. A reader a reopen
/// installs holds none.
#[derive(Clone, Default)]
struct Reader(Arc<Mutex<BTreeMap<String, String>>>);

impl Reader {
    fn holding(blocks: &[&str]) -> Reader {
        let held = blocks.iter().map(|block| (sha256_hex(block.as_bytes()), (*block).to_owned()));
        Reader(Arc::new(Mutex::new(held.collect())))
    }

    /// Walks `block`'s parts, recursively and in listed order, finding each in what the reader holds or in `kept`.
    fn walk(&self, block: &[u8], kept: &BTreeMap<String, String>) -> Result<Vec<String>, Error> {
        let held = self.0.lock().unwrap();
        let mut pending = vec![String::from_utf8_lossy(block).into_owned()];
        let (mut found, mut seen) = (Vec::new(), Vec::new());
        while let Some(current) = pending.pop() {
            let Some((name, tool, parts)) = read(current.as_bytes()) else {
                return Err(Error::new(code::PROVENANCE_INCOMPATIBLE, "the block does not read"));
            };
            if tool {
                return Err(Error::new(code::NOT_GRANTED, format!("{name} is a solver tool's output (BND.84)")));
            }
            for digest in parts.into_iter().rev() {
                if seen.contains(&digest) {
                    continue;
                }
                seen.push(digest.clone());
                let Some(part) = held.get(&digest).or_else(|| kept.get(&digest)) else {
                    let detail = format!("part {digest} has no stored content and reads unknown (BND.21, BND.85)");
                    return Err(Error::new(code::NOT_GRANTED, detail));
                };
                found.push(part.clone());
                pending.push(part.clone());
            }
        }
        Ok(found)
    }
}

impl Provenance for Reader {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        read(block).map(|_| ()).ok_or_else(|| Error::new(code::PROVENANCE_INCOMPATIBLE, "the block does not read"))
    }

    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        self.admit_kept(block, &BTreeMap::new())
    }

    fn parts(&self, block: &[u8]) -> Vec<String> {
        let held = self.0.lock().unwrap().clone();
        let mut found = Vec::new();
        let mut pending = vec![String::from_utf8_lossy(block).into_owned()];
        while let Some(current) = pending.pop() {
            for digest in read(current.as_bytes()).map(|(_, _, parts)| parts).unwrap_or_default() {
                if let Some(part) = held.get(&digest).filter(|part| !found.contains(*part)) {
                    found.push(part.clone());
                    pending.push(part.clone());
                }
            }
        }
        found
    }

    fn admit_kept(&self, block: &[u8], kept: &BTreeMap<String, String>) -> Result<(), Error> {
        self.walk(block, kept).map(|_| ())
    }
}

/// A host reader that implements only `check` and `admit`: the trait's defaults keep no part and ignore what the
/// session keeps.
struct Plain(Reader);

impl Provenance for Plain {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        self.0.check(block)
    }

    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        self.0.admit(block)
    }
}

struct Session {
    layer: CommandLayer,
    shell: Shell,
}

fn equip(mut layer: CommandLayer, reader: Box<dyn Provenance>, prefix: &str) -> Session {
    layer.set_provenance(reader);
    let overlay = Overlay { module: "notes".into(), entry: "note".into() };
    layer.register_overlay(NOTE_ROW, Kind::Workspace, &[], overlay).unwrap();
    Session { layer, shell: Shell::new(Secret::fixed(SECRET), prefix) }
}

fn fresh(reader: Box<dyn Provenance>) -> Session {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    equip(layer, reader, "live")
}

/// The session `store` holds, reopened with `reader`, as a restart: the host's reader keeps nothing across it.
fn reopened(store: MemoryStore, reader: Box<dyn Provenance>) -> Session {
    let table = Table::h08().with_drafts();
    let layer = CommandLayer::open(kernel(), table, Secret::fixed(SECRET), store).expect("the journal reopens");
    equip(layer, reader, "reopened")
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(_))
}

impl Session {
    fn write(&mut self, name: &str, word: &[i64], block: &str) -> Receipt {
        let args = json::object([("payload", letters(word)), ("provenance", text(block)), ("record", text(name))]);
        self.shell.invoke(&mut self.layer, RECORD_ROW, args).unwrap()
    }

    fn preview(&mut self, name: &str) -> Receipt {
        let args = json::object([("draft", json::object([("record", text(name))]))]);
        self.shell.invoke(&mut self.layer, "operation.preview", args).unwrap()
    }

    /// A workspace row's job that writes record `name` with `block` (LP3's C6, its second common need).
    fn job_writes(&mut self, name: &str, block: &str) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, NOTE_ROW, json::object([])).unwrap();
        let Status::Running(job) = receipt.status else { panic!("{receipt:?}") };
        self.layer.take_program_job().expect("the job the press started");
        let value = RecordValue { payload: letters(&[4]), provenance: block.to_owned() };
        let output = JobOutput { value: Value::Null, views: Vec::new(), records: [(name.to_owned(), value)].into() };
        self.layer.finish_program_job_with(job, Ok(output)).expect("a receipt")
    }

    fn undo(&mut self) -> Receipt {
        let hint = self.layer.undo_hint(Direction::Undo).expect("an action to undo");
        let args = json::object([("confirm", Value::Bool(true)), ("hint", hint.to_json())]);
        self.shell.invoke(&mut self.layer, "session.undo", args).unwrap()
    }

    fn last(&self) -> &Transaction {
        self.layer.store().log.last().expect("a durable action")
    }
}

/// The blocks in the order the journal keeps them: by the SHA-256 of their bytes.
fn by_digest(blocks: &[&str]) -> Vec<String> {
    let ordered: BTreeMap<String, String> =
        blocks.iter().map(|block| (sha256_hex(block.as_bytes()), (*block).to_owned())).collect();
    ordered.into_values().collect()
}

fn refusal(receipt: &Receipt) -> &Error {
    match &receipt.status {
        Status::Refused(error) => error,
        other => panic!("not refused: {other:?}"),
    }
}

/// CT-BND-27 (part: a restored workspace keeps a word's recursive parts, and preview admission refuses a restored word
/// whose part two levels down a solver's tool computed with `LG3:capability.not_granted` and installs no preview; not
/// run: the FP.17 blocks and the boundary's own admission, which the host's reader supplies, and a restore on M6's
/// journal, which `c600-session`'s `tests/kn5_command.rs` runs) (BND.24, BND.84, BND.85); KN5.26.
#[test]
fn a_restored_word_keeps_its_recursive_parts_and_a_nested_tool_block_is_refused() {
    let tool = block("tool", true, &[]);
    let built_in = block("built-in", false, &[&tool]);
    let word = block("composed", false, &[&built_in]);

    // The host holds the run's blocks while it writes: the write keeps both parts with the action.
    let mut live = fresh(Box::new(Reader::holding(&[&tool, &built_in])));
    assert!(completed(&live.write("w", &[1, 2], &word)));
    assert_eq!(live.last().blocks, by_digest(&[&tool, &built_in]));
    let refused = refusal(&live.preview("w")).clone();
    assert!(refused.detail.ends_with("(BND.84)"), "{}", refused.detail);

    // A reopen with a reader that holds nothing reads the restored word's parts from the session.
    let mut restored = reopened(live.layer.store().clone(), Box::new(Reader::default()));
    assert_eq!(restored.layer.blocks(), live.layer.blocks());
    assert_eq!(restored.layer.blocks().len(), 2);
    let revision = restored.layer.revision().clone();
    assert_eq!(refusal(&restored.preview("w")), &refused, "the nested tool block, not a missing part");
    assert!(restored.layer.preview().is_none(), "admission installs no preview");
    assert_eq!((restored.layer.run_jobs(), restored.layer.revision()), (0, &revision));

    // Without the home, a host that held no part at the write leaves the restored word with a missing part, which
    // fails closed for that reason instead (BND.21, BND.85).
    let mut bare = fresh(Box::new(Reader::default()));
    assert!(completed(&bare.write("w", &[1, 2], &word)));
    assert!(bare.last().blocks.is_empty());
    let mut restored = reopened(bare.layer.store().clone(), Box::new(Reader::default()));
    let receipt = restored.preview("w");
    let missing = refusal(&receipt);
    assert_eq!(missing.code, code::NOT_GRANTED);
    assert!(missing.detail.ends_with("(BND.21, BND.85)"), "{}", missing.detail);
}

/// CT-BND-27 (part: a restored built-in word whose part is a manual word is admitted; the parts a session already keeps
/// are journaled once, an undo keeps them, and a workspace row's record writes keep theirs) (BND.24, BND.85); KN5.26,
/// KN5.23.
#[test]
fn kept_parts_admit_a_restored_built_in_word_and_stay_for_the_session() {
    let manual = block("manual", false, &[]);
    let built_in = block("built-in", false, &[&manual]);
    let job_part = block("job input", false, &[]);
    let job_block = block("job output", false, &[&job_part]);

    let mut live = fresh(Box::new(Reader::holding(&[&manual, &job_part])));
    assert!(completed(&live.write("b", &[1, 2], &built_in)));
    assert_eq!(live.last().blocks, [manual.clone()]);
    // A second write citing the same part journals nothing new, and an undo of it keeps the session's parts.
    assert!(completed(&live.write("c", &[3], &built_in)));
    assert!(live.last().blocks.is_empty(), "already kept");
    assert!(matches!(live.undo().status, Status::Committed(_)), "an undo answers committed");
    assert_eq!((live.last().kind, live.last().blocks.is_empty()), (TransactionKind::Undo, true));
    assert!(live.layer.blocks().contains_key(&sha256_hex(manual.as_bytes())));
    // A workspace row's completion keeps the parts of the records its job writes.
    assert!(completed(&live.job_writes("j", &job_block)));
    assert_eq!((live.last().kind, live.last().blocks.clone()), (TransactionKind::Workspace, vec![job_part.clone()]));

    let mut restored = reopened(live.layer.store().clone(), Box::new(Reader::default()));
    assert_eq!(restored.layer.blocks(), live.layer.blocks());
    assert_eq!(restored.layer.records(), live.layer.records());
    let receipt = restored.preview("b");
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
    assert_eq!(restored.layer.run_jobs(), 1);
    assert!(restored.layer.preview().is_some(), "the restored word's preview is built");
    let receipt = restored.preview("j");
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
}

/// The trait's defaults (draft): a host reader that implements only `check` and `admit` keeps no part, and admission
/// asks it alone, so a word with a part it does not hold fails closed as before (BND.21, BND.85).
#[test]
fn a_reader_with_the_defaults_keeps_no_part() {
    let manual = block("manual", false, &[]);
    let built_in = block("built-in", false, &[&manual]);
    let mut live = fresh(Box::new(Plain(Reader::holding(&[&manual]))));
    assert!(completed(&live.write("b", &[1], &built_in)));
    assert!(live.last().blocks.is_empty() && live.layer.blocks().is_empty());
    assert!(matches!(live.preview("b").status, Status::Running(_)), "the host's reader holds the part itself");
    let mut restored = reopened(live.layer.store().clone(), Box::new(Plain(Reader::default())));
    assert_eq!(refusal(&restored.preview("b")).code, code::NOT_GRANTED);
}

/// The journal form of the kept blocks (draft; KN5.23): `blocks` round-trips on a record write and a workspace
/// completion, and a list that is empty, not of texts, out of digest order, on another kind of action or on a
/// completion that writes no record does not read, so the history stops there.
#[test]
fn the_journal_reads_kept_blocks_strictly() {
    let (one, two) = (block("one", false, &[]), block("two", false, &[]));
    let word = block("word", false, &[&one, &two]);
    let mut live = fresh(Box::new(Reader::holding(&[&one, &two])));
    assert!(completed(&live.write("w", &[1], &word)));
    let args = json::object([("confirm", Value::Bool(true)), ("policy", text("strict"))]);
    assert!(completed(&live.shell.invoke(&mut live.layer, POLICY_ROW, args).unwrap()));
    assert!(completed(&live.job_writes("j", &word)));
    let log = live.layer.store().log.clone();
    let (write, policy) = (&log[0], &log[1]);
    assert_eq!(log[2].kind, TransactionKind::Workspace);
    let mut quiet = log[2].to_json();
    if let Value::Object(fields) = &mut quiet {
        fields.remove("records");
    }
    let quiet = Transaction::from_json(&quiet).expect("a workspace completion that writes nothing");
    assert_eq!(write.blocks, by_digest(&[&one, &two]));
    assert_eq!(Transaction::from_json(&write.to_json()).as_ref(), Some(write));
    assert_eq!(Transaction::from_json(&policy.to_json()).as_ref(), Some(policy));

    let with = |base: &Transaction, blocks: Value| {
        let mut value = base.to_json();
        if let Value::Object(fields) = &mut value {
            fields.insert("blocks".to_owned(), blocks);
        }
        Transaction::from_json(&value)
    };
    let reversed: Vec<Value> = write.blocks.iter().rev().map(|block| text(block)).collect();
    for (what, base, blocks) in [
        ("empty", write, Value::Array(Vec::new())),
        ("not texts", write, Value::Array(vec![Value::Int(1)])),
        ("out of digest order", write, Value::Array(reversed)),
        ("on a policy change", policy, Value::Array(vec![text(&one)])),
        ("on a completion that writes nothing", &quiet, Value::Array(vec![text(&one)])),
    ] {
        assert_eq!(with(base, blocks), None, "{what}");
    }
}

/// The kept parts follow the fate of the write that keeps them (KN5.23; draft): a rolled-back write keeps none, a
/// write whose fate is unknown keeps none until a restart settles it, and then keeps them exactly when the store holds
/// the write. A journal that keeps a part block twice does not replay, so the history stops there.
#[test]
fn kept_parts_follow_the_fate_of_their_write() {
    let manual = block("manual", false, &[]);
    let built_in = block("built-in", false, &[&manual]);

    let mut live = fresh(Box::new(Reader::holding(&[&manual])));
    live.layer.store_mut().inject.push_back(Fate::RolledBack(Error::new("PS1:io.injected", "injected rollback")));
    let receipt = live.write("b", &[1], &built_in);
    assert!(matches!(receipt.status, Status::Failed(_)), "{:?}", receipt.status);
    assert!(live.layer.blocks().is_empty() && live.layer.store().log.is_empty());

    for written in [false, true] {
        let mut live = fresh(Box::new(Reader::holding(&[&manual])));
        let store = live.layer.store_mut();
        store.inject.push_back(Fate::Unknown);
        store.unknown_writes = written;
        let receipt = live.write("b", &[1], &built_in);
        assert!(matches!(receipt.status, Status::UnknownOutcome), "{:?}", receipt.status);
        assert!(live.layer.blocks().is_empty(), "an unsettled write keeps no part");
        live.layer.restart(Secret::fixed([8; 32]));
        assert!(live.layer.recovery().is_none(), "{:?}", live.layer.recovery());
        assert_eq!(live.layer.blocks().contains_key(&sha256_hex(manual.as_bytes())), written, "written: {written}");
        assert_eq!(live.layer.blocks().len(), usize::from(written));
    }

    // A second action that journals again a part block the first one keeps.
    let mut live = fresh(Box::new(Reader::holding(&[&manual])));
    assert!(completed(&live.write("a", &[1], &built_in)));
    assert!(completed(&live.write("b", &[2], &built_in)));
    let mut store = live.layer.store().clone();
    assert!(store.log[1].blocks.is_empty(), "a part the session keeps is journaled once");
    store.log[1].blocks = store.log[0].blocks.clone();
    let layer = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    let recovery = layer.recovery().expect("the history stops at the second action");
    assert!(recovery.detail.contains("keeps already"), "{}", recovery.detail);
    assert!(layer.record("a").is_some() && layer.record("b").is_none());
}
