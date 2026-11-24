//! CT-LB5-08 (chapter 03's test table; LB5.4, ERR.21, ERR.22): a complete D[k] rejection yields
//! `OUTSIDE_CERTIFIED_DOMAIN(k)` only; changing k or a reachability promise changes the applicable scope; neither
//! rejection nor a budget stop means unreachable.
//!
//! The entry is `research_search.membership`, run on `c600-run` with no host over small synthetic permutation actions
//! that the test also enumerates in Rust. D[k] is "within k letters of a target point" (the library's draft, LB5.4).
//! These runs test the answer's contract only and enter no research evidence (LP5 plan, section 9, reading B). A pass
//! is source and fixture evidence.
//!
//! CT-LB5-08 (source, part: the library's membership over synthetic actions; not run: the jumbling form of D[k]
//! over KN2.4's branch predicates, which needs research host reads that are not built).

mod research_common;

use c600_library::research::{SEARCH_MODULE, SEARCH_SOURCE};
use c600_run::interp::Loaded;
use c600_run::value::{equal, Value};
use research_common::*;

const OUTSIDE: &str = "OUTSIDE_CERTIFIED_DOMAIN";

/// A declared domain D[k] over a synthetic action, as the test builds it.
#[derive(Clone, Debug)]
struct Domain {
    action: Vec<Vec<u64>>,
    budget: u64,
    k: u64,
    name: &'static str,
    promise: &'static str,
    targets: Vec<u64>,
}

impl Domain {
    fn value(&self) -> Value {
        record(vec![
            ("action", list(self.action.iter().map(|images| nats(images)).collect())),
            ("budget", nat(self.budget)),
            ("k", nat(self.k)),
            ("name", text(self.name)),
            ("promise", text(self.promise)),
            ("targets", nats(&self.targets)),
        ])
    }

    /// Every word of at most k letters, by length, then letter order: the enumeration the entry declares.
    fn words(&self) -> Vec<Vec<u64>> {
        let mut level = vec![Vec::new()];
        let mut all = level.clone();
        for _ in 0..self.k {
            level = level
                .iter()
                .flat_map(|w| (0..self.action.len() as u64).map(move |a| [w.clone(), vec![a]].concat()))
                .collect();
            all.extend(level.iter().cloned());
        }
        all
    }

    fn hits(&self, input: u64, word: &[u64]) -> bool {
        let end = word.iter().fold(input, |p, &a| self.action[a as usize][p as usize]);
        self.targets.contains(&end)
    }

    /// The least number of letters that takes `input` to a target, by breadth-first search with no bound.
    fn distance(&self, input: u64) -> Option<u64> {
        let points = self.action.first().map_or(0, Vec::len);
        let mut seen = vec![false; points];
        let mut frontier = vec![input];
        seen[input as usize] = true;
        let mut depth = 0;
        while !frontier.is_empty() {
            if frontier.iter().any(|p| self.targets.contains(p)) {
                return Some(depth);
            }
            let mut next = Vec::new();
            for &p in &frontier {
                for letter in &self.action {
                    let q = letter[p as usize];
                    if !seen[q as usize] {
                        seen[q as usize] = true;
                        next.push(q);
                    }
                }
            }
            frontier = next;
            depth += 1;
        }
        None
    }
}

fn membership(loaded: &Loaded, domain: &Domain, input: u64) -> Value {
    call(loaded, SEARCH_MODULE, "membership", vec![domain.value(), nat(input)])
}

fn answer(record: &Value) -> (String, bool, String) {
    (as_text(field(record, "membership")), as_bool(field(record, "complete")), as_text(field(record, "stop")))
}

fn scope(record: &Value) -> (u64, String, String) {
    let s = field(record, "scope");
    (as_u64(field(s, "k")), as_text(field(s, "name")), as_text(field(s, "promise")))
}

/// A complete D[k] rejection yields `OUTSIDE_CERTIFIED_DOMAIN(k)` only. Over seeded actions, every input and k from 0
/// to 3: with a budget that covers every word of at most k letters, the answer is `inside` with the least depth when
/// the test's breadth-first distance is at most k, and otherwise `OUTSIDE_CERTIFIED_DOMAIN` with scope k, complete,
/// after examining every word. With a budget below the family's size, the answer is `inside` exactly when a word of
/// the examined prefix hits a target, and otherwise `unknown`: a cut-off enumeration never answers outside.
#[test]
fn ct_lb5_08_only_a_complete_rejection_answers_outside() {
    let loaded = loaded(&[SEARCH_SOURCE]);
    let mut g = Fixtures::new(0x5175_0108);
    let mut seen = (0, 0, 0);
    for _ in 0..10 {
        let (letters, points) = (1 + g.below(3) as usize, 3 + g.below(5) as usize);
        let action: Vec<Vec<u64>> = (0..letters).map(|_| g.permutation(points)).collect();
        let targets = vec![g.below(points as u64)];
        for k in 0..4 {
            let mut domain = Domain {
                action: action.clone(),
                budget: 1 << 20,
                k,
                name: "synthetic",
                promise: "none",
                targets: targets.clone(),
            };
            let words = domain.words();
            let total = words.len() as u64;
            for input in 0..points as u64 {
                let record = membership(&loaded, &domain, input);
                assert_eq!(scope(&record), (k, "synthetic".into(), "none".into()));
                assert_eq!(as_text(field(&record, "reachability")), "not_asserted");
                assert_eq!(as_u64(field(&record, "examined")), total);
                match domain.distance(input).filter(|&d| d <= k) {
                    Some(depth) => {
                        assert_eq!(answer(&record), ("inside".into(), true, "completed".into()));
                        assert_eq!(as_i64(field(&record, "depth")), depth as i64);
                        seen.0 += 1;
                    }
                    None => {
                        assert_eq!(answer(&record), (OUTSIDE.into(), true, "completed".into()), "{domain:?} {input}");
                        assert_eq!(as_i64(field(&record, "depth")), -1);
                        seen.1 += 1;
                    }
                }
                if total < 2 {
                    continue;
                }
                domain.budget = g.below(total);
                let cut = membership(&loaded, &domain, input);
                let hit = words[..domain.budget as usize].iter().any(|w| domain.hits(input, w));
                let expected = if hit { "inside" } else { "unknown" };
                assert_eq!(answer(&cut), (expected.into(), false, "budget".into()), "{domain:?} {input}");
                assert_eq!(as_u64(field(&cut, "examined")), domain.budget);
                seen.2 += usize::from(!hit);
                domain.budget = 1 << 20;
            }
        }
    }
    assert!(seen.0 > 0 && seen.1 > 0 && seen.2 > 0, "the fixtures reach every answer: {seen:?}");
}

/// Changing k or the reachability promise changes the applicable scope, and neither a rejection nor a budget stop
/// means unreachable. Point 0 reaches the target 3 in exactly three letters: D[2] rejects it completely with scope
/// k = 2, D[3] admits it at depth 3, and a budget stop on D[3] answers `unknown`. The same rejection under another
/// promise is another answer, because its scope differs; every answer says `reachability: not_asserted`, and the
/// target set's own points are inside at depth 0 for every k.
#[test]
fn ct_lb5_08_scope_follows_k_and_promise_and_no_answer_asserts_reachability() {
    let loaded = loaded(&[SEARCH_SOURCE]);
    // One letter, the cycle 0 -> 1 -> 2 -> 3 -> 0.
    let cycle =
        Domain { action: vec![vec![1, 2, 3, 0]], budget: 100, k: 2, name: "cycle", promise: "none", targets: vec![3] };
    let rejected = membership(&loaded, &cycle, 0);
    assert_eq!(answer(&rejected), (OUTSIDE.into(), true, "completed".into()));
    assert_eq!(scope(&rejected), (2, "cycle".into(), "none".into()));
    assert_eq!(cycle.distance(0), Some(3), "the rejected point is reachable in more letters");

    let wider = Domain { k: 3, ..cycle.clone() };
    let admitted = membership(&loaded, &wider, 0);
    assert_eq!(answer(&admitted), ("inside".into(), true, "completed".into()));
    assert_eq!(as_i64(field(&admitted, "depth")), 3);
    assert_eq!(scope(&admitted).0, 3);

    let stopped = membership(&loaded, &Domain { budget: 3, ..wider.clone() }, 0);
    assert_eq!(answer(&stopped), ("unknown".into(), false, "budget".into()));
    assert_eq!(as_i64(field(&stopped, "depth")), -1);

    let promised = Domain { promise: "orbit_of_point_0", ..cycle.clone() };
    let other = membership(&loaded, &promised, 0);
    assert_eq!(answer(&other), answer(&rejected));
    assert_eq!(scope(&other), (2, "cycle".into(), "orbit_of_point_0".into()));
    assert!(!equal(&other, &rejected), "another promise is another scope");
    let renamed = membership(&loaded, &Domain { name: "cycle_copy", ..cycle.clone() }, 0);
    assert!(!equal(&renamed, &rejected), "another domain name is another scope");

    for record in [&rejected, &admitted, &stopped, &other, &renamed] {
        assert_eq!(as_text(field(record, "reachability")), "not_asserted");
    }
    for k in 0..4 {
        let record = membership(&loaded, &Domain { k, ..cycle.clone() }, 3);
        assert_eq!(answer(&record), ("inside".into(), true, "completed".into()));
        assert_eq!(as_i64(field(&record, "depth")), 0);
    }
}
