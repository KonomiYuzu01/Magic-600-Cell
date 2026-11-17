//! CT-EV3-06, the off-lattice half (EV3.8, EV3.9; KN0.4 to KN0.6): an off-lattice relation is distinct from unknown.
//! The relation is `c600_jumble_tool::relation`'s "current occupant" of a sticker slot at pose level, which places each
//! lattice piece with M8's slot action of K⁺ (`c600_jumble::levels::Tables::image`). The states are those of the
//! pinned I_a menu that `c600_jumble` builds: solved; after the first record of the I_a journal, a twist off the
//! lattice at grip 57; after its first two records, whose composite puts some pieces back on the lattice at poses
//! other than the identity; retained generator 1 (H_0) at grip 0, from solved and after the first record; and the
//! retained word [1, 3, −5]. Raw pose tables (KN2.14) that `levels::state_of` reads add a rotation of the whole puzzle
//! by H_0, a turned centre, one piece off the lattice, and two pieces in one chamber, which no twist reaches.
//!
//! What runs: every one of the 259,800 slots in each state, checked against M8's sticker map `levels::labels`. Where
//! the labels cover slot t, the occupant is the piece of the sticker the label names, at the identity and at other
//! lattice poses. Where they do not, the slot answers "none: off the lattice", or `unknown(reason)` only on a table
//! whose lattice pieces collide. Also: "none" and unknown as two different values, one slot answering each in two
//! states; never the old occupant, never a sentinel piece and no boolean anywhere in the answer; the coverage and the
//! missing part of a partial result, total and undefined results; the relation, level, identity block and pose digest
//! each result names (KN0.6); and the refusal of a slot outside the model. The labels and the relation share M8's
//! level tables, so the comparison checks the relation's chamber rule against M8's sticker map, not the tables
//! themselves. Not run: the unchanged-frame half of CT-EV3-06 (EV3.10), which `c600-repr`'s linked selection runs
//! (C6); the strip's statement reference, claims, evidence and source location (chapter 06, RPI.65); EV3.8's other
//! roles; and 600-cell-Full's label-level relations. A pass is source and fixture evidence.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::levels;
use c600_jumble::model::{NP, NS};
use c600_jumble::q5::{Mat4, Q5};
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, observe, Context};
use c600_jumble_tool::level::Level;
use c600_jumble_tool::relation::{
    current_occupants, Missing, NoSuchSlot, Occupants, Partial, PIECES_COLLIDE, RELATION,
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

fn play(state: &mut State, twist: &Twist) {
    let outcome = state.apply(ctx(), twist, None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
}

/// The state after the first `count` records of the I_a journal from solved.
fn after(menu: &Arc<Menu>, count: usize) -> State {
    let journal = fixture().get("journal").expect("a journal");
    let records = journal.get("records").and_then(Value::as_array).expect("records");
    let mut state = State::new(Some(menu.clone()));
    for record in &records[..count] {
        play(&mut state, &Twist::from_record(ctx(), record).expect("a journal record"));
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

/// The state after a retained word of signed 1-based primitive ids from solved.
fn retained(menu: &Arc<Menu>, word: &[i64]) -> State {
    let mut state = State::new(Some(menu.clone()));
    for &id in word {
        play(&mut state, &levels::primitive(ctx(), id).expect("a primitive id"));
    }
    state
}

/// A raw pose table of the I_a menu, read for form and identity only (KN2.14).
fn table(poses: &[Pose]) -> State {
    levels::state_of(ctx(), Some(i_a()), poses).expect("the raw table reads")
}

/// Solved except for the listed poses.
fn raw(posed: &[(usize, Pose)]) -> State {
    let mut poses = vec![Pose::K(0); NP];
    for (piece, pose) in posed {
        poses[*piece] = pose.clone();
    }
    table(&poses)
}

/// Piece 4 and another piece of its K⁺-orbit, with the K⁺ element that carries the other's home chamber onto piece
/// 4's.
fn congruent() -> (usize, usize, u16) {
    let ctx = ctx();
    let (kp, regions) = (&ctx.kplus, &ctx.regions);
    let (orbit, to_4) = regions.rep_of(4);
    let b = (0..NP).find(|&p| p != 4 && regions.rep_of(p).0 == orbit).expect("a second piece of the orbit");
    let onto = kp.compose(to_4, kp.inv[regions.rep_of(b).1 as usize]);
    assert_eq!(levels::tables(ctx).expect("the level tables").image(ctx, b, onto), 4);
    (4, b, onto)
}

/// The exact Cayley rotation ω = (2, 0, 0) at cap 0, which lies outside K⁺.
fn cayley_q() -> Arc<Mat4> {
    let omega = [Q5::int(2), Q5::zero(), Q5::zero()];
    let twist = Twist::from_record(ctx(), &observe::cayley_record(ctx(), 0, &omega)).expect("the Cayley record");
    assert_eq!(twist.kidx, None);
    twist.matrix
}

fn all_slots() -> Vec<u32> {
    (0..NS as u32).collect()
}

fn home(slot: u32) -> u32 {
    ctx().model.slot_piece[slot as usize]
}

fn slots_of(piece: usize) -> Vec<u32> {
    levels::tables(ctx()).expect("the level tables").slots(piece).to_vec()
}

fn occupants_of(state: &State, slots: &[u32]) -> Occupants {
    current_occupants(ctx(), state, slots).expect("no engine error").expect("slots of the model")
}

/// How a result over every slot splits: occupied by its home piece, occupied by another piece, "none: off the
/// lattice" and unknown.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    home: usize,
    other: usize,
    none: usize,
    unknown: usize,
}

/// Checks a result over every slot against M8's sticker map and counts its answers. Where `levels::labels` covers
/// slot t, the occupant is the lattice piece of the sticker the label names. Where it does not, the slot is missing:
/// "none: off the lattice" when no two lattice pieces collide and some piece is off, `unknown(PIECES_COLLIDE)` when
/// two collide. The result is total exactly when nothing is missing.
fn against_labels(state: &State, occupants: &Occupants) -> Counts {
    let ctx = ctx();
    let (labels, missing) = match levels::labels(ctx, state).expect("no engine error") {
        levels::Partial::Total(labels) => (labels, levels::Missing::default()),
        levels::Partial::Partial { covered, missing } => (covered, missing),
        levels::Partial::Undefined(reason) => panic!("the labels are undefined: {reason}"),
    };
    let mut sticker = vec![None; NS];
    for (&slot, &label) in labels.slots.iter().zip(&labels.labels) {
        sticker[slot as usize] = Some(label as u32);
    }
    let mut counts = Counts::default();
    for slot in 0..NS as u32 {
        match (sticker[slot as usize], occupants.get(slot).expect("every slot is named")) {
            (Some(sticker), Ok(piece)) => {
                assert_eq!(piece, home(sticker), "slot {slot}: the piece of the sticker the label names");
                assert!(state.is_lattice_piece(piece as usize), "slot {slot}: an occupant is on the lattice");
                if piece == home(slot) {
                    counts.home += 1;
                } else {
                    counts.other += 1;
                }
            }
            (None, Err(Missing::OffLattice)) => counts.none += 1,
            (None, Err(Missing::Unknown(reason))) => {
                assert_eq!(reason, PIECES_COLLIDE, "slot {slot}");
                counts.unknown += 1;
            }
            (sticker, answer) => panic!("slot {slot}: label {sticker:?}, occupant {answer:?}"),
        }
    }
    assert_eq!(missing.off, state.off_lattice_count());
    assert_eq!(counts.none + counts.unknown, missing.uncovered + missing.collisions, "the labels' missing slots");
    assert!(counts.none == 0 || (missing.collisions == 0 && missing.off > 0), "none only off the lattice: {counts:?}");
    assert!(counts.unknown == 0 || missing.collisions > 0, "unknown only where pieces collide: {counts:?}");
    let total = counts.none + counts.unknown == 0;
    assert_eq!(matches!(occupants.answer, Partial::Total(_)), total, "{counts:?}");
    counts
}

/// The two parts of an answer for the named slots: they split the named slots, `get` reads them, and each slot's
/// answer is the one a request for every slot gives it, so no answer depends on what else was named.
fn split(state: &State, occupants: &Occupants, named: &[u32]) -> (BTreeMap<u32, u32>, BTreeMap<u32, Missing>) {
    let (covered, missing) = match &occupants.answer {
        Partial::Total(covered) => (covered.clone(), BTreeMap::new()),
        Partial::Partial { covered, missing } => {
            assert!(!covered.is_empty() && !missing.is_empty(), "a partial result has both parts");
            (covered.clone(), missing.clone())
        }
        Partial::Undefined(missing) => (BTreeMap::new(), missing.clone()),
    };
    let mut seen: Vec<u32> = covered.keys().chain(missing.keys()).copied().collect();
    seen.sort_unstable();
    let mut named = named.to_vec();
    named.sort_unstable();
    named.dedup();
    assert_eq!(seen, named, "the covered and the missing slots split the named slots");
    let every = occupants_of_every_slot(state);
    for &slot in &named {
        assert_eq!(occupants.get(slot), every.get(slot), "slot {slot}");
    }
    (covered, missing)
}

fn occupants_of_every_slot(state: &State) -> Occupants {
    occupants_of(state, &all_slots())
}

/// CT-EV3-06 (part: "an off-lattice relation is distinct from unknown"). After the first two I_a records, 5,404
/// pieces are off the lattice and 395 sit at lattice poses other than the identity, some of them in chambers not
/// their own. Each slot whose position a lattice piece fills answers that piece, as M8's labels say; each slot whose
/// position no lattice piece fills answers "none: off the lattice"; nothing is unknown. On two raw tables that differ
/// in one piece b, b's slots answer "none: off the lattice" when b is off the lattice and `unknown(reason)` when b sits
/// in piece 4's chamber, which then holds two pieces: one slot, two different values. The result names the relation,
/// the pose level, the identity block and the pose digest of the state it read.
#[test]
fn ct_ev3_06_none_off_the_lattice_is_distinct_from_unknown() {
    let ctx = ctx();
    let menu = i_a();
    let state = after(&menu, 2);
    assert_eq!((state.moved_count(), state.off_lattice_count()), (5_799, 5_404), "moved and off-lattice pieces");
    let occupants = occupants_of_every_slot(&state);
    assert_eq!(
        (occupants.relation, occupants.level, &occupants.identity, &occupants.read),
        (RELATION, Level::Poses, &ctx.identity(Some(menu.as_ref())), &state.digest(ctx)),
        "the result names what it read"
    );
    assert!(matches!(occupants.answer, Partial::Partial { .. }));
    let counts = against_labels(&state, &occupants);
    assert!(counts.other > 0 && counts.none > 0 && counts.unknown == 0, "{counts:?}");

    // One slot, two values.
    let (a, b, onto) = congruent();
    let (at_a, at_b) = (slots_of(a), slots_of(b));
    let off = raw(&[(b, Pose::M(cayley_q()))]);
    let collided = raw(&[(b, Pose::K(onto))]);
    assert_eq!((off.off_lattice_count(), collided.off_lattice_count()), (1, 0));
    let (none, unknown) = (occupants_of_every_slot(&off), occupants_of_every_slot(&collided));
    let unaffected = NS - at_a.len() - at_b.len();
    assert_eq!(against_labels(&off, &none), Counts { home: NS - at_b.len(), other: 0, none: at_b.len(), unknown: 0 });
    assert_eq!(
        against_labels(&collided, &unknown),
        Counts { home: unaffected, other: 0, none: 0, unknown: at_a.len() + at_b.len() }
    );
    for &slot in &at_b {
        assert_eq!(none.get(slot), Some(Err(Missing::OffLattice)), "slot {slot}");
        assert_eq!(unknown.get(slot), Some(Err(Missing::Unknown(PIECES_COLLIDE))), "slot {slot}");
    }
    for &slot in &at_a {
        assert_eq!(none.get(slot), Some(Ok(a as u32)), "slot {slot}");
        assert_eq!(unknown.get(slot), Some(Err(Missing::Unknown(PIECES_COLLIDE))), "slot {slot}: two pieces");
    }
    assert_ne!(Missing::OffLattice, Missing::Unknown(PIECES_COLLIDE), "two different values");
}

/// CT-EV3-06 (part: the occupant at lattice poses other than the identity). On every slot of six states, the occupant
/// is the piece whose sticker M8's labels put there: after H_0 at grip 0 from solved, after the retained word
/// [1, 3, −5] and on the whole puzzle turned by H_0, where pieces fill chambers not their own; on centre piece 0 turned
/// by a third-turn about its pole, which keeps its own chamber, so its slots answer it although its pose is not the
/// identity; after the first I_a record and then H_0, where lattice pieces of cap 0 fill other chambers while pieces
/// are off the lattice; and solved.
#[test]
fn ct_ev3_06_the_occupant_at_other_lattice_poses_agrees_with_the_labels() {
    let ctx = ctx();
    let kp = &ctx.kplus;
    let menu = i_a();
    let mut turned = State::new(Some(menu.clone()));
    play(&mut turned, &h0(&menu));
    let mut after_h0 = after(&menu, 1);
    play(&mut after_h0, &h0(&menu));
    let third = kp.a4(0).into_iter().find(|&k| k != 0 && kp.compose(k, kp.compose(k, k)) == 0).expect("a third-turn");
    let centre = *ctx.model.centre_pieces.iter().find(|&&p| ctx.model.signature(p as usize) == [0]).expect("a centre");
    let states = [
        ("solved", State::new(Some(menu.clone())), [false, false]),
        ("H_0 at grip 0", turned, [true, false]),
        ("the retained word [1, 3, -5]", retained(&menu, &[1, 3, -5]), [true, false]),
        ("the whole puzzle turned by H_0", table(&vec![Pose::K(kp.gen_idx[0]); NP]), [true, false]),
        ("a turned centre", raw(&[(centre as usize, Pose::K(third))]), [false, false]),
        ("the first I_a record, then H_0", after_h0, [true, true]),
    ];
    for (name, state, [elsewhere, off]) in &states {
        let counts = against_labels(state, &occupants_of_every_slot(state));
        assert_eq!((counts.other > 0, counts.none > 0, counts.unknown), (*elsewhere, *off, 0), "{name}: {counts:?}");
        assert_eq!(counts.home + counts.other + counts.none, NS, "{name}");
    }
    let centre_state = &states[4].1;
    assert_ne!(centre_state.pose_of(centre as usize).kidx(), Some(0), "the centre is not at the identity");
}

/// CT-EV3-06 (part: "never an old occupant"). Solved, every slot answers its home piece and the result is total. After
/// the first I_a record, the slots of the 3,097 moved pieces answer "none: off the lattice", never the piece that sat
/// there before the twist; the rest answer their home piece. After a further twist on the lattice, H_0 at grip 0, each
/// of those slots answers "none" or a lattice piece that H_0 moved into its position, and never its old occupant,
/// which stays off the lattice. Each result names the pose digest of its own state.
#[test]
fn ct_ev3_06_an_off_lattice_slot_never_answers_its_old_occupant() {
    let ctx = ctx();
    let menu = i_a();
    let solved = State::new(Some(menu.clone()));
    let at_solved = occupants_of_every_slot(&solved);
    assert!(matches!(&at_solved.answer, Partial::Total(covered) if covered.len() == NS));
    assert_eq!(against_labels(&solved, &at_solved), Counts { home: NS, ..Counts::default() });

    let mut state = after(&menu, 1);
    assert_eq!((state.moved_count(), state.off_lattice_count()), (3_097, 3_097), "moved off the lattice only");
    let off_lattice = occupants_of_every_slot(&state);
    let counts = against_labels(&state, &off_lattice);
    let none: Vec<u32> =
        (0..NS as u32).filter(|&slot| off_lattice.get(slot) == Some(Err(Missing::OffLattice))).collect();
    assert_eq!((counts.none, counts.other, counts.unknown), (none.len(), 0, 0));
    for &slot in &none {
        assert_eq!(at_solved.get(slot), Some(Ok(home(slot))), "slot {slot}: its occupant at solved");
        assert!(!state.is_lattice_piece(home(slot) as usize), "slot {slot}: its old occupant is off the lattice");
    }

    play(&mut state, &h0(&menu));
    let on_lattice = occupants_of_every_slot(&state);
    against_labels(&state, &on_lattice);
    for &slot in &none {
        match on_lattice.get(slot) {
            Some(Err(Missing::OffLattice)) => {}
            Some(Ok(piece)) => assert_ne!(piece, home(slot), "slot {slot}: never its old occupant"),
            other => panic!("slot {slot}: {other:?}"),
        }
    }
    let reads = [&at_solved.read, &off_lattice.read, &on_lattice.read];
    assert!(reads[0] != reads[1] && reads[1] != reads[2] && reads[0] != reads[2], "each names its own state");
    assert_eq!(on_lattice.read, state.digest(ctx));
}

/// CT-EV3-06 (part: "a partially defined relation retains its coverage and missing part"). Named alone, missing slots
/// give an undefined result whose reason is the missing part, kept slot by slot: "none: off the lattice" for slots of
/// moved pieces after the first I_a record, `unknown(reason)` for the slots of the two pieces in one chamber of a raw
/// table. With one occupied slot added, the result is partial with that slot covered. Each slot answers as in a request
/// for every slot; an unnamed slot has no answer, no slot is a total result with nothing in it, and a slot outside the
/// model refuses the whole call.
#[test]
fn ct_ev3_06_coverage_and_missing_part_are_kept() {
    let ctx = ctx();
    let menu = i_a();
    let off_lattice = after(&menu, 1);
    let moved: Vec<u32> =
        (0..NS as u32).filter(|&slot| !off_lattice.is_lattice_piece(home(slot) as usize)).step_by(97).collect();
    let (a, b, onto) = congruent();
    let collided = raw(&[(b, Pose::K(onto))]);
    let crowded: Vec<u32> = slots_of(a).into_iter().chain(slots_of(b)).collect();
    let cases = [(&off_lattice, moved, Missing::OffLattice), (&collided, crowded, Missing::Unknown(PIECES_COLLIDE))];
    for (state, named, expected) in cases {
        assert!(named.len() > 1, "slots to name");
        let occupants = occupants_of(state, &named);
        let (_, missing) = split(state, &occupants, &named);
        let Partial::Undefined(reason) = &occupants.answer else {
            panic!("nothing is covered: {:?}", occupants.answer)
        };
        assert!(reason.values().all(|value| *value == expected), "every slot keeps its own reason");
        assert_eq!(missing.len(), named.len());

        let every = occupants_of_every_slot(state);
        let occupied = (0..NS as u32)
            .find(|&slot| !named.contains(&slot) && every.get(slot) == Some(Ok(home(slot))))
            .expect("a slot its home piece fills");
        let mut named = named.clone();
        named.extend([occupied, occupied]);
        let occupants = occupants_of(state, &named);
        let (covered, missing) = split(state, &occupants, &named);
        assert_eq!(covered.into_iter().collect::<Vec<_>>(), vec![(occupied, home(occupied))]);
        assert!(missing.values().all(|value| *value == expected));
        let absent = (0..NS as u32).find(|slot| !named.contains(slot)).expect("an unnamed slot");
        assert_eq!(occupants.get(absent), None, "an unnamed slot has no answer");
    }

    assert_eq!(occupants_of(&off_lattice, &[]).answer, Partial::Total(BTreeMap::new()));
    let refused = current_occupants(ctx, &off_lattice, &[0, NS as u32])
        .expect("no engine error")
        .expect_err("a slot outside the model");
    assert_eq!(refused, NoSuchSlot(NS as u32));
    assert_eq!(refused.code(), codes::NAMED_OBJECT_INVALID);
}
