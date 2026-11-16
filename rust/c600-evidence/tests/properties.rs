//! Chapter 06 section 4 and Theorem D (a): property tests on small random graphs. Each test uses the SplitMix64
//! generator of `common` with a fixed seed and states its number of cases. The expected values come from the test's
//! own transcriptions of EVD.19, EVD.22, EVD.23 (Theorem B), EVD.30 and EVD.38.

mod common;

use std::collections::BTreeSet;

use c600_evidence::*;
use common::*;

fn in_u0(g: &Grade) -> bool {
    g.basis != Basis::Sample && g.check != Check::None
}

/// Theorem B, transcribed: a dominates b exactly when Pi_a lies inside Pi_b and every grade of L_a lies above some
/// grade of L_b.
fn t_dominates(a: &Alternative, b: &Alternative) -> bool {
    a.premises.is_subset(&b.premises) && a.grades.iter().all(|x| b.grades.iter().any(|y| t_le(y, x)))
}

const CASES_02: usize = 400;

/// CT-EVD-02 (whole): EVD.23, EVD.24 and Theorem C on 400 random graphs (seed 0x0E5D0002; two to four keys, two to
/// four grades, premises, shared nodes and child cycles). Summaries computed in four node orders are equal; each
/// summary's L sets are antichains and its alternatives pairwise non-dominating; the union of their Sat sets equals
/// the test's own least solution of EVD.22 at every separating scenario; and a perturbed representation of the same
/// val (dominated alternatives and non-minimal grades added, order shuffled) reduces to the same set, so the canonical
/// form is unique.
#[test]
fn ct_evd_02_summaries_are_order_free_and_canonical() {
    let e = ed();
    let all = all_grades();
    let mut rng = Rng::new(0x0E5D_0002);
    let mut scenarios_checked = 0usize;
    for _ in 0..CASES_02 {
        let base = 2 + rng.below(3);
        let pool = 2 + rng.below(3);
        let spec = random_spec(&mut rng, base, false, pool);
        let (g, _) = spec.build();
        let mut order: Vec<NodeId> = (0..g.len()).collect();
        let first = summaries_in_order(&g, &e, &order);
        for _ in 0..3 {
            rng.shuffle(&mut order);
            assert_eq!(summaries_in_order(&g, &e, &order), first, "evaluation order changed a summary");
        }
        let scen = scenarios(&spec);
        for k in spec.keys() {
            let root = g.root_of(&k).expect("root");
            let canon = first[&root].clone();
            assert_eq!(summary(&g, &k, &e).expect("no inconsistency"), canon);
            for a in &canon {
                for x in &a.grades {
                    for y in &a.grades {
                        assert!(x == y || !t_le(x, y), "L is not an antichain: {a:?}");
                    }
                }
            }
            for (i, a) in canon.iter().enumerate() {
                for (j, b) in canon.iter().enumerate() {
                    assert!(i == j || !t_dominates(a, b), "{a:?} dominates {b:?}");
                }
            }
            for (d, t) in &scen {
                let dist = |gr: &Grade| d.contains(gr);
                let truth = brute_val(&g, &dist, t)[root];
                let sat = canon.iter().any(|a| in_sat(a, &dist, t));
                assert_eq!(truth, sat, "the summary of {k:?} disagrees with val at {d:?}, {t:?}");
                scenarios_checked += 1;
            }
            let mut perturbed = canon.clone();
            for a in &canon {
                let mut more = a.clone();
                more.premises.insert(spec.key(rng.below(spec.key_count())));
                more.premises.insert(key("an unrelated premise", Covers::OneInstance));
                perturbed.push(more);
                if let Some(x) = a.grades.iter().next() {
                    let above: Vec<&Grade> = all.iter().filter(|z| t_le(x, z)).collect();
                    let mut wider = a.clone();
                    wider.grades.insert(above[rng.below(above.len())].clone());
                    perturbed.push(wider);
                }
            }
            rng.shuffle(&mut perturbed);
            assert_eq!(reduce(&e, perturbed), canon, "the canonical form is not unique");
        }
    }
    assert!(scenarios_checked > 10_000, "{scenarios_checked}");
}

const CASES_03: usize = 400;

/// CT-EVD-03 (whole): EVD.24 on 400 random graphs (seed 0x0E5D0003). Raising one leaf's grade to a grade above it,
/// or adding one OR child (a new leaf or an existing node), never shrinks val at any separating scenario; and copying
/// every shared node, so that no node has two parents, changes no summary and no val.
#[test]
fn ct_evd_03_val_is_monotone_and_sharing_is_harmless() {
    let e = ed();
    let all = all_grades();
    let mut rng = Rng::new(0x0E5D_0003);
    for _ in 0..CASES_03 {
        let base = 2 + rng.below(3);
        let pool = 2 + rng.below(3);
        let spec = random_spec(&mut rng, base, false, pool);
        let (g, _) = spec.build();

        let leaves: Vec<usize> =
            (0..spec.nodes.len()).filter(|i| matches!(spec.nodes[*i], SpecNode::Leaf(_))).collect();
        let mut raised = spec.clone();
        if !leaves.is_empty() {
            let i = leaves[rng.below(leaves.len())];
            if let SpecNode::Leaf(x) = &spec.nodes[i] {
                let above: Vec<&Grade> = all.iter().filter(|z| t_le(x, z)).collect();
                raised.nodes[i] = SpecNode::Leaf(above[rng.below(above.len())].clone());
            }
        }
        let mut added = spec.clone();
        let k = rng.below(spec.key_count());
        if rng.chance(50) || spec.nodes.is_empty() {
            added.nodes.push(SpecNode::Leaf(all[rng.below(all.len())].clone()));
            let n = added.nodes.len() - 1;
            added.or_children[k].push(n);
        } else {
            let n = rng.below(spec.nodes.len());
            if !added.or_children[k].contains(&n) {
                added.or_children[k].push(n);
            }
        }
        let unshared = spec.unshared();

        for variant in [&raised, &added] {
            let (g2, _) = variant.build();
            let mut grades = spec.grades();
            grades.extend(variant.grades());
            let mut prem = spec.premise_keys();
            prem.extend(variant.premise_keys());
            for key in spec.keys() {
                let before = summary(&g, &key, &e).expect("no inconsistency");
                let after = summary(&g2, &key, &e).expect("no inconsistency");
                for (d, t) in scenarios_over(&grades, &prem) {
                    let dist = |gr: &Grade| d.contains(gr);
                    let b = before.iter().any(|a| in_sat(a, &dist, &t));
                    let a = after.iter().any(|a| in_sat(a, &dist, &t));
                    assert!(!b || a, "val shrank for {key:?}");
                }
            }
        }
        let (g3, _) = unshared.build();
        for key in spec.keys() {
            assert_eq!(summary(&g, &key, &e).unwrap(), summary(&g3, &key, &e).unwrap(), "sharing changed a summary");
        }
        for (idx, (d, t)) in scenarios(&spec).into_iter().enumerate() {
            let dist = |gr: &Grade| d.contains(gr);
            let v = brute_val(&g, &dist, &t);
            let v3 = brute_val(&g3, &dist, &t);
            for key in spec.keys() {
                assert_eq!(v[g.root_of(&key).unwrap()], v3[g3.root_of(&key).unwrap()]);
                if idx % 8 == 0 {
                    let lib = val_at(&g3, &key, &e, &d, &t).unwrap();
                    assert_eq!(lib, v3[g3.root_of(&key).unwrap()], "val_at disagrees with EVD.22");
                }
            }
        }
    }
}

const CASES_04: usize = 600;

/// CT-EVD-04 (whole): EVD.30 on 600 random graphs with negation pairs (seed 0x0E5D0004), under U0. stands0 is the
/// least solution at omega0 = (P minus U, empty) and reads neither stands1 nor Ref; Ref = {p : stands0(neg p)};
/// stands1 is the least solution at omega1 = (P minus U, Prem minus Ref); stands0 implies stands1; a pair standing on
/// both sides reads I4. Adding a proof of neg p leaves every stands0 of a base statement unchanged, and a proof of
/// neg p that puts p in Ref turns stands1 off for a statement resting on p.
#[test]
fn ct_evd_04_two_passes() {
    let e = ed();
    let mut rng = Rng::new(0x0E5D_0004);
    let dist0 = |g: &Grade| !in_u0(g);
    let mut i4_seen = 0usize;
    let mut ref_seen = 0usize;
    for _ in 0..CASES_04 {
        let base = 2 + rng.below(3);
        let pool = 2 + rng.below(3);
        let spec = random_spec(&mut rng, base, true, pool);
        let (g, _) = spec.build();
        let v0 = brute_val(&g, &dist0, &BTreeSet::new());
        let neg = |k: usize| if k < spec.base { k + spec.base } else { k - spec.base };
        let root = |k: usize| g.root_of(&spec.key(k)).unwrap();
        let mut stands0_before = Vec::new();
        for k in 0..spec.key_count() {
            let reach = brute_reach(&g, &spec, k);
            let i4 = (0..spec.key_count())
                .any(|j| reach.contains(&root(j)) && reach.contains(&root(neg(j))) && v0[root(j)] && v0[root(neg(j))]);
            let prem: BTreeSet<Key> = reach
                .iter()
                .filter_map(|id| match &g.nodes()[*id] {
                    Node::Premise(p) => Some(p.key.clone()),
                    _ => None,
                })
                .collect();
            let refuted: BTreeSet<Key> = prem
                .iter()
                .filter(|p| {
                    let j = spec.keys().iter().position(|x| x == *p).unwrap();
                    v0[root(neg(j))]
                })
                .cloned()
                .collect();
            let assumed: BTreeSet<Key> = prem.difference(&refuted).cloned().collect();
            let v1 = brute_val(&g, &dist0, &assumed);
            match standing(&g, &spec.key(k), &e) {
                Err(err) => {
                    assert!(i4, "unexpected {err}");
                    assert!(err.has(Inconsistency::I4));
                    i4_seen += 1;
                    stands0_before.push(None);
                }
                Ok(st) => {
                    assert!(!i4, "I4 missed for key {k}");
                    assert_eq!(st.stands0, v0[root(k)]);
                    assert_eq!(st.stands1, v1[root(k)]);
                    assert!(!st.stands0 || st.stands1, "stands0 without stands1");
                    assert_eq!(st.refuted, refuted);
                    ref_seen += refuted.len();
                    stands0_before.push(Some(st.stands0));
                }
            }
        }
        // A proof of neg p for a random base key p changes Ref and leaves every base stands0 alone.
        let p = rng.below(spec.base);
        let mut g2 = g.clone();
        let np = spec.key(neg(p));
        g2.support(&np, Leaf::new(&np, th()));
        for k in 0..spec.base {
            if let (Some(before), Ok(after)) = (stands0_before[k], standing(&g2, &spec.key(k), &e)) {
                assert_eq!(after.stands0, before, "stands0 read a proof of a negated premise");
            }
        }
    }
    assert!(i4_seen > 0 && ref_seen > 0, "the generator reached I4 {i4_seen} times and Ref {ref_seen} times");

    // A statement resting on p: conditional, until a proof of neg p puts p in Ref.
    let s = key("S", Covers::OneInstance);
    let p = key("p", Covers::OneInstance);
    let np = key("not p", Covers::OneInstance);
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    pair(&mut g, finite("p"), finite("not p"));
    let l = g.add_leaf(Leaf::new(&s, th()));
    let pr = g.add_premise(&p, PremiseClass::OpenHypothesis);
    let and = g.add_and(&[l, pr]).unwrap();
    g.add_or_child(&s, and).unwrap();
    let st = standing(&g, &s, &e).unwrap();
    assert!(!st.stands0 && st.stands1);
    assert_eq!(row(&g, &s, &e), 4);
    g.support(&np, Leaf::new(&np, th()));
    let st = standing(&g, &s, &e).unwrap();
    assert!(!st.stands0 && !st.stands1);
    assert_eq!(st.refuted, [p.clone()].into_iter().collect());
    assert_eq!(row(&g, &s, &e), 7);
}

const CASES_06: usize = 1000;

/// CT-EVD-06 (whole): Theorem D (a) on 1,000 random graphs with negation pairs (seed 0x0E5D0006). Records that add no
/// edge into Reach(S) — new statements with leaves of any of the 30 grades, premises and AND nodes over keys outside
/// Reach(S), new OR children of keys outside Reach(S), search records and jobs on those keys — change no word of S.
#[test]
fn ct_evd_06_records_outside_reach_change_no_word() {
    let e = ed();
    let all = all_grades();
    let mut rng = Rng::new(0x0E5D_0006);
    let mut changed_graphs = 0usize;
    for case in 0..CASES_06 {
        let base = 2 + rng.below(3);
        let pool = 2 + rng.below(3);
        let spec = random_spec(&mut rng, base, true, pool);
        let (g, _) = spec.build();
        let s = spec.key(0);
        let before = word(&g, &s, &e);
        let reach = brute_reach(&g, &spec, 0);
        let outside: Vec<Key> =
            spec.keys().into_iter().filter(|k| !reach.contains(&g.root_of(k).unwrap())).collect();
        let mut g2 = g.clone();
        let mut fresh: Vec<Key> = Vec::new();
        for r in 0..(1 + rng.below(5)) {
            let name = format!("record {case}.{r}");
            let k = key(&name, Covers::OneInstance);
            g2.add_statement(finite(&name)).unwrap();
            fresh.push(k);
        }
        let mut targets: Vec<Key> = outside.clone();
        targets.extend(fresh.iter().cloned());
        for _ in 0..(1 + rng.below(6)) {
            let t = targets[rng.below(targets.len())].clone();
            match rng.below(4) {
                0 => {
                    g2.support(&t, Leaf::new(&t, all[rng.below(all.len())].clone()));
                }
                1 => {
                    let p = targets[rng.below(targets.len())].clone();
                    let l = g2.add_leaf(Leaf::new(&t, all[rng.below(all.len())].clone()));
                    let pr = g2.add_premise(&p, PremiseClass::OpenHypothesis);
                    let and = g2.add_and(&[l, pr]).unwrap();
                    g2.add_or_child(&t, and).unwrap();
                }
                2 => g2.attach_search(
                    &t,
                    SearchRecord::new(
                        &t.statement,
                        &Covers::OneInstance,
                        Complete::No,
                        Stop::DeterministicBudget,
                        JobOutcome::Completed,
                    ),
                ),
                _ => g2.attach_job(&t, ProducingJob::new("unrelated", JobOutcome::Cancelled)),
            }
        }
        assert_eq!(brute_reach(&g2, &spec, 0), reach, "the records stayed outside Reach(S)");
        assert_eq!(word(&g2, &s, &e), before, "a record outside Reach(S) changed the word");
        changed_graphs += 1;
    }
    assert_eq!(changed_graphs, CASES_06);
}
