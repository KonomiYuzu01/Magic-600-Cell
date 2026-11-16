//! Receipts and job results of view-lane work (KN5.21, LI.28, RM.20 to RM.22).
//!
//! Each accepted request has exactly one terminal status, and the first serialized terminal wins: a late completion,
//! cancellation or failure after it changes nothing (RM.22). A refused request changes nothing and supersedes
//! nothing (LI.23). LI.28 is the only mapping from a job outcome to a receipt status.

use crate::error::{code, Context, Error};
use crate::limits::OutputAllowance;
use crate::revision::Revision;
use crate::Digest;

/// A command identifier (KN5.21).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandId {
    /// Supplied by the caller for a standalone publication, a solver-invoked run, `view.open` or `view.close`.
    Caller(u64),
    /// Minted by the view model for a revision-driven recomputation (draft, LI.25).
    Scheduled(u64),
}

/// A job identifier (KN7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

/// KN5.21's cancel reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelReason {
    ByUser,
    Superseded { by: CommandId },
    ReplacedRevision,
    Budget,
}

/// KN5.21's completion kinds. View-lane work completes only as `view_published`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    ViewPublished,
}

/// KN5.21's `ResultRef` for a publication (RM.20).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultRef {
    /// The SHA-256 of the canonical publication.
    pub digest: Digest,
    /// The installed view generation: a number that increases with every installation.
    pub covered: u64,
    /// The job, when one exists.
    pub job: Option<JobId>,
}

/// KN5.21's `Completion`: `revision` is the publication's base revision (RM.20).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub kind: CompletionKind,
    pub revision: Revision,
    pub result: ResultRef,
}

/// KN5.21's receipt status, without `committed` (view work never commits) and `unknown_outcome`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Accepted,
    Running(JobId),
    Completed(Completion),
    Refused(Error),
    Failed(Error),
    Cancelled(CancelReason),
}

impl Status {
    /// True for every status but `accepted` and `running`.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Status::Accepted | Status::Running(_))
    }
}

/// KN7.3's job outcome, as LI.28's table reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobOutcome {
    Completed,
    Cancelled,
    CutOff(&'static str),
    Error(Error),
}

impl JobOutcome {
    /// LI.28: `completed` is completed; `cancelled` keeps its reason; a `KN7:budget.*` cut-off is
    /// `cancelled(budget)`; any other cut-off or an error is `failed(Error)`.
    pub fn receipt(&self, completion: Option<Completion>, cancelled: CancelReason) -> Status {
        match self {
            JobOutcome::Completed => match completion {
                Some(completion) => Status::Completed(completion),
                None => Status::Cancelled(cancelled),
            },
            JobOutcome::Cancelled => Status::Cancelled(cancelled),
            JobOutcome::CutOff(cut) if cut.starts_with("KN7:budget.") => Status::Cancelled(CancelReason::Budget),
            JobOutcome::CutOff(cut) => Status::Failed(Error::stop(cut, Context::None)),
            JobOutcome::Error(error) => Status::Failed(error.clone()),
        }
    }
}

/// The settled result of one accepted request (KN7.3, LI.28).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobResult {
    pub command: CommandId,
    pub job: Option<JobId>,
    pub outcome: JobOutcome,
    /// The request's output allowance after its charges (RM.13).
    pub output: OutputAllowance,
}

/// True when `code` is RM.13's output cut-off.
pub fn is_output_cut_off(cut: &str) -> bool {
    cut == code::OUTPUT
}
