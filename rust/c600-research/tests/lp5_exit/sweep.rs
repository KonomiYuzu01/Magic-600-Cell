//! The sweep: LB5.14's preflight of every group, the checks of the ready groups, each run once on a pool of threads,
//! and the summary.

#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use c600_archive::bindings::{Availability, Binding, CheckedAs, Reason};
use c600_checkers::{check, CertificateRef, CheckReport, Input, Verdict};

use crate::matching::{identity_problems, match_report, meaning, Meaning};
use crate::roles::{prepare, Prepared};
use crate::support::{bindings, engines, Checkout};

/// Every group's preflight answer, with the registry and the checkout as host, in the edition's row order.
pub fn preflight() -> &'static [Availability] {
    static ANSWERS: OnceLock<Vec<Availability>> = OnceLock::new();
    ANSWERS.get_or_init(|| {
        let b = bindings();
        b.rows().iter().map(|row| b.check_availability(&row.group, &Checkout)).collect()
    })
}

/// One check of a ready group: a claim checker the binding names, with how the record enters it.
#[derive(Clone, Debug)]
pub struct Job {
    pub row: usize,
    pub checker: String,
    pub how: CheckedAs,
}

impl Job {
    pub fn binding(&self) -> &'static Binding {
        &bindings().rows()[self.row]
    }

    pub fn record(&self) -> &'static str {
        &bindings().file(self.binding().source).path
    }

    pub fn name(&self) -> String {
        format!("{} under {}", self.binding().group, self.checker)
    }
}

/// The checks whose replay is too long for a normal test run, each with the ignored test that runs it, its reason and
/// its rough time in a release build. Every other check of a ready group runs in the fast sweep.
pub const SLOW: [(&str, &str, &str); 3] = [
    (
        "s18.composed",
        "18-tail-completion/results/d1-tail/steps.json",
        "gate 2 of T6's four steps, 1,510,756 letters played one by one",
    ),
    (
        "s18.composed",
        "18-tail-completion/results/rule-stop/steps.json",
        "gate 2 of the rule's three stages, 688,686 letters played one by one",
    ),
    (
        "s18.lattice_replay",
        "18-tail-completion/results/lattice-d1/run/run.json",
        "the lattice phase, 13,231,266 letters played one by one",
    ),
];

/// The slow entry for a job, if it is one.
pub fn slow(job: &Job) -> Option<&'static (&'static str, &'static str, &'static str)> {
    SLOW.iter().find(|(checker, record, _)| job.checker == *checker && job.record().ends_with(record))
}

/// One representative record of each stage 14 to 17 sub-checker, which the fast sweep checks. These sub-checkers
/// recompute from the stage 13 tables with full label tables, between about 3 s and 2 minutes a check in a debug
/// build (the workspace optimizes the engines, not the checkers); their other checks are the stage 14 to 17 sweep.
pub const REPRESENTATIVES: [(&str, &str); 11] = [
    ("s14.t3p", "14-blocking-structure/results/s7304-w3-certificate.json"),
    ("s14.literal", "14-blocking-structure/results/s7304-w5-replay.json"),
    ("s14.locality", "14-blocking-structure/results/locality-summary.json"),
    ("s14.levels", "14-blocking-structure/results/s7304-levels.json"),
    ("s14.blocks", "14-blocking-structure/results/s7304-coheld.json"),
    ("s15.macro", "15-lowering-machine/results/runs/dis1-r05.certificate.json"),
    ("s15.pstar", "15-lowering-machine/results/census-7304.json"),
    ("s16.macro", "16-six-jams/results/certificates/SJ-D1.json"),
    ("s16.release", "16-six-jams/results/certificates/SJ-E1-trace.json"),
    ("s16.mobility", "16-six-jams/results/mobility/7304.json"),
    ("s17.macro", "17-routing-in-driver/results/certificates/RR-7304.json"),
];

/// Which test runs a check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// The fast sweep, in a normal test run: every stage 18 and 19 check but the long replays, and one representative
    /// of each stage 14 to 17 sub-checker.
    Fast,
    /// The stage 14 to 17 sweep, an ignored test: every other check of those stages.
    Stages14To17,
    /// One of the long replays, each an ignored test.
    Slow,
}

pub fn part(job: &Job) -> Part {
    if slow(job).is_some() {
        Part::Slow
    } else if job.binding().stage >= 18
        || REPRESENTATIVES.iter().any(|(c, r)| job.checker == *c && job.record().ends_with(r))
    {
        Part::Fast
    } else {
        Part::Stages14To17
    }
}

/// Every check of every ready group, in row order.
pub fn ready_jobs() -> Vec<Job> {
    let b = bindings();
    let mut jobs = Vec::new();
    for (row, answer) in b.rows().iter().zip(preflight()) {
        if let Availability::Ready { checkers, .. } = answer {
            let how = row.checked_as.expect("a ready group names how its record enters");
            for c in checkers {
                jobs.push(Job { row: row.index, checker: c.id.clone(), how });
            }
        }
    }
    jobs
}

/// The scope-only reads: every group whose named sub-checkers read the record for its scope only.
pub fn scope_only_jobs() -> Vec<Job> {
    bindings()
        .rows()
        .iter()
        .filter(|row| row.checked_as == Some(CheckedAs::ScopeOnly))
        .flat_map(|row| {
            row.checkers.iter().map(|c| Job { row: row.index, checker: c.clone(), how: CheckedAs::ScopeOnly })
        })
        .collect()
}

/// A finished check.
pub struct Done {
    pub job: Job,
    pub report: Result<Arc<CheckReport>, String>,
    /// The wall time of the check when it ran; a check shared with an earlier job reports the time it took then.
    pub seconds: f64,
    /// Whether the report was shared with an earlier job with the same checker, certificate and inputs.
    pub shared: bool,
    /// Bound files the check did not read: (the binding's role, path).
    pub unread: Vec<(String, String)>,
    pub meaning: Result<Meaning, String>,
    pub problems: Vec<String>,
}

type Cell = Arc<OnceLock<(Result<Arc<CheckReport>, String>, f64)>>;

/// Runs a prepared check once: a check whose sub-checker, certificate and inputs are byte for byte one that already ran
/// is the same deterministic check, and its report is shared. Returns the report, the time and whether it was shared.
fn run_once(prepared: &Prepared) -> (Result<Arc<CheckReport>, String>, f64, bool) {
    static CACHE: OnceLock<Mutex<HashMap<String, Cell>>> = OnceLock::new();
    let cell = {
        let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
        Arc::clone(cache.entry(prepared.key()).or_default())
    };
    let mut ran = false;
    let (report, seconds) = cell.get_or_init(|| {
        ran = true;
        let started = Instant::now();
        let inputs: Vec<Input> =
            prepared.inputs.iter().map(|(role, bytes)| Input { role, bytes: bytes.as_slice() }).collect();
        let certificate =
            CertificateRef { digest: &prepared.digest, bytes: prepared.certificate.as_slice(), inputs: &inputs };
        let report = check(&engines(), &certificate, prepared.kind()).map(Arc::new).map_err(|r| r.to_string());
        (report, started.elapsed().as_secs_f64())
    });
    (report.clone(), *seconds, !ran)
}

/// Runs one job: prepares its check from the binding, runs it, reads the record's meaning and matches the report.
pub fn run_job(job: &Job) -> Done {
    let b = bindings();
    let row = job.binding();
    let prepared = prepare(b, row, &job.checker, job.how);
    let (report, seconds, shared) = run_once(&prepared);
    let meaning = meaning(b, row);
    let mut problems = Vec::new();
    match &report {
        Err(refusal) => problems.push(format!("refused: {refusal}")),
        Ok(report) if job.how == CheckedAs::ScopeOnly => {
            if !matches!(report.verdict, Verdict::NotApplicable { .. }) {
                problems.push(format!("a scope-only read answered {:?}", report.verdict));
            }
            problems.extend(identity_problems(&prepared, report));
        }
        Ok(report) => {
            if !report.complete() {
                let last = report.trace.last().map(|s| format!("{} = {}", s.claim, s.computed));
                problems.push(format!("{:?}; last trace step {last:?}", report.verdict));
            }
            match &meaning {
                Ok(m) => problems.extend(match_report(b, row, &prepared, report, m)),
                Err(e) => problems.push(e.clone()),
            }
            // Derived inputs are rebuilt by the check, never read: each named start is rebuilt to its stated digest.
            let rebuilds = matches!(job.checker.as_str(), "s18.composed" | "s18.tail_replay");
            for derived in row.derived.iter().filter(|_| rebuilds) {
                let k = derived.pointer.trim_start_matches("/steps/");
                let claim = format!("steps[{k}].start");
                let rebuilt = report.trace.iter().any(|s| {
                    s.claim == claim && s.held && s.computed.starts_with(&format!("derived: {}", derived.digest))
                });
                if !rebuilt {
                    problems.push(format!("{claim} is not rebuilt to {}", derived.digest));
                }
            }
        }
    }
    Done { job: job.clone(), report, seconds, shared, unread: prepared.unread, meaning, problems }
}

fn panicked(job: &Job, panic: &(dyn std::any::Any + Send)) -> Done {
    let message = panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "a panic".to_owned());
    Done {
        job: job.clone(),
        report: Err(message.clone()),
        seconds: 0.0,
        shared: false,
        unread: Vec::new(),
        meaning: Err(message.clone()),
        problems: vec![format!("panicked: {message}")],
    }
}

/// Runs jobs on a pool of threads, in no fixed order; the results come back in the jobs' order.
pub fn run_all(jobs: &[Job]) -> Vec<Done> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(jobs.len().max(1));
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Done>>> = Mutex::new((0..jobs.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let at = next.fetch_add(1, Ordering::SeqCst);
                let Some(job) = jobs.get(at) else { break };
                // A job that panics (a bound file the checkout lacks, say) is a problem of that job, so the sweep
                // still reports every other job.
                let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_job(job)))
                    .unwrap_or_else(|panic| panicked(job, panic.as_ref()));
                results.lock().unwrap()[at] = Some(done);
            });
        }
    });
    results.into_inner().unwrap().into_iter().map(|d| d.expect("every job ran")).collect()
}

/// The problems of finished jobs, one line each, for an assertion message.
pub fn problems(done: &[Done]) -> Vec<String> {
    done.iter()
        .filter(|d| !d.problems.is_empty())
        .map(|d| format!("{}: {}", d.job.name(), d.problems.join("; ")))
        .collect()
}

/// One sub-checker's share of a sweep.
#[derive(Default)]
pub struct Tally {
    pub checks: usize,
    pub passed: usize,
    /// Reports shared with an earlier job with the same checker, certificate and inputs.
    pub shared: usize,
    /// The wall seconds of the checks that ran, summed.
    pub seconds: f64,
    /// The quickest and the slowest check that ran, in seconds.
    pub quickest: f64,
    pub slowest: f64,
}

/// Per sub-checker tallies.
pub fn by_checker(done: &[Done]) -> BTreeMap<String, Tally> {
    let mut out: BTreeMap<String, Tally> = BTreeMap::new();
    for d in done {
        let entry = out.entry(d.job.checker.clone()).or_insert(Tally { quickest: f64::MAX, ..Tally::default() });
        entry.checks += 1;
        if d.problems.is_empty() {
            entry.passed += 1;
        }
        if d.shared {
            entry.shared += 1;
        } else {
            entry.seconds += d.seconds;
            entry.quickest = entry.quickest.min(d.seconds);
            entry.slowest = entry.slowest.max(d.seconds);
        }
    }
    out
}

/// The unavailable answers' reasons, counted once per group and reason.
pub fn reasons(answers: &[Availability]) -> BTreeMap<&'static str, usize> {
    let mut out = BTreeMap::new();
    for a in answers {
        for r in a.reasons() {
            *out.entry(r.word()).or_default() += 1;
        }
    }
    out
}

/// The unavailable answers by their whole reason set.
pub fn reason_sets(answers: &[Availability]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for a in answers.iter().filter(|a| !a.is_ready()) {
        let words: Vec<&str> = a.reasons().iter().map(Reason::word).collect();
        *out.entry(words.join(", ")).or_default() += 1;
    }
    out
}
