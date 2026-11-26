//! The adoption of a twist's head off the shell's frame thread (C9, 26 November 2026; a draft).
//!
//! A twist key commits on the frame thread ([`crate::m6h_key_down`]): the commit, its durable writes and its checks are
//! the command layer's and stay where they were. What the host did after the commit, before the key call returned, was
//! the view's work: building the head's engine bundle and the view's publication ([`crate::adoption`]), checking
//! both, then handing them to the display host and the core. The building and both checks now run here, on one worker
//! thread ([`Worker`]); they read only the head, the geometry and a copy of the host context, so they change nothing
//! the frame thread holds. The frame thread then adopts the checked bundle and publishes the checked publication
//! (`ShellHost::install`), which only hands them to the core.
//!
//! The worker builds the newest job it has: a job that a newer one replaced before its build started is dropped, and
//! the frame thread installs only the newest built head it finds, so a revision may be skipped and never drawn.

use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use c600_display::jumble;
use c600_display::plan::{Publication, RevisionRef};
use c600_display::produce::{self, GeometryInput};
use c600_display::validate::{CheckedBundle, CheckedPublication, Context};

use crate::turns::Head;
use crate::{adoption, host_error, publication, Failure, M6H_E_HOST};

/// A head to build at an engine revision, with the host context the frame thread had when it asked.
pub(crate) struct Job {
    pub revision: u64,
    pub head: Head,
    pub ctx: Context,
}

/// A head built and checked at its engine revision: what the host's frame thread adopts (`ShellHost::install`).
pub(crate) struct Built {
    pub revision: u64,
    /// KN0.2's digest of the head.
    pub digest: String,
    /// The host context for the head's identity block when it is not the job's (the jumbled puzzle's poses after the
    /// solved labels); its support report is the job's.
    pub ctx: Option<Context>,
    pub bundle: CheckedBundle,
    pub publication: Publication,
    /// The publication checked against the context it will be published under, or `None` when that check refused it:
    /// the frame thread then publishes it unchecked, and the display host refuses it there as before.
    pub checked: Option<CheckedPublication>,
}

/// The host context for a head under `identity` that is not the current context's block: the puzzle's declared
/// context with the current one's limits and support report.
pub(crate) fn context_for(geometry: &GeometryInput, head: &Head, base: &RevisionRef, current: &Context) -> Context {
    let limits = current.limits.clone();
    let mut ctx = match head {
        Head::Retained(_) => produce::fixture_context(geometry, limits),
        Head::Jumbled(..) => jumble::fixture_context(geometry, base.identity.clone(), limits),
    };
    ctx.support = current.support.clone();
    ctx
}

/// Builds and checks `job`'s head: its adoption ([`crate::adoption`]), the engine bundle's check, the view's
/// publication and its check against the context it will be published under. A bundle that does not check, and a head
/// that builds none, answer [`M6H_E_HOST`] as the display host's refusal did.
pub(crate) fn build(geometry: &GeometryInput, job: Job) -> Result<Built, Failure> {
    let Job { revision, head, ctx } = job;
    let (base, bundle, primitive) = adoption(geometry, &head, revision)?;
    let new_ctx = (base.identity != ctx.identity).then(|| context_for(geometry, &head, &base, &ctx));
    let publication = publication(geometry, primitive, &base);
    let publish_ctx = new_ctx.as_ref().unwrap_or(&ctx);
    // The two checks read nothing in common, so they run side by side; without a second thread they run in turn.
    let (bundle, checked) = std::thread::scope(|scope| {
        let check = || CheckedPublication::check(publication.clone(), publish_ctx).ok();
        let side = std::thread::Builder::new().name("c600-shell-adoption-check".to_owned()).spawn_scoped(scope, check);
        let bundle = CheckedBundle::check(bundle);
        let checked = match side {
            Ok(side) => side.join().unwrap_or(None),
            Err(_) => check(),
        };
        (bundle, checked)
    });
    let bundle = bundle.map_err(host_error)?;
    Ok(Built { revision, digest: head.digest(), ctx: new_ctx, bundle, publication, checked })
}

/// One built job: its engine revision and its outcome.
pub(crate) type Outcome = (u64, Result<Built, Failure>);

/// The thread that builds the heads: jobs in, built heads out, in revision order.
pub(crate) struct Worker {
    jobs: Option<Sender<Job>>,
    built: Receiver<Outcome>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    /// Starts the thread with its own copy of `geometry` (its arrays are shared, not copied); `None` when it does not
    /// start.
    pub(crate) fn start(geometry: &GeometryInput) -> Option<Worker> {
        let (jobs, inbox) = channel::<Job>();
        let (outbox, built) = channel::<Outcome>();
        let geometry = geometry.clone();
        let thread = std::thread::Builder::new()
            .name("c600-shell-adoption".to_owned())
            .spawn(move || {
                while let Ok(mut job) = inbox.recv() {
                    // Only the newest job is built: a twist that a newer one replaced is never drawn.
                    loop {
                        match inbox.try_recv() {
                            Ok(newer) => job = newer,
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => return,
                        }
                    }
                    let revision = job.revision;
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(&geometry, job)))
                        .unwrap_or_else(|_| Err((M6H_E_HOST, "the adoption's build panicked".to_owned())));
                    if outbox.send((revision, outcome)).is_err() {
                        return;
                    }
                }
            })
            .ok()?;
        Some(Worker { jobs: Some(jobs), built, thread: Some(thread) })
    }

    /// Hands `job` to the thread, or gives it back when the thread has ended.
    pub(crate) fn submit(&self, job: Job) -> Result<(), Job> {
        match &self.jobs {
            Some(jobs) => jobs.send(job).map_err(|error| error.0),
            None => Err(job),
        }
    }

    /// The next built job: at once, until `deadline`, or for as long as it takes. `None` when none is ready by then
    /// (or the thread ended).
    pub(crate) fn take(&self, wait: Wait) -> Option<Outcome> {
        match wait {
            Wait::No => self.built.try_recv().ok(),
            Wait::Until(deadline) => self.built.recv_timeout(deadline.saturating_duration_since(Instant::now())).ok(),
            Wait::Done => self.built.recv().ok(),
        }
    }
}

/// How long [`Worker::take`] waits for a build.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Wait {
    /// Not at all.
    No,
    /// Until this instant.
    Until(Instant),
    /// Until the build ends.
    Done,
}

impl Wait {
    /// A wait of at most `limit` from now.
    pub(crate) fn at_most(limit: Duration) -> Wait {
        Wait::Until(Instant::now() + limit)
    }
}

impl Drop for Worker {
    /// Ends the thread after the build it is running, and waits for it.
    fn drop(&mut self) {
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
