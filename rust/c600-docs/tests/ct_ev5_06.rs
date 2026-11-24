//! CT-EV5-06 (whole: EV5.13: chapter 06 derives explanation words from the facts envelope, with statement, scope,
//! basis, check and job outcome separate; source labels remain visible, no writer asserts formal proof in 1.0, and
//! evidence, score and permission never merge), on the explanations of the three built-in tools and the library's
//! twelve modules. The words come from `c600_evidence::word` at the pinned edition over a graph of the records' stored
//! facts; the explanation keeps each field apart and supplies no summary of its own. A pass is source evidence.

mod common;

use c600_docs::explain::{self, instance_key, ClaimView, FixtureOutcome, NO_REVIEW};
use c600_docs::record::{Part, PrincipleRecord, State};
use c600_evidence::{
    offer, word, Basis, Check, Covered, Covers, Edition, Form, Grade, Graph, JobOutcome, Leaf, ProducingJob, Statement,
};

use common::{subjects, Subject};

fn explanation(s: &Subject, record: &PrincipleRecord, fixture: FixtureOutcome, author: &str) -> explain::Explanation {
    let checked = s.checked();
    let decompositions = s.decompositions(&checked);
    let refs: Vec<_> = decompositions.iter().collect();
    explain::explain(record, &State::Current, &refs, fixture, author, None, &Edition::draft_1())
}

/// The word chapter 06 gives one finite instance with a completed producing job of `outcome`, built here directly
/// on `c600-evidence`: the instance claim's word must be this one, not a wording of the docs crate.
fn instance_word(record: &PrincipleRecord, failed: bool) -> c600_evidence::Word {
    let i = record.instance.given().unwrap();
    let key = instance_key(record, &i.input);
    let mut g = Graph::new();
    let _ = g.add_statement(Statement::new(key.statement.clone()).form(Form::Finite));
    let mut leaf = Leaf::new(&key, Grade::new(Basis::ExactComputation, Check::Producer))
        .covered(Covered::new(Covers::OneInstance, key.scope.input.clone()))
        .support(&i.fixture)
        .job(ProducingJob::new(&i.fixture, JobOutcome::Completed));
    leaf.failed = failed;
    offer(&mut g, &key, leaf).unwrap();
    word(&g, &key, &Edition::draft_1()).unwrap()
}

fn fields(c: &ClaimView) -> (String, String) {
    (c.statement.clone(), c.scope.clone())
}

/// The words are chapter 06's, and each claim keeps its statement, scope, bases, checks and job outcome apart. The
/// instance's word is the one `c600_evidence::word` gives a completed fixture on one instance; a failed fixture's leaf
/// carries no basis and gives another word, while the statement and the scope stay. An argument with evidence shows
/// its basis and check beside the word; an argument without evidence shows neither. Changing only the check changes
/// the check field and nothing else of the claim's identity.
#[test]
fn ct_ev5_06_words_come_from_chapter_06_with_their_fields_apart() {
    for s in subjects() {
        let record = s.record();
        let passed = explanation(&s, &record, FixtureOutcome::Passed, "built_in");
        let failed = explanation(&s, &record, FixtureOutcome::Failed, "built_in");
        let (p, f) = (passed.instance.as_ref().unwrap(), failed.instance.as_ref().unwrap());
        assert_eq!(p.word, instance_word(&record, false), "{}: the word is chapter 06's", s.module);
        assert_eq!(p.job_outcome.as_deref(), Some("Completed"));
        assert_eq!(
            (p.bases.clone(), p.checks.clone()),
            (vec!["exact_computation".to_string()], vec!["producer".to_string()])
        );
        assert_eq!(fields(p), fields(f), "{}: a failed fixture keeps the statement and the scope", s.module);
        assert!(f.bases.is_empty() && f.word != p.word, "{}: a failed leaf supports nothing", s.module);
        assert_eq!(p.edition, Edition::draft_1().name());

        for (a, view) in record.justification.given().unwrap().iter().zip(&passed.arguments) {
            match &a.evidence {
                Part::Given(e) => {
                    assert_eq!(view.claim.bases, vec![e.basis.clone()], "{}", s.module);
                    assert_eq!(view.claim.checks, vec![e.check.clone()], "{}", s.module);
                }
                Part::None(_) => assert!(view.claim.bases.is_empty() && view.claim.checks.is_empty()),
            }
            assert!(!view.claim.scope.is_empty());
        }

        // The check alone changes: model review for producer.
        let mut reviewed = record.clone();
        let mut touched = false;
        if let Part::Given(args) = &mut reviewed.justification {
            for a in args.iter_mut() {
                if let Part::Given(e) = &mut a.evidence {
                    e.check = "model_review".into();
                    touched = true;
                }
            }
        }
        if touched {
            let changed = explanation(&s, &reviewed, FixtureOutcome::Passed, "built_in");
            for (a, b) in passed.arguments.iter().zip(&changed.arguments) {
                assert_eq!(fields(&a.claim), fields(&b.claim), "{}", s.module);
                assert_eq!(a.claim.bases, b.claim.bases);
                if !a.claim.checks.is_empty() {
                    assert_eq!(b.claim.checks, vec!["model_review".to_string()]);
                }
            }
        }
        let page = explain::render(&passed);
        assert!(page.contains("  - Statement: `") && page.contains("  - Basis: [") && page.contains("; check: ["));
    }
}

/// Source labels and check references remain inspectable: each argument with evidence shows its source label and
/// the digest of the evidence it rests on, and the page prints the labels.
#[test]
fn ct_ev5_06_source_labels_and_check_references_stay_visible() {
    for s in subjects() {
        let record = s.record();
        let e = explanation(&s, &record, FixtureOutcome::NotRun, "built_in");
        let page = explain::render(&e);
        for (a, view) in record.justification.given().unwrap().iter().zip(&e.arguments) {
            if let Part::Given(ev) = &a.evidence {
                assert!(view.claim.source_labels.contains(&ev.label), "{}: {:?}", s.module, view.claim.source_labels);
                assert!(view.claim.check_references.contains(&ev.sha256), "{}", s.module);
                assert!(page.contains(&ev.label), "{}: the page shows the label", s.module);
            }
        }
    }
}

/// EVD.64 assigns no writer to `formal proof` in 1.0: a record whose evidence claims a formal proof is refused at
/// admission, and its explanation records the sink's refusal and shows no formal proof check. An independent checker
/// is no host write either. Where no human review or formal proof is recorded, the explanation says so beside the
/// word; a human review the host records (with its 64-hex review digest) lifts that note.
#[test]
fn ct_ev5_06_no_writer_asserts_formal_proof_in_1_0() {
    let s = subjects().into_iter().find(|s| s.module == "words").unwrap();
    let record = s.record();
    let plain = explanation(&s, &record, FixtureOutcome::NotRun, "built_in");
    assert_eq!(plain.arguments[0].claim.review_note, Some(NO_REVIEW));
    assert!(explain::render(&plain).contains(NO_REVIEW));

    for (check, refused) in [("formal_proof", true), ("independent_checker", true), ("human_review", false)] {
        let mut changed = record.clone();
        if let Part::Given(args) = &mut changed.justification {
            if let Part::Given(e) = &mut args[0].evidence {
                e.check = check.into();
            }
        }
        let e = explanation(&s, &changed, FixtureOutcome::NotRun, "built_in");
        let claim = &e.arguments[0].claim;
        assert_eq!(claim.refused.is_some(), refused, "{check}: {:?}", claim.refused);
        if refused {
            assert!(claim.checks.is_empty(), "{check}: a refused write is not in the graph");
            assert_eq!(claim.review_note, Some(NO_REVIEW));
        } else {
            assert_eq!(claim.checks, vec![check.to_string()]);
            assert_eq!(claim.review_note, None, "a recorded human review lifts the note");
        }
    }
    let mut formal = record.clone();
    if let Part::Given(args) = &mut formal.justification {
        if let Part::Given(e) = &mut args[0].evidence {
            e.check = "formal_proof".into();
        }
    }
    let admission = s.admit(&formal);
    assert_eq!(admission.state, State::Refused);
    assert!(admission.findings.iter().any(|f| f.detail.contains("formal proof has no writer in 1.0")));
}

/// Evidence, score and permission never merge (INV-5, BM2): the author class is shown as data and no word reads it,
/// a fixture's pass leaves the arguments' words as they are, the explanation takes no score and grants nothing, and
/// its permission line is the same for every author.
#[test]
fn ct_ev5_06_evidence_score_and_permission_never_merge() {
    for s in subjects() {
        let record = s.record();
        let built_in = explanation(&s, &record, FixtureOutcome::Passed, "built_in");
        for author in ["user", "fork author"] {
            let other = explanation(&s, &record, FixtureOutcome::Passed, author);
            assert_eq!(other.author_class, author);
            assert_eq!(other.arguments, built_in.arguments, "{}: the author class changes no word", s.module);
            assert_eq!(other.instance, built_in.instance);
            assert_eq!(other.permission, built_in.permission);
        }
        let not_run = explanation(&s, &record, FixtureOutcome::NotRun, "built_in");
        assert_eq!(not_run.arguments, built_in.arguments, "{}: a fixture's pass upgrades no argument", s.module);
        let page = explain::render(&built_in).to_lowercase();
        for merged in ["score", "competence", "granted"] {
            assert!(!page.contains(merged), "{}: the page says `{merged}`", s.module);
        }
    }
}
