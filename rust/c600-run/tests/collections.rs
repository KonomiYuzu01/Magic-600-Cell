//! Lists, word operations, sets and maps at run time (LP2 drafts until the IR freeze, PHS.49): their values,
//! canonical bytes and draft charges. One step per operation (DET.55 names no row for them); `expand_per_letter` per
//! letter built or scanned, `LIST_PER_ELEMENT` per element built and `compare_per_unit` per unit of a key comparison,
//! all recorded as drafts in `c600_run::interp`. The static halves are `c600-syntax`'s `collections.rs`.

use std::sync::Arc;

use c600_check::check;
use c600_run::interp::{code, load, Loaded, NoHost, Options, Run, LIST_PER_ELEMENT};
use c600_run::meter::{self, Amounts, Outcome, Summary};
use c600_run::value::{free_reduction, inverse, List, Value};
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

/// CT-LG4-01 (part: lists and words; not run: LG4.3's other ordered results and admitted routes other than RT1; sets
/// and maps are `ct_lg4_01_set_and_map_results_follow_their_key_order`): repeated runs give identical result bytes
/// and summaries, and a list's bytes follow its written order only, however it was built.
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

const SETS: &str = "export fn written() -> Set<Nat> effects [] {\n    return set<Nat>(3, 1, 2, 1);\n}\n\n\
    export fn reordered() -> Set<Nat> effects [] {\n    return set<Nat>(2, 3, 1);\n}\n\n\
    export fn inserted() -> Set<Nat> effects [] {\n    return insert(insert(insert(set<Nat>(), 2), 1), 3);\n}\n\n\
    export fn ratios() -> List<Set<Rat>> effects [] {\n    \
    return list<Set<Rat>>(set<Rat>(4/2, 2, 1/2), set<Rat>(1/2, 2, 2/1), insert(set<Rat>(2/1), 2));\n}\n\n\
    export fn has(s: Set<Nat>, x: Nat) -> Bool effects [] {\n    return contains(s, x);\n}\n\n\
    export fn listed(s: Set<Nat>) -> record { up: List<Nat>, down: List<Nat>, n: Nat } effects [] {\n    \
    return record { up = elements(s, ascending), down = elements(s, descending), n = length(s) };\n}\n";

const ONE_TWO_THREE: &str = "[{\"int\":\"1\"},{\"int\":\"2\"},{\"int\":\"3\"}]\n";

/// A set keeps each element once, in canonical key order (DET.9), whatever order it was written or inserted in (LP2
/// draft). Equal numbers in different representations are one element, and the explicit secondary key keeps the `Int`
/// before the `Rat`. A set of n written elements is priced at `LIST_PER_ELEMENT` per element and n·⌈log₂ n⌉
/// comparisons of the widest element, two of its units per comparison at `compare_per_unit` 1, and reserves every
/// element written; an insertion into n elements adds ⌈log₂(n + 1)⌉ comparisons and reserves the element it adds.
#[test]
fn a_set_keeps_each_element_once_in_key_order() {
    let loaded = build(SETS);
    let written = run(&loaded, "written", vec![]);
    assert_eq!(output(&written), ONE_TWO_THREE);
    // Four written elements of 2 bytes: 8 comparisons of 2 × 2 units; 4 + 4 × 2 bytes reserved, 4 + 3 × 2 kept.
    assert_eq!(rows(&written.summary, "set"), [(1, 4 * LIST_PER_ELEMENT + 8 * 4, 4 + 4 * 2)]);
    assert_eq!(written.value.as_ref().map(Value::size), Some(4 + 3 * 2));
    let reordered = run(&loaded, "reordered", vec![]);
    assert_eq!(output(&reordered), ONE_TWO_THREE);
    assert_eq!(rows(&reordered.summary, "set"), [(1, 3 * LIST_PER_ELEMENT + 6 * 4, 4 + 3 * 2)]);
    let inserted = run(&loaded, "inserted", vec![]);
    assert_eq!(output(&inserted), ONE_TWO_THREE);
    assert_eq!(rows(&inserted.summary, "set"), [(1, 0, 4)]);
    assert_eq!(rows(&inserted.summary, "insert"), [(1, 1, 4 + 2), (1, 2 + 4, 6 + 2), (1, 3 + 2 * 4, 8 + 2)]);

    let ratios = run(&loaded, "ratios", vec![]);
    let half_two = "[{\"rat\":[\"1\",\"2\"]},{\"int\":\"2\"}]";
    assert_eq!(output(&ratios), format!("[{half_two},{half_two},[{{\"int\":\"2\"}}]]\n"));

    let s = || Value::set(vec![Value::int(3), Value::int(1), Value::int(2)]);
    for (x, want) in [(2, "true\n"), (4, "false\n")] {
        let r = run(&loaded, "has", vec![s(), Value::int(x)]);
        assert_eq!(output(&r), want);
        // ⌈log₂ 4⌉ comparisons of a 2-byte probe; the boolean is reserved.
        assert_eq!(rows(&r.summary, "contains"), [(1, 2 * 4, 1)]);
    }
    let listed = run(&loaded, "listed", vec![s()]);
    assert_eq!(
        output(&listed),
        "{\"down\":[{\"int\":\"3\"},{\"int\":\"2\"},{\"int\":\"1\"}],\"n\":{\"int\":\"3\"},\
         \"up\":[{\"int\":\"1\"},{\"int\":\"2\"},{\"int\":\"3\"}]}\n"
    );
    assert_eq!(rows(&listed.summary, "elements"), [(1, 3, 4 + 3 * 2), (1, 3, 4 + 3 * 2)]);
    assert_eq!(rows(&listed.summary, "length"), [(1, 0, 2)]);
}

const MAPS: &str = "export fn written() -> Map<Nat, Text> effects [] {\n    \
    return map<Nat, Text>(2, \"b\", 1, \"a\", 2, \"c\");\n}\n\n\
    export fn inserted() -> Map<Nat, Text> effects [] {\n    \
    return insert(insert(insert(map<Nat, Text>(), 1, \"a\"), 2, \"b\"), 2, \"c\");\n}\n\n\
    export fn value_at(m: Map<Nat, Text>, k: Nat) -> Text effects [] {\n    return lookup(m, k);\n}\n\n\
    export fn has(m: Map<Nat, Text>, k: Nat) -> Bool effects [] {\n    return contains(m, k);\n}\n\n\
    export fn listed(m: Map<Nat, Text>) -> \
    record { k: List<Nat>, e: List<record { key: Nat, value: Text }>, n: Nat } effects [] {\n    \
    return record { k = keys(m, descending), e = entries(m, ascending), n = length(m) };\n}\n";

const ONE_A_TWO_C: &str = "[{\"key\":{\"int\":\"1\"},\"value\":\"a\"},{\"key\":{\"int\":\"2\"},\"value\":\"c\"}]\n";

/// A map keeps one entry per key, a `{ key, value }` record, in the canonical key order of its keys (DET.9), and a
/// later key's value replaces an earlier one's (LP2 draft). A map is priced as a set of its keys and reserves every
/// entry written. A lookup lends its value as a field read does, and a missing key stops with
/// `LG4:input.pointwise_failed` before its step; `keys` and `entries` give their named order (LG4.3).
#[test]
fn a_map_keeps_one_entry_per_key_in_key_order() {
    let loaded = build(MAPS);
    let written = run(&loaded, "written", vec![]);
    assert_eq!(output(&written), ONE_A_TWO_C);
    // Three entries of a 2-byte key and a one-scalar text, 4 + 2 + (4 + 8) bytes each: 6 comparisons of 2 × 2 units.
    assert_eq!(rows(&written.summary, "map"), [(1, 3 * LIST_PER_ELEMENT + 6 * 4, 4 + 3 * 18)]);
    assert_eq!(written.value.as_ref().map(Value::size), Some(4 + 2 * 18));
    let inserted = run(&loaded, "inserted", vec![]);
    assert_eq!(output(&inserted), ONE_A_TWO_C);
    assert_eq!(rows(&inserted.summary, "map"), [(1, 0, 4)]);
    assert_eq!(rows(&inserted.summary, "insert"), [(1, 1, 4 + 18), (1, 2 + 4, 22 + 18), (1, 3 + 2 * 4, 40 + 18)]);

    let m = || Value::map(vec![(Value::int(2), Value::text("c")), (Value::int(1), Value::text("a"))]);
    let found = run(&loaded, "value_at", vec![m(), Value::int(2)]);
    assert_eq!(output(&found), "\"c\"\n");
    assert_eq!(rows(&found.summary, "lookup"), [(1, 2 * 4, 0)]);
    let missing = run(&loaded, "value_at", vec![m(), Value::int(3)]);
    assert_eq!((missing.summary.outcome, missing.summary.code), (Outcome::Error, Some(meter::code::POINTWISE_FAILED)));
    assert!(rows(&missing.summary, "lookup").is_empty(), "the failed lookup is not charged as a step");
    for (k, want) in [(1, "true\n"), (3, "false\n")] {
        assert_eq!(output(&run(&loaded, "has", vec![m(), Value::int(k)])), want);
    }
    let listed = run(&loaded, "listed", vec![m()]);
    let entries = ONE_A_TWO_C.trim_end();
    let want = format!("{{\"e\":{entries},\"k\":[{{\"int\":\"2\"}},{{\"int\":\"1\"}}],\"n\":{{\"int\":\"2\"}}}}\n");
    assert_eq!(output(&listed), want);
    assert_eq!(rows(&listed.summary, "keys"), [(1, 2, 4 + 2 * 2)]);
    assert_eq!(rows(&listed.summary, "entries"), [(1, 2, 4 + 2 * 18)]);
}

fn refused(loaded: &Loaded, function: &str, args: Vec<Value>) -> &'static str {
    loaded.run("m", function, args, &mut NoHost, &Options::new(WIDE)).map(|_| ()).unwrap_err().code
}

/// A set or a map that a host builds out of canonical key order, with a repeated key or with an item that is not an
/// entry has no canonical form, so it is refused as an entry argument with `PS1:input.noncanonical_value`.
#[test]
fn a_host_set_or_map_out_of_key_order_is_refused() {
    let (sets, maps) = (build(SETS), build(MAPS));
    let raw = |items: Vec<Value>| Arc::new(List::new(items));
    let (one, two) = (Value::int(1), Value::int(2));
    for s in [raw(vec![two.clone(), one.clone()]), raw(vec![one.clone(), one.clone()])] {
        assert_eq!(refused(&sets, "has", vec![Value::Set(s), one.clone()]), code::NONCANONICAL_VALUE);
    }
    let entry = |k: &Value| Value::record(vec![("key", k.clone()), ("value", Value::text("a"))]);
    for m in [raw(vec![entry(&two), entry(&one)]), raw(vec![entry(&one), entry(&one)]), raw(vec![one.clone()])] {
        assert_eq!(refused(&maps, "has", vec![Value::Map(m), one.clone()]), code::NONCANONICAL_VALUE);
    }
    let canonical = Value::Set(raw(vec![one.clone(), two.clone()]));
    assert_eq!(output(&run(&sets, "has", vec![canonical, one])), "true\n");
}

/// CT-LG4-01 (part: sets and maps, whose listings name their order; not run: LG4.3's other ordered results, host
/// collection layouts other than this interpreter's sorted vectors, and admitted routes other than RT1) and CT-DET-05
/// (part: insertion order; not run: hash seeds and allocation perturbations, and draw order): repeated runs give
/// identical result bytes and summaries, and a set's or a map's bytes follow its canonical key order only, whatever
/// order its elements or entries were written or inserted in.
#[test]
fn ct_lg4_01_set_and_map_results_follow_their_key_order() {
    let (sets, maps) = (build(SETS), build(MAPS));
    let cases = [
        (&sets, "written", ONE_TWO_THREE),
        (&sets, "reordered", ONE_TWO_THREE),
        (&sets, "inserted", ONE_TWO_THREE),
        (&maps, "written", ONE_A_TWO_C),
        (&maps, "inserted", ONE_A_TWO_C),
    ];
    for (loaded, f, want) in cases {
        let (first, again) = (run(loaded, f, vec![]), run(loaded, f, vec![]));
        assert_eq!(first.summary, again.summary, "{f}");
        assert_eq!(first.output, again.output, "{f}");
        assert_eq!(output(&first), want, "{f}");
    }
    // Host values built from permuted items are the same value, with the same bytes.
    let permutations = [[1, 2, 3], [3, 1, 2], [2, 3, 1], [3, 2, 1]];
    let listed: Vec<Run> = permutations
        .iter()
        .map(|p| run(&sets, "listed", vec![Value::set(p.iter().map(|&n| Value::int(n)).collect())]))
        .collect();
    assert!(listed.windows(2).all(|w| w[0].output == w[1].output && w[0].summary == w[1].summary));
}
