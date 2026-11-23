//! CT-EV4-05, the headless part only (synthetic; EV4.4, EV4.6, EV4.7; LP4-8): IME composition, text undo and focus
//! loss suppress puzzle dispatch and clear held keys, on synthetic events under each declared layout. CT-EV4-05 is a
//! windows test: its half with actual Windows IME, text undo and focus loss runs on the owner's machine and is not run
//! here. Nothing here is Windows input or IME evidence.

mod common;

use c600_command::{word_draft, Status};
use c600_input::{DropReason, Focus, KeyEvent, Routed};

use common::{fixture, row, word, Fixture};

fn status(routed: &Routed) -> &Status {
    &routed.receipt().unwrap_or_else(|| panic!("nothing reached the layer: {routed:?}")).status
}

/// Previews, presents and commits `letters` by the router's keys.
fn commit(f: &mut Fixture, letters: &[i64]) {
    f.shell.draft = Some(word_draft(&word(letters)));
    let Status::Running(id) = *status(&f.tap("Ctrl+Enter")) else { panic!("the preview was not admitted") };
    assert_eq!(f.layer.run_jobs(), 1);
    f.frames.present(&mut f.shell, &mut f.layer, id).unwrap();
    assert!(matches!(status(&f.tap("Ctrl+Shift+Enter")), Status::Committed(_)));
}

/// CT-EV4-05 (headless part; not run: the actual Windows IME, text-undo and focus-loss half): a held puzzle key is
/// released when the focus enters a text field; IME composition and the text field's undo and redo, by the character
/// the active layout gives them, stay in the field and reach no row; a focus loss during composition releases every
/// held key; afterwards the keys held before repeat nothing until a fresh key-down, and only a fresh undo key-down
/// outside the text field undoes the commit.
#[test]
fn ime_text_undo_and_focus_loss_suppress_puzzle_dispatch() {
    for (layout, undo) in [("us-qwerty", "Ctrl+KeyZ"), ("fr-azerty", "Ctrl+KeyW")] {
        let mut f = fixture("ime", layout);
        commit(&mut f, &[1]);
        let (reached, revision) = (f.reached(), f.layer.revision().clone());

        assert_eq!(row(&f.down("ArrowRight")), "camera.rotate");
        f.router.set_focus(Focus::Text);
        assert!(f.router.held().is_empty(), "{layout}: entering the text field releases the held key");
        assert!(matches!(f.key(KeyEvent::Compose("かな".into())), Routed::Text));
        assert!(matches!(f.tap(undo), Routed::Text), "{layout}: text undo stays in the field");
        assert!(matches!(f.tap("Ctrl+KeyY"), Routed::Text), "{layout}: text redo stays in the field");
        assert!(matches!(f.key(KeyEvent::typed("KeyJ", 'j')), Routed::Text), "{layout}: a twist key types");
        assert!(matches!(f.key(KeyEvent::Compose("漢".into())), Routed::Text));
        assert!(matches!(f.key(KeyEvent::FocusLost), Routed::Released));
        assert!(f.router.held().is_empty() && f.router.pending().is_empty());

        f.router.set_focus(Focus::Puzzle);
        for chord in ["ArrowRight", "KeyJ", undo] {
            let repeat = f.key(KeyEvent::repeat(chord));
            assert!(matches!(repeat, Routed::Dropped(DropReason::NotHeld)), "{layout} {chord}: {repeat:?}");
        }
        assert_eq!(f.reached(), reached + 1, "{layout}: only the camera key-down reached the layer");
        assert_eq!(f.layer.revision(), &revision, "{layout}: nothing changed the state");

        assert!(matches!(status(&f.tap(undo)), Status::Committed(_)), "{layout}: a fresh undo key-down undoes");
    }
}
