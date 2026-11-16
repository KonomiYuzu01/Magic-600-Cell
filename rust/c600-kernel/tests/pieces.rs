//! LP2, label-level protection of named pieces in 600-cell-Full (KN2.4 `protection`, LB3.4), which LP3's C8 waits for
//! (`docs/progress/1.0/language-lp3-plan.md`): a [`ProtectedSet`] names moving orbits 0 to 34 only, so a fixed cell
//! centre had no label-level row. Here any piece, centres included, is protected, under Net
//! (`Kernel::piece_protection`) and Strict (`Kernel::strict_piece_protection`), and each row agrees with KN2.4's
//! `positions` rows of the base and the derived state. A pass is source evidence.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_kernel::{
    codes, Derivation, Kernel, Level, PieceProtection, ProtectedPieces, ProtectedSet, ProtectionMode, Recipe, Snapshot,
    Verdict, PIECES,
};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => kernel,
        Err(error) => panic!("the model in assets/ does not load: {error}"),
    })
}

fn word(letters: &[i64]) -> Recipe {
    kernel().word(letters).expect("a word of legal letters")
}

fn derived(base: &Snapshot, recipe: &Recipe) -> Snapshot {
    match kernel().derive(base, recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused without protection: {}", refused.refusal),
    }
}

fn pieces(named: &[usize]) -> ProtectedPieces {
    let named: Vec<i64> = named.iter().map(|&p| p as i64).collect();
    kernel().protected_pieces(&named).expect("pieces of the model")
}

fn net(base: &Snapshot, recipe: &Recipe, set: &ProtectedPieces) -> PieceProtection {
    let effect = kernel().effect(recipe).expect("no star certificate fails");
    kernel().piece_protection(base, &effect, set).expect("values of the loaded model")
}

fn strict(base: &Snapshot, recipe: &Recipe, set: &ProtectedPieces) -> PieceProtection {
    kernel().strict_piece_protection(base, recipe, set).expect("values of the loaded model")
}

/// The centre piece of cell 0 and a piece whose stickers retained generator 1 (H_0) moves, found from KN2.4's rows.
fn centre_and_moved() -> (usize, usize) {
    let kernel = kernel();
    let (solved, turned) = (kernel.solved(), derived(&kernel.solved(), &word(&[1])));
    let every: Vec<i64> = (0..PIECES as i64).collect();
    let (before, after) = (kernel.positions(&solved, &every).unwrap(), kernel.positions(&turned, &every).unwrap());
    let centre = before.iter().find(|row| row.orbit < 0 && row.home[0] / 433 == 0).expect("the centre of cell 0").piece;
    let moved = before.iter().zip(&after).find(|(b, a)| b.orbit >= 0 && b.slots != a.slots).expect("a moved piece");
    (centre, moved.0.piece)
}

/// CT-LB3-02's label half from the kernel itself: centre piece 0 under retained generator 1 from solved has a
/// label-level row, and it is clear under Net and Strict, because H_0 turns the piece with its label unchanged, while a
/// piece whose stickers H_0 moves is refused at prefix 1 under Strict and moves under Net. Each row is KN2.4's
/// `positions` row of the base and of the derived state.
#[test]
fn a_centre_piece_has_a_label_row_that_h0_leaves_clear() {
    let kernel = kernel();
    let (centre, moved) = centre_and_moved();
    let solved = kernel.solved();
    let one = word(&[1]);
    let turned = derived(&solved, &one);
    let set = pieces(&[moved, centre]);
    assert_eq!(set.pieces(), &[centre.min(moved), centre.max(moved)][..]);

    for verdict in [net(&solved, &one, &set), strict(&solved, &one, &set)] {
        assert_eq!(verdict.level, Level::Labels);
        assert_eq!(verdict.protected, set.pieces());
        assert_eq!(verdict.verdict, Verdict::Moves);
        assert_eq!(verdict.moved(), vec![moved]);
        let named: Vec<i64> = set.pieces().iter().map(|&p| p as i64).collect();
        let (before, after) = (kernel.positions(&solved, &named).unwrap(), kernel.positions(&turned, &named).unwrap());
        for ((row, b), a) in verdict.rows.iter().zip(&before).zip(&after) {
            assert_eq!((row.piece, row.orbit, &row.before, &row.after), (b.piece, b.orbit, &b.slots, &a.slots));
        }
        let centre_row = verdict.rows.iter().find(|row| row.piece == centre).expect("the centre's row");
        assert!(centre_row.orbit < 0 && !centre_row.moves(), "H_0 moves the centre's label");
    }
    assert_eq!(net(&solved, &one, &set).mode, ProtectionMode::Net);
    assert_eq!(net(&solved, &one, &set).first_refused_prefix, None);
    let refused = strict(&solved, &one, &set);
    assert_eq!((refused.mode, refused.first_refused_prefix), (ProtectionMode::Strict, Some(1)));

    let alone = pieces(&[centre]);
    for verdict in [net(&solved, &one, &alone), strict(&solved, &one, &alone)] {
        assert_eq!((verdict.verdict, verdict.first_refused_prefix, verdict.rows.len()), (Verdict::Clear, None, 1));
    }
}

/// No letter moves a fixed cell centre's sticker, so every centre's row is clear under every generator, under Net and
/// Strict alike.
#[test]
fn no_generator_moves_a_centre_label() {
    let kernel = kernel();
    let every: Vec<i64> = (0..PIECES as i64).collect();
    let centres: Vec<usize> =
        kernel.positions(&kernel.solved(), &every).unwrap().iter().filter(|r| r.orbit < 0).map(|r| r.piece).collect();
    assert_eq!(centres.len(), 600);
    let set = pieces(&centres);
    let letters: Vec<i64> = (1..=1_200).chain([-1, -600, -1_200]).collect();
    let verdict = strict(&kernel.solved(), &word(&letters), &set);
    assert_eq!((verdict.verdict, verdict.first_refused_prefix, verdict.rows.len()), (Verdict::Clear, None, 600));
    assert!(verdict.rows.iter().all(|row| row.before == row.after && row.orbit < 0));
}

/// Strict judges every literal prefix and Net only the end: `[1, -1]` moves the piece and puts it back, so Net is clear
/// and Strict refuses at prefix 1 with the slots after that letter. A first letter that leaves the piece in place moves
/// the refusal to prefix 2. A piece's row depends on the base: from the state after H_0 its stickers start elsewhere.
#[test]
fn strict_judges_each_prefix_and_rows_follow_the_base() {
    let kernel = kernel();
    let (_, moved) = centre_and_moved();
    let set = pieces(&[moved]);
    let solved = kernel.solved();

    let back = word(&[1, -1]);
    let clear = net(&solved, &back, &set);
    assert_eq!((clear.verdict, clear.rows[0].before == clear.rows[0].after), (Verdict::Clear, true));
    let refused = strict(&solved, &back, &set);
    assert_eq!((refused.verdict, refused.first_refused_prefix), (Verdict::Moves, Some(1)));
    assert_eq!(refused.rows, net(&solved, &word(&[1]), &set).rows, "the rows after prefix 1 are those of word([1])");

    let elsewhere = (2..=1_200)
        .find(|&m| !net(&solved, &word(&[m]), &set).rows[0].moves())
        .expect("a generator that leaves the piece in place");
    let later = strict(&solved, &word(&[elsewhere, 1]), &set);
    assert_eq!(later.first_refused_prefix, Some(2));
    assert_eq!(later.rows, net(&solved, &word(&[1]), &set).rows);

    let turned = derived(&solved, &word(&[1]));
    let from_turned = net(&turned, &word(&[-1]), &set);
    let row = &from_turned.rows[0];
    assert_ne!(row.before, kernel.positions(&solved, &[moved as i64]).unwrap()[0].slots);
    assert_eq!(row.after, kernel.positions(&solved, &[moved as i64]).unwrap()[0].home, "H_0⁻¹ brings the piece home");
}

/// Strict against a scan that derives every prefix and compares the protected pieces' `positions` rows with the
/// base's. From a scrambled base, a word whose first nine letters leave the pieces in place is refused at prefix 10,
/// where the scan first sees a moved sticker, with that prefix's slots; a word that never moves them is clear; and an
/// arbitrary word and a letter with its inverse get the scan's answer.
#[test]
fn strict_agrees_with_a_scan_of_every_prefix() {
    let kernel = kernel();
    let base = derived(&kernel.solved(), &word(&[5, -17, 233, 1_000, -64, 777, 2]));
    let (centre, moved) = centre_and_moved();
    let set = pieces(&[centre, moved]);
    let named: Vec<i64> = set.pieces().iter().map(|&p| p as i64).collect();
    let moves = |m: i64| net(&base, &word(&[m]), &set).verdict == Verdict::Moves;
    let still: Vec<i64> = (1..=1_200).flat_map(|m| [m, -m]).filter(|&m| !moves(m)).take(12).collect();
    let hit = (1..=1_200).find(|&m| moves(m)).expect("a generator that moves a protected sticker");
    let late: Vec<i64> = still[..9].iter().copied().chain([hit]).chain(still[9..].iter().copied()).collect();
    let mixed: Vec<i64> =
        (0..20).map(|i| (i * 389 + 17) % 1_200 + 1).map(|m| if m % 3 == 0 { -m } else { m }).collect();
    assert_eq!(strict(&base, &word(&late), &set).first_refused_prefix, Some(10));

    let home = kernel.positions(&base, &named).unwrap();
    for letters in [late, still, mixed, vec![hit, -hit]] {
        let scan = (1..=letters.len()).find_map(|n| {
            let at = kernel.positions(&derived(&base, &word(&letters[..n])), &named).unwrap();
            at.iter().zip(&home).any(|(a, h)| a.slots != h.slots).then_some((n as u64, at))
        });
        let verdict = strict(&base, &word(&letters), &set);
        assert_eq!(verdict.first_refused_prefix, scan.as_ref().map(|(n, _)| *n), "{letters:?}");
        let after = scan.map_or_else(|| home.clone(), |(_, at)| at);
        assert_eq!(verdict.rows.len(), home.len());
        for ((row, h), a) in verdict.rows.iter().zip(&home).zip(&after) {
            assert_eq!((&row.before, &row.after), (&h.slots, &a.slots), "{letters:?}");
        }
    }
}

/// The set is checked as `positions` checks a piece, repeats collapse, the empty set judges nothing, and every verdict
/// of the empty set is clear.
#[test]
fn protected_pieces_name_pieces_of_the_model() {
    let kernel = kernel();
    for bad in [-1, PIECES as i64] {
        let refusal = kernel.protected_pieces(&[3, bad]).unwrap_err();
        assert_eq!(
            (refusal.code, refusal.message.as_str()),
            (codes::NAMED_OBJECT_INVALID, "Position must be an integer in 0..177119")
        );
    }
    let set = kernel.protected_pieces(&[9, 3, 9]).unwrap();
    assert_eq!((set.pieces(), set.model_id()), (&[3, 9][..], Some(kernel.identity().model_id.as_str())));
    let empty = kernel.protected_pieces(&[]).unwrap();
    assert_eq!(empty, ProtectedPieces::none());
    assert_eq!(empty.model_id(), None);
    let one = word(&[1]);
    for verdict in [net(&kernel.solved(), &one, &empty), strict(&kernel.solved(), &one, &empty)] {
        assert_eq!((verdict.verdict, verdict.rows.len(), verdict.first_refused_prefix), (Verdict::Clear, 0, None));
    }
}
