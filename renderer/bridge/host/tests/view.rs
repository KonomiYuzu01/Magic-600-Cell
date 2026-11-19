//! The view jobs through the host's C interface (C9 U8; a draft), over the bridge's C shim and the fake core on route
//! B, with the language runtime's process that this package builds (`c600-shell-runtime`): a run that publishes, its
//! value digest and its first confirmed frame; refusals at the parse and admission stages; a run that a newer run
//! supersedes; a twist that replaces a run's revision before its delivery and before its installation; the missing
//! runtime; a destroy with a run in flight; the record bound; and the editor's drafts, stale and cleared. Every result
//! is a synthetic fixture outcome, not a GPU run, not input evidence and not a performance measurement.

#[path = "../../tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{Duration, Instant};

use c600_digest::sha256;
use c600_kernel::json::{self, Value};
use c600_render::FakeCore;
use c600_run::interp::NoHost;
use c600_run::meter::code::{CANCELLED_REVISION_REPLACED, NOT_GRANTED};
use c600_runtime::parent::{Config, Runtime};
use c600_runtime::{run_in_process, Ended, Job, JobOptions};
use c600_shell_host::view::SUPERSEDED;
use c600_shell_host::*;
use common::support::SIZE;
use common::{functions, Shim};

const CAMERA: [f32; 16] = [0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
/// How long a test waits for a run that ends at once; far below [`VIEW_DEADLINE`].
const WAIT: Duration = Duration::from_secs(30);

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n";

/// One module whose exported function returns a Nat.
fn returning(n: u32) -> String {
    format!("{HEADER}export fn one() -> Nat effects [] {{\n    return {n};\n}}\n")
}

/// A run that does not end before its deadline: 2^64 calls.
fn spinning() -> String {
    format!(
        "{HEADER}fn spin(n: Nat, top: Nat) -> Nat effects [] {{\n    return match exact_less(n, top) {{\n        \
         true => exact_add(spin(exact_add(n, 1), top), spin(exact_add(n, 1), top));\n        _ => 0;\n    }};\n}}\n\n\
         export fn forever() -> Nat effects [] {{\n    return spin(0, 64);\n}}\n"
    )
}

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn text(path: PathBuf) -> String {
    path.to_str().expect("a UTF-8 path").to_owned()
}

/// A session folder that does not exist yet, under the build's own temporary folder (never %TEMP%).
fn fresh(name: &str) -> String {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("c9-view-{name}-{}", std::process::id()));
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

/// The SHA-256 of the canonical output of `source`'s `entry`, run in this process.
fn value_digest(source: &str, entry: &str) -> [u8; 32] {
    let job = Job {
        sources: vec![source.as_bytes().to_vec()],
        module: "m".into(),
        entry: entry.into(),
        args: Vec::new(),
        options: JobOptions { record: false, ..JobOptions::new(VIEW_LIMITS) },
    };
    match run_in_process(&job, &mut NoHost, None) {
        Ended::Ran(run) => sha256(run.output.as_deref().expect("a completed run's output")),
        Ended::Refused(refused) => panic!("{refused:?}"),
    }
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

    /// A host with the runtime of this package loaded.
    fn with_runtime() -> Run {
        let run = Run::new();
        let program = env!("CARGO_BIN_EXE_c600-shell-runtime");
        // SAFETY: a live handle and readable bytes.
        let code = unsafe { m6h_load_runtime(run.host, program.as_ptr(), program.len()) };
        assert_eq!(code, M6H_OK, "{}", last_error(run.host));
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
                M6H_PUZZLE_JUMBLE_OFF,
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

    fn shell(&self) -> &ShellHost {
        // SAFETY: a live handle; the test uses it on this thread only, between calls.
        unsafe { &*self.host }
    }

    fn run_view(&self, source: &str, entry: &str) -> (i32, u64) {
        let mut ticket = 0u64;
        // SAFETY: a live handle, readable bytes and a writable ticket.
        let code =
            unsafe { m6h_run_view(self.host, source.as_ptr(), source.len(), entry.as_ptr(), entry.len(), &mut ticket) };
        (code, ticket)
    }

    /// Schedules a run and checks that the call answers at once with a pending record.
    fn start(&self, source: &str, entry: &str) -> u64 {
        let (code, ticket) = self.run_view(source, entry);
        assert_eq!(code, M6H_OK, "{}", last_error(self.host));
        let record = self.result(ticket);
        assert_eq!((record.status, record.kind), (M6H_VIEW_STATUS_PENDING, M6H_VIEW_KIND_RUN));
        assert_eq!(record.source_digest, sha256(source.as_bytes()));
        ticket
    }

    fn result(&self, ticket: u64) -> M6hViewResult {
        let mut out = M6hViewResult::default();
        // SAFETY: a live handle and a writable row.
        assert_eq!(unsafe { m6h_view_result(self.host, ticket, &mut out) }, M6H_OK, "{}", last_error(self.host));
        out
    }

    /// A refused record's code and reason, from the JSON that `m6h_last_error` answers after `m6h_view_result`.
    fn refusal(&self, ticket: u64) -> (String, String) {
        assert_eq!(self.result(ticket).status, M6H_VIEW_STATUS_REFUSED);
        let error = last_error(self.host);
        let value = json::parse(error.as_bytes()).unwrap_or_else(|e| panic!("{error}: {e:?}"));
        let field = |key: &str| value.get(key).and_then(Value::as_str).expect(key).to_owned();
        (field("code"), field("reason"))
    }

    fn draft(&self, digest: &[u8; 32]) -> u64 {
        let mut ticket = 0u64;
        // SAFETY: a live handle, 32 readable bytes and a writable ticket.
        assert_eq!(unsafe { m6h_view_draft(self.host, digest.as_ptr(), &mut ticket) }, M6H_OK);
        ticket
    }

    fn pump(&self) {
        // SAFETY: a live handle.
        assert_eq!(unsafe { m6h_pump(self.host) }, M6H_OK, "{}", last_error(self.host));
    }

    /// Pumps, completing the core's preparations, until the record of `ticket` is no longer pending.
    fn wait(&self, ticket: u64) -> M6hViewResult {
        let start = Instant::now();
        loop {
            self.pump();
            self.fake().complete();
            self.pump();
            let record = self.result(ticket);
            if record.status != M6H_VIEW_STATUS_PENDING {
                return record;
            }
            assert!(start.elapsed() < WAIT, "run {ticket} is still pending");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Pumps, without completing the core's preparations, until the run's publication went to the core.
    fn wait_delivered(&self) {
        let before = self.fake().prepared.len();
        let start = Instant::now();
        while self.fake().prepared.len() == before {
            self.pump();
            assert!(start.elapsed() < WAIT, "no publication was delivered");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Waits until the worker runs `ticket` in the runtime.
    fn wait_running(&self, ticket: u64) {
        let start = Instant::now();
        while self.shell().running_view_job() != ticket {
            self.pump();
            assert!(start.elapsed() < WAIT, "run {ticket} did not start");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn frame(&self) -> M6hFrame {
        let mut submitted = M6hFrame::default();
        // SAFETY: a live handle and a writable row.
        assert_eq!(unsafe { m6h_frame(self.host, &mut submitted) }, M6H_OK, "{}", last_error(self.host));
        submitted
    }

    /// Submits a frame, has the core confirm it, and reads the confirmations; the frame.
    fn shown(&self) -> M6hFrame {
        let frame = self.frame();
        self.fake().complete();
        let last = self.fake().submitted.len() as u64;
        assert!(self.fake().show(last));
        let mut rows = vec![M6hPresented::default(); 16];
        let mut count = 0u32;
        // SAFETY: `rows` holds 16 rows.
        assert_eq!(unsafe { m6h_read_presented(self.host, rows.as_mut_ptr(), 16, &mut count) }, M6H_OK);
        rows.truncate(count as usize);
        assert!(rows.iter().any(|r| r.sequence == frame.sequence), "frame {} was confirmed", frame.sequence);
        frame
    }

    fn press(&self, chord: &str) -> u64 {
        let mut key = M6hKey::default();
        // SAFETY: a live handle, readable bytes and a writable row.
        let code = unsafe { m6h_key_down(self.host, chord.as_ptr(), chord.len(), 0, &mut key) };
        assert_eq!((code, key.press), (M6H_OK, M6H_PRESS_COMMITTED), "{chord}: {}", last_error(self.host));
        // SAFETY: a live handle and readable bytes.
        assert_eq!(unsafe { m6h_key_up(self.host, chord.as_ptr(), chord.len()) }, M6H_OK);
        key.engine_revision
    }

    /// Destroys the handle now; the drop then only frees the shim.
    fn destroy(&mut self) {
        // SAFETY: the handle is not used again.
        unsafe { m6h_destroy(self.host) };
        self.host = ptr::null_mut();
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

#[test]
fn a_run_publishes_the_reference_publication_with_its_value_digest_and_first_frame() {
    assert_eq!(std::mem::size_of::<M6hViewResult>(), 104);
    let run = Run::with_runtime();
    let source = returning(1);
    let ticket = run.start(&source, "one");
    assert!(ticket > 1, "the ticket is a new command id after the first publication");

    let record = run.wait(ticket);
    assert_eq!(record.status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
    assert_eq!((record.kind, record.revision), (M6H_VIEW_KIND_RUN, ENGINE_REVISION));
    assert_eq!(record.source_digest, sha256(source.as_bytes()));
    assert_eq!(record.has_value, 1);
    assert_eq!(record.value_sha256, value_digest(&source, "one"), "the child's value is the in-process one");
    assert_eq!((record.frame, record.cleared_frame), (0, 0), "no frame confirmed yet");

    let frame = run.shown();
    assert_eq!(frame.publication_revision, ENGINE_REVISION);
    let record = run.result(ticket);
    assert_eq!(record.frame, frame.sequence, "the first confirmed frame that drew the run's publication");
    run.shown();
    assert_eq!(run.result(ticket).frame, frame.sequence, "the first frame stays");

    // A second run of the same text publishes again under a new ticket.
    let again = run.start(&source, "one");
    assert!(again > ticket);
    assert_eq!(run.wait(again).status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
}

#[test]
fn refusals_at_parse_and_admission_end_the_run_refused_with_their_code() {
    let run = Run::with_runtime();

    let broken = format!("{HEADER}export fn f( -> Nat effects [] {{\n    return 1;\n}}\n");
    let ticket = run.start(&broken, "f");
    assert_eq!(run.wait(ticket).status, M6H_VIEW_STATUS_REFUSED);
    let (code, reason) = run.refusal(ticket);
    assert!(!code.is_empty() && !code.starts_with("m6h:"), "a parse diagnostic's code: {code}");
    assert!(reason.contains("parse stage"), "{reason}");
    assert_eq!(run.result(ticket).has_value, 0);

    // No capability is granted: a program that names one is refused at admission.
    let random = "edition \"6cs-draft-1\";\nmodule m requires [random];\n\n\
                  export fn two() -> record { a: Nat, b: Nat } effects [random] {\n    \
                  return record { a = random_draw(), b = random_draw() };\n}\n";
    let ticket = run.start(random, "two");
    assert_eq!(run.wait(ticket).status, M6H_VIEW_STATUS_REFUSED);
    let (code, reason) = run.refusal(ticket);
    assert_eq!(code, NOT_GRANTED);
    assert!(reason.contains("admit stage"), "{reason}");

    // An entry the module does not export is refused at admission without a code.
    let ticket = run.start(&returning(1), "absent");
    assert_eq!(run.wait(ticket).status, M6H_VIEW_STATUS_REFUSED);
    assert_eq!(run.refusal(ticket).0, "m6h:refused.admit");

    // The view still draws the reference publication, and a later run publishes.
    assert_eq!(run.shown().publication_revision, ENGINE_REVISION);
    let ticket = run.start(&returning(2), "one");
    assert_eq!(run.wait(ticket).status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));

    // Bad arguments.
    let mut ticket = 0u64;
    let entry = "one";
    let invalid = [0xffu8, 0xfe];
    let mut out = M6hViewResult::default();
    // SAFETY: a live handle; the null and invalid inputs are refused before any use.
    unsafe {
        let none = m6h_run_view(run.host, ptr::null(), 0, entry.as_ptr(), entry.len(), &mut ticket);
        assert_eq!(none, M6H_E_INVALID_ARGUMENT);
        let bytes = m6h_run_view(run.host, invalid.as_ptr(), invalid.len(), entry.as_ptr(), entry.len(), &mut ticket);
        assert_eq!(bytes, M6H_E_INVALID_ARGUMENT);
        assert!(last_error(run.host).contains("not UTF-8"));
        let no_ticket =
            m6h_run_view(run.host, entry.as_ptr(), entry.len(), entry.as_ptr(), entry.len(), ptr::null_mut());
        assert_eq!(no_ticket, M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_view_result(run.host, 999_999, &mut out), M6H_E_INVALID_ARGUMENT);
        assert!(last_error(run.host).contains("no view job"));
        assert_eq!(m6h_view_result(run.host, 1, ptr::null_mut()), M6H_E_INVALID_ARGUMENT);
        assert_eq!(m6h_view_draft(run.host, ptr::null(), &mut ticket), M6H_E_INVALID_ARGUMENT);
    }
}

#[test]
fn a_second_run_supersedes_a_pending_first() {
    let run = Run::with_runtime();
    let first = run.start(&spinning(), "forever");
    run.wait_running(first);

    let source = returning(3);
    let second = run.start(&source, "one");
    assert_eq!(run.refusal(first).0, SUPERSEDED, "the first run ends at once");
    // The second runs only after the first left the runtime, which a cancel does long before its deadline.
    let record = run.wait(second);
    assert_eq!(record.status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
    assert_eq!(record.value_sha256, value_digest(&source, "one"));
    assert_eq!(run.refusal(first).0, SUPERSEDED, "the first stays superseded");
    assert_eq!(run.result(first).has_value, 0);
}

#[test]
fn a_twist_before_delivery_or_installation_ends_the_run_and_a_later_run_publishes() {
    let run = Run::with_runtime();
    let folder = fresh("twist");
    assert_eq!(run.open(&folder, &menu()), M6H_OK, "{}", last_error(run.host));
    let source = returning(4);

    // Not delivered yet: only a pump delivers, so the twist finds the run still with the worker.
    let early = run.start(&source, "one");
    assert_eq!(run.press("KeyJ"), 2);
    let (code, reason) = run.refusal(early);
    assert_eq!(code, CANCELLED_REVISION_REPLACED);
    assert!(reason.contains("replaced revision 1"), "{reason}");
    assert_eq!(run.result(early).revision, 1);

    // Delivered and being prepared: the host's adoption cancels it.
    let late = run.start(&source, "one");
    assert_eq!(run.result(late).revision, 2);
    run.wait_delivered();
    assert_eq!(run.result(late).status, M6H_VIEW_STATUS_PENDING, "not installed yet");
    assert_eq!(run.press("KeyJ"), 3);
    assert_eq!(run.refusal(late).0, CANCELLED_REVISION_REPLACED);

    // A run on the new revision publishes, and the frame draws that revision.
    let ticket = run.start(&source, "one");
    let record = run.wait(ticket);
    assert_eq!(record.status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
    assert_eq!(record.revision, 3);
    let frame = run.shown();
    assert_eq!((frame.engine_revision, frame.publication_revision), (3, 3));
    assert_eq!(run.result(ticket).frame, frame.sequence);
    assert_eq!(run.refusal(early).0, CANCELLED_REVISION_REPLACED, "the early run stays refused");
    drop(run);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_run_without_a_runtime_is_refused() {
    let run = Run::new();
    assert_eq!(run.run_view(&returning(1), "one").0, M6H_E_RUNTIME);
    assert!(last_error(run.host).contains("no runtime"), "{}", last_error(run.host));
}

#[test]
fn a_destroy_with_a_run_in_flight_ends_the_child_without_waiting_for_the_run() {
    // The worker's no-restart rule rests on this: a runtime without a program path does not start.
    assert!(Runtime::new(Config::new("")).start().is_err(), "an empty program path starts no child");

    let mut run = Run::with_runtime();
    let ticket = run.start(&spinning(), "forever");
    run.wait_running(ticket);
    std::thread::sleep(Duration::from_millis(200));
    run.pump();
    assert_eq!(run.result(ticket).status, M6H_VIEW_STATUS_PENDING, "the run is still in flight");
    assert_eq!(run.shell().running_view_job(), ticket);

    let start = Instant::now();
    run.destroy();
    // The run's deadline is VIEW_DEADLINE; a destroy that returns long before it ended the child.
    let took = start.elapsed();
    assert!(took < Duration::from_secs(10), "the destroy took {took:?}");
}

#[test]
fn the_records_are_bounded_and_keep_the_pending_run() {
    let run = Run::with_runtime();
    let pending = run.start(&spinning(), "forever");
    let first = run.draft(&[1; 32]);
    for i in 0..view::RECORDS {
        run.draft(&sha256(&i.to_le_bytes()));
    }
    assert_eq!(run.result(pending).status, M6H_VIEW_STATUS_PENDING, "the pending run's record stays");
    let mut out = M6hViewResult::default();
    // SAFETY: a live handle and a writable row.
    assert_eq!(unsafe { m6h_view_result(run.host, first, &mut out) }, M6H_E_INVALID_ARGUMENT, "the oldest draft went");
    // No draft cancelled the run.
    run.wait_running(pending);
    assert_eq!(run.result(pending).status, M6H_VIEW_STATUS_PENDING);
}

#[test]
fn a_draft_reads_stale_until_a_run_of_its_text_publishes() {
    let run = Run::with_runtime();
    let a = returning(5);
    let b = returning(6);
    let ticket = run.start(&a, "one");
    assert_eq!(run.wait(ticket).status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
    run.shown();

    let draft = run.draft(&sha256(b.as_bytes()));
    let record = run.result(draft);
    assert_eq!((record.status, record.kind), (M6H_VIEW_STATUS_PUBLISHED, M6H_VIEW_KIND_DRAFT));
    assert_eq!((record.revision, record.source_digest), (ENGINE_REVISION, sha256(b.as_bytes())));
    assert_eq!((record.frame, record.cleared_frame, record.has_value), (0, 0, 0));
    let stale = run.shown();
    assert_eq!(run.result(draft).frame, stale.sequence, "the first frame after the draft draws stale output");
    run.shown();
    assert_eq!(run.result(draft).cleared_frame, 0, "stale until a run of the draft's text publishes");

    let ticket = run.start(&b, "one");
    assert_eq!(run.wait(ticket).status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
    let cleared = run.shown();
    let record = run.result(draft);
    assert_eq!((record.frame, record.cleared_frame), (stale.sequence, cleared.sequence));
    assert_eq!(run.result(ticket).frame, cleared.sequence);
}

#[test]
fn a_draft_of_the_producing_text_is_never_stale() {
    let run = Run::with_runtime();
    let a = returning(7);
    let ticket = run.start(&a, "one");
    assert_eq!(run.wait(ticket).status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
    let draft = run.draft(&sha256(a.as_bytes()));
    for _ in 0..3 {
        run.shown();
    }
    let record = run.result(draft);
    assert_eq!((record.status, record.frame, record.cleared_frame), (M6H_VIEW_STATUS_PUBLISHED, 0, 0));
}

#[test]
fn a_draft_cancels_no_run_and_a_run_of_a_new_text_supersedes_it() {
    let run = Run::with_runtime();
    let a = returning(8);
    let c = sha256(returning(9).as_bytes());

    // A draft while the run is with the worker.
    let first = run.start(&a, "one");
    let early = run.draft(&c);
    let record = run.wait(first);
    assert_eq!(record.status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
    // The run's publication came from another text than the draft's, so the draft reads stale.
    let frame = run.shown();
    assert_eq!(run.result(first).frame, frame.sequence);
    assert_eq!(run.result(early).frame, frame.sequence);

    // A draft while the run's publication is being prepared.
    let second = run.start(&a, "one");
    run.wait_delivered();
    let late = run.draft(&c);
    let record = run.wait(second);
    assert_eq!(record.status, M6H_VIEW_STATUS_PUBLISHED, "{}", last_error(run.host));
    let frame = run.shown();
    assert_eq!(run.result(second).frame, frame.sequence);
    assert_eq!(run.result(late).frame, frame.sequence, "stale once the run ended");
    assert_eq!(run.result(late).cleared_frame, 0);
}
