//! LX-2's metered and forced-kill trials against `c600-runtime`'s child built from this checkout, in the test's
//! profile. Source and synthetic evidence only: the decisive runs use the product build on the owner's machine.

mod common;

use c600_lx2::{record_json, run_trial, Observed, Record, Trial, RUNTIME_WORD};

fn trial(t: Trial) -> Record {
    let record = run_trial(&common::runtime(), t);
    assert!(record.agrees(), "{record:#?}");
    record
}

#[test]
fn the_control_completes_in_one_child_and_in_process_alike() {
    let r = trial(Trial::Control);
    assert_eq!((r.observed, r.spawns, r.in_process), (Observed::Completed, 1, Some(Observed::Completed)));
}

#[test]
fn metered_faults_end_with_the_meters_code_and_the_child_serves_the_next_job() {
    let steps = trial(Trial::LoopMetered);
    assert_eq!(steps.observed, Observed::CutOff("KN7:budget.steps"));
    let memory = trial(Trial::RecursionMetered);
    assert_eq!(memory.observed, Observed::CutOff("KN7:budget.memory"));
    let alloc = trial(Trial::AllocMetered);
    assert_eq!(alloc.observed, Observed::CutOff("KN7:budget.memory"));
    for r in [steps, memory, alloc] {
        assert_eq!((r.spawns, r.preview_kept, r.recovered), (1, true, true), "{r:?}");
    }
}

#[test]
fn a_forced_kill_at_the_barrier_is_a_typed_loss_and_a_fresh_child_recovers() {
    let r = trial(Trial::ForcedKill);
    assert!(matches!(r.observed, Observed::Fault(c600_runtime::FaultKind::Lost, _)));
    assert_eq!((r.spawns, r.preview_kept, r.recovered, r.in_process), (2, true, true, None));
}

#[test]
fn a_record_that_disagrees_says_so_and_the_json_names_no_path() {
    let mut wrong = run_trial(&common::runtime(), Trial::Control);
    wrong.recovered = false;
    assert!(!wrong.agrees());
    let mut other = wrong.clone();
    other.recovered = true;
    other.observed = Observed::CutOff("KN7:budget.steps");
    assert!(!other.agrees(), "an unexpected outcome disagrees");
    let json = record_json("c600-runtime.exe", "00", &[wrong], &[]);
    assert!(json.contains("\"agrees\": false") && json.contains("\"file\": \"c600-runtime.exe\""));
    assert!(!json.contains(":\\\\") && !json.contains("/home/"));
    assert_eq!(Trial::ALL.len(), 5);
    assert!(Trial::RecursionMetered.source().contains("down(exact_add(n, 1))"));
    assert!(Trial::ALL.iter().all(|t| t.job().args.len() == 1) && RUNTIME_WORD == [7, -11, 1200]);
}
