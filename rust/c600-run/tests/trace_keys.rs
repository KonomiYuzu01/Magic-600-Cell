//! LB1.10's trace key `trace-key/1` at run time (LP2 draft; LP3's plan, section 2): the least topological word of a
//! Full word under the independence relation `full-disjoint-support/1` that a run pins. `c600-run` does not read the
//! model, so these tests build the relation from synthetic supports in which generators 1 and 3, 1 and 6, 4 and 5, and
//! 5 and 6 share a slot and every other generator moves a slot of its own. The model's 1,200 supports are tested in
//! `c600-tools` (`supports.rs`).

mod conformance;

use std::cmp::Ordering;
use std::sync::Arc;

use c600_run::interp::code::{INCOMPATIBLE_ALPHABET, NONCANONICAL_VALUE};
use c600_run::interp::{Host, NoHost, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome;
use c600_run::trace::{
    check_keys, letter_order, tokens_order, Relation, RelationError, TraceHeader, TraceKey, FULL_DISJOINT_SUPPORT,
    FULL_GENERATORS, KEY_CONTRACT_MISMATCH, SLOT_LIMIT, TRACE_KEY,
};
use c600_run::value::{equal, key_order, Value};
use conformance::*;

const PUZZLE: &str = "600-cell-Full";

/// Generators 1 and 3 share slot 1, 1 and 6 slot 0, 4 and 5 slot 5, and 5 and 6 slot 6; generator 2 and every
/// generator from 7 on move a slot of their own.
fn supports() -> Vec<Vec<i32>> {
    (1..=FULL_GENERATORS as i32)
        .map(|g| match g {
            1 => vec![0, 1],
            2 => vec![2],
            3 => vec![1, 3],
            4 => vec![4, 5],
            5 => vec![5, 6],
            6 => vec![6, 0],
            g => vec![g + 10],
        })
        .collect()
}

fn relation_of(model_id: &str, supports: &[Vec<i32>]) -> Relation {
    Relation::full_disjoint_support(PUZZLE, model_id, supports).expect("a relation")
}

fn relation() -> Relation {
    relation_of("model-1", &supports())
}

fn pinned(r: &Relation) -> Options<'_> {
    Options { relation: Some(r), ..Options::new(WIDE) }
}

fn key(r: &Relation, letters: &[i32]) -> Value {
    Value::TraceEqualityKey(Arc::new(r.key(letters).expect("letters of the alphabet")))
}

fn completed(run: &Run) {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
}

fn stopped(run: &Run, code: &str) {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Error, Some(code)), "{:?}", run.summary.stop);
}

fn output(run: &Run) -> &str {
    completed(run);
    std::str::from_utf8(run.output.as_deref().expect("a completed run")).expect("UTF-8")
}

/// The least topological word by its definition, in quadratic time: repeatedly emit the least pending occurrence that
/// no earlier pending occurrence of a dependent letter, its own generator's included, precedes, the earlier of two
/// equal letters first.
fn naive(r: &Relation, w: &[i32]) -> Vec<i32> {
    let mut pending: Vec<usize> = (0..w.len()).collect();
    let mut out = Vec::new();
    while !pending.is_empty() {
        let available =
            pending.iter().copied().filter(|&j| pending.iter().all(|&i| i >= j || r.independent(w[i], w[j])));
        let least = available.min_by(|&x, &y| letter_order(w[x], w[y]).then(x.cmp(&y))).expect("an available one");
        out.push(w[least]);
        pending.retain(|&p| p != least);
    }
    out
}

#[test]
fn distinct_generators_are_independent_exactly_when_their_supports_are_disjoint() {
    let r = relation();
    assert!(r.independent(1, 2) && r.independent(-2, 3) && r.independent(2, -3) && r.independent(1, 4));
    assert!(r.independent(3, 6) && r.independent(1200, -1) && r.independent(7, 8));
    for (a, b) in [(1, 3), (-3, 1), (1, -6), (4, 5), (-5, -6)] {
        assert!(!r.independent(a, b) && !r.independent(b, a), "{a} and {b} share a slot, whatever the signs");
    }
    assert!(!r.independent(1, 1) && !r.independent(1, -1) && !r.independent(-7, 7), "a generator depends on itself");
    assert!(!r.independent(0, 2) && !r.independent(2, 1201), "a letter outside the alphabet is independent of nothing");
    assert!(r.holds(-1200) && r.holds(1) && !r.holds(0) && !r.holds(1201) && !r.holds(i32::MIN));

    let h = r.header();
    assert_eq!((h.puzzle.as_str(), h.model_id.as_str(), h.generators), (PUZZLE, "model-1", 1200));
    assert_eq!((h.key.as_str(), h.contract.as_str()), ("trace-key/1", "full-disjoint-support/1"));
    assert_eq!((TRACE_KEY, FULL_DISJOINT_SUPPORT), ("trace-key/1", "full-disjoint-support/1"));
    assert_eq!(r.bits().len(), 1200 * 1199 / 2 / 8, "one bit per pair of distinct generators");

    // The wire form recomputes the digest from the parts, so equal parts are one relation.
    let again = Relation::from_parts(FULL_DISJOINT_SUPPORT, PUZZLE, "model-1", 1200, r.bits().to_vec()).unwrap();
    assert_eq!((&again, again.digest(), again.header()), (&r, r.digest(), r.header()));
    // A slot named twice in one support is one slot.
    let mut twice = supports();
    twice[1] = vec![2, 2];
    assert_eq!(relation_of("model-1", &twice), r);
    // One more independent pair is another relation, with another digest; so is another model.
    let mut apart = supports();
    apart[2] = vec![3];
    let other = relation_of("model-1", &apart);
    assert!(other.independent(1, 3) && other != r);
    assert_eq!(r.header().differing(other.header()), Some("digest"));
    assert_eq!(r.header().differing(relation_of("model-2", &supports()).header()), Some("model_id"));
    assert_eq!(r.header().differing(r.header()), None);
}

#[test]
fn a_relation_needs_one_bounded_nonempty_support_per_generator_and_a_named_model() {
    let s = supports();
    let refused =
        |puzzle: &str, model_id: &str, s: &[Vec<i32>]| Relation::full_disjoint_support(puzzle, model_id, s).err();
    assert_eq!(refused(PUZZLE, "model-1", &s[..1199]), Some(RelationError::Generators(1199)));
    let mut more = s.clone();
    more.push(vec![5000]);
    assert_eq!(refused(PUZZLE, "model-1", &more), Some(RelationError::Generators(1201)));
    let mut empty = s.clone();
    empty[6].clear();
    assert_eq!(refused(PUZZLE, "model-1", &empty), Some(RelationError::EmptySupport(7)));
    for slot in [-1, SLOT_LIMIT, i32::MAX] {
        let mut outside = s.clone();
        outside[1] = vec![2, slot];
        assert_eq!(refused(PUZZLE, "model-1", &outside), Some(RelationError::Slot { generator: 2, slot }));
    }
    assert_eq!(refused("", "model-1", &s), Some(RelationError::Identity("puzzle")));
    assert_eq!(refused(PUZZLE, "", &s), Some(RelationError::Identity("model_id")));

    let bits = relation().bits().to_vec();
    let parts = |contract: &str, puzzle: &str, generators: u32, bits: &[u8]| {
        Relation::from_parts(contract, puzzle, "model-1", generators, bits.to_vec()).err()
    };
    let other = "full-disjoint-support/2";
    assert_eq!(parts(other, PUZZLE, 1200, &bits), Some(RelationError::Contract(other.into())));
    assert_eq!(parts(FULL_DISJOINT_SUPPORT, "", 1200, &bits), Some(RelationError::Identity("puzzle")));
    assert_eq!(parts(FULL_DISJOINT_SUPPORT, PUZZLE, 1199, &bits), Some(RelationError::Generators(1199)));
    assert_eq!(parts(FULL_DISJOINT_SUPPORT, PUZZLE, 1200, &bits[1..]), Some(RelationError::Bits));
    assert_eq!(parts(FULL_DISJOINT_SUPPORT, PUZZLE, 1200, &[&bits[..], &[0]].concat()), Some(RelationError::Bits));
    for refusal in [RelationError::Bits, RelationError::Slot { generator: 2, slot: -1 }, RelationError::Generators(9)] {
        assert!(!refusal.to_string().is_empty());
    }
}

/// LG2.23's order of Full tokens: by absolute generator ID, then by sign with the inverse first, so -1 < 1 < -2 < 2;
/// sequences lexicographically, a proper prefix first. A word key (`word-key/1`) follows it as a set element.
#[test]
fn full_tokens_order_by_generator_then_sign_with_the_inverse_first() {
    let order = [-1, 1, -2, 2, -1200, 1200];
    for (i, &a) in order.iter().enumerate() {
        for (j, &b) in order.iter().enumerate() {
            assert_eq!(letter_order(a, b), i.cmp(&j), "{a} {b}");
        }
    }
    assert_eq!(tokens_order(&[1], &[1, -1]), Ordering::Less);
    assert_eq!(tokens_order(&[], &[-1]), Ordering::Less);
    assert_eq!(tokens_order(&[1, 3], &[-2]), Ordering::Less);
    assert_eq!(tokens_order(&[2, -1], &[2, 1]), Ordering::Less);
    let wk = |letters: &[i32]| Value::WordEqualityKey(letters.into());
    assert_eq!(key_order(&wk(&[1]), &wk(&[-1])), Ordering::Greater);
    assert_eq!(key_order(&wk(&[-2]), &wk(&[1])), Ordering::Greater);
    let Value::Set(set) = Value::set(vec![wk(&[2]), wk(&[-2]), wk(&[1]), wk(&[-1]), wk(&[])]) else { panic!("a set") };
    let keys: Vec<_> =
        set.items().iter().map(|v| if let Value::WordEqualityKey(k) = v { k.to_vec() } else { vec![] }).collect();
    assert_eq!(keys, [vec![], vec![-1], vec![1], vec![-2], vec![2]]);

    // Trace keys under one header order the same way.
    let r = relation();
    let Value::Set(set) = Value::set(vec![key(&r, &[2]), key(&r, &[-1]), key(&r, &[1, 2]), key(&r, &[1])]) else {
        panic!("a set")
    };
    let letters: Vec<_> = set
        .items()
        .iter()
        .map(|v| if let Value::TraceEqualityKey(k) = v { k.letters().to_vec() } else { vec![] })
        .collect();
    assert_eq!(letters, [vec![-1], vec![1], vec![1, 2], vec![2]]);
}

const KEYS: &str = "export fn traces(x: Word, y: Word) -> Bool effects [] {\n    \
    return exact_eq(trace_key(x), trace_key(y));\n}\n\n\
    export fn words(x: Word, y: Word) -> Bool effects [] {\n    return exact_eq(word_key(x), word_key(y));\n}\n\n\
    export fn kept(x: Word, y: Word) -> Bool effects [] {\n    \
    return contains(insert(set<TraceEqualityKey>(), trace_key(x)), trace_key(y));\n}\n\n\
    export fn key(x: Word) -> TraceEqualityKey effects [] {\n    return trace_key(x);\n}\n";

/// CT-LB1-06 (part: LB1.10's Full half, run through the interpreter; not run: the jumbling half, `jumble-locality/1`
/// over menu letters, which waits for an exact Δ(c,d) entry in `c600-jumble`): with a < b < c, b independent of a and
/// c, and a dependent on c, bca and cab both key to bca, which keeps c before a, abc keys to itself, and the word keys
/// of bca and cab stay distinct. Every occurrence survives: no inverse pair cancels and no letters merge. A key has no
/// canonical form, so a run that would emit one stops with `PS1:input.noncanonical_value`.
#[test]
fn ct_lb1_06_bca_and_cab_have_one_trace_key_and_abc_another() {
    let r = relation();
    let (a, b, c) = (1, 2, 3);
    let k = |w: &[i32]| r.key(w).expect("letters of the alphabet").letters().to_vec();
    assert_eq!(k(&[b, c, a]), [b, c, a]);
    assert_eq!(k(&[c, a, b]), [b, c, a]);
    assert_eq!(k(&[a, b, c]), [a, b, c]);
    // Identity-domain tokens survive: an inverse pair, a repeated letter and both signs stay.
    assert_eq!(k(&[1, -1]), [1, -1]);
    assert_eq!(k(&[-1, 1]), [-1, 1]);
    assert_eq!(k(&[3, -3, 2]), [2, 3, -3]);
    assert_eq!(k(&[2, 2, -2]), [2, 2, -2]);
    assert_eq!(k(&[-2, 1]), [1, -2]);
    assert_eq!(k(&[2, -1]), [-1, 2]);
    assert_eq!(k(&[3, 1]), [3, 1], "dependent letters never swap");
    assert_eq!(k(&[]), Vec::<i32>::new());
    assert_eq!(r.key(&[1, 0]), None);

    let (_, loaded) = build(&[&m(KEYS)]);
    let options = pinned(&r);
    let ran = |entry: &str, x: &[i32], y: &[i32]| {
        loaded.run("m", entry, vec![Value::word(x), Value::word(y)], &mut NoHost, &options).unwrap()
    };
    for (x, y, traces, words) in [
        (&[b, c, a][..], &[c, a, b][..], "true\n", "false\n"),
        (&[a, b, c], &[b, c, a], "false\n", "false\n"),
        (&[a, b, c], &[a, b, c], "true\n", "true\n"),
        (&[1, -1], &[], "false\n", "true\n"),
        (&[3, 1], &[1, 3], "false\n", "false\n"),
        (&[4, 1, 2, 6], &[2, 1, 4, 6], "true\n", "false\n"),
    ] {
        assert_eq!(output(&ran("traces", x, y)), traces, "trace keys of {x:?} and {y:?}");
        assert_eq!(output(&ran("words", x, y)), words, "word keys of {x:?} and {y:?}");
        assert_eq!(output(&ran("kept", x, y)), traces, "a set of trace keys holds {y:?}'s key after {x:?}'s");
    }
    let shown = loaded.run("m", "key", vec![Value::word(&[1])], &mut NoHost, &options).unwrap();
    stopped(&shown, NONCANONICAL_VALUE);
}

/// The scan agrees with the definition of the least topological word on many words, and keys a word exactly as it
/// keys any word that adjacent swaps of independent letters reach.
#[test]
fn the_key_is_the_least_topological_word_and_swaps_of_independent_letters_keep_it() {
    let r = relation();
    let alphabet = [1, 2, 3, 4, 5, 6, 7, 1200];
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let mut next = |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % bound as u64) as usize
    };
    for _ in 0..600 {
        let length = next(13);
        let w: Vec<i32> = (0..length)
            .map(|_| {
                let g = alphabet[next(alphabet.len())];
                if next(2) == 0 {
                    -g
                } else {
                    g
                }
            })
            .collect();
        let k = r.key(&w).expect("letters of the alphabet");
        assert_eq!(k.letters(), naive(&r, &w), "{w:?}");
        let mut sorted = w.clone();
        sorted.sort_unstable();
        let mut kept = k.letters().to_vec();
        kept.sort_unstable();
        assert_eq!(kept, sorted, "{w:?}: every occurrence is kept");
        // Swap one adjacent independent pair, if there is one: the key stays.
        if let Some(i) = (1..w.len()).find(|&i| r.independent(w[i - 1], w[i])) {
            let mut swapped = w.clone();
            swapped.swap(i - 1, i);
            assert_eq!(r.key(&swapped), Some(k.clone()), "{w:?} and {swapped:?}");
        }
        // The key's letters are their own key.
        assert_eq!(r.key(k.letters()).as_ref().map(TraceKey::letters), Some(k.letters()), "{w:?}");
    }
}

/// `trace_key` is one step, priced before the key is built by what its scan reads ([`Relation::reads`]) at
/// `expand_per_letter`, with two bytes reserved per letter. Without a pinned relation the run stops with LB1's key
/// contract code, and a letter outside the relation's alphabet, such as a letter 0, with its alphabet code.
#[test]
fn a_trace_key_is_priced_by_its_scan_and_needs_the_pinned_relation() {
    let r = relation();
    // 1,200 generators; per letter 1, the heap's 2 x 11 steps, and two reads of each generator its own depends on:
    // generator 1 depends on 3 and 6, generator 2 on none and generator 3 on 1.
    assert_eq!(r.reads(&[1, 2, 3]), 1200 + (1 + 22 + 4) + (1 + 22) + (1 + 22 + 2));
    assert_eq!(r.reads(&[]), 1200);

    let (_, loaded) = build(&[&m(KEYS)]);
    let options = pinned(&r);
    let w = [1, 2, 3];
    let ran = loaded.run("m", "traces", vec![Value::word(&w), Value::word(&w)], &mut NoHost, &options).unwrap();
    completed(&ran);
    let per_letter = options.tariffs.expand_per_letter;
    let rows: Vec<_> = ran
        .summary
        .rows
        .iter()
        .filter(|row| row.at.ends_with("#trace_key"))
        .map(|row| (row.steps, row.kernel_cost, row.memory))
        .collect();
    assert_eq!(rows, [(1, per_letter * r.reads(&w), 6); 2]);

    let unpinned = loaded.run("m", "traces", vec![Value::word(&w), Value::word(&w)], &mut NoHost, &Options::new(WIDE));
    let unpinned = unpinned.unwrap();
    stopped(&unpinned, KEY_CONTRACT_MISMATCH);
    assert!(unpinned.value.is_none());
    let zero = loaded.run("m", "traces", vec![Value::word(&[1, 0]), Value::word(&w)], &mut NoHost, &options).unwrap();
    stopped(&zero, INCOMPATIBLE_ALPHABET);
}

const ARGS: &str = "export fn same(k: TraceEqualityKey) -> Bool effects [] {\n    return exact_eq(k, k);\n}\n\n\
    export fn count(l: List<TraceEqualityKey>) -> Nat effects [] {\n    return length(l);\n}\n";

/// A trace key has no canonical form, so no argument holds one: the run is refused before it starts, as for any value
/// without one.
#[test]
fn an_argument_that_holds_a_trace_key_is_refused() {
    let (_, loaded) = build(&[&m(ARGS)]);
    let r = relation();
    let refused =
        |entry: &str, arg: Value| loaded.run("m", entry, vec![arg], &mut NoHost, &pinned(&r)).err().map(|e| e.code);
    assert_eq!(refused("same", key(&r, &[1])), Some(NONCANONICAL_VALUE));
    assert_eq!(refused("count", Value::list(vec![key(&r, &[1, 2])])), Some(NONCANONICAL_VALUE));
    assert_eq!(refused("count", Value::list(vec![])), None);
}

/// Answers every literal read with the value it holds.
struct Answering(Value);

impl Host for Answering {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 4096 })
    }

    fn perform(&mut self, r: &Request) -> Result<Value, &'static str> {
        match r {
            Request::Snapshot { .. } => Ok(self.0.clone()),
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

const READ: &str = "export fn agrees(w: Word) -> Bool effects [snapshot] {\n    \
    return exact_eq(read_snapshot<TraceEqualityKey>(\"best\"), trace_key(w));\n}\n";

/// A host's answer may hold a trace key only under the run's relation, as the key of its own trace. An answer under
/// another header (another model, key version or contract), letters out of key order, or any key where the run pins
/// no relation stops the run after the read's row, so two keys a run compares always share one header.
#[test]
fn a_host_answer_holds_trace_keys_only_under_the_pinned_relation() {
    let (_, loaded) = build(&[&m(READ)]);
    let r = relation();
    let run = |answer: Value, options: &Options| {
        loaded.run("m", "agrees", vec![Value::word(&[3, 1, 2])], &mut Answering(answer), options).unwrap()
    };
    let ok = run(key(&r, &[2, 3, 1]), &pinned(&r));
    assert_eq!(output(&ok), "true\n");

    let header = |edit: fn(&mut TraceHeader)| {
        let mut h = r.header().clone();
        edit(&mut h);
        Arc::new(h)
    };
    let foreign =
        |h: Arc<TraceHeader>, letters: &[i32]| Value::TraceEqualityKey(Arc::new(TraceKey::from_parts(h, letters)));
    let other_model = key(&relation_of("model-2", &supports()), &[2, 3, 1]);
    let mut apart = supports();
    apart[2] = vec![3];
    let other_relation = key(&relation_of("model-1", &apart), &[2, 3, 1]);
    for (answer, field) in [
        (other_model, "model_id"),
        (other_relation, "digest"),
        (foreign(header(|h| h.key = "trace-key/2".into()), &[2, 3, 1]), "key"),
        (foreign(header(|h| h.contract = "full-disjoint-support/2".into()), &[2, 3, 1]), "contract"),
        (foreign(header(|h| h.generators = 1199), &[2, 3, 1]), "generators"),
        (foreign(header(|h| h.puzzle = "600-cell-jumbled".into()), &[2, 3, 1]), "puzzle"),
    ] {
        let why = check_keys(&answer, Some(&r)).unwrap_err();
        assert!(why.contains(field), "{field}: {why}");
        let stop = run(answer, &pinned(&r));
        stopped(&stop, KEY_CONTRACT_MISMATCH);
        assert!(stop.value.is_none());
        assert!(stop.summary.rows.iter().any(|row| row.at.ends_with("#kernel")), "{:?}", stop.summary.rows);
    }
    // Letters that are not the key of their own trace: 1 and 2 are independent, so the key of 2 1 is 1 2.
    let unordered = foreign(Arc::new(r.header().clone()), &[2, 1]);
    assert!(check_keys(&unordered, Some(&r)).unwrap_err().contains("not the key"));
    stopped(&run(unordered, &pinned(&r)), KEY_CONTRACT_MISMATCH);
    // A letter outside the alphabet is not the key of any trace.
    stopped(&run(foreign(Arc::new(r.header().clone()), &[0]), &pinned(&r)), KEY_CONTRACT_MISMATCH);
    // Any trace key where the run pins no relation.
    assert!(check_keys(&key(&r, &[1]), None).is_err());
    stopped(&run(key(&r, &[2, 3, 1]), &Options::new(WIDE)), KEY_CONTRACT_MISMATCH);
    // Keys nested in records, lists and maps are checked; values without keys pass.
    let nested = Value::record(vec![(
        "k",
        Value::list(vec![key(&r, &[1]), foreign(header(|h| h.model_id = "x".into()), &[1])]),
    )]);
    assert!(check_keys(&nested, Some(&r)).unwrap_err().contains("model_id"));
    assert!(check_keys(&Value::map(vec![(Value::int(1), key(&r, &[4, 1]))]), Some(&r)).is_ok());
    assert!(check_keys(&Value::list(vec![Value::word(&[2, 1])]), None).is_ok());
    assert!(equal(&key(&r, &[2, 1]), &key(&r, &[1, 2])));
}
