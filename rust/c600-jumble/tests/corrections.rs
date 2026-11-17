//! LP2: the shipped correction catalogue and the drafted correction-set check (`c600_jumble::corrections`; DT-J1 O7
//! (a), JI.13, EH-11; KN2.4's `correction_set_check` row). `tests/test_jumbling_rust_corrections.py` plays all 36
//! corrections through the adapter in release and compares the catalogue with the research library. Nothing here
//! chooses a correction: every id below is fixed in the test.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::corrections::{self, COUNT};
use c600_jumble::levels::{self, Base, Derivation, Partial, Snapshot};
use c600_jumble::nu::{self, NuVector, I_A_FIXED, I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context, CONTRACT_REVISION};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectionMode;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn fixture(name: &str) -> Value {
    let bytes = std::fs::read(repository().join("research/jumbling/fixtures").join(name)).expect("the fixture");
    json::parse(&bytes).expect("the fixture is JSON")
}

struct Setup {
    ctx: Context,
    menu: Arc<Menu>,
}

/// The model and the pinned I_a menu, loaded once for every test of this file.
fn setup() -> &'static Setup {
    static SETUP: OnceLock<Setup> = OnceLock::new();
    SETUP.get_or_init(|| {
        let ctx = Context::load(&repository().join("assets")).expect("the jumbling model loads");
        let menu = Menu::from_record(&ctx, fixture("wj-I_a.json").get("menu").expect("a menu")).expect("I_a");
        Setup { ctx, menu: Arc::new(menu) }
    })
}

fn total(value: Partial<NuVector>) -> NuVector {
    match value {
        Partial::Total(value) => value,
        other => panic!("not total: {other:?}"),
    }
}

/// The literal word of correction `id` under I_a.
fn word(s: &Setup, id: u32) -> Vec<Twist> {
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let word = catalogue.twists(&s.ctx, &s.menu, id).expect("no engine error").expect("the word reads");
    assert_eq!(word.len(), catalogue.lookup(id).expect("a shipped id").letters().len());
    word
}

/// Correction `id` played literally from solved under I_a, every step admissible, with the snapshot it witnesses.
fn played(s: &Setup, id: u32) -> Snapshot {
    let word = word(s, id);
    let mut state = State::new(Some(s.menu.clone()));
    for twist in &word {
        let outcome = state.apply(&s.ctx, twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "correction {id}");
    }
    Snapshot::witnessed(&s.ctx, state, &word, None).expect("no engine error").expect("the word witnesses the state")
}

#[test]
fn the_catalogue_holds_the_36_bound_to_i_a() {
    let s = setup();
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    assert_eq!(catalogue.rows().len(), COUNT);
    for (index, correction) in catalogue.rows().iter().enumerate() {
        assert_eq!(correction.id() as usize, index);
        assert!(!correction.family().is_empty());
        assert!(!correction.letters().is_empty());
        let values = correction.row().values();
        assert!(I_A_FIXED.iter().all(|&i| values[i] == 0), "correction {index} moves a fixed coordinate");
    }
    let binding = catalogue.binding();
    assert_eq!(binding.model_id, s.ctx.model.model_id);
    assert_eq!((binding.menu_identity.as_str(), binding.menu_order.as_str()), (I_A_MENU_IDENTITY, I_A_ORDER_DIGEST));
    assert_eq!((s.menu.identity.as_str(), s.menu.order.as_str()), (I_A_MENU_IDENTITY, I_A_ORDER_DIGEST));
    assert_eq!(binding.contract_revision, CONTRACT_REVISION);
    assert_eq!(catalogue.library(), "research/jumbling/restoration/09-general-method/data/library-merged.json.gz");
}

/// EH-11 on the corrections of at most 37 letters (ids 0 to 16; the adapter test plays all 36 in release): from
/// solved, each word is admissible step by step and changes ν by exactly its row. The drafted check from solved
/// predicts that value, and from the played state, with the same id taken back, it predicts 0.
#[test]
fn short_corrections_change_nu_by_their_rows() {
    let s = setup();
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let solved = Snapshot::solved(&s.ctx, Some(s.menu.clone()));
    for id in 0..=16 {
        let snapshot = played(s, id);
        let value = total(nu::nu(&s.ctx, Base::Snapshot(&snapshot)).expect("no engine error").expect("not refused"));
        assert_eq!(&value, catalogue.lookup(id).expect("shipped").row(), "correction {id}");
        let predicted = corrections::correction_set_check(&s.ctx, Base::Snapshot(&solved), &[(id, 1)]);
        assert_eq!(total(predicted.expect("no engine error").expect("not refused")), value, "correction {id}");
        let back = corrections::correction_set_check(&s.ctx, Base::Snapshot(&snapshot), &[(id, -1)]);
        assert_eq!(total(back.expect("no engine error").expect("not refused")), NuVector::ZERO, "correction {id}");
    }
}

/// The row reads a derived value as it reads a snapshot.
#[test]
fn the_check_reads_a_derived_value() {
    let s = setup();
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let solved = Snapshot::solved(&s.ctx, Some(s.menu.clone()));
    let identity = s.ctx.identity(Some(&*s.menu));
    let answer = levels::derive(&s.ctx, Base::Snapshot(&solved), &identity, &word(s, 3), ProtectionMode::Net, &[])
        .expect("no engine error")
        .expect("the request is not refused");
    let Derivation::Derived(derived) = answer else { panic!("the word is admissible from solved") };
    let row = *catalogue.lookup(3).expect("shipped").row();
    let check = corrections::correction_set_check(&s.ctx, Base::Derived(&derived), &[]);
    assert_eq!(total(check.expect("no engine error").expect("not refused")), row);
    let check = corrections::correction_set_check(&s.ctx, Base::Derived(&derived), &[(3, 1), (4, 1)]);
    let row4 = *catalogue.lookup(4).expect("shipped").row();
    assert_eq!(total(check.expect("no engine error").expect("not refused")), row.add_times(&row, 1) + row4);
}

#[test]
fn predictions_add_signed_multiples_in_f2_and_f5() {
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let zero = NuVector::ZERO;
    let row = |id: u32| *catalogue.lookup(id).expect("shipped").row();
    assert_eq!(catalogue.predict(&zero, &[]), Ok(zero));
    assert_eq!(catalogue.predict(&zero, &[(0, 1), (2, 1)]), Ok(row(0) + row(2)));
    assert_eq!(catalogue.predict(&zero, &[(0, 1), (2, 1)]), catalogue.predict(&zero, &[(2, 1), (0, 1)]));
    for id in 0..COUNT as u32 {
        assert_eq!(catalogue.predict(&zero, &[(id, 1), (id, -1)]), Ok(zero));
        assert_eq!(catalogue.predict(&zero, &[(id, 3)]), Ok(row(id) + row(id) + row(id)));
        assert_eq!(catalogue.predict(&zero, &[(id, 10)]), Ok(zero), "correction {id}: 10 is 0 in F2 and in F5");
        // i64::MIN = -2^63 is 0 in F2 and 2 in F5 (2^63 = 3 mod 5), so it acts as two plays;
        // i64::MAX = 2^63 - 1 is 1 in F2 and 2 in F5, so it acts as seven.
        let plays = |k: usize| (0..k).fold(zero, |sum, _| sum + row(id));
        assert_eq!(catalogue.predict(&zero, &[(id, i64::MIN)]), Ok(plays(2)), "correction {id}");
        assert_eq!(catalogue.predict(&zero, &[(id, i64::MAX)]), Ok(plays(7)), "correction {id}");
        assert_eq!(catalogue.predict(&row(id), &[(id, -1)]), Ok(zero));
    }
    // A C5 value moves in F5: five plays of a correction with a nonzero C5 value bring it back.
    let c5 = (0..COUNT as u32).find(|&id| row(id).c5().iter().any(|v| v.value() != 0)).expect("a C5 row");
    assert_ne!(catalogue.predict(&zero, &[(c5, 2)]), Ok(zero));
    assert_eq!(catalogue.predict(&zero, &[(c5, 5)]), Ok(zero));
}

#[test]
fn unknown_ids_and_other_menus_are_refused() {
    let s = setup();
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let solved = Snapshot::solved(&s.ctx, Some(s.menu.clone()));
    for id in [COUNT as u32, 1000, u32::MAX] {
        assert_eq!(catalogue.lookup(id).map(|_| ()).unwrap_err().code, codes::NAMED_OBJECT_INVALID);
        let predicted = catalogue.predict(&NuVector::ZERO, &[(0, 1), (id, 1)]);
        assert_eq!(predicted.unwrap_err().code, codes::NAMED_OBJECT_INVALID);
        let twists = catalogue.twists(&s.ctx, &s.menu, id).expect("no engine error");
        assert_eq!(twists.unwrap_err().code, codes::NAMED_OBJECT_INVALID);
        let check = corrections::correction_set_check(&s.ctx, Base::Snapshot(&solved), &[(0, 1), (id, 1)]);
        assert_eq!(check.expect("no engine error").unwrap_err().code, codes::NAMED_OBJECT_INVALID);
    }

    // Free mode: no menu, so no correction.
    let free = Snapshot::solved(&s.ctx, None);
    let check = corrections::correction_set_check(&s.ctx, Base::Snapshot(&free), &[(0, 1)]);
    assert_eq!(check.expect("no engine error").unwrap_err().code, codes::MENU_BINDING_MISMATCH);

    // Another menu: I_b.
    let i_b = Menu::from_record(&s.ctx, fixture("wj-I_b.json").get("menu").expect("a menu")).expect("I_b");
    assert_ne!(i_b.identity, I_A_MENU_IDENTITY);
    let twists = catalogue.twists(&s.ctx, &i_b, 0).expect("no engine error");
    assert_eq!(twists.unwrap_err().code, codes::MENU_BINDING_MISMATCH);
    let other = Snapshot::solved(&s.ctx, Some(Arc::new(i_b)));
    let check = corrections::correction_set_check(&s.ctx, Base::Snapshot(&other), &[(0, 1)]);
    assert_eq!(check.expect("no engine error").unwrap_err().code, codes::MENU_BINDING_MISMATCH);

    // I_a's items in reverse order: the same twists, numbered otherwise. The letters number items in I_a's recorded
    // order, so the word is refused; the check uses rows, not letters, and reads the identity alone, as ν's gate does.
    let mut record = fixture("wj-I_a.json").get("menu").expect("a menu").clone();
    let Some(Value::Array(base)) = (match &mut record {
        Value::Object(fields) => fields.get_mut("base"),
        _ => None,
    }) else {
        panic!("a menu record has a base list")
    };
    base.reverse();
    let reordered = Menu::from_record(&s.ctx, &record).expect("I_a reordered");
    assert_eq!(reordered.identity, I_A_MENU_IDENTITY);
    assert_ne!(reordered.order, I_A_ORDER_DIGEST);
    let twists = catalogue.twists(&s.ctx, &reordered, 0).expect("no engine error");
    assert_eq!(twists.unwrap_err().code, codes::MENU_BINDING_MISMATCH);
    let base = Snapshot::solved(&s.ctx, Some(Arc::new(reordered)));
    let check = corrections::correction_set_check(&s.ctx, Base::Snapshot(&base), &[(0, 1)]);
    let row = *catalogue.lookup(0).expect("shipped").row();
    assert_eq!(total(check.expect("no engine error").expect("not refused")), row);
}

/// Off the lattice ν is undefined, and so is the check: never zeros, a partial vector or a prediction.
#[test]
fn off_the_lattice_the_check_is_undefined() {
    let s = setup();
    let records = fixture("wj-I_a.json");
    let records = records.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array).expect("records");
    let journal: Vec<Twist> =
        records[..8].iter().map(|r| Twist::from_record(&s.ctx, r).expect("a record the candidate reads")).collect();
    let mut state = State::new(Some(s.menu.clone()));
    for twist in &journal {
        state.apply(&s.ctx, twist, None, &[]).expect("no engine error");
    }
    let snapshot =
        Snapshot::witnessed(&s.ctx, state, &journal, None).expect("no engine error").expect("the journal witnesses");
    assert!(snapshot.state().off_lattice_count() > 0);
    for set in [&[][..], &[(0, 1)][..]] {
        let check = corrections::correction_set_check(&s.ctx, Base::Snapshot(&snapshot), set);
        match check.expect("no engine error").expect("not refused") {
            Partial::Undefined(reason) => assert_eq!(reason, nu::OFF_LATTICE),
            other => panic!("defined off the lattice: {other:?}"),
        }
    }
}
