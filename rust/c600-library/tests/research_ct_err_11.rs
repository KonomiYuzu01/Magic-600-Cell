//! CT-ERR-11 (chapter 13's test table; ERR.21, ERR.22): completed outside-domain membership, unknown, partial and
//! off-lattice results, false step flags and empty completed lists stay values; a cut-off membership enumeration
//! cannot emit `OUTSIDE_CERTIFIED_DOMAIN(k)`.
//!
//! Every answer below is a completed run of a research library entry, on `c600-run` with no host, with no code: the
//! answer is in the value. The fixtures are synthetic and enter no research evidence (LP5 plan, section 9, reading
//! B). A pass is source and fixture evidence.
//!
//! CT-ERR-11 (source, part: the research library's entries; not run: the jumbling form of D[k] and the jumbling
//! puzzle's own off-lattice results as research answers, which need research host reads that are not built; the
//! research host's job records keep stops as values in `c600-research/tests/r8/stop.rs`).

mod research_common;

use std::cell::Cell;

use c600_library::research::{
    FAMILIES_MODULE, FAMILIES_SOURCE, FIXPOINTS_MODULE, FIXPOINTS_SOURCE, FLOWS_MODULE, FLOWS_SOURCE, MODELS_MODULE,
    MODELS_SOURCE, RANKS_MODULE, RANKS_SOURCE, SEARCH_MODULE, SEARCH_SOURCE, SLP_MODULE, SLP_SOURCE,
};
use c600_run::interp::{Cancel, Options};
use c600_run::meter::{Amounts, Interruption, Outcome};
use c600_run::value::Value;
use research_common::*;

fn domain(action: &[&[u64]], budget: u64, k: u64, targets: &[u64]) -> Value {
    record(vec![
        ("action", list(action.iter().map(|images| nats(images)).collect())),
        ("budget", nat(budget)),
        ("k", nat(k)),
        ("name", text("cycle")),
        ("promise", text("none")),
        ("targets", nats(targets)),
    ])
}

/// The cycle 0 -> 1 -> 2 -> 3 -> 0 under one letter.
const CYCLE: &[&[u64]] = &[&[1, 2, 3, 0]];

/// Completed outside-domain membership and `unknown` stay values. A complete rejection, a budget stop and an error in
/// the enumeration each complete with no code; the rejection answers `OUTSIDE_CERTIFIED_DOMAIN` with its scope, the
/// others `unknown`, and none asserts reachability. Malformed input is an error, never a rejection, even where the
/// enumeration would never read the bad point: an image, the input or a target outside the action's points, a row of
/// another length, or an action with no letter.
#[test]
fn ct_err_11_membership_answers_are_values() {
    let loaded = loaded(&[SEARCH_SOURCE]);
    let rejected = call(&loaded, SEARCH_MODULE, "membership", vec![domain(CYCLE, 100, 2, &[3]), nat(0)]);
    assert_eq!(as_text(field(&rejected, "membership")), "OUTSIDE_CERTIFIED_DOMAIN");
    assert_eq!(as_u64(field(field(&rejected, "scope"), "k")), 2);
    let stopped = call(&loaded, SEARCH_MODULE, "membership", vec![domain(CYCLE, 2, 2, &[3]), nat(0)]);
    assert_eq!(
        (as_text(field(&stopped, "membership")), as_text(field(&stopped, "stop"))),
        ("unknown".into(), "budget".into())
    );
    let mut answers = vec![rejected, stopped];
    let malformed: [(&[&[u64]], u64, &[u64]); 7] = [
        (&[&[1, 2, 7, 0]], 0, &[3]),
        (&[&[1, 2, 3, 0], &[0, 1, 2]], 0, &[3]),
        (CYCLE, 4, &[3]),
        (&[&[1, 2, 3, 0], &[0, 1, 2, 9]], 3, &[3]),
        (CYCLE, 0, &[99]),
        (CYCLE, 0, &[3, 4]),
        (&[], 0, &[0]),
    ];
    for (action, input, targets) in malformed {
        let broken = call(&loaded, SEARCH_MODULE, "membership", vec![domain(action, 100, 3, targets), nat(input)]);
        assert_eq!(
            (as_text(field(&broken, "membership")), as_text(field(&broken, "stop"))),
            ("unknown".into(), "error".into())
        );
        assert_eq!(as_u64(field(&broken, "examined")), 0);
        answers.push(broken);
    }
    for record in &answers {
        assert_eq!(as_text(field(record, "reachability")), "not_asserted");
    }
}

struct StopAfter(Cell<u32>);

impl Cancel for StopAfter {
    fn pending(&self) -> Option<Interruption> {
        let left = self.0.get();
        self.0.set(left.saturating_sub(1));
        (left == 0).then_some(Interruption::ByUser)
    }
}

/// A cut-off membership enumeration cannot emit `OUTSIDE_CERTIFIED_DOMAIN(k)`. For every budget below the family's
/// size, the enumeration's own stop answers `unknown`; a stop of the run's own budget at every step count up to the
/// completed run's and an interruption at every poll leave no value at all, so no rejection. Only the completed run
/// answers outside.
#[test]
fn ct_err_11_a_cut_off_membership_never_answers_outside() {
    let loaded = loaded(&[SEARCH_SOURCE]);
    let action: &[&[u64]] = &[&[1, 0, 2, 4, 3], &[0, 2, 1, 3, 4]];
    // Point 0 moves among {0, 1, 2}; targets {3, 4} are never reached. Words of at most 3 letters: 1 + 2 + 4 + 8.
    let family = 15;
    for budget in 0..family {
        let record = call(&loaded, SEARCH_MODULE, "membership", vec![domain(action, budget, 3, &[3, 4]), nat(0)]);
        assert_eq!(as_text(field(&record, "membership")), "unknown", "budget {budget}");
        assert!(!as_bool(field(&record, "complete")));
    }
    let args = || vec![domain(action, family, 3, &[3, 4]), nat(0)];
    let complete = run_with(&loaded, SEARCH_MODULE, "membership", args(), &Options::new(WIDE));
    assert_eq!(complete.summary.outcome, Outcome::Completed);
    assert_eq!(as_text(field(complete.value.as_ref().expect("a value"), "membership")), "OUTSIDE_CERTIFIED_DOMAIN");
    let steps = complete.summary.used.steps;

    let mut cut_offs = 0;
    for limit in (0..steps).step_by((steps / 40).max(1) as usize) {
        let run =
            run_with(&loaded, SEARCH_MODULE, "membership", args(), &Options::new(Amounts { steps: limit, ..WIDE }));
        match run.summary.outcome {
            Outcome::CutOff => {
                assert!(run.value.is_none(), "a cut-off run leaves no value");
                cut_offs += 1;
            }
            other => panic!("{other:?} at {limit} of {steps} steps"),
        }
    }
    assert!(cut_offs > 0);

    let never = StopAfter(Cell::new(u32::MAX));
    let mut options = Options::new(WIDE);
    options.cancel = Some(&never);
    assert_eq!(run_with(&loaded, SEARCH_MODULE, "membership", args(), &options).summary.outcome, Outcome::Completed);
    let polls = u32::MAX - never.0.get();
    assert!(polls > 0, "the run polls its clock");
    for after in (0..polls).step_by((polls / 40).max(1) as usize) {
        let clock = StopAfter(Cell::new(after));
        let mut options = Options::new(WIDE);
        options.cancel = Some(&clock);
        let run = run_with(&loaded, SEARCH_MODULE, "membership", args(), &options);
        assert_eq!(run.summary.outcome, Outcome::Cancelled, "after {after} of {polls} polls");
        assert!(run.value.is_none(), "an interrupted run leaves no value");
    }
}

/// Partial and off-lattice results, false step flags and empty completed lists stay values. A restriction to a region
/// that a letter leaves is partial (-1 where the image leaves) with fewer closed letters; a quotient by blocks that a
/// letter splits is incompatible, with its conflicts counted and nothing preserved; a rank decrease that fails on a
/// stored block, a set that is no fixed point, a matching or flow that breaks an edge or a demand, a lexicographic
/// comparison that is false, an expansion over its bound and a recognition with no bijection answer `false`; a search
/// with no witness, a matching of an empty graph and a bound with no premise answer empty lists. Every run completes
/// with no code.
#[test]
fn ct_err_11_partial_and_false_and_empty_answers_are_values() {
    let models = loaded(&[MODELS_SOURCE]);
    let action = || list(vec![nats(&[1, 0, 3, 2]), nats(&[0, 2, 1, 3])]);
    let restricted = call(&models, MODELS_MODULE, "restriction", vec![action(), nats(&[0, 1])]);
    assert_eq!(i64s(&items(field(&restricted, "partial"))[1]), vec![0, -1]);
    assert_eq!(u64s(field(&restricted, "closed_letters")), vec![0]);
    let split = call(&models, MODELS_MODULE, "quotient", vec![action(), nats(&[0, 0, 1, 1]), nat(2)]);
    assert!(!as_bool(field(&split, "compatible")));
    assert!(as_u64(field(&split, "conflicts")) > 0);
    assert!(strings(field(&split, "preserved")).is_empty());
    let bound = call(&models, MODELS_MODULE, "conditional_bound", vec![text("b"), int(5), texts(&[]), text("s")]);
    assert!(!as_bool(field(&bound, "conditional")));
    assert!(items(field(&bound, "premises")).is_empty());

    let ranks = loaded(&[RANKS_SOURCE]);
    let blocks = list(vec![record(vec![("after", nats(&[1, 0])), ("before", nats(&[0, 1]))])]);
    let decrease = call(&ranks, RANKS_MODULE, "block_decrease", vec![blocks, nats(&[2, 2]), nat(1)]);
    assert!(!as_bool(field(&decrease, "holds")));
    assert!(!as_bool(field(&decrease, "global")));
    assert!(!as_bool(&call(&ranks, RANKS_MODULE, "lex_less", vec![nats(&[1, 0]), nats(&[0, 5])])));

    let fixpoints = loaded(&[FIXPOINTS_SOURCE]);
    let leaf = || record(vec![("children", nats(&[])), ("kind", text("and"))]);
    let or_node = record(vec![("children", nats(&[0])), ("kind", text("or"))]);
    let graph = list(vec![leaf(), or_node]);
    assert!(!as_bool(&call(&fixpoints, FIXPOINTS_MODULE, "is_fixed_point", vec![graph, nats(&[1])])));

    let flows = loaded(&[FLOWS_SOURCE]);
    let edge = |l: u64, r: u64| record(vec![("left", nat(l)), ("right", nat(r))]);
    let empty = call(&flows, FLOWS_MODULE, "max_matching", vec![nat(2), nat(2), list(vec![])]);
    assert!(items(field(&empty, "matching")).is_empty());
    assert!(!as_bool(field(&empty, "perfect")));
    let broken = vec![nat(2), nat(2), list(vec![edge(0, 0)]), list(vec![edge(0, 1)])];
    assert!(!as_bool(&call(&flows, FLOWS_MODULE, "check_matching", broken)));
    let arc = record(vec![("capacity", nat(1)), ("from", nat(0)), ("to", nat(1))]);
    let short = vec![nat(2), list(vec![arc]), nats(&[1, 0]), nats(&[0, 1]), list(vec![int(0)])];
    assert!(!as_bool(&call(&flows, FLOWS_MODULE, "check_flow", short)));

    let slp = loaded(&[SLP_SOURCE]);
    let rule =
        record(vec![("kind", text("leaf")), ("left", nat(0)), ("letters", Value::word(&[1, 2, 3])), ("right", nat(0))]);
    let refused = call(&slp, SLP_MODULE, "admitted_expansion", vec![list(vec![rule]), nat(2)]);
    assert!(!as_bool(field(&refused, "admitted")));
    assert_eq!(as_u64(field(&refused, "length")), 3);

    let families = loaded(&[FLOWS_SOURCE, FAMILIES_SOURCE]);
    let piece = |kind: u64, pose: u64| record(vec![("kind", nat(kind)), ("pose", nat(pose))]);
    let args = vec![list(vec![piece(0, 0), piece(1, 0)]), list(vec![piece(0, 0), piece(0, 1)]), list(vec![])];
    let unrecognized = call(&families, FAMILIES_MODULE, "recognize", args);
    assert!(!as_bool(field(&unrecognized, "recognized")));
    assert!(items(field(&unrecognized, "transferred")).is_empty());

    let search = loaded(&[SEARCH_SOURCE]);
    let empty = call(&search, SEARCH_MODULE, "search", vec![declaration(&[&[1, 0, 2]], &[2], 0)]);
    assert_eq!(as_text(field(&empty, "outcome")), "empty");
    assert_eq!(as_u64(field(&empty, "witnesses")), 0);
}

/// A search declaration over `action` from `start` to `goals`, with words of at most 3 letters and no beam.
fn declaration(action: &[&[u64]], goals: &[u64], start: u64) -> Value {
    record(vec![
        ("action", list(action.iter().map(|images| nats(images)).collect())),
        ("beam", record(vec![("applicable", boolean(false)), ("width", nat(0))])),
        ("budget", record(vec![("limit", nat(100)), ("unit", text("candidates"))])),
        ("family", text("synthetic")),
        ("goals", nats(goals)),
        ("hypotheses", texts(&[])),
        ("independent", list(vec![])),
        ("input_sha256", text("")),
        ("max_length", nat(3)),
        ("module_sha256", text("")),
        ("reduction", text("none")),
        ("seed", record(vec![("applicable", boolean(false)), ("value", nat(0))])),
        ("start", nat(start)),
    ])
}

/// A search over malformed input answers outcome `error` with no coverage and no negative, never `empty`: a goal, the
/// start point or an image outside the action's points, or an action with no letter. The well-formed search beside
/// them is the one negative.
#[test]
fn ct_err_11_a_malformed_search_is_an_error_and_no_negative() {
    let loaded = loaded(&[SEARCH_SOURCE]);
    let negative = call(&loaded, SEARCH_MODULE, "search", vec![declaration(&[&[1, 0, 2]], &[2], 0)]);
    assert!(as_bool(field(&negative, "negative")));
    let malformed: [(&[&[u64]], &[u64], u64); 5] = [
        (&[&[1, 0, 2]], &[99], 0),
        (&[&[1, 0, 2]], &[2, 3], 0),
        (&[&[1, 0, 2]], &[2], 3),
        (&[&[1, 0, 5]], &[2], 0),
        (&[], &[0], 0),
    ];
    for (action, goals, start) in malformed {
        let record = call(&loaded, SEARCH_MODULE, "search", vec![declaration(action, goals, start)]);
        assert_eq!(
            (as_text(field(&record, "outcome")), as_text(field(&record, "stop"))),
            ("error".into(), "error".into()),
            "{action:?} {goals:?} {start}"
        );
        assert!(!as_bool(field(&record, "negative")));
        assert_eq!(as_text(field(&record, "negative_family")), "none");
        assert_eq!(as_u64(field(&record, "examined")), 0);
        assert_eq!(as_u64(field(&record, "complete_lengths")), 0);
    }
}

/// No research source spells the withdrawn outside-domain code (ERR.22): the answer is the value
/// `OUTSIDE_CERTIFIED_DOMAIN` in the search module only, and no source builds an `LB5:input` code.
#[test]
fn ct_err_11_no_research_source_spells_the_withdrawn_code() {
    let withdrawn = "OUTSIDE_CERTIFIED_DOMAIN".to_lowercase();
    for source in
        [SLP_SOURCE, FIXPOINTS_SOURCE, FLOWS_SOURCE, SEARCH_SOURCE, RANKS_SOURCE, FAMILIES_SOURCE, MODELS_SOURCE]
    {
        assert!(!source.contains(&withdrawn));
        assert!(!source.contains("LB5:input"));
    }
    assert!(SEARCH_SOURCE.contains("\"OUTSIDE_CERTIFIED_DOMAIN\""));
    for source in [SLP_SOURCE, FIXPOINTS_SOURCE, FLOWS_SOURCE, RANKS_SOURCE, FAMILIES_SOURCE, MODELS_SOURCE] {
        assert!(!source.contains("OUTSIDE_CERTIFIED_DOMAIN"));
    }
}
