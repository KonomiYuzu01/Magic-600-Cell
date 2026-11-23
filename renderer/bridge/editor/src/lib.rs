//! The shell's editor library (LP4-3; chapter 04 EV1, chapter 05 KN5): the write-check-run flow of a `.6cs` macro for
//! C9's product shell, a draft. The shell's C# panel (`renderer/shell/project/Editor`) loads it from
//! `M600_EDITOR_DLL` beside the host library and shuttles JSON between the two; the libraries share no Rust object.
//!
//! - **Write:** one document, the panel's buffer, opened with the units the panel counts positions in
//!   ([`M6E_UNITS_UTF16`] for Godot's text controls). [`m6e_set_text`] replaces it and [`m6e_edit`] replaces a range.
//! - **Services:** [`m6e_diagnose`] and [`m6e_complete`] are EV1's editor services (`c600-editor`, LP4-1) on that
//!   document, under a module context of this parser's edition. They read source and run no program (EV1.9).
//! - **Check:** [`m6e_check`] checks the buffer as a single-file module with one entry, through the host's module
//!   set (`c600_module::Modules`, LP4-2), as the solver's own program (`Author::Own`) in the solving profile. A check
//!   creates a candidate and grants nothing; it never activates (EV1.8, EV1.11).
//! - **Activate:** [`m6e_activate`] is the explicit switch (EV1.11, EV1.13): the panel passes the candidate digest and
//!   the check reference that the check answered and the active digest it expects. The panel offers it as a palette
//!   entry with no default key (LP4 plan, integrator default).
//! - **Run:** the host's command layer queues a program job when the solver invokes the module's language row from the
//!   palette or by its key; the panel takes it from the host (`m6h_take_job`) and hands its JSON to [`m6e_run`], which
//!   binds the active version once (`c600_module::Modules::bind`, EV1.11) and runs the bound entry on a worker thread,
//!   in this process or, after [`m6e_load_runtime`], in the runtime's own process (route RT1). [`m6e_run_poll`]
//!   gives the run's end, which the panel reports to the host (`m6h_finish_job`). The run's engine host grants no
//!   row, and no run holds a commit token: a word the macro returns is shown to the solver, who previews it with a
//!   command (the draft and `operation.preview`'s key) and commits it with the key route's token.
//!
//! Calls: a handle is used by one thread at a time (the shell's main thread). Every function checks its pointers,
//! catches a panic and answers a status code. A panic poisons the handle: it then answers only [`M6E_E_POISONED`]
//! until it is destroyed. This library loads no library, opens no window and calls no Windows function; nothing here
//! is Windows, input or timing evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::ptr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use c600_check::{Grant, Policy};
use c600_digest::{hex, sha256};
use c600_editor::{complete, diagnose, parse, Document, Envelope, ModuleContext, OutlineKind, Stage, Units, Verdict};
use c600_kernel::json::{self, Value as Json};
use c600_module::code::MODULE_NOT_CHECKED;
use c600_module::{resolve, Binding, CheckReference, Entry, Files, Identity, Manifest, Modules, Refusal};
use c600_run::boundary::Store;
use c600_run::interp::NoHost;
use c600_run::meter::Amounts;
use c600_run::provenance::{Author, Profile};
use c600_run::value::Value;
use c600_runtime::parent::RUNTIME_LOST;
use c600_tools::modules::{run_bound_in_process, ModuleRun};
use c600_tools::process::{stage_name, Cause, ProcessHost, Stopped, IN_PROCESS, PROCESS};

/// The version of this C interface. The shell refuses a library that answers another.
pub const M6E_ABI_VERSION: u32 = 1;

pub const M6E_OK: i32 = 0;
/// A null or malformed argument, or a buffer too small for a text.
pub const M6E_E_INVALID_ARGUMENT: i32 = 1;
/// A service, the module set or a binding refused the request; [`m6e_last_error`] gives `{"code", "detail"}`, the
/// refusal's stable code and its account.
pub const M6E_E_REFUSED: i32 = 2;
/// The runtime's process was not loaded: a second load, or a worker thread that does not start.
pub const M6E_E_RUNTIME: i32 = 3;
/// A call panicked earlier; the handle answers only this until it is destroyed.
pub const M6E_E_POISONED: i32 = 6;

/// The units a host counts positions in (EV1.10): [`m6e_create`]'s `units`.
pub const M6E_UNITS_UTF8: u32 = 0;
pub const M6E_UNITS_UTF16: u32 = 1;
pub const M6E_UNITS_SCALAR: u32 = 2;

/// [`m6e_run_poll`]'s states.
pub const M6E_RUN_PENDING: u32 = 0;
pub const M6E_RUN_COMPLETED: u32 = 1;
pub const M6E_RUN_REFUSED: u32 = 2;

/// The path of the buffer's file in the module it is checked as.
pub const SOURCE_FILE: &str = "src/root.6cs";
/// The wall-clock safety stop of one run in the runtime's process (KN7.10); outside the deterministic key.
pub const RUN_DEADLINE: Duration = Duration::from_secs(60);
/// The most run records a handle keeps; the oldest goes first.
pub const RECORDS: usize = 1024;

type Failure = (i32, String);

fn invalid(text: impl Into<String>) -> Failure {
    (M6E_E_INVALID_ARGUMENT, text.into())
}

/// A refusal as [`m6e_last_error`] gives it.
fn refused(code: &str, detail: impl Into<String>) -> Failure {
    let detail = Json::String(detail.into());
    (M6E_E_REFUSED, json::object([("code", Json::String(code.to_owned())), ("detail", detail)]).canonical())
}

fn refusal(refusal: Refusal) -> Failure {
    refused(refusal.code, refusal.detail)
}

fn envelope(envelope: Envelope) -> Failure {
    refused(envelope.code, envelope.summary)
}

fn text(value: impl Into<String>) -> Json {
    Json::String(value.into())
}

fn digest(digest: &[u8; 32]) -> Json {
    text(hex(digest))
}

/// Work for the worker thread.
enum Work {
    /// Run every later job in the runtime's process that `program` starts.
    Runtime(PathBuf),
    Run {
        ticket: u64,
        job: i64,
        binding: Binding,
        run: ModuleRun,
    },
}

/// The thread that runs the bound entries, so that the shell's frame loop never waits on a run.
struct Worker {
    work: Option<Sender<Work>>,
    ends: Receiver<(u64, Json)>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn start() -> std::io::Result<Worker> {
        let (work, inbox) = mpsc::channel::<Work>();
        let (outbox, ends) = mpsc::channel();
        let thread = thread::Builder::new().name("c600-shell-editor runs".into()).spawn(move || {
            let mut process: Option<ProcessHost> = None;
            while let Ok(item) = inbox.recv() {
                match item {
                    Work::Runtime(program) => {
                        let mut host = ProcessHost::new(program);
                        host.deadline = Some(RUN_DEADLINE);
                        process = Some(host);
                    }
                    Work::Run { ticket, job, binding, run } => {
                        let ended = catch_unwind(AssertUnwindSafe(|| execute(&binding, &run, process.as_mut())))
                            .unwrap_or_else(|_| Err((RUNTIME_LOST.to_owned(), "the run panicked".to_owned())));
                        let _ = outbox.send((ticket, record(job, ended)));
                    }
                }
            }
        })?;
        Ok(Worker { work: Some(work), ends, thread: Some(thread) })
    }

    fn send(&self, work: Work) -> bool {
        self.work.as_ref().is_some_and(|w| w.send(work).is_ok())
    }
}

impl Drop for Worker {
    /// Ends the worker after the run in flight; the runtime's process ends with it.
    fn drop(&mut self) {
        self.work = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The end of a run: its record's fields, or its stop code and account.
type Ended = Result<Json, (String, String)>;

/// Runs a bound entry with no host row (the engine host grants nothing) and no argument.
fn execute(binding: &Binding, run: &ModuleRun, process: Option<&mut ProcessHost>) -> Ended {
    let (mut engine, mut store) = (NoHost, Store::default());
    let (route, ended) = match process {
        Some(host) => (PROCESS, host.run_bound(binding, run, &mut engine, &mut store)),
        None => (IN_PROCESS, run_bound_in_process(binding, run, &mut engine, &mut store)),
    };
    let bound = ended.map_err(stopped)?;
    let ran = bound.outcome.run;
    let Some(output) = ran.output else {
        let code = ran.summary.code.unwrap_or(RUNTIME_LOST);
        return Err((code.to_owned(), format!("the run ended {:?} with no output", ran.summary.outcome)));
    };
    let word = match &ran.value {
        Some(Value::Word(letters)) => Some(letters.to_vec()),
        _ => word_of(&output),
    };
    let word = word.map_or(Json::Null, |w| Json::Array(w.into_iter().map(|l| Json::Int(i64::from(l))).collect()));
    Ok(json::object([
        ("host", text(route)),
        ("closure", digest(&bound.closure)),
        ("source", digest(&bound.source)),
        ("output", text(String::from_utf8_lossy(&output))),
        ("word", word),
    ]))
}

/// The letters of a run's canonical output when it is a word (`[1,-2,3]`), as `c600_run::encode` writes one.
fn word_of(output: &[u8]) -> Option<Vec<i32>> {
    let value = json::parse(output.strip_suffix(b"\n").unwrap_or(output)).ok()?;
    value.as_array()?.iter().map(|l| l.as_i64().and_then(|l| i32::try_from(l).ok())).collect()
}

fn stopped(stopped: Stopped) -> (String, String) {
    match stopped.cause {
        Cause::Refused(refused) => {
            let code = refused.codes.first().cloned().unwrap_or_else(|| RUNTIME_LOST.to_owned());
            (code, format!("refused at the {} stage: {}", stage_name(refused.stage), refused.detail))
        }
        Cause::Lost(fault) => (fault.code().to_owned(), fault.detail),
        Cause::OtherSource => (RUNTIME_LOST.to_owned(), "the runtime was handed another source".to_owned()),
    }
}

/// A run's record: `{"job", "status": "completed", "host", "closure", "source", "output", "word"}` or
/// `{"job", "status": "refused", "code", "detail"}`.
fn record(job: i64, ended: Ended) -> Json {
    let mut fields = match ended {
        Ok(Json::Object(fields)) => fields,
        Ok(_) => BTreeMap::new(),
        Err((code, detail)) => {
            let mut fields = BTreeMap::new();
            fields.insert("code".to_owned(), text(code));
            fields.insert("detail".to_owned(), text(detail));
            fields.insert("status".to_owned(), text("refused"));
            fields
        }
    };
    fields.entry("status".to_owned()).or_insert_with(|| text("completed"));
    fields.insert("job".to_owned(), Json::Int(job));
    Json::Object(fields)
}

/// A handle: the panel's document, the host's user modules and the runs.
pub struct ShellEditor {
    document: Document,
    context: ModuleContext,
    modules: Modules,
    worker: Worker,
    runtime: bool,
    records: BTreeMap<u64, Json>,
    pending: BTreeSet<u64>,
    next: u64,
    answer: String,
    error: String,
    poisoned: bool,
}

impl ShellEditor {
    /// A handle with an empty document in `units`.
    pub fn new(units: Units) -> std::io::Result<ShellEditor> {
        let modules =
            Modules::new(Identity::full(), c600_tools::tool::contract(), c600_tools::tool::checker(), Grant::none());
        Ok(ShellEditor {
            document: Document::open(Vec::new(), units),
            context: ModuleContext::new(),
            modules,
            worker: Worker::start()?,
            runtime: false,
            records: BTreeMap::new(),
            pending: BTreeSet::new(),
            next: 1,
            answer: String::new(),
            error: String::new(),
            poisoned: false,
        })
    }

    fn set_text(&mut self, bytes: Vec<u8>) {
        self.document = Document::open(bytes, self.document.units());
        self.answer = json::object([("digest", text(self.document.digest()))]).canonical();
    }

    /// Replaces `[start, end)` in the declared units with `insert`.
    fn edit(&mut self, start: usize, end: usize, insert: &str) -> Result<(), Failure> {
        let range = self.document.range(self.document.digest(), start, end).map_err(envelope)?;
        let mut bytes = self.document.bytes().to_vec();
        bytes.splice(range.start..range.end, insert.bytes());
        self.set_text(bytes);
        Ok(())
    }

    /// A source range as `{"start", "end", "line", "column"}`: the endpoints in the declared units, the start's line
    /// and column (1-based, the column in scalars).
    fn place(&self, range: &c600_editor::SourceRange) -> Json {
        let Ok(host) = self.document.host(range) else { return Json::Null };
        let head = self.document.text().and_then(|t| t.get(..range.start)).unwrap_or("");
        let line = head.matches('\n').count() + 1;
        let column = head.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        let int = |n: usize| Json::Int(i64::try_from(n).unwrap_or(i64::MAX));
        json::object([("start", int(host.start)), ("end", int(host.end)), ("line", int(line)), ("column", int(column))])
    }

    fn diagnose(&mut self) -> Result<(), Failure> {
        let digest = self.document.digest().to_owned();
        let diagnosis = diagnose(&self.document, &digest, &self.context).map_err(envelope)?;
        let parsed = parse(&self.document, &self.context.edition);
        let exports: Vec<Json> = parsed
            .outline
            .iter()
            .filter(|item| item.exported && matches!(item.kind, OutlineKind::Fn | OutlineKind::Command))
            .filter_map(|item| item.name.clone())
            .map(Json::String)
            .collect();
        let diagnostics: Vec<Json> = diagnosis
            .diagnostics
            .iter()
            .map(|d| {
                let at = d.span.as_ref().map_or(Json::Null, |span| self.place(span));
                json::object([("code", text(d.code)), ("summary", text(d.summary.clone())), ("at", at)])
            })
            .collect();
        let (verdict, stage, module) = match &diagnosis.verdict {
            Verdict::Checked(candidate) => ("checked", Json::Null, text(candidate.module.clone())),
            Verdict::Refused => ("refused", Json::Null, Json::Null),
            Verdict::NotChecked(stage) => {
                let name = match stage {
                    Stage::Encoding => "encoding",
                    Stage::Syntax => "syntax",
                    Stage::Dependencies => "dependencies",
                    Stage::Lowering => "lowering",
                };
                ("not_checked", text(name), Json::Null)
            }
        };
        self.answer = json::object([
            ("digest", text(digest)),
            ("verdict", text(verdict)),
            ("stage", stage),
            ("module", module),
            ("exports", Json::Array(exports)),
            ("diagnostics", Json::Array(diagnostics)),
        ])
        .canonical();
        Ok(())
    }

    fn complete(&mut self, position: usize) -> Result<(), Failure> {
        let answer = complete(&self.document, self.document.digest(), position, &self.context).map_err(envelope)?;
        let candidates: Vec<Json> = answer
            .candidates
            .iter()
            .map(|c| {
                let kind = format!("{:?}", c.kind).to_lowercase();
                json::object([("label", text(c.label.clone())), ("kind", text(kind))])
            })
            .collect();
        self.answer = json::object([
            ("prefix", text(answer.prefix.clone())),
            ("replace", self.place(&answer.replace)),
            ("candidates", Json::Array(candidates)),
        ])
        .canonical();
        Ok(())
    }

    /// Checks the buffer as the single-file module its header names, with the one entry `entry`.
    fn check(&mut self, entry: &str) -> Result<(), Failure> {
        let current = self.document.digest().to_owned();
        let diagnosis = diagnose(&self.document, &current, &self.context).map_err(envelope)?;
        let module = match diagnosis.verdict {
            Verdict::Checked(candidate) => candidate.module,
            _ => {
                let (code, summary) = diagnosis
                    .diagnostics
                    .first()
                    .map_or((MODULE_NOT_CHECKED, String::new()), |d| (d.code, d.summary.clone()));
                return Err(refused(code, format!("the draft does not check yet: {summary}")));
            }
        };
        let bytes = self.document.bytes().to_vec();
        let manifest = Manifest {
            module: module.clone(),
            edition: self.context.edition.clone(),
            source: sha256(&bytes),
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            boundary: None,
            principle: None,
            supports: vec![Identity::full()],
        }
        .bytes();
        let files = Files::new().with(SOURCE_FILE, &bytes);
        let lock = resolve(&manifest, &files).map_err(refusal)?.bytes();
        let entry = Entry {
            module: module.clone(),
            function: entry.to_owned(),
            author: Author::Own,
            profile: Profile::Solving,
            grant: Grant::none(),
            policy: Policy::interim(),
        };
        let function = entry.function.clone();
        let candidate = self.modules.check(&manifest, &lock, &files, &[entry]).map_err(refusal)?;
        let active = self.modules.active(&module).map_or(Json::Null, |d| digest(&d));
        self.answer = json::object([
            ("module", text(module)),
            ("entry", text(function)),
            ("candidate", digest(&candidate.digest)),
            ("reference", digest(&candidate.reference.0)),
            ("source", digest(&candidate.source)),
            ("active", active),
        ])
        .canonical();
        Ok(())
    }

    fn activate(
        &mut self,
        module: &str,
        expected: Option<[u8; 32]>,
        candidate: [u8; 32],
        reference: [u8; 32],
    ) -> Result<(), Failure> {
        let activation =
            self.modules.activate_module(module, expected, candidate, CheckReference(reference)).map_err(refusal)?;
        self.answer = json::object([
            ("module", text(activation.module)),
            ("previous", activation.previous.map_or(Json::Null, |d| digest(&d))),
            ("new", digest(&activation.new)),
        ])
        .canonical();
        Ok(())
    }

    fn insert(&mut self, ticket: u64, record: Json) {
        self.pending.remove(&ticket);
        self.records.insert(ticket, record);
        while self.records.len() > RECORDS {
            self.records.pop_first();
        }
    }

    /// Binds the job's entry to its module's active version and hands the run to the worker; the run's ticket.
    fn run(&mut self, job: &Json) -> Result<u64, Failure> {
        let field =
            |key: &str| job.get(key).and_then(Json::as_str).ok_or_else(|| invalid(format!("the job has no `{key}`")));
        let id = job.get("job").and_then(Json::as_i64).ok_or_else(|| invalid("the job has no `job`"))?;
        let (module, entry) = (field("module")?, field("entry")?);
        let inputs = job
            .get("inputs")
            .and_then(Json::as_array)
            .ok_or_else(|| invalid("the job has no `inputs`"))?
            .iter()
            .map(|block| block.as_str().and_then(unhex).ok_or_else(|| invalid("an input block is not hex")))
            .collect::<Result<Vec<_>, _>>()?;
        let limits = job.get("limits").ok_or_else(|| invalid("the job has no `limits`"))?;
        let amount = |key: &str| {
            limits
                .get(key)
                .and_then(Json::as_str)
                .and_then(|n| n.parse::<u128>().ok())
                .ok_or_else(|| invalid(format!("the job's limit `{key}` is no decimal amount")))
        };
        let limits = Amounts {
            steps: amount("steps")?,
            kernel_cost: amount("kernel_cost")?,
            memory: amount("memory")?,
            output: amount("output")?,
        };
        let run = ModuleRun {
            run: field("run")?.to_owned(),
            base: field("base")?.to_owned(),
            args: Vec::new(),
            inputs,
            limits,
        };
        let ticket = self.next;
        self.next += 1;
        match self.modules.bind(module, entry) {
            Ok(binding) => {
                self.pending.insert(ticket);
                if !self.worker.send(Work::Run { ticket, job: id, binding, run }) {
                    let ended = Err((RUNTIME_LOST.to_owned(), "the worker thread has ended".to_owned()));
                    self.insert(ticket, record(id, ended));
                }
            }
            Err(refusal) => self.insert(ticket, record(id, Err((refusal.code.to_owned(), refusal.detail)))),
        }
        Ok(ticket)
    }

    fn poll(&mut self, ticket: u64) -> Result<u32, Failure> {
        let ends: Vec<(u64, Json)> = self.worker.ends.try_iter().collect();
        for (t, record) in ends {
            self.insert(t, record);
        }
        if self.pending.contains(&ticket) {
            self.answer = "null".to_owned();
            return Ok(M6E_RUN_PENDING);
        }
        let record = self.records.get(&ticket).ok_or_else(|| invalid(format!("no run has ticket {ticket}")))?;
        self.answer = record.canonical();
        Ok(match record.get("status").and_then(Json::as_str) {
            Some("completed") => M6E_RUN_COMPLETED,
            _ => M6E_RUN_REFUSED,
        })
    }
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok()).collect()
}

fn digest32(pointer: *const u8) -> Option<[u8; 32]> {
    // SAFETY: the caller's contract: null or 32 readable bytes.
    (!pointer.is_null()).then(|| unsafe { pointer.cast::<[u8; 32]>().read_unaligned() })
}

/// Runs `body` on a live handle and turns its outcome into a status code.
///
/// # Safety
///
/// `handle` is null or a live handle from [`m6e_create`] that no other thread is using.
unsafe fn with_editor(handle: *mut ShellEditor, body: impl FnOnce(&mut ShellEditor) -> Result<(), Failure>) -> i32 {
    // SAFETY: the caller's contract.
    let Some(editor) = (unsafe { handle.as_mut() }) else {
        return M6E_E_INVALID_ARGUMENT;
    };
    if editor.poisoned {
        return M6E_E_POISONED;
    }
    match catch_unwind(AssertUnwindSafe(|| body(&mut *editor))) {
        Ok(Ok(())) => M6E_OK,
        Ok(Err((code, text))) => {
            editor.error = text;
            code
        }
        Err(_) => {
            editor.poisoned = true;
            editor.error = "a call panicked; the handle only answers M6E_E_POISONED now".to_owned();
            M6E_E_POISONED
        }
    }
}

/// `len` bytes at `text` as UTF-8; null with length 0 is the empty text.
///
/// # Safety
///
/// `text` is null or points to `len` readable bytes that stay valid while the answer is used.
unsafe fn utf8<'a>(text: *const u8, len: usize, what: &str) -> Result<&'a str, Failure> {
    if text.is_null() {
        return if len == 0 { Ok("") } else { Err(invalid(format!("a null {what}"))) };
    }
    // SAFETY: the contract.
    let bytes = unsafe { std::slice::from_raw_parts(text, len) };
    std::str::from_utf8(bytes).map_err(|_| invalid(format!("the {what} is not UTF-8")))
}

/// Writes `text` with a NUL to `buffer` and its size with the NUL to `*out_size`; a smaller buffer answers
/// [`M6E_E_INVALID_ARGUMENT`] and writes nothing, as the host library's texts do.
///
/// # Safety
///
/// `buffer` points to `capacity` writable bytes (or is null); `out_size` is writable.
unsafe fn copy_text(text: &str, buffer: *mut u8, capacity: u32, out_size: *mut u32) -> i32 {
    let needed = text.len() + 1;
    // SAFETY: the contract.
    unsafe {
        *out_size = u32::try_from(needed).unwrap_or(u32::MAX);
        if buffer.is_null() || (capacity as usize) < needed {
            return M6E_E_INVALID_ARGUMENT;
        }
        ptr::copy_nonoverlapping(text.as_ptr(), buffer, text.len());
        *buffer.add(text.len()) = 0;
    }
    M6E_OK
}

/// The version of this C interface, [`M6E_ABI_VERSION`].
#[no_mangle]
pub extern "C" fn m6e_abi_version() -> u32 {
    M6E_ABI_VERSION
}

/// Creates a handle whose document counts positions in `units` ([`M6E_UNITS_UTF8`], [`M6E_UNITS_UTF16`] or
/// [`M6E_UNITS_SCALAR`]), with an empty buffer, an empty module set and its worker thread. `*out` gets the handle, or
/// null on a failure.
///
/// # Safety
///
/// `out` points to a writable handle.
#[no_mangle]
pub unsafe extern "C" fn m6e_create(units: u32, out: *mut *mut ShellEditor) -> i32 {
    if out.is_null() {
        return M6E_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe { out.write(ptr::null_mut()) };
    let units = match units {
        M6E_UNITS_UTF8 => Units::Utf8,
        M6E_UNITS_UTF16 => Units::Utf16,
        M6E_UNITS_SCALAR => Units::Scalar,
        _ => return M6E_E_INVALID_ARGUMENT,
    };
    match catch_unwind(|| ShellEditor::new(units)) {
        // SAFETY: the contract.
        Ok(Ok(editor)) => unsafe {
            out.write(Box::into_raw(Box::new(editor)));
            M6E_OK
        },
        Ok(Err(_)) => M6E_E_RUNTIME,
        Err(_) => M6E_E_POISONED,
    }
}

/// Runs every later run in the language runtime's process (route RT1, c600-runtime's child) that the program
/// `program` (UTF-8, `program_len` bytes; the host's `c600-shell-runtime` binary) starts on the first such run, under
/// the safety stop [`RUN_DEADLINE`]. A second load answers [`M6E_E_RUNTIME`]. Without a load, runs run in this
/// process.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `program` points to its readable bytes.
#[no_mangle]
pub unsafe extern "C" fn m6e_load_runtime(handle: *mut ShellEditor, program: *const u8, program_len: usize) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_editor(handle, |editor| {
            let program = utf8(program, program_len, "runtime program path")?;
            if program.is_empty() {
                return Err(invalid("an empty runtime program path"));
            }
            if editor.runtime {
                return Err((M6E_E_RUNTIME, "this handle has loaded its runtime already".to_owned()));
            }
            if !editor.worker.send(Work::Runtime(PathBuf::from(program))) {
                return Err((M6E_E_RUNTIME, "the worker thread has ended".to_owned()));
            }
            editor.runtime = true;
            Ok(())
        })
    }
}

/// Replaces the buffer with `text` (`text_len` bytes; bytes that are not UTF-8/LF source open too, and the services
/// report ERR.10's encoding diagnostic). [`m6e_answer`] gives `{"digest"}`, the buffer's SHA-256.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `text` is null with length 0 or points to its bytes.
#[no_mangle]
pub unsafe extern "C" fn m6e_set_text(handle: *mut ShellEditor, text: *const u8, text_len: usize) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_editor(handle, |editor| {
            let bytes = match (text.is_null(), text_len) {
                (true, 0) => Vec::new(),
                (true, _) => return Err(invalid("a null text")),
                _ => std::slice::from_raw_parts(text, text_len).to_vec(),
            };
            editor.set_text(bytes);
            Ok(())
        })
    }
}

/// Replaces the range `[start, end)` of the buffer, in the declared units, with `text` (UTF-8, `text_len` bytes). A
/// position off the buffer or inside a scalar, a reversed range, or a buffer that is not pinned source answers
/// [`M6E_E_REFUSED`] with `EV1:input.source_range_invalid` and changes nothing. [`m6e_answer`] gives `{"digest"}`.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `text` is null with length 0 or points to its bytes.
#[no_mangle]
pub unsafe extern "C" fn m6e_edit(
    handle: *mut ShellEditor,
    start: u64,
    end: u64,
    text: *const u8,
    text_len: usize,
) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_editor(handle, |editor| {
            let insert = utf8(text, text_len, "text")?;
            let start = usize::try_from(start).map_err(|_| invalid("the start is out of range"))?;
            let end = usize::try_from(end).map_err(|_| invalid("the end is out of range"))?;
            editor.edit(start, end, insert)
        })
    }
}

/// Diagnoses the buffer (EV1.4, `c600_editor::diagnose`). [`m6e_answer`] gives `{"digest", "verdict", "stage",
/// "module", "exports", "diagnostics"}`: the verdict `checked`, `refused` or `not_checked` (with the stage that
/// stopped it: `encoding`, `syntax`, `dependencies` or `lowering`), the module a checked draft declares, the exported
/// functions and commands, and each diagnostic as `{"code", "summary", "at"}`, `at` its range in the declared units
/// with the start's line and column (`{"start", "end", "line", "column"}`) or null. A checked verdict is a candidate
/// for the check, never a grant.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using.
#[no_mangle]
pub unsafe extern "C" fn m6e_diagnose(handle: *mut ShellEditor) -> i32 {
    // SAFETY: the contract.
    unsafe { with_editor(handle, ShellEditor::diagnose) }
}

/// Completes at `position`, in the declared units (EV1.3, `c600_editor::complete`). [`m6e_answer`] gives `{"prefix",
/// "replace", "candidates"}`: the identifier's part before the position, the range a candidate replaces, and each
/// candidate as `{"label", "kind"}`. A position the document refuses answers [`M6E_E_REFUSED`].
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using.
#[no_mangle]
pub unsafe extern "C" fn m6e_complete(handle: *mut ShellEditor, position: u64) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_editor(handle, |editor| {
            let position = usize::try_from(position).map_err(|_| invalid("the position is out of range"))?;
            editor.complete(position)
        })
    }
}

/// Checks the buffer as the single-file module its header names, with its function `entry` (UTF-8, `entry_len` bytes)
/// as the one entry, the solver's own program in the solving profile (EV1.2, EV1.11). A draft whose diagnosis is not
/// `checked` and a check the module set refuses answer [`M6E_E_REFUSED`] with the first code. A successful check
/// creates the module's candidate and activates nothing. [`m6e_answer`] gives `{"module", "entry", "candidate",
/// "reference", "source", "active"}` in hex: the candidate's closure digest and check reference, which
/// [`m6e_activate`] takes, its source digest, and the active version's digest or null.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `entry` points to its readable bytes.
#[no_mangle]
pub unsafe extern "C" fn m6e_check(handle: *mut ShellEditor, entry: *const u8, entry_len: usize) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_editor(handle, |editor| {
            let entry = utf8(entry, entry_len, "entry name")?;
            if entry.is_empty() {
                return Err(invalid("an empty entry name"));
            }
            editor.check(entry)
        })
    }
}

/// Activates the checked candidate `candidate` (32 bytes) of the module `module` (UTF-8, `module_len` bytes) with the
/// check reference `reference` (32 bytes), when the module's active version is `expected` (32 bytes, or null for
/// none): the explicit switch (EV1.11, EV1.13), which no check, edit or run performs. A refusal
/// (`EV1:input.module_not_checked`, `EV1:stale.activation_base`, `LG3:capability.not_granted`) answers
/// [`M6E_E_REFUSED`] and leaves the active version. [`m6e_answer`] gives `{"module", "previous", "new"}` in hex.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `module` points to its readable bytes; `candidate` and
/// `reference` point to 32 readable bytes; `expected` is null or points to 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn m6e_activate(
    handle: *mut ShellEditor,
    module: *const u8,
    module_len: usize,
    expected: *const u8,
    candidate: *const u8,
    reference: *const u8,
) -> i32 {
    if candidate.is_null() || reference.is_null() {
        return M6E_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_editor(handle, |editor| {
            let module = utf8(module, module_len, "module name")?;
            let (candidate, reference) = (digest32(candidate), digest32(reference));
            let (Some(candidate), Some(reference)) = (candidate, reference) else {
                return Err(invalid("a null digest"));
            };
            editor.activate(module, digest32(expected), candidate, reference)
        })
    }
}

/// Runs a program job: `job` (UTF-8 JSON, `job_len` bytes) is what the host's `m6h_take_job` answered. The call binds
/// the job's entry to its module's active version once (EV1.11) and hands the run to the worker thread; the run stays
/// on that version to its end, whatever activates meanwhile. `*out_ticket` gets the run's ticket for
/// [`m6e_run_poll`]. A binding the module set refuses (`EV1:input.module_not_checked`, `LG3:capability.not_granted`)
/// is a refused run at once. The entry runs with no argument and no host row.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `job` points to its readable bytes; `out_ticket` is
/// writable.
#[no_mangle]
pub unsafe extern "C" fn m6e_run(
    handle: *mut ShellEditor,
    job: *const u8,
    job_len: usize,
    out_ticket: *mut u64,
) -> i32 {
    if out_ticket.is_null() {
        return M6E_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_editor(handle, |editor| {
            out_ticket.write(0);
            let job = json::parse(utf8(job, job_len, "job")?.as_bytes()).map_err(|_| invalid("the job is not JSON"))?;
            out_ticket.write(editor.run(&job)?);
            Ok(())
        })
    }
}

/// Reads the run `ticket`: `*out_status` gets [`M6E_RUN_PENDING`], [`M6E_RUN_COMPLETED`] or [`M6E_RUN_REFUSED`], and
/// [`m6e_answer`] the run's record: `null` while it runs; `{"job", "status": "completed", "host", "closure",
/// "source", "output", "word"}` (the route, `in-process` or `process`; the bound version's closure and source digests
/// in hex; the canonical output; the letters of a returned word or null); or `{"job", "status": "refused", "code",
/// "detail"}`. The host takes `output` as the job's value and `code` with `detail` as its error. A handle keeps the
/// newest [`RECORDS`] records; an unknown or dropped ticket answers [`M6E_E_INVALID_ARGUMENT`].
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `out_status` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6e_run_poll(handle: *mut ShellEditor, ticket: u64, out_status: *mut u32) -> i32 {
    if out_status.is_null() {
        return M6E_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_editor(handle, |editor| {
            let status = editor.poll(ticket)?;
            out_status.write(status);
            Ok(())
        })
    }
}

/// The JSON answer of the last successful call that gives one, NUL-terminated; `*out_size` gets its size with the
/// NUL, and a smaller buffer answers [`M6E_E_INVALID_ARGUMENT`] and writes nothing.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `buffer` points to `capacity` writable bytes (or is
/// null); `out_size` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6e_answer(
    handle: *mut ShellEditor,
    buffer: *mut u8,
    capacity: u32,
    out_size: *mut u32,
) -> i32 {
    if out_size.is_null() {
        return M6E_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    match unsafe { handle.as_ref() } {
        // SAFETY: the contract.
        Some(editor) => unsafe { copy_text(&editor.answer, buffer, capacity, out_size) },
        None => M6E_E_INVALID_ARGUMENT,
    }
}

/// The reason for the handle's last failed call, NUL-terminated and sized as [`m6e_answer`] sizes its text: after
/// [`M6E_E_REFUSED`] the JSON `{"code", "detail"}`, otherwise plain text.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `buffer` points to `capacity` writable bytes (or is
/// null); `out_size` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6e_last_error(
    handle: *mut ShellEditor,
    buffer: *mut u8,
    capacity: u32,
    out_size: *mut u32,
) -> i32 {
    if out_size.is_null() {
        return M6E_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    match unsafe { handle.as_ref() } {
        // SAFETY: the contract.
        Some(editor) => unsafe { copy_text(&editor.error, buffer, capacity, out_size) },
        None => M6E_E_INVALID_ARGUMENT,
    }
}

/// Frees the handle. The worker thread ends after the run in flight, and the runtime's process with it.
///
/// # Safety
///
/// `handle` is null or a live handle that no other thread is using; it is not used again.
#[no_mangle]
pub unsafe extern "C" fn m6e_destroy(handle: *mut ShellEditor) {
    if !handle.is_null() {
        // SAFETY: the contract; a panic in a destructor is not carried across the C boundary.
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(handle) })));
    }
}
