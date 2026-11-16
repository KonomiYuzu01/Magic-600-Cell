//! CT-EV3-06, the off-lattice half (EV3.8, EV3.9; KN0.4 to KN0.6): an off-lattice relation is distinct from unknown.
//! The relation is `c600_jumble_tool::relation`'s "current occupant" of a sticker slot at pose level, on states of the
//! pinned I_a menu that `c600_jumble` builds: solved; after the first record of the I_a journal, a twist off the
//! lattice at grip 57; after its first two records, whose composite puts some pieces back on the lattice at poses
//! other than the identity; and after retained generator 1 (H_0) at grip 0, a twist on the lattice.
//!
//! What runs: every one of the 259,800 slots in each state; "none: off the lattice" and `unknown(reason)` as two
//! different values, both in one result; never the old occupant, never a sentinel piece and no boolean anywhere in the
//! answer; the coverage and the missing part of a partial result, total and undefined results; the relation, level,
//! identity block and pose digest each result names (KN0.6); and the refusal of a slot outside the model. Not run: the
//! unchanged-frame half of CT-EV3-06 (EV3.10), which `c600-repr`'s linked selection runs (C6); the strip's statement
//! reference, claims, evidence and source location (chapter 06, RPI.65); the occupant of a slot whose K⁺-orbit has a
//! piece at a lattice pose other than the identity, which needs a slot action the public kernel API does not have
//! (LP2); EV3.8's other roles; and 600-cell-Full's label-level relations. A pass is source and fixture evidence.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::model::NS;
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_jumble_tool::level::Level;
use c600_jumble_tool::relation::{
    current_occupants, Missing, NoSuchSlot, Occupants, Partial, NO_SLOT_ACTION, RELATION,
};
use c600_kernel::json::{self, Value};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn fixture() -> &'static Value {
    static DOC: OnceLock<Value> = OnceLock::new();
    DOC.get_or_init(|| {
        let bytes =
            std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
        json::parse(&bytes).expect("the I_a fixture is JSON")
    })
}

fn i_a() -> Arc<Menu> {
    Arc::new(Menu::from_record(ctx(), fixture().get("menu").expect("a menu")).expect("the pinned I_a menu"))
}

/// The state after the first `count` records of the I_a journal from solved.
fn after(menu: &Arc<Menu>, count: usize) -> State {
    let journal = fixture().get("journal").expect("a journal");
    let records = journal.get("records").and_then(Value::as_array).expect("records");
    let mut state = State::new(Some(menu.clone()));
    for (index, record) in records[..count].iter().enumerate() {
        let twist = Twist::from_record(ctx(), record).expect("a journal record");
        let outcome = state.apply(ctx(), &twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "record {index}");
    }
    state
}

/// Retained generator 1, H_0, as the menu's letter at grip 0.
fn h0(menu: &Menu) -> Twist {
    let ctx = ctx();
    let twists = menu.for_cap(ctx, 0).expect("the menu's twists at grip 0");
    let h0 = ctx.kplus.gen_idx[0];
    twists.into_iter().find(|twist| twist.kidx == Some(h0)).expect("the I_a menu holds H_0 at grip 0")
}

fn all_slots() -> Vec<u32> {
    (0..NS as u32).collect()
}

fn home(slot: u32) -> u32 {
    ctx().model.slot_piece[slot as usize]
}

fn at_identity(state: &State, piece: u32) -> bool {
    state.pose_of(piece as usize).kidx() == Some(0)
}

/// The covered and the missing part of an answer, each absent when the answer has none.
type Parts<'a> = (Option<&'a BTreeMap<u32, u32>>, Option<&'a BTreeMap<u32, Missing>>);

/// The answer's two parts, with the rules every result keeps: the covered slots map to their own home piece, which is
/// at the identity; every other slot is missing and its home piece has moved, so no slot answers its old occupant or a
/// sentinel; the two parts split the named slots.
fn parts<'a>(state: &State, occupants: &'a Occupants, named: &[u32]) -> Parts<'a> {
    let (covered, missing) = match &occupants.answer {
        Partial::Total(covered) => (Some(covered), None),
        Partial::Partial { covered, missing } => {
            assert!(!covered.is_empty() && !missing.is_empty(), "a partial result has both parts");
            (Some(covered), Some(missing))
        }
        Partial::Undefined(missing) => (None, Some(missing)),
    };
    let (no_cover, no_missing) = (BTreeMap::new(), BTreeMap::new());
    let (some_covered, some_missing) = (covered.unwrap_or(&no_cover), missing.unwrap_or(&no_missing));
    let mut seen: Vec<u32> = some_covered.keys().chain(some_missing.keys()).copied().collect();
    seen.sort_unstable();
    let mut named = named.to_vec();
    named.sort_unstable();
    named.dedup();
    assert_eq!(seen, named, "the covered and the missing slots split the named slots");
    for (&slot, &piece) in some_covered {
        assert!(piece == home(slot) && at_identity(state, piece), "slot {slot} answers its own home piece");
        assert_eq!(occupants.get(slot), Some(Ok(piece)));
    }
    for (&slot, &missing) in some_missing {
        assert!(!at_identity(state, home(slot)), "slot {slot} is missing though its home piece did not move");
        assert_eq!(occupants.get(slot), Some(Err(missing)), "slot {slot} answers no piece, old or sentinel");
        match missing {
            Missing::OffLattice => assert!(!state.is_lattice_piece(home(slot) as usize), "slot {slot}"),
            Missing::Unknown(reason) => assert_eq!(reason, NO_SLOT_ACTION, "slot {slot}"),
        }
    }
    (covered, missing)
}

fn count(missing: Option<&BTreeMap<u32, Missing>>, kind: fn(&Missing) -> bool) -> usize {
    missing.map_or(0, |missing| missing.values().filter(|value| kind(value)).count())
}

fn off(missing: &Missing) -> bool {
    *missing == Missing::OffLattice
}

fn unknown(missing: &Missing) -> bool {
    matches!(missing, Missing::Unknown(_))
}

/// CT-EV3-06 (part: "an off-lattice relation is distinct from unknown"). After the first two I_a records, 5,404 pieces
/// are off the lattice and 395 sit at lattice poses other than the identity. The slots of the pieces at the identity
/// answer their own home piece; a slot whose position no lattice piece can fill answers "none: off the lattice"; a
/// slot whose K⁺-orbit has a piece at a lattice pose other than the identity answers `unknown(reason)`, since without
/// the slot action the relation cannot tell whether that piece fills the position. All three answers sit in one
/// partial result, which keeps its coverage and its missing part. The result names the relation, the pose level, the
/// identity block and the pose digest of the state it read.
#[test]
fn ct_ev3_06_none_off_the_lattice_is_distinct_from_unknown() {
    let ctx = ctx();
    let menu = i_a();
    let state = after(&menu, 2);
    assert_eq!((state.moved_count(), state.off_lattice_count()), (5_799, 5_404), "moved and off-lattice pieces");

    let slots = all_slots();
    let occupants = current_occupants(ctx, &state, &slots).expect("slots of the model");
    assert_eq!(
        (occupants.relation, occupants.level, &occupants.identity, &occupants.read),
        (RELATION, Level::Poses, &ctx.identity(Some(menu.as_ref())), &state.digest(ctx)),
        "the result names what it read"
    );
    let (covered, missing) = parts(&state, &occupants, &slots);
    assert!(matches!(occupants.answer, Partial::Partial { .. }));
    let moved = slots.iter().filter(|&&slot| !at_identity(&state, home(slot))).count();
    assert_eq!(covered.map_or(0, BTreeMap::len), NS - moved);
    let (none, unknowns) = (count(missing, off), count(missing, unknown));
    assert!(none > 0 && unknowns > 0, "both answers occur: {none} none, {unknowns} unknown");
    assert_eq!(none + unknowns, moved);
    assert_ne!(Missing::OffLattice, Missing::Unknown(NO_SLOT_ACTION), "two different values");
}

/// CT-EV3-06 (part: "never an old occupant"). Solved, every slot answers its home piece and the result is total. After
/// the first I_a record, the slots of the 3,097 moved pieces answer "none: off the lattice" and never the piece that
/// sat there before the twist; the rest answer their home piece. A further twist on the lattice (H_0 at grip 0) puts a
/// piece of every K⁺-orbit at a lattice pose other than the identity, so each slot that answered "none" now answers
/// `unknown(reason)`: the relation does not claim an absence it cannot show, and it never falls back to the old
/// occupant. Each result names the pose digest of its own state.
#[test]
fn ct_ev3_06_an_off_lattice_slot_never_answers_its_old_occupant() {
    let ctx = ctx();
    let menu = i_a();
    let slots = all_slots();

    let solved = State::new(Some(menu.clone()));
    let at_solved = current_occupants(ctx, &solved, &slots).expect("slots of the model");
    let (covered, _) = parts(&solved, &at_solved, &slots);
    assert!(matches!(at_solved.answer, Partial::Total(_)) && covered.map(BTreeMap::len) == Some(NS));

    let mut state = after(&menu, 1);
    assert_eq!((state.moved_count(), state.off_lattice_count()), (3_097, 3_097), "moved off the lattice only");
    let off_lattice = current_occupants(ctx, &state, &slots).expect("slots of the model");
    let (_, missing) = parts(&state, &off_lattice, &slots);
    assert!(matches!(off_lattice.answer, Partial::Partial { .. }));
    assert_eq!(count(missing, unknown), 0);
    let none: Vec<u32> = missing.expect("a missing part").keys().copied().collect();
    for &slot in &none {
        assert_eq!(at_solved.get(slot), Some(Ok(home(slot))), "slot {slot}: its occupant at solved");
        assert_eq!(off_lattice.get(slot), Some(Err(Missing::OffLattice)), "slot {slot}");
    }

    let outcome = state.apply(ctx, &h0(&menu), None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
    let mut turned = vec![false; ctx.regions.reps.len()];
    for piece in 0..state.pose_ids().len() {
        turned[ctx.regions.orbit_of[piece] as usize] |= matches!(state.pose_of(piece).kidx(), Some(k) if k != 0);
    }
    assert!(
        turned.iter().all(|&turned| turned),
        "every K⁺-orbit has a piece at a lattice pose other than the identity"
    );
    let on_lattice = current_occupants(ctx, &state, &slots).expect("slots of the model");
    let (_, missing) = parts(&state, &on_lattice, &slots);
    assert!(count(missing, unknown) > none.len(), "the turned slots of cap 0 are unknown too");
    for &slot in &none {
        assert_eq!(on_lattice.get(slot), Some(Err(Missing::Unknown(NO_SLOT_ACTION))), "slot {slot}");
    }
    let reads = [&at_solved.read, &off_lattice.read, &on_lattice.read];
    assert!(reads[0] != reads[1] && reads[1] != reads[2] && reads[0] != reads[2], "each names its own state");
    assert_eq!(on_lattice.read, state.digest(ctx));
}

/// CT-EV3-06 (part: "a partially defined relation retains its coverage and missing part"). Named alone, the slots of
/// moved pieces give an undefined result whose reason is the missing part, kept slot by slot: "none: off the lattice"
/// after the first I_a record, `unknown(reason)` after H_0 at grip 0 from solved. With one unmoved slot added, the
/// result is partial with that slot covered. An unnamed slot has no answer, no slot is a total result with nothing in
/// it, and a slot outside the model refuses the whole call.
#[test]
fn ct_ev3_06_coverage_and_missing_part_are_kept() {
    let ctx = ctx();
    let menu = i_a();
    let moved = |state: &State| -> Vec<u32> {
        (0..NS as u32).filter(|&slot| !at_identity(state, home(slot))).step_by(97).collect()
    };

    let off_lattice = after(&menu, 1);
    let mut turned = State::new(Some(menu.clone()));
    let outcome = turned.apply(ctx, &h0(&menu), None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
    for (state, expected) in [(&off_lattice, Missing::OffLattice), (&turned, Missing::Unknown(NO_SLOT_ACTION))] {
        let named = moved(state);
        assert!(named.len() > 10, "moved slots to name");
        let occupants = current_occupants(ctx, state, &named).expect("slots of the model");
        let (_, missing) = parts(state, &occupants, &named);
        let Partial::Undefined(reason) = &occupants.answer else {
            panic!("nothing is covered: {:?}", occupants.answer)
        };
        assert!(reason.values().all(|value| *value == expected), "every slot keeps its own reason");
        assert_eq!(missing.map(BTreeMap::len), Some(named.len()));

        let unmoved = (0..NS as u32).find(|&slot| at_identity(state, home(slot))).expect("an unmoved slot");
        let mut named = named.clone();
        named.extend([unmoved, unmoved]);
        let occupants = current_occupants(ctx, state, &named).expect("slots of the model");
        let (covered, missing) = parts(state, &occupants, &named);
        assert_eq!(covered.map(|covered| covered.keys().copied().collect()), Some(vec![unmoved]));
        assert!(missing.is_some_and(|missing| missing.values().all(|value| *value == expected)));
        let absent = (0..NS as u32).find(|slot| !named.contains(slot)).expect("an unnamed slot");
        assert_eq!(occupants.get(absent), None, "an unnamed slot has no answer");
    }

    let nothing = current_occupants(ctx, &off_lattice, &[]).expect("no slot");
    assert_eq!(nothing.answer, Partial::Total(BTreeMap::new()));
    let refused = current_occupants(ctx, &off_lattice, &[0, NS as u32]).expect_err("a slot outside the model");
    assert_eq!(refused, NoSuchSlot(NS as u32));
    assert_eq!(refused.code(), codes::NAMED_OBJECT_INVALID);
}
