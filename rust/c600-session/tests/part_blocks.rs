//! CT-BND-27 on c600-command's KN5 layer over KN4's journal ([`JournalStore`]): a restored workspace keeps a word's
//! FP.17 block and its recursive parts, which the layer keeps with the record write (BND.24, BND.85; draft), and the
//! boundary's preview admission reads them after a power loss with a host reader that holds nothing. Each test names
//! the conformance rows it covers headless, with PHS.62 markers.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use c600_command::{code, CommandLayer, Error, Provenance, Receipt, Secret, Shell, Status, Table, RECORD_ROW};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_run::boundary::{admit_preview, Pins, Store};
use c600_run::provenance::{self, field, Author, Pin, Producer, Profile};
use c600_session::disk::MemDisk;
use c600_session::JournalStore;

const SECRET: [u8; 32] = [7; 32];

fn contract() -> Pin {
    Pin::of("fixture-1", b"m6 fixture capability contract\n")
}

fn checker() -> Pin {
    Pin::of("checker-1", b"m6 fixture checker\n")
}

/// The canonical block of a program's output whose module `module` an author of class `author` wrote, with `parts`.
fn output(author: Author, module: u8, parts: &[&[u8]]) -> Vec<u8> {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author,
        module: [module; 32],
        entry: "save".into(),
        profile: Profile::ToolAuthoring,
    };
    let parts: Vec<[u8; 32]> = parts.iter().map(|part| sha256(part)).collect();
    provenance::canonical(&provenance::program(&producer, false, &parts))
}

/// The host's reader, as LP3's host gives it: `c600-run`'s reader and the boundary's preview admission, over the blocks
/// it holds while a run's blocks are at hand. A reader that a restart installs holds none.
struct Reader {
    pins: Pins,
    held: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Reader {
    fn holding(blocks: &[&[u8]]) -> Reader {
        let pins = Pins { contracts: vec![contract()], checkers: vec![checker()] };
        Reader { pins, held: blocks.iter().map(|block| (sha256(block), block.to_vec())).collect() }
    }
}

fn parts_of(block: &[u8]) -> Vec<[u8; 32]> {
    provenance::read(block).map(|b| field::parts(&b)).unwrap_or_default()
}

impl Provenance for Reader {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        provenance::read(block).map(|_| ()).map_err(|why| Error::new(code::PROVENANCE_INCOMPATIBLE, why))
    }

    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        self.admit_kept(block, &BTreeMap::new())
    }

    fn parts(&self, block: &[u8]) -> Vec<String> {
        let (mut found, mut seen, mut pending) = (Vec::new(), BTreeSet::new(), parts_of(block));
        while let Some(digest) = pending.pop() {
            let Some(bytes) = self.held.get(&digest).filter(|_| seen.insert(digest)) else { continue };
            found.extend(String::from_utf8(bytes.clone()));
            pending.extend(parts_of(bytes));
        }
        found
    }

    fn admit_kept(&self, block: &[u8], kept: &BTreeMap<String, String>) -> Result<(), Error> {
        let mut store = Store::default();
        self.held.values().for_each(|block| _ = store.keep(block));
        kept.values().for_each(|block| _ = store.keep(block.as_bytes()));
        admit_preview(block, &store, &self.pins).map_err(|refusal| Error::new(refusal.code, refusal.detail))
    }
}

fn layer(store: JournalStore, created: bool, reader: Reader) -> CommandLayer<JournalStore> {
    let k = common::kernel();
    let table = Table::h08().with_drafts();
    let mut layer = match created {
        true => {
            CommandLayer::new(k.clone(), table, Secret::fixed(SECRET), store, "s1", k.solved(), ProtectedSet::none())
        }
        false => CommandLayer::open(k, table, Secret::fixed(SECRET), store).expect("the journal reopens"),
    };
    assert!(layer.recovery().is_none(), "{:?}", layer.recovery());
    layer.set_provenance(Box::new(reader));
    layer
}

fn write(layer: &mut CommandLayer<JournalStore>, shell: &mut Shell, name: &str, block: &[u8]) -> Receipt {
    let block = Value::String(String::from_utf8(block.to_vec()).unwrap());
    let payload = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let args = json::object([("payload", payload), ("provenance", block), ("record", Value::String(name.into()))]);
    shell.invoke(layer, RECORD_ROW, args).unwrap()
}

fn preview(layer: &mut CommandLayer<JournalStore>, shell: &mut Shell, name: &str) -> Receipt {
    let args = json::object([("draft", json::object([("record", Value::String(name.into()))]))]);
    shell.invoke(layer, "operation.preview", args).unwrap()
}

/// CT-BND-27 (part: restoring a workspace after a power loss preserves a word's FP.17 block and its recursive parts,
/// and the boundary's preview admission refuses the restored word, whose part is a solver tool's output, with
/// `LG3:capability.not_granted` and installs no preview, while a restored built-in word whose part is a manual word is
/// admitted; not run here: C5's reader and a file-system disk, which `c600-tools/tests/workspace.rs` and
/// `c600-durable/tests/ct_rte_08_persisted_word.rs` run) (BND.21, BND.24, BND.84, BND.85); CT-KN5-13 (part:
/// restored workspace provenance); KN5.23, KN5.26.
#[test]
fn ct_bnd_27_a_restored_word_keeps_its_recursive_parts_on_the_journal() {
    let tool = output(Author::Own, 1, &[]);
    let laundered = output(Author::BuiltIn, 2, &[&tool]);
    let manual = common::manual();
    let clean = output(Author::BuiltIn, 2, &[&manual]);

    let disk = MemDisk::new();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let mut live = layer(store, true, Reader::holding(&[&tool, &manual]));
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    for (name, block) in [("laundered", &laundered), ("clean", &clean)] {
        let receipt = write(&mut live, &mut shell, name, block);
        assert!(matches!(receipt.status, Status::Completed(_)), "{name}: {:?}", receipt.status);
    }
    let kept = live.blocks().clone();
    assert_eq!(kept.keys().collect::<BTreeSet<_>>(), [sha256_hex(&tool), sha256_hex(&manual)].iter().collect());
    drop(live);
    disk.crash();

    // A new session after the power loss, with a host reader that holds nothing.
    disk.end_holder();
    let store = JournalStore::open(Box::new(disk.clone())).expect("the journal opens");
    let mut restored = layer(store, false, Reader::holding(&[]));
    let mut shell = Shell::new(Secret::fixed(SECRET), "again");
    assert_eq!(restored.blocks(), &kept);
    let block = restored.record("laundered").expect("the restored record").provenance.clone();
    assert_eq!(block.as_bytes(), laundered.as_slice(), "the block, byte for byte");

    let revision = restored.revision().clone();
    let receipt = preview(&mut restored, &mut shell, "laundered");
    let Status::Refused(refusal) = &receipt.status else { panic!("not refused: {:?}", receipt.status) };
    assert_eq!(refusal.code, code::NOT_GRANTED);
    assert!(
        refusal.detail.contains("(BND.84)"),
        "the tool block two levels down, not a missing part: {}",
        refusal.detail
    );
    assert!(restored.preview().is_none(), "admission installs no preview");
    assert_eq!((restored.run_jobs(), restored.revision()), (0, &revision));

    let receipt = preview(&mut restored, &mut shell, "clean");
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
    assert_eq!(restored.run_jobs(), 1);
    assert!(restored.preview().is_some(), "the restored built-in word's preview is built");
}
