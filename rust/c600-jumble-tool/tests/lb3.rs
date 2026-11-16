//! The kernel halves of the LB1 and LB3 jumbling tests that `c600_jumble`'s public API reaches (LP3, C8, item 2;
//! definitions in chapter 03's test table). The LB3 library entries are language code the IR draft cannot express
//! yet (it has no list types and no word operations), so each test checks, on the kernel API, the facts the library
//! entry would return: the kernel half; the library entry waits for the IR draft's collections. None of these IDs
//! runs whole here; each test's comment names the part that runs.
//!
//! The states come from the repository's fixtures: the pinned I_a and S4 menus and the first records of the W-J
//! journals (`research/jumbling/fixtures/`), and the first letters of the X1 witness H6030
//! (`research/jumbling/restoration/10-x1-unbounded/witnesses/H6030.json`). A pass is source and fixture evidence.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_digest::sha256_hex;
use c600_jumble::model::NP;
use c600_jumble::observe::{
    cayley_record, heights, prefix_observations, tables, units, word_check, Batch, Fields, PrefixRequest,
};
use c600_jumble::q5::{identity4, matmul, transpose, Mat4, Q5};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_jumble_tool::tool::{tool, CHECK_TWIST, CHECK_WORD};
use c600_jumble_tool::word::{self, Letter, WordError};
use c600_kernel::json::{self, Value};
use c600_run::boundary::Store;
use c600_run::interp::{Host, Request, RowTariff, Sink};

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

/// A W-J fixture: its pinned menu and its journal records.
struct Fixture {
    menu: Arc<Menu>,
    records: Vec<Value>,
}

fn fixture(name: &str) -> Fixture {
    let doc = read(&format!("research/jumbling/fixtures/wj-{name}.json"));
    let menu = Menu::from_record(ctx(), field(&doc, "menu")).expect("the fixture's pinned menu");
    let records = field(field(&doc, "journal"), "records").as_array().expect("records").to_vec();
    Fixture { menu: Arc::new(menu), records }
}

impl Fixture {
    fn solved(&self) -> State {
        State::new(Some(self.menu.clone()))
    }

    /// The state after the first `count` journal records from solved.
    fn after(&self, count: usize) -> State {
        let mut state = self.solved();
        for (index, record) in self.records[..count].iter().enumerate() {
            let twist = Twist::from_record(ctx(), record).expect("a journal record");
            let outcome = state.apply(ctx(), &twist, None, &[]).expect("no engine error");
            assert_eq!(outcome.status, Status::Admissible, "record {index}");
        }
        state
    }

    fn letters(&self, pairs: &[[i32; 2]]) -> Vec<Letter> {
        word::letters(ctx(), &self.menu, &word::word_of(pairs)).expect("letters of the menu")
    }

    /// The item of the inverse rotation.
    fn inverse(&self, item: i32) -> i32 {
        let inverse = transpose(&self.menu.items[item as usize].1);
        self.menu.items.iter().position(|(_, m)| *m == inverse).expect("the menu is inverse-closed") as i32
    }
}

/// The first `count` letters of H6030, a word in the pinned I_a menu.
fn h6030(count: usize) -> Vec<[i32; 2]> {
    let doc = read("research/jumbling/restoration/10-x1-unbounded/witnesses/H6030.json");
    let word = field(&doc, "word").as_array().expect("a word");
    word[..count]
        .iter()
        .map(|pair| {
            let pair = pair.as_array().expect("a letter");
            [pair[0].as_i64().unwrap() as i32, pair[1].as_i64().unwrap() as i32]
        })
        .collect()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or_else(|| panic!("no field `{key}`"))
}

fn int(value: &Value, key: &str) -> i64 {
    field(value, key).as_i64().unwrap_or_else(|| panic!("`{key}` is no integer"))
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    field(value, key).as_str().unwrap_or_else(|| panic!("`{key}` is no string"))
}

/// A Boolean field, or `None` for null.
fn flag(value: &Value, key: &str) -> Option<bool> {
    match field(value, key) {
        Value::Bool(b) => Some(*b),
        Value::Null => None,
        other => panic!("`{key}` is {}", other.canonical()),
    }
}

fn is_null(value: &Value, key: &str) -> bool {
    matches!(field(value, key), Value::Null)
}

/// JI.10's prefix observations of a literal word, with the prefix states kept.
fn observed(start: &State, letters: &[Letter]) -> (Value, Vec<State>) {
    let request = PrefixRequest {
        letters,
        fields: Fields { n: false, heights: false, fd: false, sets: false },
        cap: None,
        batch: Batch { chunk: 16, budget: None, cancel_after: None },
        keep: true,
        eval_budget: None,
    };
    prefix_observations(ctx(), start, &request).expect("prefix observations")
}

fn checked(start: &State, letters: &[Letter], unit: Option<&[u32]>, cap: Option<usize>) -> Value {
    word_check(ctx(), start, letters, unit, cap, None).expect("a word check")
}

/// Applies one literal letter with no protection and returns its status.
fn play(state: &mut State, letter: &Letter) -> Status {
    state.apply(ctx(), &letter.1, None, &[]).expect("no engine error").status
}

/// CT-LB1-02 (part: the kernel half of "at a blocked cap, literal a,a⁻¹ is refused; empty is admitted" and of "prefix
/// protection still reads literal letters"; the library entry waits for the IR draft's collections. Not run: the
/// domain-preserving key and "keys preserve the end digest", which wait for an owner answer on LG2.18's word keys).
///
/// At the state after 12 records of the I_a journal, cap `blocked` is blocked. The literal word a,a⁻¹ at that cap
/// composes to the identity rotation, and the kernel still refuses it at its first letter, with a straddling
/// certificate and an unchanged state: the whole-word check, the prefix observations and the letter itself agree. The
/// empty word is admitted there and ends at the start. At an open cap the same literal word is admitted and returns to
/// the start; with a piece of that cap protected, its first literal letter is refused at pose level, although the
/// composed word leaves the piece's pose unchanged.
#[test]
fn ct_lb1_02_a_blocked_cap_refuses_literal_a_a_inverse_and_admits_the_empty_word() {
    let ctx = ctx();
    let fixture = fixture("I_a");
    let state = fixture.after(12);
    let start = state.digest(ctx);
    let grips: Vec<usize> = (0..12).collect();
    let statuses = state.clone().survey(ctx, &grips, None).expect("a survey");
    let blocked = grips.iter().copied().find(|&g| statuses[g] == Status::Blocked).expect("a blocked cap") as i32;
    let open = grips.iter().copied().find(|&g| statuses[g] == Status::Admissible).expect("an open cap") as i32;
    let a = fixture.menu.items.iter().position(|(_, m)| *m != identity4()).expect("a rotation") as i32;
    let inverse = fixture.inverse(a);

    // The blocked cap: the literal word composes to the identity and is refused at letter 0.
    let literal = fixture.letters(&[[blocked, a], [blocked, inverse]]);
    assert_eq!(matmul(&literal[1].1.matrix, &literal[0].1.matrix), identity4(), "a,a⁻¹ composes to the identity");
    let check = checked(&state, &literal, None, None);
    assert_eq!((flag(&check, "admissible"), int(&check, "failing_letter_index")), (Some(false), 0));
    assert_eq!((text(&check, "failing_status"), int(&check, "applied_letters")), ("blocked", 0));
    assert_eq!(text(&check, "end_digest"), start);
    for claim in ["peak_left_height", "excursion", "tame"] {
        assert!(is_null(&check, claim), "a refused word carries no {claim}");
    }
    let (prefixes, kept) = observed(&state, &literal);
    assert_eq!((int(&prefixes, "first_refused"), flag(&prefixes, "complete")), (0, Some(false)));
    assert!(is_null(&prefixes, "final") && kept.is_empty());
    let rows = field(&prefixes, "rows").as_array().expect("rows");
    assert_eq!(rows.len(), 1, "the batch stops at the refused letter");
    assert_eq!((text(&rows[0], "status"), text(&rows[0], "after")), ("blocked", start.as_str()));
    assert!(!is_null(&rows[0], "certificate"), "a blocked letter carries its straddling certificate");
    let mut work = state.clone();
    let outcome = work.apply(ctx, &literal[0].1, None, &[]).expect("no engine error");
    assert_eq!((outcome.status, outcome.straddle.is_some(), work.digest(ctx)), (Status::Blocked, true, start.clone()));

    // The empty word is admitted at the same state.
    let empty = fixture.letters(&[]);
    assert!(empty.is_empty());
    let check = checked(&state, &empty, None, None);
    assert_eq!((flag(&check, "admissible"), int(&check, "applied_letters")), (Some(true), 0));
    assert!(is_null(&check, "failing_letter_index"));
    assert_eq!((text(&check, "end_digest"), flag(&check, "tame"), int(&check, "excursion")), (&*start, Some(true), 0));
    let (prefixes, _) = observed(&state, &empty);
    assert_eq!(flag(&prefixes, "complete"), Some(true));
    assert!(is_null(&prefixes, "first_refused"));
    assert_eq!(text(field(&prefixes, "final"), "end_digest"), start);

    // An open cap: the same literal word is admitted and returns to the start.
    let literal = fixture.letters(&[[open, a], [open, inverse]]);
    let check = checked(&state, &literal, None, None);
    assert_eq!((flag(&check, "admissible"), int(&check, "applied_letters")), (Some(true), 2));
    assert_eq!(text(&check, "end_digest"), start);

    // Prefix protection reads the literal letters: the first letter would move a protected piece and is refused,
    // although the composed word leaves that piece's pose unchanged.
    let mut unprotected = state.clone();
    let moved = unprotected.apply(ctx, &literal[0].1, None, &[]).expect("no engine error");
    assert_eq!(moved.status, Status::Admissible);
    let piece = moved.inside[0];
    assert_ne!(unprotected.pose_of(piece as usize), state.pose_of(piece as usize), "the letter moves the piece");
    assert_eq!(play(&mut unprotected, &literal[1]), Status::Admissible);
    assert_eq!(unprotected.digest(ctx), start);
    assert_eq!(unprotected.pose_of(piece as usize), state.pose_of(piece as usize), "the composed word does not");
    let mut protected = state.clone();
    let outcome = protected.apply(ctx, &literal[0].1, None, &[piece]).expect("no engine error");
    assert_eq!(outcome.status, Status::Protected);
    assert_eq!(outcome.refusal.as_ref().map(|refusal| refusal.0), Some(codes::PIECE_WOULD_MOVE));
    assert_eq!(outcome.protection.expect("a protection report").pieces, [piece]);
    assert_eq!(protected.digest(ctx), start, "a refusal changes nothing");
}

/// The `[left, right]` heights of a row of JI.6's pose table.
fn heights_of(row: &Value) -> [i64; 2] {
    let pair = field(row, "heights").as_array().expect("heights");
    [pair[0].as_i64().unwrap(), pair[1].as_i64().unwrap()]
}

/// The exact Cayley rotation with parameter `omega` at cap 0, as a free-mode twist.
fn cayley(omega: [Q5; 3]) -> Twist {
    Twist::from_record(ctx(), &cayley_record(ctx(), 0, &omega)).expect("an exact rotation fixing the pole")
}

/// CT-LB3-03 (part: the kernel half of "in free mode, heights (0,0) do not imply the lattice, and domain IDs come from
/// exact coset tests", with LB3.1's "a lattice test by heights holds only on Γ(Λ)" on two menu-reached states; the
/// library entry waits for the IR draft's collections. Not run here: the agreement of heights and domain IDs with
/// `trees.py` on ENG-0J item 8's states and H6030, which M5's `tests/test_jumbling_rust_candidate.py` runs on the same
/// kernel API (CT-KN2-02: `assert_heights`, called by `FixtureComparisonTests` and `H6030Tests`) and this test does not
/// repeat; and the sticker-test sentence, which waits for M8).
///
/// On the states after 12 records of the I_a and S4 journals, a pose has heights (0,0) exactly when its domain ID is
/// the lattice's, and the pieces at heights (0,0) are exactly the pieces on the lattice. In free mode the Cayley
/// rotation ω = (1/2, 0, 0) at cap 0 (a rotation by the angle of the 3-4-5 triangle) reads heights (0,0) and lies
/// outside K⁺: every piece then reads heights (0,0) while the moved pieces are off the lattice, and only the domain
/// ID tells them apart. Over a set of exact rotations inside and outside K⁺, two have the same domain ID exactly when
/// A⁻¹B lies in K⁺, decided by the kernel's exact membership test.
#[test]
fn ct_lb3_03_heights_zero_do_not_imply_the_lattice_in_free_mode() {
    let ctx = ctx();
    let t = tables(ctx).expect("the observation tables");

    // On Γ(Λ) the lattice test by heights holds.
    for name in ["I_a", "S4"] {
        let state = fixture(name).after(12);
        let facts = heights(ctx, &state).expect("heights");
        let lattice = text(&facts, "lattice_key");
        assert_eq!(lattice, t.lattice_key());
        let mut at_zero = 0;
        let poses = field(&facts, "poses").as_object().expect("poses");
        assert!(poses.len() > 2, "{name}: several pose classes");
        for row in poses.values() {
            let zero = heights_of(row) == [0, 0];
            assert_eq!(zero, text(row, "domain") == lattice, "{name}: heights (0,0) exactly on the lattice domain");
            at_zero += if zero { int(row, "count") } else { 0 };
        }
        let off = state.off_lattice_count() as i64;
        assert!(off > 0, "{name}: pieces off the lattice");
        assert_eq!((int(&facts, "off"), at_zero), (off, NP as i64 - off), "{name}");
    }

    // Free mode: heights (0,0) off the lattice.
    let half = || Q5::of(1, 0, 2);
    let twist = cayley([half(), Q5::zero(), Q5::zero()]);
    assert_eq!(twist.kidx, None, "the rotation lies outside K⁺");
    let m: &Mat4 = &twist.matrix;
    let (left, right, _, _) = t.tree_facts(m).expect("tree facts");
    assert_eq!((left, right), (0, 0));
    assert_ne!(t.domain_id(m), t.lattice_key());
    let mut free = State::new(None);
    let outcome = free.apply(ctx, &twist, None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
    let moved = outcome.moved() as i64;
    assert!(moved > 0);
    let facts = heights(ctx, &free).expect("heights");
    let poses = field(&facts, "poses").as_object().expect("poses");
    assert!(poses.values().all(|row| heights_of(row) == [0, 0]), "every piece reads heights (0,0)");
    assert_eq!(poses.values().map(|row| int(row, "count")).sum::<i64>(), NP as i64);
    let off: Vec<&Value> = poses.values().filter(|row| text(row, "domain") != t.lattice_key()).collect();
    assert_eq!(off.len(), 1, "one pose class off the lattice");
    assert_eq!((int(off[0], "count"), int(&facts, "off")), (moved, moved));
    assert_eq!(free.off_lattice_count() as i64, moved);
    assert_eq!(text(off[0], "domain"), t.domain_id(m));

    // Domain IDs are exact coset tests: equal exactly when A⁻¹B lies in K⁺.
    let k = |index: u16| ctx.kplus.matrix(index, &ctx.model).clone();
    let other = cayley([Q5::zero(), half(), Q5::zero()]);
    let m2: &Mat4 = &other.matrix;
    let set =
        [identity4(), k(1), m.clone(), matmul(m, &k(5)), matmul(&k(3), m), m2.clone(), matmul(m, m), matmul(m2, &k(9))];
    let ids: Vec<String> = set.iter().map(|rotation| t.domain_id(rotation)).collect();
    assert_eq!((ids[0].as_str(), ids[1].as_str()), (t.lattice_key(), t.lattice_key()));
    assert_eq!(ids[2], ids[3], "M and M·k share a coset");
    let mut differing = 0;
    for (i, a) in set.iter().enumerate() {
        for (j, b) in set.iter().enumerate() {
            let same_coset = ctx.kplus.index_of_matrix(&matmul(&transpose(a), b), &ctx.model).is_some();
            assert_eq!(ids[i] == ids[j], same_coset, "rotations {i} and {j}");
            differing += usize::from(!same_coset);
        }
    }
    assert!(differing > 0, "the set meets more than one coset");
}

/// Φ = Σ max(0, h − 2) over the left heights and the largest left height, summed from JI.6's per-pose table.
fn phi_and_peak(state: &State) -> (i64, i64) {
    let facts = heights(ctx(), state).expect("heights");
    let (mut phi, mut peak) = (0, 0);
    for row in field(&facts, "poses").as_object().expect("poses").values() {
        let left = heights_of(row)[0];
        phi += int(row, "count") * (left - 2).max(0);
        peak = peak.max(left);
    }
    (phi, peak)
}

/// LB3.7's chronological observations of an admissible literal word, recomputed apart from the kernel's batch: each
/// letter is applied by itself and Φ and the peak are summed from the per-pose height table of every prefix state.
/// Returns the peak (start included), the excursion, tameness and the end state.
fn recomputed(start: &State, letters: &[Letter]) -> (i64, i64, bool, State) {
    let mut work = start.clone();
    let (start_phi, mut peak) = phi_and_peak(&work);
    let (mut top, mut previous, mut tame) = (start_phi, start_phi, true);
    for (index, letter) in letters.iter().enumerate() {
        assert_eq!(play(&mut work, letter), Status::Admissible, "letter {index}");
        let (phi, max) = phi_and_peak(&work);
        tame &= phi <= previous;
        previous = phi;
        top = top.max(phi);
        peak = peak.max(max);
    }
    (peak, top - start_phi, tame, work)
}

/// LB3.8's step flag from the start and end states: the word is admissible, the unit lands, no lattice piece is
/// lifted, and every other off piece lands or keeps its exact pose.
fn step_flag(start: &State, end: &State, unit: &[u32], admissible: bool) -> bool {
    let unit: HashSet<usize> = unit.iter().map(|&p| p as usize).collect();
    let lands = unit.iter().all(|&p| end.is_lattice_piece(p));
    let lifted = (0..NP).any(|p| start.is_lattice_piece(p) && !end.is_lattice_piece(p));
    let others_keep = (0..NP)
        .filter(|p| !start.is_lattice_piece(*p) && !unit.contains(p))
        .all(|p| end.is_lattice_piece(p) || end.pose_of(p) == start.pose_of(p));
    admissible && lands && !lifted && others_keep
}

/// CT-LB3-06 (part: the kernel half, on exact word fixtures in the pinned I_a menu; the library entry waits for the IR
/// draft's collections. Not run: Net and Strict protection of a whole word, which M5 left to milestone M8, so "no
/// protection conflict" is a run with no protected piece).
///
/// Prefix peak, excursion and tameness of three literal words are recomputed apart from the kernel's batch and equal
/// both the prefix observations' final claims and the whole-word check's: the first 35 letters of H6030 from solved
/// (the last letter raises Φ, so the word has a positive excursion and is not tame); from its end, the last letter's
/// inverse and the letter again (zero excursion, not tame, and the end is the start: the endpoint alone certifies
/// neither); and the inverse alone (tame). The step flag is recomputed from the start and end states for a named unit
/// on two words: one that lands the unit (true) and one that is admissible and lifts lattice pieces (false). The false
/// flag refuses nothing: the check reports the word admissible, its prefix observations complete, and each literal
/// letter applies.
///
/// What "separately" covers: the recomputation shares the kernel's per-pose height table (`heights`), its lattice test
/// (`State::is_lattice_piece`) and `State::apply`; the sums, maxima, tameness and the step flag's conditions are
/// computed here. The heights themselves are checked against `trees.py` by M5's `assert_heights` in
/// `tests/test_jumbling_rust_candidate.py`.
#[test]
fn ct_lb3_06_peak_excursion_tameness_and_the_step_flag_are_recomputed_separately() {
    let ctx = ctx();
    let fixture = fixture("I_a");
    let pairs = h6030(35);
    let [cap, item] = pairs[34];
    let back = [cap, fixture.inverse(item)];
    let rising = fixture.letters(&pairs);
    let raised = recomputed(&fixture.solved(), &rising).3;
    let words: [(&str, State, Vec<Letter>, (bool, bool)); 3] = [
        ("H6030[..35] from solved", fixture.solved(), rising, (true, false)),
        ("a⁻¹,a at the raised state", raised.clone(), fixture.letters(&[back, [cap, item]]), (false, false)),
        ("a⁻¹ at the raised state", raised.clone(), fixture.letters(&[back]), (false, true)),
    ];
    for (name, start, letters, (rises, tame_expected)) in &words {
        let (peak, excursion, tame, end) = recomputed(start, letters);
        assert_eq!((excursion > 0, tame), (*rises, *tame_expected), "{name}");
        let end = end.digest(ctx);
        let (prefixes, kept) = observed(start, letters);
        assert_eq!((flag(&prefixes, "complete"), kept.len()), (Some(true), letters.len()), "{name}");
        let claims = field(&prefixes, "final");
        assert_eq!(
            (
                int(claims, "peak_left_height"),
                int(claims, "excursion"),
                flag(claims, "tame"),
                text(claims, "end_digest")
            ),
            (peak, excursion, Some(tame), end.as_str()),
            "{name}: the prefix observations' final claims"
        );
        let check = checked(start, letters, None, None);
        assert_eq!(
            (
                int(&check, "peak_left_height"),
                int(&check, "excursion"),
                flag(&check, "tame"),
                text(&check, "end_digest")
            ),
            (peak, excursion, Some(tame), end.as_str()),
            "{name}: the whole-word check"
        );
        assert!(is_null(&check, "step_flag"), "{name}: no unit is named, so the flag is undefined");
    }
    let (peak, _, _, _) = recomputed(&words[1].1, &words[1].2);
    assert!(peak > 2, "the raised state holds pieces above height 2");
    assert_eq!(recomputed(&words[1].1, &words[1].2).3.digest(ctx), raised.digest(ctx), "a⁻¹,a ends at its start");

    // The step flag. From solved, letter (0, a) lifts one unit; a letter at another cap, disjoint from it, lifts a
    // second group. The named unit is the first.
    let a = fixture.menu.for_cap(ctx, 0).expect("the menu at cap 0").iter().position(|t| t.kidx.is_none());
    let a = a.expect("a letter outside K⁺") as i32;
    let mut start = fixture.solved();
    let first = start.apply(ctx, &fixture.letters(&[[0, a]])[0].1, None, &[]).expect("no engine error");
    assert_eq!(first.status, Status::Admissible);
    let unit = first.inside.clone();
    let lands = fixture.letters(&[[0, fixture.inverse(a)]]);
    let mut second = None;
    for cap in 1..600 {
        let mut work = start.clone();
        let outcome = work.apply(ctx, &fixture.letters(&[[cap, a]])[0].1, None, &[]).expect("no engine error");
        let disjoint = outcome.inside.iter().all(|piece| unit.binary_search(piece).is_err());
        if outcome.status == Status::Admissible && disjoint && play(&mut work.clone(), &lands[0]) == Status::Admissible
        {
            second = Some((cap, work));
            break;
        }
    }
    let (second_cap, with_others) = second.expect("a second cap disjoint from the unit");
    start = with_others;
    assert!(start.off_lattice_count() > unit.len(), "other pieces are off the lattice");
    let lifts = ((second_cap + 1)..600)
        .map(|cap| fixture.letters(&[[cap, a]]))
        .find(|letters| {
            let mut work = start.clone();
            play(&mut work, &letters[0]) == Status::Admissible && work.off_lattice_count() > start.off_lattice_count()
        })
        .expect("an admissible letter that lifts lattice pieces");
    for (name, letters, expected) in [("the unit lands", &lands, true), ("lattice pieces are lifted", &lifts, false)] {
        let (_, _, _, end) = recomputed(&start, letters);
        let check = checked(&start, letters, Some(&unit), None);
        assert_eq!(text(&check, "status"), "ok", "{name}");
        assert_eq!(flag(&check, "admissible"), Some(true), "{name}");
        assert!(is_null(&check, "failing_letter_index"), "{name}");
        assert_eq!(flag(&check, "step_flag"), Some(step_flag(&start, &end, &unit, true)), "{name}");
        assert_eq!(flag(&check, "step_flag"), Some(expected), "{name}");
        assert_eq!(text(&check, "end_digest"), end.digest(ctx), "{name}: each literal letter applied");
        let (prefixes, _) = observed(&start, letters);
        assert_eq!(flag(&prefixes, "complete"), Some(true), "{name}");
        assert!(is_null(&prefixes, "first_refused"), "{name}: a false flag refuses no letter");
    }
    let check = checked(&start, &lifts, Some(&unit), None);
    assert!(int(&check, "lattice_pieces_lifted_off") > 0 && flag(&check, "unit_landed") == Some(false));
}

/// The engine side of a run of the tool: each proposal is read as a literal jumbling word and checked on the kernel
/// from the host's state, which no check changes.
struct Checker {
    state: State,
    menu: Arc<Menu>,
    proposed: Vec<Vec<i32>>,
    checks: Vec<Result<Value, WordError>>,
}

impl Host for Checker {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 1, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<c600_run::value::Value, &'static str> {
        match request {
            Request::Propose { word, .. } => {
                let word = word.to_vec();
                let check = word::letters(ctx(), &self.menu, &word)
                    .map(|letters| checked(&self.state, &letters, None, word.first().map(|&cap| cap as usize)));
                self.proposed.push(word);
                self.checks.push(check);
                Ok(c600_run::value::Value::receipt(vec![("id", c600_run::value::Value::text("preview-1"))]))
            }
            _ => Err("the jumbling tool reads no state"),
        }
    }

    fn sink(&mut self, _: &Sink, _: &c600_run::value::Value, _: &[u8]) -> Result<(), &'static str> {
        Err("the jumbling tool writes nothing")
    }
}

/// Every key of an answer, at any depth, apart from the pose keys of JI.6's pose table.
fn keys(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "poses" {
                    out.insert(key.clone());
                    for row in child.as_object().into_iter().flat_map(|rows| rows.values()) {
                        keys(row, out);
                    }
                } else {
                    out.insert(key.clone());
                    keys(child, out);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys(item, out)),
        _ => {}
    }
}

/// Whether a value holds a list of lists: the shape of a word, a list of letters or a table of outcomes.
fn holds_a_table(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(|item| !matches!(item, Value::Int(_))),
        Value::Object(map) => map.values().any(holds_a_table),
        _ => false,
    }
}

/// CT-LB3-07 (part: the jumbling half, on the kernel: "named-word checks answer only their named word; no
/// recommendation, next-cap skip or correction combination leaks", checked on the whole-word check the tool's entries
/// reach and on the per-revision height and unit facts; the inspector library entries wait for the IR draft's
/// collections. Not run: the per-revision rim and survey rows' "no all-items outcome table", and the correction
/// catalogue, which M5 left to milestone M8).
///
/// A run of each entry proposes the word the solver named once, as given; read as literal letters, the kernel's check
/// answers for that word: its digest and length are the named word's, the cap is the named cap (cap 0 for the word,
/// cap 7 for the twist), and the state is unchanged. The answer holds scalars, piece-ID lists and height counts only:
/// no word, no letter list and no table of outcomes, and no field's name is a recommendation, a next cap, a ranking
/// or a correction (a check of names, not of values). A refused word and the empty word have the same fields: a
/// refusal adds nothing, and with no cap named the cap and u fields are null. A word that is no sequence of letters is
/// not checked.
#[test]
fn ct_lb3_07_a_named_word_check_answers_only_its_named_word() {
    let ctx = ctx();
    let fixture = fixture("I_a");
    let outside = |cap: usize| {
        let twists = fixture.menu.for_cap(ctx, cap).expect("the menu at the cap");
        twists.iter().position(|t| t.kidx.is_none()).expect("a letter outside K⁺") as i32
    };
    let a = outside(0);
    let named = [[0, a], [0, fixture.inverse(a)]];
    let twist = [[7, outside(7)]];
    let start = fixture.solved();
    let digest = start.digest(ctx);
    let mut answers = Vec::new();
    for (entry, pairs) in [(CHECK_WORD, &named[..]), (CHECK_TWIST, &twist[..])] {
        let word = word::word_of(pairs);
        let mut host =
            Checker { state: start.clone(), menu: fixture.menu.clone(), proposed: Vec::new(), checks: Vec::new() };
        let ran = tool(entry).expect("the entry loads").run(&word, "c8", "base", &mut host, &mut Store::default());
        assert_eq!(ran.expect("the run starts").shell.len(), 1, "{entry}: one proposal");
        assert_eq!(host.proposed, [word.clone()], "{entry}: the named word, once, as given");
        assert_eq!(host.state.digest(ctx), digest, "{entry}: a check changes nothing");
        let check = host.checks.pop().expect("one check").expect("the word is letters of the menu");
        let compact: Vec<String> = pairs.iter().map(|[cap, item]| format!("[{cap},{item}]")).collect();
        assert_eq!(text(&check, "word_digest"), sha256_hex(format!("[{}]", compact.join(",")).as_bytes()), "{entry}");
        assert_eq!(
            (int(&check, "letters"), int(&check, "cap")),
            (pairs.len() as i64, i64::from(pairs[0][0])),
            "{entry}"
        );
        assert_eq!((text(&check, "start_digest"), flag(&check, "admissible")), (&*digest, Some(true)), "{entry}");
        answers.push(check);
    }
    assert_eq!(text(&answers[0], "end_digest"), digest, "the named word a,a⁻¹ ends at solved");
    assert_ne!(text(&answers[1], "end_digest"), digest, "the named twist at cap 7 does not");

    // A refused word and the empty word have the same fields, and no answer holds a word or an outcome table.
    let state = fixture.after(12);
    let grips: Vec<usize> = (0..12).collect();
    let statuses = state.clone().survey(ctx, &grips, None).expect("a survey");
    let blocked = grips.iter().copied().find(|&g| statuses[g] == Status::Blocked).expect("a blocked cap") as i32;
    let refused = checked(&state, &fixture.letters(&[[blocked, a]]), None, None);
    assert_eq!((flag(&refused, "admissible"), int(&refused, "failing_letter_index")), (Some(false), 0));
    let empty = checked(&state, &[], None, None);
    for name in ["cap", "u_before", "u_after", "delta_u"] {
        assert!(is_null(&refused, name) && is_null(&empty, name), "no cap is named, so `{name}` is null");
    }
    answers.extend([refused, empty]);
    let fields = |value: &Value| {
        let mut out = BTreeSet::new();
        keys(value, &mut out);
        out
    };
    let facts = [heights(ctx, &state).expect("heights"), units(ctx, &state, None).expect("units")];
    for answer in &answers {
        assert_eq!(fields(answer), fields(&answers[0]), "every answer has the same fields");
        let lists = answer.as_object().expect("an answer").iter().filter(|(_, value)| holds_a_table(value));
        let lists: Vec<&str> = lists.map(|(key, _)| key.as_str()).collect();
        assert!(lists.is_empty(), "fields {lists:?} hold a list of lists");
    }
    for value in answers.iter().chain(&facts) {
        for key in fields(value) {
            for leak in ["recommend", "suggest", "next", "skip", "best", "rank", "correction", "repair", "target"] {
                assert!(!key.contains(leak), "field `{key}`");
            }
        }
    }

    // A word that is no sequence of letters of the menu is not checked.
    let items = fixture.menu.items.len();
    assert_eq!(word::letters(ctx, &fixture.menu, &[0]).err(), Some(WordError::Unpaired { integers: 1 }));
    assert_eq!(word::letters(ctx, &fixture.menu, &[600, 0]).err(), Some(WordError::Cap { index: 0, cap: 600 }));
    let off_menu = word::letters(ctx, &fixture.menu, &[0, a, 0, items as i32]).err();
    assert_eq!(off_menu, Some(WordError::Item { index: 1, item: items as i32, items }));
    assert_eq!(
        word::twist(ctx, &fixture.menu, &word::word_of(&named)).err(),
        Some(WordError::NotOneLetter { letters: 2 })
    );
    assert_eq!(word::twist(ctx, &fixture.menu, &[0, a]).expect("one letter").0, [0, i64::from(a)]);
}
