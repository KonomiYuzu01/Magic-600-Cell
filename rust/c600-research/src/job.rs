//! The KN6 check job (LP5 plan, piece R8): KN6's checks run as a research job at a digest-pinned input, under an
//! explicit deterministic budget, with a run record that reproduces byte for byte.
//!
//! - [`CheckJob`] names the checks, the input the job is pinned at ([`Pinned`]: a session revision opened read-only,
//!   or a start an archived certificate states) and the job's [`Budget`]. [`CheckJob::admit`] seals the key before
//!   anything runs (DET.41); [`Admitted::run`] runs it. A job admitted at one revision runs on that revision whatever
//!   the session commits meanwhile, and finishes (KN7.5, LB5.7); its result is bound to its input.
//! - Each check is a child scope of the job, as RT1's `child` is: its requested caps are cut to what the job has left,
//!   and an omitted cap is that remainder. The job's kernel cost reaches the checker as a cooperative stop
//!   (`c600_checkers::check_until`): a check that runs out ends as a cut-off with the budget's code, never as a
//!   verdict. A check's own cap ends that check only; the job's ends the job. A check nested inside a check is charged
//!   to every enclosing scope, so neither mints allowance (DET.25, DET.26).
//! - Accounting ([`crate::record::METERING_RULE`]): one step per check admitted; the checker's units as kernel cost;
//!   the bytes of a check's certificate and inputs as its live memory while it runs, the job's being the peak; the
//!   bytes of each published report, and of the job's value, as output.
//! - [`CheckRun`] holds the record, the published reports and the per-check ledger. Its reports are the kernel
//!   checker's own; [`crate::evidence::admit`] takes them only from a `CheckRun`, which only a run or a matching
//!   reproduction makes.
//!
//! Nothing here chooses, ranks or outputs a solving word: a check reads a certificate the caller names.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use c600_checkers::{
    check_until, checker, claims, CertificateRef, CheckReport, Checked, CheckerId, CheckerKind, Engines, Input, Refusal,
};
use c600_digest::sha256_hex;
use c600_session::persist::codec::{canonical, digest_ref, nat_text, Json};
use c600_session::persist::record::identity_json;
use c600_session::persist::run::{reading, Coverage, Key, Record};
use c600_session::{RevisionRef, SnapshotHandle};

use crate::record;

/// The codes a check job stops or refuses with (chapter 13).
pub mod codes {
    pub const STEPS: &str = "KN7:budget.steps";
    pub const KERNEL_COST: &str = "KN7:budget.kernel_cost";
    pub const MEMORY: &str = "KN7:budget.memory";
    pub const OUTPUT: &str = "KN7:budget.output";
    pub const CANCELLED: &str = "KN7:cancelled.by_user";
    /// A result read at another revision, or in another mode, than the one its input was pinned at.
    pub const STALE: &str = "KN5:stale.revision_changed";
    /// A result read under another model, menu, menu order or contract revision.
    pub const IDENTITY: &str = "KN1:input.identity_mismatch";
    pub const NO_CHECKER: &str = c600_checkers::codes::NO_CHECKER;
}

/// KN7.7's deterministic budget of a job or of one check: interpreter steps, kernel cost, logical memory and output.
/// Seconds are never a budget unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Budget {
    pub steps: u128,
    pub kernel_cost: u128,
    pub memory: u128,
    pub output: u128,
}

impl Budget {
    pub const fn new(steps: u128, kernel_cost: u128, memory: u128, output: u128) -> Budget {
        Budget { steps, kernel_cost, memory, output }
    }

    /// Each counter the smaller of the two.
    pub fn min(self, other: Budget) -> Budget {
        Budget {
            steps: self.steps.min(other.steps),
            kernel_cost: self.kernel_cost.min(other.kernel_cost),
            memory: self.memory.min(other.memory),
            output: self.output.min(other.output),
        }
    }

    /// What is left of these limits after `used`; memory is not used up (a check's memory is live only while it runs).
    fn left(self, used: Budget) -> Budget {
        Budget {
            steps: self.steps.saturating_sub(used.steps),
            kernel_cost: self.kernel_cost.saturating_sub(used.kernel_cost),
            memory: self.memory,
            output: self.output.saturating_sub(used.output),
        }
    }
}

/// An immutable input of a check, granted by the host under its role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Granted {
    pub role: String,
    pub bytes: Vec<u8>,
}

/// One check of a job: a certificate of a kind, the SHA-256 its bytes are stated to have (the archive's manifest or
/// the caller's record), its inputs and its requested caps (`None`: what the job has left).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub kind: CheckerKind,
    pub certificate: Vec<u8>,
    pub stated: String,
    pub inputs: Vec<Granted>,
    pub cap: Option<Budget>,
}

impl Check {
    /// A check of `certificate`, stated to have its own digest, with no cap of its own.
    pub fn new(kind: CheckerKind, certificate: Vec<u8>, inputs: Vec<(&str, Vec<u8>)>) -> Check {
        let stated = sha256_hex(&certificate);
        let inputs = inputs.into_iter().map(|(role, bytes)| Granted { role: role.to_owned(), bytes }).collect();
        Check { kind, certificate, stated, inputs, cap: None }
    }

    /// The bytes the check holds while it runs: its live memory.
    fn held(&self) -> u128 {
        (self.certificate.len() + self.inputs.iter().map(|input| input.bytes.len()).sum::<usize>()) as u128
    }
}

/// The input a job is pinned at (DET.1's `input` and `identity`): a level, a digest and a KN0.1 identity block, and the
/// session revision it was opened at when it is a session's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pinned {
    /// FP.8's level: `labels` or `poses`.
    pub level: String,
    pub digest: String,
    /// The identity block as FP.5's value (`identity_json`).
    pub identity: Json,
    /// The journal revision of a session snapshot: the job's engine-owned branch (ONE.76, LB5.7).
    pub revision: Option<RevisionRef>,
}

impl Pinned {
    /// A session's revision, opened read-only: the labels digest and identity block of that revision.
    pub fn snapshot(handle: &SnapshotHandle) -> Pinned {
        Pinned {
            level: "labels".into(),
            digest: handle.revision.digest.clone(),
            identity: identity_json(&handle.revision.identity),
            revision: Some(handle.revision.clone()),
        }
    }

    /// A start a certificate states, at `level`, under `identity` (`identity_json` of the engine's block).
    pub fn stated(level: &str, digest: &str, identity: Json) -> Pinned {
        Pinned { level: level.into(), digest: digest.into(), identity, revision: None }
    }

    /// The identity block's wire text, as an evidence scope holds it.
    pub fn identity_text(&self) -> String {
        let mut bytes = canonical(&self.identity);
        bytes.pop();
        String::from_utf8(bytes).unwrap_or_default()
    }

    /// Whether `report` is about this input: `Some(true)` when the checker itself read a start state with this digest,
    /// `Some(false)` when it read start states and none has it (the check was about another state), and `None` when it
    /// read no start state (a word's expansion without a replay, a record checked as data). The states a checker reads
    /// itself are bound as `digest`, or as `solved` when the certificate names no start (`c600_checkers::replay`).
    pub fn bound_by(&self, report: &CheckReport) -> Option<bool> {
        let mut states = report
            .inputs
            .iter()
            .flat_map(|input| input.identity.iter())
            .filter(|(name, _)| name == "digest" || name == "solved")
            .map(|(_, digest)| digest == &self.digest)
            .peekable();
        states.peek()?;
        Some(states.any(|same| same))
    }
}

/// A KN6 check job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckJob {
    pub checks: Vec<Check>,
    pub input: Pinned,
    pub budget: Budget,
}

impl CheckJob {
    /// Admission: every check's checker must be available, and the key is sealed over the captured inputs before
    /// anything runs (DET.7, DET.41). Refuses with `KN6:evidence.no_checker` or the key's format code.
    pub fn admit(self) -> Result<Admitted, Refusal> {
        let key = record::seal(&self)?;
        Ok(Admitted { job: self, key })
    }
}

/// A cooperative cancellation (KN7.6): set by the solver, read at the job's next charge.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Cancel {
        Cancel::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// An admitted job: its sealed key, ready to run.
#[derive(Clone, Debug)]
pub struct Admitted {
    job: CheckJob,
    key: Key,
}

/// One check's ledger entry: its caps, what it used and how it ended, with its report when it reached one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildRun {
    /// The check's place in the job: `check[i]`.
    pub at: String,
    pub kind: CheckerKind,
    pub checker: CheckerId,
    /// The SHA-256 of the certificate's bytes.
    pub certificate: String,
    pub requested: Budget,
    pub effective: Budget,
    pub used: Budget,
    /// `completed`, `cut_off`, `cancelled` or `error`.
    pub outcome: &'static str,
    pub code: Option<&'static str>,
    /// Where a stop happened: the units the check had charged, or `admission`.
    pub stop: Option<String>,
    /// The checker's own report, when the check reached its verdict and the report was published.
    pub report: Option<CheckReport>,
    /// The report's index among the job's published outputs.
    pub output: Option<usize>,
}

/// DET.30's reading of a check job: a completed job; a job its budget stopped before any check charged a unit or
/// published a report (`not run`: no examined set); a deterministic cut-off with the units it covered (`not found
/// within budget`); or an open stop with its code (cancellation or a clock).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Det30 {
    Complete,
    NotRun,
    CutOff { covered: u128 },
    Open(String),
}

/// A job that ran: its run record and the reports it published.
#[derive(Clone, Debug)]
pub struct CheckRun {
    record: Record,
    bytes: Vec<u8>,
    input: Pinned,
    children: Vec<ChildRun>,
    outputs: Vec<Vec<u8>>,
    value: Option<Vec<u8>>,
    captured: Vec<Vec<u8>>,
}

/// Where and why the job stopped.
struct JobStop {
    code: &'static str,
    at: String,
    phase: &'static str,
}

/// The code a scope stops with at `total` units: its budget first, which is deterministic, then a cancellation.
fn stop_code(total: u64, limit: u128, cancel: &Cancel) -> Option<&'static str> {
    if u128::from(total) > limit {
        Some(codes::KERNEL_COST)
    } else if cancel.is_cancelled() {
        Some(codes::CANCELLED)
    } else {
        None
    }
}

/// What a check's admission lacks: a step, or memory for what it holds.
fn lacks(budget: Budget, held: u128) -> Option<&'static str> {
    if budget.steps < 1 {
        Some(codes::STEPS)
    } else if held > budget.memory {
        Some(codes::MEMORY)
    } else {
        None
    }
}

fn outcome_of(code: &'static str) -> &'static str {
    if code == codes::CANCELLED {
        "cancelled"
    } else {
        "cut_off"
    }
}

/// The running job's state, in the order the checks ran.
#[derive(Default)]
struct Running {
    used: Budget,
    peak: u128,
    children: Vec<ChildRun>,
    outputs: Vec<Vec<u8>>,
    stop: Option<JobStop>,
}

impl Admitted {
    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn job(&self) -> &CheckJob {
        &self.job
    }

    /// Runs the job on `engines`, on its captured inputs only. The job's kernel cost and `cancel` stop it
    /// cooperatively at the next unit a checker charges; the record is written whatever the outcome. Refuses only a
    /// record that cannot be written and a run started from inside a stop hook.
    pub fn run(&self, engines: &Engines, cancel: &Cancel) -> Result<CheckRun, Refusal> {
        let limits = self.job.budget;
        let mut state = Running::default();
        let parent = cancel.clone();
        let scope = claims::scoped(
            move |total| stop_code(total, limits.kernel_cost, &parent),
            || self.run_checks(engines, cancel, &mut state),
        )
        .ok_or_else(|| Refusal { code: codes::NO_CHECKER, message: "a job cannot start inside a stop hook".into() })?;
        if let (None, Some(code)) = (&state.stop, scope.stopped) {
            state.stop = Some(JobStop { code, at: format!("unit {}", scope.charged), phase: "check" });
        }

        // The value: one entry per check, with its report's index among the outputs. A stopped job has none.
        let mut value = None;
        if state.stop.is_none() {
            let entries = state
                .children
                .iter()
                .map(|child| {
                    let verdict = child.report.as_ref().map_or(Json::Null, |report| match &report.verdict {
                        c600_checkers::Verdict::Pass => Json::text("pass"),
                        c600_checkers::Verdict::Fail { .. } => Json::text("fail"),
                        c600_checkers::Verdict::NotApplicable { .. } => Json::text("not_applicable"),
                    });
                    Json::object([
                        ("checker", Json::text(child.checker.id)),
                        ("outcome", Json::text(child.outcome)),
                        ("verdict", verdict),
                        ("report", child.output.map_or(Json::Null, |i| nat_text(i as u128))),
                    ])
                })
                .collect();
            let bytes = canonical(&Json::object([("checks", Json::List(entries))]));
            let size = bytes.len() as u128;
            if state.used.output.saturating_add(size) > limits.output {
                state.stop = Some(JobStop { code: codes::OUTPUT, at: "value".into(), phase: "output" });
            } else {
                state.used.output += size;
                value = Some(bytes);
            }
        }

        let used = Budget { memory: state.peak, ..state.used };
        let (outcome, code, stop) = match &state.stop {
            None => ("completed", Json::Null, Json::Null),
            Some(stop) => (outcome_of(stop.code), Json::text(stop.code), position(&stop.at, stop.phase, "job")),
        };
        let payload = Json::object([
            ("outcome", Json::text(outcome)),
            ("code", code),
            ("stop", stop),
            ("used", record::amounts(&used)),
            ("children", Json::List(state.children.iter().map(child_json).collect())),
            ("value", value.as_ref().map_or(Json::Null, |bytes| digest_ref("bytes", bytes))),
            ("outputs", Json::List(state.outputs.iter().map(|bytes| digest_ref("bytes", bytes)).collect())),
        ]);
        let (record, bytes) = record::write(&self.key, payload)?;
        let captured = self
            .job
            .checks
            .iter()
            .flat_map(|check| {
                std::iter::once(check.certificate.clone()).chain(check.inputs.iter().map(|i| i.bytes.clone()))
            })
            .collect();
        Ok(CheckRun {
            record,
            bytes,
            input: self.job.input.clone(),
            children: state.children,
            outputs: state.outputs,
            value,
            captured,
        })
    }

    /// The checks, one child scope each, until the job stops.
    fn run_checks(&self, engines: &Engines, cancel: &Cancel, state: &mut Running) {
        let limits = self.job.budget;
        for (index, check) in self.job.checks.iter().enumerate() {
            let at = format!("check[{index}]");
            let left = limits.left(state.used);
            let requested = check.cap.unwrap_or(left);
            let effective = requested.min(left);
            let held = check.held();
            let identity = match checker(check.kind) {
                Some(found) => found.sources.identity(),
                None => {
                    // Admission found every checker; a build that lost one stops the job as unavailable.
                    state.stop = Some(JobStop { code: codes::NO_CHECKER, at, phase: "admission" });
                    return;
                }
            };
            let mut child = ChildRun {
                at: at.clone(),
                kind: check.kind,
                checker: identity,
                certificate: sha256_hex(&check.certificate),
                requested,
                effective,
                used: Budget::default(),
                outcome: "completed",
                code: None,
                stop: None,
                report: None,
                output: None,
            };
            // The job's own budget first: what it lacks ends the job; what the check's caps lack ends the check.
            if let Some(code) = lacks(left, held) {
                state.stop = Some(JobStop { code, at, phase: "admission" });
                return;
            }
            if cancel.is_cancelled() {
                state.stop = Some(JobStop { code: codes::CANCELLED, at, phase: "admission" });
                return;
            }
            if let Some(code) = lacks(requested, held) {
                (child.outcome, child.code, child.stop) = ("cut_off", Some(code), Some("admission".into()));
                state.children.push(child);
                continue;
            }
            state.used.steps += 1;
            state.peak = state.peak.max(held);
            child.used.steps = 1;
            child.used.memory = held;

            let inputs: Vec<Input> =
                check.inputs.iter().map(|input| Input { role: &input.role, bytes: &input.bytes }).collect();
            let certificate = CertificateRef { digest: &check.stated, bytes: &check.certificate, inputs: &inputs };
            // The check's own hook enforces its own cap only. A cancellation is the job's: only the job's hook reads
            // it, so it always stops the job's scope, and every check inside it, at the same unit.
            let cap = effective.kernel_cost;
            let checked = check_until(engines, &certificate, check.kind, move |total| {
                (u128::from(total) > cap).then_some(codes::KERNEL_COST)
            });
            match checked {
                Ok(Checked::Report { report, charged }) => {
                    let charged = u128::from(charged);
                    state.used.kernel_cost += charged;
                    child.used.kernel_cost = charged;
                    let bytes = record::report_bytes(&report);
                    let size = bytes.len() as u128;
                    if size > left.output {
                        (child.outcome, child.code, child.stop) =
                            ("cut_off", Some(codes::OUTPUT), Some("output".into()));
                        state.children.push(child);
                        state.stop = Some(JobStop { code: codes::OUTPUT, at, phase: "output" });
                        return;
                    }
                    if size > requested.output {
                        (child.outcome, child.code, child.stop) =
                            ("cut_off", Some(codes::OUTPUT), Some("output".into()));
                    } else {
                        state.used.output += size;
                        child.used.output = size;
                        child.report = Some(report);
                        child.output = Some(state.outputs.len());
                        state.outputs.push(bytes);
                    }
                    state.children.push(child);
                }
                Ok(Checked::Stopped { code, charged }) => {
                    let charged = u128::from(charged);
                    state.used.kernel_cost += charged;
                    child.used.kernel_cost = charged;
                    (child.outcome, child.code, child.stop) =
                        (outcome_of(code), Some(code), Some(format!("unit {charged}")));
                    state.children.push(child);
                    // The job's own scope stopped too: the job ends here. Otherwise the check absorbed its stop.
                    if let Some(code) = claims::stopped() {
                        state.stop =
                            Some(JobStop { code, at: format!("{at} unit {}", state.used.kernel_cost), phase: "check" });
                        return;
                    }
                }
                Err(refusal) => {
                    (child.outcome, child.code) = ("error", Some(refusal.code));
                    state.children.push(child);
                }
            }
        }
    }
}

/// A stop position of the exact payload (DET.6): where, in which phase, in which scope.
fn position(at: &str, phase: &str, scope: &str) -> Json {
    Json::object([
        ("at", Json::text(at)),
        ("phase", Json::text(phase)),
        ("scope", Json::text(scope)),
        ("source", Json::Null),
    ])
}

/// One check's entry in the child ledger, in the fields of `persist::run::exact`'s.
fn child_json(child: &ChildRun) -> Json {
    let stop = child.stop.as_deref().map_or(Json::Null, |at| position(at, "check", &child.at));
    Json::object([
        ("at", Json::text(&child.at)),
        ("requested", record::amounts(&child.requested)),
        ("effective", record::amounts(&child.effective)),
        ("outcome", Json::text(child.outcome)),
        ("code", child.code.map_or(Json::Null, Json::text)),
        ("stop", stop),
        ("used", record::amounts(&child.used)),
        ("returned_memory", nat_text(0)),
        ("live_after_return", nat_text(0)),
    ])
}

impl CheckRun {
    pub fn record(&self) -> &Record {
        &self.record
    }

    /// The record's FP.5 bytes, as stored.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn key(&self) -> &Key {
        &self.record.key
    }

    /// The exact payload (DET.6).
    pub fn payload(&self) -> &Json {
        &self.record.result
    }

    /// `completed`, `cut_off` or `cancelled`.
    pub fn outcome(&self) -> &str {
        self.record.outcome()
    }

    pub fn code(&self) -> Option<&str> {
        self.record.code()
    }

    pub fn input(&self) -> &Pinned {
        &self.input
    }

    /// The ledger: one entry per check that started or was refused at its own admission.
    pub fn children(&self) -> &[ChildRun] {
        &self.children
    }

    /// The published reports' bytes, in order.
    pub fn outputs(&self) -> &[Vec<u8>] {
        &self.outputs
    }

    /// The job's value: present only when the job completed (KN7.3, KN7.11).
    pub fn value(&self) -> Option<&[u8]> {
        self.value.as_deref()
    }

    /// DET.30's reading of the job. A check stopped by its own cap is absorbed by the job, as RT1's `child` is: the job
    /// can still complete, and that check's entry in the value and the ledger keeps its cut-off, with no verdict and no
    /// report, so it adds nothing to evidence ([`crate::evidence::admit`]). Only the job's own stop makes the job's
    /// reading a cut-off or an open stop.
    pub fn reading(&self) -> Det30 {
        match reading(&self.record, &[]).coverage {
            Coverage::Complete => Det30::Complete,
            Coverage::WithinBudget => {
                let covered: u128 = self.children.iter().map(|child| child.used.kernel_cost).sum();
                if covered == 0 && self.outputs.is_empty() {
                    Det30::NotRun
                } else {
                    Det30::CutOff { covered }
                }
            }
            Coverage::Open(code) => Det30::Open(code),
        }
    }

    /// The bytes a reproduction needs, by SHA-256: the adapter's and checkers' source listings and the captured inputs.
    pub fn available(&self) -> BTreeMap<String, Vec<u8>> {
        let mut out = record::sources();
        for bytes in &self.captured {
            out.insert(sha256_hex(bytes), bytes.clone());
        }
        out
    }

    /// The job's reports as a reader at `now` may use them (KN7.3, KN7.5): only under the identity block the input was
    /// pinned under (`KN1:input.identity_mismatch`), only at the revision it was pinned at or one with an equal digest,
    /// such as after an undo, and in the same mode (`KN5:stale.revision_changed`), and only while each checker is
    /// available with the identity the key names (`KN6:evidence.no_checker`). A report whose checker read a start state
    /// other than the pinned one was about another revision and refuses the result as stale too; a report that read no
    /// state is the same at every revision. A check's static success never stands in for these run-time checks; nor
    /// does it stand in for a commit's own gates, protection among them.
    pub fn result_at(&self, now: &RevisionRef) -> Result<Vec<&CheckReport>, Refusal> {
        let refuse = |code, message: &str| Err(Refusal { code, message: message.into() });
        if identity_json(&now.identity) != self.input.identity {
            return refuse(codes::IDENTITY, "the job's input was pinned under another identity block");
        }
        let Some(pinned) = &self.input.revision else {
            return refuse(codes::STALE, "the job's input is a stated start, not a revision of a session");
        };
        if pinned.mode != now.mode || self.input.level != "labels" || self.input.digest != now.digest {
            return refuse(codes::STALE, "the job's input was pinned at another revision");
        }
        let elsewhere = |child: &ChildRun| child.report.as_ref().and_then(|r| self.input.bound_by(r)) == Some(false);
        if self.children.iter().any(elsewhere) {
            return refuse(codes::STALE, "a check read a start state other than the job's pinned revision");
        }
        for child in &self.children {
            let available = checker(child.kind).map(|found| found.sources.identity());
            if available.as_ref() != Some(&child.checker) {
                return refuse(codes::NO_CHECKER, "a checker the result relies on is not available with its identity");
            }
        }
        Ok(self.children.iter().filter_map(|child| child.report.as_ref()).collect())
    }
}
