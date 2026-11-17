//! LP2: ν, the label invariant of A5, over KN3.2's F2 and F5 (`c600_jumble::nu`; chapter 01, KN2.4's `nu` row and
//! KN3.2; state contract A5). `tests/test_jumbling_rust_nu.py` compares the same values through the adapter with the ν
//! reference (`research/jumbling/differential/references/nu.py`, over restoration stage 03's `nucoords.py`). Nothing
//! here chooses a twist: every word below is fixed in the test or read from a fixture.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::kplus::ORDER;
use c600_jumble::levels::{self, Base, Partial, ReachKind, Reachability, Snapshot, Verdict};
use c600_jumble::model::NP;
use c600_jumble::nu::{self, GroupKind, NuVector};
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context;
use c600_kernel::json::{self, Value};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

struct Setup {
    ctx: Context,
    menu: Arc<Menu>,
    fixture: Value,
}

/// The model and the pinned I_a menu with its fixture, loaded once for every test of this file.
fn setup() -> &'static Setup {
    static SETUP: OnceLock<Setup> = OnceLock::new();
    SETUP.get_or_init(|| {
        let ctx = Context::load(&repository().join("assets")).expect("the jumbling model loads");
        let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the fixture");
        let fixture = json::parse(&fixture).expect("the I_a fixture is JSON");
        let menu = Menu::from_record(&ctx, fixture.get("menu").expect("a menu")).expect("the pinned I_a menu");
        Setup { ctx, menu: Arc::new(menu), fixture }
    })
}

/// Fixed retained words of signed primitive ids, as in the M8 tests.
const WORDS: [&[i64]; 3] = [&[1, 3, -5], &[2, 2, 2, -7, 600, -1199], &[9, -9, 1024, 77, -400]];

/// The pieces' home slot counts per orbit (k of `nucoords.py`).
const SLOTS: [usize; 35] =
    [2, 1, 1, 2, 1, 1, 5, 1, 2, 2, 1, 2, 1, 1, 2, 1, 2, 5, 1, 1, 2, 1, 2, 2, 1, 2, 1, 1, 5, 1, 2, 1, 2, 1, 20];

/// `journal` applied from solved under `menu`, every step admissible, with the snapshot it witnesses.
fn witnessed(ctx: &Context, menu: Option<Arc<Menu>>, journal: &[Twist]) -> Snapshot {
    let mut state = State::new(menu);
    for twist in journal {
        let outcome = state.apply(ctx, twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible);
    }
    Snapshot::witnessed(ctx, state, journal, None).expect("no engine error").expect("the journal witnesses the state")
}

fn total(value: Partial<NuVector>) -> NuVector {
    match value {
        Partial::Total(value) => value,
        other => panic!("ν is not total: {other:?}"),
    }
}

fn reason(value: Partial<NuVector>) -> String {
    match value {
        Partial::Undefined(reason) => reason,
        other => panic!("ν is not undefined: {other:?}"),
    }
}

/// The state with every piece home except `piece`, at the K⁺ pose `k`, under I_a: a raw table (KN2.14).
fn turned(s: &Setup, piece: usize, k: u16) -> State {
    let mut poses = vec![Pose::K(0); NP];
    poses[piece] = Pose::K(k);
    levels::state_of(&s.ctx, Some(s.menu.clone()), &poses).expect("the raw table reads")
}

#[test]
fn the_tables_label_every_piece_and_follow_the_census() {
    let ctx = &setup().ctx;
    let tables = nu::tables(ctx).expect("ν's tables build");
    for (orbit, &slots) in SLOTS.iter().enumerate() {
        let group = tables.group(orbit).expect("a frame group");
        assert_eq!((group.kind(), group.order(), group.slots()), (nu::group_kind(orbit), group.kind().order(), slots));
        assert_eq!(group.generator().is_some(), group.kind() == GroupKind::C5, "orbit {orbit}");
    }
    assert!(tables.group(35).is_none());
    // r0 of each C5 group, the least element of order 5 (A5's C5 convention).
    assert_eq!(tables.group(17).and_then(|g| g.generator()), Some(&[1u8, 3, 0, 4, 2][..]));
    assert_eq!(tables.group(28).and_then(|g| g.generator()), Some(&[1u8, 2, 4, 0, 3][..]));
    let mut labelled = 0;
    for piece in 0..NP {
        let slots = tables.labelling(piece);
        match usize::try_from(ctx.model.orbit_id[piece]) {
            Err(_) => assert!(slots.is_empty(), "centre {piece}"),
            Ok(orbit) => {
                assert_eq!(slots.len(), SLOTS[orbit], "piece {piece}");
                assert!(slots.iter().all(|&s| ctx.model.slot_piece[s as usize] as usize == piece), "piece {piece}");
                labelled += 1;
            }
        }
    }
    assert_eq!(labelled, NP - 600);
    assert!(tables.labelling(NP).is_empty());
}

#[test]
fn nu_is_zero_at_solved_and_after_retained_words() {
    let s = setup();
    let ctx = &s.ctx;
    for menu in [Some(s.menu.clone()), None] {
        let solved = Snapshot::solved(ctx, menu);
        assert_eq!(total(nu::value(ctx, solved.state()).expect("no engine error")), NuVector::ZERO);
        let row = nu::nu(ctx, Base::Snapshot(&solved)).expect("no engine error").expect("no refusal");
        assert_eq!(total(row), NuVector::ZERO);
    }
    for word in WORDS {
        let journal: Vec<Twist> = word.iter().map(|&mid| levels::primitive(ctx, mid).expect("a primitive")).collect();
        for menu in [Some(s.menu.clone()), None] {
            let snapshot = witnessed(ctx, menu, &journal);
            let value = total(nu::nu(ctx, Base::Snapshot(&snapshot)).expect("no engine error").expect("no refusal"));
            assert!(value.is_zero() && value.nonzero().is_empty(), "{word:?}");
            assert_eq!(value.values(), [0; levels::NU_LEN]);
        }
    }
}

#[test]
fn nu_is_undefined_off_the_lattice() {
    let s = setup();
    let ctx = &s.ctx;
    let records = s.fixture.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array).expect("records");
    let journal: Vec<Twist> =
        records[..8].iter().map(|r| Twist::from_record(ctx, r).expect("a record the candidate reads")).collect();
    let snapshot = witnessed(ctx, Some(s.menu.clone()), &journal);
    assert!(snapshot.state().off_lattice_count() > 0);
    assert_eq!(reason(nu::value(ctx, snapshot.state()).expect("no engine error")), nu::OFF_LATTICE);
    let row = nu::nu(ctx, Base::Snapshot(&snapshot)).expect("no engine error").expect("no refusal");
    assert_eq!(reason(row), nu::OFF_LATTICE);
}

/// A piece of orbit 0 turned in its own chamber changes f0 alone, a coordinate that A5 fixes under I_a. The raw table
/// enters with an open claim, so the row gives its value and refuses nothing: A5 bounds reachable I_a states only, and
/// an open claim shows no reachability. Moved into another piece's chamber, it collides and ν is undefined.
#[test]
fn a_turned_piece_sets_its_frame_bit_and_an_open_claim_is_not_bound() {
    let s = setup();
    let ctx = &s.ctx;
    let piece = (0..NP).find(|&p| ctx.model.orbit_id[p] == 0).expect("a piece of orbit 0");
    let region = levels::region_stabiliser(ctx, piece).expect("R_A");
    let flips: Vec<u16> = region
        .iter()
        .copied()
        .filter(|&k| k != 0 && total(nu::value(ctx, &turned(s, piece, k)).expect("no engine error")) != NuVector::ZERO)
        .collect();
    assert!(!flips.is_empty(), "the chamber's stabiliser turns the piece's frame");
    let state = turned(s, piece, flips[0]);
    let value = total(nu::value(ctx, &state).expect("no engine error"));
    assert_eq!((value.nonzero(), value.fixed_nonzero()), (vec![("f0", 1)], vec!["f0"]));
    let ia = ctx.identity(Some(&*s.menu));
    let open = Reachability::stated(ReachKind::Open, Value::Null).expect("an open claim");
    let import = levels::import(ctx, Some(s.menu.clone()), &state.record(ctx), &ia, open.clone())
        .expect("the table reads")
        .expect("the identity agrees");
    let row = nu::nu(ctx, Base::Derived(&import)).expect("no engine error").expect("an open claim is not bound");
    assert_eq!(total(row), value);
    let verdict = levels::label_checkpoint(ctx, &state, &open, Some(&value.values())).expect("no engine error");
    assert!(matches!(&verdict, Verdict::No(e) if e.get("fact").and_then(Value::as_str) == Some("nu_nonzero")));
    let away = (1..ORDER as u16).find(|k| !region.contains(k)).expect("a pose outside the chamber's stabiliser");
    assert_eq!(reason(nu::value(ctx, &turned(s, piece, away)).expect("no engine error")), nu::PIECES_COLLIDE);
}
