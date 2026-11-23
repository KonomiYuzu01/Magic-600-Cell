//! The long replays of the archive's own steps and of the lattice phase, read in place. Each is `#[ignore]`d with its
//! reason: the exact engine replays about 2 to 3 ms per letter in the test profile (measured on the first 3,000
//! letters of T6's V′ and on the lattice phase's first word of 31,186 letters), far past a test run's budget. Run them
//! with `--ignored`; each prints its wall time.

use std::time::Instant;

use c600_checkers::Verdict;

use super::fixtures::*;

fn timed(
    name: &str,
    kind: c600_checkers::CheckerKind,
    record: &[u8],
    inputs: &[(String, Vec<u8>)],
) -> c600_checkers::CheckReport {
    let started = Instant::now();
    let report = check_owned(kind, record, inputs);
    eprintln!("{name}: {:?} in {:.1} s", report.verdict, started.elapsed().as_secs_f64());
    report
}

#[test]
#[ignore = "T6's four steps, 1,510,756 letters played one by one: about an hour"]
fn ct_lb5_05_gate2_composed_t6_four_steps_in_place() {
    let record = pinned(&format!("{T6}/steps.json"), T6_STEPS_SHA256);
    let report = timed("gate 2, T6", composed(), &record, &chain_inputs(T6, true));
    assert_eq!(report.verdict, Verdict::Pass, "{:#?}", report.trace.last());
    for k in 1..4 {
        let start = report.trace.iter().find(|t| t.claim == format!("steps[{k}].start")).unwrap();
        assert!(start.computed.starts_with("derived: "), "{}", start.computed);
    }
    // The rebuilt chain ends at the stored table of T6's end: every step reached its recorded end, and the stored table
    // carries the last one.
    assert!(report.trace.iter().any(|t| t.claim == "steps[3].end_digest" && t.held && t.computed == T6_END_DIGEST));
    assert!(report.trace.iter().any(|t| t.claim == "end.digest" && t.held && t.computed == T6_END_DIGEST));
}

#[test]
#[ignore = "the rule's three stages, 688,686 letters played one by one: about half an hour"]
fn ct_lb5_05_gate2_composed_rule_stages_in_place() {
    let record = pinned(&format!("{RULE}/steps.json"), RULE_STEPS_SHA256);
    let report = timed("gate 2, rule stages", composed(), &record, &chain_inputs(RULE, false));
    assert_eq!(report.verdict, Verdict::Pass, "{:#?}", report.trace.last());
}

#[test]
#[ignore = "T6's steps 1 to 3, 639,174 letters twist by twist in chunks: about half an hour"]
fn ct_lb5_05_gate3_tail_replay_t6_steps_1_to_3_in_place() {
    let record = pinned(&format!("{T6}/steps.json"), T6_STEPS_SHA256);
    let report = timed("gate 3, T6", tail_replay(), &record, &chain_inputs(T6, false));
    assert_eq!(report.verdict, Verdict::Pass, "{:#?}", report.trace.last());
    // Step 4 is never replayed for gate 3. Its only trace entries are the record form's, which every stage 18 checker
    // reads for every step (its stated length and admission and the digests and counts that chain it to step 3), never
    // a history, start, word, chunk, strict′ or end entry of a replay; the scope names it as historically incomplete.
    const FORM: [&str; 5] = ["letters", "admission", "off_after", "start_digest", "off_before"];
    assert!(
        report.trace.iter().filter_map(|t| t.claim.strip_prefix("steps[3].")).all(|rest| FORM.contains(&rest)),
        "step 4 is never replayed for gate 3"
    );
    assert!(
        report.trace.iter().any(|t| t.claim == "scope"
            && t.computed.contains("not applicable: steps[3]")
            && t.computed.contains("historical_check_incomplete")),
        "the scope names step 4 as historically incomplete"
    );
    for k in 0..3 {
        let history = report.trace.iter().find(|t| t.claim == format!("steps[{k}].history")).unwrap();
        assert!(history.computed.contains("records gate 3 complete"), "{}", history.computed);
    }
}

#[test]
#[ignore = "the lattice phase, 13,231,266 letters played one by one: about 11 hours"]
fn ct_lb5_05_lattice_replay_in_place() {
    let record = pinned(&format!("{RUN}/run.json"), RUN_SHA256);
    let inputs: Vec<_> = run_inputs()
        .into_iter()
        .filter(|(role, _)| {
            role == "menu" || role == "start-table.json" || role.starts_with("words/") || role == "tables/00052.json"
        })
        .collect();
    let report = timed("lattice replay", lattice_replay(), &record, &inputs);
    assert_eq!(report.verdict, Verdict::Pass, "{:#?}", report.trace.last());
}
