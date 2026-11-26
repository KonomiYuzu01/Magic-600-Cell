//! A user module's runs on the host, bound wholly to one checked version (chapter 04 EV1.7, EV1.8 and EV1.11; LP4-2).
//!
//! `c600_module::Modules` holds the host's user modules, their candidates and their active versions; its
//! `activate_module` is the only switch, a module service with no command-table row. This module is the host's side of
//! the routes that run such a module:
//!
//! - [`bind_job`]: a program job of an overlay row that a user module's entry serves, whichever way the solver invoked
//!   the row (the palette through the shell, a key through the input router, a recording's step, which names the
//!   version it was recorded under). The host binds it once, when it takes the job, and the job's run stays on that
//!   version to its end.
//! - [`bind_view_source`]: a view source that names an overlay row and its checked module's source digest (EV3.2,
//!   RPI.83). The host's view declaration admits it only when the digest names the row's module's active version.
//! - [`run_bound_in_process`] and [`ProcessHost::run_bound`] run a binding in this process or in the runtime's own
//!   process (route RT1). On both, the binding's boundary and separation gate answer the program in this process
//!   (`c600_module::Binding::run_with`), and the result names the version that ran.
//!
//! A refusal of the module set reaches the layer as its error with the same code: `EV1:input.module_not_checked`,
//! `EV1:stale.activation_base` or `LG3:capability.not_granted`.
//!
//! # Drafts
//!
//! - **Arguments.** A job's arguments are JSON; the host converts them to the entry's values ([`ModuleRun::of_job`]
//!   takes them converted). The conversion of each declared parameter type waits for the declared parameters of the
//!   module formats (LG5.2).
//! - **Transfer encoding.** As for the built-in tool, both hosts refuse at the start stage an argument with no transfer
//!   encoding, with `PS1:input.noncanonical_value`, before the entry runs.
//! - **Built-in entries.** A built-in entry's binding runs in process only (`c600_runtime::bound`, "Admission column").

use std::sync::Arc;

use c600_command::{Error as LayerError, ModuleVersions, Overlay, ProgramJob, ViewSourceRef};
use c600_module::{Binding, BoundRun, Halt, Identity, Modules, Refusal, RunContext, RunRequest};
use c600_run::boundary::Store;
use c600_run::interp::{code, Host, Options};
use c600_run::menu::MenuIdentity;
use c600_run::meter::Amounts;
use c600_run::value::Value;
use c600_runtime::codec::encode_value;
use c600_runtime::wire::Enc;
use c600_runtime::{BoundJob, Ended, JobOptions, Refused, Stage};

use crate::process::{Cause, ProcessHost, Stopped};
use crate::tool::limits;

/// One run of a bound entry: its name, the revision it reads, its arguments with their stored blocks, and its limits.
#[derive(Clone, Debug)]
pub struct ModuleRun {
    /// The identity of the session the run reads (KN0.1): the run is refused before any operation when the bound
    /// approval names another (LG5.5, CT-LG5-04; LP2 draft).
    pub session: Identity,
    /// The identity of the menu alphabet the run pins, when it pins one.
    pub menu: Option<MenuIdentity>,
    pub run: String,
    pub base: String,
    pub args: Vec<Value>,
    pub inputs: Vec<Vec<u8>>,
    pub limits: Amounts,
}

impl ModuleRun {
    /// The run of a program job: `job-<id>` at the job's revision, under the job's budget (KN7.7), with the job's
    /// argument blocks, on `args`, the job's arguments converted to the entry's values.
    pub fn of_job(job: &ProgramJob, args: Vec<Value>, session: Identity) -> ModuleRun {
        ModuleRun {
            session,
            menu: None,
            run: job.run(),
            base: crate::bridge::base(&job.revision),
            args,
            inputs: job.inputs.clone(),
            limits: limits(job.budget),
        }
    }
}

fn layer_error(refusal: Refusal) -> LayerError {
    LayerError::new(refusal.code, refusal.detail)
}

/// Binds the program job of an overlay row to the active version of the module its entry belongs to (EV1.11). The
/// host calls it once, when it takes the job; a refusal is the job's error.
///
/// The job of a recording's step names the module digest the step was recorded under ([`ProgramJob::module`]; EV4.10,
/// EV4.14; LP4-9): the route that names a digest, so the job binds only when that digest names the active version. A
/// digest no successful check produced is `EV1:input.module_not_checked`, a checked one that is not active
/// `EV1:stale.activation_base`, and one that is not 64 lowercase hex digits `EV4:input.recording_step_unresolved`.
pub fn bind_job(modules: &Modules, job: &ProgramJob) -> Result<Binding, LayerError> {
    let Some(recorded) = &job.module else {
        return modules.bind(&job.overlay.module, &job.overlay.entry).map_err(layer_error);
    };
    let unresolved = || {
        let detail = format!("the recorded module digest `{recorded}` is not a SHA-256 in hex");
        LayerError::new(c600_command::code::RECORDING_STEP_UNRESOLVED, detail)
    };
    let digest = unhex(recorded).ok_or_else(unresolved)?;
    modules.bind_pinned(&job.overlay.module, &job.overlay.entry, digest).map_err(layer_error)
}

/// The host's reader of its user modules' active versions for the command layer (`c600_command::ModuleVersions`;
/// EV1.7, EV4.14; LP4-9): the hex closure digest of a module's active version. The layer records it with a step that
/// an overlay row of the module served, and checks it before the step plays.
#[derive(Clone)]
pub struct ActiveVersions(pub Arc<Modules>);

impl ModuleVersions for ActiveVersions {
    fn active(&self, module: &str) -> Option<String> {
        self.0.active(module).map(|digest| c600_run::provenance::hex(&digest))
    }
}

/// Binds a view source of the overlay row `overlay` serves (EV1.7, EV3.2): an overlay source is admitted only when the
/// module digest it names, its checked module's source digest, names the active version of `overlay`'s module. A digest
/// that is not 64 hex digits is `KN5:input.argument_form` naming `source`, as the view model's form refuses it; a
/// built-in source is not a user module's and is `KN5:input.argument_form` too.
pub fn bind_view_source(modules: &Modules, overlay: &Overlay, source: &ViewSourceRef) -> Result<Binding, LayerError> {
    let form = || LayerError::new(c600_command::code::ARGUMENT_FORM, "source");
    let ViewSourceRef::Overlay { module, .. } = source else { return Err(form()) };
    let digest = unhex(module).ok_or_else(form)?;
    modules.bind_pinned(&overlay.module, &overlay.entry, digest).map_err(layer_error)
}

fn unhex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

fn start_refused(code: &str, detail: String) -> Stopped {
    Stopped {
        cause: Cause::Refused(Refused { stage: Stage::Start, codes: vec![code.to_owned()], detail }),
        shell: vec![],
    }
}

/// The start-stage refusal of an argument with no transfer encoding, which both hosts give.
fn untransferable(args: &[Value]) -> Option<Stopped> {
    args.iter()
        .any(|arg| encode_value(arg, &mut Enc::new()).is_err())
        .then(|| start_refused(code::NONCANONICAL_VALUE, "an argument has no canonical transfer form".to_owned()))
}

/// Runs a bound entry in this process on `engine` and `store`, under the run's limits, with no per-operation rows.
pub fn run_bound_in_process(
    binding: &Binding,
    run: &ModuleRun,
    engine: &mut dyn Host,
    store: &mut Store,
) -> Result<BoundRun, Stopped> {
    if let Some(stopped) = untransferable(&run.args) {
        return Err(stopped);
    }
    let request = RunRequest {
        session: &run.session,
        run: &run.run,
        base: &run.base,
        args: run.args.clone(),
        inputs: run.inputs.clone(),
        pins: binding.pins(),
        options: Options { record: false, ..Options::new(run.limits) },
    };
    binding.run(request, engine, store).map_err(|refusal| start_refused(refusal.code, refusal.detail))
}

impl ProcessHost {
    /// Runs a bound entry in the runtime's child on `engine` and `store`, as [`run_bound_in_process`] runs it here: the
    /// child gets the bound version's bytes with the run ([`BoundJob`]) and forwards each host call to the binding's
    /// boundary in this process. A run suspended at a host call resumes on the same bytes whatever activates meanwhile.
    pub fn run_bound(
        &mut self,
        binding: &Binding,
        run: &ModuleRun,
        engine: &mut dyn Host,
        store: &mut Store,
    ) -> Result<BoundRun, Stopped> {
        if let Some(stopped) = untransferable(&run.args) {
            return Err(stopped);
        }
        let options = JobOptions { record: false, ..JobOptions::new(run.limits) };
        let job = BoundJob::new(binding, run.args.clone(), options)
            .map_err(|refused| Stopped { cause: Cause::Refused(refused), shell: vec![] })?;
        let context = RunContext {
            session: &run.session,
            menu: run.menu.as_ref(),
            run: &run.run,
            base: &run.base,
            inputs: run.inputs.clone(),
            pins: binding.pins(),
        };
        let deadline = self.deadline;
        let runtime = &mut self.runtime;
        let execute = |host: &mut dyn Host| match runtime.run_bound(&job, host, deadline) {
            Ok(Ended::Ran(run)) => Ok(run),
            Ok(Ended::Refused(refused)) => Err(Cause::Refused(refused)),
            Err(fault) => Err(Cause::Lost(fault)),
        };
        binding.run_with(context, engine, store, execute).map_err(|halt| match halt {
            Halt::Refused(refusal) => start_refused(refusal.code, refusal.detail),
            Halt::Executor(halted) => Stopped { cause: halted.cause, shell: halted.shell },
        })
    }
}
