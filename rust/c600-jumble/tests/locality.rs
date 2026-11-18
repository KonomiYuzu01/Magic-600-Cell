//! LB1.10's `jumble-locality/1` table (`c600_jumble::locality`) on the model in `assets/`: R² from cap 0's region, 80
//! partners per grip, every pair of grip 0 against stage 14's exact shells (`research/jumbling/restoration/
//! 14-blocking-structure/results/locality-summary.json`), Δ recomputed from the exact poles, symmetry, antipodes and
//! the bit layout.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_jumble::locality::{Locality, JUMBLE_LOCALITY, PARTNERS, R2_FORM};
use c600_jumble::q5::{dot, Q5};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json;
use c600_run::menu::MenuIdentity;
use c600_run::trace::Relation;
use num_bigint::BigInt;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn locality() -> &'static Locality {
    static LOCALITY: OnceLock<Locality> = OnceLock::new();
    LOCALITY.get_or_init(|| Locality::build(ctx()).expect("the locality table"))
}

/// An exact number as stage 14 writes it: `a`, `a + b*sqrt(5)` or `a - b*sqrt(5)`, optionally as `(…)/d`.
fn exact(text: &str) -> Q5 {
    let (body, den) = match text.strip_prefix('(').and_then(|t| t.rsplit_once(")/")) {
        Some((body, den)) => (body, den.parse::<i64>().expect("a denominator")),
        None => (text, 1),
    };
    let (a, b) = match body
        .split_once(" + ")
        .map(|(a, b)| (a, b, 1))
        .or_else(|| body.split_once(" - ").map(|(a, b)| (a, b, -1)))
    {
        Some((a, b, sign)) => {
            let b = b.strip_suffix("*sqrt(5)").expect("a multiple of sqrt(5)");
            (a.parse::<i64>().expect("an integer"), sign * b.parse::<i64>().expect("an integer"))
        }
        None => (body.parse::<i64>().expect("an integer"), 0),
    };
    Q5::new(BigInt::from(a), BigInt::from(b), BigInt::from(den))
}

#[test]
fn r2_is_the_largest_squared_norm_of_cap_0s_region() {
    let loc = locality();
    assert_eq!(loc.r2(), &Q5::of(R2_FORM.0, R2_FORM.1, 1));
    let reached = ctx().regions.cap_vertices.iter().filter(|v| &dot(&v[..], &v[..]) == loc.r2()).count();
    assert!(reached > 0, "a vertex of cap 0's region reaches R²");
}

#[test]
fn every_grip_has_80_partners_and_the_relation_is_symmetric() {
    let loc = locality();
    assert_eq!(loc.grips(), 600);
    for c in 0..loc.grips() {
        assert_eq!(loc.partners(c).len(), PARTNERS, "grip {c}");
        assert!(!loc.independent(c, c), "grip {c} depends on itself");
        assert!(loc.delta(c, c).is_none());
        for d in 0..loc.grips() {
            assert_eq!(loc.independent(c, d), loc.independent(d, c), "({c}, {d})");
            let partner = loc.partners(c).binary_search(&(d as u16)).is_ok();
            assert_eq!(partner, c != d && !loc.independent(c, d), "({c}, {d})");
        }
    }
}

#[test]
fn the_bits_are_one_per_pair_in_ascending_order_set_when_independent() {
    let loc = locality();
    let n = loc.grips();
    let pairs = n * (n - 1) / 2;
    assert_eq!(loc.bits().len(), pairs.div_ceil(8));
    let mut k = 0usize;
    for c in 0..n {
        for d in c + 1..n {
            assert_eq!(loc.bits()[k / 8] >> (k % 8) & 1 == 1, loc.independent(c, d), "({c}, {d})");
            k += 1;
        }
    }
    let ones: u32 = loc.bits().iter().map(|b| b.count_ones()).sum();
    assert_eq!(ones as usize, pairs - n * PARTNERS / 2);
    if pairs % 8 != 0 {
        assert_eq!(loc.bits()[loc.bits().len() - 1] >> (pairs % 8), 0, "padding bits are clear");
    }
}

#[test]
fn grip_0_matches_stage_14s_exact_shells() {
    let loc = locality();
    let path = repository().join("research/jumbling/restoration/14-blocking-structure/results/locality-summary.json");
    let summary = json::parse(&std::fs::read(path).expect("the stage 14 summary")).expect("JSON");
    assert_eq!(exact(summary.get("R2").and_then(|v| v.as_str()).expect("R2")), *loc.r2());
    assert_eq!(summary.get("partner_count").and_then(|v| v.as_i64()), Some(PARTNERS as i64));
    let shells = summary.get("shells").and_then(|v| v.as_array()).expect("shells");
    let mut seen = vec![false; loc.grips()];
    let mut interacting = 0;
    for shell in shells {
        let dot_value = exact(shell.get("dot").and_then(|v| v.as_str()).expect("dot"));
        let delta = exact(shell.get("delta").and_then(|v| v.as_str()).expect("delta"));
        let interacts = shell.get("interacts") == Some(&json::Value::Bool(true));
        let grips = shell.get("grips").and_then(|v| v.as_array()).expect("grips");
        assert_eq!(shell.get("count").and_then(|v| v.as_i64()), Some(grips.len() as i64));
        for g in grips {
            let d = g.as_i64().expect("a grip") as usize;
            assert!(!seen[d], "grip {d} in two shells");
            seen[d] = true;
            if d == 0 {
                assert!(!interacts && loc.delta(0, 0).is_none(), "the diagonal is no pair");
                continue;
            }
            assert_eq!(loc.delta(0, d), Some((&dot_value, &delta)), "(0, {d})");
            assert_eq!(loc.independent(0, d), !interacts, "(0, {d})");
            interacting += usize::from(interacts);
        }
    }
    assert!(seen.iter().all(|&s| s), "the shells cover every grip");
    assert_eq!(interacting, PARTNERS);
    // Every distinct product of distinct poles is a shell of grip 0, since K⁺ is transitive on the grips.
    assert_eq!(loc.shells().len(), shells.len() - 1);
}

#[test]
fn delta_is_recomputed_from_the_exact_poles() {
    let (loc, model) = (locality(), &ctx().model);
    let two_kappa_sq = model.kappa.mul(&model.kappa).mul(&Q5::int(2));
    for c in (0..loc.grips()).step_by(37) {
        for d in (0..loc.grips()).filter(|&d| d != c) {
            let p = dot(&model.poles[c], &model.poles[d]);
            let delta = loc.r2().mul(&model.nn.add(&p)).sub(&two_kappa_sq);
            assert_eq!(loc.delta(c, d), Some((&p, &delta)), "({c}, {d})");
            assert_eq!(loc.independent(c, d), delta.sign() <= 0, "({c}, {d})");
        }
    }
}

#[test]
fn antipodal_grips_are_independent_and_interaction_falls_with_the_angle() {
    let (loc, model) = (locality(), &ctx().model);
    for c in 0..loc.grips() {
        let opposite: Vec<Q5> = model.poles[c].iter().map(Q5::neg).collect();
        let d = model.pole_of(&opposite).expect("the antipodal grip");
        assert!(loc.independent(c, d), "({c}, {d})");
        assert_eq!(loc.delta(c, d).map(|(p, _)| p.add(&model.nn)), Some(Q5::zero()));
    }
    // Interaction is a threshold on n_c·n_d: every interacting shell lies above every independent one.
    let shells = loc.shells();
    let first = shells.iter().position(|(_, delta)| delta.sign() > 0).expect("an interacting shell");
    assert!(shells[..first].iter().all(|(_, delta)| delta.sign() <= 0));
    assert!(shells[first..].iter().all(|(_, delta)| delta.sign() > 0));
}

/// The relation `c600_run::trace::Relation::jumble_locality` builds from these bits under the pinned I_a menu: it keys
/// menu words by the model's exact Δ. CT-LB1-06 (part: LB1.10's jumbling half on the model in `assets/`): with grips
/// a < b < c, b independent of a and of c, and a interacting with c, bca and cab both key to bca and abc keys to
/// itself; an antipodal pair swaps into grip order, and letters at one grip never do.
#[test]
fn ct_lb1_06_the_models_relation_keys_menu_words_by_the_exact_delta() {
    let loc = locality();
    let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    let record = json::parse(&fixture).expect("JSON").get("menu").expect("a menu").clone();
    let menu = Menu::from_record(ctx(), &record).expect("the pinned I_a menu");
    let block = ctx().identity(Some(&menu));
    let identity = MenuIdentity {
        puzzle: "jumbling".into(),
        model_id: block.model_id,
        menu_id: block.menu_id.expect("a menu identity"),
        menu_order: block.menu_order.expect("a menu order"),
        contract_rev: block.contract_rev.expect("a contract revision"),
    };
    let items = menu.items.len() as u32;
    let r = Relation::jumble_locality(&identity, loc.grips() as u32, items, loc.bits().to_vec()).expect("a relation");
    assert_eq!((r.header().contract.as_str(), r.header().generators), (JUMBLE_LOCALITY, 600));
    for c in 0..loc.grips() {
        for d in 0..loc.grips() {
            let (gc, gd) = (c as u32, d as u32);
            assert_eq!(r.menu_independent([gc, 1], [gd, 0]), c != d && loc.independent(c, d), "({c}, {d})");
        }
    }

    let a = 0usize;
    let (b, c) = (loc.partners(a).iter().map(|&c| c as usize))
        .find_map(|c| (a + 1..c).find(|&b| loc.independent(a, b) && loc.independent(b, c)).map(|b| (b, c)))
        .expect("a partner c of a with a grip b between them independent of both");
    assert!(!loc.independent(a, c));
    let [a, b, c] = [a, b, c].map(|g| [g as u32, 1]);
    let k = |w: &[[u32; 2]]| r.menu_key(w).expect("letters of the menu").menu_letters().to_vec();
    assert_eq!(k(&[b, c, a]), [b, c, a]);
    assert_eq!(k(&[c, a, b]), [b, c, a]);
    assert_eq!(k(&[a, b, c]), [a, b, c]);
    let opposite: Vec<Q5> = ctx().model.poles[0].iter().map(Q5::neg).collect();
    let antipode = ctx().model.pole_of(&opposite).expect("the antipodal grip") as u32;
    assert_eq!(k(&[[antipode, 1], [0, 2]]), [[0, 2], [antipode, 1]]);
    assert_eq!(k(&[[0, 2], [0, 1]]), [[0, 2], [0, 1]]);
    assert_eq!(r.menu_key(&[[0, items]]), None);
}
