//! The identity tower's kernel rows on the jumbling puzzle's host (T2; T1 drafts until the IR freeze, PHS.49): programs
//! of hand-built IR run on `c600-run` against the jumbling rows' bridge ([`Rows`], `rows/tower.rs`) over M8's `levels`
//! module, on a witnessed base apart from a layer or at a command layer's head, whose value store keeps the derived
//! values (`c600_command::values`). The base is the I_a walk of `wj-I_a.json` under its pinned alphabet; its words are
//! menu words of literal letters. The interpreter's halves of the same rows run against a fake host in
//! `c600-run/tests/tower.rs`; 600-cell-Full's host's in `c600-tools/tests/tower_rows.rs`. Source and fixture evidence
//! only.

mod common;

use std::sync::{Arc, OnceLock};

use c600_command::values::{BaseKey, Recipe, Source, ValueKey};
use c600_command::{code, Secret};
use c600_ir::build::{
    case, construct, exact, field, helper, imply, kernel, let_, menu_word, module, nat, pat, program, read_snapshot,
    var, word,
};
use c600_ir::{Expr, Goal, KernelOp, Level, Program, Puzzle, QTy, Ty};
use c600_jumble::codes::{PIECE_WOULD_MOVE, WORD_BLOCKED_PREFIX};
use c600_jumble::levels::{self, Base, Partial, Projection, Snapshot};
use c600_jumble::model::NP;
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble_tool::rows::{state_size, Rows, VALUE_UNAVAILABLE};
use c600_jumble_tool::word::Alphabet;
use c600_kernel::json::Value as Json;
use c600_kernel::ProtectionMode;
use c600_run::interp::code::{IDENTITY_MISMATCH, LEVEL_NOT_HELD};
use c600_run::interp::{load, Host, Options, Request, RowTariff, Run, Sink};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome, Row};
use c600_run::tower::HandleTy;
use c600_run::value::Value;

use common::{engine, Fixture, Session};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const J: Puzzle = Puzzle::Jumbling;

/// The records of the I_a walk that the first word plays, and the length of the second word that continues it.
const WALK: usize = 4;
const MORE: usize = 2;

/// The pinned I_a menu of `wj-I_a.json`.
fn i_a() -> Arc<Menu> {
    static I_A: OnceLock<Arc<Menu>> = OnceLock::new();
    I_A.get_or_init(|| Arc::new(Fixture::load("I_a").menu())).clone()
}

/// The I_a walk's twists.
fn twists() -> &'static [Twist] {
    static TWISTS: OnceLock<Vec<Twist>> = OnceLock::new();
    TWISTS.get_or_init(|| Fixture::load("I_a").twists)
}

/// The I_a alphabet with its item matrices, as a run pins it for menu words (LG2.18).
fn alphabet() -> &'static MenuAlphabet {
    static PINNED: OnceLock<MenuAlphabet> = OnceLock::new();
    PINNED.get_or_init(|| {
        let alphabet = Alphabet::of(engine(), &i_a()).expect("an inverse-closed menu");
        alphabet.pinned().expect("I_a's alphabet pins with its item matrices")
    })
}

/// `twist` as a letter `[grip, item]` of the pinned I_a menu.
fn letter(twist: &Twist) -> [u32; 2] {
    let items = i_a().for_cap(engine(), twist.grip).expect("the menu's items");
    let item = items.iter().position(|item| item.matrix == twist.matrix).expect("an item of the menu");
    [twist.grip as u32, item as u32]
}

fn letters(twists: &[Twist]) -> Vec<[u32; 2]> {
    twists.iter().map(letter).collect()
}

/// `journal` played from solved under I_a, every letter admissible, with the claim its replay witnesses.
fn witnessed(journal: &[Twist]) -> Snapshot {
    let mut state = State::new(Some(i_a()));
    for (index, twist) in journal.iter().enumerate() {
        let outcome = state.apply(engine(), twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "letter {index}");
    }
    Snapshot::witnessed(engine(), state, journal, None).expect("no engine error").expect("the journal witnesses it")
}

fn solved() -> &'static Snapshot {
    static SOLVED: OnceLock<Snapshot> = OnceLock::new();
    SOLVED.get_or_init(|| Snapshot::solved(engine(), Some(i_a())))
}

/// The state after the walk's first `n` records.
fn walked(n: usize) -> &'static Snapshot {
    static WALKS: OnceLock<Vec<Snapshot>> = OnceLock::new();
    &WALKS.get_or_init(|| (0..=WALK + MORE).map(|n| witnessed(&twists()[..n])).collect())[n]
}

/// The raw pose table of `state` (KN2.14's record).
fn table_of(state: &State) -> Json {
    let poses: Vec<Pose> = state.pose_ids().iter().map(|&id| state.poses()[id as usize].clone()).collect();
    levels::record_of(&poses)
}

/// A letter of I_a blocked at `state`: the first cap, in cap order, whose first non-identity item is refused as
/// blocked.
fn blocked_at(state: &State) -> [u32; 2] {
    for cap in 0..c600_jumble::model::POLES {
        let items = i_a().for_cap(engine(), cap).expect("the menu's items");
        for (item, twist) in items.iter().enumerate().skip(1).take(1) {
            let mut probe = state.clone();
            if probe.apply(engine(), twist, None, &[]).expect("no engine error").status == Status::Blocked {
                return [cap as u32, item as u32];
            }
        }
    }
    panic!("no blocked letter at the state");
}

fn q(ty: Ty) -> QTy {
    exact(ty)
}

fn snapshot() -> Expr {
    kernel(KernelOp::Snapshot(J), vec![], q(Ty::State(J)))
}

fn mw(letters: &[[u32; 2]]) -> Expr {
    menu_word(letters)
}

fn derive(s: Expr, w: Expr) -> Expr {
    kernel(KernelOp::Derive, vec![s, w], q(Ty::Derivation(J)))
}

fn project(s: Expr, level: Level) -> Expr {
    kernel(KernelOp::Project(level), vec![s], q(Ty::Partial(Box::new(q(Ty::LevelValue(J, level))))))
}

fn digest(v: Expr, level: Level) -> Expr {
    kernel(KernelOp::Digest, vec![v], q(Ty::Digest(J, level)))
}

fn compare(a: Expr, b: Expr, level: Level) -> Expr {
    kernel(KernelOp::Compare(level), vec![a, b], q(Ty::Partial(Box::new(q(Ty::Verdict(J, level, None))))))
}

fn reach(s: Expr) -> Expr {
    kernel(KernelOp::Reachability, vec![s], q(Ty::Reachability(J)))
}

fn import(t: Expr, claim: Expr) -> Expr {
    kernel(KernelOp::Import, vec![t, claim], q(Ty::State(J)))
}

fn table(name: &str) -> Expr {
    read_snapshot(name, q(Ty::PoseTable))
}

fn check(goal: Goal, args: Vec<Expr>) -> Expr {
    kernel(KernelOp::Check(goal), args, q(Ty::Verdict(J, goal.level(), Some(goal))))
}

/// `then` with the derived state of `d` bound to `name`; a refusal stops the run.
fn derived(d: Expr, name: &str, then: Expr) -> Expr {
    case(d, vec![(pat("derived", &[name]), then)], None)
}

/// `then` with the total value of the partial result `p` bound to `name`.
fn total(p: Expr, name: &str, then: Expr) -> Expr {
    case(p, vec![(pat("total", &[name]), then)], None)
}

fn state_digest(s: Expr, level: Level) -> Expr {
    total(project(s, level), "v", digest(var("v"), level))
}

/// The yes/no/unknown of a verdict as 1, 0 or 2.
fn decided(verdict: Expr) -> Expr {
    case(
        verdict,
        vec![(pat("yes", &["_"]), nat(1)), (pat("no", &["_"]), nat(0)), (pat("unknown", &["_"]), nat(2))],
        None,
    )
}

/// A claim's kind as a number: witnessed 0, by theorem 1, promised 2, refuted 3, open 4.
fn kind_of(claim: Expr) -> Expr {
    let kinds = ["witnessed", "by_theorem", "promised", "refuted", "open"];
    case(claim, kinds.iter().enumerate().map(|(i, k)| (pat(k, &["_"]), nat(i as u64))).collect(), None)
}

/// The snapshot's claim made `kind` with the same evidence: a claim the program states about a table it imports.
fn restated(kind: &str, then: impl FnOnce(Expr) -> Expr) -> Expr {
    let claim = construct(Ty::Reachability(J), kind, vec![var("e")]);
    case(reach(snapshot()), vec![(pat("witnessed", &["e"]), then(claim))], None)
}

fn prog(result: Ty, body: Expr) -> Program {
    program(vec![module("m", vec![helper("f", vec![], q(result), body).exported()])])
}

fn run_on(host: &mut dyn Host, result: Ty, body: Expr) -> Run {
    let options = Options { menu: Some(alphabet()), ..Options::new(WIDE) };
    load(&prog(result, body)).expect("the program loads").run("m", "f", vec![], host, &options).unwrap()
}

fn completed(run: &Run) {
    assert_eq!(run.summary.outcome, Outcome::Completed, "{:?} {:?}", run.summary.code, run.summary.stop);
}

fn nat_of(run: &Run) -> u64 {
    match run.value.as_ref() {
        Some(Value::Int(n)) => u64::try_from(&**n).expect("a small natural"),
        other => panic!("{other:?}: {:?}", run.summary.code),
    }
}

fn output(run: &Run) -> String {
    completed(run);
    String::from_utf8(run.output.clone().expect("an output")).unwrap()
}

fn kernel_rows(run: &Run) -> Vec<&Row> {
    run.summary.rows.iter().filter(|row| row.at.ends_with("#kernel")).collect()
}

/// CT-LG2-07 (part: the run-time half on the jumbling puzzle's host; not run: the source half, T1b). CT-LG2-01
/// (part: matches over every constructor of the jumbling host's reachability claims; not run: the source half,
/// T1b; the interpreter's and the checker's halves are `c600-run/tests/tower.rs` and `c600-check/tests/tower.rs`).
/// CT-LG2-02 (part: the jumbling host's half: 600-cell-Full's snapshot is not granted and the sticker class, which this
/// edition keys nowhere, is not held; not run: the source half, T1b). Raw pose data enters only through `import`
/// (KN2.14): a table the host gave, read as a `PoseTable` handle, with a claim the program states, open or promised,
/// becomes a state of the base's identity that compares at the poses with the state a derivation reaches; its claim
/// stays what was stated and a derivation from it carries the claim along. A witnessed, refuted or by-theorem claim is
/// refused at the tariff with `KN5:input.argument_form`, a table that does not validate is refused with the same code,
/// and a table the host does not hold is not granted. A derived state keeps its base's identity and level: its pose
/// digest is the engine's for the witnessed walk. A jumbling witness does not assert retained reachability: a twist off
/// K⁺ and its inverse return to solved with a witnessed claim that is not retained, so the retained checkpoint is
/// `unknown` there while the exact pose holds.
#[test]
fn ct_lg2_07_raw_poses_enter_by_import_and_derived_states_keep_their_identity() {
    let walk = letters(&twists()[..WALK]);
    let want = walked(WALK).state().digest(engine());
    let rows = || Rows::on(engine(), Base::Snapshot(solved())).with_pose_table("walk", table_of(walked(WALK).state()));

    let mut host = rows();
    let body = derived(derive(snapshot(), mw(&walk)), "s", state_digest(var("s"), Level::Poses));
    let out = output(&run_on(&mut host, Ty::Digest(J, Level::Poses), body));
    assert!(out.contains(&want) && out.contains("\"puzzle\":\"Jumbling\""), "{out}");

    for kind in ["open", "promised"] {
        let mut host = rows();
        let body = restated(kind, |claim| {
            let_(
                "i",
                import(table("walk"), claim),
                derived(
                    derive(snapshot(), mw(&walk)),
                    "s",
                    total(compare(var("i"), var("s"), Level::Poses), "v", decided(var("v"))),
                ),
            )
        });
        let r = run_on(&mut host, Ty::Nat, body);
        completed(&r);
        assert_eq!(nat_of(&r), 1, "{kind}: the imported table is the walk's state");
        let import_row = kernel_rows(&r).iter().find(|row| row.kernel_cost == NP as u128).copied();
        assert!(import_row.is_some(), "one pass for the import");

        // The imported value's claim is the stated one, and a derivation from it carries it along.
        let mut host = rows();
        let more = letters(&twists()[WALK..WALK + MORE]);
        let body = restated(kind, |claim| {
            let_("i", import(table("walk"), claim), derived(derive(var("i"), mw(&more)), "t", kind_of(reach(var("t")))))
        });
        let r = run_on(&mut host, Ty::Nat, body);
        completed(&r);
        assert_eq!(nat_of(&r), if kind == "open" { 4 } else { 2 }, "{kind}");
    }

    for kind in ["witnessed", "refuted", "by_theorem"] {
        let mut host = rows();
        let body = restated(kind, |claim| let_("i", import(table("walk"), claim), nat(0)));
        let r = run_on(&mut host, Ty::Nat, body);
        assert_eq!(r.summary.code, Some(code::ARGUMENT_FORM), "{kind}");
        assert_eq!(kernel_rows(&r).last().map(|row| row.kernel_cost), Some(0), "{kind}: refused at the tariff");
    }
    let mut host = Rows::on(engine(), Base::Snapshot(solved()));
    let r = run_on(&mut host, Ty::Nat, restated("open", |claim| let_("i", import(table("walk"), claim), nat(0))));
    assert_eq!(r.summary.code, Some(NOT_GRANTED), "no table under that name");

    let mut host =
        Rows::on(engine(), Base::Snapshot(solved())).with_pose_table("bad", Json::Object(Default::default()));
    let r = run_on(&mut host, Ty::Nat, restated("open", |claim| let_("i", import(table("bad"), claim), nat(0))));
    assert_eq!(r.summary.code, Some(code::ARGUMENT_FORM), "a table that does not validate");

    // A witness through a twist off K⁺ and back reaches solved, witnessed and not retained.
    let first = letter(&twists()[0]);
    assert!(twists()[0].kidx.is_none(), "the walk's first twist is off K⁺");
    let alphabet = Alphabet::of(engine(), &i_a()).expect("an inverse-closed menu");
    let inverse = alphabet.inverse_item(first[1] as i32).expect("an inverse item") as u32;
    let back = [first, [first[0], inverse]];
    let mut host = Rows::on(engine(), Base::Snapshot(solved()));
    for (goal, want) in [(Goal::ExactPose, 1), (Goal::RetainedCheckpoint, 2)] {
        let body = derived(derive(snapshot(), mw(&back)), "s", decided(check(goal, vec![var("s")])));
        let r = run_on(&mut host, Ty::Nat, body);
        completed(&r);
        assert_eq!(nat_of(&r), want, "{goal:?} after a twist off K⁺ and its inverse");
    }
    let body = derived(derive(snapshot(), mw(&back)), "s", kind_of(reach(var("s"))));
    assert_eq!(nat_of(&run_on(&mut host, Ty::Nat, body)), 0, "witnessed");

    // The 600-cell-Full rows are not this host's, and the sticker class is not held.
    let mut host = rows();
    let full = kernel(KernelOp::Snapshot(Puzzle::Full), vec![], q(Ty::State(Puzzle::Full)));
    let r = run_on(&mut host, Ty::Nat, let_("s", full, nat(0)));
    assert_eq!(r.summary.code, Some(NOT_GRANTED));
    let r = run_on(&mut host, Ty::Nat, let_("p", project(snapshot(), Level::StickerClass), nat(0)));
    assert_eq!(r.summary.code, Some(LEVEL_NOT_HELD));
    assert_eq!(kernel_rows(&r).last().map(|row| row.kernel_cost), Some(0), "refused at the tariff");
}

/// CT-LG2-03 (part: the jumbling host's half: a read of labels off the lattice returns `partial`; not run: the
/// source half, T1b, and no typed lift of labels to poses, a static rule). CT-LG2-09 (part: the run-time half on the
/// jumbling puzzle's host; not run: the source half, T1b). Labels off the lattice are read as `partial`, never as
/// an unknown or a guess: after the walk's first twist, which is not retained, the labels' projection is the covered
/// part with the engine's off count, and a comparison of the labels with solved is partial with the counts of both
/// sides added. The poses' projection stays total.
#[test]
fn ct_lg2_09_labels_off_the_lattice_are_partial() {
    let base = walked(1);
    let off = base.state().off_lattice_count();
    assert!(off > 0, "the first twist leaves the lattice");
    let missing = match levels::project(engine(), base.state(), levels::Level::Labels).expect("no engine error") {
        Projection::Value(Partial::Partial { missing, .. }) => missing,
        other => panic!("{other:?}"),
    };
    let mut host = Rows::on(engine(), Base::Snapshot(base));
    let body = case(
        project(snapshot(), Level::Labels),
        vec![(pat("partial", &["v", "m"]), field(var("m"), "off")), (pat("total", &["_"]), nat(0))],
        None,
    );
    let r = run_on(&mut host, Ty::Nat, body);
    completed(&r);
    assert_eq!(nat_of(&r), missing.off as u64);
    let body = case(project(snapshot(), Level::Poses), vec![(pat("total", &["_"]), nat(1))], Some(nat(0)));
    assert_eq!(nat_of(&run_on(&mut host, Ty::Nat, body)), 1);

    let walk = letters(&twists()[..1]);
    let mut host = Rows::on(engine(), Base::Snapshot(solved()));
    let body = derived(
        derive(snapshot(), mw(&walk)),
        "s",
        case(
            compare(snapshot(), var("s"), Level::Labels),
            vec![(pat("partial", &["v", "m"]), field(var("m"), "off"))],
            None,
        ),
    );
    let r = run_on(&mut host, Ty::Nat, body);
    completed(&r);
    assert_eq!(nat_of(&r), missing.off as u64, "solved misses nothing");
}

/// CT-LG2-10 (part: the run-time half on the jumbling puzzle's host; not run: the source half, T1b, and a fixture
/// of equal labels with unequal poses). Each goal is checked at its own level with the host's certificate, which
/// `imply` takes along LG2.11's edges: at solved the exact pose, the sticker test, the lattice placement, the low
/// height at bound 0, the retained checkpoint and the label checkpoint all hold, and the exact pose implies the sticker
/// test and the low height, the label checkpoint the lattice placement. Off the lattice the exact pose, the lattice
/// placement and both checkpoints are `no` and the sticker test does not hold. A state imported with an open claim is
/// not a checkpoint the claim decides: the retained and the label checkpoints are `unknown`, never `no` (CT-LG2-09's
/// unknown), while the exact pose, which reads the poses only, holds.
#[test]
fn ct_lg2_10_goals_are_checked_at_their_level_and_imply_along_the_edges() {
    use Goal::*;
    let goals = |s: fn() -> Expr| -> Vec<(Goal, Expr)> {
        [ExactPose, StickerTest, LatticePlacement, RetainedCheckpoint, LabelCheckpoint]
            .into_iter()
            .map(|g| (g, check(g, vec![s()])))
            .chain([(LowHeight, check(LowHeight, vec![s(), nat(0)]))])
            .collect()
    };
    let mut host = Rows::on(engine(), Base::Snapshot(solved()));
    for (goal, verdict) in goals(snapshot) {
        let r = run_on(&mut host, Ty::Nat, decided(verdict));
        completed(&r);
        assert_eq!(nat_of(&r), 1, "{goal:?} at solved");
        let row = kernel_rows(&r).last().copied().unwrap();
        let passes = if goal == LabelCheckpoint { 2 } else { 1 };
        assert_eq!(row.kernel_cost, passes * NP as u128, "{goal:?}");
    }
    for (from, to, bound) in
        [(ExactPose, StickerTest, None), (ExactPose, LowHeight, Some(3)), (LabelCheckpoint, LatticePlacement, None)]
    {
        let mut args = vec![var("h")];
        args.extend(bound.map(nat));
        let body = case(
            check(from, vec![snapshot()]),
            vec![
                (pat("yes", &["h"]), let_("g", imply(to, args), nat(1))),
                (pat("no", &["_"]), nat(0)),
                (pat("unknown", &["_"]), nat(0)),
            ],
            None,
        );
        let r = run_on(&mut host, Ty::Nat, body);
        completed(&r);
        assert_eq!(nat_of(&r), 1, "{from:?} to {to:?}");
    }

    let mut off = Rows::on(engine(), Base::Snapshot(walked(1)));
    for (goal, verdict) in goals(snapshot) {
        let r = run_on(&mut off, Ty::Nat, decided(verdict));
        completed(&r);
        match goal {
            ExactPose | LatticePlacement | RetainedCheckpoint | LabelCheckpoint => {
                assert_eq!(nat_of(&r), 0, "{goal:?} off the lattice")
            }
            StickerTest => assert_ne!(nat_of(&r), 1, "{goal:?} off the lattice"),
            _ => {}
        }
    }

    let mut imported =
        Rows::on(engine(), Base::Snapshot(solved())).with_pose_table("solved", table_of(solved().state()));
    for (goal, want) in [(ExactPose, 1), (RetainedCheckpoint, 2), (LabelCheckpoint, 2)] {
        let body =
            restated("open", |claim| let_("i", import(table("solved"), claim), decided(check(goal, vec![var("i")]))));
        let r = run_on(&mut imported, Ty::Nat, body);
        completed(&r);
        assert_eq!(nat_of(&r), want, "{goal:?} of an imported table with an open claim");
    }
}

/// CT-LG2-11 (part: the run-time half on the jumbling puzzle's host; not run: the source half, T1b). `derive`
/// takes a menu word's literal letters (LG2.17) and answers a refusal as a result, `refused({ prefix, outcome, code
/// })`: a letter blocked after the walk is refused at its own prefix as `blocked` with `KN2:input.word_blocked_prefix`;
/// with every piece protected a Net word is refused as a whole (the prefix is its length) and a Strict word at its
/// first letter, `protected` with `KN2:protection.piece_would_move`. No value is kept for a refusal. A Full word for
/// the jumbling state stops with `KN1:input.identity_mismatch` before the host is asked. The derive row's tariff is one
/// pass for each letter.
#[test]
fn ct_lg2_11_a_derivation_takes_menu_letters_and_refuses_as_a_result() {
    let mut walk = letters(&twists()[..WALK]);
    walk.push(blocked_at(walked(WALK).state()));
    let refused =
        Ty::Record(vec![("prefix".into(), q(Ty::Nat)), ("outcome".into(), q(Ty::Text)), ("code".into(), q(Ty::Text))]);
    let as_refusal = |w: &[[u32; 2]]| case(derive(snapshot(), mw(w)), vec![(pat("refused", &["r"]), var("r"))], None);
    let mut host = Rows::on(engine(), Base::Snapshot(solved()));
    let r = run_on(&mut host, refused.clone(), as_refusal(&walk));
    let out = output(&r);
    assert!(out.contains(WORD_BLOCKED_PREFIX) && out.contains("\"outcome\":\"blocked\""), "{out}");
    assert!(out.contains(&format!("\"prefix\":{{\"int\":\"{}\"}}", walk.len())), "{out}");
    assert_eq!(kernel_rows(&r)[1].kernel_cost, walk.len() as u128 * NP as u128);
    assert_eq!(host.values().len(), 0, "a refusal keeps no value");

    let walk = letters(&twists()[..WALK]);
    let every: Vec<u32> = (0..NP as u32).collect();
    for (policy, prefix) in [(ProtectionMode::Net, walk.len()), (ProtectionMode::Strict, 1)] {
        let mut host = Rows::on(engine(), Base::Snapshot(solved())).with_protection(policy, every.clone());
        let out = output(&run_on(&mut host, refused.clone(), as_refusal(&walk)));
        assert!(out.contains(PIECE_WOULD_MOVE) && out.contains("\"outcome\":\"protected\""), "{policy:?}: {out}");
        assert!(out.contains(&format!("\"prefix\":{{\"int\":\"{prefix}\"}}")), "{policy:?}: {out}");
    }

    let mut host = Rows::on(engine(), Base::Snapshot(solved()));
    let full = case(derive(snapshot(), word(&[1, 2])), vec![(pat("derived", &["_"]), nat(1))], Some(nat(0)));
    let r = load(&prog(Ty::Nat, full)).expect("loads").run("m", "f", vec![], &mut host, &Options::new(WIDE)).unwrap();
    assert_eq!(r.summary.code, Some(IDENTITY_MISMATCH));
}

/// A host over the rows that evicts every stored value before each row it performs.
struct Evicting<'r, 'a> {
    rows: &'r mut Rows<'a>,
    evictions: usize,
}

impl Host for Evicting<'_, '_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.rows.tariff(request)
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.evictions += self.rows.values().evict(None);
        self.rows.perform(request)
    }
    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.rows.sink(sink, value, bytes)
    }
}

/// CT-DET-08's program: the walk's second word derived from the walk's state, its pose digest, with a second alias
/// of the walk's derived state held across it.
fn chained() -> Expr {
    let first = || derive(snapshot(), mw(&letters(&twists()[..WALK])));
    let more = letters(&twists()[WALK..WALK + MORE]);
    let_(
        "held",
        first(),
        derived(first(), "a", derived(derive(var("a"), mw(&more)), "b", state_digest(var("b"), Level::Poses))),
    )
}

fn observed(run: &Run) -> (Option<Vec<u8>>, Vec<Row>, Amounts) {
    completed(run);
    (run.output.clone(), run.summary.rows.clone(), run.summary.used)
}

/// CT-DET-08 (part: the value store's states on the jumbling puzzle's host; not run: 600-cell-Full's, in
/// `c600-tools/tests/tower_rows.rs`, and a host row-answer cache, which 1.0's host does not keep (owner, 20 November
/// 2026, item 1)). The same program over the rows' cold store, warm store, a store whose values were evicted, a store
/// evicted before every row (each base recomputed from its stored derivation), a store where another run holds the
/// same value, and a command layer's store at its head: the result bytes, every declared charge and the used amounts
/// are equal, and the pose digest is the engine's for the walk's six records.
#[test]
fn ct_det_08_jumbling_values_cold_warm_evicted_and_shared_charge_alike() {
    let mut rows = Rows::on(engine(), Base::Snapshot(solved()));
    let cold = observed(&run_on(&mut rows, Ty::Digest(J, Level::Poses), chained()));
    let want = walked(WALK + MORE).state().digest(engine());
    assert!(String::from_utf8(cold.0.clone().unwrap()).unwrap().contains(&want));
    assert_eq!(rows.values().counts().misses, 2);

    let warm = observed(&run_on(&mut rows, Ty::Digest(J, Level::Poses), chained()));
    assert!(rows.values().counts().hits > 0);
    assert_eq!(warm, cold, "warm");

    assert_eq!(rows.values().evict(None), 2);
    let evicted = observed(&run_on(&mut rows, Ty::Digest(J, Level::Poses), chained()));
    assert!(rows.values().counts().recomputed > 0);
    assert_eq!(evicted, cold, "evicted");

    let every = {
        let mut host = Evicting { rows: &mut rows, evictions: 0 };
        let run = observed(&run_on(&mut host, Ty::Digest(J, Level::Poses), chained()));
        assert!(host.evictions > 0);
        run
    };
    assert_eq!(every, cold, "evicted before every row");

    let holder = derived(derive(snapshot(), mw(&letters(&twists()[..WALK]))), "s", nat(1));
    assert_eq!(nat_of(&run_on(&mut rows, Ty::Nat, holder)), 1);
    assert_eq!(observed(&run_on(&mut rows, Ty::Digest(J, Level::Poses), chained())), cold, "shared");

    let mut session = Session::entered(&Fixture::load("I_a"));
    let at_head = {
        let mut host = Rows::tower_at_head(engine(), &mut session.layer);
        observed(&run_on(&mut host, Ty::Digest(J, Level::Poses), chained()))
    };
    assert_eq!(at_head, cold, "a layer's store at its head");
    assert_eq!(session.layer.values().len(), 2, "the layer's store keeps the values");

    // Each derived state is charged its logical size when a case binds it (DET.24).
    let branches: Vec<u128> = cold.1.iter().filter(|row| row.at.ends_with("#branch")).map(|row| row.memory).collect();
    let sizes = [walked(WALK), walked(WALK + MORE)].map(|s| state_size(s.state()));
    assert_eq!(branches[..2], sizes, "{branches:?}");
}

/// A host over the rows that keeps every kernel row's answer and answers the snapshot read `old` with `old`.
struct Kept<'r, 'a> {
    rows: &'r mut Rows<'a>,
    kept: Vec<Value>,
    old: Option<Value>,
}

impl Host for Kept<'_, '_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Snapshot { query: "old", .. } => Ok(RowTariff { kernel_cost: 0, result_bound: 1 << 24 }),
            _ => self.rows.tariff(request),
        }
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        if let Request::Snapshot { query: "old", .. } = request {
            return self.old.clone().ok_or(NOT_GRANTED);
        }
        let answer = self.rows.perform(request)?;
        if matches!(request, Request::Kernel { .. }) {
            self.kept.push(answer.clone());
        }
        Ok(answer)
    }
    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.rows.sink(sink, value, bytes)
    }
}

/// CT-ONE-21 (part: the value store's half, ONE.70's "handles and derived values" on the jumbling puzzle's host at a
/// command layer's head; not run: the session store's half, in `c600-session/tests/conformance_recovery.rs`, the
/// layer's previews, drafts, receipts and permissions, in `c600-command/tests/layer.rs` and `kn5_stages.rs`, and
/// 600-cell-Full's host's, in `c600-tools/tests/tower_rows.rs`). After the command layer's restart the layer's value
/// store is a new epoch's, empty: a handle kept from the old epoch is refused with `KN2:stale.epoch_changed` at the
/// row's tariff, the derived value is gone, and its stored derivation, whose base names the head's revision and not a
/// handle, recomputes the same pose digest; a chained derivation's recipe carries its base's and recomputes too. Apart
/// from that head, the derivation's base does not resolve (`KN2:stale.value_unavailable`).
#[test]
fn ct_one_21_jumbling_values_end_at_a_restart_and_their_derivations_recompute() {
    let mut session = Session::entered(&Fixture::load("I_a"));
    let walk = letters(&twists()[..WALK]);
    let body = || derived(derive(snapshot(), mw(&walk)), "s", state_digest(var("s"), Level::Poses));
    let (before, kept) = {
        let mut rows = Rows::tower_at_head(engine(), &mut session.layer);
        let mut host = Kept { rows: &mut rows, kept: Vec::new(), old: None };
        let r = run_on(&mut host, Ty::Digest(J, Level::Poses), body());
        (output(&r), host.kept)
    };
    assert!(before.contains(&walked(WALK).state().digest(engine())));
    let Value::Sum(derivation) = &kept[1] else { panic!("a derivation: {:?}", kept[1]) };
    let handle = derivation.payload()[0].clone();
    let Value::Handle(h) = &handle else { panic!("a state handle") };
    assert_eq!((h.ty(), h.id() >> 32, h.size()), (HandleTy::State(J), 1, state_size(walked(WALK).state())));

    let revision = session.layer.revision().clone();
    let key = session.layer.values().keys().next().cloned().expect("the derived value's entry");
    let recipe = session.layer.values().recipe(&key).cloned().expect("its stored derivation");
    match (&key, &recipe) {
        (ValueKey::Derived { base: BaseKey::Revision { head, digest, .. }, .. }, Recipe::Derive { base, .. }) => {
            assert_eq!((head, digest), (&revision.head, &revision.digest), "the base is named by its revision");
            assert!(matches!(base, Source::Root(BaseKey::Revision { .. })));
        }
        other => panic!("{other:?}"),
    }
    let more = letters(&twists()[WALK..WALK + MORE]);
    let chain = derived(
        derive(snapshot(), mw(&walk)),
        "a",
        derived(derive(var("a"), mw(&more)), "b", state_digest(var("b"), Level::Poses)),
    );
    let chain_out = {
        let mut rows = Rows::tower_at_head(engine(), &mut session.layer);
        output(&run_on(&mut rows, Ty::Digest(J, Level::Poses), chain))
    };
    let chain_key =
        session.layer.values().keys().find(|k| matches!(k, ValueKey::Derived { base: BaseKey::Digest(_), .. }));
    let chain_recipe = session.layer.values().recipe(chain_key.expect("the second word's entry")).cloned().unwrap();

    session.layer.restart(Secret::fixed([12; 32]));
    assert_ne!(session.layer.revision().epoch, revision.epoch);
    assert!(session.layer.values().is_empty(), "the value store is not durable");
    assert_eq!(session.layer.values().ordinal(), 2);
    assert!(!session.layer.values().holds(&key), "the derived value is gone");

    let r = {
        let mut rows = Rows::tower_at_head(engine(), &mut session.layer);
        let mut host = Kept { rows: &mut rows, kept: Vec::new(), old: Some(handle) };
        let read = project(read_snapshot("old", q(Ty::State(J))), Level::Poses);
        run_on(&mut host, Ty::Nat, let_("p", read, nat(0)))
    };
    assert_eq!(r.summary.code, Some(code::EPOCH_CHANGED));
    assert_eq!(kernel_rows(&r).last().map(|row| (row.kernel_cost, row.memory)), Some((0, 0)));

    let mut rows = Rows::tower_at_head(engine(), &mut session.layer);
    let digest = rows.recompute(&recipe).expect("the revision is still the head");
    assert!(before.contains(&digest));
    let chained = rows.recompute(&chain_recipe).expect("the chain leads back to the revision");
    assert!(chain_out.contains(&chained));
    let again = output(&run_on(&mut rows, Ty::Digest(J, Level::Poses), body()));
    assert_eq!(again, before, "a new run's derivation in the new epoch");

    let mut elsewhere = Rows::on(engine(), Base::Snapshot(walked(1)));
    assert_eq!(elsewhere.recompute(&recipe), Err(VALUE_UNAVAILABLE));
}
