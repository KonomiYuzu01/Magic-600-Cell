//! LX-4's editor module through its C interface, on fixtures that c600-lx4 generates into a fresh folder: every edit
//! of the script replayed as lx4.py posts it (Reset, then per replacement in descending offset order a Select and the
//! insert's WM_CHAR units or VK_BACK, with the WM_CHAR 0x08 Godot's TranslateMessage makes of VK_BACK), the
//! endpoints each sample queues and the frames they name, the re-run's protocol with the shell, and the refusals.
//! Synthetic frame ids, and the shell's view job stood in for by the test, except where the reference view runs in
//! this process; nothing here is timing, Windows/DirectX or input evidence.

use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use c600_digest::{hex, sha256};
use c600_run::interp::NoHost;
use c600_run::meter::Amounts;
use c600_runtime::{run_in_process, Ended, Job, JobOptions};
use c600_shell_lx4::analysis::prefix;
use c600_shell_lx4::fixtures::{self, Edit, Loaded};
use c600_shell_lx4::*;

/// The fixture folder, generated and validated once per test run.
fn folder() -> &'static Path {
    static FOLDER: OnceLock<PathBuf> = OnceLock::new();
    FOLDER.get_or_init(|| {
        let set = c600_lx4::generate().expect("the module set generates");
        let script = c600_lx4::edits(&set).expect("the script generates");
        let validation = c600_lx4::validate(&set, &script).expect("the front end agrees with the script");
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("lx4-fixtures-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(out.join("W")).unwrap();
        for module in set.all() {
            fs::write(out.join(module.path()), &module.source).unwrap();
        }
        fs::write(out.join("MANIFEST.json"), c600_lx4::manifest_json(&set, &validation).unwrap()).unwrap();
        fs::write(out.join("EDITS.json"), c600_lx4::edits_json(&set, &script)).unwrap();
        out
    })
}

struct Handle(*mut Lx4Module);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: a live handle, used by nothing after this.
        unsafe { lx4_destroy(self.0) };
    }
}

fn create(path: &Path) -> (i32, *mut Lx4Module) {
    let path = path.to_str().expect("a UTF-8 path");
    let mut out = ptr::null_mut();
    // SAFETY: the path's bytes are readable and `out` is writable.
    let code = unsafe { lx4_create(path.as_ptr(), path.len(), &mut out) };
    (code, out)
}

fn open() -> Handle {
    let (code, handle) = create(folder());
    assert_eq!(code, LX4_OK, "{}", last_error(ptr::null_mut()));
    Handle(handle)
}

fn last_error(handle: *mut Lx4Module) -> String {
    let mut size = 0u32;
    // SAFETY: a null buffer only asks for the size.
    assert_eq!(unsafe { lx4_last_error(handle, ptr::null_mut(), 0, &mut size) }, LX4_E_INVALID_ARGUMENT);
    let mut buffer = vec![0u8; size as usize];
    // SAFETY: the buffer holds `size` bytes.
    assert_eq!(unsafe { lx4_last_error(handle, buffer.as_mut_ptr(), size, &mut size) }, LX4_OK);
    buffer.pop();
    String::from_utf8(buffer).unwrap()
}

fn text(handle: &Handle, which: u32) -> String {
    let mut size = 0u32;
    // SAFETY: a null buffer only asks for the size.
    assert_eq!(unsafe { lx4_text(handle.0, which, ptr::null_mut(), 0, &mut size) }, LX4_E_INVALID_ARGUMENT);
    let mut buffer = vec![0u8; size as usize];
    // SAFETY: the buffer holds `size` bytes.
    assert_eq!(unsafe { lx4_text(handle.0, which, buffer.as_mut_ptr(), size, &mut size) }, LX4_OK);
    assert_eq!(buffer.pop(), Some(0));
    String::from_utf8(buffer).unwrap()
}

fn update(handle: &Handle, frame: u64) -> Lx4Update {
    let mut out = Lx4Update::default();
    // SAFETY: a live handle; `out` is writable.
    assert_eq!(unsafe { lx4_update(handle.0, frame, &mut out) }, LX4_OK);
    out
}

fn drain(handle: &Handle) -> Vec<Lx4Shown> {
    let mut rows = Vec::new();
    loop {
        let mut shown = Lx4Shown { sample: 0, frame_id: 0, endpoint: 0, reserved: 0, buffer_sha256: [0; 64] };
        let mut present = 0u32;
        // SAFETY: a live handle; both outputs are writable.
        assert_eq!(unsafe { lx4_next_shown(handle.0, &mut shown, &mut present) }, LX4_OK);
        if present == 0 {
            return rows;
        }
        rows.push(shown);
    }
}

/// One of lx4.py's messages.
#[derive(Clone, Copy, Debug)]
enum Input {
    Reset(u32, u64),
    Select(u64, u64),
    Char(u16),
    Back,
}

/// lx4.py's `messages()` for an edit, turns left out.
fn messages(edit: &Edit, index: u32, sample: u64) -> Vec<Input> {
    let mut out = vec![Input::Reset(index, sample)];
    let mut order = edit.replacements.clone();
    order.sort_by_key(|r| Reverse(r.offset));
    for r in order {
        out.push(Input::Select(r.offset as u64, (r.offset + r.delete.len()) as u64));
        if r.insert.is_empty() {
            out.push(Input::Back);
        } else {
            out.extend(r.insert.replace('\n', "\r").encode_utf16().map(Input::Char));
        }
    }
    out
}

/// Posts one message as the shell forwards it; VK_BACK also brings the WM_CHAR 0x08 that Godot's loop makes of it.
fn post(handle: &Handle, input: Input) -> i32 {
    let mut taken = 0u32;
    // SAFETY: a live handle; `taken` is writable.
    unsafe {
        match input {
            Input::Reset(edit, sample) => lx4_reset(handle.0, edit, sample),
            Input::Select(start, end) => lx4_select(handle.0, start, end),
            Input::Char(unit) => {
                let code = lx4_char(handle.0, unit, 0, &mut taken);
                assert_eq!(taken, 1, "the module takes the harness's characters");
                code
            }
            Input::Back => {
                let code = lx4_backspace(handle.0);
                assert_eq!(lx4_char(handle.0, 0x08, 0x0E, &mut taken), LX4_OK);
                assert_eq!(taken, 1, "the module takes VK_BACK's translated character and ignores it");
                code
            }
        }
    }
}

/// The shell's report of a re-run's end.
fn rerun_ended(handle: &Handle, frame: u64, digest: Option<&[u8; 32]>, reason: &str) -> i32 {
    let digest = digest.map_or(ptr::null(), |d| d.as_ptr());
    // SAFETY: a live handle; the digest is null or 32 bytes, the reason `reason.len()` bytes.
    unsafe { lx4_rerun_ended(handle.0, frame, digest, reason.as_ptr(), reason.len()) }
}

/// The index of the script's one `kind` edit at `target`.
fn edit_index(loaded: &Loaded, target: &str, kind: &str) -> usize {
    let found: Vec<usize> =
        (0..loaded.edits.len()).filter(|&i| loaded.edits[i].target == target && loaded.edits[i].kind == kind).collect();
    assert_eq!(found.len(), 1, "{target} {kind}");
    found[0]
}

/// Runs frames from `frame` until the status has nothing pending and every required endpoint is queued. These frames
/// come after the update that may ask for a re-run, so none asks.
fn frames_until(handle: &Handle, mut frame: u64, required: &[&str], mut rows: Vec<Lx4Shown>) -> (Vec<Lx4Shown>, u64) {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        assert_eq!(update(handle, frame).rerun, 0, "a re-run is asked for once, by a re-run sample");
        rows.extend(drain(handle));
        let have: Vec<&str> = rows.iter().map(|r| ENDPOINTS[r.endpoint as usize]).collect();
        if !text(handle, LX4_TEXT_STATUS).contains("pending") && required.iter().all(|e| have.contains(e)) {
            return (rows, frame);
        }
        assert!(
            Instant::now() < deadline,
            "endpoints {have:?} of {required:?}; status {}",
            text(handle, LX4_TEXT_STATUS)
        );
        frame += 1;
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// The endpoints each edit kind must show, and those it must not.
fn expectations(edit: &Edit) -> (Vec<&'static str>, Vec<&'static str>) {
    let mut required = vec!["echo", "stale", "syntax"];
    let mut forbidden = vec!["rerun"];
    match edit.kind.as_str() {
        "inside-body" | "exported-signature" => required.push("diagnostics"),
        "completion-trigger" => {
            required.push("completion");
            forbidden.extend(["diagnostics", "dependents"]);
        }
        "syntax-break" => forbidden.extend(["completion", "diagnostics", "dependents"]),
        other => panic!("an edit kind the test does not know: {other}"),
    }
    if edit.kind == "inside-body" {
        forbidden.push("completion");
    }
    if edit.target == "W00" && edit.kind != "completion-trigger" && edit.kind != "syntax-break" {
        required.push("dependents");
    } else {
        forbidden.push("dependents");
    }
    (required, forbidden)
}

#[test]
fn the_abi_version_is_two() {
    assert_eq!(lx4_abi_version(), LX4_ABI_VERSION);
    assert_eq!(LX4_ABI_VERSION, 2);
}

#[test]
fn every_edit_of_the_script_queues_its_endpoints_for_the_result() {
    let loaded = fixtures::load(folder()).unwrap();
    assert_eq!(loaded.edits.len(), 16);
    let handle = open();
    let mut frame = 1_000;
    for (index, edit) in loaded.edits.iter().enumerate() {
        let sample = 7_000 + index as u64;
        let inputs = messages(edit, index as u32, sample);
        // The handshake: the frame after the Reset draws the baseline and queues `ready`, once.
        assert_eq!(post(&handle, inputs[0]), LX4_OK, "{}", last_error(handle.0));
        update(&handle, frame);
        let ready = drain(&handle);
        assert_eq!(ready.len(), 1, "{ready:?}");
        assert_eq!(
            (ENDPOINTS[ready[0].endpoint as usize], ready[0].sample, ready[0].frame_id),
            ("ready", sample, frame)
        );
        assert_eq!(std::str::from_utf8(&ready[0].buffer_sha256).unwrap(), edit.base_sha256, "ready shows the baseline");
        frame += 1;
        update(&handle, frame);
        assert!(drain(&handle).is_empty(), "ready is queued once");
        frame += 1;
        // The first typed key is the message after the first Select; the frame after it shows the key.
        let first_key = inputs.iter().position(|i| matches!(i, Input::Char(_) | Input::Back)).unwrap();
        for &input in &inputs[1..=first_key] {
            assert_eq!(post(&handle, input), LX4_OK, "{}", last_error(handle.0));
        }
        let echo_frame = frame;
        update(&handle, echo_frame);
        let early = drain(&handle);
        let rest = &inputs[first_key + 1..];
        // A syntax break's one VK_BACK is both its first and its last key.
        let last_input_frame = if rest.is_empty() { echo_frame } else { echo_frame + 1 };
        if !rest.is_empty() {
            assert!(early.is_empty(), "no endpoint before the sample's last input");
        }
        frame += 1;
        for &input in rest {
            assert_eq!(post(&handle, input), LX4_OK, "{}", last_error(handle.0));
        }
        let (required, forbidden) = expectations(edit);
        let (rows, end) = frames_until(&handle, frame, &required, early);
        let what = format!("{} {}", edit.target, edit.kind);
        let names: Vec<&str> = rows.iter().map(|r| ENDPOINTS[r.endpoint as usize]).collect();
        for name in &forbidden {
            assert!(!names.contains(name), "{what}: {name} is queued: {names:?}");
        }
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "{what}: an endpoint is queued twice: {names:?}");
        for row in &rows {
            assert_eq!(row.sample, sample, "{what}");
            assert_eq!(row.reserved, 0);
            assert_eq!(std::str::from_utf8(&row.buffer_sha256).unwrap(), edit.result_sha256, "{what}: the buffer");
            if ENDPOINTS[row.endpoint as usize] == "echo" {
                assert_eq!(row.frame_id, echo_frame, "{what}: echo names the frame after the first key");
            } else {
                assert!((last_input_frame..=end).contains(&row.frame_id), "{what}: {row:?}");
            }
        }
        let status = text(&handle, LX4_TEXT_STATUS);
        assert!(status.contains(&format!("sample {sample}: {n} of {n} inputs", n = edit.inputs())), "{status}");
        assert!(status.contains("view: STALE"), "{what}: {status}");
        match edit.kind.as_str() {
            "inside-body" | "exported-signature" => {
                assert!(status.contains("syntax: clean") && status.contains("diagnostics: clean"), "{what}: {status}")
            }
            _ => assert!(status.contains("diagnostics: not run (syntax)"), "{what}: {status}"),
        }
        if edit.kind == "completion-trigger" {
            let list = text(&handle, LX4_TEXT_COMPLETION);
            assert!(list.lines().any(|n| n == "exact_add"), "{what}: {list}");
            assert!(list.lines().all(|n| n.starts_with("exact_")), "{what}: {list}");
        }
        if edit.target == "W00" && edit.kind == "exported-signature" {
            assert!(status.contains("dependents:") && status.contains("refused in"), "{what}: {status}");
        }
        if edit.target == "W00" && edit.kind == "inside-body" {
            assert!(status.contains("dependents: clean (50 modules checked)"), "{what}: {status}");
        }
        let echo = text(&handle, LX4_TEXT_ECHO);
        assert!(echo.starts_with(&format!("{} {}  sample {sample}  ", edit.target, edit.kind)), "{echo}");
        let last = edit.replacements.iter().min_by_key(|r| r.offset).unwrap();
        if let Some(line) = last.insert.lines().find(|l| !l.trim().is_empty()) {
            assert!(echo.contains(line.trim()), "{what}: the echo shows the typed text: {echo}");
        }
        frame = end + 1;
    }
}

/// Posts a re-run sample of `index` (flagged Reset, its ready row, every input) and runs the update after its last
/// input, which asks for the re-run. Answers that update's frame.
fn rerun_sample(handle: &Handle, edit: &Edit, index: usize, sample: u64, frame: u64) -> u64 {
    let inputs = messages(edit, index as u32 | LX4_RESET_RERUN, sample);
    assert_eq!(post(handle, inputs[0]), LX4_OK, "{}", last_error(handle.0));
    let ready = update(handle, frame);
    assert_eq!(ready.rerun, 0, "the baseline is no re-run");
    let rows = drain(handle);
    assert_eq!(
        rows.iter().map(|r| (ENDPOINTS[r.endpoint as usize], r.sample)).collect::<Vec<_>>(),
        [("ready", sample)]
    );
    assert!(text(handle, LX4_TEXT_STATUS).contains("rerun: waiting for the last input"));
    for &input in &inputs[1..] {
        assert_eq!(post(handle, input), LX4_OK, "{}", last_error(handle.0));
    }
    assert_eq!(update(handle, frame + 1).rerun, 1, "the update after the last input asks for the re-run");
    frame + 1
}

#[test]
fn a_rerun_sample_asks_once_and_queues_rerun_for_the_frame_that_drew_the_run() {
    let loaded = fixtures::load(folder()).unwrap();
    let handle = open();
    let index = edit_index(&loaded, "S", "inside-body");
    let edit = &loaded.edits[index];
    let asked = rerun_sample(&handle, edit, index, 31, 1);
    // The shell runs the buffer: the edit's result.
    let buffer = text(&handle, LX4_TEXT_BUFFER);
    assert_eq!(hex(&sha256(buffer.as_bytes())), edit.result_sha256);
    let (rows, end) = frames_until(&handle, asked + 1, &["echo", "stale", "syntax", "diagnostics"], drain(&handle));
    assert!(rows.iter().all(|r| ENDPOINTS[r.endpoint as usize] != "rerun"), "no rerun row before the run ends");
    assert!(text(&handle, LX4_TEXT_STATUS).contains("rerun: running the reference view"));
    // The run ended: published, first drawn by a later confirmed frame.
    let drawn = end + 3;
    assert_eq!(rerun_ended(&handle, drawn, Some(&sha256(buffer.as_bytes())), ""), LX4_OK);
    let rows = drain(&handle);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!((ENDPOINTS[rows[0].endpoint as usize], rows[0].sample, rows[0].frame_id), ("rerun", 31, drawn));
    assert_eq!(std::str::from_utf8(&rows[0].buffer_sha256).unwrap(), edit.result_sha256);
    assert_eq!(update(&handle, end + 4).rerun, 0);
    assert!(drain(&handle).is_empty(), "rerun is queued once");
    assert!(text(&handle, LX4_TEXT_STATUS).contains(&format!("rerun: shown in frame {drawn}")));
    assert_eq!(rerun_ended(&handle, drawn, Some(&sha256(buffer.as_bytes())), ""), LX4_E_STATE);
    assert!(last_error(handle.0).contains("no re-run is running"), "{}", last_error(handle.0));
}

#[test]
fn ready_waits_for_a_running_rerun_and_a_refused_or_other_run_queues_nothing() {
    let loaded = fixtures::load(folder()).unwrap();
    let handle = open();
    let index = edit_index(&loaded, "M", "inside-body");
    let edit = &loaded.edits[index];
    let echo = edit_index(&loaded, "S", "inside-body");

    // A refused run: no row. The next sample's ready waits until the shell reports the end.
    let asked = rerun_sample(&handle, edit, index, 41, 10);
    drain(&handle);
    assert_eq!(post(&handle, Input::Reset(echo as u32, 42)), LX4_OK);
    update(&handle, asked + 1);
    assert!(drain(&handle).is_empty(), "ready waits for the running re-run");
    assert_eq!(rerun_ended(&handle, 0, None, "{\"code\":\"m6h:superseded\"}"), LX4_OK);
    update(&handle, asked + 2);
    let rows = drain(&handle);
    assert_eq!(rows.iter().map(|r| (ENDPOINTS[r.endpoint as usize], r.sample)).collect::<Vec<_>>(), [("ready", 42)]);

    // A published run of another text: no row, and the status says why.
    let asked = rerun_sample(&handle, edit, index, 43, 20);
    drain(&handle);
    assert_eq!(rerun_ended(&handle, asked + 2, Some(&sha256(b"another text")), ""), LX4_OK);
    update(&handle, asked + 3);
    assert!(drain(&handle).iter().all(|r| ENDPOINTS[r.endpoint as usize] != "rerun"));
    assert!(text(&handle, LX4_TEXT_STATUS).contains("rerun: REFUSED: the run's text"));

    // A run that ends after the next Reset still names its own sample and buffer.
    let asked = rerun_sample(&handle, edit, index, 44, 30);
    let digest = sha256(text(&handle, LX4_TEXT_BUFFER).as_bytes());
    drain(&handle);
    assert_eq!(post(&handle, Input::Reset(echo as u32, 45)), LX4_OK);
    update(&handle, asked + 1);
    assert!(drain(&handle).is_empty());
    assert_eq!(rerun_ended(&handle, asked + 1, Some(&digest), ""), LX4_OK);
    update(&handle, asked + 2);
    let rows = drain(&handle);
    let names: Vec<_> = rows.iter().map(|r| (ENDPOINTS[r.endpoint as usize], r.sample, r.frame_id)).collect();
    assert_eq!(names, [("rerun", 44, asked + 1), ("ready", 45, asked + 2)]);
    assert_eq!(std::str::from_utf8(&rows[0].buffer_sha256).unwrap(), edit.result_sha256);
    assert!(!text(&handle, LX4_TEXT_STATUS).contains("rerun"), "sample 45 is no re-run sample");
}

/// RTE.50's reference view, as the shell hands it to the host's view job: the module named in the buffer, entry
/// `REFERENCE_ENTRY`, no arguments, no host rows. At S, M and L, the baseline and the inside-body edit's result both
/// complete with a value, and the edit changes it. Limits as the host's `VIEW_LIMITS`.
#[test]
fn the_reference_view_runs_at_s_m_and_l_without_host_rows() {
    let loaded = fixtures::load(folder()).unwrap();
    let handle = open();
    let run = |source: &str| {
        let module = c600_syntax::parse(source.as_bytes()).module.map(|m| m.name.text).unwrap_or_default();
        let limits = Amounts { steps: 1 << 32, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };
        let job = Job {
            sources: vec![source.as_bytes().to_vec()],
            module,
            entry: REFERENCE_ENTRY.into(),
            args: Vec::new(),
            options: JobOptions { record: false, ..JobOptions::new(limits) },
        };
        match run_in_process(&job, &mut NoHost, None) {
            Ended::Ran(run) => run.output.expect("a completed run's output"),
            Ended::Refused(refused) => panic!("{refused:?}"),
        }
    };
    for (n, target) in ["S", "M", "L"].into_iter().enumerate() {
        let index = edit_index(&loaded, target, "inside-body");
        let edit = &loaded.edits[index];
        let sample = 50 + n as u64;
        let inputs = messages(edit, index as u32 | LX4_RESET_RERUN, sample);
        assert_eq!(post(&handle, inputs[0]), LX4_OK, "{}", last_error(handle.0));
        let baseline = text(&handle, LX4_TEXT_BUFFER);
        assert_eq!(hex(&sha256(baseline.as_bytes())), edit.base_sha256);
        for &input in &inputs[1..] {
            assert_eq!(post(&handle, input), LX4_OK, "{}", last_error(handle.0));
        }
        let result = text(&handle, LX4_TEXT_BUFFER);
        assert_eq!(hex(&sha256(result.as_bytes())), edit.result_sha256);
        assert_ne!(run(&baseline), run(&result), "{target}: the edit changes the view's value");
    }
}

#[test]
fn a_turn_keys_character_is_left_to_the_shell() {
    let handle = open();
    assert_eq!(post(&handle, Input::Reset(0, 1)), LX4_OK);
    update(&handle, 1);
    let echo = text(&handle, LX4_TEXT_ECHO);
    for scan in 0x24..=0x26u32 {
        let mut taken = 9u32;
        // SAFETY: a live handle; `taken` is writable.
        assert_eq!(unsafe { lx4_char(handle.0, u16::from(b'j'), scan, &mut taken) }, LX4_OK);
        assert_eq!(taken, 0);
    }
    let changed = update(&handle, 2);
    assert_eq!(changed.changed & 1 << LX4_TEXT_ECHO, 0, "the buffer did not change");
    assert_eq!(text(&handle, LX4_TEXT_ECHO), echo);
    assert!(text(&handle, LX4_TEXT_STATUS).contains("0 of"));
}

#[test]
fn an_input_past_the_edit_fails_the_sample_and_queues_nothing() {
    let loaded = fixtures::load(folder()).unwrap();
    let handle = open();
    for input in messages(&loaded.edits[0], 0, 42) {
        assert_eq!(post(&handle, input), LX4_OK);
    }
    assert_eq!(post(&handle, Input::Char(u16::from(b'x'))), LX4_E_STATE);
    assert!(last_error(handle.0).contains("more inputs than edit 0's"), "{}", last_error(handle.0));
    for frame in 1..50 {
        update(&handle, frame);
        assert!(drain(&handle).is_empty());
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(text(&handle, LX4_TEXT_STATUS).contains("FAILED"), "{}", text(&handle, LX4_TEXT_STATUS));
    // A new Reset starts a clean sample.
    assert_eq!(post(&handle, Input::Reset(0, 43)), LX4_OK);
    update(&handle, 50);
    assert!(!text(&handle, LX4_TEXT_STATUS).contains("FAILED"));
}

#[test]
fn inputs_the_sample_cannot_take_are_refused() {
    let handle = open();
    assert_eq!(post(&handle, Input::Char(u16::from(b'x'))), LX4_E_STATE);
    assert!(last_error(handle.0).contains("before any Magic600.LX4.Reset"));
    assert_eq!(post(&handle, Input::Reset(16, 1)), LX4_E_STATE, "the script has 16 edits");
    assert!(last_error(handle.0).contains("Reset names edit 16"));
    assert_eq!(post(&handle, Input::Reset(0, 2)), LX4_OK);
    assert_eq!(post(&handle, Input::Select(5, 4)), LX4_E_STATE);
    assert!(last_error(handle.0).contains("no selection"));
    assert_eq!(post(&handle, Input::Reset(0, 3)), LX4_OK);
    assert_eq!(post(&handle, Input::Select(0, u64::from(u32::MAX))), LX4_E_STATE);
    assert_eq!(post(&handle, Input::Reset(0, 4)), LX4_OK);
    assert_eq!(post(&handle, Input::Select(0, 0)), LX4_OK);
    assert_eq!(post(&handle, Input::Char(0xDC00)), LX4_E_STATE, "a low surrogate alone");
}

#[test]
fn a_surrogate_pair_types_one_character() {
    let loaded = fixtures::load(folder()).unwrap();
    let handle = open();
    // Edit 2 is S's completion trigger: one insert at the end, so any two of its units can be a pair.
    let edit = &loaded.edits[2];
    assert_eq!(edit.kind, "completion-trigger");
    assert_eq!(post(&handle, Input::Reset(2, 9)), LX4_OK);
    assert_eq!(
        post(&handle, Input::Select(edit.replacements[0].offset as u64, edit.replacements[0].offset as u64)),
        LX4_OK
    );
    for unit in "\u{1F600}".encode_utf16() {
        assert_eq!(post(&handle, Input::Char(unit)), LX4_OK);
    }
    update(&handle, 1);
    assert!(text(&handle, LX4_TEXT_ECHO).contains('\u{1F600}'));
    assert!(text(&handle, LX4_TEXT_STATUS).contains("3 of"));
}

#[test]
fn a_folder_that_does_not_check_is_refused() {
    let (code, handle) = create(&folder().join("missing"));
    assert_eq!(code, LX4_E_FIXTURES);
    assert!(handle.is_null());
    assert!(last_error(ptr::null_mut()).contains("MANIFEST.json"), "{}", last_error(ptr::null_mut()));

    let copy = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("lx4-tampered-{}", std::process::id()));
    let _ = fs::remove_dir_all(&copy);
    fs::create_dir_all(copy.join("W")).unwrap();
    for entry in ["MANIFEST.json", "EDITS.json", "S.6cs", "M.6cs", "L.6cs"] {
        fs::copy(folder().join(entry), copy.join(entry)).unwrap();
    }
    for k in 0..c600_lx4::W_MODULES {
        let name = format!("W/W{k:02}.6cs");
        fs::copy(folder().join(&name), copy.join(&name)).unwrap();
    }
    let (code, handle) = create(&copy);
    assert_eq!(code, LX4_OK, "{}", last_error(ptr::null_mut()));
    drop(Handle(handle));
    let mut source = fs::read(copy.join("M.6cs")).unwrap();
    source.push(b'\n');
    fs::write(copy.join("M.6cs"), source).unwrap();
    let (code, handle) = create(&copy);
    assert_eq!(code, LX4_E_FIXTURES);
    assert!(handle.is_null());
    assert!(last_error(ptr::null_mut()).contains("M.6cs: the digest differs"), "{}", last_error(ptr::null_mut()));
    let _ = fs::remove_dir_all(&copy);
}

#[test]
fn null_arguments_are_refused() {
    let mut size = 0u32;
    let mut update = Lx4Update::default();
    let mut taken = 0u32;
    // SAFETY: every pointer is null or writable.
    unsafe {
        assert_eq!(lx4_create(ptr::null(), 0, ptr::null_mut()), LX4_E_INVALID_ARGUMENT);
        let mut out = ptr::null_mut();
        assert_eq!(lx4_create(ptr::null(), 0, &mut out), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_reset(ptr::null_mut(), 0, 0), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_select(ptr::null_mut(), 0, 0), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_char(ptr::null_mut(), 32, 0, &mut taken), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_backspace(ptr::null_mut()), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_update(ptr::null_mut(), 0, &mut update), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_text(ptr::null_mut(), 0, ptr::null_mut(), 0, &mut size), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_rerun_ended(ptr::null_mut(), 0, ptr::null(), ptr::null(), 0), LX4_E_INVALID_ARGUMENT);
        lx4_destroy(ptr::null_mut());
    }
    let handle = open();
    // SAFETY: a live handle; the other pointers are null on purpose.
    unsafe {
        assert_eq!(lx4_update(handle.0, 0, ptr::null_mut()), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_char(handle.0, 32, 0, ptr::null_mut()), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_text(handle.0, 4, ptr::null_mut(), 0, &mut size), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_rerun_ended(handle.0, 0, ptr::null(), ptr::null(), 5), LX4_E_INVALID_ARGUMENT);
        assert_eq!(lx4_rerun_ended(handle.0, 0, ptr::null(), ptr::null(), 0), LX4_E_STATE, "no re-run is running");
    }
}

#[test]
fn the_prefix_is_the_identifier_before_the_caret() {
    assert_eq!(prefix("    return exact_", 17), "exact_");
    assert_eq!(prefix("    return exact_add(1, 42);", 28), "");
    assert_eq!(prefix("    return 1", 12), "");
    assert_eq!(prefix("fn exported() -> Text", 21), "Text");
    assert_eq!(prefix("", 0), "");
}
