//! LX-2's session-level conditions (RTE.42 conditions 1 and 2) against `c600-runtime`'s child built from this
//! checkout, in the test's profile, each with its session in a fresh folder under Cargo's temporary directory. Source and
//! synthetic evidence only.

mod common;

use c600_check::Profile;
use c600_lx2::session::{self, expected_command, run_session_trial, SessionRecord};
use c600_lx2::{Observed, Trial, RUNTIME_WORD};
use c600_tools::process;
use c600_tools::tool::class_rows;

/// Runs `trial` with its session in a fresh folder named for the test.
fn run(test: &str, trial: Trial) -> SessionRecord {
    let folder = common::fresh(&format!("{test}-{}", trial.name()));
    let child = if trial.native() { common::injection() } else { common::runtime() };
    run_session_trial(&child, &common::kernel(), &folder, trial)
}

fn control(test: &str) -> SessionRecord {
    let control = run(test, Trial::Control);
    assert!(control.agrees(&control), "{control:#?}");
    control
}

/// Each trial's session job is the process host's job for the trial's tool (`c600_tools::process::job`): the solving
/// profile and the tool's class rows, so that the child admits the entry in the built-in column, with the trial's
/// limits and no summary rows, and every other part the host's. The tests below run it in the child.
#[test]
fn the_session_job_is_the_process_hosts_job_with_the_trials_limits() {
    for trial in Trial::ALL.into_iter().chain(Trial::NATIVE) {
        let (tool, source) = session::tool(trial).expect("the built-in column admits the trial's program");
        let job = session::job(trial, &tool, &source);
        assert_eq!(job.options.profile, Profile::Solving, "{trial:?}");
        assert_eq!(job.options.built_in, Some(class_rows(&tool.manifest)), "{trial:?}");
        assert_eq!((job.options.limits, job.options.record), (trial.limits(), false), "{trial:?}");
        let host = process::job(&tool, &source, &RUNTIME_WORD);
        let mut options = host.options.clone();
        options.limits = trial.limits();
        options.record = false;
        assert_eq!(job.options, options, "{trial:?}: every other option is the host's");
        assert_eq!((&job.sources, &job.module, &job.entry), (&host.sources, &host.module, &host.entry), "{trial:?}");
        assert_eq!(format!("{:?}", job.args), format!("{:?}", host.args), "{trial:?}: the runtime word");
    }
}

#[test]
fn the_control_commits_three_words_and_a_reopen_replays_its_state() {
    let c = control("control");
    assert_eq!((c.observed.clone(), c.open, c.spawns), (Observed::Completed, true, 1));
    assert_eq!(c.command, "completed", "the solver's command gets the program's fixed acknowledgement");
    assert_eq!(c.replayed.as_deref(), Some(c.state.as_str()));
    assert!(c.journal.len() >= 3, "three commits are journaled: {:?}", c.journal);
}

#[test]
fn each_fault_leaves_the_journal_and_the_state_of_the_control_and_the_session_open() {
    let c = control("faults");
    for trial in
        [Trial::LoopMetered, Trial::RecursionMetered, Trial::AllocMetered, Trial::ForcedKill, Trial::AllocCeiling]
    {
        let r = run("faults", trial);
        let differs: Vec<_> = r.journal.iter().zip(&c.journal).filter(|(a, b)| a != b).take(2).collect();
        assert!(
            r.agrees(&c),
            "{trial:?}: {:?} open {} state {} replayed {:?}; rows {} vs {}; first differences {differs:#?}",
            r.observed,
            r.open,
            r.state == c.state,
            r.replayed.as_deref() == Some(r.state.as_str()),
            r.journal.len(),
            c.journal.len()
        );
        assert_eq!(r.spawns, 1, "{trial:?}: the job ran in the first child");
        assert_eq!(r.command, expected_command(trial), "{trial:?}: the solver's command fails with the run's code");
        assert!(r.command.starts_with("failed "), "{trial:?}: {}", r.command);
    }
}

/// The native recursion and loop with a session open, on the device's fault hook (feature `device`).
#[cfg(feature = "device")]
#[test]
fn each_native_fault_on_the_hook_leaves_the_journal_and_the_state_of_the_control_and_the_session_open() {
    let c = control("native");
    for trial in [Trial::RecursionNative, Trial::LoopNative] {
        let r = run("native", trial);
        assert!(r.agrees(&c), "{trial:?}: {r:#?}");
        assert_eq!(r.spawns, 1, "{trial:?}: the job ran in the first child");
        assert_eq!(r.command, expected_command(trial), "{trial:?}: the solver's command fails with the run's code");
    }
    assert_eq!(expected_command(Trial::LoopNative), "failed KN7:resource_limit.safety_stop");
}

#[test]
fn a_record_that_differs_from_the_control_disagrees() {
    let c = control("disagree");
    let mut other = c.clone();
    other.journal.pop();
    assert!(!other.agrees(&c));
    let mut closed = c.clone();
    closed.open = false;
    assert!(!closed.agrees(&c));
    let mut unreplayed = c.clone();
    unreplayed.replayed = None;
    assert!(!unreplayed.agrees(&c));
    let mut unreported = c.clone();
    unreported.command = "not reported".into();
    assert!(!unreported.agrees(&c));
}
