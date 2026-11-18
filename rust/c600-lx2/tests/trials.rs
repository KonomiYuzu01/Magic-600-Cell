//! LX-2's metered and forced-kill trials against `c600-runtime`'s child built from this checkout, in the test's
//! profile. Source and synthetic evidence only: the decisive runs use the product build on the owner's machine.

mod common;

use c600_lx2::{record_json, run_trial, Device, Observed, Record, Trial, RUNTIME_WORD};

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
fn the_injection_childs_ceiling_ends_the_child_with_a_typed_loss_and_a_fresh_child_recovers() {
    let r = run_trial(&common::injection(), Trial::AllocCeiling);
    assert!(r.agrees(), "{r:#?}");
    assert_eq!((r.spawns, r.preview_kept, r.recovered, r.in_process), (2, true, true, None));
}

#[test]
fn a_child_lost_without_the_allocation_failure_does_not_pass_for_the_ceiling() {
    let killed = c600_lx2::observe_trial(
        Trial::AllocCeiling,
        Err(c600_runtime::Fault {
            kind: c600_runtime::FaultKind::Lost,
            detail: "killed".into(),
            exit: Some(1),
            stderr: String::new(),
        }),
    );
    assert!(matches!(killed, Observed::Other(_)), "{killed:?}");
    assert_ne!(killed, Trial::AllocCeiling.expected());
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
    let json = record_json("c600-runtime.exe", "00", None, &[wrong.clone()], &[]);
    assert!(json.contains("\"agrees\": false") && json.contains("\"file\": \"c600-runtime.exe\""));
    assert!(json.contains("\"injection_device\": null") && json.contains("\"child\": \"product\""));
    let device = Device { file: "c600-lx2-child.exe".into(), sha256: "11".into() };
    let json = record_json("c600-runtime.exe", "00", Some(&device), &[wrong], &[]);
    assert!(json.contains("\"injection_device\": {\"file\": \"c600-lx2-child.exe\", \"sha256\": \"11\""));
    assert!(json.contains("\"source\": \"product source + injection device\""));
    assert!(!json.contains(":\\\\") && !json.contains("/home/"));
    assert_eq!(Trial::ALL.len(), 5);
    assert!(Trial::RecursionMetered.source().contains("down(exact_add(n, 1))"));
    assert!(Trial::ALL.iter().all(|t| t.job().args.len() == 1) && RUNTIME_WORD == [7, -11, 1200]);
}
