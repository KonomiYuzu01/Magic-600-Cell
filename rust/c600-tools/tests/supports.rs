//! LB1.10's independence relation `full-disjoint-support/1` of the model in `assets/` (LP2 draft): the relation that
//! `full_relation` builds from the 1,200 generators' complete supports, checked against the kernel's supports of a
//! sample of generators, and CT-LB1-06's example on the pinned model, run through the interpreter with the relation
//! pinned.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_check::check;
use c600_kernel::{Kernel, PUZZLE_NAME};
use c600_run::interp::{load, NoHost, Options};
use c600_run::meter::{Amounts, Outcome};
use c600_run::trace::{Relation, FULL_DISJOINT_SUPPORT, TRACE_KEY};
use c600_run::value::Value;
use c600_syntax::{lower, parse};
use c600_tools::supports::full_relation;

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => kernel,
        Err(error) => panic!("the model in assets/ does not load: {error}"),
    })
}

/// The relation of the pinned model, built once for the tests of this file from 1,200 kernel effects.
fn pinned() -> &'static Relation {
    static PINNED: OnceLock<Relation> = OnceLock::new();
    PINNED.get_or_init(|| full_relation(kernel()).expect("the supports make a relation"))
}

/// The labelled slots that the one-letter word `[g]` moves, from the kernel's effect of that word.
fn support(g: i64) -> Vec<i32> {
    kernel().effect(&kernel().word(&[g]).expect("a word")).expect("an effect").source().to_vec()
}

/// Whether two ascending slot lists share no slot.
fn disjoint(a: &[i32], b: &[i32]) -> bool {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => return false,
        }
    }
    true
}

#[test]
fn the_relation_of_the_pinned_model_comes_from_its_supports() {
    let relation = pinned();
    let header = relation.header();
    let model_id = kernel().identity().model_id;
    assert_eq!(
        (header.puzzle.as_str(), header.model_id.as_str(), header.generators),
        (PUZZLE_NAME, &model_id[..], 1200)
    );
    assert_eq!((header.key.as_str(), header.contract.as_str()), (TRACE_KEY, FULL_DISJOINT_SUPPORT));
    assert_eq!(PUZZLE_NAME, "600-cell-Full");
    // The process keeps the relation it built: a second call gives the same relation.
    let again = full_relation(kernel()).expect("the kept relation");
    assert_eq!((&again, again.digest()), (relation, relation.digest()));

    // Generators across the alphabet: each support is nonempty and ascending, an inverse moves the same slots, and
    // the relation agrees with the supports for every pair of the sample, with every sign.
    let sample: Vec<i64> = (0..50).map(|k| 1 + 24 * k).chain([2, 3, 600, 1199, 1200]).collect();
    let supports: Vec<(i64, Vec<i32>)> = sample.iter().map(|&g| (g, support(g))).collect();
    for (g, slots) in &supports {
        assert!(!slots.is_empty() && slots.windows(2).all(|p| p[0] < p[1]), "generator {g}");
        assert_eq!(support(-g), *slots, "generator {g} and its inverse");
    }
    let (mut apart, mut together) = (0, 0);
    for (c, slots_c) in &supports {
        for (d, slots_d) in &supports {
            let expected = c != d && disjoint(slots_c, slots_d);
            let (c32, d32) = (*c as i32, *d as i32);
            for (x, y) in [(c32, d32), (-c32, d32), (c32, -d32), (-c32, -d32)] {
                assert_eq!(relation.independent(x, y), expected, "{x} {y}");
            }
            if c < d {
                *(if expected { &mut apart } else { &mut together }) += 1;
            }
        }
    }
    assert!(apart > 0 && together > 0, "{apart} independent and {together} dependent pairs");
    eprintln!("{apart} of {} sampled pairs independent", apart + together);
}

const KEYS: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
    export fn traces(x: Word, y: Word) -> Bool effects [] {\n    return exact_eq(trace_key(x), trace_key(y));\n}\n\n\
    export fn words(x: Word, y: Word) -> Bool effects [] {\n    return exact_eq(word_key(x), word_key(y));\n}\n";

/// CT-LB1-06 (part: LB1.10's Full half on the pinned model; not run here: the jumbling half, in
/// `c600-jumble/tests/locality.rs` and `c600-jumble-tool/tests/ct_lb1_06.rs`): for the first generators a < b < c of
/// the model, by ID, with b independent of a and c and a dependent on c, bca and cab have the trace key bca, abc keeps
/// its own, and the word keys stay distinct.
#[test]
fn ct_lb1_06_on_the_pinned_model() {
    let relation = pinned();
    let ind = |x: i32, y: i32| relation.independent(x, y);
    let (a, b, c) = (1..=1200)
        .flat_map(|a| (a + 2..=1200).filter(move |&c| !ind(a, c)).map(move |c| (a, c)))
        .find_map(|(a, c)| (a + 1..c).find(|&b| ind(a, b) && ind(b, c)).map(|b| (a, b, c)))
        .expect("three such generators");
    eprintln!("a, b, c = {a}, {b}, {c}");
    let key = |w: &[i32]| relation.key(w).expect("letters of the alphabet").letters().to_vec();
    assert_eq!(key(&[b, c, a]), [b, c, a]);
    assert_eq!(key(&[c, a, b]), [b, c, a]);
    assert_eq!(key(&[a, b, c]), [a, b, c]);
    assert_eq!(key(&[a, -a]), [a, -a], "nothing cancels");

    let file = parse(KEYS.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    assert!(check(&lowered.program).unwrap_or_else(|e| panic!("{e}")).is_admitted());
    let loaded = load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"));
    let options = Options { relation: Some(relation), ..Options::new(WIDE) };
    let same = |entry: &str, x: &[i32], y: &[i32]| {
        let run = loaded.run("m", entry, vec![Value::word(x), Value::word(y)], &mut NoHost, &options).unwrap();
        assert_eq!(run.summary.outcome, Outcome::Completed, "{:?}", run.summary.stop);
        run.output.expect("a completed run") == b"true\n"
    };
    assert!(same("traces", &[b, c, a], &[c, a, b]) && !same("words", &[b, c, a], &[c, a, b]));
    assert!(!same("traces", &[a, b, c], &[b, c, a]) && !same("traces", &[a, b, c], &[c, a, b]));
}
