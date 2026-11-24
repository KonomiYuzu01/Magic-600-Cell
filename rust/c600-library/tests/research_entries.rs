//! The research library's non-searching LB5 entries against the test's own arithmetic: stage 19's rank R_L and its
//! bounds (LB5.5), regional-family recognition (LB5.6) and conditional models (LB5.6). These are supporting tests of
//! piece R7, not rows of a conformance table: CT-LB5-06's stage 19 archive checks are not run here. The fixtures are
//! synthetic and enter no research evidence (LP5 plan, section 9, reading B). A pass is source and fixture evidence.

mod research_common;

use c600_library::research::{
    FAMILIES_MODULE, FAMILIES_SOURCE, FLOWS_SOURCE, MODELS_MODULE, MODELS_SOURCE, RANKS_MODULE, RANKS_SOURCE,
};
use c600_run::interp::{Loaded, Options};
use c600_run::meter::Outcome;
use c600_run::value::Value;
use research_common::*;

/// A rational value as (numerator, denominator) in decimal, an integer with denominator 1.
fn rational(value: &Value) -> (String, String) {
    match value {
        Value::Int(n) => (n.to_string(), "1".into()),
        Value::Rat(r) => (r.num.to_string(), r.den.to_string()),
        other => panic!("not a rational: {other:?}"),
    }
}

fn rank_rust(q: &[u64], sizes: &[u64], l: u64) -> u128 {
    q.iter().zip(sizes).fold(0u128, |v, (&q, &n)| v * (1 + l as u128 * n as u128) + q as u128)
}

fn rank(loaded: &Loaded, q: &[u64], sizes: &[u64], l: u64) -> u128 {
    as_u64(&call(loaded, RANKS_MODULE, "rank", vec![nats(q), nats(sizes), nat(l)])) as u128
}

/// Stage 19's rank R_L = sum over t of q_t times the product over s > t of (1 + L N_s), on seeded typed vectors with
/// 0 <= q_t <= L N_t, equals the test's own sum, and its order is the typed lexicographic order (the README's
/// equivalence, on these bounded vectors only).
#[test]
fn lb5_5_rank_is_r_l_and_orders_like_typed_lexicographic_descent() {
    let loaded = loaded(&[RANKS_SOURCE]);
    let mut g = Fixtures::new(0x5175_0505);
    for _ in 0..30 {
        let types = 1 + g.below(4) as usize;
        let sizes: Vec<u64> = (0..types).map(|_| 1 + g.below(5)).collect();
        let l = 1 + g.below(4);
        let mut vector = || sizes.iter().map(|&n| g.below(l * n + 1)).collect::<Vec<u64>>();
        let (a, b) = (vector(), vector());
        let (ra, rb) = (rank(&loaded, &a, &sizes, l), rank(&loaded, &b, &sizes, l));
        assert_eq!(ra, rank_rust(&a, &sizes, l));
        assert_eq!(rb, rank_rust(&b, &sizes, l));
        let less = as_bool(&call(&loaded, RANKS_MODULE, "lex_less", vec![nats(&a), nats(&b)]));
        assert_eq!(less, a < b);
        assert_eq!(less, ra < rb, "{a:?} {b:?} {sizes:?} L = {l}");
    }
}

/// The bounds keep their premises and say what they are not: the fixed-length bound is L R_L(Z0) with peak
/// 2 R_L(Z0) + 2 + 2L, the conditional cost is max(H-2,0)/2 * (product of (1 + L N_t) - 1) as an exact rational, and
/// both name the four premises, are neither discharged, expanded nor measured, and restore no stickers. Decrease on
/// stored blocks holds only when every block decreases, and is never global.
#[test]
fn lb5_5_bounds_keep_their_premises_and_block_decrease_stays_local() {
    let loaded = loaded(&[RANKS_SOURCE]);
    let premises = ["availability", "invariant_domain", "selector", "strict_decrease"];
    let (q, sizes, l) = ([2, 0, 3], [3, 1, 2], 2);
    let r = rank_rust(&q, &sizes, l) as u64;
    let fixed = call(&loaded, RANKS_MODULE, "fixed_length_bound", vec![nats(&q), nats(&sizes), nat(l)]);
    assert_eq!(as_u64(field(&fixed, "rank")), r);
    assert_eq!(as_u64(field(&fixed, "cost")), l * r);
    assert_eq!(as_u64(field(&fixed, "peak")), 2 * r + 2 + 2 * l);
    assert_eq!(strings(field(&fixed, "premises")), premises);
    assert_eq!(as_text(field(&fixed, "conclusion")), "high_phase_cost_and_peak");
    let product: u64 = sizes.iter().map(|&n| 1 + l * n).product();
    for height in [0u64, 1, 2, 3, 4, 7] {
        let cost = call(&loaded, RANKS_MODULE, "conditional_cost", vec![nat(height), nats(&sizes), nat(l)]);
        let twice = height.saturating_sub(2) * (product - 1);
        let expected = if twice % 2 == 0 { (twice / 2, 1) } else { (twice, 2) };
        assert_eq!(rational(field(&cost, "cost")), (expected.0.to_string(), expected.1.to_string()), "H = {height}");
        assert_eq!(strings(field(&cost, "premises")), premises);
        assert_eq!(as_text(field(&cost, "conclusion")), "high_phase_cost");
        for flag in ["discharged", "expanded", "measured", "restores_stickers"] {
            assert!(!as_bool(field(&cost, flag)));
            assert!(!as_bool(field(&fixed, flag)));
        }
    }

    let block = |after: &[u64], before: &[u64]| record(vec![("after", nats(after)), ("before", nats(before))]);
    let falling = list(vec![block(&[1, 1, 3], &[2, 0, 3]), block(&[0, 0, 0], &[0, 0, 1])]);
    let all = call(&loaded, RANKS_MODULE, "block_decrease", vec![falling, nats(&sizes), nat(l)]);
    assert_eq!((as_u64(field(&all, "blocks")), as_u64(field(&all, "decreasing"))), (2, 2));
    assert!(as_bool(field(&all, "holds")));
    assert!(!as_bool(field(&all, "global")));
    assert_eq!(as_text(field(&all, "scope")), "stored_blocks");
    let mixed = list(vec![block(&[1, 1, 3], &[2, 0, 3]), block(&[2, 0, 3], &[1, 1, 3])]);
    let one = call(&loaded, RANKS_MODULE, "block_decrease", vec![mixed, nats(&sizes), nat(l)]);
    assert_eq!(as_u64(field(&one, "decreasing")), 1);
    assert!(!as_bool(field(&one, "holds")));
}

fn piece(kind: u64, pose: u64) -> Value {
    record(vec![("kind", nat(kind)), ("pose", nat(pose))])
}

fn pieces(list_of: &[(u64, u64)]) -> Value {
    list(list_of.iter().map(|&(k, p)| piece(k, p)).collect())
}

fn gauge(pairs: &[(u64, u64)]) -> Value {
    list(pairs.iter().map(|&(from, to)| record(vec![("from", nat(from)), ("to", nat(to))])).collect())
}

/// Recognition is a type-preserving bijection onto the full reference table: each instance piece goes to a distinct
/// reference piece of its kind whose pose is equal or paired by the supplied gauge. A recognized instance transfers
/// the legal word language, legality along prefixes and shape-level claims, and never distance, reachability or the
/// sticker goal; distinct fibres are not asserted. Without the gauge, or with a piece of the wrong kind or count, the
/// instance is not recognized and transfers nothing.
#[test]
fn lb5_6_recognition_is_a_type_preserving_bijection_and_transfers_no_sticker_goal() {
    let loaded = loaded(&[FLOWS_SOURCE, FAMILIES_SOURCE]);
    let reference = [(0, 0), (1, 1), (0, 2), (1, 3)];
    let instance = [(1, 5), (0, 2), (1, 1), (0, 4)];
    let pairs = [(4, 0), (5, 3)];
    let recognize = |instance: &[(u64, u64)], pairs: &[(u64, u64)]| {
        call(&loaded, FAMILIES_MODULE, "recognize", vec![pieces(&reference), pieces(instance), gauge(pairs)])
    };
    let answer = recognize(&instance, &pairs);
    assert!(as_bool(field(&answer, "recognized")));
    let bijection = i64s(field(&answer, "bijection"));
    let mut used = vec![false; reference.len()];
    for (i, &target) in bijection.iter().enumerate() {
        let (kind, pose) = instance[i];
        let (rkind, rpose) = reference[target as usize];
        assert!(!used[target as usize], "a bijection");
        used[target as usize] = true;
        assert_eq!(kind, rkind, "type-preserving");
        assert!(pose == rpose || pairs.contains(&(pose, rpose)), "the pose is equal or gauged");
    }
    assert_eq!(
        strings(field(&answer, "transferred")),
        ["legal_word_language", "legality_along_prefixes", "shape_level_claims"]
    );
    assert_eq!(strings(field(&answer, "not_transferred")), ["distance", "reachability", "sticker_goal"]);
    assert_eq!(as_text(field(&answer, "distinct_fibres")), "not_asserted");
    assert_eq!(as_u64(field(&answer, "reference_pieces")), 4);

    for (instance, pairs) in
        [(&instance[..], &[][..]), (&[(1, 5), (0, 2), (0, 1), (0, 4)][..], &pairs[..]), (&instance[..3], &pairs[..])]
    {
        let answer = recognize(instance, pairs);
        assert!(!as_bool(field(&answer, "recognized")));
        assert!(items(field(&answer, "transferred")).is_empty());
        assert_eq!(strings(field(&answer, "not_transferred")), ["distance", "reachability", "sticker_goal"]);
    }
}

fn bound(loaded: &Loaded, name: &str, value: i64, premises: &[&str], scope: &str) -> Value {
    call(loaded, MODELS_MODULE, "conditional_bound", vec![text(name), int(value), texts(premises), text(scope)])
}

fn statement(premise: &str, scope: &str) -> Value {
    record(vec![("premise", text(premise)), ("scope", text(scope))])
}

/// Conditional models state their maps: a restriction to a region every letter keeps is total, with every letter
/// closed; a quotient by a block system the letters respect is compatible, with the block action; recognition of a
/// supplied instance is membership of the region. None asserts reachability. A conditional bound keeps its premises
/// in order without repeats; combining two bounds of one scope adds their values and keeps both premise sets, and
/// two scopes do not combine (the run fails at its check). A premise is discharged only by a statement of the same
/// premise and scope; the last discharge makes the bound unconditional.
#[test]
fn lb5_6_models_state_their_maps_and_premises_discharge_only_by_identical_statements() {
    let loaded = loaded(&[MODELS_SOURCE]);
    let action = || list(vec![nats(&[1, 0, 3, 2]), nats(&[2, 3, 0, 1])]);
    let closed = call(&loaded, MODELS_MODULE, "restriction", vec![action(), nats(&[0, 1, 2, 3])]);
    assert_eq!(u64s(field(&closed, "closed_letters")), [0, 1]);
    assert!(items(field(&closed, "partial")).iter().all(|row| i64s(row).iter().all(|&image| image >= 0)));
    assert_eq!(as_text(field(&closed, "reachability")), "not_asserted");
    let quotient = call(&loaded, MODELS_MODULE, "quotient", vec![action(), nats(&[0, 0, 1, 1]), nat(2)]);
    assert!(as_bool(field(&quotient, "compatible")));
    let rows: Vec<Vec<i64>> = items(field(&quotient, "quotient")).iter().map(i64s).collect();
    assert_eq!(rows, [vec![0, 1], vec![1, 0]]);
    assert_eq!(strings(field(&quotient, "preserved")), ["block_structure"]);
    assert_eq!(as_text(field(&quotient, "reachability")), "not_asserted");
    let member = call(&loaded, MODELS_MODULE, "recognize_instance", vec![nats(&[0, 2]), nat(2)]);
    assert!(as_bool(field(&member, "member")));
    assert_eq!(as_text(field(&member, "reachability")), "not_asserted");

    let a = bound(&loaded, "a", 5, &["selector", "availability", "selector"], "stage");
    assert_eq!(strings(field(&a, "premises")), ["availability", "selector"]);
    assert!(as_bool(field(&a, "conditional")));
    let b = bound(&loaded, "b", 7, &["strict_decrease", "availability"], "stage");
    let sum = call(&loaded, MODELS_MODULE, "combine", vec![text("a+b"), a.clone(), b.clone()]);
    assert_eq!(as_i64(field(&sum, "value")), 12);
    assert_eq!(strings(field(&sum, "premises")), ["availability", "selector", "strict_decrease"]);
    let elsewhere = bound(&loaded, "c", 1, &["selector"], "other_stage");
    let mixed = run_with(&loaded, MODELS_MODULE, "combine", vec![text("a+c"), a, elsewhere], &Options::new(WIDE));
    assert_eq!(mixed.summary.outcome, Outcome::Error, "two scopes do not combine");
    assert!(mixed.value.is_none());

    let discharge = |b: &Value, premise: &str, scope: &str| {
        call(&loaded, MODELS_MODULE, "discharge", vec![b.clone(), statement(premise, scope)])
    };
    let wrong_scope = discharge(&sum, "selector", "other_stage");
    assert!(!as_bool(field(&wrong_scope, "discharged")));
    assert_eq!(strings(field(field(&wrong_scope, "bound"), "premises")).len(), 3);
    let wrong_premise = discharge(&sum, "invariant_domain", "stage");
    assert!(!as_bool(field(&wrong_premise, "discharged")));
    let mut current = sum;
    for premise in ["selector", "availability", "strict_decrease"] {
        let step = discharge(&current, premise, "stage");
        assert!(as_bool(field(&step, "discharged")));
        current = field(&step, "bound").clone();
        assert!(!strings(field(&current, "premises")).iter().any(|p| p == premise));
    }
    assert!(!as_bool(field(&current, "conditional")));
    assert_eq!(as_i64(field(&current, "value")), 12);
}
