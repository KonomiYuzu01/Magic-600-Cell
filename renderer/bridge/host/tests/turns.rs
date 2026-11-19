//! The shell's turns through the host's C interface (C9, charter E2; a draft), over the bridge's C shim and the fake
//! core on route B: a session in a fresh folder, twist keys committed through the command layer and adopted at the
//! next engine revision, frames that name the adopted and the published revision, keys that are no press, an undo
//! that the host shows and the control reports, the refusals, and the correctness control at the close. Charter E3 (a
//! draft): a session fixture written through the same turns, a copy of it reopened at the revision that counts its
//! presses, its refusals, and the runtime load's refusals. Every result is a synthetic fixture outcome, not a GPU run
//! and not input evidence.

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Arc;

use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;
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
fn record_of(handle: *mut ShellHost) -> Value {
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
        // SAFETY: a live handle and readable bytes; no fixture and no revision output.
        unsafe {
            m6h_open_session(
                self.host,
                folder.as_ptr(),
                folder.len(),
                menu.as_ptr(),
                menu.len(),
                ptr::null(),
                0,
                ptr::null_mut(),
            )
        }
    }

    /// Opens a copy of a fixture's session; the code and the revision the session opened at.
    fn reopen(&self, folder: &str, menu: &str, fixture: &str) -> (i32, u64) {
        let mut revision = 0u64;
        // SAFETY: a live handle, readable bytes and a writable revision.
        let code = unsafe {
            m6h_open_session(
                self.host,
                folder.as_ptr(),
                folder.len(),
                menu.as_ptr(),
                menu.len(),
                fixture.as_ptr(),
                fixture.len(),
                &mut revision,
            )
        };
        (code, revision)
    }

    fn load_runtime(&self, program: &str) -> i32 {
        // SAFETY: a live handle and readable bytes.
        unsafe { m6h_load_runtime(self.host, program.as_ptr(), program.len()) }
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
    assert_eq!(m6h_abi_version(), 3);
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
    let control = record_of(run.host);
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
    let control = record_of(run.host);
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
    let control = record_of(run.host);
    assert_eq!(control.get("pass"), Some(&Value::Bool(false)));
    assert_eq!(control.get("expected_shown"), Some(&Value::Null));
    let _ = std::fs::remove_dir_all(&folder);

    // No frame presented at all: the control cannot tie the screen to the journal, so it fails too.
    let run = Run::new();
    let folder = fresh("none");
    assert_eq!(run.open(&folder, &menu()), M6H_OK, "{}", last_error(run.host));
    assert_eq!(run.close(0), (M6H_OK, 0));
    assert_eq!(record_of(run.host).get("shown_revision"), Some(&Value::Null));
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
        let none = ptr::null_mut();
        assert_eq!(m6h_open_session(run.host, [0xFFu8].as_ptr(), 1, ptr::null(), 0, ptr::null(), 0, none), 1);
        assert_eq!(m6h_open_session(ptr::null_mut(), ptr::null(), 0, ptr::null(), 0, ptr::null(), 0, none), 1);
        let (folder, menu) = (fresh("null-fixture"), menu());
        let (f, m) = ((folder.as_ptr(), folder.len()), (menu.as_ptr(), menu.len()));
        assert_eq!(m6h_open_session(run.host, f.0, f.1, m.0, m.1, ptr::null(), 3, none), M6H_E_INVALID_ARGUMENT);
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

/// Copies the folder `from` into the new folder `to`, as the launcher copies a fixture's session.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a folder");
    for entry in std::fs::read_dir(from).expect("a folder") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("a copy");
        }
    }
}

/// A fixture of `keys` in a new folder; its folder and record.
fn write_fixture(name: &str, keys: &[&str]) -> (PathBuf, Value) {
    let assets = checkout().join("assets");
    let kernel = Arc::new(Kernel::load(&assets).expect("the model"));
    let out = PathBuf::from(fresh(name));
    let record = fixture::write(kernel, &assets, Path::new(&menu()), &out, keys).expect("the fixture");
    (out, record)
}

const KEYS: [&str; 3] = ["KeyJ", "Shift+KeyK", "KeyL"];

#[test]
fn a_fixture_session_reopens_at_the_revision_that_counts_its_presses() {
    let (out, record) = write_fixture("fixture", &KEYS);
    assert_eq!(record.get("format").and_then(Value::as_str), Some(fixture::FORMAT));
    assert_eq!(
        (int(&record, "revision"), record.get("committed").and_then(Value::as_array).map(<[_]>::len)),
        (Some(4), Some(3))
    );
    assert_eq!(record.get("control").and_then(|c| c.get("pass")), Some(&Value::Bool(true)));
    assert!(out.join("session").join("lock").is_file(), "the fixture keeps its session's lock file");
    let read = fixture::read(&out.join(fixture::RECORD)).expect("the record reads");
    assert_eq!((read.revision, read.committed.len()), (4, 3));
    let assets = checkout().join("assets");
    let kernel = Arc::new(Kernel::load(&assets).expect("the model"));
    let again = fixture::write(kernel, &assets, Path::new(&menu()), &out, &KEYS);
    assert!(again.is_err_and(|e| e.contains("never replaced")));
    let record_path = text(out.join(fixture::RECORD));

    // A copy reopens at revision 4: the core adopts it, and the view is published for it.
    let run = Run::new();
    let copy = fresh("fixture-copy");
    copy_dir(&out.join("session"), Path::new(&copy));
    assert_eq!(run.reopen(&copy, &menu(), &record_path), (M6H_OK, 4), "{}", last_error(run.host));
    assert_eq!(run.fake().adoptions.iter().map(|a| a.0).collect::<Vec<_>>(), vec![1, 4]);
    let before = run.frame();
    assert_eq!((before.engine_revision, before.publication_revision), (4, 1));
    run.fake().complete();
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_pump(run.host) }, M6H_OK);
    let first = run.frame();
    assert_eq!((first.engine_revision, first.publication_revision), (4, 4), "charter E3's first correct frame");

    // A press continues the session at revision 5; the control replays the fixture's presses and the run's.
    assert_eq!(run.press("KeyJ"), (M6H_PRESS_COMMITTED, 5));
    run.fake().complete();
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_pump(run.host) }, M6H_OK);
    run.frame();
    run.present();
    assert_eq!(run.close(5), (M6H_OK, 1), "{}", last_error(run.host));
    let control = record_of(run.host);
    assert_eq!(control.get("pass"), Some(&Value::Bool(true)), "{}", control.canonical());
    assert_eq!(int(&control, "committed"), Some(4));
    drop(run);

    // A cold start shows the fixture's state and closes without a press: the control passes at revision 4.
    let run = Run::new();
    let copy = fresh("fixture-cold");
    copy_dir(&out.join("session"), Path::new(&copy));
    assert_eq!(run.reopen(&copy, &menu(), &record_path), (M6H_OK, 4), "{}", last_error(run.host));
    run.fake().complete();
    // SAFETY: a live handle.
    assert_eq!(unsafe { m6h_pump(run.host) }, M6H_OK);
    run.frame();
    let shown = run.present();
    assert_eq!(shown.last().map(|row| row.engine_revision), Some(4));
    assert_eq!(run.close(4), (M6H_OK, 1), "{}", last_error(run.host));
    let control = record_of(run.host);
    assert_eq!(control.get("shown"), record.get("digest"));
    assert_eq!((int(&control, "committed"), int(&control, "shown_revision")), (Some(3), Some(4)));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn a_fixture_that_does_not_match_its_session_is_refused() {
    let (out, record) = write_fixture("tampered", &KEYS[..1]);
    let session = out.join("session");
    let refused = |name: &str, fixture: &Value, folder: Option<&Path>| -> String {
        let path = out.join(format!("{name}.json"));
        std::fs::write(&path, fixture.canonical()).expect("a record");
        let copy = fresh(name);
        if let Some(folder) = folder {
            copy_dir(folder, Path::new(&copy));
        }
        let run = Run::new();
        let (code, revision) = run.reopen(&copy, &menu(), &text(path));
        assert_eq!((code, revision), (M6H_E_SESSION, 0), "{name}");
        assert_eq!(run.fake().adoptions.len(), 1, "{name}: nothing was adopted");
        last_error(run.host)
    };
    let with = |key: &str, value: Value| {
        let mut fields = record.as_object().expect("an object").clone();
        fields.insert(key.to_owned(), value);
        Value::Object(fields)
    };
    let wrong = "0".repeat(64);
    assert!(refused("digest", &with("digest", Value::String(wrong)), Some(&session)).contains("is not the fixture's"));
    assert!(refused("revision", &with("revision", Value::Int(7)), Some(&session)).contains("does not count"));
    assert!(refused("format", &with("format", Value::String("v0".to_owned())), Some(&session)).contains("is not"));
    assert!(refused("missing", &record, None).contains("does not exist"));
    assert!(refused("presses", &with("committed", Value::Int(1)), Some(&session)).contains("not a list"));
    // An empty folder holds no journal to reopen.
    let empty = PathBuf::from(fresh("empty"));
    std::fs::create_dir_all(empty.join("x")).expect("a folder");
    assert!(refused("empty", &record, Some(&empty)).contains("journal"));
    let _ = std::fs::remove_dir_all(&out);
    let _ = std::fs::remove_dir_all(&empty);
}

#[test]
fn the_product_fixture_keys_all_commit() {
    let (out, record) = write_fixture("product-keys", &fixture::KEYS);
    assert_eq!(int(&record, "revision"), Some(1 + fixture::KEYS.len() as i64));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn a_runtime_that_does_not_start_is_refused() {
    let run = Run::new();
    let missing = text(checkout().join("work/no-such-runtime/c600-runtime.exe"));
    assert_eq!(run.load_runtime(&missing), M6H_E_RUNTIME);
    assert!(last_error(run.host).contains("did not start"), "{}", last_error(run.host));
    // SAFETY: a null program is refused before any use.
    assert_eq!(unsafe { m6h_load_runtime(run.host, ptr::null(), 0) }, M6H_E_INVALID_ARGUMENT);
}
