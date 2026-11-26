//! The host's side of EV2's debugger (chapter 04 EV2.1, EV2.8, EV2.10, EV2.12; LP4-5, LP4-6).
//!
//! `c600-debug` has no path to the command layer (its `tests/dependencies.rs`); this module gives it what it reads:
//!
//! - [`Rows`]: the debugger's `JournalRows` over a command layer's query rows, `journal`, `revision_info` and
//!   `workspace_records`, and the outcome lookup (KN4.3, LI.14, KN5.23, LI.30). It holds a shared borrow of the layer
//!   and calls only its reading methods, so journal travel cannot write, commit, select a line or retry a command.
//!   The `checkpoints` row is not served by the headless layer and answers `KN5:input.row_not_built`.
//! - [`bind_debug`]: the debugger's route to a user module. It names the executed version's digest, a closure digest
//!   or a root source digest, so it binds through `Modules::bind_pinned` (EV1.7, EV1.11): an unchecked digest is
//!   `EV1:input.module_not_checked` and a checked version that is not active `EV1:stale.activation_base`, as on the
//!   view-source route.
//! - [`launch`]: the stepped run of a binding on RT1 in this process. The binding's boundary and separation gate stay
//!   around the engine, as in every bound run, and the debugger's read-only host sits outside them, so a preview or a
//!   sink stops at its site with `EV2:capability.debug_write_forbidden` before the boundary's store or the engine sees
//!   it. [`target`] names the bound version's closure digest and bytes; a binding run in the runtime's own process has
//!   no step mapping ([`Executor::Unmapped`]), so the debugger answers `unavailable(step_mapping_missing)` there.
//!
//! # Drafts
//!
//! - The stepped run's store is a copy the caller hands over: nothing is written to it, and the session's store is
//!   never lent to a stepping thread.
//! - A stepped run keeps one row per operation, so its stops and its profile name each operation.

use std::collections::BTreeMap;
use std::sync::Arc;

use c600_command::{code as layer_code, CommandLayer, Error as LayerError, Status};
use c600_debug::host::ReadOnly;
use c600_debug::journal::{Checkpoint, Entry, JournalRows, OutcomeRow, Page, RecordRow, RevisionInfo, RowError};
use c600_debug::session::{Executor, Finished, Target};
use c600_debug::stepper::Line;
use c600_debug::Pinned;
use c600_module::{Binding, Halt, Modules, RunContext};
use c600_run::boundary::Store;
use c600_run::interp::{load, Host, Options};
use c600_run::provenance::hex;

use crate::modules::ModuleRun;

/// The debugger's journal rows over a command layer (see the module's description).
pub struct Rows<'l, S: c600_command::store::Store>(pub &'l CommandLayer<S>);

fn row_error(error: LayerError) -> RowError {
    RowError { code: error.code, detail: error.detail }
}

impl<S: c600_command::store::Store> JournalRows for Rows<'_, S> {
    fn journal(&self, from: Option<u64>, limit: u64, cursor: Option<&str>) -> Result<Page, RowError> {
        let page = self.0.journal(from, limit, cursor).map_err(row_error)?;
        let entries = page
            .entries
            .iter()
            .map(|e| Entry {
                event: e.event(),
                order: e.order,
                kind: e.kind.name().to_owned(),
                letters: e.letters,
                solved: e.solved,
                exit: e.exit.clone(),
            })
            .collect();
        Ok(Page { entries, next: page.next })
    }

    fn revision_info(&self) -> Result<RevisionInfo, RowError> {
        let r = self.0.revision();
        Ok(RevisionInfo {
            session: r.session.clone(),
            epoch: r.epoch,
            identity: r.identity.clone(),
            mode: r.mode.to_owned(),
            head: r.head.clone(),
            state: r.state,
            workspace: r.workspace,
            protection: r.protection,
            digest: r.digest.clone(),
        })
    }

    fn checkpoints(&self) -> Result<Vec<Checkpoint>, RowError> {
        Err(RowError {
            code: layer_code::ROW_NOT_BUILT,
            detail: "`checkpoints` is not built in the headless layer".into(),
        })
    }

    fn workspace_records(&self) -> Result<Vec<RecordRow>, RowError> {
        Ok(self
            .0
            .records()
            .iter()
            .map(|(name, record)| RecordRow {
                name: name.clone(),
                version: record.version,
                digest: record.value.as_ref().map_or_else(String::new, |v| v.digest()),
            })
            .collect())
    }

    fn outcome(&self, command: &str, request: &str) -> Result<Option<OutcomeRow>, RowError> {
        Ok(self.0.outcome(command, request).map(|receipt| OutcomeRow {
            command: receipt.command.clone(),
            request: receipt.request.clone(),
            status: receipt.status.name().to_owned(),
            job: receipt.job,
            event: match &receipt.status {
                Status::Committed(record) => Some(record.event.clone()),
                _ => None,
            },
        }))
    }
}

/// Binds the debugger's route to `module.function` at the version `digest` names (see the module's description).
pub fn bind_debug(modules: &Modules, module: &str, function: &str, digest: [u8; 32]) -> Result<Binding, LayerError> {
    modules.bind_pinned(module, function, digest).map_err(|r| LayerError::new(r.code, r.detail))
}

/// The debugging target of a binding: its entry, its closure digest and bytes, and the pinned input the session
/// inspects. `executor` says where the run executes: [`Executor::Evaluator`] for [`launch`], or the runtime's process.
pub fn target(binding: &Binding, input: Pinned, executor: Executor) -> Target {
    let sources: BTreeMap<String, Arc<[u8]>> =
        binding.sources().into_iter().map(|(_, digest, bytes)| (hex(&digest), Arc::from(bytes))).collect();
    Target {
        module: binding.module().to_owned(),
        function: binding.function().to_owned(),
        module_digest: hex(&binding.digest()),
        sources,
        ir_version: c600_ir::IR_VERSION.to_owned(),
        execution_contract: c600_ir::EXEC_CONTRACT.to_owned(),
        input,
        executor,
    }
}

/// The stepped run of `binding` on RT1 in this process: `run`'s arguments and limits, `engine` behind the binding's
/// boundary and the debugger's read-only host, and `store` as the boundary's store (see the module's description).
/// [`c600_debug::session::Debugger::open`] runs it on its own thread.
pub fn launch(
    binding: Binding,
    run: ModuleRun,
    engine: Box<dyn Host + Send>,
    store: Store,
) -> impl FnOnce(&mut Line<Finished>) -> Finished + Send + 'static {
    move |line| {
        let (mut engine, mut store) = (engine, store);
        let loaded = match load(&binding.closure().program) {
            Ok(loaded) => loaded,
            Err(refusal) => return Finished::Refused { code: refusal.code, detail: refusal.detail },
        };
        let context = RunContext {
            session: &run.session,
            menu: run.menu.as_ref(),
            run: &run.run,
            base: &run.base,
            inputs: run.inputs.clone(),
            pins: binding.pins(),
        };
        let options = Options { sources: Some(binding.ranges()), ..Options::new(run.limits) };
        let (module, function, args) = (binding.module(), binding.function(), run.args.clone());
        let execute = |host: &mut dyn Host| {
            let mut read_only = ReadOnly::new(host);
            let mut dispatching = line.dispatching(&mut read_only);
            loaded.run_gated(module, function, args, &mut dispatching, &options, line)
        };
        match binding.run_with(context, &mut *engine, &mut store, execute) {
            Ok(bound) => Finished::Ran(bound.outcome.run),
            Err(Halt::Refused(refusal)) => Finished::Refused { code: refusal.code, detail: refusal.detail },
            Err(Halt::Executor(halted)) => Finished::Refused { code: halted.cause.code, detail: halted.cause.detail },
        }
    }
}
