//! LB2's recipes and reports as library code (chapter 03's rows "Star and recipe construction" and "Collateral and
//! support"): the source of the module `recipes` and its two catalogue rows. CT-LB2-01 (part: the entries' source,
//! their checker summaries, their catalogue rows and the pure constructors; the whole, on the oracle's recipes through
//! the command layer, is `c600-tools/tests/ct_lb2_01.rs`). A pass is source evidence.

mod common;

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_library::{
    collateral_and_support, lower_source, recipes_sha256, star_and_recipe_construction, RECIPES_CONSTRUCTION,
    RECIPES_EXPORTS, RECIPES_MODULE, RECIPES_REPORTS, RECIPES_ROWS, RECIPES_SOURCE,
};
use c600_run::interp::{load, NoHost, Options};
use c600_run::meter::Outcome;
use c600_run::value::{equal, Value};
use c600_syntax::{format, parse};
use common::WIDE;

const CONSTRUCTORS: [&str; 3] = ["word_step", "star_step", "star_word"];

/// The source parses with no diagnostic, is in the formatter's canonical layout with LF line endings, and lowers to one
/// module, `recipes`, that requests no capability and exports the seven entries. The checker reports no diagnostic.
/// The three constructors are pure: no effect, capability, state read or row. The four other entries have the effect
/// `snapshot` and read KN2.4's `effect` row and no other, so under capability separation they read state and prepare
/// no preview. The two catalogue rows split the exports between them, their Effect field is the checker's effects of
/// their functions together, and their digest is the source's.
#[test]
fn the_recipes_source_checks_and_reads_the_effect_row() {
    let file = parse(RECIPES_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(RECIPES_SOURCE), "the source is in the canonical layout");
    assert!(!RECIPES_SOURCE.contains('\r') && RECIPES_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(RECIPES_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, RECIPES_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, RECIPES_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let rows: BTreeSet<Row> = RECIPES_ROWS.iter().map(|row| Row::Read(row.to_string())).collect();
    for entry in RECIPES_EXPORTS {
        let summary = report.summary(RECIPES_MODULE, entry).expect("a summary");
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        let separated = separation(summary, &Policy::interim());
        if CONSTRUCTORS.contains(&entry) {
            assert!(summary.effects.is_empty() && summary.state_reads.is_empty() && summary.rows.is_empty(), "{entry}");
            assert_eq!(separated, Separation { reads_state: false, prepares_previews: false });
        } else {
            assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]), "{entry}");
            assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]), "{entry}");
            assert_eq!(summary.rows, rows, "{entry}");
            assert_eq!(separated, Separation { reads_state: true, prepares_previews: false });
        }
    }

    let together: BTreeSet<&str> = RECIPES_CONSTRUCTION.iter().chain(&RECIPES_REPORTS).copied().collect();
    assert_eq!(together, RECIPES_EXPORTS.into_iter().collect(), "the two rows split the exports");
    assert_eq!(RECIPES_CONSTRUCTION.len() + RECIPES_REPORTS.len(), RECIPES_EXPORTS.len());
    for (row, entry, functions) in [
        (star_and_recipe_construction(), "Star and recipe construction", &RECIPES_CONSTRUCTION[..]),
        (collateral_and_support(), "Collateral and support", &RECIPES_REPORTS[..]),
    ] {
        assert_eq!(row.entry, entry);
        assert_eq!((row.module, row.functions), (RECIPES_MODULE, functions));
        let effects: BTreeSet<Effect> = functions
            .iter()
            .flat_map(|f| report.summary(RECIPES_MODULE, f).expect("a summary").effects.iter().copied())
            .collect();
        assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), effects, "{entry}: Effect");
        assert!(functions.iter().all(|f| row.backing.contains(&format!("`recipes.{f}`"))), "{entry}: Backing");
        assert_eq!(row.source_sha256, sha256_hex(RECIPES_SOURCE.as_bytes()));
        assert_eq!(row.source_sha256, recipes_sha256());
    }
    assert!(star_and_recipe_construction().notes.contains("setup⁻¹, relocation⁻¹, seed, relocation, setup"));
    assert!(collateral_and_support().notes.contains("never shortens the full support"));
}

/// The constructors run with no host, so they read nothing: `word_step` and `star_step` build the step records the
/// `effect` row takes, with the unused fields 0 and the empty word, and `star_word` writes setup⁻¹, relocation⁻¹, seed,
/// relocation and setup out in that order, literally, with no cancellation.
#[test]
fn the_constructors_build_steps_and_literal_star_words() {
    let loaded = load(&lower_source(RECIPES_SOURCE).unwrap()).unwrap();
    let call = |entry: &str, args: Vec<Value>| {
        let run = loaded.run(RECIPES_MODULE, entry, args, &mut NoHost, &Options::new(WIDE)).expect("the run starts");
        assert_eq!(run.summary.outcome, Outcome::Completed, "{entry}: {:?}", run.summary.code);
        run.value.expect("an answer")
    };
    let step = |kind: &str, moves: &[i32], node: i64, orbit: i64, sign: i64| {
        Value::record(vec![
            ("kind", Value::text(kind)),
            ("moves", Value::word(moves)),
            ("node", Value::int(node)),
            ("orbit", Value::int(orbit)),
            ("sign", Value::int(sign)),
        ])
    };
    assert!(equal(&call("word_step", vec![Value::word(&[3, -7])]), &step("word", &[3, -7], 0, 0, 0)));
    assert!(equal(&call("word_step", vec![Value::word(&[])]), &step("word", &[], 0, 0, 0)));
    let star = call("star_step", vec![Value::int(14), Value::int(5102), Value::int(-1)]);
    assert!(equal(&star, &step("star", &[], 5102, 14, -1)));

    let (setup, relocation, seed) = ([5, -9], [30, 83], [1, 4, 1]);
    let built = call("star_word", vec![Value::word(&setup), Value::word(&relocation), Value::word(&seed)]);
    assert!(equal(&built, &Value::word(&[9, -5, -83, -30, 1, 4, 1, 30, 83, 5, -9])));
    let no_setup = call("star_word", vec![Value::word(&[]), Value::word(&[2]), Value::word(&[-2])]);
    assert!(equal(&no_setup, &Value::word(&[-2, -2, 2])), "literal: nothing cancels");
}
