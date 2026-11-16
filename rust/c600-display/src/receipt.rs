//! A local mirror of KN5.21's receipt for view publications, and LI.28's mapping from a job outcome to a receipt
//! status. It does not depend on the command layer. A publication never ends `committed` (RPI.10), so the mirror has
//! no such status; success is `completed` with a `view_published` completion (RM.20).

use crate::error::{codes, Error};
use crate::plan::{CommandId, JobId, RevisionRef, ViewId};

/// KN5.21's `CancelReason`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CancelReason {
    ByUser,
    Superseded { by: CommandId },
    ReplacedRevision,
    Budget,
}

/// What a completed publication covers: the installed view generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Covered {
    pub view: ViewId,
    pub view_generation: u64,
    pub publication_generation: u64,
}

/// KN5.21's `ResultRef` for a publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultRef {
    pub job: Option<JobId>,
    /// The digest of the canonical publication.
    pub digest: String,
    pub covered: Covered,
}

/// KN5.21's `Completion` with `kind = view_published`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    /// The base revision of the publication.
    pub revision: RevisionRef,
    pub result: ResultRef,
}

/// The receipt statuses a publication can take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Accepted,
    Running(JobId),
    /// `completed(view_published)`.
    Completed(Box<Completion>),
    Refused(Error),
    Failed(Error),
    Cancelled(CancelReason),
    UnknownOutcome,
}

impl Status {
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Status::Accepted | Status::Running(_))
    }

    /// The error of a refused or failed status.
    pub fn error(&self) -> Option<&Error> {
        match self {
            Status::Refused(error) | Status::Failed(error) => Some(error),
            _ => None,
        }
    }
}

/// KN5.21's `Receipt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub command: CommandId,
    /// The request digest.
    pub request: String,
    pub job: Option<JobId>,
    pub status: Status,
}

/// A job outcome of LI.28, for a scheduled publication's job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobOutcome {
    Completed(Box<Completion>),
    Cancelled(CancelReason),
    CutOff(Error),
    Error(Error),
}

/// LI.28's mapping from a job outcome to a receipt status: a budget cut-off is `cancelled(budget)`, a safety stop
/// or an error is `failed`, a completion is `completed`.
pub fn receipt_status(outcome: &JobOutcome) -> Status {
    match outcome {
        JobOutcome::Completed(completion) => Status::Completed(completion.clone()),
        JobOutcome::Cancelled(reason) => Status::Cancelled(reason.clone()),
        JobOutcome::CutOff(error) if error.code.starts_with("KN7:budget.") => Status::Cancelled(CancelReason::Budget),
        JobOutcome::CutOff(error) | JobOutcome::Error(error) => Status::Failed(error.clone()),
    }
}

/// Whether an error code is the reserved producer plan budget, which this contract never emits (RM.13).
pub fn is_reserved(error: &Error) -> bool {
    error.code == codes::PLAN_LIMIT
}
