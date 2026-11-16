//! CT-LB1-03, the jumbling kernel half (LB1.8, LB1.9; chapter 03's test table): inverse, conjugate, commutator and
//! powers construct literal words without state access, application checks each prefix, and an inverse returns an
//! admitted path to its start. The constructors are `c600_jumble_tool::word`'s, over the letters of one pinned menu;
//! the prefixes are checked by `c600_jumble`'s prefix observations (JI.10), which apply each literal letter in turn.
//!
//! What runs: the constructors on words of the pinned I_a menu, with no state in existence while they run; the alphabet
//! refusal `LB1:input.incompatible_alphabet` for the S4 menu, for I_a's rotations in another pinned order and for free
//! mode; H6030's first letters followed by their inverse from solved, every prefix admitted and the path retraced to
//! its start; and a conjugate of an admitted letter refused at its first prefix. Not run: the LB1 library entries as
//! language code, which wait for collection types and word operations in the IR draft (LP2); the 600-cell-Full half
//! (signed generators); and the word and trace keys of LB1.9 and LB1.10. A pass is source and fixture evidence.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::observe::{prefix_observations, Batch, Fields, PrefixRequest};
use c600_jumble::q5::{identity4, matmul};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context;
use c600_jumble_tool::word::{self, Alphabet, Letter, LiteralWord, WordError, INCOMPATIBLE_ALPHABET};
use c600_kernel::json::{self, Value};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn read(path: &str) -> Value {
    let bytes = std::fs::read(repository().join(path)).unwrap_or_else(|error| panic!("{path}: {error}"));
    json::parse(&bytes).unwrap_or_else(|_| panic!("{path} is JSON"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or_else(|| panic!("no field `{key}`"))
}

/// A W-J fixture document (`research/jumbling/fixtures/wj-{name}.json`).
fn fixture(name: &str) -> Value {
    read(&format!("research/jumbling/fixtures/wj-{name}.json"))
}

fn menu(record: &Value) -> Arc<Menu> {
    Arc::new(Menu::from_record(ctx(), record).expect("a pinned menu"))
}

/// The pinned I_a menu with its items in reverse order: the same rotations, another pinned order.
fn reversed_i_a() -> Arc<Menu> {
    let mut record = field(&fixture("I_a"), "menu").clone();
    let Value::Object(fields) = &mut record else { panic!("a menu record is a mapping") };
    let Some(Value::Array(base)) = fields.get_mut("base") else { panic!("a menu record has a base") };
    base.reverse();
    menu(&record)
}

/// The state after the first `count` records of the I_a journal from solved.
fn after(i_a: &Arc<Menu>, count: usize) -> State {
    let doc = fixture("I_a");
    let records = field(field(&doc, "journal"), "records").as_array().expect("records");
    let mut state = State::new(Some(i_a.clone()));
    for (index, record) in records[..count].iter().enumerate() {
        let twist = Twist::from_record(ctx(), record).expect("a journal record");
        let outcome = state.apply(ctx(), &twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "record {index}");
    }
    state
}

/// The first `count` letters of H6030, a word of the pinned I_a menu from solved.
fn h6030(count: usize) -> Vec<[i32; 2]> {
    let doc = read("research/jumbling/restoration/10-x1-unbounded/witnesses/H6030.json");
    assert_eq!(field(&doc, "from").as_str(), Some("solved"));
    let word = field(&doc, "word").as_array().expect("a word");
    word[..count]
        .iter()
        .map(|pair| {
            let pair = pair.as_array().expect("a letter");
            [pair[0].as_i64().unwrap() as i32, pair[1].as_i64().unwrap() as i32]
        })
        .collect()
}

/// JI.10's prefix observations of the letters a word plays at `start`, which is not changed.
fn observed(start: &State, word: &LiteralWord) -> Value {
    let letters: Vec<Letter> = word.letters(ctx(), start).expect("the state's alphabet");
    let request = PrefixRequest {
        letters: &letters,
        fields: Fields { n: false, heights: false, fd: false, sets: false },
        cap: None,
        batch: Batch { chunk: 16, budget: None, cancel_after: None },
        keep: false,
        eval_budget: None,
    };
    prefix_observations(ctx(), start, &request).expect("prefix observations").0
}

fn rows(observations: &Value) -> &[Value] {
    field(observations, "rows").as_array().expect("rows")
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    field(value, key).as_str().unwrap_or_else(|| panic!("`{key}` is no string"))
}

/// The first item of the menu that is not the identity rotation.
fn rotation(menu: &Menu) -> i32 {
    menu.items.iter().position(|(_, m)| *m != identity4()).expect("a rotation") as i32
}

/// CT-LB1-03 (part: "inverse, conjugate, commutator and powers construct words without state access"; the jumbling
/// kernel half). Every word here is built before any state exists, from the letters alone: the constructors take a
/// word and nothing else. The inverse reverses the letters and gives each the item of the inverse rotation, exactly
/// (the two matrices multiply to the identity); the conjugate is setup, word, setup⁻¹; the commutator is u, w, u⁻¹,
/// w⁻¹; the power n repeats the word, 0 gives the empty word and a negative power repeats the inverse. Words of two
/// alphabets do not combine: the S4 menu differs in its menu identity, and I_a's rotations in another pinned order
/// differ in the menu-order digest, so item indices would mean other rotations; each is
/// `LB1:input.incompatible_alphabet`. The same menu loaded twice is one alphabet. Playing a word reads the state's
/// alphabet: a state of another menu, of another order or in free mode refuses it with the same code.
#[test]
fn ct_lb1_03_word_operations_construct_words_from_letters_alone() {
    let ctx = ctx();
    let i_a = menu(field(&fixture("I_a"), "menu"));
    let alphabet = Alphabet::of(ctx, &i_a).expect("an inverse-closed menu");
    let inv = |item: i32| alphabet.inverse_item(item).expect("an item of the menu");
    let (a, b) = (rotation(&i_a), rotation(&i_a) + 1);
    let w = alphabet.word(&[[0, a], [57, b]]).expect("two letters");
    let u = alphabet.word(&[[3, a]]).expect("one letter");

    for item in 0..alphabet.items() as i32 {
        let product = matmul(&i_a.items[inv(item) as usize].1, &i_a.items[item as usize].1);
        assert_eq!(product, identity4(), "item {item} and its inverse item compose to the identity");
    }
    assert_eq!(w.inverse().pairs(), [[57, inv(b)], [0, inv(a)]]);
    assert_eq!(w.inverse().inverse(), w);
    assert_eq!(w.conjugate(&u).expect("one alphabet").pairs(), [[3, a], [0, a], [57, b], [3, inv(a)]]);
    assert_eq!(
        u.commutator(&w).expect("one alphabet").pairs(),
        [[3, a], [0, a], [57, b], [3, inv(a)], [57, inv(b)], [0, inv(a)]]
    );
    assert_eq!(w.power(3).pairs(), [[0, a], [57, b], [0, a], [57, b], [0, a], [57, b]]);
    assert_eq!(w.power(1), w);
    assert!(w.power(0).is_empty() && w.power(0).alphabet() == &alphabet, "the empty word keeps its alphabet");
    assert_eq!(w.power(-2), w.inverse().concat(&w.inverse()).expect("one alphabet"));
    assert_eq!(w.concat(&w.inverse()).expect("one alphabet").len(), 4, "construction cancels nothing");

    // Letters are shape-checked only.
    assert_eq!(alphabet.word(&[[600, a]]).err(), Some(WordError::Cap { index: 0, cap: 600 }));
    assert_eq!(alphabet.word(&[[0, a], [0, 60]]).err(), Some(WordError::Item { index: 1, item: 60, items: 60 }));
    assert_eq!(WordError::Cap { index: 0, cap: 600 }.code(), None, "a shape error has no LB1 code");

    // Two alphabets do not combine.
    let s4 = menu(field(&fixture("S4"), "menu"));
    let other = Alphabet::of(ctx, &s4).expect("an inverse-closed menu").word(&[[0, 0]]).expect("one letter");
    let reversed = reversed_i_a();
    let turned = Alphabet::of(ctx, &reversed).expect("an inverse-closed menu");
    assert_eq!(turned.identity().menu_id, alphabet.identity().menu_id, "the same rotations");
    let turned = turned.word(&[[0, a]]).expect("one letter");
    for (refused, field) in [
        (w.concat(&other), "menu_id"),
        (other.concat(&w), "menu_id"),
        (w.conjugate(&other), "menu_id"),
        (w.commutator(&other), "menu_id"),
        (w.concat(&turned), "menu_order"),
        (turned.conjugate(&w), "menu_order"),
    ] {
        let error = refused.expect_err("two alphabets");
        assert_eq!(error, WordError::Alphabet { field });
        assert_eq!(error.code(), Some(INCOMPATIBLE_ALPHABET));
    }
    assert_eq!(INCOMPATIBLE_ALPHABET, "LB1:input.incompatible_alphabet");
    let again = Alphabet::of(ctx, &menu(field(&fixture("I_a"), "menu"))).expect("the same menu");
    assert_eq!(again, alphabet);
    assert!(w.concat(&again.word(&[[0, a]]).expect("one letter")).is_ok(), "one menu, loaded twice, is one alphabet");

    // Playing a word reads the state's alphabet; the letters are the literal ones.
    let state = State::new(Some(i_a.clone()));
    let letters = w.letters(ctx, &state).expect("the word's own alphabet");
    let read = word::letters(ctx, &i_a, &w.word()).expect("the same letters");
    assert_eq!(letters.iter().map(|l| l.0).collect::<Vec<_>>(), [[0, i64::from(a)], [57, i64::from(b)]]);
    for (played, named) in letters.iter().zip(&read) {
        assert_eq!((played.0, played.1.grip, &played.1.matrix), (named.0, named.1.grip, &named.1.matrix));
    }
    for (state, field) in
        [(State::new(Some(s4)), "menu_id"), (State::new(Some(reversed)), "menu_order"), (State::new(None), "menu_id")]
    {
        let error = w.letters(ctx, &state).expect_err("another alphabet");
        assert_eq!((error.code(), error), (Some(INCOMPATIBLE_ALPHABET), WordError::Alphabet { field }));
    }
}

/// CT-LB1-03 (part: "application checks each prefix" and "an inverse returns an admitted path to its start"; the
/// jumbling kernel half). The word w of H6030's first 20 letters and the word w·w⁻¹ are built from letters alone. From
/// solved, the prefix observations apply each of the 40 literal letters in turn: every prefix is admitted, the first
/// half leaves the start, and after w and k letters of w⁻¹ the state is the one after w's first 20 − k letters, so the
/// inverse retraces the path, prefix by prefix, to the start digest. Not run: LB1.9's word keys.
#[test]
fn ct_lb1_03_an_inverse_returns_an_admitted_path_to_its_start() {
    let ctx = ctx();
    let i_a = menu(field(&fixture("I_a"), "menu"));
    let alphabet = Alphabet::of(ctx, &i_a).expect("an inverse-closed menu");
    let w = alphabet.word(&h6030(20)).expect("letters of the I_a menu");
    let there_and_back = w.concat(&w.inverse()).expect("one alphabet");
    assert_eq!(there_and_back.len(), 40);

    let start = State::new(Some(i_a));
    let digest = start.digest(ctx);
    let observations = observed(&start, &there_and_back);
    assert_eq!(observations.get("complete"), Some(&Value::Bool(true)));
    assert_eq!(observations.get("first_refused"), Some(&Value::Null));
    let rows = rows(&observations);
    assert_eq!(rows.len(), 40, "every prefix is observed");
    let mut before = digest.as_str();
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(text(row, "status"), "admissible", "prefix {}", index + 1);
        assert_eq!(text(row, "before"), before, "letter {index} acts on the previous prefix's state");
        before = text(row, "after");
    }
    let path: Vec<&str> = rows.iter().map(|row| text(row, "after")).collect();
    assert_ne!(path[19], digest, "w leaves the start");
    for k in 1..=20 {
        let expected = if k == 20 { digest.as_str() } else { path[19 - k] };
        assert_eq!(path[19 + k], expected, "after w and {k} letters of its inverse");
    }
    assert_eq!(text(field(&observations, "final"), "end_digest"), digest, "the inverse returns to the start");
}

/// CT-LB1-03 (part: a constructed word carries no admissibility guarantee, and application checks each prefix; LB1.9's
/// "a constructed conjugate gets no admissibility guarantee"). At the state after 12 records of the I_a journal, cap
/// `open` admits the letter (open, a) and cap `blocked` refuses (blocked, a). The conjugate of the admitted letter by
/// the refused one is constructed like any word, and playing it is refused at its first prefix, with a straddling
/// certificate at the refused cap and the state unchanged; the unplayed suffix stays unchecked. A power of the refused
/// letter and the refused letter followed by its inverse (LB1.9's example) are refused the same way; the admitted
/// letter, its inverse and then the refused letter are refused at the third prefix, after a return to the start; and
/// the zeroth power, the empty word, is admitted and ends at the start.
#[test]
fn ct_lb1_03_a_constructed_conjugate_gets_no_admissibility_guarantee() {
    let ctx = ctx();
    let i_a = menu(field(&fixture("I_a"), "menu"));
    let alphabet = Alphabet::of(ctx, &i_a).expect("an inverse-closed menu");
    let state = after(&i_a, 12);
    let start = state.digest(ctx);
    let grips: Vec<usize> = (0..12).collect();
    let statuses = state.clone().survey(ctx, &grips, None).expect("a survey");
    let blocked = grips.iter().copied().find(|&g| statuses[g] == Status::Blocked).expect("a blocked cap") as i32;
    let open = grips.iter().copied().find(|&g| statuses[g] == Status::Admissible).expect("an open cap") as i32;
    let a = rotation(&i_a);

    let admitted = alphabet.word(&[[open, a]]).expect("one letter");
    let setup = alphabet.word(&[[blocked, a]]).expect("one letter");
    let conjugate = admitted.conjugate(&setup).expect("one alphabet");
    assert_eq!(conjugate.pairs(), [[blocked, a], [open, a], [blocked, alphabet.inverse_item(a).unwrap()]]);

    let observations = observed(&state, &admitted);
    assert_eq!(observations.get("complete"), Some(&Value::Bool(true)), "the letter is admitted");
    let and_back = setup.concat(&setup.inverse()).expect("one alphabet");
    for word in [&conjugate, &setup.power(2), &and_back] {
        let observations = observed(&state, word);
        assert_eq!(observations.get("complete"), Some(&Value::Bool(false)));
        assert_eq!(observations.get("first_refused"), Some(&Value::Int(0)));
        assert_eq!(observations.get("final"), Some(&Value::Null), "no end state for a refused prefix");
        let rows = rows(&observations);
        assert_eq!(rows.len(), 1, "the batch stops at the refused prefix");
        assert_eq!((text(&rows[0], "status"), text(&rows[0], "after")), ("blocked", start.as_str()));
        let certificate = rows[0].get("certificate").expect("a blocked letter carries its straddling certificate");
        assert_eq!(certificate.get("grip"), Some(&Value::Int(blocked as i64)), "the straddle is at the refused cap");
        assert_eq!(certificate.get("points_checked_against_all_constraints"), Some(&Value::Bool(true)));
    }
    // A refusal at a later prefix: the admitted letter and its inverse return to the start, and the refused letter
    // after them is refused at the third prefix, with the start unchanged.
    let late = admitted.concat(&admitted.inverse()).and_then(|w| w.concat(&setup)).expect("one alphabet");
    let observations = observed(&state, &late);
    assert_eq!(observations.get("complete"), Some(&Value::Bool(false)));
    assert_eq!(observations.get("first_refused"), Some(&Value::Int(2)));
    let rows = rows(&observations);
    assert_eq!(rows.len(), 3, "each prefix up to the refused one is checked");
    let statuses: Vec<&str> = rows.iter().map(|row| text(row, "status")).collect();
    assert_eq!(statuses, ["admissible", "admissible", "blocked"]);
    assert_eq!((text(&rows[1], "after"), text(&rows[2], "after")), (start.as_str(), start.as_str()));
    let empty = setup.power(0);
    let observations = observed(&state, &empty);
    assert_eq!(observations.get("complete"), Some(&Value::Bool(true)));
    assert_eq!(text(field(&observations, "final"), "end_digest"), start, "the empty word ends at the start");
}
