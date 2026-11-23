//! The research host of 600 Cell Script (`docs/progress/1.0/language-lp5-plan.md`, pieces R8 to R10).
//!
//! Its rows read and check; none chooses, ranks or applies a solving word, and every searching entry stays a research
//! tier candidate that a solving session refuses (the human-solve boundary). Only a KN6 checker's own report writes the
//! independent-checker value (EVD.64).
//!
//! # Piece R8: reproducible research work
//!
//! - [`job`]: the KN6 check job. A job runs `c600_checkers::check_until` for each of its checks, as a child scope of
//!   the job, at an input pinned by digest: a session revision opened read-only (the job's engine-owned branch, ONE.76,
//!   LB5.7) or the start an archived certificate states. It is admitted and its key sealed before it runs, it runs on
//!   its captured inputs alone, and it finishes whatever the session commits meanwhile (KN7.5). Its budget reaches the
//!   checker as a cooperative stop, so a long expansion ends as the job's cut-off with the budget's code, never as a
//!   verdict. A check's caps are cut to what the job has left, and a check nested in a check is charged to every
//!   enclosing scope, so neither mints allowance. A result is read only at its pinned revision, or one with an equal
//!   digest, under its identity block and while its checkers are available with their recorded identities.
//! - [`record`]: the job's run record on `c600-session`'s draft (PS1.31): the `ReproKey` with the checkers' identities
//!   among its fields, the exact payload with the checker's reports published by digest, and `reproduce`, which
//!   admits a stored record against the host's implementations before it re-runs the job from the captured bytes and
//!   compares the exact payload byte for byte (DET.5 to DET.7).
//! - [`evidence`]: the only way a report reaches the evidence graph, from a run's own reports through
//!   `c600_evidence::admit_check_report` (EVD.46).
//! - [`archived`]: the archive's runs as `c600-archive` imports them: recorded facts that supply no new run (DET.48).
//!
//! The crate reaches the engines through `c600_checkers::engines::Loaded` and the run record's types through
//! `c600-session`; it has no edge of its own to the kernel, the jumbling engine or the runtime. Its drafts: a check
//! job's key names the session's language edition and IR version `none`, its metering rule and the checkers' cost rule
//! by pinned text, and the kernel by its build string; a report is published as `c600-check-report-draft-1` without the
//! checker's source digest, which the key holds.

#![forbid(unsafe_code)]

pub mod archived;
pub mod evidence;
pub mod job;
pub mod record;

pub use job::{Admitted, Budget, Cancel, Check, CheckJob, CheckRun, ChildRun, Det30, Granted, Pinned};
pub use record::{reproduce, Reproduced};
