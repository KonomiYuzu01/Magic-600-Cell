//! ADR-D02 B18 rest: a sweep's transform named for a revision handed out but not adopted yet is held by the host and
//! set with that revision's adoption, so the revision's first frame draws phase 0, not the end pose.

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::ptr;

use c600_render::FakeCore;
use c600_shell_host::*;
use common::support::SIZE;
use common::{functions, Shim};

const PIECES: usize = 177_120;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

#[test]
fn a_transform_named_for_a_pending_revision_is_held_until_its_adoption() {
    let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
    let fake = || unsafe { &mut (*shim).fake };
    let assets = checkout().join("assets").to_str().expect("UTF-8").to_owned();
    let menu = checkout().join("research/jumbling/fixtures/wj-I_a.json").to_str().expect("UTF-8").to_owned();
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("sweep-held-{}", std::process::id()));
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
        // KeyK twists item 2 at the grip cap 0; its row is read at the head before the press.
        let (mut plane, mut angle, mut status, mut at) = ([0f32; 8], 0f32, 0u32, 0u64);
        let mut moving = vec![0u8; PIECES];
        let code = m6h_twist_transform(host, 0, 2, plane.as_mut_ptr(), &mut angle, moving.as_mut_ptr(), &mut status, &mut at);
        assert_eq!((code, status), (M6H_OK, 1));
        let set = |revision: u64, phase: f32| {
            m6h_set_transform(host, revision, plane.as_ptr(), angle, moving.as_ptr(), PIECES as u32, phase)
        };
        // Before the press the next revision is not handed out: refused, as B18-04 requires.
        assert_eq!(set(at + 1, 0.0), M6H_E_INVALID_ARGUMENT, "a revision not handed out");
        let mut out = M6hKey::default();
        assert_eq!(m6h_key_down(host, "KeyK".as_ptr(), 4, 0, &mut out), M6H_OK);
        assert_eq!((out.press, out.engine_revision), (M6H_PRESS_COMMITTED, at + 1));
        // Handed out, not adopted: held (the fake core has no sa2_transform_set, so a transform that reached it would
        // answer M6H_E_HOST). A revision past the handed-out one is still refused; a bad phase too.
        assert_eq!(set(at + 1, 0.0), M6H_OK, "a pending revision's transform is held");
        assert_eq!(set(at + 2, 0.0), M6H_E_INVALID_ARGUMENT, "a revision not handed out");
        assert_eq!(set(at + 1, 1.5), M6H_E_INVALID_ARGUMENT, "a phase above 1");
        // The adopted revision's transform still reaches the core, and so does a clear.
        assert_eq!(set(at, 0.5), M6H_E_HOST, "the adopted revision's transform reaches the core");
        assert_eq!(m6h_set_transform(host, 0, ptr::null(), 0.0, ptr::null(), 0, 0.0), M6H_E_HOST, "a clear reaches it");
        m6h_destroy(host);
        drop(Box::from_raw(shim));
    }
}
