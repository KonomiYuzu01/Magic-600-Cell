//! ADR-D02 checkpoint C3.2 (finding F1, 27 November 2026): a piece style is bound to the revision the shell computed it
//! for. A style named for a revision handed out but not adopted yet is held by the host and set with that revision's
//! adoption, so the revision's first submitted frame draws it (focus-hidden pieces never show for a frame).

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::ptr;

use c600_render::abi::{STYLE_CHANNEL_FILL, STYLE_CHANNEL_OUTLINE};
use c600_render::FakeCore;
use c600_shell_host::*;
use common::support::SIZE;
use common::{functions, Shim};

const PIECES: usize = 177_120;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

#[test]
fn the_first_frame_of_a_new_revision_draws_the_style_set_for_it() {
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    let fake = || unsafe { &mut (*shim).fake };
    let assets = checkout().join("assets").to_str().expect("UTF-8").to_owned();
    let menu = checkout().join("research/jumbling/fixtures/wj-I_a.json").to_str().expect("UTF-8").to_owned();
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("style-held-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let folder = folder.to_str().expect("UTF-8").to_owned();
    let mut host = ptr::null_mut();
    unsafe {
        assert_eq!(m6h_create(&functions(), shim.cast(), assets.as_ptr(), assets.len(), &mut host), M6H_OK);
        fake().complete();
        assert_eq!(m6h_pump(host), M6H_OK);
        assert_eq!(m6h_set_viewport(host, SIZE.0, SIZE.1), M6H_OK);
        let code = m6h_open_session(host, 1, folder.as_ptr(), folder.len(), menu.as_ptr(), menu.len(), ptr::null(), 0,
                                    ptr::null_mut());
        assert_eq!(code, M6H_OK);
        let mut frame = M6hFrame::default();
        assert_eq!(m6h_frame(host, &mut frame), M6H_OK);
        let at = frame.engine_revision;
        // The focus layer's dim bit on piece 3 and a selection outline on piece 7, set for the adopted revision.
        let mut fill = vec![0u8; PIECES];
        fill[3] = 0x80;
        let mut outline = vec![0u8; PIECES];
        outline[7] = 2;
        let set = |revision: u64, channel: u32, bytes: &[u8], by_pose: u32| {
            m6h_set_piece_style(host, revision, channel, bytes.as_ptr(), bytes.len() as u32, by_pose)
        };
        assert_eq!(set(at, STYLE_CHANNEL_FILL, &fill, 0), M6H_OK);
        assert_eq!(set(at, STYLE_CHANNEL_OUTLINE, &outline, 0), M6H_OK);
        assert!(fake().piece_styles[0].as_deref() == Some(&fill[..]), "the adopted revision's style reaches the core");
        // Before the press the next revision is not handed out: refused, and nothing changes.
        assert_eq!(set(at + 1, STYLE_CHANNEL_FILL, &fill, 0), M6H_E_INVALID_ARGUMENT, "a revision not handed out");
        let mut out = M6hKey::default();
        assert_eq!(m6h_key_down(host, "KeyK".as_ptr(), 4, 0, &mut out), M6H_OK);
        assert_eq!((out.press, out.engine_revision), (M6H_PRESS_COMMITTED, at + 1));
        // The shell sends its style for the adopted revision and, after a committed press, for the revision handed out
        // (Shell.cs at each step): the second is held. A revision past it, a per-pose style for it and a bad byte are
        // refused.
        assert_eq!(set(at, STYLE_CHANNEL_FILL, &fill, 0), M6H_OK);
        assert_eq!(set(at + 1, STYLE_CHANNEL_FILL, &fill, 0), M6H_OK, "a pending revision's fill is held");
        assert_eq!(set(at + 1, STYLE_CHANNEL_OUTLINE, &outline, 0), M6H_OK, "a pending revision's outline is held");
        assert_eq!(set(at + 2, STYLE_CHANNEL_FILL, &fill, 0), M6H_E_INVALID_ARGUMENT, "a revision not handed out");
        assert_eq!(set(at + 1, STYLE_CHANNEL_FILL, &fill[..4], 1), M6H_E_UNAVAILABLE, "a per-pose style for a pending one");
        assert_eq!(set(at + 1, STYLE_CHANNEL_OUTLINE, &fill, 0), M6H_E_INVALID_ARGUMENT, "the dim bit in an outline");
        let mut frames = 0;
        while frame.engine_revision < at + 1 {
            assert!(frames < 400, "the twist's revision was never adopted");
            assert_eq!(m6h_frame(host, &mut frame), M6H_OK);
            frames += 1;
        }
        // The first frame submitted at the new revision: nothing adopts after its submission, so the style the core
        // holds now is the style it drew with.
        assert_eq!(frame.engine_revision, at + 1);
        assert!(fake().piece_styles[0].as_deref() == Some(&fill[..]), "the fill set for the new revision is drawn");
        assert!(fake().piece_styles[1].as_deref() == Some(&outline[..]), "the outline set for the new revision is drawn");
        // The old revision's style no longer attaches to the new one: refused as stale, and the drawn style stays.
        assert_eq!(set(at, STYLE_CHANNEL_OUTLINE, &vec![0u8; PIECES], 0), M6H_E_UNAVAILABLE, "a stale revision");
        assert!(fake().piece_styles[1].as_deref() == Some(&outline[..]), "a stale style changes nothing");
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}
