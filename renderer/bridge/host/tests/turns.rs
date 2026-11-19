//! The shell's turns through the host's C interface (C9, charter E2; a draft), over the bridge's C shim and the fake
//! core on route B: a session in a fresh folder, twist keys committed through the command layer and adopted at the
//! next engine revision, frames that name the adopted and the published revision, keys that are no press, an undo
//! that the host shows and the control reports, the refusals, and the correctness control at the close. Every result
//! is a synthetic fixture outcome, not a GPU run and not input evidence.

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::ptr;

use c600_kernel::json::{self, Value};
use c600_render::FakeCore;
use c600_shell_host::*;
use common::support::SIZE;
use common::{functions, Shim};

const CAMERA: [f32; 16] = [0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn text(path: PathBuf) -> String {
    path.to_str().expect("a UTF-8 path").to_owned()
}

/// A session folder that does not exist yet, under the build's own temporary folder (never %TEMP%).
fn fresh(name: &str) -> String {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("c9-turns-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    text(path)
}

fn menu() -> String {
    text(checkout().join("research/jumbling/fixtures/wj-I_a.json"))
}

fn last_error(handle: *mut ShellHost) -> String {
    let mut size = 0u32;
    // SAFETY: a null buffer only asks for the size.
    assert_eq!(unsafe { m6h_last_error(handle, ptr::null_mut(), 0, &mut size) }, M6H_E_INVALID_ARGUMENT);
    let mut buffer = vec![0u8; size as usize];
    // SAFETY: the buffer holds `size` bytes.
    assert_eq!(unsafe { m6h_last_error(handle, buffer.as_mut_ptr(), size, &mut size) }, M6H_OK);
    assert_eq!(buffer.pop(), Some(0));
    String::from_utf8(buffer).expect("UTF-8")
}

/// The control's record after the close.
fn record(handle: *mut ShellHost) -> Value {
    let mut size = 0u32;
    // SAFETY: a null buffer only asks for the size.
    assert_eq!(unsafe { m6h_check(handle, ptr::null_mut(), 0, &mut size) }, M6H_E_INVALID_ARGUMENT);
    let mut buffer = vec![0u8; size as usize];
    // SAFETY: the buffer holds `size` bytes.
    assert_eq!(unsafe { m6h_check(handle, buffer.as_mut_ptr(), size, &mut size) }, M6H_OK);
    assert_eq!(buffer.pop(), Some(0));
    json::parse(&buffer).expect("the record is JSON")
}

struct Run {
    shim: *mut Shim,
    host: *mut ShellHost,
}

impl Run {
    /// A host on the fake core with its first publication installed and a viewport and camera set.
    fn new() -> Run {
        let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
        let assets = text(checkout().join("assets"));
        let mut host = ptr::null_mut();
        // SAFETY: the table is the shim's, the context a live shim, and the path's bytes are readable.
        let code = unsafe { m6h_create(&functions(), shim.cast(), assets.as_ptr(), assets.len(), &mut host) };
        assert_eq!(code, M6H_OK, "{}", last_error(ptr::null_mut()));
        let run = Run { shim, host };
        run.fake().complete();
        // SAFETY: a live handle and 16 readable floats.
        unsafe {
            assert_eq!(m6h_pump(host), M6H_OK);
            assert_eq!(m6h_set_viewport(host, SIZE.0, SIZE.1), M6H_OK);
            assert_eq!(m6h_set_camera(host, CAMERA.as_ptr()), M6H_OK);
        }
        run
    }

    #[allow(clippy::mut_from_ref)]
    fn fake(&self) -> &mut FakeCore {
        // SAFETY: the host uses the shim only inside its calls, never at the same time as the test.
        unsafe { &mut (*self.shim).fake }
    }

    fn open(&self, folder: &str, menu: &str) -> i32 {
        // SAFETY: a live handle and readable bytes.
        unsafe { m6h_open_session(self.host, folder.as_ptr(), folder.len(), menu.as_ptr(), menu.len()) }
    }

    fn down(&self, chord: &str, repeat: bool) -> (i32, M6hKey) {
        let mut out = M6hKey::default();
        // SAFETY: a live handle, readable bytes and a writable row.
        let code = unsafe { m6h_key_down(self.host, chord.as_ptr(), chord.len(), u32::from(repeat), &mut out) };
        (code, out)
    }

    /// A key-down and its key-up; the press it was and the adopted revision after it.
    fn press(&self, chord: &str) -> (u32, u64) {
        let (code, key) = self.down(chord, false);
        assert_eq!(code, M6H_OK, "{chord}: {}", last_error(self.host));
        // SAFETY: a live handle and readable bytes.
        assert_eq!(unsafe { m6h_key_up(self.host, chord.as_ptr(), chord.len()) }, M6H_OK);
        (key.press, key.engine_revision)
    }

    fn frame(&self) -> M6hFrame {
        let mut submitted = M6hFrame::default();
        // SAFETY: a live handle and a writable row.
        assert_eq!(unsafe { m6h_frame(self.host, &mut submitted) }, M6H_OK, "{}", last_error(self.host));
        submitted
    }

    /// Confirms every submitted frame up to the last and reads the confirmations.
    fn present(&self) -> Vec<M6hPresented> {
        self.fake().complete();
        let last = self.fake().submitted.len() as u64;
        assert!(self.fake().show(last));
        let mut rows = vec![M6hPresented::default(); 16];
        let mut count = 0u32;
        // SAFETY: `rows` holds 16 rows.
        assert_eq!(unsafe { m6h_read_presented(self.host, rows.as_mut_ptr(), 16, &mut count) }, M6H_OK);
        rows.truncate(count as usize);
        rows
    }

    fn close(&self, shown_revision: u64) -> (i32, u32) {
        let mut pass = u32::MAX;
        // SAFETY: a live handle and a writable flag.
        let code = unsafe { m6h_close_session(self.host, shown_revision, &mut pass) };
        (code, pass)
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        // SAFETY: the handle is not used again; the shim outlives it.
        unsafe {
            m6h_destroy(self.host);
            drop(Box::from_raw(self.shim));
        }
    }
}

fn int(record: &Value, key: &str) -> Option<i64> {
    record.get(key).and_then(Value::as_i64)
}

#[test]
fn twist_keys_commit_through_the_command_layer_and_the_control_passes() {
    assert_eq!(m6h_abi_version(), 2);
    let run = Run::new();
    let folder = fresh("pass");

    // Keys before the session opens are refused.
    assert_eq!(run.down("KeyJ", false).0, M6H_E_SESSION);
    assert!(last_error(run.host).contains("no session"), "{}", last_error(run.host));
    assert_eq!(run.open(&folder, &menu()), M6H_OK, "{}", last_error(run.host));
    assert_eq!(run.open(&fresh("again"), &menu()), M6H_E_SESSION, "a handle opens one session");
    assert!(Path::new(&folder).join("objects").is_dir(), "the journal's folder");

    // KeyJ commits item 1 of the I_a menu at cap 0: the core adopts revision 2, and the view is published for it.
    assert_eq!(run.fake().adoptions.len(), 1);
    let prepared = run.fake().prepared.len();
    assert_eq!(run.press("KeyJ"), (M6H_PRESS_COMMITTED, 2));
    assert_eq!(run.fake().adoptions.len(), 2);
    assert_eq!(run.fake().adoptions[1].0, 2, "the core adopts the next revision");
    assert_eq!(run.fake().prepared.len(), prepared + 1, "the view is published again");

    // Until the new publication installs, frames draw revision 2's labels over revision 1's publication.
    let before = run.frame();
    assert_eq!((before.engine_revision, before.publication_revision), (2, 1));
    run.fake().complete();
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_pump(run.host) }, M6H_OK);
    let after = run.frame();
    assert_eq!((after.engine_revision, after.publication_revision), (2, 2));

    // Shift+KeyK commits item 5 (KeyK's inverse); a repeat, a key without a built row and a focus loss are no press.
    assert_eq!(run.press("Shift+KeyK"), (M6H_PRESS_COMMITTED, 3));
    let (code, key) = run.down("KeyL", true);
    assert_eq!((code, key.press, key.engine_revision), (M6H_OK, M6H_PRESS_NONE, 3));
    assert_eq!(run.press("Escape"), (M6H_PRESS_NONE, 3));
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_focus_lost(run.host) }, M6H_OK);
    assert_eq!(run.down("KeyNope", false).0, M6H_E_INVALID_ARGUMENT);
    assert_eq!(run.down("Hyper+KeyJ", false).0, M6H_E_INVALID_ARGUMENT);
    assert_eq!(run.fake().adoptions.len(), 3, "only the two commits were adopted");

    // The newest presented frame names revision 3; the control at the close passes.
    run.fake().complete();
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_pump(run.host) }, M6H_OK);
    let last = run.frame();
    assert_eq!((last.engine_revision, last.publication_revision), (3, 3));
    let shown = run.present();
    assert_eq!(shown.last().map(|row| (row.sequence, row.engine_revision)), Some((last.sequence, 3)));
    assert_eq!(run.close(3), (M6H_OK, 1), "{}", last_error(run.host));
    let control = record(run.host);
    assert_eq!(control.get("control").and_then(Value::as_str), Some("journal-reopen-and-kernel-replay"));
    assert_eq!(control.get("pass"), Some(&Value::Bool(true)), "{}", control.canonical());
    assert_eq!((int(&control, "committed"), int(&control, "refused")), (Some(2), Some(0)));
    assert_eq!((int(&control, "other_changes"), int(&control, "shown_revision")), (Some(0), Some(3)));
    let live = control.get("live").and_then(Value::as_str).expect("the live digest");
    assert_eq!(control.get("reopened").and_then(Value::as_str), Some(live));
    assert_eq!(control.get("replayed").and_then(Value::as_str), Some(live));
    assert_eq!(control.get("shown").and_then(Value::as_str), Some(live));

    // After the close, keys, a second close and a new session are refused.
    assert_eq!(run.down("KeyJ", false).0, M6H_E_SESSION);
    assert_eq!(run.close(3).0, M6H_E_SESSION);
    assert_eq!(run.open(&fresh("after"), &menu()), M6H_E_SESSION);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn an_undo_is_shown_and_fails_the_control() {
    let run = Run::new();
    let folder = fresh("undo");
    assert_eq!(run.open(&folder, &menu()), M6H_OK, "{}", last_error(run.host));
    assert_eq!(run.press("KeyJ"), (M6H_PRESS_COMMITTED, 2));
    // Ctrl+Z is the draft map's `session.undo`, which the layer builds: the host adopts the solved state again.
    assert_eq!(run.press("Ctrl+KeyZ"), (M6H_PRESS_OTHER, 3));
    assert_eq!(run.fake().adoptions.len(), 3);
    assert_eq!(run.close(3), (M6H_OK, 0));
    let control = record(run.host);
    assert_eq!(control.get("pass"), Some(&Value::Bool(false)));
    assert_eq!((int(&control, "committed"), int(&control, "other_changes")), (Some(1), Some(1)));
    assert_eq!(control.get("reopened"), control.get("live"), "the journal still replays the live state");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_shown_revision_the_host_never_adopted_fails_the_control() {
    let run = Run::new();
    let folder = fresh("unseen");
    assert_eq!(run.open(&folder, &menu()), M6H_OK, "{}", last_error(run.host));
    assert_eq!(run.press("KeyJ"), (M6H_PRESS_COMMITTED, 2));
    assert_eq!(run.close(5), (M6H_OK, 0));
    let control = record(run.host);
    assert_eq!(control.get("pass"), Some(&Value::Bool(false)));
    assert_eq!(control.get("expected_shown"), Some(&Value::Null));
    let _ = std::fs::remove_dir_all(&folder);

    // No frame presented at all: the control cannot tie the screen to the journal, so it fails too.
    let run = Run::new();
    let folder = fresh("none");
    assert_eq!(run.open(&folder, &menu()), M6H_OK, "{}", last_error(run.host));
    assert_eq!(run.close(0), (M6H_OK, 0));
    assert_eq!(record(run.host).get("shown_revision"), Some(&Value::Null));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_session_does_not_open_on_a_used_folder_or_a_missing_menu() {
    let run = Run::new();
    let used = fresh("used");
    std::fs::create_dir_all(&used).expect("a folder");
    std::fs::write(Path::new(&used).join("journal"), b"x").expect("a file");
    assert_eq!(run.open(&used, &menu()), M6H_E_SESSION);
    assert!(last_error(run.host).contains("not empty"), "{}", last_error(run.host));
    let missing = text(checkout().join("research/jumbling/fixtures/no-such-menu.json"));
    assert_eq!(run.open(&fresh("menu"), &missing), M6H_E_SESSION);
    assert!(last_error(run.host).contains("twist menu"), "{}", last_error(run.host));
    // SAFETY: invalid UTF-8 and null pointers are refused before any use.
    unsafe {
        assert_eq!(m6h_open_session(run.host, [0xFFu8].as_ptr(), 1, ptr::null(), 0), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_open_session(ptr::null_mut(), ptr::null(), 0, ptr::null(), 0), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_key_down(ptr::null_mut(), ptr::null(), 0, 0, ptr::null_mut()), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_close_session(run.host, 0, ptr::null_mut()), M6H_E_INVALID_ARGUMENT);
        let mut size = 0u32;
        assert_eq!(m6h_check(ptr::null_mut(), ptr::null_mut(), 0, &mut size), M6H_E_INVALID_ARGUMENT);
    }
    // Before a close the record is empty.
    let mut size = 0u32;
    // SAFETY: a null buffer only asks for the size.
    assert_eq!(unsafe { m6h_check(run.host, ptr::null_mut(), 0, &mut size) }, M6H_E_INVALID_ARGUMENT);
    assert_eq!(size, 1);
    let _ = std::fs::remove_dir_all(&used);
}
