//! A job bound wholly to one checked module version (chapter 04 EV1.7, EV1.8 and EV1.11; LP4-2).
//!
//! [`BoundJob::new`] builds a [`Job`] from one `c600_module::Binding`: the bound version's module bytes in the lock's
//! order, its approved entry, and the approval's profile, grant and enabled declassifiers. Every field comes from that
//! one binding, so a job never mixes two versions. The child receives the bytes with the run and holds them to the
//! run's end: a run suspended at a host call, while the host activates another version, resumes on the same bytes,
//! and its source ranges name them. Each run of the same binding is a new invocation of the same bytes; a run after
//! another activation needs a new binding.
//!
//! # Drafts
//!
//! - **Admission column.** An approval carries no class rows, so a built-in entry's binding has no built-in column to
//!   name (`JobOptions::built_in`, BND.57). [`BoundJob::new`] refuses it at the admission stage, without a code; such
//!   an entry runs in process (`c600_module::Binding::run`). A solver's own, forked or imported entry is admitted by
//!   the child with `c600_check::admit` under the approval's profile, grant and policy, the check its approval rests
//!   on (`c600_module::admit_program`).
//! - **Options.** The caller's options give the limits, tariffs, seed, record and source-range flags, menu alphabet and
//!   trace relation; the binding replaces the profile, grant, declassifiers and admission column.

use c600_module::Binding;
use c600_run::interp::{Cancel, Host};
use c600_run::provenance::{Author, Profile};
use c600_run::value::Value;

use crate::parent::{Fault, Runtime};
use crate::program::{run_in_process, Ended, Job, JobOptions, Refused, Stage};

/// A job whose bytes, entry and admission inputs all come from one binding.
#[derive(Clone, Debug)]
pub struct BoundJob {
    job: Job,
    closure: [u8; 32],
    source: [u8; 32],
}

impl BoundJob {
    /// The job of one run of `binding`'s entry on `args` under `options`, whose profile, grant, declassifiers and
    /// admission column the binding replaces. A built-in entry is refused (see "Admission column").
    pub fn new(binding: &Binding, args: Vec<Value>, mut options: JobOptions) -> Result<BoundJob, Refused> {
        let entry = &binding.approval().entry;
        if entry.author == Author::BuiltIn {
            let detail = format!(
                "the built-in entry `{}.{}` has no class rows in its approval; it runs in process (draft)",
                entry.module, entry.function
            );
            return Err(Refused { stage: Stage::Admit, codes: Vec::new(), detail });
        }
        options.profile = match entry.profile {
            Profile::Solving => c600_check::Profile::Solving,
            Profile::ToolAuthoring => c600_check::Profile::ToolAuthoring,
            Profile::Research => c600_check::Profile::Research,
        };
        options.grant = entry.grant.capabilities.iter().copied().collect();
        options.declassifiers = entry.policy.enabled_declassifiers.iter().cloned().collect();
        options.built_in = None;
        let job = Job {
            sources: binding.sources().into_iter().map(|(_, _, bytes)| bytes.to_vec()).collect(),
            module: entry.module.clone(),
            entry: entry.function.clone(),
            args,
            options,
        };
        Ok(BoundJob { job, closure: binding.digest(), source: binding.source() })
    }

    /// The job the runtime runs.
    pub fn job(&self) -> &Job {
        &self.job
    }

    /// The bound version's closure digest.
    pub fn closure(&self) -> [u8; 32] {
        self.closure
    }

    /// The bound version's root source digest.
    pub fn source(&self) -> [u8; 32] {
        self.source
    }
}

impl Runtime {
    /// Runs a bound job in the child ([`Runtime::run`]). The child holds the bound bytes for the whole run.
    pub fn run_bound(
        &mut self,
        job: &BoundJob,
        host: &mut dyn Host,
        deadline: Option<std::time::Duration>,
    ) -> Result<Ended, Fault> {
        self.run(&job.job, host, deadline)
    }
}

/// Runs a bound job in this process ([`run_in_process`]): the reference the child is held to.
pub fn run_bound_in_process(job: &BoundJob, host: &mut dyn Host, cancel: Option<&dyn Cancel>) -> Ended {
    run_in_process(&job.job, host, cancel)
}
