//! CT-KN2-03 (whole; LP2 plan, section 11, item 7; LB3.4, CT-LB3-02): a protection verdict carries its level. With
//! centre piece 0 protected, every letter of the pinned I_a menu at grip 0 that turns the piece by a non-identity
//! element of A4₀ is refused at pose level inside Jumble with the report of selection 4, and nothing changes; the same
//! retained word, crossed at solved into 600-cell-Full, passes at label level: under M4's kernel no slot of the piece
//! moves.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use c600_jumble::model::NS;
use c600_jumble::state::{State, Status};
use c600_jumble::twist::Menu;
use c600_jumble::{codes, Context};
use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Kernel, ProtectedSet};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

#[test]
fn centre_piece_zero_is_refused_at_pose_level_and_passes_at_label_level() {
    let assets = repository().join("assets");
    let ctx = Context::load(&assets).expect("the jumbling model loads");
    let kernel = Kernel::load(&assets).expect("the 600-cell-Full model loads");
    let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    let fixture = json::parse(&fixture).expect("the I_a fixture is JSON");
    let menu = Arc::new(Menu::from_record(&ctx, fixture.get("menu").expect("a menu")).expect("the pinned I_a menu"));

    let centre = *ctx
        .model
        .centre_pieces
        .iter()
        .find(|&&p| ctx.model.signature(p as usize) == [0])
        .expect("cell 0 has a centre piece");
    let slots: Vec<i32> = (0..NS).filter(|&s| ctx.model.slot_piece[s] == centre).map(|s| s as i32).collect();
    assert!(!slots.is_empty(), "the centre piece of cell 0 has a slot");

    let a4 = ctx.kplus.a4(0);
    let twists = menu.for_cap(&ctx, 0).expect("the menu's twists at grip 0");
    let retained: Vec<(usize, u16)> = twists
        .iter()
        .enumerate()
        .filter_map(|(item, twist)| twist.kidx.filter(|k| *k != 0 && a4.contains(k)).map(|k| (item, k)))
        .collect();
    assert!(!retained.is_empty(), "the I_a menu has a retained letter at grip 0");

    for (item, k) in retained {
        // Pose level, inside Jumble: refused with the report of selection 4, and nothing changes.
        let mut state = State::new(Some(menu.clone()));
        let before = (state.digest(&ctx), state.lattice_digest(), state.record(&ctx));
        let outcome = state.apply(&ctx, &twists[item], None, &[centre]).expect("no engine error");
        assert_eq!(outcome.status, Status::Protected, "item {item}");
        assert_eq!(outcome.refusal.as_ref().map(|r| r.0), Some(codes::PIECE_WOULD_MOVE), "item {item}");
        let report = outcome.protection.as_ref().expect("a protection report");
        assert_eq!((report.protected.as_slice(), report.pieces.as_slice()), (&[centre][..], &[centre][..]));
        let report = report.to_json();
        assert_eq!(report.get("level"), Some(&Value::String("poses".into())));
        assert_eq!(report.get("mode"), Some(&Value::String("net".into())));
        assert_eq!(report.get("verdict"), Some(&Value::String("moves".into())));
        assert_eq!(report.get("first_refused_prefix"), Some(&Value::Null));
        assert_eq!((state.digest(&ctx), state.lattice_digest(), state.record(&ctx)), before, "item {item}");

        // Unprotected, the same letter is admissible and turns the piece.
        let outcome = state.apply(&ctx, &twists[item], None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "item {item}");
        assert!(outcome.inside.binary_search(&centre).is_ok(), "item {item} turns the centre piece");
        assert_ne!(state.digest(&ctx), before.0, "item {item} changes the pose digest");

        // Label level, after the crossing at solved: the retained word of the same rotation under M4's kernel.
        let word = ctx.kplus.a4_word(0, k).expect("every element of A4 has a retained word");
        assert!(word.iter().all(|&g| g.unsigned_abs() <= 2), "the word uses H_0 and T_0 only");
        assert_eq!(ctx.kplus.word_matrix(&word, &ctx.model).as_ref(), Some(ctx.kplus.matrix(k, &ctx.model)));
        let recipe = kernel.word(&word).expect("the kernel reads the retained word");
        let effect = kernel.effect(&recipe).expect("the effect is computed");
        assert!(effect.moved_stickers() > 0, "item {item}: the word turns cap 0");
        assert!(effect.source().iter().all(|s| slots.binary_search(s).is_err()), "item {item}: a centre slot moves");
        match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
            Derivation::Derived(derived) => {
                let labels = derived.state.labels();
                assert!(slots.iter().all(|&s| labels[s as usize] == s), "item {item}: a centre label changed");
            }
            Derivation::Refused(refused) => panic!("item {item}: refused at label level: {}", refused.refusal),
        }
    }
}
