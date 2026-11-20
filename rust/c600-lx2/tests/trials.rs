//! LX-2's metered and forced-kill trials against `c600-runtime`'s child built from this checkout, in the test's
//! profile. Source and synthetic evidence only: the decisive runs use the product build on the owner's machine.

mod common;

use std::path::Path;

use c600_lx2::{record_json, run_trial, Device, NativeFault, Observed, Record, Trial, RUNTIME_WORD};
use c600_runtime::{Fault, FaultKind};

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
    assert!(matches!(r.observed, Observed::Fault(FaultKind::Lost, _)));
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
    let killed = c600_lx2::observe_trial(Trial::AllocCeiling, Err(fault(FaultKind::Lost, "")));
    assert!(matches!(killed, Observed::Other(_)), "{killed:?}");
    assert_ne!(killed, Trial::AllocCeiling.expected());
}

/// A fault of `kind` whose child wrote `stderr`.
fn fault(kind: FaultKind, stderr: &str) -> Fault {
    Fault { kind, detail: "ended".into(), exit: Some(1), stderr: stderr.into() }
}

#[test]
fn a_native_fault_counts_only_with_the_devices_line_and_its_cause() {
    let line = |fault: NativeFault| format!("{} after 2 answers\n", fault.line());
    let overflow = "\nthread 'main' has overflowed its stack\n";
    let recursion = |stderr: &str| c600_lx2::observe_trial(Trial::RecursionNative, Err(fault(FaultKind::Lost, stderr)));
    let lost_by_overflow = recursion(&format!("{}{overflow}", line(NativeFault::Recursion)));
    assert_eq!(lost_by_overflow, Trial::RecursionNative.expected());
    for stderr in
        [String::new(), line(NativeFault::Recursion), overflow.to_string(), line(NativeFault::Loop) + overflow]
    {
        assert!(matches!(recursion(&stderr), Observed::Other(_)), "{stderr:?}");
    }
    let looped = |stderr: &str| c600_lx2::observe_trial(Trial::LoopNative, Err(fault(FaultKind::Deadline, stderr)));
    assert_eq!(looped(&line(NativeFault::Loop)), Trial::LoopNative.expected());
    assert_eq!(Trial::LoopNative.expected(), Observed::Fault(FaultKind::Deadline, "KN7:resource_limit.safety_stop"));
    assert!(matches!(looped(""), Observed::Other(_)), "a stop without the device's line is not the native loop");
    let lost = c600_lx2::observe_trial(Trial::LoopNative, Err(fault(FaultKind::Lost, &line(NativeFault::Loop))));
    assert_ne!(lost, Trial::LoopNative.expected(), "the native loop ends at the watchdog, not by a loss");
}

#[test]
fn the_native_faults_arm_after_the_proposals_answers_and_parse_back() {
    assert_eq!((c600_lx2::FAULT_VARIABLE, c600_lx2::FAULT_AFTER), ("C600_LX2_FAULT", 2));
    for (trial, fault) in [(Trial::RecursionNative, NativeFault::Recursion), (Trial::LoopNative, NativeFault::Loop)] {
        assert_eq!(trial.fault(), Some(fault));
        assert!(trial.native() && trial.faults());
        assert_eq!(NativeFault::parse(&fault.armed()), Some((fault, 2)));
        assert_eq!(trial.source(), Trial::Control.source(), "the job is the control's; the device faults it");
        let config = c600_lx2::config(Path::new("child"), trial);
        assert!(config.env.iter().any(|(name, value)| name == "C600_LX2_FAULT" && *value == *fault.armed()));
        assert!(config.env.iter().all(|(name, _)| name != c600_lx2::CEILING_VARIABLE));
    }
    assert_eq!(Trial::LoopNative.deadline(), c600_lx2::WATCHDOG);
    assert_eq!(Trial::RecursionNative.deadline(), c600_lx2::DEADLINE);
    for text in ["recursion", "loop@", "loop@x", "spin@2", ""] {
        assert_eq!(NativeFault::parse(text), None, "{text:?}");
    }
    assert!(Trial::ALL.iter().all(|t| t.fault().is_none() && !t.native()));
}

/// The native recursion and loop on the device's fault hook: they run only when the device is built with the hook.
#[cfg(feature = "device")]
#[test]
fn the_devices_native_recursion_overflows_its_stack_and_a_fresh_child_recovers() {
    let r = run_trial(&common::injection(), Trial::RecursionNative);
    assert!(r.agrees(), "{r:#?}");
    assert_eq!(r.observed, Observed::Fault(FaultKind::Lost, "KN7:resource_limit.runtime_lost"));
    assert_eq!((r.spawns, r.preview_kept, r.recovered, r.in_process), (2, true, true, None));
}

#[cfg(feature = "device")]
#[test]
fn the_devices_native_loop_meets_the_watchdog_and_a_fresh_child_recovers() {
    let started = std::time::Instant::now();
    let r = run_trial(&common::injection(), Trial::LoopNative);
    assert!(r.agrees(), "{r:#?}");
    assert!(started.elapsed() >= c600_lx2::WATCHDOG, "the run ended at the watchdog");
    assert_eq!((r.spawns, r.preview_kept, r.recovered, r.in_process), (2, true, true, None));
}

/// Without the feature the device has no hook, so it refuses an armed native fault before it announces itself rather
/// than run the job unfaulted.
#[cfg(not(feature = "device"))]
#[test]
fn a_device_without_the_hook_refuses_an_armed_native_fault() {
    for trial in [Trial::RecursionNative, Trial::LoopNative] {
        let r = run_trial(&common::injection(), trial);
        assert!(!r.agrees(), "{r:#?}");
        assert!(matches!(r.observed, Observed::Fault(..)), "{r:#?}");
        assert!(!r.preview_kept && !r.recovered, "{r:#?}");
    }
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
    assert!(json.contains("\"fault_variable\": \"C600_LX2_FAULT\", \"fault_after_answers\": 2"));
    assert!(json.contains("\"deadline_ms\": 60000, \"watchdog_ms\": 3000"));
    assert!(!json.contains(":\\\\") && !json.contains("/home/"));
    assert_eq!((Trial::ALL.len(), Trial::NATIVE.len()), (5, 3));
    assert!(Trial::RecursionMetered.source().contains("down(exact_add(n, 1))"));
    assert!(Trial::ALL.iter().all(|t| t.job().args.len() == 1) && RUNTIME_WORD == [7, -11, 1200]);
}
