//! Chapter 06 section 9: the worked examples E1, E2 and E3, transcribed from the chapter's tables into synthetic
//! graphs. Expected words come from those tables and the clauses they cite (EVD.63, section 5's (c1) to (c3), EVD.42).
//! The records themselves are not imported: the label-table importer (EVD.48 to EVD.51) is not implemented, so the
//! forms of E1 to E3 come from the test's own statements, as section 13's open item on the `form` fact allows.

mod common;

use c600_evidence::*;
use common::*;

/// EVD.58's table.
const INCONSISTENT_FIELDS: &str = "EVD:evidence.inconsistent_fields";
const NEGATIVE_WITHOUT_COMPLETION: &str = "EVD:evidence.negative_without_completion";
const PREMISE_UNMET: &str = "EVD:evidence.premise_unmet";
const SCOPE_MISMATCH: &str = "EVD:evidence.scope_mismatch";

// --- E1 -------------------------------------------------------------------------------------------------------

fn e1() -> (Graph, Key) {
    let start = poses("b47d8474: start pose");
    let s1 = Key::new(sid("S1: 104:8,46:4,104:15 from b47d8474 ends at 1db0dd54"), scope_in(Covers::OneInstance, start.clone()));
    let mut g = Graph::new();
    g.add_statement(Statement::new(s1.statement.clone()).form(Form::Finite).subject(Level::Pose)).unwrap();
    let covered = Covered::new(Covers::OneInstance, Some(start));
    // a1: (certificate, producer) from results/certificates/R01.json, and (theorem, model review) from Theorem MC.
    let cert = g.add_leaf(
        Leaf::new(&s1, gr(Basis::Certificate, Check::Producer)).covered(covered.clone()).support("R01.json"),
    );
    let mc = g.add_leaf(Leaf::new(&s1, th()).support("Theorem MC"));
    let a1 = g.add_and(&[cert, mc]).unwrap();
    g.add_or_child(&s1, a1).unwrap();
    // a2 and a3: (exact computation, producer) from the two replay paths.
    g.support(&s1, Leaf::new(&s1, exact()).covered(covered.clone()).support("macro.py check --replay"));
    g.support(&s1, Leaf::new(&s1, exact()).covered(covered).support("replay_word.py"));
    (g, s1)
}

/// CT-EVD-12 (part: E1 as a synthetic transcription of section 9; not run: the import of the actual records through
/// the label table, EVD.48 to EVD.51): EVD.63. At the trial threshold U0 S1 reads finite instance; a threshold that
/// requires a check above producer need not keep that word, because neither replay path gains an independent-checker
/// upgrade; with the replay paths distrusted, a1 alone keeps the word.
#[test]
fn ct_evd_12_e1_macro_certificate() {
    let (g, s1) = e1();
    let e = ed();
    let w = word(&g, &s1, &e).unwrap();
    assert_eq!((w.row, w.word, w.claim), (2, WordKind::FiniteInstance, Claim::Proved));
    assert_eq!(w.scope_phrase, "on one instance");

    // A threshold requiring a later check: (b, k) with b != sample and k above producer. Two producer replays do not
    // add up to an independent checker, and a1's certificate is a producer check too.
    let later: std::collections::BTreeSet<Grade> =
        u0().into_iter().filter(|g| !matches!(g.check, Check::None | Check::Producer)).collect();
    let w = word(&g, &s1, &e.with_threshold(&later).unwrap()).unwrap();
    assert_eq!((w.row, w.claim), (7, Claim::Open));

    // U0 without (exact computation, producer): a1 alone keeps finite instance.
    let no_replays: std::collections::BTreeSet<Grade> = u0().into_iter().filter(|g| *g != exact()).collect();
    assert_eq!(row(&g, &s1, &e.with_threshold(&no_replays).unwrap()), 2);
}

// --- E2 -------------------------------------------------------------------------------------------------------

struct E2 {
    g: Graph,
    implication: Key,
    sel: Key,
    bound: Key,
    u: Key,
    tz: Key,
    reach_z: Key,
    adm_rejected: Key,
    not_adm_rejected: Key,
    t_rejected: Key,
    reach_rejected: Key,
}

/// E2's graph; `outside` adds the `OUTSIDE_CERTIFIED_DOMAIN(k)` leaf for the rejected Z'.
fn e2(outside: bool) -> E2 {
    let dk = Covers::admitted_domain("D[k]", "");
    let mut g = Graph::new();

    // The fixed-L implication holds Sel as a guard (an open hypothesis): proved, given Sel.
    let implication = key("fixed-L implication", dk.clone());
    g.add_statement(universal("fixed-L implication").guard(PremiseClass::OpenHypothesis, "Sel")).unwrap();
    g.support(&implication, Leaf::new(&implication, th()));

    // Sel: every admitted nonterminal state has a legal selected macro. A mixed statement, outer quantifier universal.
    let sel = key("Sel", dk.clone());
    pair(
        &mut g,
        Statement::new(sid("Sel")).form(Form::Mixed(Quantifier::Forall)),
        Statement::new(sid("not Sel")).form(Form::Mixed(Quantifier::Exists)),
    );

    // The bound: AND(implication, premise Sel).
    let bound = Key::new(sid("cost <= L*R_L(Z0)"), scope_in(Covers::OneInstance, poses("Z0")));
    g.add_statement(finite("cost <= L*R_L(Z0)")).unwrap();
    let by_implication = g.add_premise_discharged_by(&implication, PremiseClass::OpenHypothesis, &implication);
    let on_sel = g.add_premise(&sel, PremiseClass::OpenHypothesis);
    let and = g.add_and(&[by_implication, on_sel]).unwrap();
    g.add_or_child(&bound, and).unwrap();

    // U(h, k) on D[k], whose reachability promise is a guard of the statement.
    let u = key("U(h, k)", dk.clone());
    g.add_statement(universal("U(h, k)").guard(PremiseClass::ReachabilityPromise, "reach")).unwrap();
    g.support(&u, Leaf::new(&u, th()));

    // T(Z) = AND(theorem on D[k], admission(Z), reach(Z)) for a concrete Z, as section 5's (c2) writes it.
    let apply = |g: &mut Graph, z: &str| -> (Key, Key, Key, Key) {
        let at = poses(z);
        let t = Key::new(sid(&format!("T({z})")), scope_in(Covers::OneInstance, at.clone()));
        let adm = Key::new(sid(&format!("Adm_k({z})")), scope_in(Covers::OneInstance, at.clone()));
        let not_adm = Key::new(sid(&format!("not Adm_k({z})")), scope_in(Covers::OneInstance, at.clone()));
        let reach = Key::new(sid(&format!("reach({z})")), scope_in(Covers::OneInstance, at));
        g.add_statement(finite(&format!("T({z})"))).unwrap();
        pair(g, finite(&format!("Adm_k({z})")), finite(&format!("not Adm_k({z})")));
        g.add_statement(finite(&format!("reach({z})"))).unwrap();
        g.root(&reach);
        let on_u = g.add_premise_discharged_by(&u, PremiseClass::OpenHypothesis, &u);
        let on_adm = g.add_premise_discharged_by(&adm, PremiseClass::OpenHypothesis, &adm);
        let on_reach = g.add_premise(&reach, PremiseClass::ReachabilityPromise);
        let and = g.add_and(&[on_u, on_adm, on_reach]).unwrap();
        g.add_or_child(&t, and).unwrap();
        (t, adm, not_adm, reach)
    };
    let (tz, adm_z, _, reach_z) = apply(&mut g, "Z");
    g.support(&adm_z, Leaf::new(&adm_z, exact()));
    // A rejected Z': OUTSIDE_CERTIFIED_DOMAIN(k) is a leaf for not Adm_k(Z').
    let (t_rejected, adm_rejected, not_adm_rejected, reach_rejected) = apply(&mut g, "Z'");
    if outside {
        g.support(&not_adm_rejected, Leaf::new(&not_adm_rejected, exact()));
    }

    E2 { g, implication, sel, bound, u, tz, reach_z, adm_rejected, not_adm_rejected, t_rejected, reach_rejected }
}

/// CT-EVD-12 (part: E2 as a synthetic transcription of section 9; not run: the import of the actual records through
/// the label table, EVD.48 to EVD.51): the implication reads proved, given Sel; the bound reads conditional on Sel;
/// U(h, k) reads "proved on D[k], for reachable inputs"; U applied to a concrete Z reads conditional on reach(Z); for a
/// rejected Z, not Adm_k(Z) reads finite instance and T(Z) reads open, not refuted (section 5's (c2) and (c3)). A
/// macro found at one state discharges nothing, and a failed budgeted search at one state does not refute Sel.
#[test]
fn ct_evd_12_e2_conditional_bound() {
    let E2 { g, implication, sel, bound, u, tz, reach_z, adm_rejected, not_adm_rejected, t_rejected, reach_rejected } =
        e2(true);
    let e = ed();

    let w = word(&g, &implication, &e).unwrap();
    assert_eq!((w.row, w.word, w.claim), (3, WordKind::Proved, Claim::Proved));
    assert!(w.phrase_required);
    assert!(w.display().starts_with("proved") && w.display().ends_with(", given Sel"), "{}", w.display());

    let w = word(&g, &bound, &e).unwrap();
    assert_eq!((w.row, w.word, w.claim), (4, WordKind::Conditional, Claim::Conditional));
    assert_eq!(w.premises, vec![sel.clone()]);

    let w = word(&g, &u, &e).unwrap();
    assert_eq!((w.row, w.claim), (3, Claim::Proved));
    assert_eq!(w.display(), "proved on D[k], for reachable inputs");

    let w = word(&g, &tz, &e).unwrap();
    assert_eq!((w.row, w.claim), (4, Claim::Conditional));
    assert_eq!(w.premises, vec![reach_z.clone()]);

    // The rejected Z' (section 5's (c3)): not Adm reads finite instance and T(Z') falls to open, not refuted. The leaf
    // has no edge to reachability, so reach(Z') reads as it does in E2 without that leaf.
    assert_eq!(row(&g, &not_adm_rejected, &e), 2);
    assert_eq!(row(&g, &adm_rejected, &e), 1);
    let w = word(&g, &t_rejected, &e).unwrap();
    assert_eq!((w.row, w.claim), (7, Claim::Open));
    let bare = e2(false);
    assert_eq!(word(&g, &reach_rejected, &e), word(&bare.g, &reach_rejected, &e));

    // A macro found at one state has its own key and discharges nothing.
    let mut g1 = g.clone();
    let at_x = Key::new(sid("Sel at x"), scope_in(Covers::OneInstance, poses("x")));
    g1.add_statement(finite("Sel at x")).unwrap();
    g1.support(&at_x, Leaf::new(&at_x, exact()));
    assert_eq!(row(&g1, &at_x, &e), 2);
    assert_eq!(discharge(&g1, &sel, &at_x, &e).unwrap_err().code, PREMISE_UNMET);
    assert_eq!(word(&g1, &bound, &e), word(&g, &bound, &e));

    // A failed budgeted search at one state reads not found within budget there and refutes nothing.
    let mut g2 = g.clone();
    let at_y = Key::new(sid("Sel at y"), scope_in(Covers::OneInstance, poses("y")));
    g2.add_statement(finite("Sel at y")).unwrap();
    let macros = Covers::search_set("selected macros", "legal macros", "budget 10^6");
    g2.attach_search(
        &at_y,
        SearchRecord::new(&sid("Sel at y"), &macros, Complete::No, Stop::DeterministicBudget, JobOutcome::Completed),
    );
    assert_eq!(row(&g2, &at_y, &e), 5);
    assert_ne!(row(&g2, &sel, &e), 1);
    assert_eq!(word(&g2, &bound, &e), word(&g, &bound, &e));
    // Offered as a refutation of Sel (witness y plus that search as the negative at y), it is I5, not a refutation.
    let mut g3 = g.clone();
    let not_sel = key("not Sel", Covers::admitted_domain("D[k]", ""));
    let witness = g3.add_leaf(
        Leaf::new(&not_sel, exact()).covered(Covered::new(Covers::OneInstance, Some(poses("y")))).witness(),
    );
    let negative = g3.add_leaf(
        Leaf::new(&not_sel, exhaustive())
            .covered(Covered::new(macros.clone(), Some(poses("y"))))
            .search(SearchRecord::new(&sid("not Sel"), &macros, Complete::No, Stop::DeterministicBudget, JobOutcome::Completed)),
    );
    let and = g3.add_and(&[witness, negative]).unwrap();
    g3.add_or_child(&not_sel, and).unwrap();
    for k in [&sel, &bound] {
        assert_eq!(word(&g3, k, &e).unwrap_err().code, NEGATIVE_WITHOUT_COMPLETION);
    }
}

// --- E3 -------------------------------------------------------------------------------------------------------

/// The family-R set of E3: at most three retained letters.
fn f() -> Covers {
    Covers::search_set("R", "retained letters", "three letters")
}

/// E3 and S4 = neg E3 at one stop, each with its OR node.
fn e3_pair(state: &str, label: Option<&str>) -> (Graph, Key, Key) {
    let at = poses(state);
    let e3 = Key::new(sid("E3"), scope_in(f(), at.clone()));
    let s4 = Key::new(sid("S4"), scope_in(f(), at));
    let mut g = Graph::new();
    let (mut e, mut s) = (existence("E3"), universal("S4"));
    if let Some(l) = label {
        e = e.label(l);
        s = s.label(l);
    }
    pair(&mut g, e, s);
    g.root(&e3);
    g.root(&s4);
    (g, e3, s4)
}

/// The offered exhaustive negative of S4 with EVD.42's guard established, at the given job outcome.
fn guarded(e3: &Key, s4: &Key, job: JobOutcome) -> Leaf {
    Leaf::new(s4, exhaustive())
        .covered(Covered::of_scope(&e3.scope))
        .search(SearchRecord::new(&e3.statement, &f(), Complete::Yes, Stop::Closed, job.clone()))
        .job(ProducingJob::new("search 7303", job))
}

fn both_read(g: &Graph, e3: &Key, s4: &Key, code: &str, what: &str) {
    let e = ed();
    for k in [e3, s4] {
        let r = word(g, k, &e);
        assert_eq!(r.as_ref().err().map(|x| x.code), Some(code), "{what}: {k:?} gave {r:?}");
    }
}

/// CT-EVD-12 (part: E3 as a synthetic transcription of section 9; not run: the import of the actual records through
/// the label table, EVD.48 to EVD.51): the truncated beam reads not found within budget for both; the two-letter subset
/// has its own key and reads finite instance; the unbeamed witness gives E3 finite instance and S4 refuted; the
/// guarded 7303 search gives S4 finite instance and E3 refuted, both with "three letters of family R only"; a different
/// declared set, partial coverage, another failed guard, a matching label or an exact-computation grade gives row 0
/// inconsistent_fields for both; a job that did not complete keeps negative_without_completion; "not stated" stays row
/// 7; the cancelled runs read incomplete; the two-level stop proves the five-family negative only.
#[test]
fn ct_evd_12_e3_cut_off_search() {
    let e = ed();
    let beam = || SearchRecord::new(&sid("E3"), &f(), Complete::No, Stop::DeterministicBudget, JobOutcome::Completed);

    // Beam 64, truncated at three letters: job completed, search complete = no.
    let (mut g, e3, s4) = e3_pair("state 7302", None);
    g.attach_search(&e3, beam());
    for k in [&e3, &s4] {
        let w = word(&g, k, &e).unwrap();
        assert_eq!((w.row, w.word, w.claim), (5, WordKind::NotFoundWithinBudget, Claim::Open));
    }

    // The two-letter subset, exhaustive: its own key, finite instance, and the pair's words do not move.
    let two = Covers::search_set("R", "retained letters", "two letters");
    let s2 = Key::new(sid("S2: no two-letter R word lowers Phi"), scope_in(two.clone(), poses("state 7302")));
    let mut g2 = g.clone();
    g2.add_statement(universal("S2: no two-letter R word lowers Phi")).unwrap();
    g2.support(
        &s2,
        Leaf::new(&s2, exhaustive()).covered(Covered::of_scope(&s2.scope)).search(SearchRecord::new(
            &sid("E2: some two-letter R word lowers Phi"),
            &two,
            Complete::Yes,
            Stop::Closed,
            JobOutcome::Completed,
        )),
    );
    assert_ne!(s2, e3);
    assert_ne!(s2, s4);
    let w = word(&g2, &s2, &e).unwrap();
    assert_eq!((w.row, w.word), (2, WordKind::FiniteInstance));
    assert_eq!(word(&g2, &e3, &e), word(&g, &e3, &e));
    assert_eq!(word(&g2, &s4, &e), word(&g, &s4, &e));

    // The unbeamed search: a checked witness with scope membership. E3 reads finite instance and S4 refuted.
    let mut g3 = g.clone();
    g3.support(
        &e3,
        Leaf::new(&e3, exact()).covered(Covered::new(Covers::OneInstance, Some(poses("state 7302")))).member_of(
            &s4,
            "535:1,363:5,535:1,597:14 has three retained letters of family R",
            exact(),
        ),
    );
    let w = word(&g3, &e3, &e).unwrap();
    assert_eq!((w.row, w.word, w.claim), (2, WordKind::FiniteInstance, Claim::Proved));
    let w = word(&g3, &s4, &e).unwrap();
    assert_eq!((w.row, w.word, w.claim), (1, WordKind::Refuted, Claim::Refuted));

    // The 7303 stop, 0 words, with EVD.42's finite exhaustive guard established.
    let (mut g, e3, s4) = e3_pair("state 7303", None);
    g.support(&s4, guarded(&e3, &s4, JobOutcome::Completed));
    let w = word(&g, &s4, &e).unwrap();
    assert_eq!((w.row, w.word, w.claim), (2, WordKind::FiniteInstance, Claim::Proved));
    assert_eq!(format!("{}, {}", w.word.text(), w.scope_phrase), "finite instance, three letters of family R only");
    let w = word(&g, &e3, &e).unwrap();
    assert_eq!((w.row, w.word, w.claim), (1, WordKind::Refuted, Claim::Refuted));
    assert!(w.phrase_required, "row 1 through the guard prints its scope phrase");
    assert_eq!(w.display(), "refuted, three letters of family R only");

    // That negative from a completed job with a different declared set, partial coverage or another failed guard.
    let broken: Vec<(&str, Box<dyn Fn(&Key, &Key) -> Leaf>)> = vec![
        (
            "a different declared set, covered computed from it",
            Box::new(|e3, s4| {
                let mut q = guarded(e3, s4, JobOutcome::Completed);
                q.search = Some(SearchRecord::new(&e3.statement, &two, Complete::Yes, Stop::Closed, JobOutcome::Completed));
                q.covered = Some(Covered::new(two.clone(), e3.scope.input.clone()));
                q
            }),
        ),
        (
            "a different declared set under a covered set copied from the statement",
            Box::new(|e3, s4| {
                let mut q = guarded(e3, s4, JobOutcome::Completed);
                q.search = Some(SearchRecord::new(&e3.statement, &two, Complete::Yes, Stop::Closed, JobOutcome::Completed));
                q
            }),
        ),
        (
            "partial coverage: a subset of F",
            Box::new(|e3, s4| guarded(e3, s4, JobOutcome::Completed).covered(Covered::new(two.clone(), e3.scope.input.clone()))),
        ),
        (
            "a superset of F",
            Box::new(|e3, s4| {
                let four = Covers::search_set("R", "retained letters", "four letters");
                guarded(e3, s4, JobOutcome::Completed).covered(Covered::new(four, e3.scope.input.clone()))
            }),
        ),
        (
            "a stop that is not closed",
            Box::new(|e3, s4| {
                let mut q = guarded(e3, s4, JobOutcome::Completed);
                q.search.as_mut().unwrap().stop = Stop::DeclaredLimit;
                q
            }),
        ),
        (
            "a search record whose target is not E3",
            Box::new(|e3, s4| {
                let mut q = guarded(e3, s4, JobOutcome::Completed);
                q.search.as_mut().unwrap().target = s4.statement.clone();
                q
            }),
        ),
        (
            "an exact-computation grade alone",
            Box::new(|e3, s4| {
                let mut q = guarded(e3, s4, JobOutcome::Completed);
                q.grade = exact();
                q
            }),
        ),
    ];
    for (what, leaf) in &broken {
        let (mut g, e3, s4) = e3_pair("state 7303", None);
        g.support(&s4, leaf(&e3, &s4));
        both_read(&g, &e3, &s4, INCONSISTENT_FIELDS, what);
        // A matching label does not repair the guard.
        let (mut g, e3, s4) = e3_pair("state 7303", Some("exhaustive, complete, 0 words"));
        g.support(&s4, leaf(&e3, &s4));
        both_read(&g, &e3, &s4, INCONSISTENT_FIELDS, what);
    }

    // A job that did not complete keeps negative_without_completion; "not stated" stays row 7 with its label.
    for job in [JobOutcome::Cancelled, JobOutcome::CutOff, JobOutcome::Error, JobOutcome::NotRun] {
        let (mut g, e3, s4) = e3_pair("state 7303", None);
        g.support(&s4, guarded(&e3, &s4, job.clone()));
        both_read(&g, &e3, &s4, NEGATIVE_WITHOUT_COMPLETION, &format!("{job:?}"));
    }
    let (mut g, e3, s4) = e3_pair("state 7303", Some("0 words"));
    let mut q = guarded(&e3, &s4, JobOutcome::NotStated);
    q.producing_job = None;
    g.support(&s4, q);
    for k in [&e3, &s4] {
        let w = word(&g, k, &e).unwrap();
        assert_eq!((w.row, w.claim), (7, Claim::Open));
        assert_eq!(w.source_label.as_deref(), Some("0 words"));
    }

    // The runs at 7305 and 7306, stopped by the integrator: cancelled jobs read incomplete.
    let (mut g, e3, s4) = e3_pair("state 7305", None);
    g.attach_search(
        &e3,
        SearchRecord::new(
            &sid("E3"),
            &f(),
            Complete::No,
            Stop::Other("stopped by the integrator".into()),
            JobOutcome::Cancelled,
        ),
    );
    for k in [&e3, &s4] {
        let w = word(&g, k, &e).unwrap();
        assert_eq!((w.row, w.word, w.claim), (6, WordKind::Incomplete, Claim::Open));
    }
    let (mut g, e3, s4) = e3_pair("state 7306", None);
    g.attach_job(&e3, ProducingJob::new("run 7306", JobOutcome::Cancelled));
    for k in [&e3, &s4] {
        assert_eq!(row(&g, k, &e), 6);
    }

    // The two-level stop at Phi 498: a negative for the five declared families only; "no word descends there" stays
    // open, and the narrower search is kept out of its arguments.
    let at = poses("state at Phi 498");
    let five = Covers::search_set("five declared families", "retained letters", "declared bounds");
    let n5 = Key::new(sid("no word of the five families descends"), scope_in(five.clone(), at.clone()));
    let all = Key::new(sid("no word descends"), scope_in(Covers::admitted_domain("every word", ""), at.clone()));
    let mut g = Graph::new();
    g.add_statement(universal("no word of the five families descends")).unwrap();
    g.add_statement(universal("no word descends")).unwrap();
    g.root(&all);
    let narrow = Leaf::new(&n5, exhaustive()).covered(Covered::of_scope(&n5.scope)).search(SearchRecord::new(
        &sid("some word of the five families descends"),
        &five,
        Complete::Yes,
        Stop::Closed,
        JobOutcome::Completed,
    ));
    offer(&mut g, &n5, narrow.clone()).unwrap();
    assert_eq!(row(&g, &n5, &e), 2);
    assert_eq!(row(&g, &all, &e), 7);
    let before = g.clone();
    assert_eq!(offer(&mut g, &all, narrow.clone()).unwrap_err().code, SCOPE_MISMATCH);
    let mut rekeyed = narrow;
    rekeyed.key = all.clone();
    assert_eq!(offer(&mut g, &all, rekeyed).unwrap_err().code, SCOPE_MISMATCH);
    assert_eq!(g, before);
}
