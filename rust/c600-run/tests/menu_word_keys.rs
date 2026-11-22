//! LG2.18's word key of a menu word at run time (LP2 draft): one token per maximal run of adjacent letters at one grip,
//! the grip and the exact product of the run's items in chronological order, under the item matrices of the run's
//! pinned menu. `c600-run` does not read the model, so these tests pin a synthetic menu of seven items: the identity,
//! a rotation by the angle whose cosine is 3/5, a rotation whose entries need √5, and a quarter turn, each with its
//! inverse. The digests of the identity block are the ones the matrices give.

mod conformance;

use std::cmp::Ordering;
use std::sync::Arc;

use num_bigint::BigInt;

use c600_run::interp::code::{INCOMPATIBLE_ALPHABET, NONCANONICAL_VALUE};
use c600_run::interp::{Host, NoHost, Options, Request, RowTariff, Run, Sink};
use c600_run::menu::{menu_digests, ItemMatrix, MenuAlphabet, MenuError, MenuIdentity};
use c600_run::menu_key::{self, check_keys, MenuWordKey, Token, KEY_CONTRACT_MISMATCH, WORD_KEY};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome;
use c600_run::value::{add, equal, key_order, mul, Quad, Value};
use conformance::*;

type Triple = (i64, i64, i64);
const O: Triple = (0, 0, 1);
const L: Triple = (1, 0, 1);

fn q(a: i64, b: i64, d: i64) -> Quad {
    Quad::new(BigInt::from(a), BigInt::from(b), BigInt::from(d))
}

fn copy(x: &Quad) -> Quad {
    Quad::new(x.a.clone(), x.b.clone(), x.d.clone())
}

fn matrix(rows: [[Triple; 4]; 4]) -> ItemMatrix {
    rows.map(|row| row.map(|(a, b, d)| q(a, b, d)))
}

fn identity_matrix() -> ItemMatrix {
    matrix([[L, O, O, O], [O, L, O, O], [O, O, L, O], [O, O, O, L]])
}

/// A rotation in the plane of axes 0 and 1 by the angle whose cosine is 3/5, of infinite order.
fn a() -> ItemMatrix {
    matrix([[(3, 0, 5), (-4, 0, 5), O, O], [(4, 0, 5), (3, 0, 5), O, O], [O, O, L, O], [O, O, O, L]])
}

/// A rotation in the plane of axes 1 and 2 by the angle whose cosine is √5/5, of infinite order; it does not commute
/// with [`a`].
fn b() -> ItemMatrix {
    matrix([[L, O, O, O], [O, (0, 1, 5), (0, -2, 5), O], [O, (0, 2, 5), (0, 1, 5), O], [O, O, O, L]])
}

/// A quarter turn in the plane of axes 2 and 3.
fn quarter() -> ItemMatrix {
    matrix([[L, O, O, O], [O, L, O, O], [O, O, O, (-1, 0, 1)], [O, O, L, O]])
}

fn transpose(m: &ItemMatrix) -> ItemMatrix {
    std::array::from_fn(|i| std::array::from_fn(|j| copy(&m[j][i])))
}

/// The exact product `x · y`, entry by entry with the interpreter's Q(√5) arithmetic.
fn product(x: &ItemMatrix, y: &ItemMatrix) -> ItemMatrix {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let sum = (0..4).fold(Value::int(0), |s, k| {
                let term = mul(&Value::Q5(Arc::new(copy(&x[i][k]))), &Value::Q5(Arc::new(copy(&y[k][j])))).unwrap();
                add(&s, &term).unwrap()
            });
            let Value::Q5(entry) = sum else { panic!("a Q(√5) entry: {sum:?}") };
            copy(&entry)
        })
    })
}

/// Items 0 to 6: the identity, a and its inverse, b and its inverse, the quarter turn and its inverse.
fn items() -> Vec<ItemMatrix> {
    vec![identity_matrix(), a(), transpose(&a()), b(), transpose(&b()), quarter(), transpose(&quarter())]
}

const INVERSE: [u32; 7] = [0, 2, 1, 4, 3, 6, 5];

fn block(items: &[ItemMatrix]) -> MenuIdentity {
    let (menu_id, menu_order) = menu_digests(items);
    MenuIdentity {
        puzzle: "600-cell-jumbled".into(),
        model_id: "model-1".into(),
        menu_id,
        menu_order,
        contract_rev: "rev-1".into(),
    }
}

/// The 600 grips and the seven items, with their matrices.
fn alphabet() -> MenuAlphabet {
    let items = items();
    let bare = MenuAlphabet::new(block(&items), 600, INVERSE.to_vec()).expect("an alphabet");
    bare.with_matrices(items).expect("the menu's matrices")
}

/// The same alphabet without its matrices.
fn bare() -> MenuAlphabet {
    MenuAlphabet::new(block(&items()), 600, INVERSE.to_vec()).expect("an alphabet")
}

fn key_of(alpha: &MenuAlphabet, w: &[[u32; 2]]) -> MenuWordKey {
    menu_key::key(alpha, w).expect("letters of the alphabet")
}

fn key(alpha: &MenuAlphabet, w: &[[u32; 2]]) -> Value {
    Value::MenuWordKey(Arc::new(key_of(alpha, w)))
}

fn menu(alpha: &MenuAlphabet, letters: &[[u32; 2]]) -> Value {
    Value::menu_word(alpha.shared_identity(), letters).expect("indices below 600")
}

fn pinned(alpha: &MenuAlphabet) -> Options<'_> {
    Options { menu: Some(alpha), ..Options::new(WIDE) }
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

/// CT-LB1-02 (part: LG2.18's jumbling half, on a synthetic menu; not run here: the key and kernel halves on M5's
/// states, in `c600-jumble-tool/tests/lb3.rs`, and the 600-cell-Full half, in `c600-tools/tests/ct_lb1_02.rs`): a
/// maximal run of adjacent letters at one grip is one token, the grip and the exact product of the run's items in
/// chronological order, so `[c, x] [c, y]` holds `M_y M_x`. A token is kept when its product is the identity, a product
/// outside the menu stays an exact element, and runs at distinct grips are neither swapped nor merged.
#[test]
fn ct_lb1_02_a_run_at_one_grip_is_one_token_of_its_exact_product_in_chronological_order() {
    let alpha = alphabet();
    let k = |w: &[[u32; 2]]| key_of(&alpha, w);
    let ba = product(&b(), &a());
    assert_ne!(ba, product(&a(), &b()), "a and b do not commute");
    let first = k(&[[4, 1], [4, 3]]);
    assert_eq!(first.tokens(), [Token { grip: 4, product: product(&b(), &a()) }]);
    assert_ne!(first, k(&[[4, 3], [4, 1]]), "the run's order is kept");
    assert_eq!((first.identity(), first.grips(), first.items()), (alpha.identity(), 600, 7));
    assert_eq!(WORD_KEY, "word-key/1");

    // An item and its inverse leave one identity token. It is not the empty key, and it keeps its grip.
    let undone = k(&[[4, 1], [4, 2]]);
    assert_eq!(undone.tokens(), [Token { grip: 4, product: identity_matrix() }]);
    assert_ne!(undone, k(&[]));
    assert_ne!(undone, k(&[[5, 1], [5, 2]]));
    assert_eq!(undone, k(&[[4, 0]]), "the identity item at grip 4 has the same token");
    // Exact products recognize equal elements: four quarter turns, or two quarter turns either way.
    assert_eq!(k(&[[7, 5]; 4]), k(&[[7, 0]]));
    assert_eq!(k(&[[7, 5], [7, 5]]), k(&[[7, 6], [7, 6]]));
    assert_eq!(k(&[[7, 5]; 3]), k(&[[7, 6]]));
    assert_ne!(k(&[[7, 5]; 3]), k(&[[7, 5]]));

    // Runs at distinct grips stay apart and in order: no commuting swap, and no merge across another grip.
    let three = k(&[[0, 1], [1, 1], [0, 2]]);
    assert_eq!(three.tokens().iter().map(|t| t.grip).collect::<Vec<_>>(), [0, 1, 0]);
    assert_ne!(three, k(&[[0, 1], [0, 2], [1, 1]]));
    assert_ne!(k(&[[0, 1], [1, 1]]), k(&[[1, 1], [0, 1]]));
    // A product outside the menu is kept as an exact element, never as an item.
    assert!(!items().contains(&ba));
    assert!(first.canonical() && three.canonical() && k(&[]).canonical() && k(&[]).tokens().is_empty());

    // Letters outside the alphabet, or an alphabet without matrices, give no key and no price.
    assert!(menu_key::key(&alpha, &[[600, 0]]).is_none() && menu_key::key(&alpha, &[[0, 7]]).is_none());
    assert!(menu_key::price(&alpha, &[[0, 7]], 1).is_none());
    assert!(menu_key::key(&bare(), &[[0, 1]]).is_none() && menu_key::price(&bare(), &[], 1).is_none());
    assert_eq!(bare().item_matrices(), None);
    assert_eq!(alpha.item_matrices(), Some(&items()[..]));
}

/// Keys under one header order token by token (LG2.23): by grip, then by the entries row-major, each entry's triple
/// (a, b, d) lexicographically as integers, with a proper prefix first.
#[test]
fn menu_word_keys_order_by_grip_then_entries_with_a_proper_prefix_first() {
    let alpha = alphabet();
    let v = |w: &[[u32; 2]]| key(&alpha, w);
    // At grip 0, b's first row is the identity's, and its entry (1, 1) is (0, 1, 5), before the identity's (1, 0, 1).
    // The quarter turn's rows 0 and 1 are the identity's, and its entry (2, 2) is (0, 0, 1). a's entry (0, 0) is
    // (3, 0, 5), after (1, 0, 1), and its entry (0, 1) is (-4, 0, 5), before its inverse's (4, 0, 5).
    let words: [&[[u32; 2]]; 8] =
        [&[[1, 0]], &[[0, 2]], &[[0, 1], [1, 0]], &[[0, 1]], &[[0, 0]], &[[0, 5]], &[[0, 3]], &[]];
    let Value::Set(set) = Value::set(words.iter().map(|w| v(w)).collect()) else { panic!("a set") };
    let sorted: [&[[u32; 2]]; 8] =
        [&[], &[[0, 3]], &[[0, 5]], &[[0, 0]], &[[0, 1]], &[[0, 1], [1, 0]], &[[0, 2]], &[[1, 0]]];
    let expected: Vec<Value> = sorted.iter().map(|w| v(w)).collect();
    assert_eq!(set.items().len(), expected.len());
    for (got, want) in set.items().iter().zip(&expected) {
        assert!(equal(got, want), "{got:?} where {want:?} is due");
    }
    assert_eq!(key_order(&v(&[[0, 1], [0, 2]]), &v(&[[0, 0]])), Ordering::Equal);
    assert_eq!(key_order(&v(&[[3, 0]]), &v(&[[2, 6]])), Ordering::Greater, "the grip first");
    assert!(equal(&v(&[[9, 5], [9, 5]]), &v(&[[9, 6], [9, 6]])));
}

const KEYS: &str = "export fn same(x: MenuWord, y: MenuWord) -> Bool effects [] {\n    \
    return exact_eq(word_key(x), word_key(y));\n}\n\n\
    export fn kept(x: MenuWord, y: MenuWord) -> Bool effects [] {\n    \
    return contains(insert(set<WordEqualityKey>(), word_key(x)), word_key(y));\n}\n\n\
    export fn literal() -> Bool effects [] {\n    return exact_eq(word_key(menu[4:3, 4:4]), word_key(menu[4:0]));\n}\n\n\
    export fn key(x: MenuWord) -> WordEqualityKey effects [] {\n    return word_key(x);\n}\n\n\
    export fn full(x: Word) -> Bool effects [] {\n    return exact_eq(word_key(x), word_key(x));\n}\n";

/// `word_key` on menu words through the interpreter: equal keys exactly for equal token sequences, in `exact_eq` and in
/// a set. A key has no canonical form, so a run that would emit one stops with `PS1:input.noncanonical_value`.
#[test]
fn a_run_keys_menu_words_under_its_menu_matrices() {
    let (_, loaded) = build(&[&m(KEYS)]);
    let alpha = alphabet();
    let options = pinned(&alpha);
    let ran = |entry: &str, x: &[[u32; 2]], y: &[[u32; 2]]| {
        loaded.run("m", entry, vec![menu(&alpha, x), menu(&alpha, y)], &mut NoHost, &options).unwrap()
    };
    let cases: [(&[[u32; 2]], &[[u32; 2]], &str); 6] = [
        (&[[4, 1], [4, 2]], &[[4, 0]], "true\n"),
        (&[[4, 1], [4, 2]], &[], "false\n"),
        (&[[7, 5]; 4], &[[7, 0]], "true\n"),
        (&[[4, 1], [4, 3]], &[[4, 3], [4, 1]], "false\n"),
        (&[[0, 1], [1, 1], [0, 2]], &[[0, 1], [0, 2], [1, 1]], "false\n"),
        (&[[0, 5], [0, 5], [2, 1]], &[[0, 6], [0, 6], [2, 1]], "true\n"),
    ];
    for (x, y, same) in cases {
        assert_eq!(output(&ran("same", x, y)), same, "keys of {x:?} and {y:?}");
        assert_eq!(output(&ran("kept", x, y)), same, "a set of keys holds {y:?}'s key after {x:?}'s");
    }
    assert_eq!(output(&loaded.run("m", "literal", vec![], &mut NoHost, &options).unwrap()), "true\n");
    let emitted = loaded.run("m", "key", vec![menu(&alpha, &[[0, 1]])], &mut NoHost, &options).unwrap();
    stopped(&emitted, NONCANONICAL_VALUE);
}

/// `word_key` on a menu word is one step, priced before the key is built: `expand_per_letter` per letter for the scan,
/// plus the draft price of its products ([`menu_key::price`]), with the price's bound of its tokens reserved. The bound
/// holds the built key, products of non-symmetries among them.
#[test]
fn a_menu_word_key_is_priced_before_it_is_built_and_its_reservation_holds_it() {
    let alpha = alphabet();
    let per_pair = Options::new(WIDE).draft.field_per_unit_pair;
    let w = [[3, 1], [3, 3], [3, 3], [5, 0]];
    let (work, reservation) = menu_key::price(&alpha, &w, per_pair).expect("a price");
    assert!(work > 0 && reservation >= key_of(&alpha, &w).size());
    assert_eq!(menu_key::price(&alpha, &[], per_pair), Some((0, 0)));

    let (_, loaded) = build(&[&m(KEYS)]);
    let options = pinned(&alpha);
    let ran = loaded.run("m", "same", vec![menu(&alpha, &w), menu(&alpha, &w)], &mut NoHost, &options).unwrap();
    completed(&ran);
    let scan = options.tariffs.expand_per_letter * w.len() as u128;
    let rows: Vec<_> = ran
        .summary
        .rows
        .iter()
        .filter(|row| row.at.ends_with("#word_key"))
        .map(|row| (row.steps, row.kernel_cost, row.memory))
        .collect();
    assert_eq!(rows, [(1, scan + work, reservation); 2]);

    // Long runs of non-symmetries grow their entries; the reservation still bounds every key.
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % bound as u64) as usize
    };
    for _ in 0..300 {
        let length = next(24);
        let w: Vec<[u32; 2]> = (0..length).map(|_| [[0, 1, 599][next(3)], next(7) as u32]).collect();
        let (_, reservation) = menu_key::price(&alpha, &w, per_pair).expect("a price");
        let built = key_of(&alpha, &w);
        assert!(built.size() <= reservation, "{w:?}: {} > {reservation}", built.size());
        assert!(built.canonical(), "{w:?}");
    }
    let long: Vec<[u32; 2]> = (0..60).map(|i| [8, [1, 3][i % 2]]).collect();
    let (_, reservation) = menu_key::price(&alpha, &long, per_pair).expect("a price");
    assert!(key_of(&alpha, &long).size() <= reservation);
}

/// A run keys the words of one menu: without the item matrices a menu word's key stops the run with LB1's key contract
/// code before the step, and a run that keys menu words keys no Full word.
#[test]
fn a_menu_word_key_needs_the_menu_matrices_and_a_run_keys_one_kind_of_word() {
    let (_, loaded) = build(&[&m(KEYS)]);
    let alpha = alphabet();
    let plain = bare();
    let args = |a: &MenuAlphabet| vec![menu(a, &[[0, 1]]), menu(a, &[[0, 1]])];
    let unpinned = loaded.run("m", "same", args(&plain), &mut NoHost, &pinned(&plain)).unwrap();
    stopped(&unpinned, KEY_CONTRACT_MISMATCH);
    assert!(unpinned.value.is_none());
    let full = |options: &Options| loaded.run("m", "full", vec![Value::word(&[1, -1, 2])], &mut NoHost, options);
    stopped(&full(&pinned(&alpha)).unwrap(), KEY_CONTRACT_MISMATCH);
    assert_eq!(output(&full(&pinned(&plain)).unwrap()), "true\n");
    assert_eq!(output(&full(&Options::new(WIDE)).unwrap()), "true\n");
    // A menu word of another alphabet never reaches the step: the run refuses the argument.
    let other_items = vec![identity_matrix()];
    let other = MenuAlphabet::new(block(&other_items), 600, vec![0]).unwrap().with_matrices(other_items).unwrap();
    let refused = loaded.run("m", "same", args(&other), &mut NoHost, &pinned(&alpha));
    assert_eq!(refused.err().map(|e| e.code), Some(INCOMPATIBLE_ALPHABET));
}

const ARGS: &str = "export fn same(k: WordEqualityKey) -> Bool effects [] {\n    return exact_eq(k, k);\n}\n\n\
    export fn count(l: List<WordEqualityKey>) -> Nat effects [] {\n    return length(l);\n}\n";

/// A menu word key has no canonical form either, so no argument holds one.
#[test]
fn an_argument_that_holds_a_menu_word_key_is_refused() {
    let (_, loaded) = build(&[&m(ARGS)]);
    let alpha = alphabet();
    let refused =
        |entry: &str, arg: Value| loaded.run("m", entry, vec![arg], &mut NoHost, &pinned(&alpha)).err().map(|e| e.code);
    assert_eq!(refused("same", key(&alpha, &[[0, 1]])), Some(NONCANONICAL_VALUE));
    assert_eq!(refused("count", Value::list(vec![key(&alpha, &[])])), Some(NONCANONICAL_VALUE));
    assert_eq!(refused("count", Value::list(vec![])), None);
}

/// Answers every literal read with the value it holds.
struct Answering(Value);

impl Host for Answering {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 1 << 20 })
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
    return exact_eq(read_snapshot<WordEqualityKey>(\"best\"), word_key(w));\n}\n";

/// A host's answer may hold a menu word key only under the run's alphabet, with its matrices, and in canonical form. A
/// key under another menu or alphabet size, tokens out of form, a Full word key in a run that keys menu words, or a
/// menu word key in a run without matrices stop the run after the read's row.
#[test]
fn a_host_answer_holds_menu_word_keys_only_under_the_pinned_menu() {
    let (_, loaded) = build(&[&m(READ)]);
    let alpha = alphabet();
    let w = [[4, 1], [4, 3], [2, 5]];
    let run = |answer: Value| {
        loaded.run("m", "agrees", vec![menu(&alpha, &w)], &mut Answering(answer), &pinned(&alpha)).unwrap()
    };
    assert_eq!(output(&run(key(&alpha, &w))), "true\n");
    assert_eq!(output(&run(key(&alpha, &[[4, 1]]))), "false\n");

    let parts = |identity: MenuIdentity, grips: u32, items: u32, tokens: Vec<Token>| {
        Value::MenuWordKey(Arc::new(MenuWordKey::from_parts(Arc::new(identity), grips, items, tokens)))
    };
    let token = |grip: u32, m: ItemMatrix| Token { grip, product: m };
    let mut other_model = alpha.identity().clone();
    other_model.model_id = "model-2".into();
    let other_items = vec![identity_matrix()];
    for (answer, field) in [
        (parts(block(&other_items), 600, 7, vec![token(4, a())]), "menu_id"),
        (parts(other_model, 600, 7, vec![token(4, a())]), "model_id"),
        (parts(alpha.identity().clone(), 599, 7, vec![token(4, a())]), "grips"),
        (parts(alpha.identity().clone(), 600, 8, vec![token(4, a())]), "items"),
        (parts(alpha.identity().clone(), 600, 7, vec![token(4, a()), token(4, b())]), "canonical"),
        (parts(alpha.identity().clone(), 600, 7, vec![token(600, a())]), "canonical"),
        (Value::WordEqualityKey(vec![1, 2].into()), "Full"),
    ] {
        let why = check_keys(&answer, Some(&alpha)).unwrap_err();
        assert!(why.contains(field), "{field}: {why}");
        let stop = run(answer);
        stopped(&stop, KEY_CONTRACT_MISMATCH);
        assert!(stop.summary.rows.iter().any(|row| row.at.ends_with("#kernel")), "{:?}", stop.summary.rows);
    }
    // An entry out of lowest terms, deep in a list.
    let mut halves = a();
    halves[0][0] = Quad { a: BigInt::from(6), b: BigInt::from(0), d: BigInt::from(10) };
    let nested = Value::list(vec![parts(alpha.identity().clone(), 600, 7, vec![token(4, halves)])]);
    assert!(check_keys(&nested, Some(&alpha)).unwrap_err().contains("canonical"));
    // Without matrices no menu word key is held, and a Full key is.
    let held = key(&alpha, &w);
    assert!(check_keys(&held, Some(&bare())).unwrap_err().contains("matrices"));
    assert!(check_keys(&held, None).is_err());
    assert!(check_keys(&Value::WordEqualityKey(vec![1].into()), Some(&bare())).is_ok());
    assert!(check_keys(&held, Some(&alpha)).is_ok());
}

/// The matrices are the menu's: one per item, each entry in lowest terms with a positive denominator, an inverse item's
/// matrix the item's transpose, and the identity block's `menu_id` and `menu_order` the digests the matrices give.
#[test]
fn an_alphabet_takes_only_the_matrices_its_identity_block_names() {
    let all = block(&items());
    let with = |identity: &MenuIdentity, inverse: Vec<u32>, matrices: Vec<ItemMatrix>| {
        MenuAlphabet::new(identity.clone(), 600, inverse).expect("an alphabet").with_matrices(matrices).err()
    };
    let mut short = items();
    short.pop();
    assert_eq!(with(&all, INVERSE.to_vec(), short), Some(MenuError::Matrices { items: 7, matrices: 6 }));
    let mut halves = items();
    halves[1][0][0] = Quad { a: BigInt::from(6), b: BigInt::from(0), d: BigInt::from(10) };
    assert_eq!(with(&all, INVERSE.to_vec(), halves), Some(MenuError::Entry { item: 1 }));
    let mut negative = items();
    negative[3][0][0] = Quad { a: BigInt::from(-1), b: BigInt::from(0), d: BigInt::from(-1) };
    assert_eq!(with(&all, INVERSE.to_vec(), negative), Some(MenuError::Entry { item: 3 }));
    // Each item its own inverse: a is not its own transpose.
    assert_eq!(with(&all, (0..7).collect(), items()), Some(MenuError::Transpose { item: 1 }));
    // Another menu's block, and the same items in another order (one identity, another order digest).
    assert_eq!(with(&block(&items()[..1]), INVERSE.to_vec(), items()), Some(MenuError::Digest("menu_id")));
    let mut swapped = items();
    swapped.swap(1, 2);
    assert_eq!(menu_digests(&swapped).0, all.menu_id);
    assert_ne!(menu_digests(&swapped).1, all.menu_order);
    assert_eq!(with(&all, INVERSE.to_vec(), swapped), Some(MenuError::Digest("menu_order")));
    for refusal in
        [MenuError::Matrices { items: 7, matrices: 6 }, MenuError::Entry { item: 1 }, MenuError::Digest("menu_id")]
    {
        assert!(!refusal.to_string().is_empty());
    }

    // The compact key of the identity matrix, as the menu record's digests read it.
    let compact = "[[[1,0,1],[0,0,1],[0,0,1],[0,0,1]],[[0,0,1],[1,0,1],[0,0,1],[0,0,1]],\
                   [[0,0,1],[0,0,1],[1,0,1],[0,0,1]],[[0,0,1],[0,0,1],[0,0,1],[1,0,1]]]";
    let digest = c600_digest::sha256_hex(format!("[{compact}]").as_bytes());
    assert_eq!(menu_digests(&[identity_matrix()]), (digest.clone(), digest));
    let (menu_id, menu_order) = menu_digests(&[a()]);
    assert_eq!(menu_id, c600_digest::sha256_hex(b"[[[[3,0,5],[-4,0,5],[0,0,1],[0,0,1]],[[4,0,5],[3,0,5],[0,0,1],[0,0,1]],[[0,0,1],[0,0,1],[1,0,1],[0,0,1]],[[0,0,1],[0,0,1],[0,0,1],[1,0,1]]]]"));
    assert_eq!(menu_id, menu_order);
}
