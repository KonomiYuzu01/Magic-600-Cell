//! Lists and word operations at run time (LP2 drafts until the IR freeze, PHS.49): their values, canonical bytes and
//! draft charges. One step per operation (DET.55 names no row for them); `expand_per_letter` per letter built or
//! scanned and `LIST_PER_ELEMENT` per element built, both recorded as drafts in `c600_run::interp`. The static
//! halves are `c600-syntax`'s `collections.rs`.

use c600_check::check;
use c600_run::interp::{code, load, Loaded, NoHost, Options, Run, LIST_PER_ELEMENT};
use c600_run::meter::{self, Amounts, Outcome, Summary};
use c600_run::value::{free_reduction, inverse, Value};
use c600_syntax::{lower, parse};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const HEADER: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n";

/// Parses, lowers, checks and loads one module `m`; the checker must accept it.
fn build(declarations: &str) -> Loaded {
    let source = format!("{HEADER}{declarations}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"))
}

fn run(loaded: &Loaded, function: &str, args: Vec<Value>) -> Run {
    loaded.run("m", function, args, &mut NoHost, &Options::new(WIDE)).unwrap_or_else(|r| panic!("{r:?}"))
}

fn output(run: &Run) -> &str {
    assert_eq!(run.summary.outcome, Outcome::Completed, "{:?} {:?}", run.summary.code, run.summary.stop);
    std::str::from_utf8(run.output.as_deref().expect("a completed run")).expect("UTF-8")
}

/// The rows of operation `op` as (steps, kernel cost, memory), in order.
fn rows(summary: &Summary, op: &str) -> Vec<(u128, u128, u128)> {
    let suffix = format!("#{op}");
    summary.rows.iter().filter(|r| r.at.ends_with(&suffix)).map(|r| (r.steps, r.kernel_cost, r.memory)).collect()
}

fn nats(items: &[i64]) -> Value {
    Value::list(items.iter().map(|&n| Value::int(n)).collect())
}

const LISTS: &str = "export fn make() -> List<Nat> effects [] {\n    return push(list<Nat>(1, 2), 3);\n}\n\n\
    export fn second(xs: List<Nat>) -> Nat effects [] {\n    return index(xs, 1);\n}\n\n\
    export fn count(xs: List<Nat>, w: Word) -> Nat effects [] {\n    return exact_add(length(xs), length(w));\n}\n\n\
    export fn join(xs: List<Nat>, ys: List<Nat>) -> List<Nat> effects [] {\n    return concat(xs, ys);\n}\n\n\
    export fn words() -> List<Word> effects [] {\n    return list<Word>(word[+1, -2], word[]);\n}\n";

/// A list is built, read and measured in written order, and its canonical form is a JSON array (LP2 draft). An
/// integer of one magnitude byte is 2 logical bytes, so `[1, 2]` is 4 + 2 + 2 (DET.57's sequence row as a record
/// reads it), and a pushed or concatenated list reserves its exact size.
#[test]
fn a_list_is_built_read_and_measured_in_order() {
    let loaded = build(LISTS);
    let made = run(&loaded, "make", vec![]);
    assert_eq!(output(&made), "[{\"int\":\"1\"},{\"int\":\"2\"},{\"int\":\"3\"}]\n");
    assert_eq!(rows(&made.summary, "push"), [(1, 3 * LIST_PER_ELEMENT, 4 + 2 + 2 + 2)]);
    assert!(matches!(&made.value, Some(Value::List(l)) if l.items().len() == 3));

    let second = run(&loaded, "second", vec![nats(&[5, 7])]);
    assert_eq!(output(&second), "{\"int\":\"7\"}\n");
    // A borrowed list lends its element, as a field read does.
    assert_eq!(rows(&second.summary, "index"), [(1, 0, 0)]);
    let outside = run(&loaded, "second", vec![nats(&[5])]);
    assert_eq!((outside.summary.outcome, outside.summary.code), (Outcome::Error, Some(meter::code::POINTWISE_FAILED)));
    assert!(rows(&outside.summary, "index").is_empty(), "the failed index is not charged as a step");

    let counted = run(&loaded, "count", vec![nats(&[5, 7]), Value::word(&[1, 2, 3])]);
    assert_eq!(output(&counted), "{\"int\":\"5\"}\n");
    assert_eq!(rows(&counted.summary, "length"), [(1, 0, 2), (1, 0, 2)]);

    let joined = run(&loaded, "join", vec![nats(&[1]), nats(&[2, 3])]);
    assert_eq!(output(&joined), "[{\"int\":\"1\"},{\"int\":\"2\"},{\"int\":\"3\"}]\n");
    assert_eq!(rows(&joined.summary, "concat"), [(1, 3 * LIST_PER_ELEMENT, (4 + 2) + (4 + 2 + 2) - 4)]);

    // A word in a list stores its eight-byte length (DET.57); the empty word is an element of its own.
    let words = run(&loaded, "words", vec![]);
    assert_eq!(output(&words), "[[1,-2],[]]\n");
    assert_eq!(words.value.as_ref().map(Value::size), Some(4 + (2 * 2 + 8) + 8));
}

const WORDS: &str = "export fn ops(x: Word, y: Word) -> \
    record { i: Word, c: Word, k: Word, p: Word, n: Nat } effects [] {\n    \
    return record { i = inverse(x), c = conjugate(x, y), k = commutator(x, y), p = compose(x, y), n = length(x) };\n}\n";

/// CT-LG2-12 (part, the run-time language half: R-11's inverse, conjugate, commutator and composition are total on
/// the validated word domain, the empty word included, and form literal words without asserting admissibility; not
/// run: validation, which is the kernel's, and the jumbling fixtures under LG2.14's four hypotheses, which wait for
/// the jumbled puzzle's alphabet, M5; the static half is `c600-syntax`'s `collections.rs`). Conjugate is x·y·x⁻¹,
/// commutator x·y·x⁻¹·y⁻¹ and composition x then y, in chronological order; each is priced per letter it builds.
#[test]
fn ct_lg2_12_word_operations_are_total_on_literal_words() {
    let loaded = build(WORDS);
    let r = run(&loaded, "ops", vec![Value::word(&[1, -2]), Value::word(&[3])]);
    assert_eq!(
        output(&r),
        "{\"c\":[1,-2,3,2,-1],\"i\":[2,-1],\"k\":[1,-2,3,2,-1,-3],\"n\":{\"int\":\"2\"},\"p\":[1,-2,3]}\n"
    );
    // Letters built: inverse 2, conjugate 5, commutator 6, composition 3; two logical bytes per letter.
    assert_eq!(rows(&r.summary, "inverse"), [(1, 2, 4)]);
    assert_eq!(rows(&r.summary, "conjugate"), [(1, 5, 10)]);
    assert_eq!(rows(&r.summary, "commutator"), [(1, 6, 12)]);
    assert_eq!(rows(&r.summary, "compose"), [(1, 3, 6)]);
    assert_eq!(rows(&r.summary, "length"), [(1, 0, 2)]);
    // The empty word is in the domain: every operation is total on it.
    let empty = run(&loaded, "ops", vec![Value::word(&[]), Value::word(&[])]);
    assert_eq!(output(&empty), "{\"c\":[],\"i\":[],\"k\":[],\"n\":{\"int\":\"0\"},\"p\":[]}\n");
    // An inverse pair stays literal: forming a word never reduces it.
    let pair = run(&loaded, "ops", vec![Value::word(&[1]), Value::word(&[-1])]);
    assert_eq!(output(&pair), "{\"c\":[1,-1,-1],\"i\":[-1],\"k\":[1,-1,-1,1],\"n\":{\"int\":\"1\"},\"p\":[1,-1]}\n");
}

const KEYS: &str = "export fn same(x: Word, y: Word) -> Bool effects [] {\n    \
    return exact_eq(word_key(x), word_key(y));\n}\n\n\
    export fn key(x: Word) -> WordEqualityKey effects [] {\n    return word_key(x);\n}\n";

/// LG2.18's Full key: a chronological stack scan that cancels only adjacent inverse pairs, with no generator-order
/// relation or commuting swap. Equal keys are sound, not complete: words that commute keep unequal keys. A key has no
/// output form (LG2.13), so a run that would emit one stops with `PS1:input.noncanonical_value`.
#[test]
fn the_full_word_key_is_free_reduction() {
    assert_eq!(free_reduction(&[1, 2, -2, -1, 3]), [3]);
    assert_eq!(free_reduction(&[1, -1, 1]), [1]);
    assert_eq!(free_reduction(&[1, 2, -1, -2]), [1, 2, -1, -2]);
    assert_eq!(free_reduction(&[1, 1]), [1, 1]);
    assert_eq!(inverse(&[1, -2, 3]), [-3, 2, -1]);
    let loaded = build(KEYS);
    for (x, y, equal) in [
        (&[1, -1][..], &[][..], true),
        (&[1, -1, 2], &[2], true),
        (&[2, 1, -1, -2], &[], true),
        (&[1, -1], &[-1, 1], true),
        (&[1, 2], &[2, 1], false),
        (&[1, 1], &[], false),
        (&[1], &[-1], false),
    ] {
        let r = run(&loaded, "same", vec![Value::word(x), Value::word(y)]);
        let want = if equal { "true\n" } else { "false\n" };
        assert_eq!(output(&r), want, "{x:?} {y:?}");
        // One step per key, priced per letter scanned, with two bytes per letter reserved before the scan.
        assert_eq!(
            rows(&r.summary, "word_key"),
            [(1, x.len() as u128, 2 * x.len() as u128), (1, y.len() as u128, 2 * y.len() as u128)]
        );
    }
    let shown = run(&loaded, "key", vec![Value::word(&[1, -1, 2])]);
    assert_eq!((shown.summary.outcome, shown.summary.code), (Outcome::Error, Some(code::NONCANONICAL_VALUE)));
}

/// CT-LG4-01 (part: lists and words; not run: maps and sets with named orders, LG4.3's other ordered results, and
/// admitted routes other than RT1): repeated runs give identical result bytes and summaries, and a list's bytes follow
/// its written order only, however it was built.
#[test]
fn ct_lg4_01_list_and_word_results_repeat_byte_for_byte() {
    let source = "export fn literal() -> List<Word> effects [] {\n    \
                  return list<Word>(word[+1], word[-2, +3], word[]);\n}\n\n\
                  export fn pushed() -> List<Word> effects [] {\n    \
                  return push(push(push(list<Word>(), word[+1]), word[-2, +3]), word[]);\n}\n\n\
                  export fn joined() -> List<Word> effects [] {\n    \
                  return concat(list<Word>(word[+1]), concat(list<Word>(word[-2, +3]), list<Word>(word[])));\n}\n";
    let loaded = build(source);
    let mut bytes = Vec::new();
    for f in ["literal", "pushed", "joined"] {
        let first = run(&loaded, f, vec![]);
        let again = run(&loaded, f, vec![]);
        assert_eq!(first.summary, again.summary, "{f}");
        assert_eq!(first.output, again.output, "{f}");
        bytes.push(output(&first).to_string());
    }
    assert!(bytes.iter().all(|b| b == "[[1],[-2,3],[]]\n"), "{bytes:?}");
    let ops = build(WORDS);
    let args = || vec![Value::word(&[4, -5, 6]), Value::word(&[-4])];
    let (first, again) = (run(&ops, "ops", args()), run(&ops, "ops", args()));
    assert_eq!((first.summary, first.output), (again.summary, again.output));
}
