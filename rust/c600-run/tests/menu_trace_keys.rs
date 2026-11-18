//! LB1.10's trace key of a menu word at run time (LP2 draft; LP3's plan, section 2): the least topological word under
//! `jumble-locality/1`, the relation of the jumbling model's 600 grips that a run pins with its menu alphabet. Letters
//! at one grip are dependent, the identity item's among them, and two distinct grips are independent exactly when the
//! relation's bit for the pair is set. `c600-run` does not read the model, so these tests build the relation from
//! synthetic bits in which grips 0 and 2, 1 and 3, and 2 and 599 interact and every other pair is independent;
//! `c600-jumble` regenerates the model's bits from the exact Δ(c,d) (`tests/locality.rs`).

mod conformance;

use std::cmp::Ordering;
use std::sync::Arc;

use c600_run::interp::code::{INCOMPATIBLE_ALPHABET, NONCANONICAL_VALUE};
use c600_run::interp::{Host, NoHost, Options, Request, RowTariff, Run, Sink};
use c600_run::menu::{MenuAlphabet, MenuIdentity};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome;
use c600_run::trace::{
    check_keys, MenuHeader, Relation, RelationError, TraceHeader, TraceKey, FULL_DISJOINT_SUPPORT, FULL_GENERATORS,
    JUMBLE_GRIPS, JUMBLE_LOCALITY, KEY_CONTRACT_MISMATCH, TRACE_KEY,
};
use c600_run::value::{equal, key_order, Value};
use conformance::*;

const INTERACTING: [(usize, usize); 3] = [(0, 2), (1, 3), (2, 599)];

fn identity(menu_id: &str) -> MenuIdentity {
    MenuIdentity {
        puzzle: "600-cell-jumbled".into(),
        model_id: "model-1".into(),
        menu_id: menu_id.into(),
        menu_order: "order-1".into(),
        contract_rev: "rev-1".into(),
    }
}

/// The 600 grips and a menu of three items: item 0 is the identity rotation, items 1 and 2 are each other's inverses.
fn alphabet(menu_id: &str) -> MenuAlphabet {
    MenuAlphabet::new(identity(menu_id), JUMBLE_GRIPS, vec![0, 2, 1]).expect("an alphabet")
}

/// One bit per pair (c, d) of the 600 grips with c < d, set unless the pair is in `interacting`.
fn bits(interacting: &[(usize, usize)]) -> Vec<u8> {
    let n = JUMBLE_GRIPS as usize;
    let mut bits = vec![0u8; (n * (n - 1) / 2).div_ceil(8)];
    let mut k = 0;
    for c in 0..n {
        for d in c + 1..n {
            if !interacting.contains(&(c, d)) {
                bits[k / 8] |= 1 << (k % 8);
            }
            k += 1;
        }
    }
    bits
}

fn relation_of(identity: &MenuIdentity, interacting: &[(usize, usize)]) -> Relation {
    Relation::jumble_locality(identity, JUMBLE_GRIPS, 3, bits(interacting)).expect("a relation")
}

fn relation() -> Relation {
    relation_of(&identity("menu-a"), &INTERACTING)
}

fn pinned<'a>(a: &'a MenuAlphabet, r: &'a Relation) -> Options<'a> {
    Options { menu: Some(a), relation: Some(r), ..Options::new(WIDE) }
}

fn menu(a: &MenuAlphabet, letters: &[[u32; 2]]) -> Value {
    Value::menu_word(a.shared_identity(), letters).expect("indices below 600")
}

fn key(r: &Relation, letters: &[[u32; 2]]) -> Value {
    Value::TraceEqualityKey(Arc::new(r.menu_key(letters).expect("letters of the alphabet")))
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

/// The least topological word by its definition, as in `trace_keys.rs`, with menu letters ordered by grip and then
/// item.
fn naive(r: &Relation, w: &[[u32; 2]]) -> Vec<[u32; 2]> {
    let mut pending: Vec<usize> = (0..w.len()).collect();
    let mut out = Vec::new();
    while !pending.is_empty() {
        let available =
            pending.iter().copied().filter(|&j| pending.iter().all(|&i| i >= j || r.menu_independent(w[i], w[j])));
        let least = available.min_by(|&x, &y| w[x].cmp(&w[y]).then(x.cmp(&y))).expect("an available one");
        out.push(w[least]);
        pending.retain(|&p| p != least);
    }
    out
}

#[test]
fn distinct_grips_are_independent_exactly_as_the_bits_say_and_one_grip_depends_on_itself() {
    let r = relation();
    assert!(r.is_menu() && r.menu_independent([0, 1], [1, 1]) && r.menu_independent([0, 0], [599, 2]));
    assert!(r.menu_independent([3, 2], [2, 1]) && r.menu_independent([598, 1], [599, 1]));
    for (a, b) in [([0, 1], [2, 1]), ([2, 0], [0, 2]), ([1, 1], [3, 0]), ([599, 2], [2, 2])] {
        assert!(!r.menu_independent(a, b) && !r.menu_independent(b, a), "{a:?} and {b:?} interact, whatever the items");
    }
    for (a, b) in [([0, 1], [0, 2]), ([0, 0], [0, 1]), ([5, 0], [5, 0])] {
        assert!(!r.menu_independent(a, b), "letters at one grip are dependent, the identity item's too");
    }
    assert!(!r.menu_independent([600, 0], [1, 0]) && !r.menu_independent([0, 3], [1, 0]), "outside the alphabet");
    assert!(r.holds_menu([599, 2]) && !r.holds_menu([600, 0]) && !r.holds_menu([0, 3]));
    assert!(!r.holds(1) && r.key(&[1]).is_none() && !r.independent(1, 2), "no Full letter is of a jumbling relation");

    let h = r.header();
    assert_eq!((h.puzzle.as_str(), h.model_id.as_str(), h.generators), ("600-cell-jumbled", "model-1", 600));
    assert_eq!((h.key.as_str(), h.contract.as_str()), (TRACE_KEY, JUMBLE_LOCALITY));
    let menu =
        MenuHeader { menu_id: "menu-a".into(), menu_order: "order-1".into(), contract_rev: "rev-1".into(), items: 3 };
    assert_eq!(h.menu, Some(menu));
    assert_eq!(h.menu_identity(), Some(identity("menu-a")));
    assert_eq!(r.bits().len(), (600 * 599 / 2usize).div_ceil(8), "one bit per pair of distinct grips");
    assert!(r.keys_menu(&alphabet("menu-a")) && !r.keys_menu(&alphabet("menu-b")));
    let fewer_items = MenuAlphabet::new(identity("menu-a"), JUMBLE_GRIPS, vec![0]).unwrap();
    let fewer_caps = MenuAlphabet::new(identity("menu-a"), 599, vec![0, 2, 1]).unwrap();
    assert!(!r.keys_menu(&fewer_items) && !r.keys_menu(&fewer_caps));

    // The digest is the model's table: another menu of the model keeps it, and the header names the menu.
    let other_menu = relation_of(&identity("menu-b"), &INTERACTING);
    assert_eq!(other_menu.digest(), r.digest());
    assert_eq!(r.header().differing(other_menu.header()), Some("menu_id"));
    let mut other_model = identity("menu-a");
    other_model.model_id = "model-2".into();
    assert_eq!(r.header().differing(relation_of(&other_model, &INTERACTING).header()), Some("model_id"));
    let apart = relation_of(&identity("menu-a"), &INTERACTING[..2]);
    assert!(apart.menu_independent([2, 0], [599, 0]) && apart != r);
    assert_eq!(r.header().differing(apart.header()), Some("digest"));
    let mut later = identity("menu-a");
    later.contract_rev = "rev-2".into();
    assert_eq!(r.header().differing(relation_of(&later, &INTERACTING).header()), Some("contract_rev"));
    assert_eq!(r.header().differing(r.header()), None);
    let full =
        Relation::from_parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "model-1", FULL_GENERATORS, vec![0xff; 89925]);
    assert_eq!(full.expect("a relation").header().differing(r.header()), Some("puzzle"));
}

#[test]
fn a_jumbling_relation_needs_the_600_grips_a_menu_and_a_named_identity_block() {
    let parts = |identity: &MenuIdentity, grips: u32, items: u32, bits: Vec<u8>| {
        Relation::jumble_locality(identity, grips, items, bits).err()
    };
    let b = bits(&INTERACTING);
    assert_eq!(parts(&identity("m"), 599, 3, b.clone()), Some(RelationError::Grips(599)));
    assert_eq!(parts(&identity("m"), 1200, 3, b.clone()), Some(RelationError::Grips(1200)));
    assert_eq!(parts(&identity("m"), 600, 0, b.clone()), Some(RelationError::Items(0)));
    assert_eq!(parts(&identity("m"), 600, 601, b.clone()), Some(RelationError::Items(601)));
    assert!(parts(&identity("m"), 600, 600, b.clone()).is_none());
    assert_eq!(parts(&identity("m"), 600, 3, b[1..].to_vec()), Some(RelationError::Bits));
    let mut padded = b.clone();
    *padded.last_mut().unwrap() |= 0x10;
    assert_eq!(parts(&identity("m"), 600, 3, padded), Some(RelationError::Bits));
    for field in ["puzzle", "model_id", "menu_id", "menu_order", "contract_rev"] {
        let mut unnamed = identity("m");
        match field {
            "puzzle" => unnamed.puzzle.clear(),
            "model_id" => unnamed.model_id.clear(),
            "menu_id" => unnamed.menu_id.clear(),
            "menu_order" => unnamed.menu_order.clear(),
            _ => unnamed.contract_rev.clear(),
        }
        assert_eq!(parts(&unnamed, 600, 3, b.clone()), Some(RelationError::Identity(field)));
    }
    // The Full constructor takes no jumbling contract.
    let refused = Relation::from_parts(JUMBLE_LOCALITY, "600-cell-jumbled", "model-1", 600, b).err();
    assert_eq!(refused, Some(RelationError::Contract(JUMBLE_LOCALITY.into())));
    for refusal in [RelationError::Grips(599), RelationError::Items(0)] {
        assert!(!refusal.to_string().is_empty());
    }
}

const KEYS: &str = "export fn traces(x: MenuWord, y: MenuWord) -> Bool effects [] {\n    \
    return exact_eq(trace_key(x), trace_key(y));\n}\n\n\
    export fn kept(x: MenuWord, y: MenuWord) -> Bool effects [] {\n    \
    return contains(insert(set<TraceEqualityKey>(), trace_key(x)), trace_key(y));\n}\n\n\
    export fn literal() -> Bool effects [] {\n    return exact_eq(trace_key(menu[2:1, 0:1, 1:1]), \
    trace_key(menu[1:1, 2:1, 0:1]));\n}\n\n\
    export fn key(x: MenuWord) -> TraceEqualityKey effects [] {\n    return trace_key(x);\n}\n\n\
    export fn full(x: Word) -> Bool effects [] {\n    return exact_eq(trace_key(x), trace_key(x));\n}\n";

/// CT-LB1-06 (part: LB1.10's jumbling half, run through the interpreter on a synthetic relation; the model's relation
/// is regenerated in `c600-jumble`): with a < b < c at grips 0, 1 and 2, b independent of a and c, and a dependent on
/// c, bca and cab both key to bca, which keeps c before a, and abc keys to itself. Every occurrence survives: an item
/// and its inverse at one grip do not cancel, and the identity item stays. A key has no canonical form, so a run that
/// would emit one stops with `PS1:input.noncanonical_value`.
#[test]
fn ct_lb1_06_bca_and_cab_have_one_menu_trace_key_and_abc_another() {
    let r = relation();
    let (a, b, c) = ([0, 1], [1, 1], [2, 1]);
    let k = |w: &[[u32; 2]]| r.menu_key(w).expect("letters of the alphabet").menu_letters().to_vec();
    assert_eq!(k(&[b, c, a]), [b, c, a]);
    assert_eq!(k(&[c, a, b]), [b, c, a]);
    assert_eq!(k(&[a, b, c]), [a, b, c]);
    assert_eq!(k(&[[0, 1], [0, 2]]), [[0, 1], [0, 2]], "an item and its inverse at one grip stay");
    assert_eq!(k(&[[0, 2], [0, 1]]), [[0, 2], [0, 1]], "letters at one grip never swap");
    assert_eq!(k(&[[1, 0], [0, 0]]), [[0, 0], [1, 0]], "identity items at distinct independent grips swap");
    assert_eq!(k(&[[2, 0], [0, 0]]), [[2, 0], [0, 0]], "and at interacting grips do not");
    assert_eq!(k(&[]), Vec::<[u32; 2]>::new());
    assert_eq!(r.menu_key(&[[0, 3]]), None);
    assert_eq!(r.menu_key(&[[600, 0]]), None);

    let (_, loaded) = build(&[&m(KEYS)]);
    let alpha = alphabet("menu-a");
    let options = pinned(&alpha, &r);
    let ran = |entry: &str, x: &[[u32; 2]], y: &[[u32; 2]]| {
        loaded.run("m", entry, vec![menu(&alpha, x), menu(&alpha, y)], &mut NoHost, &options).unwrap()
    };
    for (x, y, traces) in [
        (&[b, c, a][..], &[c, a, b][..], "true\n"),
        (&[a, b, c], &[b, c, a], "false\n"),
        (&[a, b, c], &[a, b, c], "true\n"),
        (&[[0, 1], [0, 2]], &[], "false\n"),
        (&[[3, 1], [0, 2], [1, 1]], &[[0, 2], [1, 1], [3, 1]], "false\n"),
        (&[[4, 1], [0, 2], [1, 1]], &[[0, 2], [1, 1], [4, 1]], "true\n"),
    ] {
        assert_eq!(output(&ran("traces", x, y)), traces, "trace keys of {x:?} and {y:?}");
        assert_eq!(output(&ran("kept", x, y)), traces, "a set of trace keys holds {y:?}'s key after {x:?}'s");
    }
    assert_eq!(output(&loaded.run("m", "literal", vec![], &mut NoHost, &options).unwrap()), "true\n");
    stopped(&loaded.run("m", "key", vec![menu(&alpha, &[a])], &mut NoHost, &options).unwrap(), NONCANONICAL_VALUE);
}

/// The scan agrees with the definition of the least topological word on many menu words, and keys a word exactly as it
/// keys any word that adjacent swaps of independent letters reach.
#[test]
fn the_menu_key_is_the_least_topological_word_and_swaps_of_independent_letters_keep_it() {
    let r = relation();
    let grips = [0, 1, 2, 3, 4, 599];
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % bound as u64) as usize
    };
    for _ in 0..600 {
        let length = next(13);
        let w: Vec<[u32; 2]> = (0..length).map(|_| [grips[next(grips.len())], next(3) as u32]).collect();
        let k = r.menu_key(&w).expect("letters of the alphabet");
        assert_eq!(k.menu_letters(), naive(&r, &w), "{w:?}");
        assert!(k.letters().is_empty() && k.is_menu() && k.len() == w.len() && k.size() == 4 * w.len() as u128);
        let mut sorted = w.clone();
        sorted.sort_unstable();
        let mut kept = k.menu_letters().to_vec();
        kept.sort_unstable();
        assert_eq!(kept, sorted, "{w:?}: every occurrence is kept");
        if let Some(i) = (1..w.len()).find(|&i| r.menu_independent(w[i - 1], w[i])) {
            let mut swapped = w.clone();
            swapped.swap(i - 1, i);
            assert_eq!(r.menu_key(&swapped), Some(k.clone()), "{w:?} and {swapped:?}");
        }
        assert_eq!(r.menu_key(k.menu_letters()).as_ref().map(TraceKey::menu_letters), Some(k.menu_letters()));
    }
}

/// Menu keys under one header order by their letters, grip then item, a proper prefix first; a Full key and a menu key
/// differ in their headers first.
#[test]
fn menu_keys_order_by_grip_then_item() {
    let r = relation();
    let Value::Set(set) =
        Value::set(vec![key(&r, &[[1, 0]]), key(&r, &[[0, 2]]), key(&r, &[[0, 1], [1, 0]]), key(&r, &[])])
    else {
        panic!("a set")
    };
    let letters: Vec<_> = set
        .items()
        .iter()
        .map(|v| if let Value::TraceEqualityKey(k) = v { k.menu_letters().to_vec() } else { vec![] })
        .collect();
    assert_eq!(letters, [vec![], vec![[0, 1], [1, 0]], vec![[0, 2]], vec![[1, 0]]]);
    assert_eq!(key_order(&key(&r, &[[0, 1]]), &key(&r, &[[0, 1]])), Ordering::Equal);
    assert!(equal(&key(&r, &[[1, 0], [0, 0]]), &key(&r, &[[0, 0], [1, 0]])));
}

/// `trace_key` on a menu word is one step, priced before the key is built by what its scan reads
/// ([`Relation::menu_reads`]) at `expand_per_letter`, with four bytes reserved per letter. The run must pin a jumbling
/// relation that keys its menu alphabet; otherwise it stops with LB1's key contract code before the step, as a Full
/// word does under a jumbling relation.
#[test]
fn a_menu_trace_key_is_priced_by_its_scan_and_needs_a_relation_of_the_pinned_alphabet() {
    let r = relation();
    // 600 grips; per letter 1, the heap's 2 x 10 steps, and two reads of each grip its own depends on: grip 0 depends
    // on 2, grip 1 on 3, and grip 2 on 0 and 599.
    let w = [[0, 1], [1, 1], [2, 1]];
    assert_eq!(r.menu_reads(&w), 600 + (1 + 20 + 2) + (1 + 20 + 2) + (1 + 20 + 4));
    assert_eq!(r.menu_reads(&[]), 600);

    let (_, loaded) = build(&[&m(KEYS)]);
    let alpha = alphabet("menu-a");
    let options = pinned(&alpha, &r);
    let args = || vec![menu(&alpha, &w), menu(&alpha, &w)];
    let ran = loaded.run("m", "traces", args(), &mut NoHost, &options).unwrap();
    completed(&ran);
    let per_letter = options.tariffs.expand_per_letter;
    let rows: Vec<_> = ran
        .summary
        .rows
        .iter()
        .filter(|row| row.at.ends_with("#trace_key"))
        .map(|row| (row.steps, row.kernel_cost, row.memory))
        .collect();
    assert_eq!(rows, [(1, per_letter * r.menu_reads(&w), 12); 2]);

    let other_menu = relation_of(&identity("menu-b"), &INTERACTING);
    let full =
        Relation::from_parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "model-1", FULL_GENERATORS, vec![0xff; 89925])
            .expect("a relation");
    let unpinned = Options { menu: Some(&alpha), ..Options::new(WIDE) };
    for options in [unpinned, pinned(&alpha, &other_menu), pinned(&alpha, &full)] {
        let stop = loaded.run("m", "traces", args(), &mut NoHost, &options).unwrap();
        stopped(&stop, KEY_CONTRACT_MISMATCH);
        assert!(stop.value.is_none());
    }
    // A Full word's key needs a Full relation.
    let full_word = loaded.run("m", "full", vec![Value::word(&[1])], &mut NoHost, &options).unwrap();
    stopped(&full_word, KEY_CONTRACT_MISMATCH);
    // A menu word of another alphabet never reaches the step: the run refuses the argument.
    let other = alphabet("menu-b");
    let refused = loaded.run("m", "traces", vec![menu(&other, &w), menu(&alpha, &w)], &mut NoHost, &options);
    assert_eq!(refused.err().map(|e| e.code), Some(INCOMPATIBLE_ALPHABET));
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

const READ: &str = "export fn agrees(w: MenuWord) -> Bool effects [snapshot] {\n    \
    return exact_eq(read_snapshot<TraceEqualityKey>(\"best\"), trace_key(w));\n}\n";

/// A host's answer may hold a menu trace key only under the run's jumbling relation, as the key of its own trace. An
/// answer under another menu, model or table, a Full key under the menu header, a menu key under a Full header, or
/// letters out of key order stop the run after the read's row.
#[test]
fn a_host_answer_holds_menu_trace_keys_only_under_the_pinned_relation() {
    let (_, loaded) = build(&[&m(READ)]);
    let r = relation();
    let alpha = alphabet("menu-a");
    let run = |answer: Value| {
        let w = menu(&alpha, &[[2, 1], [0, 1], [1, 1]]);
        loaded.run("m", "agrees", vec![w], &mut Answering(answer), &pinned(&alpha, &r)).unwrap()
    };
    assert_eq!(output(&run(key(&r, &[[1, 1], [2, 1], [0, 1]]))), "true\n");

    let foreign = |h: TraceHeader, letters: &[[u32; 2]]| {
        Value::TraceEqualityKey(Arc::new(TraceKey::from_menu_parts(Arc::new(h), letters)))
    };
    let edited = |edit: fn(&mut TraceHeader)| {
        let mut h = r.header().clone();
        edit(&mut h);
        h
    };
    let mut other_model = identity("menu-a");
    other_model.model_id = "model-2".into();
    let letters = [[1, 1], [2, 1], [0, 1]];
    for (answer, field) in [
        (key(&relation_of(&identity("menu-b"), &INTERACTING), &letters), "menu_id"),
        (key(&relation_of(&other_model, &INTERACTING), &letters), "model_id"),
        (key(&relation_of(&identity("menu-a"), &INTERACTING[1..]), &letters), "digest"),
        (foreign(edited(|h| h.menu.as_mut().unwrap().items = 4), &letters), "items"),
        (foreign(edited(|h| h.menu = None), &letters), "menu_id"),
        (foreign(edited(|h| h.contract = "jumble-locality/2".into()), &letters), "contract"),
    ] {
        let why = check_keys(&answer, Some(&r)).unwrap_err();
        assert!(why.contains(field), "{field}: {why}");
        let stop = run(answer);
        stopped(&stop, KEY_CONTRACT_MISMATCH);
        assert!(stop.summary.rows.iter().any(|row| row.at.ends_with("#kernel")), "{:?}", stop.summary.rows);
    }
    // Full letters under the menu header, and letters out of key order: 0 and 1 are independent, so 1 0 keys to 0 1.
    let full_letters = Value::TraceEqualityKey(Arc::new(TraceKey::from_parts(Arc::new(r.header().clone()), &[1])));
    assert!(check_keys(&full_letters, Some(&r)).unwrap_err().contains("alphabet"));
    stopped(&run(full_letters), KEY_CONTRACT_MISMATCH);
    let unordered = foreign(r.header().clone(), &[[1, 0], [0, 0]]);
    assert!(check_keys(&unordered, Some(&r)).unwrap_err().contains("not the key"));
    stopped(&run(unordered), KEY_CONTRACT_MISMATCH);
    stopped(&run(foreign(r.header().clone(), &[[0, 3]])), KEY_CONTRACT_MISMATCH);
}
