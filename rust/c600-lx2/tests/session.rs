//! LX-2's session-level conditions (RTE.42 conditions 1 and 2) against `c600-runtime`'s child built from this
//! checkout, in the test's profile, each with its session in a fresh folder under Cargo's temporary directory. Source and
//! synthetic evidence only.

mod common;

use c600_lx2::session::{run_session_trial, SessionRecord};
use c600_lx2::{Observed, Trial};

/// Runs `trial` with its session in a fresh folder named for the test.
fn run(test: &str, trial: Trial) -> SessionRecord {
    let folder = common::fresh(&format!("{test}-{}", trial.name()));
    run_session_trial(&common::runtime(), &common::kernel(), &folder, trial)
}

fn control(test: &str) -> SessionRecord {
    let control = run(test, Trial::Control);
    assert!(control.agrees(&control), "{control:#?}");
    control
}

#[test]
fn the_control_commits_three_words_and_a_reopen_replays_its_state() {
    let c = control("control");
    assert_eq!((c.observed.clone(), c.open, c.spawns), (Observed::Completed, true, 1));
    assert_eq!(c.replayed.as_deref(), Some(c.state.as_str()));
    assert!(c.journal.len() >= 3, "three commits are journaled: {:?}", c.journal);
}

#[test]
fn each_fault_leaves_the_journal_and_the_state_of_the_control_and_the_session_open() {
    let c = control("faults");
    for trial in [Trial::LoopMetered, Trial::RecursionMetered, Trial::ForcedKill] {
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
    }
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
}
