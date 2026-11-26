//! LP4-3's write-check-run flow, headless: the editor library and the host library through their C interfaces, as the
//! shell's panel drives them, over the bridge's C shim and the fake core. The solver types a macro with an error and
//! sees its diagnostic, fixes it, checks it, activates it explicitly, runs it as a program job from the palette and by
//! its key (in this process and in the runtime's own process), previews the word it returned with a command, commits
//! the preview by the key route's token and undoes the commit. Every step that reaches the session goes through the
//! command layer: the invocations through the shell, the run's end as the job's result, the preview, the commit and
//! the undo as rows the keys invoke. Every key event here is synthetic and every frame a fake core's; none of this is
//! Windows, input or GPU evidence.

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::ptr;

use c600_kernel::json::{self, Value};
use c600_render::FakeCore;
use c600_shell_editor::*;
use c600_shell_host::*;
use common::support::SIZE;
use common::{functions, Shim};

const CAMERA: [f32; 16] = [0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

/// The macro as the solver first types it: its command returns a truth value where it declares a word.
const BROKEN: &str = "edition \"6cs-draft-1\";\nmodule flow requires [];\n\n\
                      export command shown() -> Word effects [] {\n    return true;\n}\n";
/// The fix: the word it returns.
const FIX: &str = "word[+1, -2, +3]";
const WORD: [i32; 3] = [1, -2, 3];
const ROW: &str = "tool.flow.shown";
const KEY: &str = "Ctrl+Shift+KeyR";
const PREVIEW_KEY: &str = "Ctrl+Enter";
const COMMIT_KEY: &str = "Ctrl+Shift+Enter";
const UNDO_KEY: &str = "Ctrl+KeyZ";

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn path_text(path: PathBuf) -> String {
    path.to_str().expect("a UTF-8 path").to_owned()
}

/// A session folder that does not exist yet, under the build's own temporary folder (never %TEMP%).
fn fresh(name: &str) -> String {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("lp4-3-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path_text(path)
}

/// Reads a NUL-terminated text through one of the libraries' sized calls.
fn read(call: impl Fn(*mut u8, u32, *mut u32) -> i32) -> String {
    let mut size = 0u32;
    call(ptr::null_mut(), 0, &mut size);
    let mut buffer = vec![0u8; size as usize];
    assert_eq!(call(buffer.as_mut_ptr(), size, &mut size), 0);
    assert_eq!(buffer.pop(), Some(0));
    String::from_utf8(buffer).expect("UTF-8")
}

fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap_or_else(|e| panic!("{e:?}: {text}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_else(|| panic!("no `{key}` in {}", value.canonical()))
}

fn unhex(text: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

/// The receipt's status name.
fn status(receipt: &Value) -> &str {
    receipt.get("status").and_then(|s| s.get("name")).and_then(Value::as_str).expect("a status")
}

/// The host on the fake core with a session open.
struct Host {
    shim: *mut Shim,
    host: *mut ShellHost,
}

impl Host {
    fn open(name: &str) -> Host {
        let shim = Box::into_raw(Box::new(Shim { fake: FakeCore::on_surface(SIZE), pad: 0 }));
        let assets = path_text(checkout().join("assets"));
        let mut host = ptr::null_mut();
        // SAFETY: the table is the shim's, the context a live shim, the path's bytes are readable.
        let code = unsafe { m6h_create(&functions(), shim.cast(), assets.as_ptr(), assets.len(), &mut host) };
        assert_eq!(code, M6H_OK);
        let run = Host { shim, host };
        run.fake().complete();
        let (folder, menu) = (fresh(name), path_text(checkout().join("research/jumbling/fixtures/wj-I_a.json")));
        // SAFETY: a live handle, readable bytes, no fixture and no revision output.
        unsafe {
            assert_eq!(m6h_pump(host), M6H_OK);
            assert_eq!(m6h_set_viewport(host, SIZE.0, SIZE.1), M6H_OK);
            assert_eq!(m6h_set_camera(host, CAMERA.as_ptr()), M6H_OK);
            let code = m6h_open_session(
                host,
                M6H_PUZZLE_JUMBLE_OFF,
                folder.as_ptr(),
                folder.len(),
                menu.as_ptr(),
                menu.len(),
                ptr::null(),
                0,
                ptr::null_mut(),
            );
            assert_eq!(code, M6H_OK, "{}", run.error());
        }
        run
    }

    #[allow(clippy::mut_from_ref)]
    fn fake(&self) -> &mut FakeCore {
        // SAFETY: the host uses the shim only inside its calls, never at the same time as the test.
        unsafe { &mut (*self.shim).fake }
    }

    fn error(&self) -> String {
        // SAFETY: a live handle; the buffer holds `capacity` bytes.
        read(|buffer, capacity, size| unsafe { m6h_last_error(self.host, buffer, capacity, size) })
    }

    fn answer(&self) -> Value {
        // SAFETY: a live handle; the buffer holds `capacity` bytes.
        parse(&read(|buffer, capacity, size| unsafe { m6h_answer(self.host, buffer, capacity, size) }))
    }

    fn ok(&self, code: i32, what: &str) {
        assert_eq!(code, M6H_OK, "{what}: {}", self.error());
    }

    fn focus(&self, focus: u32) {
        // SAFETY: a live handle.
        self.ok(unsafe { m6h_set_focus(self.host, focus) }, "focus");
    }

    /// A key-down and its key-up; the press it was and the adopted revision after it.
    fn press(&self, chord: &str) -> M6hKey {
        let mut key = M6hKey::default();
        // SAFETY: a live handle, readable bytes and a writable row.
        self.ok(unsafe { m6h_key_down(self.host, chord.as_ptr(), chord.len(), 0, &mut key) }, chord);
        // SAFETY: a live handle and readable bytes.
        self.ok(unsafe { m6h_key_up(self.host, chord.as_ptr(), chord.len()) }, chord);
        key
    }

    /// Submits a frame, confirms every submitted frame and reads the confirmations, which the host reports to the
    /// command layer.
    fn present(&self) -> Vec<M6hPresented> {
        let mut submitted = M6hFrame::default();
        // SAFETY: a live handle and a writable row.
        self.ok(unsafe { m6h_frame(self.host, &mut submitted) }, "frame");
        self.fake().complete();
        let last = self.fake().submitted.len() as u64;
        assert!(self.fake().show(last));
        let mut rows = vec![M6hPresented::default(); 16];
        let mut count = 0u32;
        // SAFETY: `rows` holds 16 rows.
        self.ok(unsafe { m6h_read_presented(self.host, rows.as_mut_ptr(), 16, &mut count) }, "presented");
        rows.truncate(count as usize);
        rows
    }

    fn register(&self, module: &str, entry: &str, label: &str, chord: &str) -> i32 {
        // SAFETY: a live handle and readable bytes.
        unsafe {
            m6h_register_row(
                self.host,
                module.as_ptr(),
                module.len(),
                entry.as_ptr(),
                entry.len(),
                label.as_ptr(),
                label.len(),
                chord.as_ptr(),
                chord.len(),
            )
        }
    }

    fn palette(&self) -> Value {
        // SAFETY: a live handle.
        self.ok(unsafe { m6h_palette(self.host) }, "palette");
        self.answer()
    }

    /// Invokes `row` from the palette; its receipt and its program job's ID (0 for none).
    fn invoke(&self, row: &str) -> (Value, u64) {
        let mut job = u64::MAX;
        // SAFETY: a live handle, readable bytes, no arguments and a writable ID.
        self.ok(unsafe { m6h_invoke(self.host, row.as_ptr(), row.len(), ptr::null(), 0, &mut job) }, row);
        (self.answer(), job)
    }

    /// Takes the next program job: its ID and its JSON.
    fn take_job(&self) -> Option<(u64, String)> {
        let mut job = u64::MAX;
        // SAFETY: a live handle and a writable ID.
        self.ok(unsafe { m6h_take_job(self.host, &mut job) }, "take job");
        // SAFETY: a live handle; the buffer holds `capacity` bytes.
        let text = read(|buffer, capacity, size| unsafe { m6h_answer(self.host, buffer, capacity, size) });
        (job != 0).then_some((job, text))
    }

    /// Reports the job with the run's record: its output as the value, or its code and detail as the error. The
    /// receipt of the command that invoked the job.
    fn finish(&self, job: u64, record: &Value) -> Value {
        let (value, code, detail) = match field(record, "status") {
            "completed" => (field(record, "output"), "", ""),
            _ => ("", field(record, "code"), field(record, "detail")),
        };
        // SAFETY: a live handle and readable bytes.
        let code = unsafe {
            m6h_finish_job(
                self.host,
                job,
                value.as_ptr(),
                value.len(),
                code.as_ptr(),
                code.len(),
                detail.as_ptr(),
                detail.len(),
            )
        };
        self.ok(code, "finish job");
        self.answer()
    }

    fn set_draft(&self, word: &[i32]) {
        // SAFETY: a live handle and `word.len()` readable letters.
        self.ok(unsafe { m6h_set_draft_word(self.host, word.as_ptr(), word.len()) }, "draft");
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // SAFETY: the handle is not used again; the shim outlives it.
        unsafe {
            m6h_destroy(self.host);
            drop(Box::from_raw(self.shim));
        }
    }
}

/// The editor library's handle.
struct Editor(*mut ShellEditor);

impl Editor {
    fn new() -> Editor {
        let mut handle = ptr::null_mut();
        // SAFETY: a writable handle.
        assert_eq!(unsafe { m6e_create(M6E_UNITS_UTF8, &mut handle) }, M6E_OK);
        assert!(!handle.is_null());
        Editor(handle)
    }

    fn error(&self) -> String {
        // SAFETY: a live handle; the buffer holds `capacity` bytes.
        read(|buffer, capacity, size| unsafe { m6e_last_error(self.0, buffer, capacity, size) })
    }

    fn answer(&self) -> Value {
        // SAFETY: a live handle; the buffer holds `capacity` bytes.
        parse(&read(|buffer, capacity, size| unsafe { m6e_answer(self.0, buffer, capacity, size) }))
    }

    fn ok(&self, code: i32, what: &str) -> Value {
        assert_eq!(code, M6E_OK, "{what}: {}", self.error());
        self.answer()
    }

    fn set_text(&self, text: &str) -> Value {
        // SAFETY: a live handle and readable bytes.
        self.ok(unsafe { m6e_set_text(self.0, text.as_ptr(), text.len()) }, "set text")
    }

    fn edit(&self, start: usize, end: usize, text: &str) -> Value {
        // SAFETY: a live handle and readable bytes.
        self.ok(unsafe { m6e_edit(self.0, start as u64, end as u64, text.as_ptr(), text.len()) }, "edit")
    }

    fn diagnose(&self) -> Value {
        // SAFETY: a live handle.
        self.ok(unsafe { m6e_diagnose(self.0) }, "diagnose")
    }

    fn complete(&self, position: usize) -> Value {
        // SAFETY: a live handle.
        self.ok(unsafe { m6e_complete(self.0, position as u64) }, "complete")
    }

    fn check(&self, entry: &str) -> (i32, String) {
        // SAFETY: a live handle and readable bytes.
        let code = unsafe { m6e_check(self.0, entry.as_ptr(), entry.len()) };
        (code, if code == M6E_OK { self.answer().canonical() } else { self.error() })
    }

    fn activate(&self, module: &str, expected: Option<[u8; 32]>, candidate: [u8; 32], reference: [u8; 32]) -> i32 {
        let expected = expected.as_ref().map_or(ptr::null(), |d| d.as_ptr());
        // SAFETY: a live handle, readable bytes and 32-byte digests.
        unsafe { m6e_activate(self.0, module.as_ptr(), module.len(), expected, candidate.as_ptr(), reference.as_ptr()) }
    }

    /// Runs a job and waits for its record.
    fn run(&self, job: &str) -> Value {
        let mut ticket = 0u64;
        // SAFETY: a live handle, readable bytes and a writable ticket.
        self.ok(unsafe { m6e_run(self.0, job.as_ptr(), job.len(), &mut ticket) }, "run");
        assert_ne!(ticket, 0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            let mut state = u32::MAX;
            // SAFETY: a live handle and a writable state.
            let record = self.ok(unsafe { m6e_run_poll(self.0, ticket, &mut state) }, "poll");
            match state {
                M6E_RUN_PENDING => {
                    assert!(std::time::Instant::now() < deadline, "the run did not end");
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                M6E_RUN_COMPLETED => {
                    assert_eq!(field(&record, "status"), "completed");
                    return record;
                }
                M6E_RUN_REFUSED => {
                    assert_eq!(field(&record, "status"), "refused");
                    return record;
                }
                other => panic!("an unknown run state {other}"),
            }
        }
    }

    fn load_runtime(&self, program: &str) -> i32 {
        // SAFETY: a live handle and readable bytes.
        unsafe { m6e_load_runtime(self.0, program.as_ptr(), program.len()) }
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        // SAFETY: the handle is not used again.
        unsafe { m6e_destroy(self.0) }
    }
}

fn word(record: &Value) -> Vec<i32> {
    record
        .get("word")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("no word in {}", record.canonical()))
        .iter()
        .map(|l| l.as_i64().unwrap() as i32)
        .collect()
}

#[test]
fn the_abi_versions() {
    assert_eq!(m6e_abi_version(), 1);
    assert_eq!(m6h_abi_version(), 10);
}

#[test]
fn a_macro_is_written_checked_activated_run_previewed_committed_and_undone_through_the_command_layer() {
    let host = Host::open("flow");
    let editor = Editor::new();

    // Write: the macro with an error, and its diagnostic.
    let digest = editor.set_text(BROKEN);
    let diagnosis = editor.diagnose();
    assert_eq!(field(&diagnosis, "digest"), field(&digest, "digest"));
    assert_eq!(field(&diagnosis, "verdict"), "refused", "{}", diagnosis.canonical());
    let first = &diagnosis.get("diagnostics").and_then(Value::as_array).expect("diagnostics")[0];
    assert_eq!(field(first, "code"), "LG2:type.mismatch", "{}", diagnosis.canonical());
    let at = BROKEN.find("true").unwrap();
    let place = first.get("at").expect("a range");
    assert_eq!(place.get("line").and_then(Value::as_i64), Some(5), "{}", diagnosis.canonical());
    assert!(place.get("start").and_then(Value::as_i64).is_some_and(|s| s as usize <= at), "{}", diagnosis.canonical());
    // A draft that does not check is refused before the module set sees it.
    let (code, error) = editor.check("shown");
    assert_eq!(code, M6E_E_REFUSED, "{error}");
    assert_eq!(field(&parse(&error), "code"), "LG2:type.mismatch");

    // Fix: replace `true` with the word; completion offers `word` where the solver types it.
    editor.edit(at, at + "true".len(), "wor");
    let completion = editor.complete(at + 3);
    let labels: Vec<&str> =
        completion.get("candidates").and_then(Value::as_array).unwrap().iter().map(|c| field(c, "label")).collect();
    assert!(labels.contains(&"word"), "{labels:?}");
    assert_eq!(field(&completion, "prefix"), "wor");
    editor.edit(at, at + 3, FIX);
    let diagnosis = editor.diagnose();
    assert_eq!(field(&diagnosis, "verdict"), "checked", "{}", diagnosis.canonical());
    assert_eq!(field(&diagnosis, "module"), "flow");
    assert_eq!(diagnosis.get("exports"), Some(&Value::Array(vec![Value::String("shown".into())])));

    // Check: a candidate, not active.
    let (code, checked) = editor.check("shown");
    assert_eq!(code, M6E_OK, "{checked}");
    let checked = parse(&checked);
    assert_eq!(field(&checked, "module"), "flow");
    assert_eq!(checked.get("active"), Some(&Value::Null));
    let (candidate, reference) = (unhex(field(&checked, "candidate")), unhex(field(&checked, "reference")));

    // The module's row, with its key, in the palette.
    assert_eq!(host.register("flow", "shown", "Show the flow word", KEY), M6H_OK, "{}", host.error());
    assert_eq!(field(&host.answer(), "row"), ROW);
    assert_eq!(host.register("flow", "shown", "Again", ""), M6H_E_HOST, "a row registers once");
    assert_eq!(host.register("flow", "other", "Taken", KEY), M6H_E_HOST, "a key binds one row");
    let palette = host.palette();
    let entry = palette
        .get("entries")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .find(|e| field(e, "row") == ROW)
        .unwrap_or_else(|| panic!("{ROW} is not in the palette: {}", palette.canonical()))
        .clone();
    assert_eq!(field(&entry, "label"), "Show the flow word");
    assert_eq!(entry.get("language_defined"), Some(&Value::Bool(true)));
    assert_eq!(entry.get("keys"), Some(&Value::Array(vec![Value::String(KEY.into())])), "{}", entry.canonical());

    // Checked is not active: the job the palette starts is refused when the editor binds it, and fails.
    let (receipt, id) = host.invoke(ROW);
    assert_eq!(status(&receipt), "running", "{}", receipt.canonical());
    let (job, text) = host.take_job().expect("a program job");
    assert_eq!(job, id);
    let refused = editor.run(&text);
    assert_eq!(field(&refused, "code"), "EV1:input.module_not_checked", "{}", refused.canonical());
    let receipt = host.finish(job, &refused);
    assert_eq!(status(&receipt), "failed", "{}", receipt.canonical());
    assert!(host.take_job().is_none());

    // Activate, explicitly; a stale expectation is refused.
    assert_eq!(editor.activate("flow", Some([7; 32]), candidate, reference), M6E_E_REFUSED);
    assert_eq!(field(&parse(&editor.error()), "code"), "EV1:stale.activation_base");
    assert_eq!(editor.activate("flow", None, candidate, reference), M6E_OK, "{}", editor.error());
    let activation = editor.answer();
    assert_eq!(activation.get("previous"), Some(&Value::Null));
    assert_eq!(field(&activation, "new"), field(&checked, "candidate"));

    // Run from the palette, in this process: the job completes with the word.
    let (receipt, id) = host.invoke(ROW);
    assert_eq!(status(&receipt), "running");
    let (job, text) = host.take_job().expect("a program job");
    assert_eq!(job, id);
    let job_json = parse(&text);
    assert_eq!(
        (field(&job_json, "module"), field(&job_json, "entry"), field(&job_json, "row")),
        ("flow", "shown", ROW)
    );
    let ran = editor.run(&text);
    assert_eq!(field(&ran, "host"), "in-process");
    assert_eq!(field(&ran, "closure"), field(&checked, "candidate"));
    assert_eq!(field(&ran, "output"), "[1,-2,3]\n");
    assert_eq!(word(&ran), WORD);
    let receipt = host.finish(job, &ran);
    assert_eq!(status(&receipt), "completed", "{}", receipt.canonical());

    // Run by its key, in the runtime's own process: the same word. While the editor's text field has the focus, a
    // twist key types and reaches no row.
    assert_eq!(editor.load_runtime(env!("CARGO_BIN_EXE_c600-shell-editor-runtime")), M6E_OK, "{}", editor.error());
    host.focus(M6H_FOCUS_TEXT);
    let typed = host.press("KeyJ");
    assert_eq!(typed.press, M6H_PRESS_NONE);
    assert!(host.take_job().is_none());
    host.focus(M6H_FOCUS_PUZZLE);
    let before = host.press(KEY);
    assert_eq!(before.press, M6H_PRESS_NONE, "a query row changes no state");
    let (job, text) = host.take_job().expect("the key's program job");
    let ran = editor.run(&text);
    assert_eq!(field(&ran, "host"), "process", "{}", ran.canonical());
    assert_eq!(word(&ran), WORD);
    assert_eq!(status(&host.finish(job, &ran)), "completed");

    // Preview: the solver sets the returned word as the draft and previews it by operation.preview's key; the frames
    // that show the preview are confirmed and reported to the layer.
    host.set_draft(&word(&ran));
    let previewed = host.press(PREVIEW_KEY);
    assert_eq!(previewed.press, M6H_PRESS_NONE, "a preview changes no state");
    assert_eq!(previewed.engine_revision, before.engine_revision);
    let unseen = host.press(COMMIT_KEY);
    assert_eq!(unseen.press, M6H_PRESS_NONE, "no commit before a frame showed the preview");
    assert!(!host.present().is_empty(), "a frame showed the preview");

    // Commit by the key route's token, then undo: both adopt a new head.
    let committed = host.press(COMMIT_KEY);
    assert_eq!(committed.press, M6H_PRESS_OTHER, "the commit changed the head");
    assert_eq!(committed.engine_revision, before.engine_revision + 1);
    host.present();
    let undone = host.press(UNDO_KEY);
    assert_eq!(undone.press, M6H_PRESS_OTHER, "the undo changed the head");
    assert_eq!(undone.engine_revision, committed.engine_revision + 1);
    host.present();

    // A second commit by key finds no presented preview: nothing commits without one.
    let again = host.press(COMMIT_KEY);
    assert_eq!(again.press, M6H_PRESS_NONE);
    assert_eq!(again.engine_revision, undone.engine_revision);

    // The close's control: two changes that were no twist press (the commit and the undo), a journal that reopens to
    // the live head, and a live head that is the start again, as the replay of no twist press gives it. (The control
    // passes only for twist presses, so it does not pass here.)
    let mut pass = u32::MAX;
    // SAFETY: a live handle and a writable flag.
    host.ok(unsafe { m6h_close_session(host.host, 0, &mut pass) }, "close");
    // SAFETY: a live handle; the buffer holds `capacity` bytes.
    let control = parse(&read(|buffer, capacity, size| unsafe { m6h_check(host.host, buffer, capacity, size) }));
    assert_eq!(control.get("other_changes").and_then(Value::as_i64), Some(2), "{}", control.canonical());
    assert_eq!(control.get("committed").and_then(Value::as_i64), Some(0));
    let live = control.get("live").and_then(Value::as_str).expect("a live head");
    assert_eq!(control.get("reopened").and_then(Value::as_str), Some(live), "{}", control.canonical());
    assert_eq!(control.get("replayed").and_then(Value::as_str), Some(live), "{}", control.canonical());
}

#[test]
fn a_refused_check_and_a_wrong_activation_leave_the_module_set() {
    let editor = Editor::new();
    let fixed = BROKEN.replace("true", FIX);
    editor.set_text(&fixed);
    // An entry the module does not export is refused by the module set.
    let (code, error) = editor.check("missing");
    assert_eq!(code, M6E_E_REFUSED, "{error}");
    // A candidate no check created is refused.
    assert_eq!(editor.activate("flow", None, [1; 32], [2; 32]), M6E_E_REFUSED);
    assert_eq!(field(&parse(&editor.error()), "code"), "EV1:input.module_not_checked");
    // A range off the buffer is refused and changes nothing.
    let before = editor.set_text(&fixed);
    // SAFETY: a live handle and readable bytes.
    let code = unsafe { m6e_edit(editor.0, 0, fixed.len() as u64 + 1, "x".as_ptr(), 1) };
    assert_eq!(code, M6E_E_REFUSED);
    assert_eq!(field(&parse(&editor.error()), "code"), "EV1:input.source_range_invalid");
    let after = editor.diagnose();
    assert_eq!(field(&after, "digest"), field(&before, "digest"));
    // A job that names no active module's entry is a refused run at once.
    let job = r#"{"job":9,"row":"tool.none.go","module":"none","entry":"go","run":"job-9","base":"b","inputs":[],
                 "limits":{"steps":"1","kernel_cost":"1","memory":"1","output":"1"}}"#;
    let record = editor.run(job);
    assert_eq!(field(&record, "code"), "EV1:input.module_not_checked");
    assert_eq!(record.get("job").and_then(Value::as_i64), Some(9));
    // Null arguments.
    // SAFETY: null pointers are refused before any use.
    unsafe {
        assert_eq!(m6e_create(M6E_UNITS_UTF8, ptr::null_mut()), M6E_E_INVALID_ARGUMENT);
        assert_eq!(m6e_diagnose(ptr::null_mut()), M6E_E_INVALID_ARGUMENT);
        assert_eq!(m6e_run(editor.0, ptr::null(), 0, ptr::null_mut()), M6E_E_INVALID_ARGUMENT);
        assert_eq!(
            m6e_activate(editor.0, ptr::null(), 0, ptr::null(), ptr::null(), ptr::null()),
            M6E_E_INVALID_ARGUMENT
        );
    }
}
