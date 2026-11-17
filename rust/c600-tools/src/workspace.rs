//! The host's provenance reader on `c600-command` (KN5's stage 3): the command layer reads a record's block with it
//! before `workspace.set` writes the record (PS1.16), and admits a draft that names a record, `{"record": name}`, with
//! it before the record's word is previewed (KN5.26, BND.84, BND.85).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use c600_command::{CommandLayer, Error, Provenance, Store, Table};
use c600_run::boundary::{admit_preview, Pins, Store as RunStore};
use c600_run::provenance;

use crate::tool;

/// The provenance reader of a host of these tools. [`Provenance::check`] reads a block with
/// `c600_run::provenance::read`; [`Provenance::admit`] is `c600_run::boundary::admit_preview` over the block and the
/// blocks the host keeps, under the pins the policy in force holds. Clones share the kept blocks, so a host keeps a
/// run's blocks through its clone after the layer took the reader.
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
    /// block and every part, read recursively from the kept blocks, refused with `admit_preview`'s code and detail.
    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        admit_preview(block, &self.kept(), &self.pins).map_err(|refusal| Error::new(refusal.code, refusal.detail))
    }
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
