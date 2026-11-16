//! Chapter 06 sections 5 and 6: the decision list of EVD.32, the admission conditions of EVD.40 to EVD.42, cycles and
//! discharges (EVD.17, EVD.26). Expected rows are transcribed from the chapter's tables, never from library output.

mod common;

use std::collections::BTreeSet;

use c600_evidence::error::{INCONSISTENT_FIELDS, NEGATIVE_WITHOUT_COMPLETION, PREMISE_UNMET, SCOPE_MISMATCH};
use c600_evidence::*;
use common::*;

// --- CT-EVD-01 ------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum St {
    Neither,
    OnlyOne,
    Zero,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Srch {
    Absent,
    ClosedComplete,
    BudgetIncomplete,
}

fn covers_axis() -> [Covers; 4] {
    [
        Covers::OneInstance,
        Covers::search_set("R", "retained letters", "three letters"),
        Covers::admitted_domain("D[7]", "k = 7"),
        Covers::every_reachable("J600"),
    ]
}

fn jobs_axis() -> [Option<JobOutcome>; 7] {
    [
        None,
        Some(JobOutcome::Completed),
        Some(JobOutcome::Cancelled),
        Some(JobOutcome::CutOff),
        Some(JobOutcome::Error),
        Some(JobOutcome::NotRun),
        Some(JobOutcome::NotStated),
    ]
}

/// One node-level case: S and neg S over the same scope; stands0(S) from a (theorem, model review) leaf, stands1(S)
/// only from AND(theorem leaf, unresolved premise), stands0(neg S) from a theorem leaf; the search record and the job
/// recorded for S. A statement over one instance is finite and its negation too; otherwise S is universal and neg S
/// existence (EVD.9).
fn case(st: St, n0: bool, cov: &Covers, srch: Srch, job: &Option<JobOutcome>) -> (Graph, Key) {
    let mut g = Graph::new();
    let s = key("S", cov.clone());
    let n = key("N", cov.clone());
    let p = key("p", Covers::OneInstance);
    if *cov == Covers::OneInstance {
        pair(&mut g, finite("S").label("source label of S"), finite("N"));
    } else {
        pair(&mut g, universal("S").label("source label of S"), existence("N"));
    }
    g.add_statement(finite("p")).unwrap();
    g.root(&s);
    g.root(&n);
    match st {
        St::Neither => {}
        St::OnlyOne => {
            let leaf = g.add_leaf(Leaf::new(&s, th()));
            let prem = g.add_premise(&p, PremiseClass::OpenHypothesis);
            let and = g.add_and(&[leaf, prem]).unwrap();
            g.add_or_child(&s, and).unwrap();
        }
        St::Zero => {
            g.support(&s, Leaf::new(&s, th()));
        }
    }
    if n0 {
        g.support(&n, Leaf::new(&n, th()));
    }
    let target = if *cov == Covers::OneInstance { sid("S") } else { sid("N") };
    let stated = job.clone().unwrap_or(JobOutcome::NotStated);
    match srch {
        Srch::Absent => {
            if let Some(o) = job {
                g.attach_job(&s, ProducingJob::new("job-1", o.clone()));
            }
        }
        Srch::ClosedComplete => g.attach_search(&s, SearchRecord::new(&target, cov, Complete::Yes, Stop::Closed, stated)),
        Srch::BudgetIncomplete => {
            g.attach_search(&s, SearchRecord::new(&target, cov, Complete::No, Stop::DeterministicBudget, stated))
        }
    }
    (g, s)
}

/// EVD.32's decision list, transcribed: row 0 (here only I4 can arise), then rows 1 to 7, first match wins.
fn expected(st: St, n0: bool, cov: &Covers, srch: Srch, job: &Option<JobOutcome>) -> Result<(u8, Claim), &'static str> {
    if st == St::Zero && n0 {
        return Err(INCONSISTENT_FIELDS); // I4: stands0(S) and stands0(neg S)
    }
    if n0 {
        return Ok((1, Claim::Refuted));
    }
    if st == St::Zero {
        let finite_kind = matches!(cov, Covers::OneInstance | Covers::SearchSet { .. });
        return Ok((if finite_kind { 2 } else { 3 }, Claim::Proved));
    }
    if st == St::OnlyOne {
        return Ok((4, Claim::Conditional));
    }
    // Row 5: a search record with complete = no, job != not_run and a deterministic-budget stop.
    let search_job = job.clone().unwrap_or(JobOutcome::NotStated);
    if srch == Srch::BudgetIncomplete && search_job != JobOutcome::NotRun {
        return Ok((5, Claim::Open));
    }
    // Row 6: a job ended cancelled or in error, or a non-search job cut off. A search job cut off with stop closed was
    // not stopped by wall clock or a resource (its stop says closed), so it is not row 6.
    let row6 = match srch {
        Srch::Absent => matches!(job, Some(JobOutcome::Cancelled | JobOutcome::Error | JobOutcome::CutOff)),
        _ => matches!(search_job, JobOutcome::Cancelled | JobOutcome::Error),
    };
    if row6 {
        return Ok((6, Claim::Open));
    }
    Ok((7, Claim::Open))
}

/// CT-EVD-01 (whole): the 504 node-level cases, enumerated over stands0 and stands1 (6: stands0(S) and stands1(S) in
/// {neither, stands1 only, both} times stands0(neg S) in {no, yes}), covered-set kind (4), search record (3: absent,
/// complete = yes with stop closed, complete = no with a deterministic budget) and job outcome (7: none recorded, the
/// four KN7.3 outcomes and EVD.47's not run and not stated). Each gives exactly one row of EVD.32 or a typed
/// inconsistency, and the row is the one the decision list names.
#[test]
fn ct_evd_01_node_level_cases_give_one_row_each() {
    let e = ed();
    let mut count = 0;
    let mut per_row = [0usize; 8];
    for st in [St::Neither, St::OnlyOne, St::Zero] {
        for n0 in [false, true] {
            for cov in covers_axis() {
                for srch in [Srch::Absent, Srch::ClosedComplete, Srch::BudgetIncomplete] {
                    for job in jobs_axis() {
                        count += 1;
                        let (g, s) = case(st, n0, &cov, srch, &job);
                        let got = word(&g, &s, &e);
                        let want = expected(st, n0, &cov, srch, &job);
                        let label = format!("{st:?} n0={n0} {cov:?} {srch:?} {job:?}");
                        match (got, want) {
                            (Err(err), Err(c)) => {
                                assert_eq!(err.code, c, "{label}");
                                assert!(err.has(Inconsistency::I4), "{label}");
                                per_row[0] += 1;
                            }
                            (Ok(w), Ok((r, claim))) => {
                                assert_eq!((w.row, w.claim), (r, claim), "{label}");
                                if r == 3 {
                                    assert!(w.phrase_required, "{label}: row 3 cannot be shown without its phrase");
                                }
                                if r == 4 {
                                    assert_eq!(w.premises, vec![key("p", Covers::OneInstance)], "{label}");
                                }
                                if r == 7 {
                                    assert_eq!(w.source_label.as_deref(), Some("source label of S"), "{label}");
                                }
                                per_row[r as usize] += 1;
                            }
                            (got, want) => panic!("{label}: got {got:?}, want {want:?}"),
                        }
                    }
                }
            }
        }
    }
    assert_eq!(count, 504);
    assert_eq!(per_row.iter().sum::<usize>(), 504);
    // Every row of the list is reached by some case.
    assert!(per_row.iter().all(|n| *n > 0), "rows reached: {per_row:?}");
}

// --- CT-EVD-05 ------------------------------------------------------------------------------------------------

/// A universal S over a declared search set with an exhaustive leaf whose `covered` matches S's scope, its search
/// record saying `complete` with stop closed and the given job outcome.
fn exhaustive_case(complete: Complete, job: JobOutcome) -> (Graph, Key, Key) {
    let f = Covers::search_set("R", "retained letters", "three letters");
    let input = poses("state 7303");
    let s = Key::new(sid("S4"), scope_in(f.clone(), input.clone()));
    let e = Key::new(sid("E3"), scope_in(f.clone(), input.clone()));
    let mut g = Graph::new();
    pair(&mut g, universal("S4").label("stage 15 record"), existence("E3"));
    g.root(&e);
    let leaf = Leaf::new(&s, exhaustive())
        .covered(Covered::new(f.clone(), Some(input)))
        .search(SearchRecord::new(&sid("E3"), &f, complete, Stop::Closed, job));
    g.support(&s, leaf);
    (g, s, e)
}

/// CT-EVD-05 (whole): Theorem D (b) and (b1). A sample leaf of any check, added under S or neg S, never moves S from
/// rows 5 to 7 into rows 1 to 4, for every threshold U inside U0 (all 83,521 for one case, a deterministic sample of
/// 40 for each of the 504 cases of CT-EVD-01 that read rows 5 to 7); changing only the job outcome, with the same
/// inert leaves, never does either. The exhaustive-leaf pair reads row 7 under not stated and row 2 under completed,
/// and with complete = no both read row 0 (I5).
#[test]
fn ct_evd_05_samples_and_job_outcomes_never_upgrade() {
    let e = ed();
    let checks = [
        Check::None,
        Check::Producer,
        Check::IndependentChecker,
        Check::ModelReview,
        Check::HumanReview,
        Check::FormalProof,
    ];
    let mut thresholds: Vec<BTreeSet<Grade>> = Vec::new();
    let mut i = 0usize;
    let mut rng = Rng::new(0x0E5D_0005);
    let mut picks: BTreeSet<usize> = (0..40).map(|_| rng.below(83_521)).collect();
    picks.insert(0); // the empty threshold
    picks.insert(83_520); // U0 itself
    for_each_threshold(|u| {
        if picks.contains(&i) {
            thresholds.push(u.clone());
        }
        i += 1;
    });
    assert_eq!(i, 83_521);
    let editions: Vec<Edition> = thresholds.iter().map(|u| e.with_threshold(u).expect("up-set in U0")).collect();
    let mut checked = 0usize;
    for st in [St::Neither, St::OnlyOne, St::Zero] {
        for n0 in [false, true] {
            for cov in covers_axis() {
                for srch in [Srch::Absent, Srch::ClosedComplete, Srch::BudgetIncomplete] {
                    for job in jobs_axis() {
                        let (g, s) = case(st, n0, &cov, srch, &job);
                        let n = Key::new(sid("N"), s.scope.clone());
                        for ed_u in &editions {
                            let Ok(base) = word(&g, &s, ed_u) else { continue };
                            if base.row < 5 {
                                continue;
                            }
                            for k in &checks {
                                for target in [&s, &n] {
                                    let mut g2 = g.clone();
                                    g2.support(target, Leaf::new(target, gr(Basis::Sample, k.clone())));
                                    let w = word(&g2, &s, ed_u).expect("a sample adds no inconsistency");
                                    assert!(w.row >= 5, "a sample moved row {} to {}", base.row, w.row);
                                    checked += 1;
                                }
                            }
                            for other in jobs_axis() {
                                let (g3, _) = case(st, n0, &cov, srch, &other);
                                let w = word(&g3, &s, ed_u).expect("job outcomes add no inconsistency here");
                                assert!(w.row >= 5, "a job outcome moved row {} to {}", base.row, w.row);
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 0);
    // One case over every threshold inside U0.
    let (g, s) = case(St::Neither, false, &Covers::admitted_domain("D[7]", "k = 7"), Srch::BudgetIncomplete, &None);
    let mut all = 0usize;
    for_each_threshold(|u| {
        let ed_u = e.with_threshold(u).expect("up-set in U0");
        let mut g2 = g.clone();
        g2.support(&s, Leaf::new(&s, gr(Basis::Sample, Check::FormalProof)));
        assert_eq!(row(&g2, &s, &ed_u), 5);
        all += 1;
    });
    assert_eq!(all, 83_521);

    // The exhaustive-leaf pair (EVD.41, Theorem D (b1)).
    let (g, s, _) = exhaustive_case(Complete::Yes, JobOutcome::NotStated);
    let w = word(&g, &s, &e).unwrap();
    assert_eq!(w.row, 7);
    assert_eq!(w.source_label.as_deref(), Some("stage 15 record"));
    let (g, s, _) = exhaustive_case(Complete::Yes, JobOutcome::Completed);
    assert_eq!(row(&g, &s, &e), 2);
    for job in [JobOutcome::NotStated, JobOutcome::Completed] {
        let (g, s, _) = exhaustive_case(Complete::No, job);
        let err = word(&g, &s, &e).unwrap_err();
        assert_eq!(err.code, NEGATIVE_WITHOUT_COMPLETION);
        assert!(err.has(Inconsistency::I5));
    }
}

// --- CT-EVD-07 ------------------------------------------------------------------------------------------------

/// CT-EVD-07 (whole): EVD.26. A pure child cycle reads open with a diagnostic, a cycle gives its members only what
/// they have from outside, and a circular discharge stays conditional on its premises and is never proved.
#[test]
fn ct_evd_07_cycles_read_open_and_circular_discharge_stays_conditional() {
    let e = ed();
    // A pure child cycle through one key.
    let mut g = Graph::new();
    let s = key("S", Covers::OneInstance);
    g.add_statement(finite("S")).unwrap();
    let root = g.root(&s);
    let and = g.add_and(&[root]).unwrap();
    g.add_or_child(&s, and).unwrap();
    let w = word(&g, &s, &e).unwrap();
    assert_eq!((w.row, w.claim), (7, Claim::Open));
    assert!(w.diagnostics.contains(&Diagnostic::ChildCycle));

    // A pure child cycle through two keys.
    let mut g = Graph::new();
    let t = key("T", Covers::OneInstance);
    g.add_statement(finite("S")).unwrap();
    g.add_statement(finite("T")).unwrap();
    let rs = g.root(&s);
    let rt = g.root(&t);
    let a1 = g.add_and(&[rt]).unwrap();
    let a2 = g.add_and(&[rs]).unwrap();
    g.add_or_child(&s, a1).unwrap();
    g.add_or_child(&t, a2).unwrap();
    for k in [&s, &t] {
        let w = word(&g, k, &e).unwrap();
        assert_eq!(w.row, 7);
        assert!(w.diagnostics.contains(&Diagnostic::ChildCycle));
    }
    // The same cycle with support from outside gives both members only that support.
    g.support(&t, Leaf::new(&t, th()));
    assert_eq!(row(&g, &s, &e), 2);
    assert_eq!(row(&g, &t, &e), 2);

    // A circular discharge: S rests on premise T and T on premise S.
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    g.add_statement(finite("T")).unwrap();
    let l1 = g.add_leaf(Leaf::new(&s, th()));
    let p1 = g.add_premise(&t, PremiseClass::OpenHypothesis);
    let a1 = g.add_and(&[l1, p1]).unwrap();
    g.add_or_child(&s, a1).unwrap();
    let l2 = g.add_leaf(Leaf::new(&t, th()));
    let p2 = g.add_premise(&s, PremiseClass::OpenHypothesis);
    let a2 = g.add_and(&[l2, p2]).unwrap();
    g.add_or_child(&t, a2).unwrap();
    let ws = word(&g, &s, &e).unwrap();
    let wt = word(&g, &t, &e).unwrap();
    assert_eq!((ws.row, ws.claim), (4, Claim::Conditional));
    assert_eq!((wt.row, wt.claim), (4, Claim::Conditional));
    assert!(ws.premises.contains(&t));
    assert!(wt.premises.contains(&s));
    let st = standing(&g, &s, &e).unwrap();
    assert!(!st.stands0 && st.stands1);
}

// --- CT-EVD-08 ------------------------------------------------------------------------------------------------

/// CT-EVD-08 (whole): EVD.17, EVD.8, EVD.26 and EVD.58. Discharge by the identical key succeeds; an equivalent
/// statement under another ID is refused with `EVD:evidence.premise_unmet` until a bridge lemma exists, and after it
/// the discharge still goes by the premise's own key; a proof for D[7] does not discharge D[9]; a statement resting on
/// a theorem leaf and an unresolved premise reads conditional with no error, and requesting that premise's discharge
/// with no matching claim raises `EVD:evidence.premise_unmet`.
#[test]
fn ct_evd_08_discharge_is_by_identical_key() {
    let e = ed();
    let p = key("P", Covers::OneInstance);
    let q = key("Q, the same meaning as P under another ID", Covers::OneInstance);
    let s = key("S", Covers::OneInstance);
    let mut g = Graph::new();
    for name in ["P", "Q, the same meaning as P under another ID", "S"] {
        g.add_statement(finite(name)).unwrap();
    }
    g.support(&q, Leaf::new(&q, th()));
    let l = g.add_leaf(Leaf::new(&s, th()));
    let prem = g.add_premise(&p, PremiseClass::OpenHypothesis);
    let and = g.add_and(&[l, prem]).unwrap();
    g.add_or_child(&s, and).unwrap();

    // Before any claim of P: conditional with no error, and every discharge request is refused.
    let w = word(&g, &s, &e).unwrap();
    assert_eq!((w.row, w.claim), (4, Claim::Conditional));
    assert_eq!(w.premises, vec![p.clone()]);
    assert_eq!(discharge(&g, &p, &p, &e).unwrap_err().code, PREMISE_UNMET);
    assert_eq!(discharge(&g, &p, &q, &e).unwrap_err().code, PREMISE_UNMET);

    // A bridge lemma: P's own key gets an argument AND(theorem leaf, premise Q).
    let bridge = g.add_leaf(Leaf::new(&p, th()).support("bridge lemma P <-> Q"));
    let pq = g.add_premise(&q, PremiseClass::OpenHypothesis);
    let arg = g.add_and(&[bridge, pq]).unwrap();
    g.add_or_child(&p, arg).unwrap();
    assert!(discharge(&g, &p, &p, &e).is_ok());
    assert_eq!(discharge(&g, &p, &q, &e).unwrap_err().code, PREMISE_UNMET);
    assert_eq!(row(&g, &s, &e), 2);

    // Identical key: a direct proof of P discharges P.
    let mut g2 = Graph::new();
    g2.add_statement(finite("P")).unwrap();
    g2.support(&p, Leaf::new(&p, th()));
    assert!(discharge(&g2, &p, &p, &e).is_ok());

    // D[7] does not discharge D[9] (EVD.8: the ID binds k).
    let d7 = key("U(h, 7) on D[7]", Covers::admitted_domain("D[7]", "k = 7"));
    let d9 = key("U(h, 9) on D[9]", Covers::admitted_domain("D[9]", "k = 9"));
    let t = key("T rests on U(h, 9)", Covers::OneInstance);
    let mut g3 = Graph::new();
    g3.add_statement(universal("U(h, 7) on D[7]")).unwrap();
    g3.add_statement(universal("U(h, 9) on D[9]")).unwrap();
    g3.add_statement(finite("T rests on U(h, 9)")).unwrap();
    g3.support(&d7, Leaf::new(&d7, th()));
    let l = g3.add_leaf(Leaf::new(&t, th()));
    let p9 = g3.add_premise(&d9, PremiseClass::OpenHypothesis);
    let and = g3.add_and(&[l, p9]).unwrap();
    g3.add_or_child(&t, and).unwrap();
    assert_eq!(discharge(&g3, &d9, &d7, &e).unwrap_err().code, PREMISE_UNMET);
    let w = word(&g3, &t, &e).unwrap();
    assert_eq!((w.row, w.premises.clone()), (4, vec![d9.clone()]));
    assert_eq!(row(&g3, &d7, &e), 3);
}

// --- CT-EVD-09 ------------------------------------------------------------------------------------------------

fn err_of(g: &Graph, k: &Key, e: &Edition) -> EvidenceError {
    match word(g, k, e) {
        Err(err) => err,
        Ok(w) => panic!("expected row 0, got row {}", w.row),
    }
}

/// CT-EVD-09 (whole): EVD.40, EVD.41 and EVD.60. Each of I1 to I8 raises its code, I5 with
/// `EVD:evidence.negative_without_completion` and the others with `EVD:evidence.inconsistent_fields`; job outcome
/// "not stated" raises nothing. A universal statement whose OR holds one AND with no children reads row 0 under every
/// threshold inside U0, never proved, and a supported theorem elsewhere in the graph keeps its word.
#[test]
fn ct_evd_09_each_inconsistency_raises_its_code() {
    // The codes are EVD.58's literal strings.
    assert_eq!(INCONSISTENT_FIELDS, "EVD:evidence.inconsistent_fields");
    assert_eq!(NEGATIVE_WITHOUT_COMPLETION, "EVD:evidence.negative_without_completion");
    assert_eq!(PREMISE_UNMET, "EVD:evidence.premise_unmet");
    assert_eq!(SCOPE_MISMATCH, "EVD:evidence.scope_mismatch");
    let e = ed();
    let f = Covers::search_set("R", "retained letters", "three letters");
    let d7 = Covers::admitted_domain("D[7]", "k = 7");
    let expect = |g: &Graph, k: &Key, e: &Edition, case: Inconsistency, code: &str| {
        let err = err_of(g, k, e);
        assert!(err.has(case), "{case:?} not found in {err}");
        assert_eq!(err.code, code, "{case:?}");
    };

    // I1: a value outside its enumeration, and a pair the invalid-pairs table lists.
    let s = key("S", Covers::OneInstance);
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    g.support(&s, Leaf::new(&s, gr(Basis::Other("measured somehow".into()), Check::Producer)));
    expect(&g, &s, &e, Inconsistency::I1, INCONSISTENT_FIELDS);
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    g.support(&s, Leaf::new(&s, gr(Basis::Sample, Check::FormalProof)));
    assert_eq!(row(&g, &s, &e), 7);
    let listed = e.with_invalid_pairs(&[gr(Basis::Sample, Check::FormalProof)]);
    expect(&g, &s, &listed, Inconsistency::I1, INCONSISTENT_FIELDS);
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    g.attach_job(&s, ProducingJob::new("job", JobOutcome::Other("paused".into())));
    expect(&g, &s, &e, Inconsistency::I1, INCONSISTENT_FIELDS);

    // I2: a discharge edge between different keys; a negation pair that is not mutual; a leaf with other identity.
    let p = key("P", Covers::OneInstance);
    let q = key("Q", Covers::OneInstance);
    let mut g = Graph::new();
    for n in ["S", "P", "Q"] {
        g.add_statement(finite(n)).unwrap();
    }
    g.support(&q, Leaf::new(&q, th()));
    let prem = g.add_premise_discharged_by(&p, PremiseClass::OpenHypothesis, &q);
    let l = g.add_leaf(Leaf::new(&s, th()));
    let and = g.add_and(&[l, prem]).unwrap();
    g.add_or_child(&s, and).unwrap();
    expect(&g, &s, &e, Inconsistency::I2, INCONSISTENT_FIELDS);
    let mut g = Graph::new();
    g.add_statement(finite("S").negation(&sid("N"))).unwrap();
    g.add_statement(finite("N").negation(&sid("M"))).unwrap();
    g.root(&s);
    g.root(&key("N", Covers::OneInstance));
    expect(&g, &s, &e, Inconsistency::I2, INCONSISTENT_FIELDS);
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    g.support(&s, Leaf::new(&s, th()).identity("identity:another-puzzle"));
    expect(&g, &s, &e, Inconsistency::I2, INCONSISTENT_FIELDS);

    // I3: a label digest under a pose-level subject (EVD.11).
    let z = Key::new(sid("Z reachable"), scope_in(Covers::OneInstance, labels("labels of Z")));
    let mut g = Graph::new();
    g.add_statement(finite("Z reachable").subject(Level::Pose)).unwrap();
    g.root(&z);
    expect(&g, &z, &e, Inconsistency::I3, INCONSISTENT_FIELDS);

    // I4: stands0(S) and stands0(neg S).
    let n = key("N", Covers::OneInstance);
    let mut g = Graph::new();
    pair(&mut g, finite("S"), finite("N"));
    g.support(&s, Leaf::new(&s, th()));
    g.support(&n, Leaf::new(&n, th()));
    expect(&g, &s, &e, Inconsistency::I4, INCONSISTENT_FIELDS);

    // I5: an exhaustive leaf on a search that says complete = no; complete = yes with a cancelled job; a non-search
    // negative from a job that errored.
    for (complete, job) in [(Complete::No, JobOutcome::Completed), (Complete::Yes, JobOutcome::Cancelled)] {
        let (g, s4, _) = exhaustive_case(complete, job);
        expect(&g, &s4, &e, Inconsistency::I5, NEGATIVE_WITHOUT_COMPLETION);
    }
    let u = key("U", d7.clone());
    let mut g = Graph::new();
    g.add_statement(universal("U")).unwrap();
    g.support(&u, Leaf::new(&u, th()).job(ProducingJob::new("job", JobOutcome::Error)));
    expect(&g, &u, &e, Inconsistency::I5, NEGATIVE_WITHOUT_COMPLETION);

    // "Not stated" is no contradiction.
    let (g, s4, _) = exhaustive_case(Complete::Yes, JobOutcome::NotStated);
    assert_eq!(row(&g, &s4, &e), 7);

    // I6: a universal refuted by a failure with no checked scope membership.
    let su = key("S", d7.clone());
    let ne = key("N", d7.clone());
    let mut g = Graph::new();
    pair(&mut g, universal("S"), existence("N"));
    g.support(&ne, Leaf::new(&ne, exact()).covered(Covered::new(Covers::OneInstance, Some(poses("x")))));
    expect(&g, &su, &e, Inconsistency::I6, INCONSISTENT_FIELDS);

    // I7 (a): a computed leaf in a universal's argument whose covered set is not the statement's scope.
    let mut g = Graph::new();
    g.add_statement(universal("U")).unwrap();
    g.support(&u, Leaf::new(&u, exact()).covered(Covered::new(Covers::OneInstance, None)));
    expect(&g, &u, &e, Inconsistency::I7, INCONSISTENT_FIELDS);
    // I7 (b): an exhaustive leaf covering an admitted domain.
    let x = key("X", Covers::OneInstance);
    let mut g = Graph::new();
    g.add_statement(Statement::new(sid("X")).form(Form::Existence)).unwrap();
    g.support(&x, Leaf::new(&x, exhaustive()).covered(Covered::new(d7.clone(), None)));
    expect(&g, &x, &e, Inconsistency::I7, INCONSISTENT_FIELDS);
    // I7 (c): a finite statement whose covers is not one instance.
    let fs = key("F", f.clone());
    let mut g = Graph::new();
    g.add_statement(finite("F")).unwrap();
    g.root(&fs);
    expect(&g, &fs, &e, Inconsistency::I7, INCONSISTENT_FIELDS);

    // I8: a universal statement whose OR holds one AND with no children, under every threshold inside U0; a theorem
    // elsewhere keeps its word.
    let v = key("V", d7.clone());
    let mut g = Graph::new();
    g.add_statement(universal("U")).unwrap();
    g.add_statement(universal("V")).unwrap();
    let empty = g.add_and(&[]).unwrap();
    g.add_or_child(&u, empty).unwrap();
    g.support(&v, Leaf::new(&v, th()));
    let mut n_thresholds = 0;
    for_each_threshold(|t| {
        let ed_u = e.with_threshold(t).expect("up-set in U0");
        let err = err_of(&g, &u, &ed_u);
        assert!(err.has(Inconsistency::I8));
        assert_eq!(err.code, INCONSISTENT_FIELDS);
        let w = word(&g, &v, &ed_u).unwrap();
        if t.contains(&th()) {
            assert_eq!((w.row, w.word), (3, WordKind::Proved));
        } else {
            assert_eq!(w.row, 7);
        }
        n_thresholds += 1;
    });
    assert_eq!(n_thresholds, 83_521);
}

// --- CT-EVD-10 ------------------------------------------------------------------------------------------------

/// CT-EVD-10 (part: synthetic stand-ins for the pair; not run: the actual solved and centre-0-turned states of J(A4₀)
/// and their kernel digests, which belong to the jumbling half, M5): EVD.11. The two states share their labels and
/// differ as poses. A reachability statement at pose level whose scope names a label digest is refused as I3; with a
/// poses digest it is admitted, and the two states get different keys.
#[test]
fn ct_evd_10_coarser_scope_digest_is_refused() {
    let e = ed();
    // Synthetic stand-ins: the same labels, different poses (EVD.11's counterexample).
    let solved_labels = labels("labels: every sticker on its home face");
    let turned_labels = labels("labels: every sticker on its home face");
    let solved_poses = poses("poses: solved");
    let turned_poses = poses("poses: solved with centre 0 turned");
    assert_eq!(solved_labels, turned_labels);
    assert_ne!(solved_poses, turned_poses);

    let reach = || finite("Z is reachable in J(A4_0)").subject(Level::Pose);
    let by_labels_solved = Key::new(sid("Z is reachable in J(A4_0)"), scope_in(Covers::OneInstance, solved_labels));
    let by_labels_turned = Key::new(sid("Z is reachable in J(A4_0)"), scope_in(Covers::OneInstance, turned_labels));
    assert_eq!(by_labels_solved, by_labels_turned, "a label digest cannot tell the reachable state from the other");
    let mut g = Graph::new();
    g.add_statement(reach()).unwrap();
    g.root(&by_labels_solved);
    let err = word(&g, &by_labels_solved, &e).unwrap_err();
    assert_eq!(err.code, INCONSISTENT_FIELDS);
    assert!(err.has(Inconsistency::I3));

    let by_poses_solved = Key::new(sid("Z is reachable in J(A4_0)"), scope_in(Covers::OneInstance, solved_poses));
    let by_poses_turned = Key::new(sid("Z is reachable in J(A4_0)"), scope_in(Covers::OneInstance, turned_poses));
    assert_ne!(by_poses_solved, by_poses_turned);
    let mut g = Graph::new();
    g.add_statement(reach()).unwrap();
    g.root(&by_poses_solved);
    g.root(&by_poses_turned);
    assert_eq!(row(&g, &by_poses_solved, &e), 7);
    assert_eq!(row(&g, &by_poses_turned, &e), 7);
}

// --- CT-EVD-11 ------------------------------------------------------------------------------------------------

/// CT-EVD-11 (whole): EVD.42 and EVD.43. A universal is refuted by one checked counterexample with membership, and not
/// without it; an existence statement is not refuted by a cut-off search; a plain existence statement is refuted by a
/// proof, a complete certificate or the guarded exhaustive search of exactly its declared finite set; a mixed
/// statement with outer quantifier universal needs a witness and a complete negative, and one with outer quantifier
/// existential needs a proof or a complete certificate.
#[test]
fn ct_evd_11_refutation_is_asymmetric() {
    let e = ed();
    let d7 = Covers::admitted_domain("D[7]", "k = 7");

    // A universal and one counterexample.
    let s = key("S", d7.clone());
    let n = key("N", d7.clone());
    let base = || {
        let mut g = Graph::new();
        pair(&mut g, universal("S"), existence("N"));
        g.root(&s);
        g
    };
    let x = Covered::new(Covers::OneInstance, Some(poses("x")));
    let mut g = base();
    g.support(&n, Leaf::new(&n, exact()).covered(x.clone()).member_of(&s, "x lies in D[7]", exact()));
    let w = word(&g, &s, &e).unwrap();
    assert_eq!((w.row, w.claim), (1, Claim::Refuted));
    assert_eq!(row(&g, &n, &e), 3);
    let mut g = base();
    g.support(&n, Leaf::new(&n, exact()).covered(x.clone()));
    assert_eq!(word(&g, &s, &e).unwrap_err().code, INCONSISTENT_FIELDS);
    // Membership checked with "none" is no checked membership.
    let mut g = base();
    g.support(&n, Leaf::new(&n, exact()).covered(x).member_of(&s, "x", gr(Basis::ExactComputation, Check::None)));
    assert!(word(&g, &s, &e).unwrap_err().has(Inconsistency::I6));

    // An existence statement and a cut-off search.
    let f = Covers::search_set("R", "retained letters", "three letters");
    let input = poses("state 7302");
    let e3 = Key::new(sid("E3"), scope_in(f.clone(), input.clone()));
    let s4 = Key::new(sid("S4"), scope_in(f.clone(), input.clone()));
    let base3 = || {
        let mut g = Graph::new();
        pair(&mut g, existence("E3"), universal("S4"));
        g.root(&e3);
        g.root(&s4);
        g
    };
    let mut g = base3();
    g.attach_search(&e3, SearchRecord::new(&sid("E3"), &f, Complete::No, Stop::DeterministicBudget, JobOutcome::CutOff));
    assert_eq!(row(&g, &e3, &e), 5);
    let mut g = base3();
    g.attach_search(&e3, SearchRecord::new(&sid("E3"), &f, Complete::No, Stop::Other("wall clock".into()), JobOutcome::CutOff));
    assert_eq!(row(&g, &e3, &e), 6);
    let mut g = base3();
    let cut = SearchRecord::new(&sid("E3"), &f, Complete::No, Stop::Other("wall clock".into()), JobOutcome::CutOff);
    g.support(&s4, Leaf::new(&s4, exhaustive()).covered(Covered::new(f.clone(), Some(input.clone()))).search(cut));
    assert_eq!(word(&g, &e3, &e).unwrap_err().code, NEGATIVE_WITHOUT_COMPLETION);

    // A plain existence statement refuted by a proof, a complete certificate, or the guarded exhaustive search.
    let mut g = base3();
    g.support(&s4, Leaf::new(&s4, th()));
    assert_eq!(row(&g, &e3, &e), 1);
    let mut g = base3();
    let cert = Leaf::new(&s4, gr(Basis::Certificate, Check::IndependentChecker))
        .covered(Covered::new(f.clone(), Some(input.clone())))
        .complete();
    g.support(&s4, cert.clone());
    assert_eq!(row(&g, &e3, &e), 1);
    let mut cert_incomplete = cert;
    cert_incomplete.complete_certificate = false;
    let mut g = base3();
    g.support(&s4, cert_incomplete);
    assert!(word(&g, &e3, &e).unwrap_err().has(Inconsistency::I6));
    let guarded = |stop: Stop| {
        Leaf::new(&s4, exhaustive())
            .covered(Covered::new(f.clone(), Some(input.clone())))
            .search(SearchRecord::new(&sid("E3"), &f, Complete::Yes, stop, JobOutcome::Completed))
            .job(ProducingJob::new("search 7303", JobOutcome::Completed))
    };
    let mut g = base3();
    g.support(&s4, guarded(Stop::Closed));
    let w = word(&g, &e3, &e).unwrap();
    assert_eq!((w.row, w.claim), (1, Claim::Refuted));
    assert!(w.phrase_required);
    assert_eq!(w.display(), "refuted, three letters of family R only");
    assert_eq!(row(&g, &s4, &e), 2);
    // Any broken part of the guard is I6 for both sides (here: the stop is not closed).
    let mut g = base3();
    g.support(&s4, guarded(Stop::DeclaredLimit));
    for k in [&e3, &s4] {
        let err = word(&g, k, &e).unwrap_err();
        assert_eq!(err.code, INCONSISTENT_FIELDS);
        assert!(err.has(Inconsistency::I6));
    }

    // Mixed with outer quantifier universal: a witness plus a complete negative at it.
    let m = key("M", d7.clone());
    let nm = key("not M", d7.clone());
    let basem = || {
        let mut g = Graph::new();
        pair(
            &mut g,
            Statement::new(sid("M")).form(Form::Mixed(Quantifier::Forall)),
            Statement::new(sid("not M")).form(Form::Mixed(Quantifier::Exists)),
        );
        g.root(&m);
        g
    };
    let witness = || Leaf::new(&nm, exact()).covered(Covered::new(Covers::OneInstance, Some(poses("w")))).witness();
    let mut g = basem();
    let w1 = g.add_leaf(witness());
    let neg = g.add_leaf(Leaf::new(&nm, th()));
    let and = g.add_and(&[w1, neg]).unwrap();
    g.add_or_child(&nm, and).unwrap();
    assert_eq!(row(&g, &m, &e), 1);
    let mut g = basem();
    g.support(&nm, witness());
    assert!(word(&g, &m, &e).unwrap_err().has(Inconsistency::I6));
    let mut g = basem();
    g.support(&nm, Leaf::new(&nm, th()));
    assert!(word(&g, &m, &e).unwrap_err().has(Inconsistency::I6), "a negative without its witness");

    // Mixed with outer quantifier existential: a proof or a complete certificate.
    let x2 = key("X", d7.clone());
    let nx = key("not X", d7.clone());
    let basex = || {
        let mut g = Graph::new();
        pair(
            &mut g,
            Statement::new(sid("X")).form(Form::Mixed(Quantifier::Exists)),
            Statement::new(sid("not X")).form(Form::Mixed(Quantifier::Forall)),
        );
        g.root(&x2);
        g
    };
    let mut g = basex();
    g.support(&nx, Leaf::new(&nx, th()));
    assert_eq!(row(&g, &x2, &e), 1);
    let mut g = basex();
    g.support(
        &nx,
        Leaf::new(&nx, gr(Basis::Certificate, Check::IndependentChecker)).covered(Covered::of_scope(&nx.scope)).complete(),
    );
    assert_eq!(row(&g, &x2, &e), 1);
    // The finite-set case does not extend to mixed(exists): a completed exhaustive search of exactly its declared set
    // is no refutation (I6), for both sides of the pair.
    let fx = Covers::search_set("R", "retained letters", "three letters");
    let xf = key("X over F", fx.clone());
    let nxf = key("not X over F", fx.clone());
    let mut g = Graph::new();
    pair(
        &mut g,
        Statement::new(sid("X over F")).form(Form::Mixed(Quantifier::Exists)),
        Statement::new(sid("not X over F")).form(Form::Mixed(Quantifier::Forall)),
    );
    g.root(&xf);
    g.support(
        &nxf,
        Leaf::new(&nxf, exhaustive())
            .covered(Covered::of_scope(&nxf.scope))
            .search(SearchRecord::new(&sid("X over F"), &fx, Complete::Yes, Stop::Closed, JobOutcome::Completed)),
    );
    for k in [&xf, &nxf] {
        let err = word(&g, k, &e).unwrap_err();
        assert_eq!(err.code, INCONSISTENT_FIELDS);
        assert!(err.has(Inconsistency::I6));
    }
}

// --- CT-EVD-13 ------------------------------------------------------------------------------------------------

/// CT-EVD-13 (whole): EVD.27 and EVD.44. A checked witness found by sampling proves its existence statement, because
/// its leaf's basis is exact computation, not sample; two samples stay below every threshold inside U0, and sampling
/// never upgrades a universal claim.
#[test]
fn ct_evd_13_sampled_witness_proves_existence_and_samples_never_do() {
    let e = ed();
    let f = Covers::search_set("R", "retained letters", "three letters");
    let input = poses("state 7302");
    let ex = Key::new(sid("E"), scope_in(f.clone(), input.clone()));
    let un = Key::new(sid("not E"), scope_in(f.clone(), input.clone()));
    let witness = Leaf::new(&ex, exact())
        .covered(Covered::new(Covers::OneInstance, Some(poses("the sampled word"))))
        .member_of(&un, "the sampled word is in family R with three letters", exact())
        .support("found by sampling");
    let mut g = Graph::new();
    pair(&mut g, existence("E"), universal("not E"));
    g.support(&ex, witness);
    let w = word(&g, &ex, &e).unwrap();
    assert_eq!((w.row, w.word, w.claim), (2, WordKind::FiniteInstance, Claim::Proved));
    assert_eq!(row(&g, &un, &e), 1);

    // Two samples, of any checks, never stand.
    let mut g = Graph::new();
    pair(&mut g, existence("E"), universal("not E"));
    g.support(&ex, Leaf::new(&ex, gr(Basis::Sample, Check::IndependentChecker)));
    g.support(&ex, Leaf::new(&ex, gr(Basis::Sample, Check::FormalProof)));
    g.support(&un, Leaf::new(&un, gr(Basis::Sample, Check::HumanReview)));
    g.support(&un, Leaf::new(&un, gr(Basis::Sample, Check::ModelReview)));
    let mut n = 0;
    for_each_threshold(|t| {
        let ed_u = e.with_threshold(t).expect("up-set in U0");
        assert_eq!(row(&g, &ex, &ed_u), 7);
        assert_eq!(row(&g, &un, &ed_u), 7);
        n += 1;
    });
    assert_eq!(n, 83_521);
    let alts = summary(&g, &ex, &e).unwrap();
    assert_eq!(alts.len(), 2, "both incomparable samples stay as alternatives (EVD.27)");
}

// --- CT-EVD-22 ------------------------------------------------------------------------------------------------

/// CT-EVD-22 (whole): EVD.18, EVD.40, EVD.41, EVD.59 and Theorem D (b2). A leaf narrower than its statement reads row
/// 0 on the stored path and is refused at entry; an absent `covered` is inert, so the word reads as without the leaf,
/// under every threshold inside U0; a mixed(exists) refutation by a proof or a complete certificate passes.
#[test]
fn ct_evd_22_narrower_support_never_upgrades() {
    let e = ed();
    let d7 = Covers::admitted_domain("D[7]", "k = 7");
    let u = key("U", d7.clone());
    let base = || {
        let mut g = Graph::new();
        g.add_statement(universal("U").label("research record")).unwrap();
        g.root(&u);
        g
    };
    let narrow = Leaf::new(&u, exact()).covered(Covered::new(Covers::OneInstance, Some(poses("one state"))));

    let mut g = base();
    g.support(&u, narrow.clone());
    let err = word(&g, &u, &e).unwrap_err();
    assert_eq!(err.code, INCONSISTENT_FIELDS);
    assert!(err.has(Inconsistency::I7));

    let mut g = base();
    let before = g.clone();
    assert_eq!(offer(&mut g, &u, narrow).unwrap_err().code, SCOPE_MISMATCH);
    assert_eq!(g, before, "a refused offer changes nothing");

    // Absent covered: inert, for a universal and for a statement with no form fact.
    let mut g = base();
    g.support(&u, Leaf::new(&u, exact()));
    g.support(&u, Leaf::new(&u, gr(Basis::Certificate, Check::IndependentChecker)));
    let plain = base();
    let mut n = 0;
    for_each_threshold(|t| {
        if n % 97 == 0 {
            let ed_u = e.with_threshold(t).expect("up-set in U0");
            assert_eq!(word(&g, &u, &ed_u), word(&plain, &u, &ed_u));
        }
        n += 1;
    });
    let nf = key("no form", d7.clone());
    let mut g = Graph::new();
    g.add_statement(Statement::new(sid("no form"))).unwrap();
    g.support(&nf, Leaf::new(&nf, exact()).covered(Covered::new(Covers::OneInstance, None)));
    g.support(&nf, Leaf::new(&nf, exact()));
    assert_eq!(row(&g, &nf, &e), 7);

    // Mixed(exists) refuted by a proof or a complete certificate.
    let x = key("X", d7.clone());
    let nx = key("not X", d7.clone());
    for leaf in [
        Leaf::new(&nx, th()),
        Leaf::new(&nx, gr(Basis::Certificate, Check::IndependentChecker)).covered(Covered::of_scope(&nx.scope)).complete(),
    ] {
        let mut g = Graph::new();
        pair(
            &mut g,
            Statement::new(sid("X")).form(Form::Mixed(Quantifier::Exists)),
            Statement::new(sid("not X")).form(Form::Mixed(Quantifier::Forall)),
        );
        g.root(&x);
        g.support(&nx, leaf);
        let w = word(&g, &x, &e).unwrap();
        assert_eq!((w.row, w.claim), (1, Claim::Refuted));
    }
}
