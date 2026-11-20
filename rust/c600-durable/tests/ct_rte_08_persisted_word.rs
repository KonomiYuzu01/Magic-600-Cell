//! CT-RTE-08, the durable leg of BM2's persisted-word case (BND.22, BND.84, BND.85; LG5.1, LG5.6; CT-LG3-07's
//! saved-word half). `c600-module`'s `tests/ct_rte_08.rs` runs the case on `c600-run`'s in-memory store. Here the word
//! goes through KN5's command layer over M6's journal (`JournalStore`), with C5's provenance reader installed: an
//! approved own tool stores the word, the solver persists it with its block through `workspace.set`, the session ends
//! and a new one opens the same disk, and the word read back keeps the block, names the module the approval binds, and
//! is refused at the layer's own preview admission of the record (KN5.26), as is a solving run that proposes it. The
//! same entry approved as a built-in tool stores the same word: its record's preview is admitted once the host finds
//! the block's part, and a later write of that record makes the preview stale (KN0.8). That write keeps the part with
//! the record, so after another power loss a host that keeps no block admits the preview (CT-BND-27; LP2 draft).

mod common;

use std::collections::BTreeMap;

use c600_check::{Grant, Policy};
use c600_command::{code as layer_code, PreviewState, Provenance, RecordValue, Status, RECORD_ROW};
use c600_digest::{sha256, sha256_hex};
use c600_module::{approve, load, resolve, verify, Approval, Closure, Entry, Files, Identity, Manifest, RunRequest};
use c600_run::boundary::{Answer, Place, Store, Stored};
use c600_run::interp::{Host, Options as Limits, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome as Ended;
use c600_run::provenance::{self, field, Author, Profile};
use c600_run::value::Value;
use c600_session::disk::{Disk, FsDisk, MemDisk};
use c600_tools::bridge::{base, Bridge};
use common::layer::{completed, letters, record_args, record_draft, refused, text, Fixture};
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

/// Runs `m.save`, approved under `author`, on a scratch store: the boundary records the word with the run's block
/// (BND.22). Returns the approval, the block and the run's store, which also holds the block of the run's input.
fn saved(author: Author) -> (Approval, Vec<u8>, Store) {
    let (closure, saver) = approved("save", author, Profile::ToolAuthoring);
    let ready = load(&closure, &saver).unwrap_or_else(|r| panic!("{r:?}"));
    let mut run = Store::default();
    let ran = ready.run(request("save", "rev", vec![Value::word(&LETTERS)]), &mut Engine::default(), &mut run);
    assert_eq!(ran.unwrap_or_else(|r| panic!("{r:?}")).run.summary.outcome, Ended::Completed);
    let block = run.records[&Place::Workspace("saved".into())].block.clone();
    (saver, block, run)
}

/// What a refusal may not change: the revision, the records, the pending preview and the journal's bytes.
fn fixed(f: &Fixture, disk: &dyn Fn() -> Box<dyn Disk>) -> String {
    let journal = disk().read(JOURNAL).expect("a readable disk").expect("a journal");
    let pending = f.layer.preview().map(|preview| preview.id);
    format!("{:?} {:?} {pending:?} {}", f.layer.revision(), f.layer.records(), sha256_hex(&journal))
}

/// The durable leg on a disk that `disk` hands out again for each session, with `power_loss` between the sessions.
///
/// CT-RTE-08 (the persisted-word case, its durable leg); CT-BND-27 (part: a restored workspace keeps a tool word's
/// block, byte for byte, and the layer's preview admission in this session refuses the restored word with
/// `LG3:capability.not_granted` and installs no preview; a built-in tool's word whose one part the layer kept with
/// its write (LP2 draft) is admitted after a power loss by a host that keeps no block; not run: parts of parts, which
/// `c600-session`'s `tests/part_blocks.rs` runs on the journal, and C5's reader lists in `c600-tools`'
/// `tests/workspace.rs`); CT-KN5-13 (part: the host's BND.84/BND.85 adapter refuses a solver tool's word from restored
/// workspace provenance, and fails closed on a restored built-in word's missing part; not run: picks, draft insertion,
/// composition, file round trips, imported 0.4 macros and KN5.9's preview with a protection conflict); KN0.8 and
/// KN5.26.
fn durable_leg(disk: &dyn Fn() -> Box<dyn Disk>, power_loss: &dyn Fn()) {
    // An approved own tool stores the word, and so does the same entry approved as a built-in tool (BND.22).
    let (saver, block, _) = saved(Author::Own);
    let (_, built, run) = saved(Author::BuiltIn);

    // The solver persists both through `workspace.set` on the command layer over M6's journal, and the host's reader
    // reads each block first (PS1.16). A program cannot deliver the row itself (KN5.15, ONE.77).
    let mut f = Fixture::create(disk());
    let forged = Bridge::new(&mut f.layer, "forged").invoke(RECORD_ROW, record_args("saved", &LETTERS, &block), None);
    assert_eq!(refused(&forged), Some(layer_code::NOT_GRANTED));
    let persisted = f.write("saved", &LETTERS, &block);
    let also = f.write("built", &LETTERS, &built);
    assert!(completed(&persisted) && completed(&also), "{persisted:?} {also:?}");
    assert_eq!(f.layer.revision().workspace, 2, "the two writes are the only durable actions");
    let before = f.kept();
    drop(f);
    power_loss();

    // A new session opens the disk: the records, the actions and their receipts are durable (KN5.23).
    let mut f = Fixture::open(disk());
    assert_eq!(f.kept(), before);
    for receipt in [&persisted, &also] {
        assert_eq!(f.layer.outcome(&receipt.command, &receipt.request).as_ref(), Some(receipt));
    }

    // The word read back keeps the boundary's block, byte for byte.
    let stored = f.layer.record("saved").expect("the persisted record").clone();
    assert_eq!(stored, RecordValue { payload: letters(&LETTERS), provenance: text(&block) });
    let read = provenance::read(stored.provenance.as_bytes()).expect("a valid block");
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

    // The layer's own preview of the record is refused at admission with the reader's refusal, and nothing changes
    // (KN5.26, BND.84) ...
    let unchanged = fixed(&f, disk);
    let refusal = f.reader.admit(stored.provenance.as_bytes()).expect_err("an own tool's word");
    assert_eq!(refusal.code, layer_code::NOT_GRANTED);
    assert!(refusal.detail.ends_with("(BND.84)"), "{}", refusal.detail);
    assert_eq!(f.ask(record_draft("saved")).status, Status::Refused(refusal));
    assert_eq!((fixed(&f, disk), f.layer.run_jobs()), (unchanged.clone(), 0));

    // ... and a solving run that proposes it, reading the reopened session's revision, is refused at admission, with
    // nothing reaching the engine (BND.39, BND.84).
    let (closure, solver) = approved("go", Author::BuiltIn, Profile::Solving);
    let go = load(&closure, &solver).unwrap_or_else(|r| panic!("{r:?}"));
    let value = Value::word(&LETTERS);
    let mut store = Store::default();
    let restored = stored.provenance.as_bytes().to_vec();
    let record = Stored { value: value.clone(), bytes: encoded(&value), block: restored.clone() };
    store.records.insert(Place::Workspace("saved".into()), record);
    let mut engine = Engine::default();
    let out = go.run(request("run", &base(f.layer.revision()), vec![Value::target("c0")]), &mut engine, &mut store);
    let out = out.unwrap_or_else(|r| panic!("{r:?}"));
    assert_eq!(out.shell.len(), 1);
    assert!(matches!(&out.shell[0].answer, Answer::Refused(r) if r.code == NOT_GRANTED), "{:?}", out.shell);
    assert!(out.separated.is_empty(), "admission refused it first (BND.84)");
    assert_eq!(engine.proposals, 0);
    assert_eq!(store.records[&Place::Workspace("saved".into())].block, restored, "the block is unchanged");
    assert_eq!(fixed(&f, disk), unchanged, "the run changed nothing in the session");

    // The built-in tool's word lists the run's input block as its part. The first session's host kept no run block,
    // so its write kept no part with the record (CT-BND-27): admission fails closed (BND.21, BND.85) until the host
    // keeps the run's blocks again (crate documentation, Drafts).
    assert!(f.layer.blocks().is_empty(), "the first session kept no part block");
    let part = f.reader.admit(built.as_slice()).expect_err("a part with no stored content");
    assert_eq!(part.code, layer_code::NOT_GRANTED);
    assert!(part.detail.ends_with("(BND.21, BND.85)"), "{}", part.detail);
    assert_eq!(f.ask(record_draft("built")).status, Status::Refused(part));
    assert_eq!((fixed(&f, disk), f.layer.run_jobs()), (unchanged, 0));
    f.reader.keep_run(&run);
    let id = f.preview(record_draft("built"));
    let press = f.press(id);

    // A write of another record leaves the preview fresh ...
    assert!(completed(&f.write("other", &[3], &manual())));
    assert_eq!(f.layer.availability("operation.execute"), Ok(()));
    assert_eq!(f.layer.preview_state(id), Some(PreviewState::Presented));
    // ... and a later write of its own record, even with the same word and block, makes it stale (KN0.8): the commit
    // row is unavailable, and the press from before is refused and commits nothing.
    assert!(completed(&f.write("built", &LETTERS, &built)));
    let stale = f.layer.availability("operation.execute").map_err(|error| error.code);
    assert_eq!(stale, Err(layer_code::REVISION_CHANGED));
    let revision = f.layer.revision().clone();
    assert_eq!(refused(&f.shell.send(&mut f.layer, press)), Some(layer_code::REVISION_CHANGED));
    assert_eq!(f.layer.revision(), &revision, "nothing was committed");

    // That write came after the host kept the run's blocks, so the layer kept the word's part with it (CT-BND-27; LP2
    // draft). After another power loss, the part comes back with the journal, and a new session whose host keeps no
    // block admits the record's preview with it, while the host's reader alone still fails closed.
    let parts = BTreeMap::from([(sha256_hex(&manual()), text(&manual()))]);
    assert_eq!(f.layer.blocks(), &parts, "the run's input block is the word's one part");
    drop(f);
    power_loss();
    let mut f = Fixture::open(disk());
    assert_eq!(f.layer.blocks(), &parts);
    let alone = f.reader.admit(built.as_slice()).map_err(|error| error.code);
    assert_eq!(alone, Err(layer_code::NOT_GRANTED));
    let id = f.preview(record_draft("built"));
    assert_eq!(f.layer.preview_state(id), Some(PreviewState::Presented));
}

/// CT-RTE-08's persisted-word case, durable leg, on a memory disk with a power loss between the sessions: only what
/// the store synced survives. The parts each run are named at [`durable_leg`].
#[test]
fn ct_rte_08_a_persisted_tool_word_survives_a_power_loss_with_its_block() {
    let disk = MemDisk::new();
    durable_leg(&|| Box::new(disk.clone()) as Box<dyn Disk>, &|| disk.crash());
}

/// CT-RTE-08's persisted-word case, durable leg, on a directory of the file system that a new session opens. The parts
/// each run are named at [`durable_leg`].
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
