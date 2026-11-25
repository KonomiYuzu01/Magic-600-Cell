//! CT-DET-11 (part: checks nested in a job and in other scopes, and a job's child checks). The LP2 half
//! (`c600-run/tests/child_scopes.rs`, `conformance_det.rs`) ran RT1's `child` for interpreter runs; its "not run"
//! part, checks nested in a program and child jobs, runs here and in `c600-checkers/tests/kn6_stop.rs` (a check
//! nested in a stop scope). A check's requested caps are cut to what the job has left, a check that asks for more than
//! is left is stopped where the job runs out and ends the job, a check's own cap ends that check only, and the ledger
//! reconciles: the job's counters are the sum of its checks', each within its effective caps.

use c600_checkers::claims;
use c600_research::job::codes;
use c600_research::{Budget, Cancel, ChildRun, Det30};

use crate::common::*;

fn ledger_reconciles(run: &c600_research::CheckRun) {
    let used =
        |name: &str| run.payload().get("used").and_then(|u| u.get(name)).and_then(|v| v.as_text()).map(str::to_owned);
    let children: &[ChildRun] = run.children();
    let sum = |f: fn(&Budget) -> u128| children.iter().map(|c| f(&c.used)).sum::<u128>();
    assert_eq!(used("kernel_cost"), Some(sum(|b| b.kernel_cost).to_string()));
    assert_eq!(used("steps"), Some(sum(|b| b.steps).to_string()));
    let peak = children.iter().map(|c| c.used.memory).max().unwrap_or(0);
    assert_eq!(used("memory"), Some(peak.to_string()), "the job's memory is its peak");
    for child in children {
        assert!(child.used.kernel_cost <= child.effective.kernel_cost, "{child:?}");
        assert!(child.used.steps <= child.effective.steps, "{child:?}");
        assert!(child.used.output <= child.effective.output, "{child:?}");
        assert_eq!(child.effective, child.requested.min(child.effective), "effective caps never exceed the request");
    }
}

/// CT-DET-11 (part: a check that requests more than the job has left gets the remainder; when it runs out it is
/// stopped there, with the job's code, and the job ends: the check minted no allowance).
#[test]
fn ct_det_11_a_check_cannot_run_past_what_the_job_has_left() {
    let mut greedy = long_check(1_000, 1_000);
    greedy.cap = Some(WIDE);
    let mut job = stated_job(vec![short_check(&[1, 2, 3, 4]), greedy, short_check(&[5])], WIDE);
    job.budget.kernel_cost = 1_000;
    let run = run_job(job);
    let children = run.children();
    assert_eq!(children.len(), 2, "the job ends at the greedy check");
    assert_eq!(children[1].requested.kernel_cost, WIDE.kernel_cost);
    assert_eq!(children[1].effective.kernel_cost, 996, "the remainder after the first check's four letters");
    assert_eq!((children[1].outcome, children[1].code), ("cut_off", Some(codes::KERNEL_COST)));
    assert_eq!(children[1].used.kernel_cost, 996);
    assert_eq!((run.outcome(), run.code()), ("cut_off", Some(codes::KERNEL_COST)));
    assert_eq!(run.reading(), Det30::CutOff { covered: 1_000 });
    ledger_reconciles(&run);
}

/// CT-DET-11 (part: a check's own cap ends that check only; the job absorbs the stop, goes on to its next check and
/// completes, and the value records the stopped check without a verdict).
#[test]
fn ct_det_11_a_checks_own_cap_ends_that_check_only() {
    let mut capped = long_check(1_000, 1_000);
    capped.cap = Some(Budget { kernel_cost: 300, ..WIDE });
    let run = run_job(stated_job(vec![capped, short_check(&[1, 2, 3])], WIDE));
    let children = run.children();
    assert_eq!((children[0].outcome, children[0].code), ("cut_off", Some(codes::KERNEL_COST)));
    assert_eq!(children[0].used.kernel_cost, 300);
    assert_eq!(children[0].stop.as_deref(), Some("unit 300"));
    assert!(children[0].report.is_none(), "a stop is no verdict");
    assert_eq!((children[1].outcome, children[1].used.kernel_cost), ("completed", 3));
    assert_eq!(run.outcome(), "completed");
    let value = String::from_utf8(run.value().expect("a completed job's value").to_vec()).expect("UTF-8");
    assert!(value.contains("\"outcome\":\"cut_off\",\"report\":null,\"verdict\":null"), "{value}");
    ledger_reconciles(&run);
}

/// CT-DET-11 (part: memory and steps: a check that holds more than the job's memory, or a job with no step left, ends
/// the job at the check's admission; a check whose own memory cap is too small is cut off there and the job goes on).
#[test]
fn ct_det_11_memory_and_steps_are_admitted_per_check() {
    let small = short_check(&[1]);
    let held = small.certificate.len() as u128;

    let mut job = stated_job(vec![small.clone(), small.clone()], WIDE);
    job.budget.memory = held - 1;
    let run = run_job(job);
    assert_eq!((run.outcome(), run.code()), ("cut_off", Some(codes::MEMORY)));
    assert!(run.children().is_empty());
    assert_eq!(run.reading(), Det30::NotRun, "no check ran: nothing was examined");

    let mut job = stated_job(vec![small.clone(), small.clone()], WIDE);
    job.budget.steps = 1;
    let run = run_job(job);
    assert_eq!((run.outcome(), run.code()), ("cut_off", Some(codes::STEPS)));
    assert_eq!(run.children().len(), 1);
    assert_eq!(run.reading(), Det30::CutOff { covered: 1 });

    let mut tight = small.clone();
    tight.cap = Some(Budget { memory: held - 1, ..WIDE });
    let run = run_job(stated_job(vec![tight, small], WIDE));
    assert_eq!(run.outcome(), "completed");
    let children = run.children();
    assert_eq!(
        (children[0].outcome, children[0].code, children[0].stop.as_deref()),
        ("cut_off", Some(codes::MEMORY), Some("admission"))
    );
    assert_eq!(children[0].used, Budget::default(), "a check refused at its admission uses nothing");
    assert_eq!(children[1].outcome, "completed");
    ledger_reconciles(&run);
}

/// CT-DET-11 (part: a job nested in an enclosing stop scope, as a job started inside another job or program would be,
/// is charged in that scope too and stops where it runs out, with its code).
#[test]
fn ct_det_11_a_nested_job_cannot_mint_allowance() {
    let admitted = admit(stated_job(vec![long_check(1_000, 1_000)], WIDE));
    let outer = claims::scoped(
        |total| (total > 700).then_some(codes::STEPS),
        || admitted.run(&engines(), &Cancel::new()).expect("the record is written"),
    )
    .expect("a scope");
    let run = outer.value;
    assert_eq!((outer.charged, outer.stopped), (700, Some(codes::STEPS)));
    assert_eq!((run.outcome(), run.code()), ("cut_off", Some(codes::STEPS)));
    assert_eq!(run.children()[0].used.kernel_cost, 700);
    assert_eq!(claims::stopped(), None, "no scope is left open");
}
