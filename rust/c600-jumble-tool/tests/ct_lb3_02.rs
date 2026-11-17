//! CT-LB3-02, LP3's side (LB3.4, KN1.11): a protection verdict carries its level and never crosses it. M5's
//! `c600-jumble/tests/crossing.rs` runs the kernel facts as CT-KN2-03: every retained letter of the pinned I_a menu at
//! grip 0 that turns the centre piece of cell 0 is refused at pose level inside Jumble and passes at label level after
//! the crossing at solved. This test does not repeat them. It builds `c600_jumble_tool::protection`'s verdicts from one
//! such letter, retained generator 1 (H_0), on each side, and checks that each verdict answers only for a puzzle of its
//! own level.
//!
//! What runs: the pose-level verdict of H_0 at grip 0 with the centre piece of cell 0 protected, which moves, and the
//! label-level verdict of the kernel word `[1]` from solved, which is clear, each read at its own level and refused at
//! the other with `KN1:input.level_not_held`; a label verdict that moves and a pose verdict that is clear, so no
//! finding follows from the level alone; and the digest each verdict names. Not run: Strict protection of a whole word,
//! which M8's `levels::derive` has at pose level and the kernel's `strict_piece_protection` at label level; the
//! kernel's label-level protection of named pieces, centres included (`piece_protection`), in place of the label
//! verdict composed from KN2.4's `positions` rows; and the pose-to-label crossing projection with its checkpoints,
//! which M8's `levels` has. This test does not take them up yet. A pass is source and fixture evidence.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::model::{NP, NS};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_jumble_tool::level::Level;
use c600_jumble_tool::protection::{Crossing, Finding, RowsDiffer, Verdict, LEVEL_NOT_HELD};
use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Kernel, PiecePosition, ProtectedSet, Puzzle, Snapshot};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| Kernel::load(&repository().join("assets")).expect("the 600-cell-Full model loads"))
}

fn i_a() -> Arc<Menu> {
    let bytes = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    let doc = json::parse(&bytes).expect("the I_a fixture is JSON");
    Arc::new(Menu::from_record(ctx(), doc.get("menu").expect("a menu")).expect("the pinned I_a menu"))
}

/// Retained generator 1, H_0, as the menu's letter at grip 0.
fn h0(menu: &Menu) -> Twist {
    let ctx = ctx();
    let twists = menu.for_cap(ctx, 0).expect("the menu's twists at grip 0");
    let h0 = ctx.kplus.gen_idx[0];
    twists.into_iter().find(|twist| twist.kidx == Some(h0)).expect("the I_a menu holds H_0 at grip 0")
}

/// The centre piece of cell `c`.
fn centre_of(c: u16) -> u32 {
    let model = &ctx().model;
    *model.centre_pieces.iter().find(|&&p| model.signature(p as usize) == [c]).expect("a centre piece")
}

/// The state after retained generator 1 from solved in 600-cell-Full, through the kernel's word `[1]`.
fn after_h0() -> (Snapshot, Snapshot) {
    let kernel = kernel();
    let solved = kernel.solved();
    let recipe = kernel.word(&[1]).expect("the kernel reads retained generator 1");
    match kernel.derive(&solved, &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => (solved, derived.state),
        Derivation::Refused(refused) => panic!("refused at label level: {}", refused.refusal),
    }
}

fn positions(state: &Snapshot, pieces: &[u32]) -> Vec<PiecePosition> {
    let pieces: Vec<i64> = pieces.iter().map(|&p| i64::from(p)).collect();
    kernel().positions(state, &pieces).expect("pieces of the model")
}

/// CT-LB3-02 (part: "a protection verdict carries its level and never crosses it"). The same rotation, H_0 about pole
/// 0, gets two verdicts for the centre piece of cell 0: inside Jumble the pose-level verdict says the piece moves, and
/// in 600-cell-Full, after the crossing at solved, the label-level verdict says it is clear, because H_0 turns the
/// piece with its labels unchanged. Each answers for its own puzzle. Asked for the other, each is refused with
/// `KN1:input.level_not_held`: the label pass grants no pose protection inside Jumble, and the pose refusal says
/// nothing about labels. Each verdict names the digest of the state it judged at its own level.
#[test]
fn ct_lb3_02_a_protection_verdict_never_crosses_its_level() {
    let ctx = ctx();
    let menu = i_a();
    let centre = centre_of(0);
    let h0 = h0(&menu);
    assert_eq!(ctx.kplus.word_matrix(&[1], &ctx.model).as_ref(), Some(h0.matrix.as_ref()), "kernel letter 1 is H_0");

    let mut state = State::new(Some(menu.clone()));
    let read = state.digest(ctx);
    let outcome = state.apply(ctx, &h0, None, &[centre]).expect("no engine error");
    assert_eq!(outcome.status, Status::Protected);
    assert_eq!(outcome.refusal.as_ref().map(|refusal| refusal.0), Some(codes::PIECE_WOULD_MOVE));
    let report = outcome.protection.as_ref().expect("a protection report");
    assert_eq!(report.to_json().get("level"), Some(&Value::String(Level::Poses.as_str().into())));
    let pose = Verdict::poses(report, &read);
    assert_eq!((pose.level(), pose.protected(), pose.read()), (Level::Poses, &[centre][..], read.as_str()));
    assert_eq!(pose.answer(Puzzle::Jumbling), Ok(&Finding::Moves(vec![centre])));
    assert_eq!(state.digest(ctx), read, "the refused twist changed nothing");

    let (solved, turned) = after_h0();
    let (before, after) = (positions(&solved, &[centre]), positions(&turned, &[centre]));
    let slots: Vec<i32> = (0..NS).filter(|&s| ctx.model.slot_piece[s] == centre).map(|s| s as i32).collect();
    assert_eq!(before[0].home, slots, "the kernel's row names the same piece");
    let label = Verdict::labels(&before, &after, solved.digest()).expect("rows of the same pieces");
    assert_eq!((label.level(), label.protected(), label.read()), (Level::Labels, &[centre][..], solved.digest()));
    assert_eq!(label.answer(Puzzle::Full), Ok(&Finding::Clear));

    let crossing = label.answer(Puzzle::Jumbling).expect_err("a label verdict grants no pose protection");
    assert_eq!(crossing, Crossing { held: Level::Labels, asked: Level::Poses });
    assert_eq!(crossing.code(), LEVEL_NOT_HELD);
    assert_eq!(LEVEL_NOT_HELD, "KN1:input.level_not_held");
    let crossing = pose.answer(Puzzle::Full).expect_err("a pose verdict answers nothing about labels");
    assert_eq!(crossing, Crossing { held: Level::Poses, asked: Level::Labels });
    assert_ne!(pose.answer(Puzzle::Jumbling), label.answer(Puzzle::Full), "the two levels judge H_0 differently");
}

/// CT-LB3-02 (part: no finding follows from the level). Protecting a piece of cap 0 whose stickers H_0 turns as well,
/// the label-level verdict finds that piece moving and the centre piece of cell 0 still clear; protecting the centre
/// piece of cell 1 as well, which lies outside cap 0, the pose-level verdict of an admitted H_0 is clear. Label rows of
/// different pieces before and after give no verdict.
#[test]
fn ct_lb3_02_each_level_finds_both_clear_and_moves() {
    let ctx = ctx();
    let (solved, turned) = after_h0();
    let zero = centre_of(0);
    let moving = (0..NP as u32)
        .filter(|&p| p != zero && ctx.model.in_sig(p as usize, 0))
        .find(|&p| positions(&solved, &[p]) != positions(&turned, &[p]))
        .expect("H_0 turns the stickers of a piece of cap 0");
    let pieces = [zero, moving];
    let label = Verdict::labels(&positions(&solved, &pieces), &positions(&turned, &pieces), solved.digest())
        .expect("rows of the same pieces");
    assert_eq!(label.protected(), &[zero.min(moving), zero.max(moving)][..]);
    assert_eq!(label.answer(Puzzle::Full), Ok(&Finding::Moves(vec![moving])));
    assert_eq!(label.answer(Puzzle::Jumbling), Err(Crossing { held: Level::Labels, asked: Level::Poses }));
    let other = Verdict::labels(&positions(&solved, &[moving]), &positions(&turned, &[zero]), solved.digest());
    assert_eq!(other, Err(RowsDiffer));

    let menu = i_a();
    let mut state = State::new(Some(menu.clone()));
    let read = state.digest(ctx);
    let outside = centre_of(1);
    assert!(!ctx.model.in_sig(outside as usize, 0), "the centre piece of cell 1 lies outside cap 0");
    let outcome = state.apply(ctx, &h0(&menu), None, &[outside]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
    let pose = Verdict::poses(outcome.protection.as_ref().expect("a protection report"), &read);
    assert_eq!(pose.answer(Puzzle::Jumbling), Ok(&Finding::Clear));
    assert_eq!(pose.answer(Puzzle::Full), Err(Crossing { held: Level::Poses, asked: Level::Labels }));
    assert_ne!(state.digest(ctx), read, "the admitted twist turned cap 0");
}
