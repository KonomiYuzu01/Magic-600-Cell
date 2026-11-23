//! CT-LB5-07 (chapter 03's test table; LB5.2, LB5.3, DET.30): one search key repeats coverage and stop reason; a
//! budget, clock or error stop yields no negative; a reduced negative cannot leave its family without a matching
//! reduction lemma.
//!
//! The search is language code, the research library's module `research_search`, run on `c600-run` with no host over
//! small synthetic permutation actions that the test also enumerates in Rust. These searches test the record contract
//! only: their outputs enter no research evidence and no search result counts as acceptance (LP5 plan, section 9,
//! reading B; PHS.31). The key is the canonical encoding of the declaration (DET.59), which the record keeps whole.
//! A pass is source and fixture evidence.
//!
//! CT-LB5-07 (source, part: the library's search record, stops and reduction gate on synthetic families; not run
//! here: the puzzle form over KN2.4's rows and a `ReproKey` of a job record, which are R8's, CT-DET-24).

mod research_common;

use std::cell::Cell;

use c600_digest::sha256_hex;
use c600_library::research::{REDUCTION_SCOPE_MISMATCH, SEARCH_MODULE, SEARCH_SOURCE};
use c600_run::encode::encode;
use c600_run::interp::{Cancel, Loaded, Options};
use c600_run::meter::{Amounts, Interruption, Outcome};
use c600_run::value::{equal, Value};
use research_common::*;

/// A search declaration as the test builds it; [`Decl::value`] gives the record `research_search.search` takes.
#[derive(Clone, Debug)]
struct Decl {
    action: Vec<Vec<u64>>,
    beam: Option<u64>,
    limit: u64,
    unit: &'static str,
    family: &'static str,
    goals: Vec<u64>,
    hypotheses: Vec<&'static str>,
    independent: Vec<(u64, u64)>,
    max_length: u64,
    reduction: &'static str,
    seed: Option<u64>,
    start: u64,
}

impl Decl {
    fn new(action: Vec<Vec<u64>>, start: u64, goals: Vec<u64>, max_length: u64) -> Decl {
        Decl {
            action,
            beam: None,
            limit: 1_000_000,
            unit: "candidates",
            family: "synthetic",
            goals,
            hypotheses: Vec::new(),
            independent: Vec::new(),
            max_length,
            reduction: "none",
            seed: None,
            start,
        }
    }

    fn value(&self) -> Value {
        let action = list(self.action.iter().map(|images| nats(images)).collect());
        let independent =
            list(self.independent.iter().map(|&(a, b)| record(vec![("a", nat(a)), ("b", nat(b))])).collect());
        record(vec![
            ("action", action),
            (
                "beam",
                record(vec![("applicable", boolean(self.beam.is_some())), ("width", nat(self.beam.unwrap_or(0)))]),
            ),
            ("budget", record(vec![("limit", nat(self.limit)), ("unit", text(self.unit))])),
            ("family", text(self.family)),
            ("goals", nats(&self.goals)),
            ("hypotheses", texts(&self.hypotheses)),
            ("independent", independent),
            ("input_sha256", text("3f1d4ac0f3c9e7d0a3bb3a8b0d6f3e5c1f2b0a9d8c7e6f5a4b3c2d1e0f9a8b7c")),
            ("max_length", nat(self.max_length)),
            ("module_sha256", text(&c600_library::research::search_sha256())),
            ("reduction", text(self.reduction)),
            (
                "seed",
                record(vec![("applicable", boolean(self.seed.is_some())), ("value", nat(self.seed.unwrap_or(0)))]),
            ),
            ("start", nat(self.start)),
        ])
    }

    /// The search key: the SHA-256 of the declaration's canonical encoding (DET.59).
    fn key(&self) -> String {
        let mut bytes = Vec::new();
        encode(&self.value(), &mut bytes).expect("a declaration has a canonical form");
        sha256_hex(&bytes)
    }

    fn independent(&self, x: u64, y: u64) -> bool {
        self.independent.iter().any(|&(a, b)| (a, b) == (x, y) || (a, b) == (y, x))
    }

    /// Every word of the declared family, by length, then letter order, with the reduction applied.
    fn family_words(&self) -> Vec<Vec<u64>> {
        let letters = self.action.len() as u64;
        let mut level = vec![Vec::new()];
        let mut all = level.clone();
        for _ in 0..self.max_length {
            let mut next = Vec::new();
            for word in &level {
                for a in 0..letters {
                    let skipped = self.reduction == "local_commuting"
                        && word.last().is_some_and(|&last| a < last && self.independent(last, a));
                    if !skipped {
                        next.push([word.clone(), vec![a]].concat());
                    }
                }
            }
            all.extend(next.iter().cloned());
            level = next;
        }
        all
    }

    fn end(&self, word: &[u64]) -> u64 {
        word.iter().fold(self.start, |p, &a| self.action[a as usize][p as usize])
    }

    fn witnesses(&self, words: &[Vec<u64>]) -> Vec<Vec<u64>> {
        words.iter().filter(|w| self.goals.contains(&self.end(w))).cloned().collect()
    }
}

fn swap(points: u64, a: u64, b: u64) -> Vec<u64> {
    (0..points)
        .map(|p| {
            if p == a {
                b
            } else if p == b {
                a
            } else {
                p
            }
        })
        .collect()
}

fn search_module() -> Loaded {
    loaded(&[SEARCH_SOURCE])
}

fn search(loaded: &Loaded, decl: &Decl) -> Value {
    call(loaded, SEARCH_MODULE, "search", vec![decl.value()])
}

fn outcome(record: &Value) -> (String, String, bool) {
    (as_text(field(record, "outcome")), as_text(field(record, "stop")), as_bool(field(record, "negative")))
}

/// A seeded synthetic action: `letters` permutations of `points` points.
fn action(g: &mut Fixtures, letters: usize, points: usize) -> Vec<Vec<u64>> {
    (0..letters).map(|_| g.permutation(points)).collect()
}

/// One search key repeats its coverage and stop reason: for completed, budget-stopped, beamed and reduced searches
/// over seeded actions, two runs of one declaration give equal records, and the record keeps the declaration whole
/// (family, alphabet, bounds, beam, seed, budget, cost unit and identities) under the same key. The coverage is the
/// test's own enumeration's: a completed search examines every word of the declared (reduced) family and counts its
/// witnesses and their least length. A changed budget is another key, with its own coverage. A beamed search covers
/// completely only the lengths before the first level the beam truncated, never every length once it discarded.
#[test]
fn ct_lb5_07_one_search_key_repeats_coverage_and_stop_reason() {
    let loaded = search_module();
    let mut g = Fixtures::new(0x5175_0107);
    let mut truncated = 0;
    for round in 0..24 {
        let (letters, points) = (1 + g.below(3) as usize, 2 + g.below(5) as usize);
        let mut decl = Decl::new(
            action(&mut g, letters, points),
            g.below(points as u64),
            vec![g.below(points as u64)],
            1 + g.below(4),
        );
        match round % 4 {
            1 => decl.limit = 1 + g.below(12),
            2 => decl.beam = Some(1 + g.below(3)),
            3 if letters > 1 => {
                decl.reduction = "local_commuting";
                decl.independent = vec![(0, 1)];
            }
            _ => {}
        }
        let first = search(&loaded, &decl);
        let second = search(&loaded, &decl);
        assert!(equal(&first, &second), "one key, one record: {decl:?}");
        assert!(equal(field(&first, "declaration"), &decl.value()), "the record keeps the declaration");
        let mut bytes = Vec::new();
        encode(field(&first, "declaration"), &mut bytes).expect("canonical");
        assert_eq!(sha256_hex(&bytes), decl.key());

        let words = decl.family_words();
        let (result, stop, _) = outcome(&first);
        if stop == "completed" && decl.beam.is_none() {
            assert_eq!(as_u64(field(&first, "examined")), words.len() as u64, "{decl:?}");
            let witnesses = decl.witnesses(&words);
            assert_eq!(as_u64(field(&first, "witnesses")), witnesses.len() as u64);
            let least = witnesses.iter().map(|w| w.len() as i64).min().unwrap_or(-1);
            assert_eq!(as_i64(field(&first, "least_witness_length")), least);
            assert_eq!(result, if witnesses.is_empty() { "empty" } else { "found" });
            assert_eq!(as_u64(field(&first, "complete_lengths")), decl.max_length + 1);
        }
        if stop == "budget" {
            assert_eq!(as_u64(field(&first, "examined")), decl.limit);
            let mut other = decl.clone();
            other.limit += 1;
            assert_ne!(other.key(), decl.key(), "another budget is another key");
            assert_eq!(as_u64(field(&search(&loaded, &other), "examined")), (decl.limit + 1).min(words.len() as u64));
        }
        if let (Some(width), "completed") = (decl.beam, stop.as_str()) {
            // The lengths before the first level the beam truncated are the only ones covered completely.
            let (mut size, mut complete) = (1u64, decl.max_length + 1);
            for level in 1..=decl.max_length {
                size *= letters as u64;
                if size > width {
                    complete = level;
                    break;
                }
            }
            assert_eq!(as_u64(field(&first, "complete_lengths")), complete, "{decl:?}");
            assert_eq!(as_u64(field(&first, "discarded")) > 0, complete <= decl.max_length, "{decl:?}");
            truncated += usize::from(complete <= decl.max_length);
        }
    }
    assert!(truncated > 0, "a beamed fixture discards candidates");
}

/// An interruption that arrives after a number of the run's polls, as a host's clock or the solver stops a run.
struct StopAfter(Cell<u32>);

impl Cancel for StopAfter {
    fn pending(&self) -> Option<Interruption> {
        let left = self.0.get();
        self.0.set(left.saturating_sub(1));
        (left == 0).then_some(Interruption::ByUser)
    }
}

/// A budget, clock or error stop yields no negative (DET.30). Over a family with no witness, the completed search is
/// `empty` and the one negative. With the search's own budget below the family's size it is
/// `not_found_within_budget`: open, with the examined count and the lengths covered completely, and no negative; a
/// witness found before the budget ran out still counts (`found`, stop `budget`). A run budget that stops the run and
/// a clock that interrupts it leave no record at all, so no negative. An error in the search, an image outside the
/// action's points, answers outcome `error` with no coverage and no negative. A beam that discarded candidates gives
/// `not_found_in_beam`, and an applicable seed or another cost unit, which the enumeration cannot use, gives `not_run`.
#[test]
fn ct_lb5_07_budget_clock_and_error_stops_yield_no_negative() {
    let loaded = search_module();
    // Point 0 moves among {0, 1, 4, 5} only, so point 3 is never reached.
    let points = 6;
    let action = vec![swap(points, 0, 1), swap(points, 2, 3), vec![4, 1, 2, 3, 5, 0]];
    let decl = Decl::new(action.clone(), 0, vec![3], 3);
    let family = decl.family_words().len() as u64;
    assert_eq!(family, 1 + 3 + 9 + 27);
    let complete = search(&loaded, &decl);
    assert_eq!(outcome(&complete), ("empty".into(), "completed".into(), true));
    assert_eq!(as_text(field(&complete, "negative_family")), "declared");
    assert!(as_bool(field(&complete, "exhaustive")));

    for limit in [0, 1, 4, 5, 13, family - 1] {
        let mut cut = decl.clone();
        cut.limit = limit;
        let record = search(&loaded, &cut);
        assert_eq!(outcome(&record), ("not_found_within_budget".into(), "budget".into(), false), "limit {limit}");
        assert_eq!(as_u64(field(&record, "examined")), limit);
        let levels = [1, 4, 13, 40].iter().filter(|&&covered| covered <= limit).count() as u64;
        assert_eq!(as_u64(field(&record, "complete_lengths")), levels, "limit {limit}");
        assert!(!as_bool(field(&record, "exhaustive")));
        assert_eq!(as_text(field(&record, "negative_family")), "none");
    }
    let mut reachable = Decl::new(action.clone(), 0, vec![5], 3);
    reachable.limit = 20;
    let record = search(&loaded, &reachable);
    assert_eq!(outcome(&record), ("found".into(), "budget".into(), false));
    assert_eq!(as_i64(field(&record, "least_witness_length")), 2, "letter 2 takes point 0 to 4 and 4 to 5");
    assert_eq!(as_u64(field(&record, "complete_lengths")), 3);

    let short = Amounts { steps: 200, ..WIDE };
    let run = run_with(&loaded, SEARCH_MODULE, "search", vec![decl.value()], &Options::new(short));
    assert_eq!(run.summary.outcome, Outcome::CutOff, "{:?}", run.summary.stop);
    assert!(run.value.is_none(), "a run stopped on its budget leaves no record and so no negative");

    let clock = StopAfter(Cell::new(150));
    let mut options = Options::new(WIDE);
    options.cancel = Some(&clock);
    let run = run_with(&loaded, SEARCH_MODULE, "search", vec![decl.value()], &options);
    assert_eq!(run.summary.outcome, Outcome::Cancelled, "{:?}", run.summary.stop);
    assert!(run.value.is_none(), "an interrupted run leaves no record and so no negative");

    let mut broken = decl.clone();
    broken.action[2] = vec![4, 1, 2, 3, 9, 0];
    let record = search(&loaded, &broken);
    assert_eq!(outcome(&record), ("error".into(), "error".into(), false));
    assert_eq!(as_u64(field(&record, "examined")), 0);
    assert_eq!(as_text(field(&record, "negative_family")), "none");

    let mut beamed = decl.clone();
    beamed.beam = Some(2);
    let record = search(&loaded, &beamed);
    assert_eq!(outcome(&record), ("not_found_in_beam".into(), "completed".into(), false));
    assert_eq!(as_u64(field(&record, "discarded")), 1 + 4 + 4, "of 3 words at length 1 and of 6 at 2 and 3, 2 kept");
    assert_eq!(as_u64(field(&record, "complete_lengths")), 1, "only length 0 escaped the beam");
    assert!(!as_bool(field(&record, "exhaustive")));

    let mut seeded = decl.clone();
    seeded.seed = Some(7);
    let record = search(&loaded, &seeded);
    assert_eq!(outcome(&record), ("not_run".into(), "unsupported".into(), false));
    assert_eq!(as_u64(field(&record, "examined")), 0);
    let mut timed = decl.clone();
    timed.unit = "milliseconds";
    assert_eq!(outcome(&search(&loaded, &timed)), ("not_run".into(), "unsupported".into(), false));
}

fn lemma(family: &str, alphabet: u64, hypotheses: &[&str], reduction: &str, conclusion: &str) -> Value {
    record(vec![
        ("alphabet", nat(alphabet)),
        ("conclusion", text(conclusion)),
        ("family", text(family)),
        ("hypotheses", texts(hypotheses)),
        ("reduction", text(reduction)),
    ])
}

fn transport(loaded: &Loaded, result: &Value, lemma: Value) -> (bool, String, Vec<String>) {
    let answer = call(loaded, SEARCH_MODULE, "transport_negative", vec![result.clone(), lemma]);
    (
        as_bool(field(&answer, "transported")),
        as_text(field(&answer, "negative_family")),
        strings(field(&answer, "errors")),
    )
}

const HYPOTHESIS: &str = "declared_independent_letters_commute";
const CONCLUSION: &str = "no_witness_in_unreduced_family";

/// A reduced negative cannot leave its family without a matching reduction lemma (LB5.3). Under the local commuting
/// reduction a completed search with no witness is a negative of the reduced family only. A lemma with the same
/// family, alphabet, hypotheses, reduction and conclusion carries it to the declared family; a lemma that differs in
/// any one of them is refused with `LB5:evidence.reduction_scope_mismatch` and the negative stays reduced. Records
/// that are no negative transport nothing and raise nothing. Brute-force agreement is a scoped test, not the lemma:
/// where the declared independence is false, the reduced family has no witness while the unreduced family has one, so
/// the gate, not the brute force, keeps the negative in its family.
#[test]
fn ct_lb5_07_a_reduced_negative_needs_a_matching_lemma() {
    let loaded = search_module();
    let points = 6;
    // Letters 0 and 1 act on disjoint points, so they commute; point 0 never reaches point 3.
    let mut decl = Decl::new(vec![swap(points, 0, 1), swap(points, 2, 3), vec![4, 1, 2, 3, 5, 0]], 0, vec![3], 3);
    decl.reduction = "local_commuting";
    decl.independent = vec![(0, 1)];
    decl.hypotheses = vec![HYPOTHESIS];
    let reduced = search(&loaded, &decl);
    assert_eq!(outcome(&reduced), ("empty".into(), "completed".into(), true));
    assert_eq!(as_text(field(&reduced, "negative_family")), "reduced");
    let examined = as_u64(field(&reduced, "examined"));
    assert_eq!(examined, decl.family_words().len() as u64);
    let mut unreduced = decl.clone();
    unreduced.reduction = "none";
    let all = unreduced.family_words();
    assert!(examined < all.len() as u64, "the reduction skipped candidates");
    assert!(unreduced.witnesses(&all).is_empty(), "the scoped brute force agrees here");

    let matching = lemma("synthetic", 3, &[HYPOTHESIS], "local_commuting", CONCLUSION);
    assert_eq!(transport(&loaded, &reduced, matching), (true, "declared".into(), vec![]));
    let mismatch = (false, "reduced".to_string(), vec![REDUCTION_SCOPE_MISMATCH.to_string()]);
    for other in [
        lemma("another", 3, &[HYPOTHESIS], "local_commuting", CONCLUSION),
        lemma("synthetic", 4, &[HYPOTHESIS], "local_commuting", CONCLUSION),
        lemma("synthetic", 3, &[], "local_commuting", CONCLUSION),
        lemma("synthetic", 3, &[HYPOTHESIS, "letter_2_fixes_point_3"], "local_commuting", CONCLUSION),
        lemma("synthetic", 3, &[HYPOTHESIS], "symmetry", CONCLUSION),
        lemma("synthetic", 3, &[HYPOTHESIS], "local_commuting", "no_witness_up_to_length_2"),
    ] {
        assert_eq!(transport(&loaded, &reduced, other), mismatch);
    }

    let matching = || lemma("synthetic", 3, &[HYPOTHESIS], "local_commuting", CONCLUSION);
    let declared = search(&loaded, &unreduced);
    assert_eq!(transport(&loaded, &declared, matching()), (false, "declared".into(), vec![]));
    let mut stopped = decl.clone();
    stopped.limit = 3;
    assert_eq!(transport(&loaded, &search(&loaded, &stopped), matching()), (false, "none".into(), vec![]));
    let mut found = decl.clone();
    found.goals = vec![5];
    assert_eq!(transport(&loaded, &search(&loaded, &found), matching()), (false, "none".into(), vec![]));

    // Letters 0 and 1 do not commute: from point 2 only the word [1, 0] reaches point 1, and the reduction skips it.
    let mut false_independence = Decl::new(vec![swap(3, 0, 1), swap(3, 0, 2)], 2, vec![1], 2);
    false_independence.reduction = "local_commuting";
    false_independence.independent = vec![(0, 1)];
    false_independence.hypotheses = vec![HYPOTHESIS];
    let reduced = search(&loaded, &false_independence);
    assert_eq!(as_text(field(&reduced, "negative_family")), "reduced");
    let mut all = false_independence.clone();
    all.reduction = "none";
    assert_eq!(all.witnesses(&all.family_words()), vec![vec![1, 0]], "the unreduced family has a witness");
    let unmatched = lemma("synthetic", 2, &[], "local_commuting", CONCLUSION);
    assert_eq!(transport(&loaded, &reduced, unmatched), mismatch);
}
