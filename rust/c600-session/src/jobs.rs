//! Program jobs (KN7): one exported function of a loaded program, run on opened snapshots on a lane, under an explicit
//! budget, with cancellation by the user or by a replaced revision.
//!
//! Lanes (KN7.2): the exclusive lane holds one job and the program lane its declared capacity; a full lane refuses a
//! job with `lane_full`, so concurrent submissions cannot pass a limit. A commit, an undo, a redo or a switch cancels
//! the queries that read the replaced revision; file jobs and pinned jobs finish on their original input (KN5.18,
//! KN7.5). A job reaches the session only through its host, which answers whole-state queries on its inputs and
//! grants nothing else: no file, no workspace write, no proposal (KN4, KN5.15).

use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex, MutexGuard};

use c600_digest::sha256_hex;
use c600_ir::Ty;
use c600_kernel::{codes, Snapshot, SLOTS};
use c600_run::interp::{self, Cancel, CancelFlag, Host, Options as RunOptions, Request, RowTariff, Run, Sink};
use c600_run::meter::{self, Interruption, Outcome, Summary};
use c600_run::value::Value;

use crate::session::{Admin, Session};
use crate::types::{
    code, CancelReason, Completion, CompletionKind, Error, JobId, JobKind, JobOutcome, JobResult, JobSpec, Lane,
    Receipt, ResultRef, RevisionRef, SnapshotHandle, Status,
};

/// The kernel cost of a whole-state query: one unit per 1,024 slots, rounded up. A cache hit is charged the same as a
/// computation (KN7.8).
pub(crate) const STATE_QUERY_COST: u128 = (SLOTS as u128).div_ceil(1024);
/// The kernel cost of a query answered from the job's own inputs.
const SMALL_QUERY_COST: u128 = 1;
/// The result bound of a natural-number answer.
const NAT_BOUND: u128 = 16;

struct Entry {
    command: String,
    request: String,
    kind: JobKind,
    lane: Lane,
    flag: Arc<CancelFlag>,
    /// The `state` counters of the inputs.
    states: Vec<u64>,
    result: Option<JobResult>,
}

#[derive(Default)]
pub(crate) struct Jobs {
    entries: HashMap<u64, Entry>,
    next: u64,
    exclusive: usize,
    program: usize,
}

impl Jobs {
    pub(crate) fn running(&self) -> usize {
        self.exclusive + self.program
    }

    /// Cancels the queries that read the replaced revision. The first interruption of a job is the one it reports.
    pub(crate) fn fence(&self, replaced: u64) {
        for entry in self.entries.values() {
            let reads = entry.kind == JobKind::Query && entry.states.contains(&replaced);
            if entry.result.is_none() && reads && entry.flag.pending().is_none() {
                entry.flag.cancel(Interruption::RevisionReplaced);
            }
        }
    }

    /// A job's result: gone when the job is of another epoch or unknown, `None` while it runs.
    pub(crate) fn result(&self, epoch: u64, job: JobId) -> Result<Option<JobResult>, Error> {
        match self.entries.get(&job.n) {
            Some(entry) if job.epoch == epoch => Ok(entry.result.clone()),
            _ => Err(Error::new(code::RESULT_GONE, "no such job in this engine epoch")),
        }
    }
}

/// The answers of whole-state queries, by the digests of the states they compare.
#[derive(Default)]
pub(crate) struct DiffCache(Mutex<HashMap<(String, String), u64>>);

impl DiffCache {
    fn lock(&self) -> MutexGuard<'_, HashMap<(String, String), u64>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The number of slots where two states differ.
    fn diff(&self, a: &Snapshot, b: &Snapshot) -> u64 {
        let key = (a.digest().to_owned(), b.digest().to_owned());
        if let Some(&n) = self.lock().get(&key) {
            return n;
        }
        let n = a.labels().iter().zip(b.labels()).filter(|(x, y)| x != y).count() as u64;
        self.lock().insert(key, n);
        n
    }

    pub(crate) fn clear(&self) {
        self.lock().clear();
    }

    pub(crate) fn len(&self) -> usize {
        self.lock().len()
    }
}

/// `job.submit`: admits a job on its lane, captures its inputs and starts it.
pub(crate) fn submit(
    session: &Session,
    mut admin: MutexGuard<'_, Admin>,
    command: &str,
    request: &str,
    spec: JobSpec,
) -> Receipt {
    let inner = session.inner();
    let answer = |job, status| Receipt { command: command.to_owned(), request: request.to_owned(), job, status };
    let full = match spec.lane {
        Lane::Exclusive => admin.jobs.exclusive >= 1,
        Lane::Program => admin.jobs.program >= inner.options.program_capacity,
    };
    if full {
        return inner
            .finish(&mut admin, answer(None, Status::Refused(Error::new(code::LANE_FULL, "the lane is full"))));
    }
    if spec.inputs.is_empty() {
        let e = Error::new(codes::NAMED_OBJECT_INVALID, "a job needs an input");
        return inner.finish(&mut admin, answer(None, Status::Refused(e)));
    }
    let mut handles = Vec::with_capacity(spec.inputs.len());
    for input in &spec.inputs {
        match inner.capture(input) {
            Ok(handle) => handles.push(handle),
            Err(e) => return inner.finish(&mut admin, answer(None, Status::Refused(e))),
        }
    }
    admin.jobs.next += 1;
    let job = JobId { epoch: inner.epoch, n: admin.jobs.next };
    let flag = Arc::new(CancelFlag::default());
    match spec.lane {
        Lane::Exclusive => admin.jobs.exclusive += 1,
        Lane::Program => admin.jobs.program += 1,
    }
    admin.jobs.entries.insert(
        job.n,
        Entry {
            command: command.to_owned(),
            request: request.to_owned(),
            kind: spec.kind,
            lane: spec.lane,
            flag: Arc::clone(&flag),
            states: handles.iter().map(|h| h.revision.state).collect(),
            result: None,
        },
    );
    let receipt = inner.finish(&mut admin, answer(Some(job), Status::Running(job)));
    drop(admin);

    let base = JobResult {
        command: command.to_owned(),
        job,
        outcome: JobOutcome::Completed,
        input: handles[0].state().digest().to_owned(),
        inputs: handles.iter().map(|h| h.revision.clone()).collect(),
        params: request.to_owned(),
        budget: spec.budget,
        summary: None,
        value: None,
    };
    let worker = session.clone();
    let lost = base.clone();
    let kind = spec.kind;
    let spawned = std::thread::Builder::new().name(format!("c600-job-{}", job.n)).spawn(move || {
        let ran = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut host = JobHost { session: &worker, handles: &handles };
            let options = RunOptions { record: false, cancel: Some(&*flag), ..RunOptions::new(spec.budget) };
            spec.program.run(&spec.module, &spec.function, spec.args.clone(), &mut host, &options)
        }));
        settle(&worker, kind, base, ran.map_err(|_| ()));
    });
    if spawned.is_err() {
        settle(session, kind, lost, Err(()));
    }
    receipt
}

/// `job.stop` (LI.24): asks a running job to stop. The stop itself completes at once (`completed(app)`); the job's
/// own receipt turns `cancelled(by_user)` when the interpreter observes it.
pub(crate) fn stop(
    session: &Session,
    mut admin: MutexGuard<'_, Admin>,
    command: &str,
    request: &str,
    job: JobId,
) -> Receipt {
    let inner = session.inner();
    let status = match admin.jobs.entries.get(&job.n) {
        Some(entry) if job.epoch == inner.epoch => {
            if entry.result.is_none() && entry.flag.pending().is_none() {
                entry.flag.cancel(Interruption::ByUser);
            }
            Status::Completed(Completion { kind: CompletionKind::App, revision: inner.revision(), result: None })
        }
        _ => Status::Refused(Error::new(code::RESULT_GONE, "no such job in this engine epoch")),
    };
    let receipt = Receipt { command: command.to_owned(), request: request.to_owned(), job: Some(job), status };
    inner.finish(&mut admin, receipt)
}

/// What a terminal run means for the job's receipt and result.
pub(crate) struct Terminal {
    pub status: Status,
    pub outcome: JobOutcome,
    pub summary: Option<Summary>,
    pub value: Option<Vec<u8>>,
}

/// LI.28's mapping of a run's end: a completed run is `completed(query)` or `completed(file)` with its output's
/// digest; an interruption is `cancelled(by_user)` or `cancelled(replaced_revision)`; a budget cut-off is
/// `cancelled(budget)` and a `cut_off` result without a value; an error or a refusal is `failed`; a lost runtime is
/// `failed(runtime_lost)` without a summary.
pub(crate) fn terminal(
    kind: JobKind,
    job: JobId,
    revision: &RevisionRef,
    ran: Result<Result<Run, interp::Refusal>, ()>,
) -> Terminal {
    let failed = |e: Error, summary| Terminal {
        status: Status::Failed(e.clone()),
        outcome: JobOutcome::Error(e),
        summary,
        value: None,
    };
    let run = match ran {
        Err(()) => return failed(Error::new(code::RUNTIME_LOST, "the runtime was lost"), None),
        Ok(Err(refusal)) => return failed(Error::new(refusal.code, refusal.detail), None),
        Ok(Ok(run)) => run,
    };
    let summary = run.summary;
    match summary.outcome {
        Outcome::Completed => {
            let bytes = run.output.unwrap_or_default();
            let kind = if kind == JobKind::File { CompletionKind::File } else { CompletionKind::Query };
            let result = ResultRef { job: Some(job), preview: None, action: None, digest: sha256_hex(&bytes) };
            Terminal {
                status: Status::Completed(Completion { kind, revision: revision.clone(), result: Some(result) }),
                outcome: JobOutcome::Completed,
                summary: Some(summary),
                value: Some(bytes),
            }
        }
        Outcome::Cancelled => {
            let reason = if summary.code == Some(meter::code::CANCELLED_REVISION_REPLACED) {
                CancelReason::ReplacedRevision
            } else {
                CancelReason::ByUser
            };
            Terminal {
                status: Status::Cancelled(reason.clone()),
                outcome: JobOutcome::Cancelled(reason),
                summary: Some(summary),
                value: None,
            }
        }
        Outcome::CutOff => {
            let code = summary.code.unwrap_or(meter::code::STEPS);
            Terminal {
                status: Status::Cancelled(CancelReason::Budget),
                outcome: JobOutcome::CutOff(code),
                summary: Some(summary),
                value: None,
            }
        }
        Outcome::Error => {
            let e = Error::new(summary.code.unwrap_or(code::RUNTIME_LOST), "the program stopped with an error");
            failed(e, Some(summary))
        }
    }
}

fn settle(session: &Session, kind: JobKind, mut result: JobResult, ran: Result<Result<Run, interp::Refusal>, ()>) {
    let terminal = terminal(kind, result.job, &result.inputs[0], ran);
    result.outcome = terminal.outcome;
    result.summary = terminal.summary;
    result.value = terminal.value;
    let inner = session.inner();
    let mut admin = inner.admin();
    let Some(entry) = admin.jobs.entries.get_mut(&result.job.n) else {
        return;
    };
    let lane = entry.lane;
    let receipt = Receipt {
        command: entry.command.clone(),
        request: entry.request.clone(),
        job: Some(result.job),
        status: terminal.status,
    };
    entry.result = Some(result);
    match lane {
        Lane::Exclusive => admin.jobs.exclusive -= 1,
        Lane::Program => admin.jobs.program -= 1,
    }
    inner.finish(&mut admin, receipt);
}

/// A job's host: whole-state queries on its inputs, nothing else.
struct JobHost<'a> {
    session: &'a Session,
    handles: &'a [SnapshotHandle],
}

impl Host for JobHost<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        let Request::Snapshot { query, ty } = request else {
            return Err(meter::code::NOT_GRANTED);
        };
        if ty.ty != Ty::Nat || ty.display {
            return Err(codes::NAMED_OBJECT_INVALID);
        }
        let kernel_cost = match *query {
            "inputs" | "state" => SMALL_QUERY_COST,
            "moved" | "diff" => STATE_QUERY_COST,
            _ => return Err(codes::NAMED_OBJECT_INVALID),
        };
        Ok(RowTariff { kernel_cost, result_bound: NAT_BOUND })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let Request::Snapshot { query, .. } = request else {
            return Err(meter::code::NOT_GRANTED);
        };
        let first = &self.handles[0];
        let n = match *query {
            "inputs" => self.handles.len() as u64,
            "state" => first.revision.state,
            "moved" => {
                first.state().labels().iter().enumerate().filter(|&(slot, &label)| slot as i32 != label).count() as u64
            }
            "diff" => {
                let inner = self.session.inner();
                let other = self.handles.get(1).map_or_else(|| inner.kernel.solved(), |h| h.state().clone());
                inner.diffs.diff(first.state(), &other)
            }
            _ => return Err(codes::NAMED_OBJECT_INVALID),
        };
        Ok(Value::int(n))
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(meter::code::NOT_GRANTED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Mode;
    use c600_kernel::IdentityBlock;
    use c600_run::meter::Amounts;

    fn revision() -> RevisionRef {
        RevisionRef {
            session: "s".into(),
            epoch: 1,
            identity: IdentityBlock::full("m"),
            mode: Mode::JumbleOff,
            head: 0,
            state: 0,
            workspace: 0,
            protection: 0,
            digest: "d".into(),
        }
    }

    fn summary(outcome: Outcome, code: Option<&'static str>) -> Summary {
        Summary {
            outcome,
            code,
            stop: None,
            used: Amounts::default(),
            live_before_cleanup: 0,
            live_after_cleanup: 0,
            rows: Vec::new(),
            children: Vec::new(),
        }
    }

    fn run(
        outcome: Outcome,
        code: Option<&'static str>,
        output: Option<&[u8]>,
    ) -> Result<Result<Run, interp::Refusal>, ()> {
        Ok(Ok(Run { summary: summary(outcome, code), value: None, output: output.map(<[u8]>::to_vec) }))
    }

    /// LI.28: each way a run ends maps to one receipt status and one job outcome; a lost runtime keeps no summary.
    #[test]
    fn terminal_statuses_follow_the_run() {
        let job = JobId { epoch: 1, n: 1 };
        let r = revision();
        let done = terminal(JobKind::File, job, &r, run(Outcome::Completed, None, Some(b"1\n")));
        assert!(matches!(&done.status, Status::Completed(c) if c.kind == CompletionKind::File));
        assert_eq!((done.outcome, done.value.as_deref()), (JobOutcome::Completed, Some(&b"1\n"[..])));

        let by_user =
            terminal(JobKind::Query, job, &r, run(Outcome::Cancelled, Some(meter::code::CANCELLED_BY_USER), None));
        assert_eq!(by_user.status, Status::Cancelled(CancelReason::ByUser));
        let replaced = terminal(
            JobKind::Query,
            job,
            &r,
            run(Outcome::Cancelled, Some(meter::code::CANCELLED_REVISION_REPLACED), None),
        );
        assert_eq!(replaced.outcome, JobOutcome::Cancelled(CancelReason::ReplacedRevision));

        let cut = terminal(JobKind::Query, job, &r, run(Outcome::CutOff, Some(meter::code::OUTPUT), None));
        assert_eq!(
            (cut.status, cut.outcome, cut.value),
            (Status::Cancelled(CancelReason::Budget), JobOutcome::CutOff(meter::code::OUTPUT), None)
        );

        let error = terminal(JobKind::Query, job, &r, run(Outcome::Error, Some(meter::code::NOT_GRANTED), None));
        assert!(matches!(error.status, Status::Failed(ref e) if e.code == meter::code::NOT_GRANTED));

        let lost = terminal(JobKind::Pinned, job, &r, Err(()));
        assert!(matches!(lost.status, Status::Failed(ref e) if e.code == code::RUNTIME_LOST));
        assert!(lost.summary.is_none());
    }
}
