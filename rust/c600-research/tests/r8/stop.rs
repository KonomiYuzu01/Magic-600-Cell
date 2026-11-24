//! The job's cooperative stop (KN7.6, KN7.10, DET.30): the job's budget reaches the checker as a stop, so a long
//! expansion ends as the job's cut-off with the budget's code, never as a verdict; a cancellation ends a job as an open
//! stop that reproduces as `not run`; output and kernel-cost limits of zero leave nothing examined.

use c600_research::job::codes;
use c600_research::{Budget, Cancel, Det30};
use c600_session::persist::run::Reproduction;

use crate::common::*;

/// A 2·10^12-letter expansion with a wrong claimed digest under a kernel cost of 50,000 is cut off at that unit with
/// the budget's code, has no verdict and no value, reads `not found within budget` and reproduces byte for byte.
#[test]
fn a_long_expansion_is_cut_off_and_never_a_verdict() {
    let mut job = stated_job(vec![long_check(1_000_000, 1_000_000)], WIDE);
    job.budget.kernel_cost = 50_000;
    let run = run_job(job);
    assert_eq!((run.outcome(), run.code()), ("cut_off", Some(codes::KERNEL_COST)));
    assert_eq!(run.reading(), Det30::CutOff { covered: 50_000 });
    assert!(run.children()[0].report.is_none() && run.value().is_none() && run.outputs().is_empty());
    let again = reproduce(&run);
    assert_eq!(again.answer, Reproduction::Same(run.payload().clone()));
    assert!(again.run.is_some());
}

/// A job with no kernel cost, or no step, examines nothing: `not run`, never `Empty`.
#[test]
fn a_job_with_nothing_to_spend_examines_nothing() {
    for budget in [Budget { kernel_cost: 0, ..WIDE }, Budget { steps: 0, ..WIDE }] {
        let run = run_job(stated_job(vec![short_check(&[1, 2])], budget));
        assert_eq!(run.outcome(), "cut_off", "{budget:?}");
        assert_eq!(run.reading(), Det30::NotRun, "{budget:?}");
    }
}

/// A cancellation is an open stop: the job reads `cancelled`, establishes nothing, and reproduces as `not run` with
/// its code. A cancellation that lands while the checker runs stops the job at the letter it reaches.
#[test]
fn a_cancelled_job_is_open_and_reproduces_as_not_run() {
    let admitted = admit(stated_job(vec![short_check(&[1, 2]), short_check(&[3])], WIDE));
    let cancel = Cancel::new();
    cancel.cancel();
    let run = admitted.run(&engines(), &cancel).expect("the record is written");
    assert_eq!((run.outcome(), run.code()), ("cancelled", Some(codes::CANCELLED)));
    assert_eq!(run.reading(), Det30::Open(codes::CANCELLED.into()));
    assert_eq!(reproduce(&run).answer, Reproduction::NotRun(codes::CANCELLED.into()));

    // Mid-run: an enclosing scope's hook cancels at unit 1,000, and the job's own hook reads the cancellation at that
    // same unit, so the check stops after 999 units, with no verdict, and the job ends cancelled.
    let admitted = admit(stated_job(vec![long_check(1_000, 1_000), short_check(&[4])], WIDE));
    let cancel = Cancel::new();
    let trigger = cancel.clone();
    let outer = c600_checkers::claims::scoped(
        move |total| {
            if total >= 1_000 {
                trigger.cancel();
            }
            None
        },
        || admitted.run(&engines(), &cancel).expect("the record is written"),
    )
    .expect("a scope");
    let run = outer.value;
    assert_eq!((run.outcome(), run.code()), ("cancelled", Some(codes::CANCELLED)));
    assert_eq!(run.children().len(), 1, "the job ends at the check the cancellation reached");
    let child = &run.children()[0];
    assert_eq!((child.outcome, child.used.kernel_cost), ("cancelled", 999));
    assert!(child.report.is_none() && run.value().is_none());

    // A job that would never end is stopped by a cancellation from another thread (at its admission or at a letter,
    // whichever the cancellation reaches first).
    let admitted = admit(stated_job(vec![long_check(1_000_000, 1_000_000)], WIDE));
    let cancel = Cancel::new();
    let remote = cancel.clone();
    let running = std::thread::spawn(move || admitted.run(&engines(), &cancel).expect("the record is written"));
    remote.cancel();
    let run = running.join().expect("the job ends");
    assert_eq!((run.outcome(), run.code()), ("cancelled", Some(codes::CANCELLED)));
    assert!(run.children().iter().all(|child| child.report.is_none()));
}

/// Reproducing a completed record under a cancellation is `not run` with the cancellation's code, never a mismatch.
#[test]
fn a_cancelled_reproduction_is_not_run() {
    let completed = run_job(stated_job(vec![short_check(&[1, 2])], WIDE));
    let cancel = Cancel::new();
    cancel.cancel();
    let answer = c600_research::reproduce(&engines(), completed.bytes(), &completed.available(), &cancel);
    assert_eq!(answer.answer, Reproduction::NotRun(codes::CANCELLED.into()));
    assert!(answer.run.is_none());
    assert!(matches!(reproduce(&completed).answer, Reproduction::Same(_)), "without the cancellation it reproduces");
}

/// The output limit: a report the job cannot hold ends the job with `KN7:budget.output`; a report only the check's
/// own cap cannot hold cuts off that check, and the job completes.
#[test]
fn output_limits_bound_the_published_reports() {
    let mut job = stated_job(vec![short_check(&[1, 2])], WIDE);
    job.budget.output = 10;
    let run = run_job(job);
    assert_eq!((run.outcome(), run.code()), ("cut_off", Some(codes::OUTPUT)));
    assert!(run.outputs().is_empty() && run.value().is_none());

    let mut capped = short_check(&[1, 2]);
    capped.cap = Some(Budget { output: 10, ..WIDE });
    let run = run_job(stated_job(vec![capped, short_check(&[3])], WIDE));
    assert_eq!(run.outcome(), "completed");
    assert_eq!((run.children()[0].outcome, run.children()[0].code), ("cut_off", Some(codes::OUTPUT)));
    assert_eq!(run.outputs().len(), 1, "only the second check's report is published");
    assert_eq!(run.children()[1].output, Some(0));
}
