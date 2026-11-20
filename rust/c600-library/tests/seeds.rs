//! LB2's seeds as library code (chapter 03's rows "Seeds and full seed analysis" and "Orbit catalogue and collateral
//! stage order"): the source of the module `seeds`, its two catalogue rows and the pure stage order, without a host.
//! `seed_report` runs whole, through the command layer, on the model's 35 seeds in `c600-tools/tests/seeds.rs`. A pass
//! is source evidence.

mod common;

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_library::{
    collateral_stage_order, lower_source, seeds_and_full_seed_analysis, seeds_sha256, SEEDS_ANALYSIS, SEEDS_EXPORTS,
    SEEDS_MODULE, SEEDS_ROWS, SEEDS_SOURCE, SEEDS_STAGE_ORDER,
};
use c600_run::interp::{load, NoHost, Options};
use c600_run::meter::Outcome;
use c600_run::value::Value;
use c600_syntax::{format, parse};
use common::WIDE;

/// The source parses with no diagnostic, is in the formatter's canonical layout with LF line endings, and lowers to one
/// module, `seeds`, that requests no capability and exports the two entries. The checker reports no diagnostic.
/// `seed_report` has the effect `snapshot` and reads KN2.4's `effect` row and no other, so under capability separation
/// it reads state and prepares no preview; `stage_order` is pure: no effect, capability, state read or row. The two
/// catalogue rows split the exports between them, their Effect field is the checker's effects of their functions, and
/// their digest is the source's.
#[test]
fn the_seeds_source_checks_and_reads_the_effect_row() {
    let file = parse(SEEDS_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(SEEDS_SOURCE), "the source is in the canonical layout");
    assert!(!SEEDS_SOURCE.contains('\r') && SEEDS_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(SEEDS_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, SEEDS_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, SEEDS_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let rows: BTreeSet<Row> = SEEDS_ROWS.iter().map(|row| Row::Read(row.to_string())).collect();
    for entry in SEEDS_EXPORTS {
        let summary = report.summary(SEEDS_MODULE, entry).expect("a summary");
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        let separated = separation(summary, &Policy::interim());
        if SEEDS_STAGE_ORDER.contains(&entry) {
            assert!(summary.effects.is_empty() && summary.state_reads.is_empty() && summary.rows.is_empty(), "{entry}");
            assert_eq!(separated, Separation { reads_state: false, prepares_previews: false });
        } else {
            assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]), "{entry}");
            assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]), "{entry}");
            assert_eq!(summary.rows, rows, "{entry}");
            assert_eq!(separated, Separation { reads_state: true, prepares_previews: false });
        }
    }

    let together: BTreeSet<&str> = SEEDS_ANALYSIS.iter().chain(&SEEDS_STAGE_ORDER).copied().collect();
    assert_eq!(together, SEEDS_EXPORTS.into_iter().collect(), "the two rows split the exports");
    assert_eq!(SEEDS_ANALYSIS.len() + SEEDS_STAGE_ORDER.len(), SEEDS_EXPORTS.len());
    for (row, entry, functions) in [
        (seeds_and_full_seed_analysis(), "Seeds and full seed analysis", &SEEDS_ANALYSIS[..]),
        (collateral_stage_order(), "Orbit catalogue and collateral stage order", &SEEDS_STAGE_ORDER[..]),
    ] {
        assert_eq!(row.entry, entry);
        assert_eq!((row.module, row.functions), (SEEDS_MODULE, functions));
        let effects: BTreeSet<Effect> = functions
            .iter()
            .flat_map(|f| report.summary(SEEDS_MODULE, f).expect("a summary").effects.iter().copied())
            .collect();
        assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), effects, "{entry}: Effect");
        assert!(functions.iter().all(|f| row.backing.contains(&format!("`seeds.{f}`"))), "{entry}: Backing");
        assert_eq!(row.source_sha256, sha256_hex(SEEDS_SOURCE.as_bytes()));
        assert_eq!(row.source_sha256, seeds_sha256());
    }
    assert!(seeds_and_full_seed_analysis().notes.contains("never assumed by omission"));
    assert!(collateral_stage_order().notes.contains("not orbit ID order"));
}

/// `stage_order` on the caller's rows, each an orbit and its seed's collateral, run with no host: the answer's
/// `order` and `cycle`.
fn stage_order(rows: &[(i64, &[i64])]) -> (Vec<i64>, Vec<i64>) {
    let loaded = load(&lower_source(SEEDS_SOURCE).unwrap()).unwrap();
    let ints = |orbits: &[i64]| Value::list(orbits.iter().map(|&orbit| Value::int(orbit)).collect());
    let row = |&(orbit, collateral): &(i64, &[i64])| {
        Value::record(vec![("collateral", ints(collateral)), ("orbit", Value::int(orbit))])
    };
    let seeds = Value::list(rows.iter().map(row).collect());
    let run = loaded.run(SEEDS_MODULE, "stage_order", vec![seeds], &mut NoHost, &Options::new(WIDE)).unwrap();
    assert_eq!(run.summary.outcome, Outcome::Completed, "{:?}", run.summary.code);
    let answer = run.value.expect("an answer");
    let Value::Record(record) = &answer else { panic!("a record: {answer:?}") };
    let names: Vec<&str> = record.fields().iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["cycle", "order"]);
    let list = |name: &str| {
        let Some(Value::List(items)) = record.get(name) else { panic!("a list {name}: {answer:?}") };
        let int = |item: &Value| {
            let Value::Int(n) = item else { panic!("an integer: {item:?}") };
            n.to_string().parse::<i64>().expect("an i64")
        };
        items.items().iter().map(int).collect::<Vec<i64>>()
    };
    (list("order"), list("cycle"))
}

/// The stage order follows the collateral: an orbit comes before every orbit its seed moves, whatever the order the
/// caller lists them in. Among the rows ready at a step the caller's order decides, so an order that already respects
/// every edge comes back unchanged and rows with no edge between them keep the caller's order, not their IDs'. A
/// collateral orbit with no row of its own holds nothing back. Rows a cycle holds, a self-edge included, come back in
/// `cycle` in the caller's order with every row behind them, and `order` holds the rows placed before it: no edge is
/// dropped to break the cycle.
#[test]
fn the_stage_order_follows_the_collateral_and_keeps_cycles() {
    assert_eq!(stage_order(&[(3, &[5]), (5, &[8]), (8, &[])]), (vec![3, 5, 8], vec![]));
    assert_eq!(stage_order(&[(8, &[]), (5, &[8]), (3, &[5])]), (vec![3, 5, 8], vec![]), "listed backwards");
    assert_eq!(stage_order(&[(3, &[5, 8]), (8, &[]), (5, &[8])]), (vec![3, 5, 8], vec![]));
    assert_eq!(stage_order(&[(9, &[]), (2, &[]), (4, &[])]), (vec![9, 2, 4], vec![]), "not orbit ID order");
    assert_eq!(stage_order(&[(4, &[]), (9, &[]), (2, &[])]), (vec![4, 9, 2], vec![]));
    assert_eq!(stage_order(&[(1, &[30]), (2, &[])]), (vec![1, 2], vec![]), "orbit 30 has no row");
    assert_eq!(stage_order(&[]), (vec![], vec![]));

    assert_eq!(stage_order(&[(1, &[2]), (2, &[1]), (6, &[])]), (vec![6], vec![1, 2]), "a cycle");
    assert_eq!(stage_order(&[(4, &[4]), (7, &[])]), (vec![7], vec![4]), "a self-edge");
    assert_eq!(stage_order(&[(1, &[2]), (2, &[1, 3]), (3, &[])]), (vec![], vec![1, 2, 3]), "held behind a cycle");
    assert_eq!(stage_order(&[(5, &[1]), (1, &[2]), (2, &[1])]), (vec![5], vec![1, 2]));
}
