//! CT-LB3-02, whole (LB3.4, KN1.11): centre piece 0 under retained generator 1 (H_0) distinguishes pose and label
//! protection, and a pose refusal cannot be replaced by the label pass at a crossing. Every verdict is an engine's own,
//! held by `c600_jumble_tool::protection` with its level: inside Jumble, M8's protection of a whole word
//! (`levels::derive`); in 600-cell-Full, the kernel's protection of named pieces, centres included
//! (`Kernel::piece_protection`, `Kernel::strict_piece_protection`); each under Net and Strict (KN2.7). The crossing at
//! solved is M8's: the label checkpoint and the retained checkpoint answer at their own levels, and `levels::project`
//! gives the labels that 600-cell-Full starts from.
//!
//! M5's `c600-jumble/tests/crossing.rs` runs CT-KN2-03, the facts beneath, letter by letter: every retained letter of
//! the pinned I_a menu at grip 0 that turns the centre piece, through one twist's protection (`State::apply`) and the
//! kernel's `derive`. This test does not repeat it.
//!
//! Not run here: the crossing as a commit row of the command layer (ONE.54, ONE.80), which waits for LP2's KN5. A pass
//! is source and fixture evidence.
//!
//! CT-LB3-02 (whole: the two `ct_lb3_02_*` tests of this file, on the pose and label halves of centre piece 0, run
//! both modes and both protection policies at M8's levels and the kernel's; `c600-jumble`'s `tests/crossing.rs` runs
//! the kernel facts beneath as CT-KN2-03).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::levels::{self, Base, LevelValue, Witness, LABEL_CHECKPOINT_LEVEL, RETAINED_CHECKPOINT_LEVEL};
use c600_jumble::model::NS;
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_jumble_tool::level::Level;
use c600_jumble_tool::protection::{Crossing, Finding, Verdict, LEVEL_NOT_HELD};
use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Kernel, PieceProtection, ProtectedSet, ProtectionMode, Puzzle, Snapshot};

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

/// Retained generator `mid` (signed and 1-based, as the kernel's letters) as a twist of the pinned menu at its grip.
fn letter(menu: &Menu, mid: i64) -> Twist {
    let twist = levels::primitive(ctx(), mid).expect("a retained generator");
    assert!(menu.contains(ctx(), &twist), "the menu holds retained generator {mid}");
    twist
}

/// The centre piece of cell `c`.
fn centre_of(c: u16) -> u32 {
    let model = &ctx().model;
    *model.centre_pieces.iter().find(|&&p| model.signature(p as usize) == [c]).expect("a centre piece")
}

/// M8's derivation of `twists` from `base`, with `protect` protected at pose level under `mode`.
fn pose_derivation(
    base: &levels::Snapshot,
    twists: &[Twist],
    mode: ProtectionMode,
    protect: &[u32],
) -> levels::Derivation {
    let ctx = ctx();
    let identity = ctx.identity(base.state().menu());
    levels::derive(ctx, Base::Snapshot(base), &identity, twists, mode, protect)
        .expect("no engine error")
        .expect("the base's own identity and pieces of the model")
}

/// The pose-level verdict of a derivation, from its value's protection report or its refusal's, on the base whose
/// pose digest is `read`.
fn pose_verdict(derivation: &levels::Derivation, read: &str) -> Verdict {
    let report = match derivation {
        levels::Derivation::Derived(derived) => derived.protection(),
        levels::Derivation::Refused(refused) => refused.protection.as_ref(),
    };
    Verdict::poses(report.expect("a protection report"), read)
}

/// The kernel's label-level protection of `pieces` for the word `letters` from `base`, under `mode`.
fn label_protection(base: &Snapshot, letters: &[i64], mode: ProtectionMode, pieces: &[u32]) -> PieceProtection {
    let kernel = kernel();
    let named: Vec<i64> = pieces.iter().map(|&p| i64::from(p)).collect();
    let set = kernel.protected_pieces(&named).expect("pieces of the model");
    let recipe = kernel.word(letters).expect("a word of retained generators");
    match mode {
        ProtectionMode::Net => {
            let effect = kernel.effect(&recipe).expect("no star certificate fails");
            kernel.piece_protection(base, &effect, &set)
        }
        ProtectionMode::Strict => kernel.strict_piece_protection(base, &recipe, &set),
    }
    .expect("values of the loaded model")
}

/// M8's labels of a lattice state, total (KN1.9).
fn total_labels(state: &State) -> levels::Labels {
    match levels::project(ctx(), state, levels::Level::Labels).expect("no engine error") {
        levels::Projection::Value(levels::Partial::Total(LevelValue::Labels(labels))) => labels,
        other => panic!("a lattice state projects to labels totally: {other:?}"),
    }
}

/// The level an end test's verdict names.
fn level_of(verdict: &Value) -> &str {
    verdict.get("level").and_then(Value::as_str).expect("a verdict names its level")
}

/// `verdict` answers `finding` for the puzzle of its own level and is refused with `KN1:input.level_not_held` for the
/// other.
fn answers_at_its_level_only(verdict: &Verdict, finding: &Finding) {
    let (own, other) = match verdict.level() {
        Level::Poses => (Puzzle::Jumbling, Puzzle::Full),
        Level::Labels => (Puzzle::Full, Puzzle::Jumbling),
    };
    assert_eq!(verdict.answer(own), Ok(finding));
    let crossing = verdict.answer(other).expect_err("a verdict never crosses its level");
    assert_eq!(crossing, Crossing { held: verdict.level(), asked: Level::of(other) });
    assert_eq!(crossing.code(), LEVEL_NOT_HELD);
}

/// CT-LB3-02. Inside Jumble the word [H_0] from solved, with centre piece 0 protected, is refused at pose level under
/// Net and under Strict, because H_0 turns the piece. Jumble off at solved is a label checkpoint, and from the labels
/// M8 projects, the kernel's word `[1]` passes at label level under both modes, because H_0 leaves the piece's labels
/// in place. Each verdict answers only at its own level and is refused at the other with `KN1:input.level_not_held`.
/// The label pass replaces nothing: inside Jumble the same word is refused again with the same report, and after `[1]`
/// is committed in 600-cell-Full, the state Jumble on replays has the centre piece turned with its labels unchanged,
/// which only the pose level sees.
#[test]
fn ct_lb3_02_a_pose_refusal_is_not_replaced_by_the_label_pass_at_the_crossing() {
    let (ctx, kernel) = (ctx(), kernel());
    let menu = i_a();
    let centre = centre_of(0);
    let h0 = letter(&menu, 1);
    assert_eq!(ctx.kplus.word_matrix(&[1], &ctx.model).as_ref(), Some(h0.matrix.as_ref()), "kernel letter 1 is H_0");
    let modes = [ProtectionMode::Net, ProtectionMode::Strict];

    // Inside Jumble, at pose level: Net refuses the whole word, Strict its first letter.
    let base = levels::Snapshot::solved(ctx, Some(menu.clone()));
    let read = base.state().digest(ctx);
    let mut refusals = Vec::new();
    for (mode, prefix) in modes.into_iter().zip([None, Some(1)]) {
        let derivation = pose_derivation(&base, &[h0.clone()], mode, &[centre]);
        let levels::Derivation::Refused(refused) = &derivation else {
            panic!("H_0 with centre piece 0 protected is admitted at pose level under {mode:?}");
        };
        let found = (refused.refusal.code, refused.outcome, refused.first_refused_prefix, refused.base.as_str());
        assert_eq!(found, (codes::PIECE_WOULD_MOVE, Status::Protected, prefix, read.as_str()));
        let pose = pose_verdict(&derivation, &read);
        assert_eq!((pose.level(), pose.mode(), pose.protected()), (Level::Poses, mode, &[centre][..]));
        assert_eq!((pose.first_refused_prefix(), pose.read()), (prefix, read.as_str()));
        answers_at_its_level_only(&pose, &Finding::Moves(vec![centre]));
        refusals.push(refused.clone());
    }
    assert_eq!(base.state().digest(ctx), read, "a refused word changes nothing");

    // Jumble off at solved (LB3.3, A5): a label checkpoint, with which the retained checkpoint of the empty journal
    // agrees; each end test's verdict names the level it read.
    let checkpoint = levels::label_checkpoint(ctx, base.state(), base.reachability(), None).expect("no engine error");
    let levels::Verdict::Yes(evidence) = &checkpoint else {
        panic!("solved is a label checkpoint: {checkpoint:?}");
    };
    assert_eq!(level_of(&checkpoint.to_json_at(LABEL_CHECKPOINT_LEVEL)), "labels");
    let retained = levels::retained_checkpoint(ctx, base.state(), Witness::Journal(&[])).expect("no engine error");
    assert_eq!((retained.name(), level_of(&retained.to_json_at(RETAINED_CHECKPOINT_LEVEL))), ("yes", "poses"));

    // 600-cell-Full starts from M8's projection of the poses to labels (KN1.9), never from a lift (KN1.10).
    let labels = total_labels(base.state());
    let digest = labels.digest();
    assert_eq!(evidence.get("labels_digest").and_then(Value::as_str), Some(digest.as_str()));
    let full = kernel.state_from_labels(labels.labels).expect("the labels of a lattice state are a permutation");
    assert_eq!((full.labels(), full.digest()), (kernel.solved().labels(), digest.as_str()));

    // In 600-cell-Full, at label level: the kernel's word [1] with centre piece 0 protected is clear in both modes.
    // The kernel's row names the piece by the home slots `c600_jumble` gives it.
    let home: Vec<i32> = (0..NS).filter(|&s| ctx.model.slot_piece[s] == centre).map(|s| s as i32).collect();
    let mut passes = Vec::new();
    for mode in modes {
        let report = label_protection(&full, &[1], mode, &[centre]);
        let rows: Vec<_> = report.rows.iter().map(|row| (row.piece, row.orbit, &row.before, &row.after)).collect();
        assert_eq!(rows, [(centre as usize, -1, &home, &home)], "H_0 leaves the labels of centre piece 0 in place");
        let label = Verdict::labels(&report, full.digest());
        assert_eq!((label.level(), label.mode(), label.protected()), (Level::Labels, mode, &[centre][..]));
        assert_eq!((label.first_refused_prefix(), label.read()), (None, full.digest()));
        answers_at_its_level_only(&label, &Finding::Clear);
        passes.push(label);
    }

    // The label pass replaces nothing: inside Jumble again, the word is refused with the same report.
    for (mode, refused) in modes.into_iter().zip(&refusals) {
        match pose_derivation(&base, &[h0.clone()], mode, &[centre]) {
            levels::Derivation::Refused(again) => assert_eq!(&again, refused, "{mode:?}"),
            levels::Derivation::Derived(_) => panic!("the label pass admitted H_0 inside Jumble under {mode:?}"),
        }
    }
    assert!(passes.iter().all(|pass| pass.read() != read), "the label pass read Digest<labels>, not the poses");

    // What the label pass cannot see: committed in 600-cell-Full, [1] leaves the centre piece's labels in place, while
    // Jumble on, which replays the retained moves since the crossing from the stored exit poses, here solved (LB3.3,
    // ONE.54), turns the piece by H_0. The two states agree at label level, where the turn does not show.
    let recipe = kernel.word(&[1]).expect("the kernel reads retained generator 1");
    let committed = match kernel.derive(&full, &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused at label level: {}", refused.refusal),
    };
    assert!(home.iter().all(|&s| committed.labels()[s as usize] == s), "a label of centre piece 0 moved");
    let replayed = levels::replay(ctx, Some(menu), &[h0.clone()], None).expect("no engine error");
    let replayed = replayed.expect("H_0 is admissible from solved");
    assert_eq!(replayed.pose_of(centre as usize).kidx(), h0.kidx, "Jumble on turns centre piece 0 by H_0");
    assert_eq!(total_labels(&replayed).labels.as_slice(), committed.labels());
}

/// CT-LB3-02 (part: each level judges both modes, and no finding follows from the level). For a piece of cap 0 whose
/// stickers H_0 moves, the word [H_0, H_0⁻¹] is clear under Net and refused at prefix 1 under Strict, at pose level
/// inside Jumble and at label level in 600-cell-Full alike, each verdict answering only at its own level. Protecting
/// that piece with centre piece 0, the label-level verdict of `[1]` finds it moving and the centre clear; protecting
/// the centre piece of cell 1, which lies outside cap 0, the pose-level verdict of [H_0] is clear in both modes.
#[test]
fn ct_lb3_02_each_level_judges_net_and_strict_and_finds_both_clear_and_moves() {
    let (ctx, kernel) = (ctx(), kernel());
    let menu = i_a();
    let (zero, h0, h0_inverse) = (centre_of(0), letter(&menu, 1), letter(&menu, -1));
    let base = levels::Snapshot::solved(ctx, Some(menu));
    let read = base.state().digest(ctx);
    let full = kernel.solved();
    let cap = ctx.model.cap_members(0);
    let moved = label_protection(&full, &[1], ProtectionMode::Net, cap).moved();
    let moving = *moved.first().expect("H_0 moves the stickers of a piece of cap 0") as u32;
    assert!(moving != zero && !moved.contains(&(zero as usize)), "H_0 moves no label of centre piece 0");

    let back = [h0.clone(), h0_inverse];
    for (mode, finding, prefix) in
        [(ProtectionMode::Net, Finding::Clear, None), (ProtectionMode::Strict, Finding::Moves(vec![moving]), Some(1))]
    {
        let pose = pose_verdict(&pose_derivation(&base, &back, mode, &[moving]), &read);
        let label = Verdict::labels(&label_protection(&full, &[1, -1], mode, &[moving]), full.digest());
        for verdict in [&pose, &label] {
            assert_eq!(
                (verdict.mode(), verdict.protected(), verdict.first_refused_prefix()),
                (mode, &[moving][..], prefix)
            );
            answers_at_its_level_only(verdict, &finding);
        }
        assert_eq!((pose.level(), label.level()), (Level::Poses, Level::Labels));
    }

    let label = Verdict::labels(&label_protection(&full, &[1], ProtectionMode::Net, &[zero, moving]), full.digest());
    assert_eq!(label.protected(), &[zero.min(moving), zero.max(moving)][..]);
    answers_at_its_level_only(&label, &Finding::Moves(vec![moving]));

    let outside = centre_of(1);
    assert!(!ctx.model.in_sig(outside as usize, 0), "the centre piece of cell 1 lies outside cap 0");
    for mode in [ProtectionMode::Net, ProtectionMode::Strict] {
        let derivation = pose_derivation(&base, &[h0.clone()], mode, &[outside]);
        let levels::Derivation::Derived(derived) = &derivation else {
            panic!("H_0 with the centre piece of cell 1 protected is refused under {mode:?}");
        };
        assert_ne!(derived.digest(ctx), read, "the admitted word turned cap 0");
        let pose = pose_verdict(&derivation, &read);
        assert_eq!((pose.level(), pose.mode(), pose.protected()), (Level::Poses, mode, &[outside][..]));
        answers_at_its_level_only(&pose, &Finding::Clear);
    }
}
