//! The host's provenance reader on `c600-command` (KN5's stage 3): the command layer reads a record's block with it
//! before `workspace.set` writes the record (PS1.16), and admits a draft that names a record, `{"record": name}`, with
//! it before the record's word is previewed (KN5.26, BND.84, BND.85). It lists a block's parts that the host keeps,
//! which the layer keeps with the action that writes the record, and it admits a record's word with the part blocks
//! the session keeps as well as its own (CT-BND-27; LP2 draft).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use c600_command::{CommandLayer, Error, Provenance, Store, Table};
use c600_run::boundary::{admit_preview, Pins, Store as RunStore};
use c600_run::provenance::{self, field};

use crate::tool;

/// The provenance reader of a host of these tools. [`Provenance::check`] reads a block with
/// `c600_run::provenance::read`; [`Provenance::admit`] is `c600_run::boundary::admit_preview` over the block and the
/// blocks the host keeps, under the pins the policy in force holds, and [`Provenance::admit_kept`] reads the part
/// blocks the session keeps as well. Clones share the kept blocks, so a host keeps a run's blocks through its clone
/// after the layer took the reader.
#[derive(Clone, Debug)]
pub struct Reader {
    pins: Pins,
    /// Only the store's blocks are read: preview admission resolves parts by content digest.
    kept: Arc<Mutex<RunStore>>,
}

impl Reader {
    /// A reader under `pins` that keeps no block yet.
    pub fn new(pins: Pins) -> Reader {
        Reader { pins, kept: Arc::default() }
    }

    /// Keeps block bytes, readable or not, under their content digest (PS1.33) and returns the digest, so that a word
    /// whose block lists them as a part finds them at admission (BND.85).
    pub fn keep(&self, block: &[u8]) -> [u8; 32] {
        self.kept().keep(block)
    }

    /// Keeps every block a run's store holds: the blocks of the run's inputs and of what it proposed or stored.
    /// Kept blocks are never dropped.
    pub fn keep_run(&self, run: &RunStore) {
        let mut kept = self.kept();
        for (digest, block) in &run.blocks {
            kept.blocks.entry(*digest).or_insert_with(|| block.clone());
        }
    }

    fn kept(&self) -> MutexGuard<'_, RunStore> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Provenance for Reader {
    /// A record's block must read as a canonical FP.17 block (PS1.16); anything else reads `unknown` (BND.21) and is
    /// refused with `PS1:capability.provenance_incompatible`. The pins and the producer are admission's.
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        provenance::read(block).map(|_| ()).map_err(|code| {
            Error::new(code, "the record's provenance does not read as a canonical FP.17 block (PS1.16, BND.21)")
        })
    }

    /// Preview admission of a record's word in a solving session (KN5.26), the only profile of this host: the word's
    /// block and every part, read recursively from the blocks the host keeps, refused with `admit_preview`'s code and
    /// detail.
    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        self.admit_kept(block, &BTreeMap::new())
    }

    /// The blocks of `block`'s parts, recursively, among the blocks the host keeps, as text (BND.24, BND.85). The layer
    /// keeps them durably with the action that writes a record with `block`, so a reopened session admits the record's
    /// word without the host keeping the run's blocks again (CT-BND-27; LP2 draft). A part the host does not keep is
    /// not listed, nor is one whose block is not UTF-8 (such a block does not read, so it has no parts); each block is
    /// listed once.
    fn parts(&self, block: &[u8]) -> Vec<String> {
        let kept = self.kept();
        let (mut found, mut seen, mut pending) = (Vec::new(), BTreeSet::new(), parts_of(block));
        while let Some(digest) = pending.pop() {
            let Some(part) = kept.blocks.get(&digest).filter(|_| seen.insert(digest)) else { continue };
            found.extend(String::from_utf8(part.clone()));
            pending.extend(parts_of(part));
        }
        found
    }

    /// [`Provenance::admit`] with the part blocks the session keeps (`kept`) as well as the host's: a part is read
    /// from either, each block under the digest of its own bytes whatever its key in `kept`, and a part that neither
    /// holds reads `unknown` and fails closed (BND.21, BND.85).
    fn admit_kept(&self, block: &[u8], kept: &BTreeMap<String, String>) -> Result<(), Error> {
        let held = self.kept();
        let admitted = if kept.is_empty() {
            admit_preview(block, &held, &self.pins)
        } else {
            let mut store = RunStore::default();
            store.blocks.clone_from(&held.blocks);
            for part in kept.values() {
                store.keep(part.as_bytes());
            }
            admit_preview(block, &store, &self.pins)
        };
        admitted.map_err(|refusal| Error::new(refusal.code, refusal.detail))
    }
}

/// The content digests of a block's direct parts; a block that does not read has none.
fn parts_of(block: &[u8]) -> Vec<[u8; 32]> {
    provenance::read(block).map(|read| field::parts(&read)).unwrap_or_default()
}

/// The command table of a host of these tools: H-08 with the command layer's draft rows (`workspace.set`).
pub fn table() -> Table {
    Table::h08().with_drafts()
}

/// Installs a reader under the pins of these tools' runs ([`tool::pins`]) on `layer` and returns a clone of it, through
/// which the host keeps the blocks its runs leave.
pub fn install<S: Store>(layer: &mut CommandLayer<S>) -> Reader {
    let reader = Reader::new(tool::pins());
    layer.set_provenance(Box::new(reader.clone()));
    reader
}
