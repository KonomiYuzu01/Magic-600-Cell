//! CT-RTE-08, the durable leg of BM2's persisted-word case (BND.22, BND.84, BND.85; LG5.1, LG5.6; CT-LG3-07's
//! saved-word half). `c600-module`'s `tests/ct_rte_08.rs` runs the case on `c600-run`'s in-memory store. Here the word
//! goes through M6's session store: an approved own tool stores it, the host persists it with its block through
//! `workspace.set`, the session ends and a new one opens the same disk, and the word read back keeps the block, names
//! the module the approval binds, and is refused at preview admission.
//!
//! Waits for LP2's KN5 implementation (crate documentation): the layer's own `operation.preview` of the record,
//! which KN5.26 has read the word's block and parts through the BND.84 adapter in a solving session.
//! `admit_preview` on the read-back block stands in for it.

mod common;

use c600_check::{Grant, Policy};
use c600_digest::sha256;
use c600_module::{approve, load, resolve, verify, Approval, Closure, Entry, Files, Identity, Manifest, RunRequest};
use c600_run::boundary::{admit_preview, Answer, Place, Store, Stored};
use c600_run::interp::{Host, Options as Limits, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome as Ended;
use c600_run::provenance::{self, field, Author, Profile};
use c600_run::value::Value;
use c600_session::disk::{Disk, FsDisk, MemDisk};
use c600_session::{Options, Row, Session};
use common::*;

/// The module of `c600-module`'s persisted-word case: an own tool `save` writes a word, and `go` proposes the stored
/// word.
const ROOT: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
    export fn save(w: Word) -> Unit effects [workspace] {\n    \
    return write_workspace<Word>(\"saved\", w);\n}\n\n\
    export fn go(t: Target) -> Text effects [snapshot, preview] {\n    \
    return receipt_id(propose(read_snapshot<Word>(\"saved\"), t));\n}\n";

const LETTERS: [i32; 2] = [1, 2];

/// The manifest, the lock and the local files of the module.
fn package() -> (Vec<u8>, Vec<u8>, Files) {
    let files = Files::new().with("src/root.6cs", ROOT.as_bytes());
    let manifest = Manifest {
        module: "m".into(),
        edition: "6cs-draft-1".into(),
        source: sha256(ROOT.as_bytes()),
        dependencies: Vec::new(),
        capabilities: Vec::new(),
        boundary: None,
        principle: None,
        supports: vec![Identity::full()],
    }
    .bytes();
    let lock = resolve(&manifest, &files).unwrap_or_else(|r| panic!("{r:?}")).bytes();
    (manifest, lock, files)
}

fn closure() -> Closure {
    let (manifest, lock, files) = package();
    verify(&manifest, &lock, &files, &Identity::full()).unwrap_or_else(|r| panic!("{r:?}"))
}

/// Approves the entry `m.<function>` under an author class and profile, with no grant and the interim policy.
fn approved(function: &str, author: Author, profile: Profile) -> (Closure, Approval) {
    let closure = closure();
    let entry = Entry {
        module: "m".into(),
        function: function.into(),
        author,
        profile,
        grant: Grant::none(),
        policy: Policy::interim(),
    };
    let approval = approve(&closure, entry, contract(), checker()).unwrap_or_else(|r| panic!("{r:?}"));
    (closure, approval)
}

fn request<'a>(run: &'a str, base: &'a str, args: Vec<Value>) -> RunRequest<'a> {
    RunRequest { run, base, args, inputs: vec![manual()], pins: pins(), options: Limits::new(WIDE) }
}

/// The engine side of a run: it counts the proposals that reach it and serves nothing else.
#[derive(Default)]
struct Engine {
    proposals: usize,
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 1, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Propose { .. } => {
                self.proposals += 1;
                Ok(Value::receipt(vec![("base", Value::text("engine")), ("id", Value::text("engine/1"))]))
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

/// The durable leg on a disk that `disk` hands out again for each session, with `power_loss` between the sessions.
fn durable_leg(disk: &dyn Fn() -> Box<dyn Disk>, power_loss: &dyn Fn()) {
    // The approved own tool stores the word; the boundary records it with the run's block (BND.22).
    let (closure, saver) = approved("save", Author::Own, Profile::ToolAuthoring);
    let ready = load(&closure, &saver).unwrap_or_else(|r| panic!("{r:?}"));
    let mut scratch = Store::default();
    let saved = ready.run(request("save", "rev", vec![Value::word(&LETTERS)]), &mut Engine::default(), &mut scratch);
    assert_eq!(saved.unwrap_or_else(|r| panic!("{r:?}")).run.summary.outcome, Ended::Completed);
    let block = scratch.records[&Place::Workspace("saved".into())].block.clone();

    // The host persists it through M6's store. A program cannot deliver the row itself (KN5.15, ONE.77).
    let session = Session::create(kernel(), disk(), "s1", Options::default()).expect("the session is created");
    let row = Row::WorkspaceSet { record: "saved".into(), payload: word_payload(&LETTERS), provenance: block.clone() };
    assert_eq!(code(&from_program(&session, "forged", row.clone())), NOT_GRANTED);
    let persisted = shell(&session, "persist", row);
    assert_eq!(action(&persisted), 1, "the only durable action");
    let before = durable(&session);
    drop(session);
    power_loss();

    // A new session opens the disk: the action and its receipt are durable (KN5.23).
    let session = Session::open(kernel(), disk(), Options::default()).expect("the session reopens");
    assert_eq!(durable(&session), before);
    assert_eq!(session.outcome("persist", &persisted.request), persisted);

    // The word read back keeps the boundary's block, byte for byte.
    let journal = disk().read(JOURNAL).expect("a readable disk").expect("a journal");
    let stored = journal_record(&journal, "saved").expect("the persisted record");
    assert_eq!(stored.payload, word_payload(&LETTERS));
    assert_eq!(stored.provenance, block);
    let read = provenance::read(&stored.provenance).expect("a valid block");
    assert_eq!((field::author(&read), field::producer(&read)), ("own", "program"));
    assert_eq!(field::profile(&read), "tool_authoring");
    assert_eq!(field::parts(&read), [sha256(&manual())]);

    // The block names the module and entry the approval binds, and the closure's bytes still load under that
    // approval (LG5.1, LG5.6, RTE.45).
    let producer = read.get("producer").expect("a producer");
    assert_eq!(producer.get("module"), Some(&provenance::digest_ref("source", &saver.module)));
    assert_eq!(producer.get("entry").and_then(|e| e.as_text()), Some("save"));
    assert!(saver.modules.contains(&saver.module));
    let again = self::closure();
    assert_eq!(again.digest, saver.closure);
    assert!(load(&again, &saver).is_ok());

    // Preview admission refuses the word read back (BND.84) ...
    let refusal = admit_preview(&stored.provenance, &Store::default(), &pins()).expect_err("an own tool's word");
    assert_eq!(refusal.code, NOT_GRANTED);

    // ... and a solving run that proposes it is refused at admission, with nothing reaching the engine (BND.39,
    // BND.84).
    let (closure, solver) = approved("go", Author::BuiltIn, Profile::Solving);
    let go = load(&closure, &solver).unwrap_or_else(|r| panic!("{r:?}"));
    let value = Value::word(&LETTERS);
    let mut store = Store::default();
    let record = Stored { value: value.clone(), bytes: encoded(&value), block: stored.provenance.clone() };
    store.records.insert(Place::Workspace("saved".into()), record);
    let base = session.overview().revision.digest;
    let mut engine = Engine::default();
    let out = go.run(request("run", &base, vec![Value::target("c0")]), &mut engine, &mut store);
    let out = out.unwrap_or_else(|r| panic!("{r:?}"));
    assert_eq!(out.shell.len(), 1);
    assert!(matches!(&out.shell[0].answer, Answer::Refused(r) if r.code == NOT_GRANTED), "{:?}", out.shell);
    assert!(out.separated.is_empty(), "admission refused it first (BND.84)");
    assert_eq!(engine.proposals, 0);
    assert_eq!(store.records[&Place::Workspace("saved".into())].block, stored.provenance, "the block is unchanged");
    assert_eq!(durable(&session), before, "the run changed nothing in the session");
}

/// CT-RTE-08's persisted-word case, durable leg, on a memory disk with a power loss between the sessions: only what
/// the store synced survives.
#[test]
fn ct_rte_08_a_persisted_tool_word_survives_a_power_loss_with_its_block() {
    let disk = MemDisk::new();
    durable_leg(&|| Box::new(disk.clone()) as Box<dyn Disk>, &|| disk.crash());
}

/// CT-RTE-08's persisted-word case, durable leg, on a directory of the file system that a new session opens.
#[test]
fn ct_rte_08_a_persisted_tool_word_survives_a_new_session_on_disk() {
    let dir = TempDir::new("ct-rte-08");
    let path = dir.path().to_path_buf();
    durable_leg(&|| Box::new(FsDisk::new(&path).expect("a store directory")) as Box<dyn Disk>, &|| {});
}

/// LG5.1, LG5.6: a stored block names one module digest. Other module bytes are another module and another closure:
/// the approval of the stored word's tool does not load them, and a block they write names their own digest.
#[test]
fn ct_rte_08_the_persisted_block_names_only_the_approved_module() {
    let (_, saver) = approved("save", Author::Own, Profile::ToolAuthoring);
    let other = format!("{ROOT}\nexport fn extra() -> Int effects [] {{\n    return 1;\n}}\n");
    let files = Files::new().with("src/root.6cs", other.as_bytes());
    let (manifest, _, _) = package();
    let mut m = Manifest::read(&manifest).unwrap_or_else(|r| panic!("{r:?}"));
    m.source = sha256(other.as_bytes());
    let manifest = m.bytes();
    let lock = resolve(&manifest, &files).unwrap_or_else(|r| panic!("{r:?}")).bytes();
    let swapped = verify(&manifest, &lock, &files, &Identity::full()).unwrap_or_else(|r| panic!("{r:?}"));
    assert_ne!(swapped.digest, saver.closure);
    assert!(load(&swapped, &saver).is_err(), "the approval does not cover the other closure");

    let (closure, other_saver) = approved_from(&swapped, "save");
    assert_ne!(other_saver.module, saver.module);
    let ready = load(&closure, &other_saver).unwrap_or_else(|r| panic!("{r:?}"));
    let mut scratch = Store::default();
    let run = ready.run(request("save", "rev", vec![Value::word(&LETTERS)]), &mut Engine::default(), &mut scratch);
    assert_eq!(run.unwrap_or_else(|r| panic!("{r:?}")).run.summary.outcome, Ended::Completed);
    let block = &scratch.records[&Place::Workspace("saved".into())].block;
    let read = provenance::read(block).expect("a valid block");
    let named = read.get("producer").and_then(|p| p.get("module")).cloned();
    assert_eq!(named, Some(provenance::digest_ref("source", &other_saver.module)));
    assert_ne!(named, Some(provenance::digest_ref("source", &saver.module)));
}

/// Approves `m.<function>` of a verified closure as an own tool.
fn approved_from(closure: &Closure, function: &str) -> (Closure, Approval) {
    let entry = Entry {
        module: "m".into(),
        function: function.into(),
        author: Author::Own,
        profile: Profile::ToolAuthoring,
        grant: Grant::none(),
        policy: Policy::interim(),
    };
    let approval = approve(closure, entry, contract(), checker()).unwrap_or_else(|r| panic!("{r:?}"));
    (closure.clone(), approval)
}
