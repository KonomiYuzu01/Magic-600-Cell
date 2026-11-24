//! CT-LG3-05 (part: a check's static success at run time). The LP2 half (`c600-command/tests/kn5.rs`,
//! `c600-run/tests/conformance_lg3.rs`) ran the command layer's and the runtime's own refusals; this half shows that a
//! check job's pass never stands in for them: its result is read only at the revision it was pinned at (or one with an
//! equal digest), under its identity block, while its checkers are available with their identities, and a commit of
//! the checked word still meets the session's protection gate.
//!
//! Not run: a mode refusal. The session store holds `JumbleOff` only (M6), so no second mode exists to read a result
//! in; `result_at` compares the mode with the others.

use c600_checkers::{CheckerKind, Verdict};
use c600_research::job::codes;
use c600_research::record::admission_now;
use c600_research::{Check, CheckJob, Pinned};
use c600_session::persist::record::identity_json;
use c600_session::persist::run::{codes as run_codes, reproduce as reproduce_stored, Reproduction};

use crate::common::*;

const FIRST: [i32; 2] = [11, -7];
const WORD: [i32; 4] = [300, 2, -1199, 45];

/// CT-LG3-05 (part: stale and identity refusals, KN7.5): a job admitted at R finishes on R after the session commits
/// R'; its result reads at R, and again after an undo returns the head to R's digest, but not at R' or under another
/// identity block; a stated start is no revision and never reads as one.
#[test]
fn ct_lg3_05_a_pass_reads_only_at_its_pinned_revision() {
    let s = session();
    committed(&commit(&s, "first", &FIRST));
    let pin = snapshot(&s);
    let start = labels_record(&pin);
    let end = end_digest(&FIRST, &WORD);
    let certificate = expansion(&WORD, Some((&start, &pin.revision.digest, &end)));
    let job = CheckJob {
        checks: vec![Check::new(CheckerKind::Expansion, certificate, vec![("start", start)])],
        input: Pinned::snapshot(&pin),
        budget: WIDE,
    };
    let admitted = admit(job.clone());

    // The head moves after admission; the job runs on its pinned revision and finishes.
    committed(&commit(&s, "word", &WORD));
    assert_eq!(digest(&s), end);
    let moved = s.overview().revision;
    let run = run(&admitted);
    assert_eq!(run.outcome(), "completed");
    assert_eq!(run.children()[0].report.as_ref().map(|r| r.verdict.clone()), Some(Verdict::Pass));
    assert_eq!(run.input().revision.as_ref(), Some(&pin.revision));

    assert_eq!(run.result_at(&pin.revision).expect("its own revision").len(), 1);
    assert_eq!(run.result_at(&moved).expect_err("a later revision").code, codes::STALE);
    committed(&undo(&s, "undo"));
    let back = s.overview().revision;
    assert_eq!(back.digest, pin.revision.digest);
    assert_eq!(run.result_at(&back).expect("an equal digest").len(), 1);

    let mut other = pin.revision.clone();
    other.identity.model_id.push_str("-other");
    assert_eq!(run.result_at(&other).expect_err("another model").code, codes::IDENTITY);

    let stated = CheckJob {
        input: Pinned::stated("labels", &pin.revision.digest, identity_json(&pin.revision.identity)),
        ..job
    };
    let stated = run_job(stated);
    assert_eq!(stated.result_at(&pin.revision).expect_err("a stated start").code, codes::STALE);
}

/// CT-LG3-05 (part: availability and model at run time): a job naming a checker this build lacks is refused at
/// admission with `KN6:evidence.no_checker`; a stored pass re-runs only under the model and checkers it was recorded
/// with.
#[test]
fn ct_lg3_05_availability_and_model_are_checked_at_run_time() {
    let missing =
        stated_job(vec![Check::new(CheckerKind::Archive("no-such-checker"), b"{}".to_vec(), Vec::new())], WIDE);
    assert_eq!(missing.admit().expect_err("no such checker").code, codes::NO_CHECKER);

    let run = run_job(stated_job(vec![short_check(&[1, 2])], WIDE));
    let now = admission_now(run.key(), &engines()).expect("the host's admission");
    let mut other_model = now.clone();
    other_model.identity.model_id.push_str("-other");
    let refused = reproduce_stored(run.bytes(), &run.available(), &other_model, |_, _| panic!("never runs"));
    assert_eq!(refused, Reproduction::Refused(run_codes::KEY_MISMATCH.into()));
    let mut unavailable = now;
    unavailable.checkers.clear();
    let refused = reproduce_stored(run.bytes(), &run.available(), &unavailable, |_, _| panic!("never runs"));
    assert_eq!(refused, Reproduction::Refused(run_codes::KEY_MISMATCH.into()));
}

/// CT-LG3-05 (part: protection): a word whose check passed at the head is still refused at commit when the session
/// protects every orbit it would move; the pass is no permit.
#[test]
fn ct_lg3_05_a_pass_never_bypasses_protection() {
    let s = session();
    let pin = snapshot(&s);
    let start = labels_record(&pin);
    let end = end_digest(&[], &WORD);
    let certificate = expansion(&WORD, Some((&start, &pin.revision.digest, &end)));
    let job = CheckJob {
        checks: vec![Check::new(CheckerKind::Expansion, certificate, vec![("start", start)])],
        input: Pinned::snapshot(&pin),
        budget: WIDE,
    };
    let run = run_job(job);
    assert_eq!(run.result_at(&s.overview().revision).expect("at the head").len(), 1);
    let every: Vec<i64> = (0..35).collect();
    protect(&s, "protect", &every);
    assert_eq!(s.overview().protected.len(), 35);
    assert_eq!(code(&commit(&s, "word", &WORD)), "KN2:protection.orbit_would_move");
    assert_eq!(digest(&s), pin.revision.digest, "nothing moved");
}
