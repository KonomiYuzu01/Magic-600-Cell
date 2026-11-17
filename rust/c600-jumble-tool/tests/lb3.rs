//! The kernel halves of the LB1 and LB3 jumbling tests that `c600_jumble`'s public API reaches (LP3, C8, item 2;
//! definitions in chapter 03's test table), with CT-BND-17 beside CT-LB3-05. The LB3 library entries are language
//! code the IR draft cannot express yet (it has no list types and no word operations), so each test checks, on the
//! kernel API, the facts the library entry would return: the kernel half; the library entry waits for the IR draft's
//! collections. None of these IDs runs whole here; each test's comment names the part that runs. Milestone M8's
//! `levels` end tests and `observe::cuts::repair_list` carry CT-LB3-01, the sticker-test sentence of CT-LB3-03, and
//! CT-LB3-05 with CT-BND-17.
//!
//! The states come from the repository's fixtures: the pinned I_a and S4 menus and the first records of the W-J
//! journals (`research/jumbling/fixtures/`), the first letters of the X1 witness H6030
//! (`research/jumbling/restoration/10-x1-unbounded/witnesses/H6030.json`), retained words, and raw pose tables that
//! `levels::state_of` and `levels::import` read. A pass is source and fixture evidence.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_digest::sha256_hex;
use c600_jumble::levels::{self, ReachKind, Reachability, Verdict, Witness};
use c600_jumble::model::NP;
use c600_jumble::observe::cuts::{release_check, repair_list, rim_summary};
use c600_jumble::observe::{
    cayley_record, heights, prefix_observations, tables, units, word_check, Batch, Fields, PrefixRequest,
};
use c600_jumble::q5::{identity4, matmul, transpose, Mat4, Q5};
use c600_jumble::state::{Pose, State, Status};
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
    let unit: Option<Vec<i64>> = unit.map(|pieces| pieces.iter().map(|&p| i64::from(p)).collect());
    word_check(ctx(), start, letters, unit.as_deref(), cap.map(|c| c as i64), None).expect("a word check")
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
    // JI.11: no end-state field covers a refused prefix.
    assert!(is_null(&check, "end_digest") && text(&check, "end_state_undefined") == "refused_prefix");
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

/// A5's three end tests on one state: the sticker test, the retained checkpoint with `word` as its witness, and the
/// label checkpoint under `claim`, with ν supplied or not.
fn end_tests(state: &State, word: &[i64], claim: &Reachability, nu: Option<&[i64]>) -> [Verdict; 3] {
    let ctx = ctx();
    [
        levels::sticker_test(ctx, state).expect("no engine error"),
        levels::retained_checkpoint(ctx, state, Witness::Word(word)).expect("no engine error"),
        levels::label_checkpoint(ctx, state, claim, nu).expect("no engine error"),
    ]
}

/// A text field of a verdict's evidence.
fn evidence<'a>(verdict: &'a Verdict, key: &str) -> Option<&'a str> {
    match verdict {
        Verdict::Yes(evidence) | Verdict::No(evidence) => evidence.get(key).and_then(Value::as_str),
        Verdict::Unknown(_) => None,
    }
}

/// CT-LB3-01 (part: the kernel half of "separate fixtures distinguish retained checkpoint, label checkpoint and sticker
/// test" and of "an imported table without reachability stays unestablished", on M8's `levels` end tests; the library
/// entry waits for the IR draft's collections. Not run: "a reachable nonzero-ν lattice state fails the label
/// checkpoint". M8 computes no ν: the label checkpoint reads ν from the identity labels, a retained witness or the
/// caller, and no reachable state with a nonzero ν can be built before LP2's F2 and F5).
///
/// Ten fixtures in the pinned I_a menu, each read by the sticker test, the retained checkpoint and the label checkpoint:
/// solved under its witnessed claim (yes, yes, yes); a raw table with a centre turned by a third-turn about its own
/// pole, under an open claim (yes, unknown, unknown); the retained word [1, 3, −5] under the claim its journal
/// witnesses (no, yes, yes) and under an open claim with ν = 0 supplied (no, yes, unknown); the solved table imported
/// with an open claim, with ν = 0 supplied or not, and with a promised claim (yes, yes, unknown); the retained word's
/// table imported with an open claim and ν = 0 (no, yes, unknown); and the state after the first I_a record, off the
/// lattice (no, no, no). Each pair of the three tests answers differently on some fixture. An imported table stays
/// unestablished at the label checkpoint, at ν = 0 too and with the poses of a retained word, and it cannot enter
/// with a witnessed claim. A nonzero ν that the caller supplies makes the label checkpoint answer no on the retained
/// word's state under an open claim (no, yes, no), with the supplied ν as its source: a known nonzero ν, not the
/// reachable state above. A supplied ν that the identity labels contradict is an engine error, not a verdict.
#[test]
fn ct_lb3_01_separate_fixtures_distinguish_the_three_end_tests() {
    let ctx = ctx();
    let fixture = fixture("I_a");
    let menu = fixture.menu.clone();
    let identity = ctx.identity(Some(&*menu));
    let stated = |kind| Reachability::stated(kind, Value::Null).expect("a stated claim");
    let zero = vec![0i64; levels::NU_LEN];
    let mut one = zero.clone();
    one[0] = 1;
    let (zero, one) = (zero.as_slice(), one.as_slice());

    let solved = fixture.solved();
    let solved_claim = Reachability::solved(ctx);
    let word = [1, 3, -5];
    let twists: Vec<Twist> = word.iter().map(|&id| levels::primitive(ctx, id).expect("a retained generator")).collect();
    let mut retained = fixture.solved();
    for twist in &twists {
        assert_eq!(retained.apply(ctx, twist, None, &[]).expect("no engine error").status, Status::Admissible);
    }
    let witnessed = levels::witness(ctx, &retained, Some(menu.clone()), &twists, None)
        .expect("no engine error")
        .expect("the journal witnesses the state");
    let kp = &ctx.kplus;
    let third = kp.a4(0).into_iter().find(|&k| k != 0 && kp.compose(k, kp.compose(k, k)) == 0).expect("a third-turn");
    let centre = *ctx.model.centre_pieces.iter().find(|&&p| ctx.model.signature(p as usize) == [0]).expect("a centre");
    let mut poses = vec![Pose::K(0); NP];
    poses[centre as usize] = Pose::K(third);
    let turned = levels::state_of(ctx, Some(menu.clone()), &poses).expect("the raw table reads");
    let import = |state: &State, kind| {
        levels::import(ctx, Some(menu.clone()), &state.record(ctx), &identity, stated(kind))
            .expect("the table reads")
            .expect("the identity agrees")
    };
    let solved_open = import(&solved, ReachKind::Open);
    let solved_promised = import(&solved, ReachKind::Promised);
    let retained_open = import(&retained, ReachKind::Open);
    let off = fixture.after(1);
    let open = stated(ReachKind::Open);
    let empty: &[i64] = &[];
    let word: &[i64] = &word;

    type Row<'a> = (&'a str, &'a State, &'a [i64], &'a Reachability, Option<&'a [i64]>, [&'a str; 3]);
    let rows: [Row<'_>; 10] = [
        ("solved, witnessed", &solved, empty, &solved_claim, None, ["yes", "yes", "yes"]),
        ("a turned centre, open", &turned, empty, &open, None, ["yes", "unknown", "unknown"]),
        ("the retained word, witnessed by its journal", &retained, word, &witnessed, None, ["no", "yes", "yes"]),
        ("the retained word, open, ν = 0", &retained, word, &open, Some(zero), ["no", "yes", "unknown"]),
        (
            "the solved table imported, open",
            solved_open.state(),
            empty,
            solved_open.reachability(),
            None,
            ["yes", "yes", "unknown"],
        ),
        (
            "the solved table imported, open, ν = 0",
            solved_open.state(),
            empty,
            solved_open.reachability(),
            Some(zero),
            ["yes", "yes", "unknown"],
        ),
        (
            "the solved table imported, promised, ν = 0",
            solved_promised.state(),
            empty,
            solved_promised.reachability(),
            Some(zero),
            ["yes", "yes", "unknown"],
        ),
        (
            "the retained table imported, open, ν = 0",
            retained_open.state(),
            word,
            retained_open.reachability(),
            Some(zero),
            ["no", "yes", "unknown"],
        ),
        ("the retained word, open, a nonzero ν supplied", &retained, word, &open, Some(one), ["no", "yes", "no"]),
        ("off the lattice after the first record", &off, empty, &open, None, ["no", "no", "no"]),
    ];
    for (name, state, witness, claim, nu, expected) in &rows {
        let verdicts = end_tests(state, witness, claim, *nu);
        assert_eq!(verdicts.each_ref().map(|verdict| verdict.name()), *expected, "{name}: {verdicts:?}");
    }
    let tests = ["the sticker test", "the retained checkpoint", "the label checkpoint"];
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        assert!(rows.iter().any(|row| row.5[i] != row.5[j]), "{} and {} always agree", tests[i], tests[j]);
    }

    // What each verdict rests on.
    let [sticker, _, label] = end_tests(&retained, word, &witnessed, None);
    assert_eq!(
        (evidence(&sticker, "fact"), evidence(&label, "nu_source")),
        (Some("labels_differ"), Some("retained_witness"))
    );
    let [_, retained_yes, nonzero] = end_tests(&retained, word, &open, Some(one));
    assert_eq!(evidence(&retained_yes, "pose_digest"), Some(retained.digest(ctx).as_str()));
    assert_eq!((evidence(&nonzero, "fact"), evidence(&nonzero, "nu_source")), (Some("nu_nonzero"), Some("supplied")));
    let [off_sticker, off_retained, off_label] = end_tests(&off, empty, &open, None);
    for verdict in [&off_sticker, &off_retained, &off_label] {
        assert_eq!(evidence(verdict, "fact"), Some("off_lattice"), "{verdict:?}");
    }

    // An imported table has no derivation and never enters with a witnessed claim, which is never stated either.
    assert!(solved_open.derivation().is_none() && retained_open.derivation().is_none());
    let witnessed_import =
        levels::import(ctx, Some(menu.clone()), &solved.record(ctx), &identity, Reachability::solved(ctx));
    assert!(witnessed_import.is_err(), "a raw table never enters witnessed");
    assert!(Reachability::stated(ReachKind::Witnessed, Value::Null).is_none());
    // A supplied ν that the identity labels contradict is an engine error.
    let contradicted = levels::label_checkpoint(ctx, solved_open.state(), solved_open.reachability(), Some(one));
    assert!(contradicted.is_err(), "{contradicted:?}");
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
/// repeat. The sticker-test sentence runs in the next test).
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

/// The number of pieces at left height `left`, summed from JI.6's per-pose table.
fn at_left_height(state: &State, left: i64) -> i64 {
    let facts = heights(ctx(), state).expect("heights");
    let rows = field(&facts, "poses").as_object().expect("poses");
    rows.values().filter(|row| heights_of(row)[0] == left).map(|row| int(row, "count")).sum()
}

/// CT-LB3-03 (part: "Φ = 0 with height-2 pieces does not pass the sticker test", on M8's `levels::sticker_test`; the
/// library entry waits for the IR draft's collections).
///
/// After the first record of the I_a journal, and after 12 records of the I_a and S4 journals, Φ is 0 while pieces sit
/// at left height 2, the largest left height there, so no piece adds to Φ. The sticker test answers no on each, with
/// the off-lattice count as its evidence: a piece at height 2 is off the lattice. Solved, every piece is at height 0,
/// Φ is 0, and the sticker test answers yes.
#[test]
fn ct_lb3_03_phi_zero_with_height_two_pieces_does_not_pass_the_sticker_test() {
    let ctx = ctx();
    for (name, count) in [("I_a", 1), ("I_a", 12), ("S4", 12)] {
        let state = fixture(name).after(count);
        assert_eq!(phi_and_peak(&state), (0, 2), "{name} after {count}: Φ and the largest left height");
        assert!(at_left_height(&state, 2) > 0, "{name} after {count}: pieces at height 2");
        let verdict = levels::sticker_test(ctx, &state).expect("no engine error");
        let Verdict::No(evidence) = &verdict else { panic!("{name} after {count}: {verdict:?}") };
        assert_eq!(
            (text(evidence, "fact"), int(evidence, "off")),
            ("off_lattice", state.off_lattice_count() as i64),
            "{name} after {count}"
        );
    }
    let solved = fixture("I_a").solved();
    assert_eq!((phi_and_peak(&solved), at_left_height(&solved, 0)), ((0, 0), NP as i64));
    assert!(matches!(levels::sticker_test(ctx, &solved).expect("no engine error"), Verdict::Yes(_)));
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

/// The integers of a JSON array.
fn ints(value: &Value) -> Vec<i64> {
    value.as_array().expect("an array").iter().map(|v| v.as_i64().expect("an integer")).collect()
}

/// The change of N_c under each item of the pinned menu at cap `cap` from `state`, in pinned order, each read by
/// JI.6's release check of that one letter.
fn changes(fixture: &Fixture, state: &State, cap: i64) -> Vec<i64> {
    (0..fixture.menu.items.len() as i32)
        .map(|item| {
            let letter = fixture.letters(&[[cap as i32, item]]).remove(0);
            int(&release_check(ctx(), state, &letter, None, None).expect("a release check"), "delta_N_c")
        })
        .collect()
}

/// The pinned tie rule, recomputed: the item of the lowest value, then of the smaller pinned index; with the number of
/// items that reach the lowest value.
fn lowest(values: &[i64]) -> (i64, usize) {
    let low = *values.iter().min().expect("items");
    let first = values.iter().position(|&value| value == low).expect("the lowest value");
    (first as i64, values.iter().filter(|&&value| value == low).count())
}

/// CT-LB3-05 and CT-BND-17 (part: the kernel half, on M8's `observe::cuts::repair_list`, JI.12; the library entry waits
/// for the IR draft's collections. Not run: a job that names its cap through the command layer (KN5), the built-in
/// column's repair-list exception in `c600-check`, an empty list worded as BND.2 states, and "no key skips caps with an
/// empty list", which need LP2's command layer and checker).
///
/// After the first two records of the I_a journal, caps 57 and 86 are the misaligned caps. The list at the named cap 57
/// names that cap, the pinned menu and its order, the tie rule and the input digest. It completes with one outcome per
/// item x other than the identity, in pinned order, and meeting cap d, in cap-ID order (here cap 86 only). Its rows are
/// the outcomes whose delta is negative, in the same x-then-d order: the deltas rise and fall from row to row, each row
/// is five integers with no mark, and no field's name ranks, recommends or skips. For item 4, y and z are recomputed
/// apart from the list with JI.6's release check of every item: both choices tie, the list takes the smaller pinned
/// index, and the row's delta is the change of N of its three-letter word. A row whose z is the identity item has the
/// delta of its two-letter word x, y. After the first record alone, no cap meeting cap 57 is misaligned, so the list
/// completes empty, while the one-letter word that undoes the record lowers N: an empty list speaks only of the pairs
/// at its cap. A stopped list has no rows, also with no outcomes, so only a completed list is empty. A cap outside
/// 0..599 is refused with no outcomes; a closed cap, a blocked cap and free mode are errors with no outcomes.
#[test]
fn ct_lb3_05_the_repair_list_runs_at_a_named_cap_in_neutral_order() {
    let ctx = ctx();
    let fixture = fixture("I_a");
    let state = fixture.after(2);
    assert_eq!(fixture.menu.items[0].1, identity4(), "item 0 is the identity");
    let items = fixture.menu.items.len();
    let rotations: Vec<i64> =
        (1..items).filter(|&k| fixture.menu.items[k].1 != identity4()).map(|k| k as i64).collect();

    // The misaligned caps, and a blocked cap, by the rim summary.
    let summary = rim_summary(ctx, &mut state.clone(), None, None).expect("a rim summary");
    let caps = field(&summary, "rows").as_array().expect("rows");
    let open = |row: &&Value| flag(row, "blocked") == Some(false);
    let misaligned: Vec<i64> =
        caps.iter().filter(open).filter(|row| int(row, "N_c") > 0).map(|row| int(row, "cap")).collect();
    assert_eq!(misaligned, [57, 86]);
    let blocked = caps.iter().find(|row| flag(row, "blocked") == Some(true)).map(|row| int(row, "cap"));
    let blocked = blocked.expect("a blocked cap");

    // The list at the named cap 57.
    let list = repair_list(ctx, &state, 57, None, None, None).expect("no engine error");
    assert_eq!((text(&list, "status"), text(&list, "input_digest")), ("completed", state.digest(ctx).as_str()));
    assert!(is_null(&list, "message") && is_null(&list, "by"));
    let parameters = field(&list, "parameters");
    assert_eq!(int(parameters, "cap"), 57, "the list names its cap");
    assert_eq!(text(parameters, "tie_rule"), "lowest value, then smaller pinned index");
    assert_eq!(
        (text(parameters, "menu_identity"), text(parameters, "order_digest")),
        (fixture.menu.identity.as_str(), fixture.menu.order.as_str())
    );
    let coverage = field(&list, "coverage");
    assert_eq!(
        (ints(field(coverage, "meeting_caps")), ints(field(coverage, "items_done"))),
        (vec![86], rotations.clone())
    );
    let outcomes = field(&list, "outcomes").as_array().expect("outcomes");
    assert_eq!(int(coverage, "pairs_tried"), outcomes.len() as i64);
    let pairs: Vec<[i64; 2]> = outcomes
        .iter()
        .map(|outcome| {
            let outcome = outcome.as_array().expect("an outcome");
            [outcome[0].as_i64().expect("x"), outcome[1].as_i64().expect("d")]
        })
        .collect();
    assert_eq!(pairs, rotations.iter().map(|&x| [x, 86]).collect::<Vec<_>>(), "x in pinned order, then d");

    // The rows: the outcomes that lower N, in the same order, unranked.
    let lowering: Vec<Vec<i64>> = outcomes
        .iter()
        .filter_map(|outcome| {
            let outcome = outcome.as_array().expect("an outcome");
            let (delta, chosen) = (outcome[2].as_i64()?, outcome[3].as_array()?);
            let (x, d) = (outcome[0].as_i64().expect("x"), outcome[1].as_i64().expect("d"));
            (delta < 0).then(|| vec![x, d, chosen[0].as_i64().expect("y"), chosen[1].as_i64().expect("z"), delta])
        })
        .collect();
    let rows: Vec<Vec<i64>> =
        field(&list, "rows").as_array().expect("a completed list has rows").iter().map(ints).collect();
    assert_eq!(rows, lowering, "the rows are the outcomes that lower N, in their order");
    assert!(rows.len() > 2 && rows.len() < outcomes.len(), "{} rows of {} outcomes", rows.len(), outcomes.len());
    assert!(rows.iter().all(|row| row.len() == 5), "a row is x, d, y, z and delta, with no mark");
    let deltas: Vec<i64> = rows.iter().map(|row| row[4]).collect();
    assert!(deltas.windows(2).any(|w| w[0] < w[1]) && deltas.windows(2).any(|w| w[0] > w[1]), "not ordered by delta");
    let mut names = BTreeSet::new();
    keys(&list, &mut names);
    for name in &names {
        for leak in ["recommend", "suggest", "next", "skip", "best", "rank", "score", "target"] {
            assert!(!name.contains(leak), "field `{name}`");
        }
    }

    // The tie rule, recomputed for item 4 with the release check of every item at d after x and at c after x and y.
    let row = rows.iter().find(|row| row[0] == 4).expect("a row for item 4");
    let (x, d, y, z, delta) = (row[0], row[1], row[2], row[3], row[4]);
    let letter = |cap: i64, item: i64| fixture.letters(&[[cap as i32, item as i32]]).remove(0);
    let mut after_x = state.clone();
    assert_eq!(play(&mut after_x, &letter(57, x)), Status::Admissible);
    let at_d = changes(&fixture, &after_x, d);
    let mut after_xy = after_x.clone();
    assert_eq!(play(&mut after_xy, &letter(d, y)), Status::Admissible);
    let at_c = changes(&fixture, &after_xy, 57);
    let ((lowest_y, ties_y), (lowest_z, ties_z)) = (lowest(&at_d), lowest(&at_c));
    assert_eq!((y, z), (lowest_y, lowest_z), "the lowest value, then the smaller pinned index");
    assert!(ties_y > 1 && ties_z > 1, "both choices tie: {ties_y} and {ties_z} items");
    let x_change = int(&release_check(ctx, &state, &letter(57, x), None, None).expect("a release check"), "delta_N_c");
    assert_eq!(x_change + at_d[y as usize] + at_c[z as usize], delta, "delta sums the three changes of the own cuts");
    let three = fixture.letters(&[[57, x as i32], [d as i32, y as i32], [57, z as i32]]);
    assert_eq!(int(&checked(&state, &three, None, None), "delta_N"), delta, "and is the change of N of x, y, z");
    // A row whose z is the identity: its two-letter word x, y has the row's delta.
    let short = rows.iter().find(|row| row[3] == 0).expect("a row whose z is the identity item");
    let two = fixture.letters(&[[57, short[0] as i32], [short[1] as i32, short[2] as i32]]);
    assert_eq!(int(&checked(&state, &two, None, None), "delta_N"), short[4], "the two-letter word x, y");

    // A stopped list keeps its outcomes as coverage and has no rows, even with no outcomes.
    let budget = Some("KN7:budget.kernel_cost");
    let stops = [
        ("cancelled before its first item", None, Some(0), "cancelled", None, 0),
        ("cut off at no pair", Some(0), None, "cut_off", budget, 0),
        ("cut off after one pair", Some(1), None, "cut_off", budget, 1),
    ];
    for (name, max_pairs, cancel_after, status, by, tried) in stops {
        let stopped = repair_list(ctx, &state, 57, max_pairs, cancel_after, None).expect("no engine error");
        assert_eq!((text(&stopped, "status"), field(&stopped, "by").as_str()), (status, by), "{name}");
        assert!(is_null(&stopped, "rows"), "{name}: no rows, so never an empty list");
        assert_eq!(field(&stopped, "outcomes").as_array().expect("outcomes")[..], outcomes[..tried], "{name}");
        assert_eq!(int(field(&stopped, "parameters"), "cap"), 57, "{name}");
    }

    // After the first record alone the list completes empty; a word outside its repair family still lowers N.
    let first = fixture.after(1);
    let empty = repair_list(ctx, &first, 57, None, None, None).expect("no engine error");
    assert_eq!(text(&empty, "status"), "completed");
    assert!(ints(field(&empty, "rows")).is_empty() && ints(field(&empty, "outcomes")).is_empty(), "an empty list");
    let coverage = field(&empty, "coverage");
    assert_eq!((ints(field(coverage, "meeting_caps")), ints(field(coverage, "items_done"))), (vec![], rotations));
    let record = Twist::from_record(ctx, &fixture.records[0]).expect("the first record");
    let at_57 = fixture.menu.for_cap(ctx, 57).expect("the menu at cap 57");
    let item = at_57.iter().position(|twist| twist.matrix == record.matrix).expect("a letter of the menu") as i32;
    assert_eq!(record.grip, 57);
    let undo = checked(&first, &fixture.letters(&[[57, fixture.inverse(item)]]), None, None);
    assert_eq!(text(&undo, "end_digest"), fixture.solved().digest(ctx), "the word undoes the record");
    assert!(int(&undo, "delta_N") < 0, "and lowers N, outside the list's family");

    // A cap outside the model is refused before anything is read; a closed cap, a blocked cap and free mode are errors.
    for cap in [600, -1] {
        let refused = repair_list(ctx, &state, cap, None, None, None).expect("no engine error");
        assert_eq!((text(&refused, "status"), text(&refused, "code")), ("refused", codes::NAMED_OBJECT_INVALID));
        assert_eq!((text(&refused, "argument"), ints(field(&refused, "supplied"))), ("cap", vec![cap]));
        assert!(refused.get("outcomes").is_none() && refused.get("rows").is_none(), "cap {cap}: nothing is read");
    }
    let errors = [
        (fixture.solved(), 57, "cap 57 is closed (N_c = 0), not misaligned".to_owned()),
        (state.clone(), blocked, format!("cap {blocked} is blocked or uncertain")),
        (State::new(None), 57, "free mode pins no menu, so a repair list has no pinned order".to_owned()),
    ];
    for (start, cap, message) in errors {
        let answer = repair_list(ctx, &start, cap, None, None, None).expect("no engine error");
        assert_eq!((text(&answer, "status"), text(&answer, "message")), ("error", message.as_str()));
        assert_eq!(int(field(&answer, "parameters"), "cap"), cap, "{message}");
        assert!(is_null(&answer, "rows") && ints(field(&answer, "outcomes")).is_empty(), "{message}: no outcomes");
    }
}

/// CT-LB3-06 (part: the kernel half, on exact word fixtures in the pinned I_a menu; the library entry waits for the IR
/// draft's collections. Not run: Net and Strict protection of a whole word, which M8's `levels::derive` has and this
/// test does not take up yet, so "no protection conflict" is a run with no protected piece).
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
/// collections. Not run: the per-revision rim and survey rows' "no all-items outcome table", not taken up yet, and the
/// correction catalogue, which M8 does not have).
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
