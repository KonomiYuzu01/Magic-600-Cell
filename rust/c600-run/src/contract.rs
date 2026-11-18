//! Whole-state contracts (LG4.10, LB1.7; LP2 draft until the IR freeze). A pointwise contract is an `assert` in a
//! body: its predicate runs as ordinary operations and its one `assert` step is charged to the run that holds it
//! (DET.55). A whole-state contract is a function of kind [`FnKind::Contract`]: it takes no parameter, returns an exact
//! `Bool` and reads its revision's state and nothing else, and no function calls or holds it (the checker refuses
//! both), so no call and no loop runs it per call. It runs once per revision, through [`Revisions`], or as an
//! agreement test in CI, which is a test that runs it.
//!
//! [`Revisions`] keeps, for each contract, its run at the latest revision it was asked about. Asked again at that
//! revision, it gives the kept run and runs nothing; asked at another revision, it runs the contract and replaces the
//! kept run, so a changed input revision invalidates the earlier result. The key is the program's digest, the module
//! and the function, so a changed program runs again too. The caller names the revision and supplies a host that reads
//! that revision; since a contract reads no clock, file or randomness (the checker), its result is a function of the
//! revision. A kept run is a value, never evidence: a contract assertion is no independent checker pass and promotes
//! no evidence of its own (LG4.10).

use std::collections::BTreeMap;

use c600_ir::FnKind;

use crate::interp::{code, Host, Loaded, Options, Refusal, Run};

/// A whole-state contract: the program's digest, its module and its function.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContractId {
    pub program: String,
    pub module: String,
    pub function: String,
}

/// Whether [`Revisions::at`] ran the contract or gave the run kept for the revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Ran,
    Kept,
}

/// The kept runs of whole-state contracts, one per contract, at the latest revision asked about.
#[derive(Debug, Default)]
pub struct Revisions {
    kept: BTreeMap<ContractId, (String, Run)>,
}

impl Revisions {
    /// The contract's run at `revision`: the kept one when it ran at this revision, otherwise a new run with no
    /// argument through `host`, which replaces the kept run of another revision. A function that is no whole-state
    /// contract is refused with `LG4:input.ir_rejected`, and so is a contract the run refuses before its first
    /// operation; neither replaces a kept run.
    pub fn at(
        &mut self,
        loaded: &Loaded,
        id: &ContractId,
        revision: &str,
        host: &mut dyn Host,
        options: &Options,
    ) -> Result<(&Run, Source), Refusal> {
        let function = loaded.function(&id.module, &id.function).filter(|f| loaded.kind(*f) == FnKind::Contract);
        if function.is_none() {
            let detail = format!("{}.{} is no whole-state contract (LG4.10)", id.module, id.function);
            return Err(Refusal { code: code::IR_REJECTED, detail });
        }
        let kept = matches!(self.kept.get(id), Some((r, _)) if r == revision);
        if !kept {
            let run = loaded.run(&id.module, &id.function, Vec::new(), host, options)?;
            self.kept.insert(id.clone(), (revision.to_string(), run));
        }
        let (_, run) = self.kept.get(id).expect("a kept run");
        Ok((run, if kept { Source::Kept } else { Source::Ran }))
    }

    /// The revision of the contract's kept run, if it has one.
    pub fn revision(&self, id: &ContractId) -> Option<&str> {
        self.kept.get(id).map(|(r, _)| r.as_str())
    }
}
