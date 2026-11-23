//! CT-DET-02 (part: a KN6 check job's stored record, with the checker's reports and traces published in it, and its
//! reproduction to the exact payload byte for byte). The LP2 half (`c600-run/tests/conformance_det.rs`,
//! `c600-session/tests/persist_run.rs`) ran the run record and its exact payload for interpreter runs; its "not run"
//! part, evidence records and checker traces in a stored record, runs here.
//!
//! The payload digests are pinned, so a CI run on Linux compares the same bytes as this machine. A payload names the
//! reports by digest and leaves the checker's source digest to the key, so a pin changes only when a check's result
//! does.

use c600_checkers::Verdict;
use c600_research::record::{report_bytes, REPORT_FORMAT};
use c600_research::{Det30, Pinned};
use c600_session::persist::codec::{canonical, digest_ref, Json};
use c600_session::persist::run::{Record, Reproduction};

use crate::common::*;

/// The word the job checks, and the word committed before it.
const BEFORE: [i32; 3] = [1, 2, 3];
const WORD: [i32; 5] = [17, -40, 1200, -1, 600];

/// The pinned exact payloads (SHA-256 of their FP.5 bytes).
const COMPLETED_PAYLOAD: &str = "165964fb00fc81f9c60bce203c35005be78b441fc976405e151bd3f8cd49a497";
const CUT_OFF_PAYLOAD: &str = "331d238ed2268b2d1b36f86f7a83b45361863bad3b6d0be27f2d5bef4614b166";

/// A job pinned at the revision after `BEFORE`, checking `WORD`'s expansion with its replay from that revision to the
/// end a session's own commit reaches, and a short check without replay.
fn job() -> (c600_research::CheckJob, String) {
    let s = session();
    committed(&commit(&s, "before", &BEFORE));
    let pin = snapshot(&s);
    let start = labels_record(&pin);
    let end = end_digest(&BEFORE, &WORD);
    let certificate = expansion(&WORD, Some((&start, &pin.revision.digest, &end)));
    let checks = vec![
        c600_research::Check::new(c600_checkers::CheckerKind::Expansion, certificate, vec![("start", start)]),
        short_check(&[5, 6, 7]),
    ];
    (c600_research::CheckJob { checks, input: Pinned::snapshot(&pin), budget: WIDE }, end)
}

/// CT-DET-02 (part: a completed check job's stored record holds its reports and their traces by digest, reads back
/// to the same record, and reproduces to the exact payload byte for byte).
#[test]
fn ct_det_02_completed_check_job_reproduces_byte_for_byte() {
    let (job, end) = job();
    let run = run_job(job);
    assert_eq!((run.outcome(), run.code()), ("completed", None));
    assert_eq!(run.reading(), Det30::Complete);
    assert_eq!(Record::read(run.bytes()).expect("the stored record reads"), *run.record());

    // The reports: each the checker's own, published by digest with its trace.
    let reports: Vec<_> = run.children().iter().map(|child| child.report.clone().expect("a report")).collect();
    assert!(reports.iter().all(|report| report.verdict == Verdict::Pass), "{reports:?}");
    let replay = &reports[0];
    assert!(replay.trace.iter().any(|step| step.claim == "replay.end.digest" && step.computed == end), "{replay:?}");
    let outputs = run.payload().get("outputs").expect("outputs");
    let published: Vec<Json> = reports.iter().map(|report| digest_ref("bytes", &report_bytes(report))).collect();
    assert_eq!(outputs, &Json::List(published));
    assert_eq!(run.outputs(), reports.iter().map(report_bytes).collect::<Vec<_>>());
    let text = String::from_utf8(run.outputs()[0].clone()).expect("UTF-8");
    assert!(text.contains(REPORT_FORMAT) && text.contains("\"trace\""));
    assert!(!text.contains(&replay.checker.source), "the checker's source digest is the key's, not the payload's");
    assert!(run.key().checkers.iter().any(|pin| pin.version == replay.checker.id));

    // DET.5 to DET.7: the stored bytes alone reproduce the payload, and the re-run's reports are the same bytes.
    let again = reproduce(&run);
    assert_eq!(again.answer, Reproduction::Same(run.payload().clone()));
    let rerun = again.run.expect("a matching re-run");
    assert_eq!(canonical(rerun.payload()), canonical(run.payload()));
    assert_eq!(rerun.outputs(), run.outputs());
    assert_eq!(payload_digest(&run), COMPLETED_PAYLOAD, "the pinned payload");
}

/// CT-DET-02 (part: a deterministic cut-off reproduces like a completed run, DET.5): the record keeps the stop's
/// position and the units charged, and the re-run stops at the same unit.
#[test]
fn ct_det_02_deterministic_cut_off_reproduces_byte_for_byte() {
    let (mut job, _) = job();
    job.checks.insert(0, long_check(1_000, 1_000));
    job.budget = c600_research::Budget { kernel_cost: 12_345, ..WIDE };
    let run = run_job(job);
    assert_eq!((run.outcome(), run.code()), ("cut_off", Some("KN7:budget.kernel_cost")));
    assert_eq!(run.reading(), Det30::CutOff { covered: 12_345 });
    assert!(run.value().is_none() && run.outputs().is_empty());
    let stop = run.payload().get("stop").expect("a stop");
    assert_eq!(stop.get("at").and_then(Json::as_text), Some("check[0] unit 12345"));
    assert_eq!(run.children().len(), 1, "the job ends at its first check");
    let again = reproduce(&run);
    assert_eq!(again.answer, Reproduction::Same(run.payload().clone()));
    assert_eq!(payload_digest(&run), CUT_OFF_PAYLOAD, "the pinned payload");
}

/// CT-DET-02 (part: the same job on another thread writes the same payload bytes; the stop scopes are per thread).
#[test]
fn ct_det_02_another_thread_writes_the_same_payload() {
    let (job, _) = job();
    let here = run_job(job.clone());
    let there = std::thread::spawn(move || {
        let run = run_job(job);
        (canonical(run.payload()), run.key().digest())
    })
    .join()
    .expect("the thread ends");
    assert_eq!(there, (canonical(here.payload()), here.key().digest()));
}
