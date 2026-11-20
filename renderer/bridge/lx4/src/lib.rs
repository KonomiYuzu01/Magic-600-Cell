//! LX-4's editor module for C9's product shell (runbook U8; chapter 11, RTE.47 to RTE.53), a draft. The shell's C#
//! module (`renderer/shell/project/Lx4`) loads this library from `M600_LX4_DLL`, hands it the fixture folder from
//! `M600_LX4_FIXTURES`, forwards lx4.py's window messages to it, and once per process frame asks it what to draw
//! and which endpoints that frame shows first. The protocol is lx4.py's docstring.
//!
//! - **Buffer:** [`lx4_reset`] (registered `Magic600.LX4.Reset`: wParam the edit's index in EDITS.json, lParam the
//!   sample id) loads the edit's baseline and checks its digest; [`lx4_select`] (`Magic600.LX4.Select`, UTF-8 byte
//!   offsets), [`lx4_char`] (WM_CHAR) and [`lx4_backspace`] (VK_BACK's key-down) edit it. A WM_CHAR with scan code
//!   0x24 to 0x26 is Godot's TranslateMessage of a turn key, J, K or L: [`lx4_char`] leaves it to the shell.
//! - **Work:** the language work runs on a worker thread (the `analysis` module), so the frame loop never waits on
//!   it. [`lx4_update`] sends the buffer's newest revision, takes the answers for that revision, and recomposes the
//!   three texts: the echo (the lines around the caret), the status and the completion list.
//! - **Ready:** the first frame after a good Reset that draws the baseline, before any input of the sample, queues
//!   `ready` with the baseline's digest; lx4.py posts the sample's inputs only after it sees that row, so no timed
//!   input waits behind the Reset's work. While a re-run is running (below), `ready` waits for it to end.
//! - **Endpoints:** once a sample's last input has arrived, [`lx4_update`] queues an [`Lx4Shown`] for each endpoint
//!   the frame shows for the final buffer, each once per sample, and the shell writes them as `lx4` rows with this
//!   frame's 3D frame id: `echo` names the frame that first showed the sample's first typed key; `stale` the frame
//!   that shows the view stale for the final buffer; `syntax`, `completion` (a non-empty list), `diagnostics` (the
//!   module checked, only when its syntax is clean) and, for W00, `dependents` the frame that first shows the final
//!   revision's answer. A failed sample (an input past the edit's end, a selection off a character boundary) queues
//!   nothing, so the harness times it out and the run is invalid.
//! - **Re-run** (`rerun`, RTE.50; C9, 20 Nov 2026): a Reset whose `edit` carries [`LX4_RESET_RERUN`] starts a sample
//!   that times the re-run. In the first [`lx4_update`] after its last input, [`Lx4Update::rerun`] asks the shell to
//!   run the reference view on the final buffer ([`LX4_TEXT_BUFFER`]): the edited module's [`REFERENCE_ENTRY`] as the
//!   host's view job (`m6h_run_view`), in the language runtime with no host rows. When the job ends, the shell calls
//!   [`lx4_rerun_ended`] with the first confirmed frame that drew the run's publication and the run's source digest;
//!   when that digest is the final buffer's, the module queues `rerun` for that frame. The host does not draw the
//!   run's value yet (its draft), so that frame shows the reference publication the run delivered.
//!
//! The stale state is c600-view's (the `view` module): each sample's view model has a view of the baseline, the
//! editor's draft reaches it by source digest, and the view model says when the view lags (EV1.7, EV3.6, RTE.49).
//! Nothing here is timing, Windows/DirectX or input evidence.
//!
//! Calls: a handle is used by one thread at a time (the shell's main thread). Every function checks its pointers,
//! catches a panic and answers a status code. A panic poisons the handle: it then answers only [`LX4_E_POISONED`]
//! until it is destroyed. This library loads no library and calls no Windows function.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::ptr;
use std::sync::Arc;

use c600_digest::{hex, sha256};

pub mod analysis;
pub mod editor;
pub mod fixtures;
pub mod view;

use analysis::{Answer, Job, Worker, COMPLETION, DEPENDENTS, DIAGNOSTICS, SYNTAX};
use editor::{Editor, Typed};
use fixtures::Loaded;
use view::StaleView;

/// The version of this C interface. The shell refuses a library that answers another. Version 2 adds the re-run:
/// [`LX4_RESET_RERUN`], [`Lx4Update::rerun`], [`LX4_TEXT_BUFFER`] and [`lx4_rerun_ended`].
pub const LX4_ABI_VERSION: u32 = 2;

pub const LX4_OK: i32 = 0;
/// A null or malformed argument, or a buffer too small for a text.
pub const LX4_E_INVALID_ARGUMENT: i32 = 1;
/// The fixture folder does not load or check.
pub const LX4_E_FIXTURES: i32 = 2;
/// An input the sample cannot take: before any Reset, past the edit's last input, or off the buffer. The sample has
/// failed; [`lx4_last_error`] says why.
pub const LX4_E_STATE: i32 = 3;
/// A call panicked earlier; the handle answers only this until it is destroyed.
pub const LX4_E_POISONED: i32 = 6;

/// [`lx4_text`]'s texts and [`Lx4Update::changed`]'s bits (`1 << text`).
pub const LX4_TEXT_ECHO: u32 = 0;
pub const LX4_TEXT_STATUS: u32 = 1;
pub const LX4_TEXT_COMPLETION: u32 = 2;
/// [`lx4_text`]'s whole buffer, which the shell runs as the reference view; no [`Lx4Update::changed`] bit.
pub const LX4_TEXT_BUFFER: u32 = 3;

/// [`lx4_reset`]'s flag in `edit` (bits 0 to 15 are the edit's index): the sample times `rerun`.
pub const LX4_RESET_RERUN: u32 = 1 << 16;

/// The reference view's entry, the exported function every edited module has once (RTE.50: S, M and L import
/// nothing and read no host row).
pub const REFERENCE_ENTRY: &str = "exported";

/// The endpoints by [`Lx4Shown::endpoint`], with lx4.py's names. `ready` is the handshake, no metric.
pub const ENDPOINTS: [&str; 8] =
    ["echo", "syntax", "completion", "diagnostics", "stale", "rerun", "dependents", "ready"];
pub const ECHO: usize = 0;
pub const STALE: usize = 4;
pub const RERUN: usize = 5;
pub const READY: usize = 7;

/// How many lines the echo shows on each side of the caret's.
pub const CONTEXT: usize = 12;

/// What [`lx4_update`] changed: which texts to draw again, how many endpoints are queued for [`lx4_next_shown`], and
/// whether the shell is to run the reference view on the buffer now.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lx4Update {
    pub changed: u32,
    pub shown: u32,
    /// 1 once per re-run sample, in the first update after its last input: the shell runs [`REFERENCE_ENTRY`] of
    /// the buffer ([`LX4_TEXT_BUFFER`]) as the host's view job and reports its end with [`lx4_rerun_ended`].
    pub rerun: u32,
    pub reserved: u32,
}

/// An endpoint a frame shows first, for an `lx4` row: `{"t": "lx4", "sample", "endpoint", "frame_id",
/// "buffer_sha256"}`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lx4Shown {
    pub sample: u64,
    pub frame_id: u64,
    /// An index into [`ENDPOINTS`].
    pub endpoint: u32,
    pub reserved: u32,
    /// The buffer's SHA-256 when the endpoint was queued, 64 lowercase hex digits, not NUL-terminated.
    pub buffer_sha256: [u8; 64],
}

/// The answers for the current revision, one per stage.
#[derive(Default)]
struct Answers {
    by_stage: [Option<Answer>; ENDPOINTS.len()],
}

/// The re-run the shell was asked for and has not reported ended: its sample and the buffer's digest it runs.
#[derive(Clone, Copy, Debug)]
struct Running {
    sample: u64,
    digest: [u8; 32],
}

/// One editor module.
pub struct Lx4Module {
    loaded: Arc<Loaded>,
    editor: Editor,
    worker: Worker,
    /// The revision last sent to the worker, and its digest.
    sent: u64,
    digest: String,
    answers: Answers,
    /// The sample's view of its baseline, whose stale state the frames show.
    view: Option<StaleView>,
    /// The frame that first showed the sample's first typed key, and the endpoints queued for this sample.
    echo_frame: Option<u64>,
    queued: [bool; ENDPOINTS.len()],
    /// Whether the sample times the re-run, whether the shell was asked for it, the re-run not yet ended (it may be
    /// an earlier sample's), and the status line about the sample's re-run.
    rerun: bool,
    rerun_asked: bool,
    running: Option<Running>,
    rerun_line: String,
    texts: [String; 3],
    shown: VecDeque<Lx4Shown>,
    poisoned: bool,
    error: String,
}

thread_local! {
    /// Why the last [`lx4_create`] on this thread failed.
    static CREATE_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

impl Lx4Module {
    /// Loads and checks the fixture folder and starts the worker.
    pub fn new(folder: &Path) -> Result<Lx4Module, String> {
        let loaded = Arc::new(fixtures::load(folder)?);
        let workspace = Arc::new(loaded.fixtures.workspace.clone());
        let worker = Worker::start(workspace).map_err(|error| format!("the worker thread does not start: {error}"))?;
        let mut module = Lx4Module {
            loaded,
            editor: Editor::default(),
            worker,
            sent: 0,
            digest: hex(&sha256(b"")),
            answers: Answers::default(),
            view: None,
            echo_frame: None,
            queued: [false; ENDPOINTS.len()],
            rerun: false,
            rerun_asked: false,
            running: None,
            rerun_line: String::new(),
            texts: Default::default(),
            shown: VecDeque::new(),
            poisoned: false,
            error: String::new(),
        };
        module.texts[LX4_TEXT_ECHO as usize] = "LX-4: waiting for Magic600.LX4.Reset".to_owned();
        Ok(module)
    }

    /// Starts sample `sample` of edit `edit`; `rerun` when the sample times the re-run. A re-run still running for an
    /// earlier sample goes on; `ready` waits for it.
    pub fn reset(&mut self, edit: usize, sample: u64, rerun: bool) -> Result<(), String> {
        self.echo_frame = None;
        self.queued = [false; ENDPOINTS.len()];
        self.rerun = rerun;
        self.rerun_asked = false;
        self.rerun_line = if rerun { "rerun: waiting for the last input".to_owned() } else { String::new() };
        self.view = None;
        self.editor.reset(&self.loaded, edit, sample)?;
        match StaleView::open(sha256(self.editor.text.as_bytes())) {
            Ok(view) => {
                self.view = Some(view);
                Ok(())
            }
            Err(reason) => Err(self.editor.fail(format!("the view of the baseline: {reason}"))),
        }
    }

    pub fn select(&mut self, start: usize, end: usize) -> Result<(), String> {
        self.editor.select(start, end)
    }

    /// A WM_CHAR unit with its scan code. `Ok(false)` leaves it to the shell: Godot's character for a turn key.
    pub fn char(&mut self, unit: u16, scan: u32) -> Result<bool, String> {
        if (0x24..=0x26).contains(&scan) {
            return Ok(false);
        }
        self.editor.char(unit).map(|_: Typed| true)
    }

    pub fn backspace(&mut self) -> Result<(), String> {
        self.editor.backspace()
    }

    pub fn text(&self, which: u32) -> Option<&str> {
        if which == LX4_TEXT_BUFFER {
            return Some(&self.editor.text);
        }
        self.texts.get(which as usize).map(String::as_str)
    }

    /// The shell's report that the re-run it was asked for has ended: `frame` the first confirmed frame that drew the
    /// run's publication and `digest` the run's source digest, or `frame` 0 and `reason` when the run was refused or
    /// never started. A published run of the asked buffer queues `rerun` for its sample, even when a later Reset has
    /// started another sample since. Errors when no re-run is running.
    pub fn rerun_ended(&mut self, frame: u64, digest: Option<[u8; 32]>, reason: &str) -> Result<(), String> {
        let Some(run) = self.running.take() else {
            return Err("no re-run is running".to_owned());
        };
        let line = match digest {
            Some(digest) if frame != 0 && digest == run.digest => {
                let mut buffer_sha256 = [0u8; 64];
                buffer_sha256.copy_from_slice(hex(&run.digest).as_bytes());
                self.shown.push_back(Lx4Shown {
                    sample: run.sample,
                    frame_id: frame,
                    endpoint: RERUN as u32,
                    reserved: 0,
                    buffer_sha256,
                });
                format!("rerun: shown in frame {frame}")
            }
            Some(digest) if frame != 0 => format!(
                "rerun: REFUSED: the run's text {} is not the buffer {}",
                &hex(&digest)[..12],
                &hex(&run.digest)[..12]
            ),
            _ if reason.is_empty() => "rerun: REFUSED".to_owned(),
            _ => format!("rerun: REFUSED: {reason}"),
        };
        if self.editor.sample.as_ref().is_some_and(|s| s.id == run.sample) {
            self.rerun_line = line;
        }
        Ok(())
    }

    pub fn next_shown(&mut self) -> Option<Lx4Shown> {
        self.shown.pop_front()
    }

    /// One process frame, whose 3D frame is `frame`: sends a changed buffer to the worker, takes the answers for
    /// the current revision, recomposes the texts and queues the endpoints this frame shows first.
    pub fn update(&mut self, frame: u64) -> Lx4Update {
        let mut changed = 0;
        if self.editor.revision != self.sent {
            self.sent = self.editor.revision;
            let digest = sha256(self.editor.text.as_bytes());
            self.digest = hex(&digest);
            if let Some(view) = &mut self.view {
                view.edit(digest);
            }
            self.answers = Answers::default();
            self.texts[LX4_TEXT_ECHO as usize] = self.echo();
            self.texts[LX4_TEXT_COMPLETION as usize].clear();
            changed |= 1 << LX4_TEXT_ECHO | 1 << LX4_TEXT_COMPLETION;
            if let Some(sample) = &self.editor.sample {
                let w00 = self.loaded.edits.get(sample.edit).is_some_and(|e| e.target == "W00");
                self.worker.send(Job {
                    revision: self.sent,
                    text: self.editor.text.clone(),
                    caret: self.editor.end,
                    w00: w00.then_some(self.loaded.w00),
                });
            }
        }
        if self.echo_frame.is_none() && self.editor.sample.as_ref().is_some_and(|s| s.typed) {
            self.echo_frame = Some(frame);
        }
        for answer in self.worker.drain() {
            if answer.revision != self.sent {
                continue;
            }
            if answer.stage == COMPLETION {
                self.texts[LX4_TEXT_COMPLETION as usize].clone_from(&answer.text);
                changed |= 1 << LX4_TEXT_COMPLETION;
            }
            let stage = answer.stage;
            self.answers.by_stage[stage] = Some(answer);
        }
        let complete = self.editor.sample.as_ref().is_some_and(|s| s.complete());
        let mut rerun = 0;
        if complete && self.rerun && !self.rerun_asked {
            // The host runs one view job at a time, so this run supersedes a run an earlier sample left running.
            self.rerun_asked = true;
            let sample = self.editor.sample.as_ref().map_or(0, |s| s.id);
            self.running = Some(Running { sample, digest: sha256(self.editor.text.as_bytes()) });
            self.rerun_line = "rerun: running the reference view".to_owned();
            rerun = 1;
        }
        // The status also counts the inputs and shows a failure, which change no revision.
        let status = self.status();
        if status != self.texts[LX4_TEXT_STATUS as usize] {
            self.texts[LX4_TEXT_STATUS as usize] = status;
            changed |= 1 << LX4_TEXT_STATUS;
        }
        let baseline = self.editor.sample.as_ref().is_some_and(|s| s.failure.is_none() && s.received == 0);
        if baseline && self.view.is_some() && self.running.is_none() {
            self.push(READY, frame);
        }
        if complete {
            self.queue(frame);
        }
        Lx4Update { changed, shown: self.shown.len() as u32, rerun, reserved: 0 }
    }

    /// Queues the endpoints the frame shows for the complete sample's final buffer, each once.
    fn queue(&mut self, frame: u64) {
        let stale = self.stale();
        let mut due = Vec::new();
        if let Some(echo) = self.echo_frame {
            due.push((ECHO, echo));
        }
        if stale {
            due.push((STALE, frame));
        }
        for stage in [SYNTAX, COMPLETION, DIAGNOSTICS, DEPENDENTS] {
            if self.answers.by_stage[stage].as_ref().is_some_and(|a| a.endpoint) {
                due.push((stage, frame));
            }
        }
        for (endpoint, frame_id) in due {
            self.push(endpoint, frame_id);
        }
    }

    /// Queues `endpoint` for the sample with the buffer's digest, unless the sample queued it already.
    fn push(&mut self, endpoint: usize, frame_id: u64) {
        let Some(sample) = self.editor.sample.as_ref().map(|s| s.id) else { return };
        if std::mem::replace(&mut self.queued[endpoint], true) {
            return;
        }
        let mut buffer_sha256 = [0u8; 64];
        buffer_sha256.copy_from_slice(self.digest.as_bytes());
        self.shown.push_back(Lx4Shown { sample, frame_id, endpoint: endpoint as u32, reserved: 0, buffer_sha256 });
    }

    /// Whether the frame shows the view stale: its output's producer, the baseline, has a draft in the editor.
    fn stale(&self) -> bool {
        self.view.as_ref().and_then(StaleView::lag).is_some()
    }

    /// The lines around the caret, under a header with the sample, the caret and the buffer's digest.
    fn echo(&self) -> String {
        let Some(sample) = &self.editor.sample else { return String::new() };
        let edit = self.loaded.edits.get(sample.edit);
        let (line, column) = self.editor.caret();
        let mut out = format!(
            "{} {}  sample {}  {line}:{column}  {}\n",
            edit.map_or("?", |e| e.target.as_str()),
            edit.map_or("?", |e| e.kind.as_str()),
            sample.id,
            &self.digest[..12]
        );
        let lines: Vec<&str> = self.editor.text.split('\n').collect();
        let first = (line - 1).saturating_sub(CONTEXT);
        let last = (line + CONTEXT).min(lines.len());
        for (index, text) in lines[first..last].iter().enumerate() {
            let _ = writeln!(out, "{:>5}  {text}", first + index + 1);
        }
        out
    }

    fn status(&self) -> String {
        let Some(sample) = &self.editor.sample else { return String::new() };
        let mut out = format!("sample {}: {} of {} inputs", sample.id, sample.received, sample.expected);
        if let Some(failure) = &sample.failure {
            let _ = write!(out, ", FAILED: {failure}");
        }
        out.push('\n');
        let w00 = self.loaded.edits.get(sample.edit).is_some_and(|e| e.target == "W00");
        let pending = ["", "syntax: pending", "", "diagnostics: pending", "", "", "dependents: pending"];
        for stage in [SYNTAX, DIAGNOSTICS] {
            let answer = self.answers.by_stage[stage].as_ref();
            let _ = writeln!(out, "{}", answer.map_or(pending[stage], |a| a.text.as_str()));
        }
        match self.view.as_ref().and_then(StaleView::lag) {
            Some(lag) => {
                let (producer, draft) = (hex(&lag.producer), hex(&lag.draft));
                let _ = writeln!(out, "view: STALE (its producer {} has the draft {})", &producer[..12], &draft[..12]);
            }
            None => {
                let _ = writeln!(out, "view: current");
            }
        }
        if w00 {
            let answer = self.answers.by_stage[DEPENDENTS].as_ref();
            let _ = writeln!(out, "{}", answer.map_or(pending[DEPENDENTS], |a| a.text.as_str()));
        }
        if !self.rerun_line.is_empty() {
            let _ = writeln!(out, "{}", self.rerun_line);
        }
        out
    }
}

type Failure = (i32, String);

/// Runs `body` on a live handle and answers its status code, recording the error text.
///
/// # Safety
///
/// `handle` is null or a live handle from [`lx4_create`] that no other thread is using.
unsafe fn with_module(handle: *mut Lx4Module, body: impl FnOnce(&mut Lx4Module) -> Result<(), Failure>) -> i32 {
    // SAFETY: the caller's contract.
    let Some(module) = (unsafe { handle.as_mut() }) else {
        return LX4_E_INVALID_ARGUMENT;
    };
    if module.poisoned {
        return LX4_E_POISONED;
    }
    match catch_unwind(AssertUnwindSafe(|| body(&mut *module))) {
        Ok(Ok(())) => LX4_OK,
        Ok(Err((code, text))) => {
            module.error = text;
            code
        }
        Err(_) => {
            module.poisoned = true;
            module.error = "a call panicked; the handle only answers LX4_E_POISONED now".to_owned();
            LX4_E_POISONED
        }
    }
}

fn state(result: Result<(), String>) -> Result<(), Failure> {
    result.map_err(|text| (LX4_E_STATE, text))
}

/// The version of this C interface, [`LX4_ABI_VERSION`].
#[no_mangle]
pub extern "C" fn lx4_abi_version() -> u32 {
    LX4_ABI_VERSION
}

/// Loads and checks the fixture folder and starts the worker. On success `*out` is the new handle; on failure it is
/// null and [`lx4_last_error`] with a null handle gives the reason.
///
/// # Safety
///
/// `folder` points to `folder_len` bytes, the UTF-8 path of the fixture folder; `out` points to a writable handle.
#[no_mangle]
pub unsafe extern "C" fn lx4_create(folder: *const u8, folder_len: usize, out: *mut *mut Lx4Module) -> i32 {
    if out.is_null() {
        return LX4_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe { *out = ptr::null_mut() };
    let failure = |code: i32, text: String| {
        CREATE_ERROR.with(|e| *e.borrow_mut() = text);
        code
    };
    if folder.is_null() {
        return failure(LX4_E_INVALID_ARGUMENT, "the fixture folder's path is null".to_owned());
    }
    // SAFETY: the contract.
    let bytes = unsafe { std::slice::from_raw_parts(folder, folder_len) };
    let Ok(path) = std::str::from_utf8(bytes) else {
        return failure(LX4_E_INVALID_ARGUMENT, "the fixture folder's path is not UTF-8".to_owned());
    };
    match catch_unwind(|| Lx4Module::new(Path::new(path))) {
        Ok(Ok(module)) => {
            // SAFETY: the contract.
            unsafe { *out = Box::into_raw(Box::new(module)) };
            LX4_OK
        }
        Ok(Err(text)) => failure(LX4_E_FIXTURES, text),
        Err(_) => failure(LX4_E_FIXTURES, "loading the fixtures panicked".to_owned()),
    }
}

/// Stops the worker and frees the handle. A null handle is ignored.
///
/// # Safety
///
/// `handle` is null or a live handle from [`lx4_create`], used by nothing after this call.
#[no_mangle]
pub unsafe extern "C" fn lx4_destroy(handle: *mut Lx4Module) {
    if !handle.is_null() {
        // SAFETY: the contract; the worker joins in its Drop.
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// `Magic600.LX4.Reset`: starts sample `sample` of the edit whose index is `edit` without [`LX4_RESET_RERUN`], from
/// the edit's baseline; with the flag, the sample times the re-run.
///
/// # Safety
///
/// As [`with_module`].
#[no_mangle]
pub unsafe extern "C" fn lx4_reset(handle: *mut Lx4Module, edit: u32, sample: u64) -> i32 {
    let rerun = edit & LX4_RESET_RERUN != 0;
    // SAFETY: the contract.
    unsafe { with_module(handle, |m| state(m.reset((edit & !LX4_RESET_RERUN) as usize, sample, rerun))) }
}

/// `Magic600.LX4.Select`: the selection `[start, end)` in UTF-8 bytes of the buffer.
///
/// # Safety
///
/// As [`with_module`].
#[no_mangle]
pub unsafe extern "C" fn lx4_select(handle: *mut Lx4Module, start: u64, end: u64) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_module(handle, |m| {
            let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
                return Err((LX4_E_STATE, "the selection does not fit this machine's offsets".to_owned()));
            };
            state(m.select(start, end))
        })
    }
}

/// WM_CHAR: `unit` is wParam, `scan` lParam's bits 16 to 23. `*out_taken` is 1 when the module took the character
/// (the shell then drops the message) and 0 when it belongs to the shell (Godot's character for a turn key).
///
/// # Safety
///
/// As [`with_module`]; `out_taken` is writable.
#[no_mangle]
pub unsafe extern "C" fn lx4_char(handle: *mut Lx4Module, unit: u16, scan: u32, out_taken: *mut u32) -> i32 {
    if out_taken.is_null() {
        return LX4_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe { *out_taken = 1 };
    // SAFETY: the contract.
    unsafe {
        with_module(handle, |m| {
            let taken = m.char(unit, scan).map_err(|text| (LX4_E_STATE, text))?;
            // SAFETY: checked above.
            *out_taken = u32::from(taken);
            Ok(())
        })
    }
}

/// VK_BACK's key-down. The shell drops its key-up and the WM_CHAR 0x08 Godot makes of it ([`lx4_char`] ignores it).
///
/// # Safety
///
/// As [`with_module`].
#[no_mangle]
pub unsafe extern "C" fn lx4_backspace(handle: *mut Lx4Module) -> i32 {
    // SAFETY: the contract.
    unsafe { with_module(handle, |m| state(m.backspace())) }
}

/// One process frame, whose submitted 3D frame has id `frame_id`; see [`Lx4Module::update`].
///
/// # Safety
///
/// As [`with_module`]; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn lx4_update(handle: *mut Lx4Module, frame_id: u64, out: *mut Lx4Update) -> i32 {
    if out.is_null() {
        return LX4_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_module(handle, |m| {
            *out = m.update(frame_id);
            Ok(())
        })
    }
}

/// The end of the re-run [`Lx4Update::rerun`] asked for; see [`Lx4Module::rerun_ended`]. A published run passes the
/// first confirmed frame that drew its publication as `frame_id` and its 32-byte source digest; a refused run, or one
/// the shell could not start, passes `frame_id` 0, a null `digest` and `reason_len` bytes of UTF-8 `reason`. Answers
/// [`LX4_E_STATE`] when no re-run is running.
///
/// # Safety
///
/// As [`with_module`]; `digest` is null or points to 32 readable bytes; `reason` is null with `reason_len` 0 or
/// points to `reason_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn lx4_rerun_ended(
    handle: *mut Lx4Module,
    frame_id: u64,
    digest: *const u8,
    reason: *const u8,
    reason_len: usize,
) -> i32 {
    if reason.is_null() && reason_len != 0 {
        return LX4_E_INVALID_ARGUMENT;
    }
    let digest = (!digest.is_null()).then(|| {
        let mut out = [0u8; 32];
        // SAFETY: the contract.
        unsafe { ptr::copy_nonoverlapping(digest, out.as_mut_ptr(), out.len()) };
        out
    });
    let reason = if reason.is_null() {
        String::new()
    } else {
        // SAFETY: the contract.
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(reason, reason_len) }).into_owned()
    };
    // SAFETY: the contract.
    unsafe { with_module(handle, |m| state(m.rerun_ended(frame_id, digest, &reason))) }
}

/// Copies `text` and a NUL into `buffer`; `*out_size` is the bytes needed, NUL included.
///
/// # Safety
///
/// `buffer` points to `capacity` writable bytes or is null; `out_size` is writable.
unsafe fn copy_text(text: &str, buffer: *mut u8, capacity: u32, out_size: *mut u32) -> Result<(), Failure> {
    let needed = text.len() + 1;
    // SAFETY: the contract.
    unsafe {
        *out_size = u32::try_from(needed).map_err(|_| (LX4_E_INVALID_ARGUMENT, "the text is too long".to_owned()))?;
        if buffer.is_null() || (capacity as usize) < needed {
            return Err((LX4_E_INVALID_ARGUMENT, format!("the text needs {needed} bytes")));
        }
        ptr::copy_nonoverlapping(text.as_ptr(), buffer, text.len());
        *buffer.add(text.len()) = 0;
    }
    Ok(())
}

/// One of the three texts ([`LX4_TEXT_ECHO`], [`LX4_TEXT_STATUS`], [`LX4_TEXT_COMPLETION`]) or the buffer
/// ([`LX4_TEXT_BUFFER`]) as NUL-terminated UTF-8.
/// `*out_size` is the bytes needed, NUL included; a null or short buffer answers [`LX4_E_INVALID_ARGUMENT`] with the
/// size set.
///
/// # Safety
///
/// As [`with_module`]; `buffer` points to `capacity` writable bytes or is null; `out_size` is writable.
#[no_mangle]
pub unsafe extern "C" fn lx4_text(
    handle: *mut Lx4Module,
    which: u32,
    buffer: *mut u8,
    capacity: u32,
    out_size: *mut u32,
) -> i32 {
    if out_size.is_null() {
        return LX4_E_INVALID_ARGUMENT;
    }
    // A size query is no error, so this call leaves the handle's last error as it is.
    // SAFETY: the contract.
    let Some(module) = (unsafe { handle.as_ref() }) else {
        return LX4_E_INVALID_ARGUMENT;
    };
    if module.poisoned {
        return LX4_E_POISONED;
    }
    let Some(text) = module.text(which) else {
        return LX4_E_INVALID_ARGUMENT;
    };
    // SAFETY: the contract.
    match unsafe { copy_text(text, buffer, capacity, out_size) } {
        Ok(()) => LX4_OK,
        Err((code, _)) => code,
    }
}

/// The oldest queued endpoint into `*out`, with `*out_present` 1; `*out_present` 0 when none is queued.
///
/// # Safety
///
/// As [`with_module`]; `out` and `out_present` are writable.
#[no_mangle]
pub unsafe extern "C" fn lx4_next_shown(handle: *mut Lx4Module, out: *mut Lx4Shown, out_present: *mut u32) -> i32 {
    if out.is_null() || out_present.is_null() {
        return LX4_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        *out_present = 0;
        with_module(handle, |m| {
            if let Some(shown) = m.next_shown() {
                *out = shown;
                *out_present = 1;
            }
            Ok(())
        })
    }
}

/// The last error of `handle`, or of the last failed [`lx4_create`] on this thread when `handle` is null, as
/// NUL-terminated UTF-8, sized as [`lx4_text`] sizes its texts.
///
/// # Safety
///
/// `handle` is null or a live handle that no other thread is using; `buffer` points to `capacity` writable bytes or
/// is null; `out_size` is writable.
#[no_mangle]
pub unsafe extern "C" fn lx4_last_error(
    handle: *mut Lx4Module,
    buffer: *mut u8,
    capacity: u32,
    out_size: *mut u32,
) -> i32 {
    if out_size.is_null() {
        return LX4_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    let text = match unsafe { handle.as_ref() } {
        Some(module) => module.error.clone(),
        None => CREATE_ERROR.with(|e| e.borrow().clone()),
    };
    // SAFETY: the contract.
    match unsafe { copy_text(&text, buffer, capacity, out_size) } {
        Ok(()) => LX4_OK,
        Err((code, _)) => code,
    }
}
