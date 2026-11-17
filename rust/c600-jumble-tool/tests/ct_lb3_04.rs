//! CT-LB3-04 (whole: "off-lattice ν is undefined; the pinned C5 convention round-trips. A planted nonzero fixed I_a
//! coordinate is a consistency error. A drafted correction set is checked without selecting one", at C8's level: on
//! KN2.4's `nu` and `correction_set_check` rows, which chapter 03 names for LB3's "ν and drafted correction-set check"
//! entry, as LP2 built them in `c600_jumble::nu` and `c600_jumble::corrections`).
//!
//! The LB3 library entry is language code, so, as in `tests/lb3.rs`, each test checks on the kernel API the facts the
//! entry would return; the rows are not called from a 600 Cell Script program here, since the IR draft has no F2 or F5
//! number kinds yet. Nothing here chooses, ranks or proposes a correction: every id and multiplicity below is fixed in
//! the test, as a solver names them, and the rows answer with ν's 45 values only.
//!
//! LP2's `rust/c600-jumble/tests/nu.rs` and `rust/c600-jumble/tests/corrections.rs`, with the adapter comparisons
//! `tests/test_jumbling_rust_nu.py` and `tests/test_jumbling_rust_corrections.py`, check ν's tables, its values against
//! the reference and the 36 rows; these tests do not repeat them. The states come from the pinned I_a and I_b menus
//! (`research/jumbling/fixtures/`), the catalogue's own words and stage 04's b4 loop word
//! (`research/jumbling/restoration/04-card5-lattice-invariants/words/b4-loop-word-6.json`). A pass is source and
//! fixture evidence.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::corrections;
use c600_jumble::levels::{self, Base, Derivation, Partial, ReachKind, Reachability, Snapshot, Verdict};
use c600_jumble::model::POLES;
use c600_jumble::nu::{self, GroupKind, NuVector};
use c600_jumble::q5::transpose;
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_jumble_tool::word;
use c600_kernel::exact::F5;
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectionMode;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The jumbling model, loaded once for the tests of this file.
fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn read(path: &str) -> Value {
    let bytes = std::fs::read(repository().join(path)).unwrap_or_else(|error| panic!("{path}: {error}"));
    json::parse(&bytes).unwrap_or_else(|_| panic!("{path} is JSON"))
}

/// The pinned menu of a W-J fixture.
fn menu(name: &str) -> Menu {
    let doc = read(&format!("research/jumbling/fixtures/wj-{name}.json"));
    Menu::from_record(ctx(), doc.get("menu").expect("a menu")).expect("the fixture's pinned menu")
}

/// The pinned I_a menu, the catalogue's.
fn i_a() -> Arc<Menu> {
    static I_A: OnceLock<Arc<Menu>> = OnceLock::new();
    I_A.get_or_init(|| Arc::new(menu("I_a"))).clone()
}

/// `journal` played from solved under `menu`, every letter admissible, with the claim its replay witnesses.
fn witnessed(menu: &Arc<Menu>, journal: &[Twist]) -> Snapshot {
    let ctx = ctx();
    let mut state = State::new(Some(menu.clone()));
    for (index, twist) in journal.iter().enumerate() {
        let outcome = state.apply(ctx, twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "letter {index}");
    }
    Snapshot::witnessed(ctx, state, journal, None).expect("no engine error").expect("the journal witnesses the state")
}

/// KN2.4's `nu` row on a base it does not refuse.
fn nu_row(base: Base<'_>) -> Partial<NuVector> {
    nu::nu(ctx(), base).expect("no engine error").expect("the row does not refuse")
}

/// KN2.4's `correction_set_check` row on a base and a drafted set it does not refuse.
fn check(base: Base<'_>, set: &[(u32, i64)]) -> Partial<NuVector> {
    corrections::correction_set_check(ctx(), base, set).expect("no engine error").expect("the check does not refuse")
}

fn total(value: Partial<NuVector>) -> NuVector {
    match value {
        Partial::Total(value) => value,
        other => panic!("ν is not total: {other:?}"),
    }
}

/// The literal word of correction `id` under `menu`.
fn correction(menu: &Menu, id: u32) -> Vec<Twist> {
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    catalogue.twists(ctx(), menu, id).expect("no engine error").expect("the word reads")
}

/// The ν row of correction `id`.
fn row(id: u32) -> NuVector {
    *corrections::catalogue().expect("the catalogue reads").lookup(id).expect("a shipped id").row()
}

/// The twists of `[cap, item]` letters of `menu`.
fn letters(menu: &Menu, pairs: &[[i32; 2]]) -> Vec<Twist> {
    let letters = word::letters(ctx(), menu, &word::word_of(pairs)).expect("letters of the menu");
    letters.into_iter().map(|(_, twist)| twist).collect()
}

/// The permutations of 0..n in lexicographic order.
fn permutations(n: u8) -> Vec<Vec<u8>> {
    let mut current: Vec<u8> = (0..n).collect();
    let mut out = vec![current.clone()];
    while let Some(i) = (1..current.len()).rev().find(|&i| current[i - 1] < current[i]) {
        let j = (i..current.len()).rev().find(|&j| current[j] > current[i - 1]).expect("a larger label");
        current.swap(i - 1, j);
        current[i..].reverse();
        out.push(current.clone());
    }
    out
}

/// x then y, as ν's tables compose: (x·y)[i] = y[x[i]].
fn compose(x: &[u8], y: &[u8]) -> Vec<u8> {
    x.iter().map(|&i| y[i as usize]).collect()
}

/// The first letter outside K⁺, in cap and item order, that is admissible at `state` and lifts pieces off the lattice,
/// with the inverse letter at the same cap: a fixed rule of the test.
fn outside_k_plus(menu: &Menu, state: &State) -> (Twist, Twist) {
    let ctx = ctx();
    for cap in 0..POLES {
        let twists = menu.for_cap(ctx, cap).expect("the menu at the cap");
        for (item, twist) in twists.iter().enumerate().filter(|(_, twist)| twist.kidx.is_none()) {
            let mut work = state.clone();
            let outcome = work.apply(ctx, twist, None, &[]).expect("no engine error");
            if outcome.status == Status::Admissible && work.off_lattice_count() > 0 {
                let inverse = transpose(&menu.items[item].1);
                let back = menu.items.iter().position(|(_, m)| *m == inverse).expect("the menu is inverse-closed");
                return (twist.clone(), twists[back].clone());
            }
        }
    }
    panic!("no admissible letter outside K⁺")
}

/// CT-LB3-04's first sentence, "off-lattice ν is undefined" (A5; chapter 03: never stale coordinates, zeros or a
/// partial ν). Along correction 2's literal word from solved under I_a, the ν row and the drafted check are undefined
/// with `off_lattice` at every prefix that has a piece off the lattice, never zeros, a partial vector or a prediction,
/// and total at every lattice prefix, where the check adds the drafted row. At the word's end ν is correction 2's row,
/// b34 = 1. One admissible letter outside K⁺ from there lifts pieces off the lattice, and ν is undefined again, not
/// the stale b34; the inverse letter at the same cap returns to the same configuration, where ν is the row again.
#[test]
fn ct_lb3_04_off_lattice_nu_is_undefined_never_stale_zero_or_partial() {
    let ctx = ctx();
    let menu = i_a();
    let word = correction(&menu, 2);
    let row2 = row(2);
    assert_eq!(row2.nonzero(), vec![("b34", 1)]);
    let undefined = Partial::Undefined(nu::OFF_LATTICE.to_owned());
    let mut off = 0;
    for length in 0..=word.len() {
        let snapshot = witnessed(&menu, &word[..length]);
        let value = nu_row(Base::Snapshot(&snapshot));
        let checks = [check(Base::Snapshot(&snapshot), &[]), check(Base::Snapshot(&snapshot), &[(2, 1)])];
        if snapshot.state().off_lattice_count() > 0 {
            off += 1;
            assert_eq!(value, undefined, "prefix {length}");
            assert_eq!(checks, [undefined.clone(), undefined.clone()], "prefix {length}");
        } else {
            let value = total(value);
            assert_eq!(checks, [Partial::Total(value), Partial::Total(value + row2)], "prefix {length}");
        }
    }
    assert!(off > 0, "the word passes off the lattice");

    let end = witnessed(&menu, &word);
    assert_eq!(total(nu_row(Base::Snapshot(&end))), row2);
    let (lift, back) = outside_k_plus(&menu, end.state());
    let mut journal = word.clone();
    journal.push(lift);
    let lifted = witnessed(&menu, &journal);
    assert!(lifted.state().off_lattice_count() > 0);
    assert_eq!(nu_row(Base::Snapshot(&lifted)), undefined, "not the stale b34");
    assert_eq!(check(Base::Snapshot(&lifted), &[(2, 1)]), undefined);
    assert_eq!(nu::value(ctx, lifted.state()).expect("no engine error"), undefined);
    journal.push(back);
    let returned = witnessed(&menu, &journal);
    assert_eq!(returned.state().digest(ctx), end.state().digest(ctx));
    assert_eq!(total(nu_row(Base::Snapshot(&returned))), row2);
}

/// CT-LB3-04's second sentence, "the pinned C5 convention round-trips" (A5; DT-J1 M13). In each C5 orbit's frame
/// group, r0 is the least element of order 5 among the permutations of the group's five slots, and the power r0^i has
/// class i. Correction 27, derived from solved under I_a, reaches a lattice configuration whose ν, read in r0, is its
/// row: C5_17 = 4 and nothing else. The value comes back unchanged through its 45 values, its JSON list and the label
/// checkpoint's evidence. The manual's rule "make correction 27 v times", with v that value, checks to 0. Read in
/// another generator r0^k of the group, every class, and so the value, is multiplied by k⁻¹ in F5, and the same rule
/// leaves C5_17 nonzero.
#[test]
fn ct_lb3_04_the_pinned_c5_convention_round_trips() {
    let ctx = ctx();
    let tables = nu::tables(ctx).expect("ν's tables build");
    let identity: Vec<u8> = (0..5).collect();
    for orbit in nu::C5_ORBITS {
        let group = tables.group(orbit).expect("a frame group");
        assert_eq!((group.kind(), group.slots(), group.order()), (GroupKind::C5, 5, 5), "orbit {orbit}");
        let elements: Vec<Vec<u8>> = permutations(5).into_iter().filter(|h| group.class(h).is_some()).collect();
        assert_eq!(elements.len(), 5, "orbit {orbit}");
        // In C5 every element but the identity has order 5, and the identity is the least permutation.
        let r0 = elements.iter().find(|h| **h != identity).expect("an element of order 5");
        assert_eq!(group.generator(), Some(&r0[..]), "orbit {orbit}");
        let mut power = identity.clone();
        for i in 0..5 {
            assert_eq!(group.class(&power), Some(i), "orbit {orbit}: r0^{i}");
            power = compose(&power, r0);
        }
        assert_eq!(power, identity, "orbit {orbit}: r0 has order 5");
    }

    let menu = i_a();
    let solved = Snapshot::solved(ctx, Some(menu.clone()));
    let block = ctx.identity(Some(&*menu));
    let word = correction(&menu, 27);
    let answer = levels::derive(ctx, Base::Snapshot(&solved), &block, &word, ProtectionMode::Net, &[])
        .expect("no engine error")
        .expect("the request is not refused");
    let Derivation::Derived(derived) = answer else { panic!("correction 27 is admissible from solved") };
    assert_eq!(derived.state().off_lattice_count(), 0);
    let value = total(nu_row(Base::Derived(&derived)));
    assert_eq!(value, row(27));
    assert_eq!(value.nonzero(), vec![("C5_17", 4)]);

    let values = value.values();
    assert_eq!(NuVector::from_values(&values), Some(value));
    assert_eq!(value.to_json(), Value::Array(values.iter().map(|&v| Value::Int(v)).collect()));
    let verdict =
        levels::label_checkpoint(ctx, derived.state(), derived.reachability(), Some(&values)).expect("no engine error");
    let Verdict::No(evidence) = &verdict else { panic!("a nonzero ν fails the label checkpoint: {verdict:?}") };
    let shown = evidence.get("nu").and_then(Value::as_array).expect("ν in the evidence");
    let shown: Vec<i64> = shown.iter().map(|v| v.as_i64().expect("an integer")).collect();
    assert_eq!(NuVector::from_values(&shown), Some(value));

    let v = i64::from(value.c5()[0].value());
    assert_eq!(total(check(Base::Derived(&derived), &[(27, v)])), NuVector::ZERO, "the manual's rule");
    let group = tables.group(17).expect("orbit 17's frame group");
    let mut readings = 0;
    for g in permutations(5) {
        let Some(k) = group.class(&g).filter(|&k| k != 0) else { continue };
        let read = F5::new(v) * F5::new(i64::from(k)).inv().expect("k is a unit");
        let left = total(check(Base::Derived(&derived), &[(27, i64::from(read.value()))]));
        assert_eq!(left.is_zero(), Some(&g[..]) == group.generator(), "generator {g:?}, class {k}");
        readings += 1;
    }
    assert_eq!(readings, 4);
}

/// A six-letter word of the I_b menu as `[cap, item]` letters, some of them not in I_a. It is the image of stage 04's
/// b4 loop word (`research/jumbling/restoration/04-card5-lattice-invariants/words/b4-loop-word-6.json`, I_a letters
/// played from solved) in the reflection M = I − 2rrᵀ/(r·r) with r = p₁₂ − p₁₃, the difference of two of pole 0's
/// nearest poles: M has determinant −1, fixes pole 0 and maps poles to poles, and maps a letter (grip g, matrix R) to
/// (M·g, M·R·Mᵀ). A one-off search over the reflections through pairs of the poles 1, 10, 12 and 13 found that each
/// image word lies in I_b, is admissible at every letter and ends on the lattice with b5 nonzero; this is the last.
/// The tests below check those facts again rather than trust this note.
const MIRROR: [[i32; 2]; 6] = [[0, 12], [43, 9], [0, 1], [46, 3], [0, 4], [0, 43]];

/// CT-LB3-04's third sentence, "a planted nonzero fixed I_a coordinate is a consistency error"
/// (`KN2:consistency.fixed_nu_nonzero`; A5, DT-J1 A17). Under I_a no reachable lattice configuration has one: stage
/// 04's b4 loop word, witnessed under I_a, sets the sign bits of its odd orbits and no fixed coordinate. The plant is
/// a menu that is not I_a but carries I_a's order-free identity: the I_b menu with I_a's identity written over its own
/// (a draft of this test). [`MIRROR`], witnessed under it, reaches a lattice configuration whose ν has b5 = 1, a fixed
/// coordinate. The ν row refuses the value with the consistency error, naming b5, and the drafted check passes the
/// refusal through; neither gives the value. The same configuration with an open claim, or witnessed under the true
/// I_b menu, binds nothing: the row gives the value, b5 included, and the check refuses I_b's menu. The label
/// checkpoint answers no with the value supplied.
#[test]
fn ct_lb3_04_a_planted_nonzero_fixed_i_a_coordinate_is_a_consistency_error() {
    let ctx = ctx();
    let i_a = i_a();

    // The control: under I_a, stage 04's word sets the sign bits of its odd orbits and no fixed coordinate.
    let doc = read("research/jumbling/restoration/04-card5-lattice-invariants/words/b4-loop-word-6.json");
    let pairs = doc.get("word").and_then(Value::as_array).expect("a word");
    let pairs: Vec<[i32; 2]> = pairs
        .iter()
        .map(|pair| {
            let pair = pair.as_array().expect("a letter");
            [pair[0].as_i64().expect("a cap") as i32, pair[1].as_i64().expect("an item") as i32]
        })
        .collect();
    let odd = doc.get("odd_orbits").and_then(Value::as_array).expect("odd orbits");
    let odd: Vec<String> = odd.iter().map(|o| format!("b{}", o.as_i64().expect("an orbit"))).collect();
    assert_eq!(doc.get("lattice_end"), Some(&Value::Bool(true)));
    let control = witnessed(&i_a, &letters(&i_a, &pairs));
    assert_eq!(control.state().off_lattice_count(), 0);
    let value = total(nu_row(Base::Snapshot(&control)));
    let bits: Vec<&str> = value.nonzero().into_iter().map(|(name, _)| name).filter(|n| n.starts_with('b')).collect();
    assert_eq!(bits, odd);
    assert!(value.fixed_nonzero().is_empty());

    // The plant.
    let mut planted = menu("I_b");
    assert_ne!(planted.identity, nu::I_A_MENU_IDENTITY);
    planted.identity = nu::I_A_MENU_IDENTITY.to_owned();
    let planted = Arc::new(planted);
    let journal = letters(&planted, &MIRROR);
    assert!(journal.iter().any(|twist| !i_a.contains(ctx, twist)), "the word leaves I_a");
    let snapshot = witnessed(&planted, &journal);
    assert_eq!(snapshot.state().off_lattice_count(), 0);
    assert_eq!(snapshot.reachability().kind(), ReachKind::Witnessed);
    let raw = total(nu::value(ctx, snapshot.state()).expect("no engine error"));
    assert_eq!(raw.fixed_nonzero(), vec!["b5"]);
    let refusal = nu::nu(ctx, Base::Snapshot(&snapshot)).expect("no engine error").expect_err("the plant is refused");
    assert_eq!(refusal.code, codes::FIXED_NU_NONZERO);
    assert!(refusal.message.contains("b5"), "{}", refusal.message);
    let checked = corrections::correction_set_check(ctx, Base::Snapshot(&snapshot), &[(2, 1)]);
    assert_eq!(checked.expect("no engine error").expect_err("the refusal passes through"), refusal);

    // An open claim and the true I_b menu bind nothing.
    let block = ctx.identity(Some(&*planted));
    let open = Reachability::stated(ReachKind::Open, Value::Null).expect("an open claim");
    let import = levels::import(ctx, Some(planted.clone()), &snapshot.state().record(ctx), &block, open)
        .expect("the table reads")
        .expect("the identity agrees");
    assert_eq!(total(nu_row(Base::Derived(&import))), raw);
    let i_b = Arc::new(menu("I_b"));
    let honest = witnessed(&i_b, &letters(&i_b, &MIRROR));
    assert_eq!(honest.state().digest(ctx), snapshot.state().digest(ctx));
    assert_eq!(total(nu_row(Base::Snapshot(&honest))), raw);
    let checked = corrections::correction_set_check(ctx, Base::Snapshot(&honest), &[(2, 1)]);
    let refused = checked.expect("no engine error").expect_err("I_b is not the catalogue's menu");
    assert_eq!(refused.code, codes::MENU_BINDING_MISMATCH);

    let verdict = levels::label_checkpoint(ctx, snapshot.state(), snapshot.reachability(), Some(&raw.values()))
        .expect("no engine error");
    assert!(
        matches!(&verdict, Verdict::No(e) if e.get("fact").and_then(Value::as_str) == Some("nu_nonzero")),
        "{verdict:?}"
    );
}

/// CT-LB3-04's fourth sentence, "a drafted correction set is checked without selecting one" (KN2.4's
/// `correction_set_check`; JI.13, EH-11). The base is the retained word [1, 3, −5] followed by correction 2 under I_a:
/// a witnessed lattice configuration whose ν is correction 2's row. Each drafted set below, (id, signed multiplicity)
/// as a solver names them, checks to ν plus its rows in F2 and F5, as the catalogue predicts: the empty set gives ν
/// itself, not a set that would cancel it; correction 2 once, or taken back, gives 0, and twice ν; correction 3 gives
/// another nonzero value; a set's order does not matter. A set with an id the catalogue does not ship is refused whole.
/// Each answer is ν's 45 values: no id, set, ranking or word.
#[test]
fn ct_lb3_04_a_drafted_correction_set_is_checked_without_selecting_one() {
    let ctx = ctx();
    let menu = i_a();
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let mut journal: Vec<Twist> =
        [1, 3, -5].iter().map(|&id| levels::primitive(ctx, id).expect("a retained generator")).collect();
    journal.extend(correction(&menu, 2));
    let base = witnessed(&menu, &journal);
    let value = total(nu_row(Base::Snapshot(&base)));
    assert_eq!(value, row(2));

    let drafts: [&[(u32, i64)]; 7] =
        [&[], &[(2, 1)], &[(2, -1)], &[(2, 2)], &[(3, 1)], &[(2, 1), (3, 1)], &[(3, 1), (2, 1)]];
    let mut answers = Vec::new();
    for draft in drafts {
        let answer = total(check(Base::Snapshot(&base), draft));
        assert_eq!(Ok(answer), catalogue.predict(&value, draft), "{draft:?}");
        let Value::Array(values) = answer.to_json() else { panic!("ν is a list") };
        assert!(values.len() == levels::NU_LEN && values.iter().all(|v| matches!(v, Value::Int(_))), "{draft:?}");
        answers.push(answer);
    }
    assert_eq!(answers[0], value, "the empty draft is ν itself");
    assert_eq!([answers[1], answers[2], answers[3]], [NuVector::ZERO, NuVector::ZERO, value]);
    assert!(!answers[4].is_zero());
    assert_eq!((answers[5], answers[6]), (row(3), row(3)));

    let refused = corrections::correction_set_check(ctx, Base::Snapshot(&base), &[(2, 1), (36, 1)]);
    assert_eq!(refused.expect("no engine error").expect_err("id 36 is not shipped").code, codes::NAMED_OBJECT_INVALID);
}
